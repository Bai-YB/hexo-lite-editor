param(
    [Parameter(Mandatory = $true)]
    [string]$NsisPath,
    [Parameter(Mandatory = $true)]
    [string]$MsiPath
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$nsis = (Resolve-Path -LiteralPath $NsisPath).Path
$msi = (Resolve-Path -LiteralPath $MsiPath).Path
$tempBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$smokeRoot = Join-Path $tempBase ("hlex-installer-smoke-" + [guid]::NewGuid().ToString("N"))
$nsisInstallDir = Join-Path $smokeRoot "nsis"
$msiExtractDir = Join-Path $smokeRoot "msi"
$appProcess = $null
$uninstaller = $null

function Get-InstalledHexoLiteEditor {
    $uninstallRegistryPaths = @(
        "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*",
        "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*",
        "HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*"
    )
    Get-ItemProperty $uninstallRegistryPaths -ErrorAction SilentlyContinue |
        Where-Object {
            $displayName = $_.PSObject.Properties["DisplayName"]
            $null -ne $displayName -and $displayName.Value -eq "Hexo Lite Editor"
        }
}

if (Get-InstalledHexoLiteEditor) {
    throw "Refusing installer smoke test because Hexo Lite Editor is already installed"
}

New-Item -ItemType Directory -Path $nsisInstallDir, $msiExtractDir -Force | Out-Null
try {
    $install = Start-Process -FilePath $nsis -ArgumentList @("/S", "/D=$nsisInstallDir") -PassThru -Wait
    if ($install.ExitCode -ne 0) {
        throw "NSIS installer failed with code $($install.ExitCode)"
    }

    $installedExe = Join-Path $nsisInstallDir "hexo-lite-editor.exe"
    $uninstaller = Join-Path $nsisInstallDir "uninstall.exe"
    $installedRouteHelper = Join-Path $nsisInstallDir "resources\resolve-hexo-route.cjs"
    foreach ($required in @($installedExe, $uninstaller, $installedRouteHelper)) {
        if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
            throw "Missing NSIS installed file: $required"
        }
    }

    $appProcess = Start-Process -FilePath $installedExe -PassThru
    $ready = $false
    for ($attempt = 0; $attempt -lt 40; $attempt++) {
        Start-Sleep -Milliseconds 250
        $appProcess.Refresh()
        if ($appProcess.HasExited) {
            throw "Installed app exited early with code $($appProcess.ExitCode)"
        }
        if ($appProcess.Responding -and $appProcess.MainWindowHandle -ne 0) {
            $ready = $true
            break
        }
    }
    if (-not $ready) {
        throw "Installed app did not create a responsive window in time"
    }
    Stop-Process -Id $appProcess.Id -Force
    $appProcess.WaitForExit(5000) | Out-Null
    $appProcess = $null

    $arguments = @("/a", ('"' + $msi + '"'), "/qn", ("TARGETDIR=" + '"' + $msiExtractDir + '"'))
    $extract = Start-Process -FilePath "msiexec.exe" -ArgumentList $arguments -PassThru -Wait
    if ($extract.ExitCode -ne 0) {
        throw "MSI administrative extraction failed with code $($extract.ExitCode)"
    }
    $msiExe = Get-ChildItem -LiteralPath $msiExtractDir -Recurse -Filter "hexo-lite-editor.exe" -File | Select-Object -First 1
    $msiRouteHelper = Get-ChildItem -LiteralPath $msiExtractDir -Recurse -Filter "resolve-hexo-route.cjs" -File | Select-Object -First 1
    if ($null -eq $msiExe -or $null -eq $msiRouteHelper) {
        throw "MSI extraction is missing the executable or route helper"
    }

    Write-Output "Installer smoke passed: NSIS install/start and MSI administrative extraction"
} finally {
    if ($null -ne $appProcess -and -not $appProcess.HasExited) {
        Stop-Process -Id $appProcess.Id -Force -ErrorAction SilentlyContinue
        $appProcess.WaitForExit(5000) | Out-Null
    }
    if ($null -ne $uninstaller -and (Test-Path -LiteralPath $uninstaller -PathType Leaf)) {
        $uninstall = Start-Process -FilePath $uninstaller -ArgumentList "/S" -PassThru -Wait
        if ($uninstall.ExitCode -ne 0) {
            Write-Warning "NSIS uninstaller returned code $($uninstall.ExitCode)"
        }
    }
    $resolvedSmokeRoot = [IO.Path]::GetFullPath($smokeRoot)
    if (-not $resolvedSmokeRoot.StartsWith($tempBase, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to remove a path outside the system temp directory: $resolvedSmokeRoot"
    }
    Remove-Item -LiteralPath $resolvedSmokeRoot -Recurse -Force -ErrorAction SilentlyContinue
}
