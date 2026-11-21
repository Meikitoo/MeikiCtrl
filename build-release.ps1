# Construit l'installateur de MeikiCtrl (+ une version portable).
# Usage (PowerShell, depuis ce dossier) :
#   powershell -ExecutionPolicy Bypass -File .\build-release.ps1

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

# Outil Tauri (crée l'installateur). Installé une seule fois.
$hasTauri = (cargo --list) -match "^\s+tauri\b"
if (-not $hasTauri) {
    Write-Host "Installation de l'outil Tauri (une seule fois, quelques minutes)..." -ForegroundColor Cyan
    cargo install tauri-cli --version "^2" --locked
    if ($LASTEXITCODE -ne 0) { throw "Installation de tauri-cli impossible." }
}

Write-Host "Compilation de MeikiCtrl + création de l'installateur..." -ForegroundColor Cyan
cargo tauri build
if ($LASTEXITCODE -ne 0) { throw "La compilation a échoué." }

$version = (Get-Content "tauri.conf.json" -Raw | ConvertFrom-Json).version
$dist = Join-Path $PSScriptRoot "dist"
New-Item -ItemType Directory -Force $dist | Out-Null

$setup = Get-ChildItem "target\release\bundle\nsis\*-setup.exe" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $setup) { throw "Installateur introuvable dans target\release\bundle\nsis." }
$setupOut = Join-Path $dist "MeikiCtrl-Setup-$version.exe"
Copy-Item $setup.FullName $setupOut -Force
$portable = Join-Path $dist "MeikiCtrl-portable.exe"
Copy-Item "target\release\meikictrl.exe" $portable -Force

# Nettoyage de l'ancienne version portable (v0.4 sans installateur) et de son raccourci.
$old = Join-Path $dist "MeikiCtrl.exe"
if (Test-Path $old) { Remove-Item $old -Force }
$oldLnk = Join-Path ([Environment]::GetFolderPath("Desktop")) "MeikiCtrl.lnk"
if ((Test-Path $oldLnk) -and ((New-Object -ComObject WScript.Shell).CreateShortcut($oldLnk).TargetPath -like "*\dist\*")) {
    Remove-Item $oldLnk -Force
}

$size = [math]::Round((Get-Item $setupOut).Length / 1MB, 1)
Write-Host ""
Write-Host "C'est prêt :" -ForegroundColor Green
Write-Host "  Installateur : $setupOut ($size Mo)"
Write-Host "  Portable     : $portable"
Write-Host "Lance l'installateur, puis ouvre MeikiCtrl depuis le menu Démarrer ou le bureau."
