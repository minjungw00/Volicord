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
New-Item -ItemType Directory -Force -Path $repo | Out-Null

$oldHome = $env:VOLICORD_HOME
try {
    git init -q $repo
    Invoke-Checked -FilePath $command -Arguments @("--help")
    Invoke-Checked -FilePath $command -Arguments @("mcp", "--help")
    $env:VOLICORD_HOME = $home
    Invoke-Checked -FilePath $command -Arguments @("init", "--host", "codex", "--repo", $repo, "--dry-run", "--json")
    $env:VOLICORD_HOME = $oldHome
    Invoke-Checked -FilePath $command -Arguments @("guard", "--help")
    Invoke-Checked -FilePath $command -Arguments @("serve", "--help")
    Write-Host "volicord smoke test passed for $command"
} finally {
    $env:VOLICORD_HOME = $oldHome
    Remove-Item -LiteralPath $workDir -Recurse -Force -ErrorAction SilentlyContinue
}
