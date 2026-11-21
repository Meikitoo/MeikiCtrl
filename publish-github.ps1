# Publie MeikiCtrl dans un dépôt GitHub PRIVÉ (compte Meikitoo).
# Usage (PowerShell, depuis ce dossier) :
#   powershell -ExecutionPolicy Bypass -File .\publish-github.ps1

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot
$repo = "MeikiCtrl"

if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    throw "Git n'est pas installé : https://git-scm.com/download/win"
}

if (-not (Test-Path ".git")) {
    git init -b main | Out-Null
    Write-Host "Dépôt Git créé." -ForegroundColor Cyan
}
if (-not (git config user.name)) { git config user.name "Meikitoo" }
if (-not (git config user.email)) { git config user.email "Meikitoo@users.noreply.github.com" }

$version = (Get-Content "tauri.conf.json" -Raw | ConvertFrom-Json).version
git add -A
git diff --cached --quiet
if ($LASTEXITCODE -ne 0) {
    git commit -m "MeikiCtrl v$version" | Out-Null
    Write-Host "Commit : MeikiCtrl v$version" -ForegroundColor Cyan
}

$hasRemote = (git remote) -contains "origin"
if (Get-Command gh -ErrorAction SilentlyContinue) {
    gh auth status 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { gh auth login }
    if (-not $hasRemote) {
        gh repo create $repo --private --source . --remote origin --description "MeikiCtrl - overlay manette 3D, émulation Xbox et réglages, by Meikitoow" --push
    } else {
        git push -u origin main
    }
} else {
    if (-not $hasRemote) {
        Write-Host ""
        Write-Host "GitHub CLI (gh) absent. Deux options :" -ForegroundColor Yellow
        Write-Host "  1. Installe-le (winget install GitHub.cli) puis relance ce script."
        Write-Host "  2. Crée le dépôt à la main sur https://github.com/new :"
        Write-Host "     nom '$repo', visibilité PRIVATE, sans README, puis relance ce script."
        $url = Read-Host "URL du dépôt créé (ex. https://github.com/Meikitoo/MeikiCtrl.git), ou Entrée pour annuler"
        if (-not $url) { exit 1 }
        git remote add origin $url
    }
    git push -u origin main
}
if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "Publié en privé : https://github.com/Meikitoo/$repo" -ForegroundColor Green
}
