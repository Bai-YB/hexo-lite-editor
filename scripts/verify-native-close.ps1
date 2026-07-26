param(
  [string]$Executable = (Join-Path $PSScriptRoot "..\src-tauri\target\release\hexo-lite-editor.exe")
)

$ErrorActionPreference = "Stop"
$resolvedExecutable = (Resolve-Path -LiteralPath $Executable).Path
$testRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("hexo-lite-native-close-" + [guid]::NewGuid().ToString("N"))
$testAppData = Join-Path $testRoot "AppData"
$previousAppData = [Environment]::GetEnvironmentVariable("APPDATA", "Process")
$process = $null

Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class HexoLiteNativeWindow {
  [DllImport("user32.dll", SetLastError = true)]
  public static extern bool PostMessage(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam);
}
"@

try {
  New-Item -ItemType Directory -Path $testAppData -Force | Out-Null
  [Environment]::SetEnvironmentVariable("APPDATA", $testAppData, "Process")
  $process = Start-Process -FilePath $resolvedExecutable -PassThru

  $deadline = [DateTime]::UtcNow.AddSeconds(20)
  do {
    Start-Sleep -Milliseconds 200
    $process.Refresh()
  } while ($process.MainWindowHandle -eq 0 -and !$process.HasExited -and [DateTime]::UtcNow -lt $deadline)

  if ($process.HasExited) {
    throw "The app exited before showing its main window. Exit code: $($process.ExitCode)"
  }
  if ($process.MainWindowHandle -eq 0) {
    throw "The native main window was not found within 20 seconds."
  }

  $wmClose = 0x0010
  if (-not [HexoLiteNativeWindow]::PostMessage($process.MainWindowHandle, $wmClose, [IntPtr]::Zero, [IntPtr]::Zero)) {
    throw "Failed to send WM_CLOSE to the native window."
  }
  if (-not $process.WaitForExit(10000)) {
    throw "The native window did not exit within 10 seconds after WM_CLOSE."
  }
  if ($process.ExitCode -ne 0) {
    throw "The app returned a non-zero exit code after closing: $($process.ExitCode)"
  }

  Write-Host "PASS native-close: the Tauri main window handled a real WM_CLOSE and exited cleanly."
} finally {
  [Environment]::SetEnvironmentVariable("APPDATA", $previousAppData, "Process")
  if ($process -and -not $process.HasExited) {
    Stop-Process -Id $process.Id -Force
  }
  if (Test-Path -LiteralPath $testRoot) {
    Remove-Item -LiteralPath $testRoot -Recurse -Force
  }
}
