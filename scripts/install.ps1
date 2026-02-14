# PowerShell installation script for gdscript-formatter-mcp
# Usage: .\install.ps1 [command] [options]

param(
    [Parameter(Position = 0)]
    [string]$Command = "install",

    [Parameter(Position = 1)]
    [string]$Arg1,

    [switch]$FromSource,
    [string]$Version,
    [switch]$All,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

$BinaryName = "gdscript-formatter-mcp"
$InstallRoot = if ($env:MCP_INSTALL_ROOT) { $env:MCP_INSTALL_ROOT } else { Join-Path $env:LOCALAPPDATA "mcp\$BinaryName" }
$BinDir = if ($env:MCP_BIN_DIR) { $env:MCP_BIN_DIR } else { Join-Path $env:LOCALAPPDATA "bin" }
$PublicLink = Join-Path $BinDir "$BinaryName.exe"
$CurrentLink = Join-Path $InstallRoot "current"
$ProtocolVersion = "2024-11-05"

$GitHubRepo = "poyu0692/gdscript-formatter-mcp"
$ReleasesUrl = "https://github.com/$GitHubRepo/releases"

function Show-Usage {
    @"
Usage: .\install.ps1 <command> [args]

Commands:
  install [options]   Install from prebuilt binary (default) or source.
    -FromSource       Build from source instead of downloading prebuilt binary
    -Version VERSION  Install specific version (default: latest release)
  link [version]      Re-point links to installed version.
  uninstall [version] Uninstall one version.
    -All              Remove all installed versions and links.
  status              Show install state and active links.
  doctor              Verify executable exists and MCP handshake/tools list work.
  help                Show this help message.
"@
}

function Get-PackageVersion {
    $cargoToml = Join-Path $PSScriptRoot "..\Cargo.toml"
    $content = Get-Content $cargoToml -Raw
    if ($content -match 'version\s*=\s*"([^"]+)"') {
        return $Matches[1]
    }
    throw "Could not find version in Cargo.toml"
}

function Get-LatestVersion {
    try {
        $response = Invoke-RestMethod -Uri "https://api.github.com/repos/$GitHubRepo/releases/latest"
        return $response.tag_name
    } catch {
        throw "Failed to fetch latest version: $_"
    }
}

function Download-AndInstall {
    param(
        [string]$VersionTag,
        [string]$Target
    )

    # Remove 'v' prefix if present
    $CleanVersion = $VersionTag -replace '^v', ''
    $VersionDir = Join-Path $InstallRoot $CleanVersion
    $TargetBin = Join-Path $VersionDir "$BinaryName.exe"

    if (Test-Path $TargetBin) {
        Write-Host "Version $CleanVersion is already installed at $TargetBin"
        New-Item -ItemType SymbolicLink -Path $CurrentLink -Target $VersionDir -Force | Out-Null
        New-Item -ItemType SymbolicLink -Path $PublicLink -Target (Join-Path $CurrentLink "$BinaryName.exe") -Force | Out-Null
        Write-Host "Active command: $PublicLink"
        return
    }

    $DownloadUrl = "$ReleasesUrl/download/$VersionTag/$BinaryName-$Target.zip"
    $TempDir = Join-Path $env:TEMP ([System.IO.Path]::GetRandomFileName())
    New-Item -ItemType Directory -Path $TempDir | Out-Null

    try {
        Write-Host "Downloading from: $DownloadUrl"
        $ZipPath = Join-Path $TempDir "archive.zip"
        Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipPath

        Write-Host "Extracting archive..."
        Expand-Archive -Path $ZipPath -DestinationPath $TempDir -Force

        New-Item -ItemType Directory -Path $VersionDir -Force | Out-Null
        New-Item -ItemType Directory -Path $BinDir -Force | Out-Null

        Copy-Item -Path (Join-Path $TempDir "$BinaryName.exe") -Destination $TargetBin -Force

        New-Item -ItemType SymbolicLink -Path $CurrentLink -Target $VersionDir -Force | Out-Null
        New-Item -ItemType SymbolicLink -Path $PublicLink -Target (Join-Path $CurrentLink "$BinaryName.exe") -Force | Out-Null

        Write-Host "Installed: $TargetBin"
        Write-Host "Active command: $PublicLink"
    } finally {
        Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Install-FromSource {
    $Version = Get-PackageVersion
    $VersionDir = Join-Path $InstallRoot $Version
    $RepoRoot = Join-Path $PSScriptRoot ".."
    $SourceBin = Join-Path $RepoRoot "target\release\$BinaryName.exe"
    $TargetBin = Join-Path $VersionDir "$BinaryName.exe"

    Write-Host "Building release binary from source..."
    Push-Location $RepoRoot
    try {
        cargo build --release
    } finally {
        Pop-Location
    }

    New-Item -ItemType Directory -Path $VersionDir -Force | Out-Null
    New-Item -ItemType Directory -Path $BinDir -Force | Out-Null

    Copy-Item -Path $SourceBin -Destination $TargetBin -Force

    New-Item -ItemType SymbolicLink -Path $CurrentLink -Target $VersionDir -Force | Out-Null
    New-Item -ItemType SymbolicLink -Path $PublicLink -Target (Join-Path $CurrentLink "$BinaryName.exe") -Force | Out-Null

    Write-Host "Installed: $TargetBin"
    Write-Host "Active command: $PublicLink"
}

function Invoke-Install {
    if ($FromSource) {
        Install-FromSource
        return
    }

    $Target = "x86_64-pc-windows-msvc"

    $VersionTag = if ($Version) { $Version } else {
        Write-Host "Fetching latest release version..."
        Get-LatestVersion
    }

    Write-Host "Installing version: $VersionTag"
    Write-Host "Platform: $Target"

    Download-AndInstall -VersionTag $VersionTag -Target $Target
}

function Invoke-Link {
    $VersionToLink = if ($Arg1) { $Arg1 } else { Get-PackageVersion }
    $TargetBin = Join-Path $InstallRoot "$VersionToLink\$BinaryName.exe"

    if (-not (Test-Path $TargetBin)) {
        throw "Not installed: $TargetBin"
    }

    New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    New-Item -ItemType SymbolicLink -Path $CurrentLink -Target (Join-Path $InstallRoot $VersionToLink) -Force | Out-Null
    New-Item -ItemType SymbolicLink -Path $PublicLink -Target (Join-Path $CurrentLink "$BinaryName.exe") -Force | Out-Null

    Write-Host "Linked to version: $VersionToLink"
    Write-Host "Active command: $PublicLink"
}

function Invoke-Uninstall {
    if ($All) {
        Remove-Item -Path $InstallRoot -Recurse -Force -ErrorAction SilentlyContinue
        Remove-Item -Path $PublicLink -Force -ErrorAction SilentlyContinue
        Write-Host "Removed all installations under $InstallRoot"
        return
    }

    $VersionToRemove = if ($Arg1) { $Arg1 } else { Get-PackageVersion }
    $TargetDir = Join-Path $InstallRoot $VersionToRemove

    if (Test-Path $TargetDir) {
        Remove-Item -Path $TargetDir -Recurse -Force
        Write-Host "Removed: $TargetDir"
    } else {
        Write-Host "Version not found: $VersionToRemove"
    }

    if (Test-Path $CurrentLink) {
        $CurrentTarget = (Get-Item $CurrentLink).Target
        if ($CurrentTarget -eq $TargetDir) {
            Remove-Item -Path $CurrentLink -Force
        }
    }

    if ((Test-Path $PublicLink) -and -not (Test-Path (Get-Item $PublicLink).Target)) {
        Remove-Item -Path $PublicLink -Force
    }
}

function Invoke-Status {
    Write-Host "Install root: $InstallRoot"
    Write-Host "Binary link:  $PublicLink"

    if (Test-Path $PublicLink) {
        if ((Get-Item $PublicLink).LinkType -eq "SymbolicLink") {
            Write-Host "Link target: $((Get-Item $PublicLink).Target)"
        } else {
            Write-Host "Link target: (regular executable)"
        }
    } else {
        Write-Host "Link target: (missing)"
    }

    if (Test-Path $CurrentLink) {
        Write-Host "Current dir: $((Get-Item $CurrentLink).Target)"
    } else {
        Write-Host "Current dir: (missing)"
    }

    Write-Host "Installed versions:"
    if (Test-Path $InstallRoot) {
        Get-ChildItem -Path $InstallRoot -Directory |
            Where-Object { $_.Name -ne "current" } |
            Select-Object -ExpandProperty Name |
            Sort-Object
    }
}

function Invoke-Doctor {
    $Exe = $PublicLink
    if (-not (Test-Path $Exe)) {
        throw "Executable not found: $Exe"
    }

    $InitMsg = @{
        jsonrpc = "2.0"
        id = 1
        method = "initialize"
        params = @{
            protocolVersion = $ProtocolVersion
            capabilities = @{}
            clientInfo = @{
                name = "doctor"
                version = "0.0.0"
            }
        }
    } | ConvertTo-Json -Compress

    $ListMsg = @{
        jsonrpc = "2.0"
        id = 2
        method = "tools/list"
    } | ConvertTo-Json -Compress

    function Make-McpMessage {
        param([string]$Json)
        $length = [System.Text.Encoding]::UTF8.GetByteCount($Json)
        "Content-Length: $length`r`n`r`n$Json"
    }

    $Input = (Make-McpMessage $InitMsg) + (Make-McpMessage $ListMsg)
    $Output = $Input | & $Exe

    if ($Output -notmatch '"gdscript_format"') {
        throw "Doctor failed: gdscript_format tool not found in tools/list output."
    }
    if ($Output -notmatch '"gdscript_lint"') {
        throw "Doctor failed: gdscript_lint tool not found in tools/list output."
    }

    Write-Host "Doctor OK"
}

# Main execution
try {
    if ($Help) {
        Show-Usage
        exit 0
    }

    switch ($Command.ToLower()) {
        "install" { Invoke-Install }
        "link" { Invoke-Link }
        "uninstall" { Invoke-Uninstall }
        "status" { Invoke-Status }
        "doctor" { Invoke-Doctor }
        "help" { Show-Usage }
        default {
            Write-Host "Unknown command: $Command"
            Show-Usage
            exit 1
        }
    }
} catch {
    Write-Host "Error: $_" -ForegroundColor Red
    exit 1
}
