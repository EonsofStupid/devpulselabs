# Pull the rolling GitHub Release tag `nightly` for Windows x64.
# Requires: gh (authenticated).
$ErrorActionPreference = "Stop"

$Repo = if ($env:DPL_NIGHTLY_REPO) { $env:DPL_NIGHTLY_REPO } else { "EonsofStupid/devpulselabs" }
$DestRoot = if ($env:DPL_NIGHTLY_DIR) { $env:DPL_NIGHTLY_DIR } else {
    Join-Path $env:USERPROFILE "Downloads\devpulse-nightly"
}
$Date = (Get-Date).ToUniversalTime().ToString("yyyy-MM-dd")
$Pattern = "devpulselabs-windows-x64.zip"
$Out = Join-Path $DestRoot $Date
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$Tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("dpl-nightly-" + [guid]::NewGuid().ToString("n"))
New-Item -ItemType Directory -Force -Path $Tmp | Out-Null

try {
    Write-Host "Downloading $Pattern from $Repo @ nightly → $Out"
    gh release download nightly -R $Repo -p $Pattern -D $Tmp --clobber
    Expand-Archive -Path (Join-Path $Tmp $Pattern) -DestinationPath $Out -Force
    Write-Host "Ready: $Out"
    Get-ChildItem $Out
}
finally {
    Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}
