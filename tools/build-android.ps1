param(
    [switch]$Setup,
    [switch]$DebugBuild,
    [string]$SdkPath = $env:ANDROID_HOME,
    [string]$NdkPath = $env:NDK_HOME
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$toolchain = '1.95.0'
$toolRevision = 'bdb61325ce1aecf09ed8969690f53c86c846c118'
$toolRoot = Join-Path $projectRoot 'target/android-tools'
$apkTool = Join-Path $toolRoot 'bin/cargo-quad-apk.exe'
function Invoke-Checked([string]$Command, [string[]]$Arguments) {
    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) { throw "Command failed: $Command $($Arguments -join ' ')" }
}
if (-not $SdkPath) { $SdkPath = Join-Path $env:LOCALAPPDATA 'Android/Sdk' }
if (-not (Test-Path -LiteralPath $SdkPath)) { throw 'Android SDK missing. Install it using Android Studio, then pass -SdkPath. See android/README.md.' }
if (-not $NdkPath) { $NdkPath = Join-Path $SdkPath 'ndk/28.2.13676358' }
foreach ($required in @('platforms/android-36/android.jar', 'build-tools/36.0.0/apksigner.bat')) {
    if (-not (Test-Path -LiteralPath (Join-Path $SdkPath $required))) { throw "Missing SDK component: $required. See android/README.md." }
}
if (-not (Test-Path -LiteralPath (Join-Path $NdkPath 'source.properties'))) { throw 'Android NDK r28 is missing. Install NDK 28.2.13676358 or pass -NdkPath.' }
if (-not (Get-Command 7z -ErrorAction SilentlyContinue)) {
    $zipDirectory = 'C:/Program Files/7-Zip'
    if (Test-Path -LiteralPath (Join-Path $zipDirectory '7z.exe')) { $env:PATH = "$zipDirectory;$env:PATH" }
    else { throw 'Install 7-Zip and add 7z to PATH.' }
}
if ($Setup) {
    Invoke-Checked 'rustup' @('toolchain', 'install', $toolchain, '--profile', 'minimal')
    Invoke-Checked 'rustup' @('target', 'add', '--toolchain', $toolchain, 'aarch64-linux-android')
    Invoke-Checked 'cargo' @("+$toolchain", 'install', '--git', 'https://github.com/hoklims/cargo-quad-apk', '--rev', $toolRevision, '--locked', '--root', $toolRoot, 'cargo-quad-apk')
}
if (-not (Test-Path -LiteralPath $apkTool)) { throw 'APK builder missing. Run this script with -Setup once.' }
$previousToolchain = $env:RUSTUP_TOOLCHAIN
$previousSdk = $env:ANDROID_HOME
$previousNdk = $env:NDK_HOME
$previousPath = $env:PATH
Push-Location $projectRoot
try {
    $env:RUSTUP_TOOLCHAIN = $toolchain
    $env:ANDROID_HOME = $SdkPath
    $env:NDK_HOME = $NdkPath
    $env:PATH = "$(Join-Path $toolRoot 'bin');$env:PATH"
    $buildArguments = @("+$toolchain", 'quad-apk', 'build')
    if (-not $DebugBuild) { $buildArguments += '--release' }
    Invoke-Checked 'cargo' $buildArguments
    $profile = if ($DebugBuild) { 'debug' } else { 'release' }
    $apkDirectory = Join-Path $projectRoot "target/android-artifacts/$profile/apk"
    $packages = @(Get-ChildItem -LiteralPath $apkDirectory -Filter '*.apk')
    if (-not $packages.Count) { throw "No APK found in $apkDirectory" }
    foreach ($package in $packages) {
        Invoke-Checked (Join-Path $SdkPath 'build-tools/36.0.0/apksigner.bat') @('verify', $package.FullName)
        Write-Output "APK: $($package.FullName)"
    }
} finally {
    Pop-Location
    $env:RUSTUP_TOOLCHAIN = $previousToolchain
    $env:ANDROID_HOME = $previousSdk
    $env:NDK_HOME = $previousNdk
    $env:PATH = $previousPath
}
