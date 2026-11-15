; SenseHue v3.0 — Custom Branded NSIS Installer Theme & Rounded Window Chrome

!define MUI_HEADERIMAGE_RIGHT
!define MUI_HEADERIMAGE_BITMAP_STRETCH "FitControl"
!define MUI_HEADERIMAGE_UNBITMAP_STRETCH "FitControl"
!define MUI_WELCOMEFINISHPAGE_BITMAP_STRETCH "FitControl"
!define MUI_UNWELCOMEFINISHPAGE_BITMAP_STRETCH "FitControl"
!define MUI_HEADER_TRANSPARENT_TEXT
!define MUI_FORCECLASSICCONTROLS

; Obsidian brand palette matching SenseHue desktop UI
!define MUI_BGCOLOR "0B0F19"
!define MUI_TEXTCOLOR "F1F5F9"

!define MUI_FINISHPAGE_LINK "github.com/Ignaciobrenas/SenseHue"
!define MUI_FINISHPAGE_LINK_LOCATION "https://github.com/Ignaciobrenas/SenseHue"
!define MUI_FINISHPAGE_LINK_COLOR "38BDF8"

!define MUI_CUSTOMFUNCTION_GUIINIT SenseHueApplyRoundedDarkChrome
!define MUI_CUSTOMFUNCTION_UNGUIINIT un.SenseHueApplyRoundedDarkChrome

; Apply Windows 11 DWM rounded corners, dark obsidian titlebar, and SenseHue cyan border,
; with a clean GDI rounded-region fallback on Windows 10 or older hosts.
!macro SENSEHUE_APPLY_CHROME
  ; DWMWA_USE_IMMERSIVE_DARK_MODE (20) = TRUE (1)
  System::Call 'dwmapi::DwmSetWindowAttribute(p $HWNDPARENT, i 20, *i 1, i 4)'
  ; DWMWA_WINDOW_CORNER_PREFERENCE (33) = DWMWCP_ROUND (2)
  System::Call 'dwmapi::DwmSetWindowAttribute(p $HWNDPARENT, i 33, *i 2, i 4) i.r0'
  ; DWMWA_BORDER_COLOR (34) = #38BDF8 (BGR 0x00F8BD38)
  System::Call 'dwmapi::DwmSetWindowAttribute(p $HWNDPARENT, i 34, *i 0x00F8BD38, i 4)'
  ; DWMWA_CAPTION_COLOR (35) = #0B0F19 (BGR 0x00190F0B)
  System::Call 'dwmapi::DwmSetWindowAttribute(p $HWNDPARENT, i 35, *i 0x00190F0B, i 4)'
  ; DWMWA_TEXT_COLOR (36) = #F1F5F9 (BGR 0x00F9F5F1)
  System::Call 'dwmapi::DwmSetWindowAttribute(p $HWNDPARENT, i 36, *i 0x00F9F5F1, i 4)'

  ${If} $0 <> 0
    System::Call '*(i, i, i, i) .r1'
    System::Call 'user32::GetWindowRect(p $HWNDPARENT, p r1)'
    System::Call '*$1(i .r2, i .r3, i .r4, i .r5)'
    System::Free $1
    IntOp $4 $4 - $2
    IntOp $5 $5 - $3
    System::Call 'gdi32::CreateRoundRectRgn(i 0, i 0, i r4, i r5, i 24, i 24) p.r6'
    System::Call 'user32::SetWindowRgn(p $HWNDPARENT, p r6, i 1)'
  ${EndIf}
!macroend

Function SenseHueApplyRoundedDarkChrome
  !insertmacro SENSEHUE_APPLY_CHROME
FunctionEnd

Function un.SenseHueApplyRoundedDarkChrome
  !insertmacro SENSEHUE_APPLY_CHROME
FunctionEnd
