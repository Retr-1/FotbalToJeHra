param(
    [string]$ConverterPath = (Join-Path $PSScriptRoot 'fbx2gltf/FBX2glTF-windows-x86_64/FBX2glTF-windows-x86_64.exe')
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
if (-not (Test-Path -LiteralPath $ConverterPath -PathType Leaf)) {
    throw 'Provide -ConverterPath pointing to FBX2glTF. Download: https://github.com/godotengine/FBX2glTF/releases/tag/v0.13.1'
}
$destination = Join-Path $projectRoot 'assets/generated'
New-Item -ItemType Directory -Force -Path $destination | Out-Null
$conversions = @(
    @('assets/models/X Bot.fbx', 'x_bot.glb'),
    @('assets/models/Y Bot.fbx', 'y_bot.glb'),
    @('assets/animations/Running.fbx', 'running.glb'),
    @('assets/animations/Idle.fbx', 'idle.glb')
)
foreach ($conversion in $conversions) {
    $source = Join-Path $projectRoot $conversion[0]
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) { throw "Missing source: $source" }
    & $ConverterPath --binary --input $source --output (Join-Path $destination $conversion[1])
    if ($LASTEXITCODE -ne 0) { throw "FBX conversion failed: $source" }
}
