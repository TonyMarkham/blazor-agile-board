# install.ps1 - Install Blazor Agile Board tools into <repo>\.pm\bin\
#
# Usage:
#   irm https://raw.githubusercontent.com/TonyMarkham/blazor-agile-board/main/install.ps1 | iex
#   # or
#   .\install.ps1
#
# Environment variables:
#   PM_VERSION     - Specific version (default: latest)
#   PM_REPO        - GitHub repo (default: TonyMarkham/blazor-agile-board)

$ErrorActionPreference = "Stop"

# =========================================================================
# Configuration
# =========================================================================

$Repo = if ($env:PM_REPO) { $env:PM_REPO } else { "TonyMarkham/blazor-agile-board" }

# =========================================================================
# Output Helpers
# =========================================================================

function Write-Info($msg) { Write-Host "[info]  $msg" -ForegroundColor Blue }
function Write-Ok($msg) { Write-Host "[ok]    $msg" -ForegroundColor Green }
function Write-Err($msg) { Write-Host "[error] $msg" -ForegroundColor Red; exit 1 }

# =========================================================================
# Repository Root Detection
# =========================================================================

$RepoRoot = & git rev-parse --show-toplevel 2>$null
if (-not $RepoRoot) {
    Write-Err "Not inside a git repository. Run this from within a repo."
}

$InstallDir = Join-Path $RepoRoot ".pm\bin"

# =========================================================================
# Main
# =========================================================================

Write-Info "Blazor Agile Board Installer"
Write-Host ""
Write-Info "Repository: $RepoRoot"

# Detect architecture
$Arch = switch ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture) {
    "X64"   { "x86_64" }
    "Arm64" { "aarch64" }
    default { Write-Err "Unsupported architecture: $_" }
}

$Target = "${Arch}-pc-windows-msvc"
Write-Info "Platform: $Target"

# Determine version
$Version = $env:PM_VERSION
if (-not $Version) {
    Write-Info "Detecting latest version..."
    try {
        $Release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
        $Version = $Release.tag_name
    }
    catch {
        Write-Err "Could not determine latest version. Set PM_VERSION manually."
    }
}
Write-Info "Version: $Version"

# Build download URL
$VerNum = $Version -replace '^v', ''
$ArchiveName = "pm-${VerNum}-${Target}"
$ArchiveFile = "${ArchiveName}.tar.gz"
$Url = "https://github.com/$Repo/releases/download/$Version/$ArchiveFile"

# Create install directory
New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null

# Download to temp directory
Write-Info "Downloading $ArchiveFile..."
$TmpDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $TmpDir -Force | Out-Null

try {
    $ArchivePath = Join-Path $TmpDir $ArchiveFile
    Invoke-WebRequest -Uri $Url -OutFile $ArchivePath

    # Extract archive (tar is available on Windows 10+)
    Write-Info "Extracting to $InstallDir\..."
    tar xzf $ArchivePath -C $TmpDir

    # Copy binaries
    $BinDir = Join-Path $TmpDir $ArchiveName "bin"
    if (Test-Path $BinDir) {
        Copy-Item "$BinDir\*" $InstallDir -Recurse -Force
    }
    else {
        Write-Err "Archive missing expected bin\ directory"
    }

    # Install wrapper script to repo root
    $WrapperScript = Join-Path $TmpDir $ArchiveName "pm.bat"
    if (Test-Path $WrapperScript) {
        Write-Info "Installing wrapper script to repository root..."
        Copy-Item $WrapperScript (Join-Path $RepoRoot "pm.bat") -Force
        Write-Ok "Installed $(Join-Path $RepoRoot 'pm.bat')"
    }
}
finally {
    # Clean up temp directory
    Remove-Item $TmpDir -Recurse -Force -ErrorAction SilentlyContinue
}

# Write config.json for Tauri double-click support
Write-Info "Writing config.json (repo_root for Tauri)..."
$ConfigJson = @{ repo_root = $RepoRoot } | ConvertTo-Json -Compress
Set-Content (Join-Path $InstallDir "config.json") $ConfigJson -Encoding utf8
Write-Ok "Created $(Join-Path $InstallDir 'config.json')"

# Create .pm\.gitignore if it doesn't exist
$GitignorePath = Join-Path $RepoRoot ".pm\.gitignore"
if (-not (Test-Path $GitignorePath)) {
    Write-Info "Creating .pm\.gitignore..."
    @"
# Runtime files - not tracked
# data.json and config.toml ARE tracked (not listed here = tracked).

# SQLite database (local performance — use data.json for git sync)
data.db

# Negate root .gitignore's *.json pattern if one exists
!data.json

bin/
*.db-wal
*.db-shm
server.json
server.lock
logs/
log/
tauri/
"@ | Set-Content $GitignorePath -Encoding utf8
    Write-Ok "Created $GitignorePath"
}

# Print success
Write-Host ""
Write-Ok "Installation complete!"
Write-Host ""
Write-Host "  Installed to: $InstallDir"
Write-Host ""
Write-Host "  Usage:"
Write-Host "    .pm\bin\pm project list --pretty"
Write-Host "    .pm\bin\pm desktop"
Write-Host ""
Write-Host "  Add to PATH (current session):"
Write-Host "    `$env:PATH = `"$RepoRoot\.pm\bin;`$env:PATH`""
Write-Host ""
