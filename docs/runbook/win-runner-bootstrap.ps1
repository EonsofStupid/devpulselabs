# win-runner-bootstrap.ps1 -- blank Windows x64 -> registered dpl-win runner.
#
# Idempotent: every step checks "already satisfied" before acting, every
# installer's REAL exit code is enforced, and the script never reports a
# state it did not verify (the lesson of the 2026-07-28 provisioning: a
# done-marker written without exit-code checks cost an hour of false
# "done"). Run as SYSTEM or an elevated admin. Windows PowerShell 5.1.
#
# Usage (fresh registration token, minted seconds before):
#   $t = gh api -X POST repos/EonsofStupid/devpulselabs/actions/runners/registration-token --jq .token
#   powershell -ExecutionPolicy Bypass -File win-runner-bootstrap.ps1 -Token $t
#
#   -VerifyOnly   audit every requirement and the service, install nothing
#                 (the heartbeat / post-restart check; needs no token).
#
# Version pins are the PROVEN set (green on devpulse-runner-win-01,
# 2026-07-28). Bumping a pin is a reviewed diff, not a drive-by.

[CmdletBinding()]
param(
    [string]$RepoUrl = "https://github.com/EonsofStupid/devpulselabs",
    [string]$Token = "",
    [string]$RunnerName = "$env:COMPUTERNAME".ToLower(),
    [string]$Labels = "dpl-win",
    [string]$RunnerRoot = "C:\actions-runner",
    [switch]$VerifyOnly
)

$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

# -- pins ----------------------------------------------------------------
$GitUrl   = 'https://github.com/git-for-windows/git/releases/download/v2.55.0.windows.3/Git-2.55.0.3-64-bit.exe'
$RustVer  = '1.92.0'   # matches rust-toolchain.toml; default profile = clippy + rustfmt
$LlvmUrl  = 'https://github.com/llvm/llvm-project/releases/download/llvmorg-18.1.8/LLVM-18.1.8-win64.exe'
$VsUrl    = 'https://aka.ms/vs/17/release/vs_BuildTools.exe'
$CargoBin = 'C:\Windows\System32\config\systemprofile\.cargo\bin'
$Stage    = 'C:\bootstrap'

$failures = @()

function Step([string]$m) { Write-Host "== $m" }
function Skip([string]$m) { Write-Host "   SKIP (already satisfied): $m" }
function Fail([string]$m) { $script:failures += $m; Write-Host "   FAIL: $m" }

function Invoke-Installer([string]$exe, [string[]]$Arguments, [int[]]$Ok = @(0)) {
    $p = Start-Process -FilePath $exe -ArgumentList $Arguments -Wait -PassThru -NoNewWindow
    if ($Ok -notcontains $p.ExitCode) {
        throw "$exe exited $($p.ExitCode) (accepted: $($Ok -join ',')). Installer logs: C:\Windows\SystemTemp (SYSTEM) or %TEMP%."
    }
}

function Get-File([string]$url, [string]$dest) {
    if (-not (Test-Path $Stage)) { New-Item -ItemType Directory -Force $Stage | Out-Null }
    (New-Object Net.WebClient).DownloadFile($url, $dest)
    if (-not (Test-Path $dest) -or (Get-Item $dest).Length -lt 1MB) { throw "download looks wrong: $dest" }
}

# -- requirement checks (shared by install + verify) ---------------------
function Test-Git   { try { (& "$env:ProgramFiles\Git\cmd\git.exe" --version) -match 'git version' } catch { $false } }
function Test-Cargo { Test-Path "$CargoBin\cargo.exe" }
function Test-Msvc  {
    $vc = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC"
    (Test-Path $vc) -and @(Get-ChildItem $vc -Directory).Count -gt 0
}
function Test-Llvm  { Test-Path "$env:ProgramFiles\LLVM\bin\libclang.dll" }
function Test-ExecPolicy { (Get-ExecutionPolicy -Scope LocalMachine) -in @('RemoteSigned', 'Unrestricted', 'Bypass') }
function Test-MachinePath { ([Environment]::GetEnvironmentVariable('Path', 'Machine')) -like "*$CargoBin*" }
function Get-RunnerService {
    Get-Service | Where-Object { $_.Name -like 'actions.runner.*' -and $_.Name -like "*$RunnerName*" } | Select-Object -First 1
}

if ($VerifyOnly) {
    Step "verify-only audit"
    if (-not (Test-Git))         { Fail "git missing" }
    if (-not (Test-Cargo))       { Fail "cargo missing at $CargoBin" }
    if (-not (Test-Msvc))        { Fail "MSVC toolset missing" }
    if (-not (Test-Llvm))        { Fail "libclang (LLVM) missing" }
    if (-not (Test-ExecPolicy))  { Fail "LocalMachine ExecutionPolicy blocks runner step scripts" }
    if (-not (Test-MachinePath)) { Fail "machine PATH lacks $CargoBin" }
    $svc = Get-RunnerService
    if (-not $svc)                          { Fail "no actions.runner service for $RunnerName" }
    elseif ($svc.Status -ne 'Running')      { Fail "runner service is $($svc.Status), not Running" }
    if ($failures.Count) { Write-Host "VERIFY: $($failures.Count) problem(s)"; exit 1 }
    Write-Host "VERIFY: all green"; exit 0
}

if (-not $Token) { throw "-Token is required to (re)register (or use -VerifyOnly)" }

# -- 1 - execution policy (runner steps are PS scripts) ------------------
Step "execution policy"
if (Test-ExecPolicy) { Skip "LocalMachine = $(Get-ExecutionPolicy -Scope LocalMachine)" }
else { Set-ExecutionPolicy RemoteSigned -Scope LocalMachine -Force }

# -- 2 - git (machine-wide; actions/checkout needs it) -------------------
Step "git"
if (Test-Git) { Skip (& "$env:ProgramFiles\Git\cmd\git.exe" --version) }
else {
    Get-File $GitUrl "$Stage\git-installer.exe"
    Invoke-Installer "$Stage\git-installer.exe" @('/VERYSILENT', '/NORESTART', "/LOG=$Stage\git-install.log")
    if (-not (Test-Git)) { throw "git still missing after install -- see $Stage\git-install.log" }
}

# -- 3 - rust (SYSTEM profile -- the service account's home) --------------
Step "rust $RustVer (default profile)"
if (Test-Cargo) { Skip "cargo present at $CargoBin" }
else {
    Get-File 'https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe' "$Stage\rustup-init.exe"
    Invoke-Installer "$Stage\rustup-init.exe" @('-y', '--default-toolchain', $RustVer, '--profile', 'default')
    if (-not (Test-Cargo)) { throw "cargo still missing after rustup -- expected $CargoBin\cargo.exe" }
}

# -- 4 - VS Build Tools (MSVC + Windows SDK; 3010 = ok, reboot pending) --
Step "VS Build Tools (VCTools workload)"
if (Test-Msvc) { Skip "MSVC toolset present" }
else {
    Get-File $VsUrl "$Stage\vs_BuildTools.exe"
    Invoke-Installer "$Stage\vs_BuildTools.exe" @(
        '--quiet', '--wait', '--norestart',
        '--add', 'Microsoft.VisualStudio.Workload.VCTools', '--includeRecommended'
    ) @(0, 3010)
    if (-not (Test-Msvc)) { throw "MSVC still missing after VS Build Tools -- logs in C:\Windows\SystemTemp\dd_*.log" }
}

# -- 5 - LLVM (bindgen needs libclang; VS BuildTools does NOT ship it) ---
Step "LLVM 18.1.8 (libclang)"
if (Test-Llvm) { Skip "libclang.dll present" }
else {
    Get-File $LlvmUrl "$Stage\llvm.exe"
    Invoke-Installer "$Stage\llvm.exe" @('/S')
    if (-not (Test-Llvm)) { throw "libclang.dll still missing after LLVM install" }
}

# -- 6 - machine PATH carries cargo for the service ----------------------
Step "machine PATH += $CargoBin"
if (Test-MachinePath) { Skip "already on machine PATH" }
else {
    $p = [Environment]::GetEnvironmentVariable('Path', 'Machine')
    [Environment]::SetEnvironmentVariable('Path', "$p;$CargoBin", 'Machine')
}

# -- 7 - the runner, as a SYSTEM service ---------------------------------
Step "actions runner -> $RunnerRoot"
if (Test-Path "$RunnerRoot\config.cmd") { Skip "runner unpacked" }
else {
    $rel = Invoke-RestMethod 'https://api.github.com/repos/actions/runner/releases/latest'
    $ver = $rel.tag_name.TrimStart('v')
    Get-File "https://github.com/actions/runner/releases/download/v$ver/actions-runner-win-x64-$ver.zip" "$Stage\runner.zip"
    New-Item -ItemType Directory -Force $RunnerRoot | Out-Null
    tar -xf "$Stage\runner.zip" -C $RunnerRoot
    if (-not (Test-Path "$RunnerRoot\config.cmd")) { throw "runner extraction failed" }
}

Step "register + service install ($RunnerName, labels: $Labels)"
Push-Location $RunnerRoot
try {
    # --replace: safe re-run; a same-named registration is superseded.
    & .\config.cmd --unattended --replace --url $RepoUrl --token $Token `
        --name $RunnerName --labels $Labels `
        --runasservice --windowslogonaccount 'NT AUTHORITY\SYSTEM'
    if ($LASTEXITCODE -ne 0) { throw "config.cmd exited $LASTEXITCODE" }
} finally { Pop-Location }

# -- 8 - verify or die ---------------------------------------------------
Step "final verify"
$svc = Get-RunnerService
if (-not $svc) { throw "runner service missing after config" }
if ($svc.Status -ne 'Running') {
    Restart-Service $svc.Name
    $svc.Refresh()
    if ($svc.Status -ne 'Running') { throw "runner service is $($svc.Status)" }
}
foreach ($check in @(
        @{ n = 'git'; ok = (Test-Git) }, @{ n = 'cargo'; ok = (Test-Cargo) },
        @{ n = 'MSVC'; ok = (Test-Msvc) }, @{ n = 'libclang'; ok = (Test-Llvm) })) {
    if (-not $check.ok) { throw "post-install verify failed: $($check.n)" }
}
Write-Host "BOOTSTRAP GREEN: $RunnerName registered, service '$($svc.Name)' Running."
Write-Host "Confirm on the repo: gh api repos/<owner>/<repo>/actions/runners"
