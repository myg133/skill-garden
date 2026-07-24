<#
.SYNOPSIS
    Build Skill Garden CLI binaries and copy to data/cli/ for distribution.

.DESCRIPTION
    Cross-compiles the CLI binary for all supported targets and places them
    in the directory structure expected by the server's cli.setup MCP tool.

    Output structure: data/cli/{version}/{os}-{arch}/skill-garden(.exe)

.PARAMETER Targets
    Comma-separated list of targets to build. Default: all.
    Values: windows-x86_64, windows-aarch64, linux-x86_64, linux-aarch64, macos-x86_64, macos-aarch64

.PARAMETER DataDir
    Data directory root. Default: cli-dist

.EXAMPLE
    .\deploy\build-cli.ps1
    Build for native target only (current OS + arch), copy to data/cli/.

.EXAMPLE
    .\deploy\build-cli.ps1 -Targets "windows-x86_64,linux-x86_64"
    Build for two specific targets.
#>

param(
    [string]$Targets = "",
    [string]$DataDir = "cli-dist"
)

$ErrorActionPreference = "Stop"

# Read version from Cargo.toml
$cargoToml = Get-Content "Cargo.toml" -Raw
$versionMatch = [regex]::Match($cargoToml, 'version\s*=\s*"([^"]+)"')
if (-not $versionMatch.Success) {
    Write-Error "Cannot parse version from Cargo.toml"
    exit 1
}
$version = $versionMatch.Groups[1].Value
Write-Host "=== Building Skill Garden CLI v$version ===" -ForegroundColor Cyan

# Target definitions: name -> (rust-triple, binary-name)
$targetMap = @{
    "windows-x86_64"  = @{ triple = "x86_64-pc-windows-msvc";      bin = "skill-garden.exe" }
    "windows-aarch64" = @{ triple = "aarch64-pc-windows-msvc";     bin = "skill-garden.exe" }
    "linux-x86_64"    = @{ triple = "x86_64-unknown-linux-gnu";    bin = "skill-garden" }
    "linux-aarch64"   = @{ triple = "aarch64-unknown-linux-gnu";   bin = "skill-garden" }
    "macos-x86_64"    = @{ triple = "x86_64-apple-darwin";         bin = "skill-garden" }
    "macos-aarch64"   = @{ triple = "aarch64-apple-darwin";        bin = "skill-garden" }
}

# Determine which targets to build
if ($Targets) {
    $buildTargets = $Targets -split "," | ForEach-Object { $_.Trim() }
}
else {
    # Default: build for current OS native targets
    $buildTargets = @()
    if ($IsWindows -or $env:OS -eq "Windows_NT") {
        $buildTargets += "windows-x86_64"
        # aarch64 only if toolchain available
        if (rustup target list --installed | Select-String "aarch64-pc-windows-msvc") {
            $buildTargets += "windows-aarch64"
        }
    }
    elseif ($IsLinux) {
        $buildTargets += "linux-x86_64"
        if (rustup target list --installed | Select-String "aarch64-unknown-linux-gnu") {
            $buildTargets += "linux-aarch64"
        }
    }
    elseif ($IsMacOS) {
        $buildTargets += "macos-x86_64"
        if (rustup target list --installed | Select-String "aarch64-apple-darwin") {
            $buildTargets += "macos-aarch64"
        }
    }
}

Write-Host "Targets to build: $($buildTargets -join ', ')" -ForegroundColor Yellow
Write-Host ""

$success = @()
$failed = @()
$skipped = @()

foreach ($name in $buildTargets) {
    if (-not $targetMap.ContainsKey($name)) {
        Write-Host "  SKIP: Unknown target '$name'" -ForegroundColor Gray
        $skipped += $name
        continue
    }

    $t = $targetMap[$name]
    $triple = $t.triple
    $bin = $t.bin

    Write-Host "--- Building: $name ($triple) ---" -ForegroundColor White

    # Install rust target if needed
    $installed = rustup target list --installed | Select-String $triple -SimpleMatch
    if (-not $installed) {
        Write-Host "  Installing target: $triple..."
        rustup target add $triple
        if ($LASTEXITCODE -ne 0) {
            Write-Host "  FAIL: Could not install target $triple" -ForegroundColor Red
            $failed += $name
            continue
        }
    }

    # Build (--no-default-features to skip server deps: sqlx, tantivy, axum, etc.)
    Write-Host "  Compiling..."
    $prevErrorAction = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $buildResult = cargo build --release --no-default-features --features cli --target $triple 2>&1
    $ErrorActionPreference = $prevErrorAction
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  FAIL: Build error for $name" -ForegroundColor Red
        Write-Host ($buildResult -join "`n")
        $failed += $name
        continue
    }

    # Copy to cli-dist/
    $src = "target/$triple/release/$bin"
    $destDir = "$DataDir/$version/$name"
    New-Item -ItemType Directory -Force -Path $destDir | Out-Null
    Copy-Item $src -Destination "$destDir/$bin" -Force

    $fileSize = (Get-Item "$destDir/$bin").Length
    $sizeMB = [math]::Round($fileSize / 1MB, 2)
    Write-Host "  OK -> $destDir/$bin ($sizeMB MB)" -ForegroundColor Green
    $success += $name
}

Write-Host ""
Write-Host "=== Build Summary ===" -ForegroundColor Cyan
Write-Host "  Success : $($success.Count) [$($success -join ', ')]" -ForegroundColor Green
if ($failed.Count -gt 0) {
    Write-Host "  Failed  : $($failed.Count) [$($failed -join ', ')]" -ForegroundColor Red
}
if ($skipped.Count -gt 0) {
    Write-Host "  Skipped : $($skipped.Count) [$($skipped -join ', ')]" -ForegroundColor Gray
}

if ($success.Count -eq 0) {
    Write-Host ""
    Write-Host "No binaries built. To add targets for cross-compilation:" -ForegroundColor Yellow
    Write-Host "  rustup target add x86_64-unknown-linux-gnu"
    Write-Host "  rustup target add x86_64-apple-darwin"
    exit 1
}

Write-Host ""
Write-Host "CLI binaries ready in: $DataDir/$version/" -ForegroundColor Green
Write-Host "Directory tree:"
Get-ChildItem -Recurse -File "$DataDir/$version/" | ForEach-Object {
    $relPath = $_.FullName.Replace((Resolve-Path $DataDir).Path + "\", "")
    $sizeMB = [math]::Round($_.Length / 1MB, 2)
    Write-Host "  $relPath ($sizeMB MB)"
}
