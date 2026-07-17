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
try {
    $env:PATH = "$binDir$([System.IO.Path]::PathSeparator)$oldPath"
    Invoke-Checked -FilePath $command -Arguments @("--help")
    Invoke-Checked -FilePath $command -Arguments @("mcp", "--help")
    $env:VOLICORD_HOME = $home
    Invoke-Checked -FilePath $command -Arguments @("status", "--help")
    Invoke-Checked -FilePath $command -Arguments @("connection", "--help")
    Invoke-Checked -FilePath $command -Arguments @("inbox", "--help")

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
    if ($stdio.Stdout -notmatch '"name":"volicord.request_user_action"') {
        Fail "MCP stdio did not list user-action request creation"
    }
    if ($stdio.Stdout -match '"name":"volicord.resolve_user_action"') {
        Fail "MCP stdio exposed user-only action resolution"
    }

    Write-Host "volicord smoke test passed for $command"
} finally {
    $env:VOLICORD_HOME = $oldHome
    $env:PATH = $oldPath
    Remove-Item -LiteralPath $workDir -Recurse -Force -ErrorAction SilentlyContinue
}
