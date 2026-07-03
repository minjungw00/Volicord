[CmdletBinding()]
param(
    [string]$Bin = "volicord"
)

Set-StrictMode -Version 2.0
$ErrorActionPreference = "Stop"

function Fail {
    param([string]$Message)
    Write-Error "volicord smoke test: $Message"
    exit 1
}

function Invoke-Checked {
    param(
        [string]$FilePath,
        [string[]]$Arguments
    )
    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        Fail "$FilePath $($Arguments -join ' ') exited with $LASTEXITCODE"
    }
}

function Invoke-Captured {
    param(
        [string]$FilePath,
        [string[]]$Arguments
    )
    $output = & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        Fail "$FilePath $($Arguments -join ' ') exited with $LASTEXITCODE"
    }
    return ($output | Out-String)
}

function Invoke-WithInput {
    param(
        [string]$FilePath,
        [string[]]$Arguments,
        [string]$InputText,
        [string]$RuntimeHome,
        [string]$PathValue
    )

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $FilePath
    foreach ($arg in $Arguments) {
        [void]$startInfo.ArgumentList.Add($arg)
    }
    $startInfo.RedirectStandardInput = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.UseShellExecute = $false
    $startInfo.Environment["VOLICORD_HOME"] = $RuntimeHome
    $startInfo.Environment["PATH"] = $PathValue

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    [void]$process.Start()
    $process.StandardInput.Write($InputText)
    $process.StandardInput.Close()
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    if (-not $process.WaitForExit(10000)) {
        $process.Kill()
        Fail "$FilePath $($Arguments -join ' ') timed out"
    }
    if ($process.ExitCode -ne 0) {
        Fail "$FilePath $($Arguments -join ' ') exited with $($process.ExitCode): $stderr"
    }
    return @{ Stdout = $stdout; Stderr = $stderr }
}

if ($Bin -match "[/\\]") {
    if (-not (Test-Path -LiteralPath $Bin -PathType Leaf)) {
        Fail "$Bin was not found"
    }
    $command = (Resolve-Path -LiteralPath $Bin).Path
} else {
    $resolved = Get-Command $Bin -ErrorAction SilentlyContinue
    if ($null -eq $resolved) {
        Fail "$Bin was not found on PATH"
    }
    $command = $resolved.Source
}

$workDir = Join-Path ([System.IO.Path]::GetTempPath()) ("volicord-smoke-" + [System.Guid]::NewGuid().ToString("N"))
$repo = Join-Path $workDir "product-repo"
$home = Join-Path $workDir "runtime-home"
$binDir = Join-Path $workDir "bin"
New-Item -ItemType Directory -Force -Path (Join-Path $repo ".git") | Out-Null
New-Item -ItemType Directory -Force -Path $binDir | Out-Null

$volicordShim = Join-Path $binDir "volicord.cmd"
Set-Content -LiteralPath $volicordShim -Encoding ascii -Value "@echo off`r`n`"$command`" %*`r`n"
$codexShim = Join-Path $binDir "codex.cmd"
Set-Content -LiteralPath $codexShim -Encoding ascii -Value "@echo off`r`nif `"%1`"==`"--version`" (`r`n  echo codex 1.2.3-test`r`n  exit /b 0`r`n)`r`necho unexpected codex invocation 1>&2`r`nexit /b 2`r`n"

$oldHome = $env:VOLICORD_HOME
$oldPath = $env:PATH
$serveProcess = $null
try {
    $env:PATH = "$binDir$([System.IO.Path]::PathSeparator)$oldPath"
    Invoke-Checked -FilePath $command -Arguments @("--help")
    Invoke-Checked -FilePath $command -Arguments @("mcp", "--help")
    $env:VOLICORD_HOME = $home
    Invoke-Checked -FilePath $command -Arguments @("status", "--help")
    Invoke-Checked -FilePath $command -Arguments @("connection", "--help")
    Invoke-Checked -FilePath $command -Arguments @("inbox", "--help")
    Invoke-Checked -FilePath $command -Arguments @("serve", "--help")

    $initText = Invoke-Captured -FilePath $command -Arguments @("init", "--host", "codex", "--repo", $repo, "--profile", "record", "--json")
    $init = $initText | ConvertFrom-Json
    $connectionId = $init.connection.connection_id
    if ([string]::IsNullOrWhiteSpace($connectionId)) {
        Fail "init JSON did not include connection_id"
    }

    $stdioInput = @(
        '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"volicord-smoke","version":"0.0.0"}}}'
        '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}'
        '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
    ) -join "`n"
    $stdioInput = "$stdioInput`n"
    $stdio = Invoke-WithInput -FilePath $command -Arguments @("mcp", "--stdio", "--connection", $connectionId) -InputText $stdioInput -RuntimeHome $home -PathValue $env:PATH
    if ($stdio.Stdout -notmatch '"protocolVersion":"2025-11-25"') {
        Fail "MCP stdio did not negotiate the current protocol"
    }
    if ($stdio.Stdout -notmatch '"name":"volicord.status"') {
        Fail "MCP stdio did not list status"
    }
    if ($stdio.Stdout -notmatch '"name":"volicord.close_task"') {
        Fail "MCP stdio workflow tools did not list close_task"
    }
    if ($stdio.Stdout -match '"name":"volicord.record_user_judgment"') {
        Fail "MCP stdio exposed user-only judgment recording"
    }

    $curl = Get-Command curl.exe -ErrorAction SilentlyContinue
    if ($null -ne $curl) {
        $serveErr = Join-Path $workDir "serve.stderr"
        $serveOut = Join-Path $workDir "serve.stdout"
        $token = "volicord-smoke-token"
        $serveProcess = Start-Process -FilePath $command `
            -ArgumentList @("serve", "--transport", "local-http", "--listen", "127.0.0.1:0", "--connection", $connectionId, "--token", $token) `
            -RedirectStandardOutput $serveOut `
            -RedirectStandardError $serveErr `
            -NoNewWindow `
            -PassThru

        $listenUrl = $null
        for ($i = 0; $i -lt 100; $i++) {
            if (Test-Path -LiteralPath $serveErr) {
                $errText = Get-Content -LiteralPath $serveErr -Raw -ErrorAction SilentlyContinue
                if ($errText -match 'http://\S+/mcp') {
                    $listenUrl = $Matches[0]
                    break
                }
            }
            if ($serveProcess.HasExited) {
                $errText = if (Test-Path -LiteralPath $serveErr) { Get-Content -LiteralPath $serveErr -Raw } else { "" }
                if ($errText -match "Operation not permitted") {
                    Write-Warning "volicord smoke test skipped Local HTTP TCP checks: local bind is unavailable"
                    break
                }
                Fail "Local HTTP server exited before startup: $errText"
            }
            Start-Sleep -Milliseconds 100
        }

        if ($null -ne $listenUrl) {
            $healthUrl = $listenUrl -replace '/mcp$', '/healthz'
            $unauthBody = Join-Path $workDir "unauth.json"
            $unauthCode = & $curl.Source -sS -o $unauthBody -w "%{http_code}" $healthUrl
            if ($LASTEXITCODE -ne 0) { Fail "Local HTTP unauthenticated health request failed" }
            if ($unauthCode -ne "401") { Fail "Local HTTP health without token returned $unauthCode" }
            if ((Get-Content -LiteralPath $unauthBody -Raw) -notmatch "AUTH_REQUIRED") {
                Fail "Local HTTP unauthenticated health did not return AUTH_REQUIRED"
            }

            $healthBody = Join-Path $workDir "health.json"
            $authCode = & $curl.Source -sS -o $healthBody -w "%{http_code}" -H "Authorization: Bearer $token" $healthUrl
            if ($LASTEXITCODE -ne 0) { Fail "Local HTTP authenticated health request failed" }
            if ($authCode -ne "200") { Fail "Local HTTP health with token returned $authCode" }

            $initPayload = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"volicord-smoke","version":"0.0.0"}}}'
            $originBody = Join-Path $workDir "origin.json"
            $originCode = & $curl.Source -sS -o $originBody -w "%{http_code}" -X POST $listenUrl -H "Authorization: Bearer $token" -H "Accept: application/json, text/event-stream" -H "Content-Type: application/json" -H "Origin: https://example.invalid" --data $initPayload
            if ($LASTEXITCODE -ne 0) { Fail "Local HTTP Origin check request failed" }
            if ($originCode -ne "403") { Fail "Local HTTP invalid Origin returned $originCode" }
            if ((Get-Content -LiteralPath $originBody -Raw) -notmatch "ORIGIN_NOT_ALLOWED") {
                Fail "Local HTTP invalid Origin did not return ORIGIN_NOT_ALLOWED"
            }

            $headers = Join-Path $workDir "init.headers"
            $initBody = Join-Path $workDir "init-http.json"
            $initCode = & $curl.Source -sS -D $headers -o $initBody -w "%{http_code}" -X POST $listenUrl -H "Authorization: Bearer $token" -H "Accept: application/json, text/event-stream" -H "Content-Type: application/json" --data $initPayload
            if ($LASTEXITCODE -ne 0) { Fail "Local HTTP initialize request failed" }
            if ($initCode -ne "200") { Fail "Local HTTP initialize returned $initCode" }
            if ((Get-Content -LiteralPath $headers -Raw) -notmatch "(?im)^Mcp-Session-Id:") {
                Fail "Local HTTP initialize did not return Mcp-Session-Id"
            }

            Stop-Process -Id $serveProcess.Id -Force -ErrorAction SilentlyContinue
            $serveProcess.WaitForExit()
            $serveProcess = $null
        }
    } else {
        Write-Warning "volicord smoke test skipped Local HTTP checks: curl.exe is unavailable"
    }

    Write-Host "volicord smoke test passed for $command"
} finally {
    if ($null -ne $serveProcess -and -not $serveProcess.HasExited) {
        Stop-Process -Id $serveProcess.Id -Force -ErrorAction SilentlyContinue
        $serveProcess.WaitForExit()
    }
    $env:VOLICORD_HOME = $oldHome
    $env:PATH = $oldPath
    Remove-Item -LiteralPath $workDir -Recurse -Force -ErrorAction SilentlyContinue
}
