; Hexo Lite Editor installer hooks.
;
; The in-app updater always runs the Windows installer with
; `/D=<directory the running application lives in>`, so a portable copy keeps
; updating itself in place and an installation keeps whatever directory the user
; picked (or the default one). What is left for these hooks is the desktop
; shortcut policy:
;
; * an interactive install keeps the finish page checkbox, so a user that
;   declines a desktop shortcut does not get one;
; * every later update (always silent, `/UPDATE`) ends up with a desktop
;   shortcut that points at the freshly installed binary, even when the first
;   install declined it or the shortcut was deleted afterwards.

!include "LogicLib.nsh"

; Creates or refreshes the desktop shortcut for the installed binary.
; An existing desktop shortcut that already targets the binary is reused, so an
; update never leaves the user with two icons for the same application.
!macro HLEX_ENSURE_DESKTOP_SHORTCUT
  Push $0
  Push $1
  Push $2
  Push $3
  Push $R6
  Push $R7
  Push $R8
  Push $R9

  StrCpy $R8 ""
  FindFirst $R7 $R9 "$DESKTOP\*.lnk"
  hlex_desktop_shortcut_scan:
    StrCmp $R9 "" hlex_desktop_shortcut_scan_done
    !insertmacro IsShortcutTarget "$DESKTOP\$R9" "$INSTDIR\${MAINBINARYNAME}.exe"
    Pop $R6
    ${If} $R6 = 1
    ${AndIf} $R8 == ""
      StrCpy $R8 "$R9"
    ${EndIf}
    FindNext $R7 $R9
    Goto hlex_desktop_shortcut_scan
  hlex_desktop_shortcut_scan_done:
  FindClose $R7

  ${If} $R8 == ""
    StrCpy $R8 "${PRODUCTNAME}.lnk"
  ${EndIf}
  CreateShortcut "$DESKTOP\$R8" "$INSTDIR\${MAINBINARYNAME}.exe"
  !insertmacro SetLnkAppUserModelId "$DESKTOP\$R8"

  Pop $R9
  Pop $R8
  Pop $R7
  Pop $R6
  Pop $3
  Pop $2
  Pop $1
  Pop $0
!macroend

!macro NSIS_HOOK_PREINSTALL
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ${If} $UpdateMode = 1
    !insertmacro HLEX_ENSURE_DESKTOP_SHORTCUT
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
!macroend
