param(
    [Parameter(Mandatory = $true)]
    [string]$ArchivePath
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$archive = (Resolve-Path -LiteralPath $ArchivePath).Path
$tempBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$smokeDir = Join-Path $tempBase ("hlex-portable-smoke-" + [guid]::NewGuid().ToString("N"))
$process = $null

New-Item -ItemType Directory -Path $smokeDir | Out-Null
try {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [IO.Compression.ZipFile]::ExtractToDirectory($archive, $smokeDir)

    $executable = Join-Path $smokeDir "Hexo Lite Editor.exe"
    $required = @(
        $executable,
        (Join-Path $smokeDir "resources\resolve-hexo-route.cjs"),
        (Join-Path $smokeDir "README.txt")
    )
    foreach ($item in $required) {
        if (-not (Test-Path -LiteralPath $item -PathType Leaf)) {
            throw "Missing portable file: $item"
        }
    }

    $process = Start-Process -FilePath $executable -PassThru
    $ready = $false
    for ($attempt = 0; $attempt -lt 40; $attempt++) {
        Start-Sleep -Milliseconds 250
        $process.Refresh()
        if ($process.HasExited) {
            throw "Portable app exited early with code $($process.ExitCode)"
        }
        if ($process.Responding -and $process.MainWindowHandle -ne 0) {
            $ready = $true
            break
        }
    }
    if (-not $ready) {
        throw "Portable app did not create a responsive window in time"
    }

    $version = (Get-Item -LiteralPath $executable).VersionInfo.ProductVersion
    Write-Output "Portable smoke passed: pid=$($process.Id), version=$version"
} finally {
    if ($null -ne $process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        $process.WaitForExit(5000) | Out-Null
    }
    $resolvedSmokeDir = [IO.Path]::GetFullPath($smokeDir)
    if (-not $resolvedSmokeDir.StartsWith($tempBase, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to remove a path outside the system temp directory: $resolvedSmokeDir"
    }
    Remove-Item -LiteralPath $resolvedSmokeDir -Recurse -Force -ErrorAction SilentlyContinue
}
