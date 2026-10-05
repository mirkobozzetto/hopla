# Installs the latest hopla on Windows, in PowerShell:
#   irm https://raw.githubusercontent.com/mirkobozzetto/hopla/main/install.ps1 | iex
$ErrorActionPreference = 'Stop'
$repo = 'mirkobozzetto/hopla'
$name = 'hopla-x86_64-pc-windows-msvc'
$dir = Join-Path $env:LOCALAPPDATA 'hopla'
$zip = Join-Path $env:TEMP "$name.zip"

Invoke-WebRequest "https://github.com/$repo/releases/latest/download/$name.zip" -OutFile $zip
Expand-Archive $zip -DestinationPath $env:TEMP -Force
New-Item -ItemType Directory -Force $dir | Out-Null
Copy-Item (Join-Path $env:TEMP "$name\hopla.exe") $dir -Force

$path = [Environment]::GetEnvironmentVariable('Path', 'User')
if (-not (($path -split ';') -contains $dir)) {
  [Environment]::SetEnvironmentVariable('Path', "$path;$dir", 'User')
  Write-Host "Added $dir to your PATH: open a new terminal."
}
Write-Host 'hopla is installed. Run: hopla'
