$ErrorActionPreference = 'Stop'
$projectRoot = $PSScriptRoot
$buildTarget = Join-Path $projectRoot 'target'
if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
    throw 'Docker is missing. Install Docker Desktop and enable Linux containers.'
}
$engine = docker info --format '{{.OSType}}'
if ($LASTEXITCODE -ne 0 -or $engine -ne 'linux') {
    throw 'Start Docker Desktop with Linux containers, then run this script again.'
}
New-Item -ItemType Directory -Force -Path $buildTarget | Out-Null
# Send only the Dockerfile as context: avoid uploading target/, FBX and .git.
Get-Content -Raw (Join-Path $projectRoot 'Dockerfile.android') | docker build --platform linux/amd64 --tag fotbal-android:api36 -
if ($LASTEXITCODE -ne 0) { throw 'Android build image failed; see Docker output above.' }

docker run --rm --platform linux/amd64 `
    --mount 'type=volume,source=fotbal-android-signing,target=/root/.android' `
    --mount 'type=volume,source=fotbal-android-registry,target=/usr/local/cargo/registry' `
    --mount 'type=volume,source=fotbal-android-build,target=/root/src/target' `
    --mount "type=bind,source=$projectRoot,target=/project,readonly" `
    --mount "type=bind,source=$buildTarget,target=/output" `
    -e CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse `
    -w /root/src fotbal-android:api36 sh /project/build-android.sh
if ($LASTEXITCODE -ne 0) { throw 'Android build failed; see Docker output above.' }
Write-Host "APK: $buildTarget\android-artifacts\release\apk\game.apk"
