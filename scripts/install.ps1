[CmdletBinding()]
param(
    [string]$Repo = $env:VOLICORD_REPO,
    [string]$Version = $env:VOLICORD_VERSION,
    [string]$ReleaseBaseUrl = $env:VOLICORD_RELEASE_BASE_URL,
    [string]$InstallDir = $env:VOLICORD_INSTALL_DIR,
    [switch]$DryRun,
    [switch]$PrintTarget,
    [switch]$RequireChecksum,
    [switch]$UpdateUserPath
)

Set-StrictMode -Version 2.0
$ErrorActionPreference = "Stop"

function Fail {
    param([string]$Message)
    Write-Error "volicord install: $Message"
    exit 1
}

function Download-File {
    param(
        [string]$Url,
        [string]$Output,
        [bool]$Required
    )
    try {
        Invoke-WebRequest -Uri $Url -OutFile $Output -UseBasicParsing | Out-Null
        return $true
    } catch {
        if ($Required) {
            Fail "failed to download $Url ($($_.Exception.Message))"
        }
        return $false
    }
}

function Get-WindowsArchitecture {
    if ($env:OS -ne "Windows_NT") {
        Fail "native Windows install requires PowerShell on Windows; use scripts/install.sh in Linux, WSL2, or macOS"
    }

    try {
        $arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
    } catch {
        $arch = $env:PROCESSOR_ARCHITECTURE
    }

    return $arch
}

function Get-ArchitectureTarget {
    param([string]$Architecture)

    switch -Regex ($Architecture.ToLowerInvariant()) {
        "^(x64|amd64)$" { return "x86_64-pc-windows-msvc" }
        default {
            Fail "unsupported Windows CPU architecture: $Architecture; this installer expects x86_64-pc-windows-msvc"
        }
    }
}

function Get-OptionalReleaseBaseUrl {
    if (-not [string]::IsNullOrWhiteSpace($ReleaseBaseUrl)) {
        return $ReleaseBaseUrl.TrimEnd("/")
    }

    if ([string]::IsNullOrWhiteSpace($Repo)) {
        return $null
    }
    if ($Repo -notmatch "^[^/]+/[^/]+$") {
        Fail "VOLICORD_REPO must use OWNER/REPO form"
    }

    $selectedVersion = $Version
    if ([string]::IsNullOrWhiteSpace($selectedVersion)) {
        $selectedVersion = "latest"
    }

    if ($selectedVersion -eq "latest") {
        return "https://github.com/$Repo/releases/latest/download"
    }
    return "https://github.com/$Repo/releases/download/$selectedVersion"
}

function Get-ReleaseBaseUrl {
    $resolvedBaseUrl = Get-OptionalReleaseBaseUrl
    if ([string]::IsNullOrWhiteSpace($resolvedBaseUrl)) {
        Fail "set VOLICORD_REPO=OWNER/REPO, pass -Repo OWNER/REPO, or set VOLICORD_RELEASE_BASE_URL before running this script"
    }
    return $resolvedBaseUrl
}

function Get-InstallDirectory {
    if (-not [string]::IsNullOrWhiteSpace($InstallDir)) {
        return $InstallDir
    }
    if ([string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) {
        Fail "LOCALAPPDATA is not set; pass -InstallDir or set VOLICORD_INSTALL_DIR"
    }
    return (Join-Path $env:LOCALAPPDATA "Volicord\bin")
}

function Test-PathListContains {
    param(
        [string]$PathList,
        [string]$Directory
    )
    if ([string]::IsNullOrWhiteSpace($PathList)) {
        return $false
    }
    $expected = [System.IO.Path]::GetFullPath($Directory).TrimEnd("\")
    foreach ($entry in $PathList -split ";") {
        if ([string]::IsNullOrWhiteSpace($entry)) {
            continue
        }
        try {
            $candidate = [System.IO.Path]::GetFullPath($entry).TrimEnd("\")
        } catch {
            $candidate = $entry.TrimEnd("\")
        }
        if ([System.StringComparer]::OrdinalIgnoreCase.Equals($candidate, $expected)) {
            return $true
        }
    }
    return $false
}

if ($env:VOLICORD_REQUIRE_CHECKSUM -eq "1") {
    $RequireChecksum = $true
}
if ($env:VOLICORD_UPDATE_USER_PATH -eq "1") {
    $UpdateUserPath = $true
}

$architecture = Get-WindowsArchitecture
$target = Get-ArchitectureTarget -Architecture $architecture
if ($PrintTarget) {
    Write-Output $target
    return
}

$destDir = Get-InstallDirectory
$archiveName = "volicord-$target.zip"

if ($DryRun) {
    Write-Host "volicord install dry run"
    Write-Host "detected platform: Windows_NT/$architecture"
    Write-Host "target: $target"
    $resolvedBaseUrl = Get-OptionalReleaseBaseUrl
    if (-not [string]::IsNullOrWhiteSpace($resolvedBaseUrl)) {
        $archiveUrl = "$resolvedBaseUrl/$archiveName"
        $checksumName = "$archiveUrl.sha256"
        Write-Host "release asset URL: $archiveUrl"
    } else {
        $checksumName = "$archiveName.sha256"
        Write-Host "release asset name: $archiveName"
    }
    if ($RequireChecksum) {
        Write-Host "checksum verification: required; would download $checksumName and fail if it is unavailable, invalid, or mismatched"
    } else {
        Write-Host "checksum verification: would try $checksumName; if unavailable, installation would warn and continue"
    }
    Write-Host "install directory: $destDir"
    Write-Host "binary to install: volicord.exe"
    if ($UpdateUserPath) {
        Write-Host "PATH update: would update the user PATH if the install directory is missing"
    } else {
        Write-Host "PATH update: no persistent PATH update requested"
    }
    return
}

$baseUrl = Get-ReleaseBaseUrl
$archiveUrl = "$baseUrl/$archiveName"
$checksumUrl = "$archiveUrl.sha256"

$workDir = Join-Path ([System.IO.Path]::GetTempPath()) ("volicord-install-" + [System.Guid]::NewGuid().ToString("N"))
$extractDir = Join-Path $workDir "extract"
New-Item -ItemType Directory -Force -Path $extractDir | Out-Null

try {
    $archive = Join-Path $workDir $archiveName
    $checksum = Join-Path $workDir "$archiveName.sha256"

    Write-Host "downloading $archiveUrl"
    Download-File -Url $archiveUrl -Output $archive -Required $true | Out-Null

    $verified = $false
    if (Download-File -Url $checksumUrl -Output $checksum -Required $false) {
        $checksumText = Get-Content -LiteralPath $checksum -Raw
        if ($checksumText -notmatch "^\s*([0-9a-fA-F]{64})\b") {
            Fail "checksum file does not contain a 64-character SHA-256 digest"
        }
        $expected = $Matches[1].ToLowerInvariant()
        $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $archive).Hash.ToLowerInvariant()
        if ($actual -ne $expected) {
            Fail "checksum mismatch for $archiveName"
        }
        $verified = $true
    } elseif ($RequireChecksum) {
        Fail "checksum file is unavailable and checksum verification is required"
    } else {
        Write-Warning "checksum file unavailable; installing without checksum verification"
    }

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [System.IO.Compression.ZipFile]::OpenRead($archive)
    try {
        $entries = @($zip.Entries)
        if ($entries.Count -ne 1 -or $entries[0].FullName -ne "volicord.exe") {
            Fail "archive must contain only volicord.exe"
        }
    } finally {
        $zip.Dispose()
    }

    Expand-Archive -LiteralPath $archive -DestinationPath $extractDir -Force
    $extracted = Join-Path $extractDir "volicord.exe"
    if (-not (Test-Path -LiteralPath $extracted -PathType Leaf)) {
        Fail "archive did not extract a volicord.exe executable"
    }

    New-Item -ItemType Directory -Force -Path $destDir | Out-Null
    $installed = Join-Path $destDir "volicord.exe"
    Copy-Item -LiteralPath $extracted -Destination $installed -Force

    Write-Host "installed $installed"
    if ($verified) {
        Write-Host "verified $archiveName with SHA-256 checksum"
    }

    $processHasPath = Test-PathListContains -PathList $env:Path -Directory $destDir
    if ($UpdateUserPath) {
        $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
        if (-not (Test-PathListContains -PathList $userPath -Directory $destDir)) {
            if ([string]::IsNullOrWhiteSpace($userPath)) {
                $nextUserPath = $destDir
            } else {
                $nextUserPath = "$userPath;$destDir"
            }
            [Environment]::SetEnvironmentVariable("Path", $nextUserPath, "User")
            Write-Host "updated user PATH with $destDir"
        }
        if (-not $processHasPath) {
            $env:Path = "$destDir;$env:Path"
            $processHasPath = $true
        }
    } elseif (-not $processHasPath) {
        $escaped = $destDir.Replace("'", "''")
        Write-Warning "$destDir is not on PATH for this PowerShell session"
        Write-Host "For this session, run:"
        Write-Host "  `$env:Path = '$escaped;' + `$env:Path"
        Write-Host "For persistent user PATH, rerun with -UpdateUserPath or update the user PATH through Windows settings."
    }

    & $installed --version
    if ($processHasPath) {
        Write-Host "Next check: volicord doctor"
    } else {
        Write-Host "Next check after PATH update: volicord doctor"
        Write-Host "Without PATH update, run: & '$installed' doctor"
    }
} finally {
    Remove-Item -LiteralPath $workDir -Recurse -Force -ErrorAction SilentlyContinue
}
