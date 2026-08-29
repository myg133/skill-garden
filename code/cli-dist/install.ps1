# Skill Garden CLI Installer
$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$CliName = "skill-garden"

$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if ($isAdmin) {
    $targetDir = "C:\Program Files\skill-garden"
    if (-not (Test-Path $targetDir)) { New-Item -ItemType Directory -Path $targetDir -Force | Out-Null }
    Copy-Item -Path "$ScriptDir\$CliName.exe" -Destination "$targetDir\$CliName.exe" -Force

    $oldPath = [Environment]::GetEnvironmentVariable("Path", "Machine")
    if ($oldPath -notlike "*$targetDir*") {
        [Environment]::SetEnvironmentVariable("Path", "$oldPath;$targetDir", "Machine")
    }
} else {
    $targetDir = "$env:USERPROFILE\.skill-garden\bin"
    if (-not (Test-Path $targetDir)) { New-Item -ItemType Directory -Path $targetDir -Force | Out-Null }
    Copy-Item -Path "$ScriptDir\$CliName.exe" -Destination "$targetDir\$CliName.exe" -Force

    $oldPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($oldPath -notlike "*$targetDir*") {
        [Environment]::SetEnvironmentVariable("Path", "$oldPath;$targetDir", "User")
    }
}

$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
Write-Host "skill-garden installed and added to PATH"
