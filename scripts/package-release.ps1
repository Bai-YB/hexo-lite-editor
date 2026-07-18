param(
    [string]$Version = "1.0.3",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

if ($Version -notmatch '^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$') {
    throw "Invalid release version: $Version"
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$releaseRoot = Join-Path $repoRoot "release-artifacts"
$outputDir = Join-Path $releaseRoot $Version
$targetDir = Join-Path $repoRoot "src-tauri\target\release"

New-Item -ItemType Directory -Path $releaseRoot -Force | Out-Null
if (Test-Path -LiteralPath $outputDir) {
    $resolvedOutput = (Resolve-Path -LiteralPath $outputDir).Path
    $resolvedReleaseRoot = (Resolve-Path -LiteralPath $releaseRoot).Path
    if (-not $resolvedOutput.StartsWith($resolvedReleaseRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to clear a path outside release-artifacts: $resolvedOutput"
    }
    Remove-Item -LiteralPath $resolvedOutput -Recurse -Force
}
New-Item -ItemType Directory -Path $outputDir -Force | Out-Null

Push-Location $repoRoot
try {
    if (-not $SkipBuild) {
        & pnpm tauri build
        if ($LASTEXITCODE -ne 0) { throw "Tauri installer build failed" }

        # Rebuild without bundle metadata so the portable executable resolves
        # resources relative to its extracted directory.
        & pnpm tauri build --no-bundle
        if ($LASTEXITCODE -ne 0) { throw "Tauri portable build failed" }
    }

    $nsisSource = Join-Path $targetDir "bundle\nsis\Hexo Lite Editor_${Version}_x64-setup.exe"
    $msiSource = Join-Path $targetDir "bundle\msi\Hexo Lite Editor_${Version}_x64_en-US.msi"
    $exeSource = Join-Path $targetDir "hexo-lite-editor.exe"
    $routeHelper = Join-Path $targetDir "resources\resolve-hexo-route.cjs"

    foreach ($required in @($nsisSource, $msiSource, $exeSource, $routeHelper)) {
        if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
            throw "Missing release input: $required"
        }
    }

    $setupName = "Hexo-Lite-Editor_${Version}_windows-x64-setup.exe"
    $msiName = "Hexo-Lite-Editor_${Version}_windows-x64.msi"
    $portableName = "Hexo-Lite-Editor_${Version}_windows-x64-portable.zip"
    Copy-Item -LiteralPath $nsisSource -Destination (Join-Path $outputDir $setupName)
    Copy-Item -LiteralPath $msiSource -Destination (Join-Path $outputDir $msiName)

    $portableStage = Join-Path $outputDir "portable-stage"
    New-Item -ItemType Directory -Path (Join-Path $portableStage "resources") -Force | Out-Null
    Copy-Item -LiteralPath $exeSource -Destination (Join-Path $portableStage "Hexo Lite Editor.exe")
    Copy-Item -LiteralPath $routeHelper -Destination (Join-Path $portableStage "resources\resolve-hexo-route.cjs")
    $portableReadme = @(
        "Hexo Lite Editor $Version - Windows x64 portable edition",
        "",
        "1. Extract the entire ZIP before running the application.",
        "2. Double-click Hexo Lite Editor.exe. No command line is required.",
        "3. Keep the resources directory next to the executable.",
        "4. If WebView2 is missing, the app opens the official Microsoft download page.",
        "5. The app starts without Node.js. Hexo preview and publishing require project dependencies.",
        "6. This build is unsigned and Windows SmartScreen may show an unknown publisher warning."
    )
    $portableReadme | Set-Content -LiteralPath (Join-Path $portableStage "README.txt") -Encoding utf8

    Compress-Archive -Path (Join-Path $portableStage "*") -DestinationPath (Join-Path $outputDir $portableName) -CompressionLevel Optimal
    Remove-Item -LiteralPath $portableStage -Recurse -Force

    $assetNames = @($setupName, $msiName, $portableName)
    $commit = (& git rev-parse HEAD).Trim()
    $assetEntries = foreach ($name in $assetNames) {
        $file = Get-Item -LiteralPath (Join-Path $outputDir $name)
        $hash = Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256
        [ordered]@{
            name = $name
            size = $file.Length
            sha256 = $hash.Hash.ToLowerInvariant()
        }
    }
    $manifest = [ordered]@{
        version = $Version
        platform = "windows"
        architecture = "x64"
        sourceCommit = $commit
        generatedAt = (Get-Date).ToUniversalTime().ToString("o")
        codeSigned = $false
        assets = @($assetEntries)
    }
    $manifestPath = Join-Path $outputDir "release-manifest.json"
    $manifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $manifestPath -Encoding utf8

    $checksumNames = $assetNames + "release-manifest.json"
    $checksumLines = foreach ($name in $checksumNames) {
        $hash = Get-FileHash -LiteralPath (Join-Path $outputDir $name) -Algorithm SHA256
        "$($hash.Hash.ToLowerInvariant())  $name"
    }
    $checksumLines | Set-Content -LiteralPath (Join-Path $outputDir "SHA256SUMS.txt") -Encoding ascii
    Write-Output "Release artifacts: $outputDir"
} finally {
    Pop-Location
}
