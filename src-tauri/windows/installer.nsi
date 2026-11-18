Unicode true
ManifestDPIAware true
; Add in `dpiAwareness` `PerMonitorV2` to manifest for Windows 10 1607+ (note this should not affect lower versions since they should be able to ignore this and pick up `dpiAware` `true` set by `ManifestDPIAware true`)
; Currently undocumented on NSIS's website but is in the Docs folder of source tree, see
; https://github.com/kichik/nsis/blob/5fc0b87b819a9eec006df4967d08e522ddd651c9/Docs/src/attributes.but#L286-L300
; https://github.com/tauri-apps/tauri/pull/10106
ManifestDPIAwareness PerMonitorV2

!if "{{compression}}" == "none"
  SetCompress off
!else
  ; Set the compression algorithm. We default to LZMA.
  SetCompressor /SOLID "{{compression}}"
!endif

!include MUI2.nsh
!include FileFunc.nsh
!include x64.nsh
!include WordFunc.nsh
!include "utils.nsh"
!include "FileAssociation.nsh"
!include "Win\COM.nsh"
!include "Win\Propkey.nsh"
!include "StrFunc.nsh"
${StrCase}
${StrLoc}

{{#if installer_hooks}}
!include "{{installer_hooks}}"
{{/if}}

!define WEBVIEW2APPGUID "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"

!define MANUFACTURER "{{manufacturer}}"
!define PRODUCTNAME "{{product_name}}"
!define VERSION "{{version}}"
!define VERSIONWITHBUILD "{{version_with_build}}"
!define HOMEPAGE "{{homepage}}"
!define INSTALLMODE "{{install_mode}}"
!define LICENSE "{{license}}"
!define INSTALLERICON "{{installer_icon}}"
!define SIDEBARIMAGE "{{sidebar_image}}"
!define HEADERIMAGE "{{header_image}}"
!define UNINSTALLERICON "{{uninstaller_icon}}"
!define UNINSTALLERHEADERIMAGE "{{uninstaller_header_image}}"
!define MAINBINARYNAME "{{main_binary_name}}"
!define MAINBINARYSRCPATH "{{main_binary_path}}"
!define BUNDLEID "{{bundle_id}}"
!define COPYRIGHT "{{copyright}}"
!define OUTFILE "{{out_file}}"
!define ARCH "{{arch}}"
!define ADDITIONALPLUGINSPATH "{{additional_plugins_path}}"
!define ALLOWDOWNGRADES "{{allow_downgrades}}"
!define DISPLAYLANGUAGESELECTOR "{{display_language_selector}}"
!define INSTALLWEBVIEW2MODE "{{install_webview2_mode}}"
!define WEBVIEW2INSTALLERARGS "{{webview2_installer_args}}"
!define WEBVIEW2BOOTSTRAPPERPATH "{{webview2_bootstrapper_path}}"
!define WEBVIEW2INSTALLERPATH "{{webview2_installer_path}}"
!define MINIMUMWEBVIEW2VERSION "{{minimum_webview2_version}}"
!define UNINSTKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCTNAME}"
!define MANUKEY "Software\${MANUFACTURER}"
!define MANUPRODUCTKEY "${MANUKEY}\${PRODUCTNAME}"
!define UNINSTALLERSIGNCOMMAND "{{uninstaller_sign_cmd}}"
!define ESTIMATEDSIZE "{{estimated_size}}"
!define STARTMENUFOLDER "{{start_menu_folder}}"

Var PassiveMode
Var UpdateMode
Var NoShortcutMode
Var WixMode
Var OldMainBinaryName

; Lectus one-click installer (fork of the stock Tauri 2.11.2 template, see
; scripts/check-nsis-template.mjs). A plain double-click shows one small
; branded window, installs in place and opens the app. Modes:
;   (no flags)  one-click: branded window, asks before closing a running Lectus
;   /P          passive (the updater adds /UPDATE /R): same branded window
;   /S          silent: no window at all
;   /NOLAUNCH   one-click without opening the app afterwards
;   /LECTUSTHEME=dark|light  force the window theme (screenshots and tests)
Var OneClickMode
Var NoLaunchMode
Var FreshInstall
Var InstalledVersion
Var LectusDark
Var LectusDpi
Var LectusTrack
Var LectusBorder
Var LectusFaceTitle
Var LectusFaceBody
Var LectusPage
Var LectusTitle
Var LectusSubtitle
Var LectusProgress

!define /ifndef PBM_SETBARCOLOR 0x0409
!define /ifndef PBM_SETBKCOLOR 0x2001
!define /ifndef STM_SETIMAGE 0x0172
; Window size in device-independent pixels, scaled by the monitor DPI.
!define LECTUS_WIN_W 440
!define LECTUS_WIN_H 264
; #2F6FED as a COLORREF (0x00BBGGRR), the app's --accent.
!define LECTUS_ACCENT 0x00ED6F2F

Name "${PRODUCTNAME}"
; Lectus: with no copyright configured NSIS would print "Nullsoft Install System".
!if "${COPYRIGHT}" == ""
  BrandingText " "
!else
  BrandingText "${COPYRIGHT}"
!endif
OutFile "${OUTFILE}"
ShowInstDetails nevershow

; We don't actually use this value as default install path,
; it's just for nsis to append the product name folder in the directory selector
; https://nsis.sourceforge.io/Reference/InstallDir
!define PLACEHOLDER_INSTALL_DIR "placeholder\${PRODUCTNAME}"
InstallDir "${PLACEHOLDER_INSTALL_DIR}"

VIProductVersion "${VERSIONWITHBUILD}"
VIAddVersionKey "ProductName" "${PRODUCTNAME}"
VIAddVersionKey "FileDescription" "${PRODUCTNAME}"
VIAddVersionKey "LegalCopyright" "${COPYRIGHT}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"

# additional plugins
!addplugindir "${ADDITIONALPLUGINSPATH}"

; Uninstaller signing command
!if "${UNINSTALLERSIGNCOMMAND}" != ""
  !uninstfinalize '${UNINSTALLERSIGNCOMMAND}'
!endif

; Handle install mode, `perUser`, `perMachine` or `both`
!if "${INSTALLMODE}" == "perMachine"
  RequestExecutionLevel admin
!endif

!if "${INSTALLMODE}" == "currentUser"
  RequestExecutionLevel user
!endif

!if "${INSTALLMODE}" == "both"
  !define MULTIUSER_MUI
  !define MULTIUSER_INSTALLMODE_INSTDIR "${PRODUCTNAME}"
  !define MULTIUSER_INSTALLMODE_COMMANDLINE
  !if "${ARCH}" == "x64"
    !define MULTIUSER_USE_PROGRAMFILES64
  !else if "${ARCH}" == "arm64"
    !define MULTIUSER_USE_PROGRAMFILES64
  !endif
  !define MULTIUSER_INSTALLMODE_DEFAULT_REGISTRY_KEY "${UNINSTKEY}"
  !define MULTIUSER_INSTALLMODE_DEFAULT_REGISTRY_VALUENAME "CurrentUser"
  !define MULTIUSER_INSTALLMODEPAGE_SHOWUSERNAME
  !define MULTIUSER_INSTALLMODE_FUNCTION RestorePreviousInstallLocation
  !define MULTIUSER_EXECUTIONLEVEL Highest
  !include MultiUser.nsh
!endif

; Installer icon
!if "${INSTALLERICON}" != ""
  !define MUI_ICON "${INSTALLERICON}"
!endif

; Installer sidebar image
!if "${SIDEBARIMAGE}" != ""
  !define MUI_WELCOMEFINISHPAGE_BITMAP "${SIDEBARIMAGE}"
!endif

; Enable header images for installer and uninstaller pages when either image is configured.
!if "${HEADERIMAGE}" != ""
  !define MUI_HEADERIMAGE
!else if "${UNINSTALLERHEADERIMAGE}" != ""
  !define MUI_HEADERIMAGE
!endif

; Installer header image
!if "${HEADERIMAGE}" != ""
  !define MUI_HEADERIMAGE_BITMAP "${HEADERIMAGE}"
!endif

; Uninstaller header image
!if "${UNINSTALLERHEADERIMAGE}" != ""
  !define MUI_HEADERIMAGE_UNBITMAP "${UNINSTALLERHEADERIMAGE}"
!endif

; Uninstaller icon
!if "${UNINSTALLERICON}" != ""
  !define MUI_UNICON "${UNINSTALLERICON}"
!endif

; Define registry key to store installer language
!define MUI_LANGDLL_REGISTRY_ROOT "HKCU"
!define MUI_LANGDLL_REGISTRY_KEY "${MANUPRODUCTKEY}"
!define MUI_LANGDLL_REGISTRY_VALUENAME "Installer Language"

; Restyle the main window into the compact branded window (not called when silent).
!define MUI_CUSTOMFUNCTION_GUIINIT LectusGuiInit

; Installer pages, must be ordered as they appear
; 1. Welcome Page
!define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
!insertmacro MUI_PAGE_WELCOME

; 2. License Page (if defined)
!if "${LICENSE}" != ""
  !define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
  !insertmacro MUI_PAGE_LICENSE "${LICENSE}"
!endif

; 3. Install mode (if it is set to `both`)
!if "${INSTALLMODE}" == "both"
  !define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
  !insertmacro MULTIUSER_PAGE_INSTALLMODE
!endif

; 4. Custom page to ask user if he wants to reinstall/uninstall
;    only if a previous installation was detected
Var ReinstallPageCheck
Page custom PageReinstall PageLeaveReinstall
Function PageReinstall
  ; Uninstall previous WiX installation if exists.
  ;
  ; A WiX installer stores the installation info in registry
  ; using a UUID and so we have to loop through all keys under
  ; `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall`
  ; and check if `DisplayName` and `Publisher` keys match ${PRODUCTNAME} and ${MANUFACTURER}
  ;
  ; This has a potential issue that there maybe another installation that matches
  ; our ${PRODUCTNAME} and ${MANUFACTURER} but wasn't installed by our WiX installer,
  ; however, this should be fine since the user will have to confirm the uninstallation
  ; and they can chose to abort it if doesn't make sense.
  StrCpy $0 0
  wix_loop:
    EnumRegKey $1 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall" $0
    StrCmp $1 "" wix_loop_done ; Exit loop if there is no more keys to loop on
    IntOp $0 $0 + 1
    ReadRegStr $R0 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\$1" "DisplayName"
    ReadRegStr $R1 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\$1" "Publisher"
    StrCmp "$R0$R1" "${PRODUCTNAME}${MANUFACTURER}" 0 wix_loop
    ReadRegStr $R0 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\$1" "UninstallString"
    ${StrCase} $R1 $R0 "L"
    ${StrLoc} $R0 $R1 "msiexec" ">"
    StrCmp $R0 0 0 wix_loop_done
    StrCpy $WixMode 1
    StrCpy $R6 "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\$1"
    Goto compare_version
  wix_loop_done:

  ; Check if there is an existing installation, if not, abort the reinstall page
  ReadRegStr $R0 SHCTX "${UNINSTKEY}" ""
  ReadRegStr $R1 SHCTX "${UNINSTKEY}" "UninstallString"
  ${If} "$R0$R1" == ""
    ; Nothing installed yet: remember it so one-click only adds the desktop
    ; shortcut on a first install.
    StrCpy $FreshInstall 1
    Abort
  ${EndIf}

  ; Compare this installar version with the existing installation
  ; and modify the messages presented to the user accordingly
  compare_version:
  StrCpy $R4 "$(older)"
  ${If} $WixMode = 1
    ReadRegStr $R0 HKLM "$R6" "DisplayVersion"
  ${Else}
    ReadRegStr $R0 SHCTX "${UNINSTKEY}" "DisplayVersion"
  ${EndIf}
  ${IfThen} $R0 == "" ${|} StrCpy $R4 "$(unknown)" ${|}
  StrCpy $InstalledVersion $R0

  nsis_tauri_utils::SemverCompare "${VERSION}" $R0
  Pop $R0
  ; Reinstalling the same version
  ${If} $R0 = 0
    StrCpy $R1 "$(alreadyInstalledLong)"
    StrCpy $R2 "$(addOrReinstall)"
    StrCpy $R3 "$(uninstallApp)"
    !insertmacro MUI_HEADER_TEXT "$(alreadyInstalled)" "$(chooseMaintenanceOption)"
  ; Upgrading
  ${ElseIf} $R0 = 1
    StrCpy $R1 "$(olderOrUnknownVersionInstalled)"
    StrCpy $R2 "$(uninstallBeforeInstalling)"
    StrCpy $R3 "$(dontUninstall)"
    !insertmacro MUI_HEADER_TEXT "$(alreadyInstalled)" "$(choowHowToInstall)"
  ; Downgrading
  ${ElseIf} $R0 = -1
    StrCpy $R1 "$(newerVersionInstalled)"
    StrCpy $R2 "$(uninstallBeforeInstalling)"
    !if "${ALLOWDOWNGRADES}" == "true"
      StrCpy $R3 "$(dontUninstall)"
    !else
      StrCpy $R3 "$(dontUninstallDowngrade)"
    !endif
    !insertmacro MUI_HEADER_TEXT "$(alreadyInstalled)" "$(choowHowToInstall)"
  ${Else}
    Abort
  ${EndIf}

  ; Skip showing the page if passive
  ;
  ; Note that we don't call this earlier at the begining
  ; of this function because we need to populate some variables
  ; related to current installed version if detected and whether
  ; we are downgrading or not.
  ${If} $PassiveMode = 1
    Call PageLeaveReinstall
  ${Else}
    nsDialogs::Create 1018
    Pop $R4
    ${IfThen} $(^RTL) = 1 ${|} nsDialogs::SetRTL $(^RTL) ${|}

    ${NSD_CreateLabel} 0 0 100% 24u $R1
    Pop $R1

    ${NSD_CreateRadioButton} 30u 50u -30u 8u $R2
    Pop $R2
    ${NSD_OnClick} $R2 PageReinstallUpdateSelection

    ${NSD_CreateRadioButton} 30u 70u -30u 8u $R3
    Pop $R3
    ; Disable this radio button if downgrading and downgrades are disabled
    !if "${ALLOWDOWNGRADES}" == "false"
      ${IfThen} $R0 = -1 ${|} EnableWindow $R3 0 ${|}
    !endif
    ${NSD_OnClick} $R3 PageReinstallUpdateSelection

    ; Check the first radio button if this the first time
    ; we enter this page or if the second button wasn't
    ; selected the last time we were on this page
    ${If} $ReinstallPageCheck <> 2
      SendMessage $R2 ${BM_SETCHECK} ${BST_CHECKED} 0
    ${Else}
      SendMessage $R3 ${BM_SETCHECK} ${BST_CHECKED} 0
    ${EndIf}

    ${NSD_SetFocus} $R2
    nsDialogs::Show
  ${EndIf}
FunctionEnd
Function PageReinstallUpdateSelection
  ${NSD_GetState} $R2 $R1
  ${If} $R1 == ${BST_CHECKED}
    StrCpy $ReinstallPageCheck 1
  ${Else}
    StrCpy $ReinstallPageCheck 2
  ${EndIf}
FunctionEnd
Function PageLeaveReinstall
  ${NSD_GetState} $R2 $R1

  ; If migrating from Wix, always uninstall
  ${If} $WixMode = 1
    Goto reinst_uninstall
  ${EndIf}

  ; In update mode, always proceeds without uninstalling
  ${If} $UpdateMode = 1
    Goto reinst_done
  ${EndIf}

  ; One-click never runs the old uninstaller first: it overwrites in place like
  ; the updater, so shortcuts, pins and the launch-at-login entry survive.
  ; A downgrade is the only case that asks.
  ${If} $OneClickMode = 1
    ${If} $R0 = -1
      MessageBox MB_YESNO|MB_ICONQUESTION|MB_DEFBUTTON2 "$(lectusDowngrade)" IDYES reinst_done
      Quit
    ${EndIf}
    Goto reinst_done
  ${EndIf}

  ; $R0 holds whether same(0)/upgrading(1)/downgrading(-1) version
  ; $R1 holds the radio buttons state:
  ;   1 => first choice was selected
  ;   0 => second choice was selected
  ${If} $R0 = 0 ; Same version, proceed
    ${If} $R1 = 1              ; User chose to add/reinstall
      Goto reinst_done
    ${Else}                    ; User chose to uninstall
      Goto reinst_uninstall
    ${EndIf}
  ${ElseIf} $R0 = 1 ; Upgrading
    ${If} $R1 = 1              ; User chose to uninstall
      Goto reinst_uninstall
    ${Else}
      Goto reinst_done         ; User chose NOT to uninstall
    ${EndIf}
  ${ElseIf} $R0 = -1 ; Downgrading
    ${If} $R1 = 1              ; User chose to uninstall
      Goto reinst_uninstall
    ${Else}
      Goto reinst_done         ; User chose NOT to uninstall
    ${EndIf}
  ${EndIf}

  reinst_uninstall:
    HideWindow
    ClearErrors

    ${If} $WixMode = 1
      ReadRegStr $R1 HKLM "$R6" "UninstallString"
      ExecWait '$R1' $0
    ${Else}
      ReadRegStr $4 SHCTX "${MANUPRODUCTKEY}" ""
      ReadRegStr $R1 SHCTX "${UNINSTKEY}" "UninstallString"
      ${IfThen} $UpdateMode = 1 ${|} StrCpy $R1 "$R1 /UPDATE" ${|} ; append /UPDATE
      ${IfThen} $PassiveMode = 1 ${|} StrCpy $R1 "$R1 /P" ${|} ; append /P
      StrCpy $R1 "$R1 _?=$4" ; append uninstall directory
      ExecWait '$R1' $0
    ${EndIf}

    BringToFront

    ${IfThen} ${Errors} ${|} StrCpy $0 2 ${|} ; ExecWait failed, set fake exit code

    ${If} $0 <> 0
    ${OrIf} ${FileExists} "$INSTDIR\${MAINBINARYNAME}.exe"
      ; User cancelled wix uninstaller? return to select un/reinstall page
      ${If} $WixMode = 1
      ${AndIf} $0 = 1602
        Abort
      ${EndIf}

      ; User cancelled NSIS uninstaller? return to select un/reinstall page
      ${If} $0 = 1
        Abort
      ${EndIf}

      ; Other erros? show generic error message and return to select un/reinstall page
      MessageBox MB_ICONEXCLAMATION "$(unableToUninstall)"
      Abort
    ${EndIf}
  reinst_done:
FunctionEnd

; 5. Choose install directory page
!define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
!insertmacro MUI_PAGE_DIRECTORY

; 6. Start menu shortcut page
Var AppStartMenuFolder
!if "${STARTMENUFOLDER}" != ""
  !define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
  !define MUI_STARTMENUPAGE_DEFAULTFOLDER "${STARTMENUFOLDER}"
!else
  !define MUI_PAGE_CUSTOMFUNCTION_PRE Skip
!endif
!insertmacro MUI_PAGE_STARTMENU Application $AppStartMenuFolder

; 7. Installation page (laid out as the branded window, failure state on leave)
!define MUI_PAGE_CUSTOMFUNCTION_SHOW LectusInstFilesShow
!define MUI_PAGE_CUSTOMFUNCTION_LEAVE LectusInstFilesLeave
!insertmacro MUI_PAGE_INSTFILES

; 8. Finish page
;
; Don't auto jump to finish page after installation page,
; because the installation page has useful info that can be used debug any issues with the installer.
!define MUI_FINISHPAGE_NOAUTOCLOSE
; Use show readme button in the finish page as a button create a desktop shortcut
!define MUI_FINISHPAGE_SHOWREADME
!define MUI_FINISHPAGE_SHOWREADME_TEXT "$(createDesktop)"
!define MUI_FINISHPAGE_SHOWREADME_FUNCTION CreateOrUpdateDesktopShortcut
; Show run app after installation.
!define MUI_FINISHPAGE_RUN
!define MUI_FINISHPAGE_RUN_FUNCTION RunMainBinary
!define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
!insertmacro MUI_PAGE_FINISH

Function RunMainBinary
  nsis_tauri_utils::RunAsUser "$INSTDIR\${MAINBINARYNAME}.exe" ""
FunctionEnd

; Uninstaller Pages
; 1. Confirm uninstall page
Var DeleteAppDataCheckbox
Var DeleteAppDataCheckboxState
!define /ifndef WS_EX_LAYOUTRTL         0x00400000
!define MUI_PAGE_CUSTOMFUNCTION_SHOW un.ConfirmShow
Function un.ConfirmShow ; Add add a `Delete app data` check box
  ; $1 inner dialog HWND
  ; $2 window DPI
  ; $3 style
  ; $4 x
  ; $5 y
  ; $6 width
  ; $7 height
  FindWindow $1 "#32770" "" $HWNDPARENT ; Find inner dialog
  System::Call "user32::GetDpiForWindow(p r1) i .r2"
  ${If} $(^RTL) = 1
    StrCpy $3 "${__NSD_CheckBox_EXSTYLE} | ${WS_EX_LAYOUTRTL}"
    IntOp $4 50 * $2
  ${Else}
    StrCpy $3 "${__NSD_CheckBox_EXSTYLE}"
    IntOp $4 0 * $2
  ${EndIf}
  IntOp $5 100 * $2
  IntOp $6 400 * $2
  IntOp $7 25 * $2
  IntOp $4 $4 / 96
  IntOp $5 $5 / 96
  IntOp $6 $6 / 96
  IntOp $7 $7 / 96
  System::Call 'user32::CreateWindowEx(i r3, w "${__NSD_CheckBox_CLASS}", w "$(deleteAppData)", i ${__NSD_CheckBox_STYLE}, i r4, i r5, i r6, i r7, p r1, i0, i0, i0) i .s'
  Pop $DeleteAppDataCheckbox
  SendMessage $HWNDPARENT ${WM_GETFONT} 0 0 $1
  SendMessage $DeleteAppDataCheckbox ${WM_SETFONT} $1 1
FunctionEnd
!define MUI_PAGE_CUSTOMFUNCTION_LEAVE un.ConfirmLeave
Function un.ConfirmLeave
  SendMessage $DeleteAppDataCheckbox ${BM_GETCHECK} 0 0 $DeleteAppDataCheckboxState
FunctionEnd
!define MUI_PAGE_CUSTOMFUNCTION_PRE un.SkipIfPassive
!insertmacro MUI_UNPAGE_CONFIRM

; 2. Uninstalling Page
!insertmacro MUI_UNPAGE_INSTFILES

;Languages
{{#each languages}}
!insertmacro MUI_LANGUAGE "{{this}}"
{{/each}}
!insertmacro MUI_RESERVEFILE_LANGDLL
{{#each language_files}}
  !include "{{this}}"
{{/each}}

; Lectus one-click strings. The language follows the Windows display language.
!ifdef LANG_ENGLISH
  LangString lectusPreparing ${LANG_ENGLISH} "Getting ${PRODUCTNAME} ready"
  LangString lectusUpdating ${LANG_ENGLISH} "Updating ${PRODUCTNAME}"
  LangString lectusSubtitle ${LANG_ENGLISH} "This takes less than a minute."
  LangString lectusReady ${LANG_ENGLISH} "${PRODUCTNAME} is ready"
  LangString lectusOpening ${LANG_ENGLISH} "Opening it now."
  LangString lectusFailedTitle ${LANG_ENGLISH} "${PRODUCTNAME} couldn't be installed"
  LangString lectusFailedBody ${LANG_ENGLISH} "Run the installer again. If it keeps happening, restart Windows and try once more."
  LangString lectusCloseApp ${LANG_ENGLISH} "${PRODUCTNAME} is open. Close it and continue installing?"
  LangString lectusDowngrade ${LANG_ENGLISH} "A newer version of ${PRODUCTNAME} ($InstalledVersion) is already installed. Replace it with version ${VERSION}?"
  LangString lectusWebView2 ${LANG_ENGLISH} "Installing Microsoft WebView2..."
  LangString lectusFontTitle ${LANG_ENGLISH} "Segoe UI Variable Display"
  LangString lectusFontBody ${LANG_ENGLISH} "Segoe UI Variable Text"
!endif
!ifdef LANG_PORTUGUESE
  LangString lectusPreparing ${LANG_PORTUGUESE} "A preparar o ${PRODUCTNAME}"
  LangString lectusUpdating ${LANG_PORTUGUESE} "A atualizar o ${PRODUCTNAME}"
  LangString lectusSubtitle ${LANG_PORTUGUESE} "Isto demora menos de um minuto."
  LangString lectusReady ${LANG_PORTUGUESE} "O ${PRODUCTNAME} está pronto"
  LangString lectusOpening ${LANG_PORTUGUESE} "A abrir agora."
  LangString lectusFailedTitle ${LANG_PORTUGUESE} "Não foi possível instalar o ${PRODUCTNAME}"
  LangString lectusFailedBody ${LANG_PORTUGUESE} "Execute o instalador novamente. Se continuar a acontecer, reinicie o Windows e tente outra vez."
  LangString lectusCloseApp ${LANG_PORTUGUESE} "O ${PRODUCTNAME} está aberto. Fechar e continuar a instalação?"
  LangString lectusDowngrade ${LANG_PORTUGUESE} "Já está instalada uma versão mais recente do ${PRODUCTNAME} ($InstalledVersion). Substituir pela versão ${VERSION}?"
  LangString lectusWebView2 ${LANG_PORTUGUESE} "A instalar o Microsoft WebView2..."
  LangString lectusFontTitle ${LANG_PORTUGUESE} "Segoe UI Variable Display"
  LangString lectusFontBody ${LANG_PORTUGUESE} "Segoe UI Variable Text"
!endif
!ifdef LANG_SIMPCHINESE
  LangString lectusPreparing ${LANG_SIMPCHINESE} "正在准备 ${PRODUCTNAME}"
  LangString lectusUpdating ${LANG_SIMPCHINESE} "正在更新 ${PRODUCTNAME}"
  LangString lectusSubtitle ${LANG_SIMPCHINESE} "这只需要不到一分钟。"
  LangString lectusReady ${LANG_SIMPCHINESE} "${PRODUCTNAME} 已准备就绪"
  LangString lectusOpening ${LANG_SIMPCHINESE} "正在打开。"
  LangString lectusFailedTitle ${LANG_SIMPCHINESE} "无法安装 ${PRODUCTNAME}"
  LangString lectusFailedBody ${LANG_SIMPCHINESE} "请重新运行安装程序。如果问题仍然出现，请重启 Windows 后再试一次。"
  LangString lectusCloseApp ${LANG_SIMPCHINESE} "${PRODUCTNAME} 正在运行。要关闭它并继续安装吗？"
  LangString lectusDowngrade ${LANG_SIMPCHINESE} "已安装较新版本的 ${PRODUCTNAME}（$InstalledVersion）。要替换为 ${VERSION} 版本吗？"
  LangString lectusWebView2 ${LANG_SIMPCHINESE} "正在安装 Microsoft WebView2..."
  LangString lectusFontTitle ${LANG_SIMPCHINESE} "Microsoft YaHei UI"
  LangString lectusFontBody ${LANG_SIMPCHINESE} "Microsoft YaHei UI"
!endif
!ifdef LANG_TRADCHINESE
  LangString lectusPreparing ${LANG_TRADCHINESE} "正在準備 ${PRODUCTNAME}"
  LangString lectusUpdating ${LANG_TRADCHINESE} "正在更新 ${PRODUCTNAME}"
  LangString lectusSubtitle ${LANG_TRADCHINESE} "這只需要不到一分鐘。"
  LangString lectusReady ${LANG_TRADCHINESE} "${PRODUCTNAME} 已準備就緒"
  LangString lectusOpening ${LANG_TRADCHINESE} "正在開啟。"
  LangString lectusFailedTitle ${LANG_TRADCHINESE} "無法安裝 ${PRODUCTNAME}"
  LangString lectusFailedBody ${LANG_TRADCHINESE} "請重新執行安裝程式。如果問題仍然發生，請重新啟動 Windows 後再試一次。"
  LangString lectusCloseApp ${LANG_TRADCHINESE} "${PRODUCTNAME} 正在執行。要關閉它並繼續安裝嗎？"
  LangString lectusDowngrade ${LANG_TRADCHINESE} "已安裝較新版本的 ${PRODUCTNAME}（$InstalledVersion）。要以 ${VERSION} 版取代嗎？"
  LangString lectusWebView2 ${LANG_TRADCHINESE} "正在安裝 Microsoft WebView2..."
  LangString lectusFontTitle ${LANG_TRADCHINESE} "Microsoft JhengHei UI"
  LangString lectusFontBody ${LANG_TRADCHINESE} "Microsoft JhengHei UI"
!endif

Function .onInit
  ${GetOptions} $CMDLINE "/P" $PassiveMode
  ${IfNot} ${Errors}
    StrCpy $PassiveMode 1
  ${EndIf}

  ${GetOptions} $CMDLINE "/NS" $NoShortcutMode
  ${IfNot} ${Errors}
    StrCpy $NoShortcutMode 1
  ${EndIf}

  ${GetOptions} $CMDLINE "/UPDATE" $UpdateMode
  ${IfNot} ${Errors}
    StrCpy $UpdateMode 1
  ${EndIf}

  ${GetOptions} $CMDLINE "/NOLAUNCH" $NoLaunchMode
  ${IfNot} ${Errors}
    StrCpy $NoLaunchMode 1
  ${EndIf}

  ; A plain double-click (neither /P nor /S) is one-click mode. It reuses every
  ; passive path (pages skipped, auto close) with the changes marked "One-click".
  ${If} $PassiveMode <> 1
  ${AndIfNot} ${Silent}
    StrCpy $OneClickMode 1
    StrCpy $PassiveMode 1
  ${EndIf}

  !if "${DISPLAYLANGUAGESELECTOR}" == "true"
    !insertmacro MUI_LANGDLL_DISPLAY
  !endif

  ; One-click: ask before closing a running Lectus, before anything changes, so a
  ; double-click never ends a dictation silently. The updater (/P) keeps the stock
  ; behaviour of closing it without asking.
  ${If} $OneClickMode = 1
    !if "${INSTALLMODE}" == "currentUser"
      nsis_tauri_utils::FindProcessCurrentUser "${MAINBINARYNAME}.exe"
    !else
      nsis_tauri_utils::FindProcess "${MAINBINARYNAME}.exe"
    !endif
    Pop $R0
    ${If} $R0 = 0
      MessageBox MB_OKCANCEL|MB_ICONINFORMATION "$(lectusCloseApp)" IDOK +2
      Quit
    ${EndIf}
  ${EndIf}

  !insertmacro SetContext

  ${If} $INSTDIR == "${PLACEHOLDER_INSTALL_DIR}"
    ; Set default install location
    !if "${INSTALLMODE}" == "perMachine"
      ${If} ${RunningX64}
        !if "${ARCH}" == "x64"
          StrCpy $INSTDIR "$PROGRAMFILES64\${PRODUCTNAME}"
        !else if "${ARCH}" == "arm64"
          StrCpy $INSTDIR "$PROGRAMFILES64\${PRODUCTNAME}"
        !else
          StrCpy $INSTDIR "$PROGRAMFILES\${PRODUCTNAME}"
        !endif
      ${Else}
        StrCpy $INSTDIR "$PROGRAMFILES\${PRODUCTNAME}"
      ${EndIf}
    !else if "${INSTALLMODE}" == "currentUser"
      StrCpy $INSTDIR "$LOCALAPPDATA\${PRODUCTNAME}"
    !endif

    Call RestorePreviousInstallLocation
  ${EndIf}


  !if "${INSTALLMODE}" == "both"
    !insertmacro MULTIUSER_INIT
  !endif
FunctionEnd


Section EarlyChecks
  ; Abort silent installer if downgrades is disabled
  !if "${ALLOWDOWNGRADES}" == "false"
  ${If} ${Silent}
    ; If downgrading
    ${If} $R0 = -1
      System::Call 'kernel32::AttachConsole(i -1)i.r0'
      ${If} $0 <> 0
        System::Call 'kernel32::GetStdHandle(i -11)i.r0'
        System::call 'kernel32::SetConsoleTextAttribute(i r0, i 0x0004)' ; set red color
        FileWrite $0 "$(silentDowngrades)"
      ${EndIf}
      Abort
    ${EndIf}
  ${EndIf}
  !endif

SectionEnd

Section WebView2
  ; Check if Webview2 is already installed and skip this section
  ${If} ${RunningX64}
    ReadRegStr $4 HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
  ${Else}
    ReadRegStr $4 HKLM "SOFTWARE\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
  ${EndIf}
  ${If} $4 == ""
    ReadRegStr $4 HKCU "SOFTWARE\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
  ${EndIf}

  ${If} $4 == ""
    ; Webview2 installation
    ;
    ; Skip if updating
    ${If} $UpdateMode <> 1
      !if "${INSTALLWEBVIEW2MODE}" == "downloadBootstrapper"
        Delete "$TEMP\MicrosoftEdgeWebview2Setup.exe"
        DetailPrint "$(webview2Downloading)"
        ; Quiet download: the stock one draws its own controls over the branded window.
        ${IfNot} ${Silent}
          SendMessage $LectusSubtitle ${WM_SETTEXT} 0 "STR:$(lectusWebView2)"
        ${EndIf}
        NSISdl::download_quiet "https://go.microsoft.com/fwlink/p/?LinkId=2124703" "$TEMP\MicrosoftEdgeWebview2Setup.exe"
        Pop $0
        ${If} $0 == "success"
          DetailPrint "$(webview2DownloadSuccess)"
        ${Else}
          DetailPrint "$(webview2DownloadError)"
          Abort "$(webview2AbortError)"
        ${EndIf}
        StrCpy $6 "$TEMP\MicrosoftEdgeWebview2Setup.exe"
        Goto install_webview2
      !endif

      !if "${INSTALLWEBVIEW2MODE}" == "embedBootstrapper"
        Delete "$TEMP\MicrosoftEdgeWebview2Setup.exe"
        File "/oname=$TEMP\MicrosoftEdgeWebview2Setup.exe" "${WEBVIEW2BOOTSTRAPPERPATH}"
        DetailPrint "$(installingWebview2)"
        StrCpy $6 "$TEMP\MicrosoftEdgeWebview2Setup.exe"
        Goto install_webview2
      !endif

      !if "${INSTALLWEBVIEW2MODE}" == "offlineInstaller"
        Delete "$TEMP\MicrosoftEdgeWebView2RuntimeInstaller.exe"
        File "/oname=$TEMP\MicrosoftEdgeWebView2RuntimeInstaller.exe" "${WEBVIEW2INSTALLERPATH}"
        DetailPrint "$(installingWebview2)"
        StrCpy $6 "$TEMP\MicrosoftEdgeWebView2RuntimeInstaller.exe"
        Goto install_webview2
      !endif

      Goto webview2_done

      install_webview2:
        DetailPrint "$(installingWebview2)"
        ; $6 holds the path to the webview2 installer
        ExecWait "$6 ${WEBVIEW2INSTALLERARGS} /install" $1
        ${If} $1 = 0
          DetailPrint "$(webview2InstallSuccess)"
        ${Else}
          DetailPrint "$(webview2InstallError)"
          Abort "$(webview2AbortError)"
        ${EndIf}
      webview2_done:
    ${EndIf}
  ${Else}
    !if "${MINIMUMWEBVIEW2VERSION}" != ""
      ${VersionCompare} "${MINIMUMWEBVIEW2VERSION}" "$4" $R0
      ${If} $R0 = 1
        update_webview:
          DetailPrint "$(installingWebview2)"
          ${If} ${RunningX64}
            ReadRegStr $R1 HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate" "path"
          ${Else}
            ReadRegStr $R1 HKLM "SOFTWARE\Microsoft\EdgeUpdate" "path"
          ${EndIf}
          ${If} $R1 == ""
            ReadRegStr $R1 HKCU "SOFTWARE\Microsoft\EdgeUpdate" "path"
          ${EndIf}
          ${If} $R1 != ""
            ; Chromium updater docs: https://source.chromium.org/chromium/chromium/src/+/main:docs/updater/user_manual.md
            ; Modified from "HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Microsoft EdgeWebView\ModifyPath"
            ExecWait `"$R1" /install appguid=${WEBVIEW2APPGUID}&needsadmin=true` $1
            ${If} $1 = 0
              DetailPrint "$(webview2InstallSuccess)"
            ${Else}
              MessageBox MB_ICONEXCLAMATION|MB_ABORTRETRYIGNORE "$(webview2InstallError)" IDIGNORE ignore IDRETRY update_webview
              Quit
              ignore:
            ${EndIf}
          ${EndIf}
      ${EndIf}
    !endif
  ${EndIf}
SectionEnd

Section Install
  SetOutPath $INSTDIR

  !ifmacrodef NSIS_HOOK_PREINSTALL
    !insertmacro NSIS_HOOK_PREINSTALL
  !endif

  !insertmacro CheckIfAppIsRunning "${MAINBINARYNAME}.exe" "${PRODUCTNAME}"

  ; Lectus: the app loads every DLL in backends\, so drop the previous version's
  ; set before copying the current one (in-place upgrades would keep stale DLLs).
  Delete "$INSTDIR\backends\*.dll"

  ; Copy main executable
  File "${MAINBINARYSRCPATH}"

  ; Copy resources
  {{#each resources_dirs}}
    CreateDirectory "$INSTDIR\\{{this}}"
  {{/each}}
  {{#each resources}}
    File /a "/oname={{this.[1]}}" "{{no-escape @key}}"
  {{/each}}

  ; Copy external binaries
  {{#each binaries}}
    File /a "/oname={{this}}" "{{no-escape @key}}"
  {{/each}}

  ; Create file associations
  {{#each file_associations as |association| ~}}
    {{#each association.ext as |ext| ~}}
       !insertmacro APP_ASSOCIATE "{{ext}}" "{{or association.name ext}}" "{{association-description association.description ext}}" "$INSTDIR\${MAINBINARYNAME}.exe,0" "Open with ${PRODUCTNAME}" "$INSTDIR\${MAINBINARYNAME}.exe $\"%1$\""
    {{/each}}
  {{/each}}

  ; Register deep links
  {{#each deep_link_protocols as |protocol| ~}}
    WriteRegStr SHCTX "Software\Classes\\{{protocol}}" "URL Protocol" ""
    WriteRegStr SHCTX "Software\Classes\\{{protocol}}" "" "URL:${BUNDLEID} protocol"
    WriteRegStr SHCTX "Software\Classes\\{{protocol}}\DefaultIcon" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0"
    WriteRegStr SHCTX "Software\Classes\\{{protocol}}\shell\open\command" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""
  {{/each}}

  ; Create uninstaller
  WriteUninstaller "$INSTDIR\uninstall.exe"

  ; Save $INSTDIR in registry for future installations
  WriteRegStr SHCTX "${MANUPRODUCTKEY}" "" $INSTDIR

  !if "${INSTALLMODE}" == "both"
    ; Save install mode to be selected by default for the next installation such as updating
    ; or when uninstalling
    WriteRegStr SHCTX "${UNINSTKEY}" $MultiUser.InstallMode 1
  !endif

  ; Remove old main binary if it doesn't match new main binary name
  ReadRegStr $OldMainBinaryName SHCTX "${UNINSTKEY}" "MainBinaryName"
  ${If} $OldMainBinaryName != ""
  ${AndIf} $OldMainBinaryName != "${MAINBINARYNAME}.exe"
    Delete "$INSTDIR\$OldMainBinaryName"
  ${EndIf}

  ; Save current MAINBINARYNAME for future updates
  WriteRegStr SHCTX "${UNINSTKEY}" "MainBinaryName" "${MAINBINARYNAME}.exe"

  ; Registry information for add/remove programs
  WriteRegStr SHCTX "${UNINSTKEY}" "DisplayName" "${PRODUCTNAME}"
  WriteRegStr SHCTX "${UNINSTKEY}" "DisplayIcon" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\""
  WriteRegStr SHCTX "${UNINSTKEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr SHCTX "${UNINSTKEY}" "Publisher" "${MANUFACTURER}"
  WriteRegStr SHCTX "${UNINSTKEY}" "InstallLocation" "$\"$INSTDIR$\""
  WriteRegStr SHCTX "${UNINSTKEY}" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
  WriteRegDWORD SHCTX "${UNINSTKEY}" "NoModify" "1"
  WriteRegDWORD SHCTX "${UNINSTKEY}" "NoRepair" "1"

  ${GetSize} "$INSTDIR" "/M=uninstall.exe /S=0K /G=0" $0 $1 $2
  IntOp $0 $0 + ${ESTIMATEDSIZE}
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD SHCTX "${UNINSTKEY}" "EstimatedSize" "$0"

  !if "${HOMEPAGE}" != ""
    WriteRegStr SHCTX "${UNINSTKEY}" "URLInfoAbout" "${HOMEPAGE}"
    WriteRegStr SHCTX "${UNINSTKEY}" "URLUpdateInfo" "${HOMEPAGE}"
    WriteRegStr SHCTX "${UNINSTKEY}" "HelpLink" "${HOMEPAGE}"
  !endif

  ; Create start menu shortcut
  !insertmacro MUI_STARTMENU_WRITE_BEGIN Application
    Call CreateOrUpdateStartMenuShortcut
  !insertmacro MUI_STARTMENU_WRITE_END

  ; Create desktop shortcut for silent and passive installers
  ; because finish page will be skipped.
  ; One-click: only on a first install, so a removed shortcut stays removed.
  ${If} $OneClickMode = 1
    ${If} $FreshInstall = 1
      Call CreateOrUpdateDesktopShortcut
    ${EndIf}
  ${ElseIf} $PassiveMode = 1
  ${OrIf} ${Silent}
    Call CreateOrUpdateDesktopShortcut
  ${EndIf}

  !ifmacrodef NSIS_HOOK_POSTINSTALL
    !insertmacro NSIS_HOOK_POSTINSTALL
  !endif

  ; Auto close this page for passive mode
  ${If} $PassiveMode = 1
    SetAutoClose true
  ${EndIf}
SectionEnd

; Lectus: final hidden section. It runs on the install thread, so the window can
; repaint the "ready" state while it waits.
Section "-LectusReady"
  ${IfNot} ${Silent}
    ; Subtitle: "Opening it now." when the app is about to start, else empty.
    StrCpy $0 ""
    ${If} $OneClickMode = 1
      ${IfThen} $NoLaunchMode <> 1 ${|} StrCpy $0 "$(lectusOpening)" ${|}
    ${Else}
      ClearErrors
      ${GetOptions} $CMDLINE "/R" $1
      ${IfNotThen} ${Errors} ${|} StrCpy $0 "$(lectusOpening)" ${|}
    ${EndIf}
    SendMessage $LectusProgress ${PBM_SETPOS} 30000 0
    SendMessage $LectusTitle ${WM_SETTEXT} 0 "STR:$(lectusReady)"
    SendMessage $LectusSubtitle ${WM_SETTEXT} 0 "STR:$0"
    Sleep 700
  ${EndIf}
SectionEnd

Function .onInstSuccess
  ; One-click opens the app once, in front (it lands in the setup wizard on a
  ; first install). /NOLAUNCH skips it.
  ${If} $OneClickMode = 1
    ${If} $NoLaunchMode <> 1
      System::Call 'user32::AllowSetForegroundWindow(i -1)'
      nsis_tauri_utils::RunAsUser "$INSTDIR\${MAINBINARYNAME}.exe" ""
    ${EndIf}
    Return
  ${EndIf}

  ; Check for `/R` flag only in silent and passive installers because
  ; GUI installer has a toggle for the user to (re)start the app
  ${If} $PassiveMode = 1
  ${OrIf} ${Silent}
    ${GetOptions} $CMDLINE "/R" $R0
    ${IfNot} ${Errors}
      ${GetOptions} $CMDLINE "/ARGS" $R0
      nsis_tauri_utils::RunAsUser "$INSTDIR\${MAINBINARYNAME}.exe" "$R0"
    ${EndIf}
  ${EndIf}
FunctionEnd

Function un.onInit
  !insertmacro SetContext

  !if "${INSTALLMODE}" == "both"
    !insertmacro MULTIUSER_UNINIT
  !endif

  !insertmacro MUI_UNGETLANGUAGE

  ${GetOptions} $CMDLINE "/P" $PassiveMode
  ${IfNot} ${Errors}
    StrCpy $PassiveMode 1
  ${EndIf}

  ${GetOptions} $CMDLINE "/UPDATE" $UpdateMode
  ${IfNot} ${Errors}
    StrCpy $UpdateMode 1
  ${EndIf}
FunctionEnd

Section Uninstall

  !ifmacrodef NSIS_HOOK_PREUNINSTALL
    !insertmacro NSIS_HOOK_PREUNINSTALL
  !endif

  !insertmacro CheckIfAppIsRunning "${MAINBINARYNAME}.exe" "${PRODUCTNAME}"

  ; Delete the app directory and its content from disk
  ; Copy main executable
  Delete "$INSTDIR\${MAINBINARYNAME}.exe"

  ; Delete resources
  {{#each resources}}
    Delete "$INSTDIR\\{{this.[1]}}"
  {{/each}}

  ; Delete external binaries
  {{#each binaries}}
    Delete "$INSTDIR\\{{this}}"
  {{/each}}

  ; Delete app associations
  {{#each file_associations as |association| ~}}
    {{#each association.ext as |ext| ~}}
      !insertmacro APP_UNASSOCIATE "{{ext}}" "{{or association.name ext}}"
    {{/each}}
  {{/each}}

  ; Delete deep links
  {{#each deep_link_protocols as |protocol| ~}}
    ReadRegStr $R7 SHCTX "Software\Classes\\{{protocol}}\shell\open\command" ""
    ${If} $R7 == "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""
      DeleteRegKey SHCTX "Software\Classes\\{{protocol}}"
    ${EndIf}
  {{/each}}


  ; Delete uninstaller
  Delete "$INSTDIR\uninstall.exe"

  {{#each resources_ancestors}}
  RMDir /REBOOTOK "$INSTDIR\\{{this}}"
  {{/each}}
  RMDir "$INSTDIR"

  ; Remove shortcuts if not updating
  ${If} $UpdateMode <> 1
    !insertmacro DeleteAppUserModelId

    ; Remove start menu shortcut
    !insertmacro MUI_STARTMENU_GETFOLDER Application $AppStartMenuFolder
    !insertmacro IsShortcutTarget "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    Pop $0
    ${If} $0 = 1
      !insertmacro UnpinShortcut "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk"
      Delete "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk"
      RMDir "$SMPROGRAMS\$AppStartMenuFolder"
    ${EndIf}
    !insertmacro IsShortcutTarget "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    Pop $0
    ${If} $0 = 1
      !insertmacro UnpinShortcut "$SMPROGRAMS\${PRODUCTNAME}.lnk"
      Delete "$SMPROGRAMS\${PRODUCTNAME}.lnk"
    ${EndIf}

    ; Remove desktop shortcuts
    !insertmacro IsShortcutTarget "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    Pop $0
    ${If} $0 = 1
      !insertmacro UnpinShortcut "$DESKTOP\${PRODUCTNAME}.lnk"
      Delete "$DESKTOP\${PRODUCTNAME}.lnk"
    ${EndIf}
  ${EndIf}

  ; Remove registry information for add/remove programs
  !if "${INSTALLMODE}" == "both"
    DeleteRegKey SHCTX "${UNINSTKEY}"
  !else if "${INSTALLMODE}" == "perMachine"
    DeleteRegKey HKLM "${UNINSTKEY}"
  !else
    DeleteRegKey HKCU "${UNINSTKEY}"
  !endif

  ; Removes the Autostart entry for ${PRODUCTNAME} from the HKCU Run key if it exists.
  ; This ensures the program does not launch automatically after uninstallation if it exists.
  ; If it doesn't exist, it does nothing.
  ; We do this when not updating (to preserve the registry value on updates)
  ${If} $UpdateMode <> 1
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}"
  ${EndIf}

  ; Delete app data if the checkbox is selected
  ; and if not updating
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    ; Clear the install location $INSTDIR from registry
    DeleteRegKey SHCTX "${MANUPRODUCTKEY}"
    DeleteRegKey /ifempty SHCTX "${MANUKEY}"

    ; Clear the install language from registry
    DeleteRegValue HKCU "${MANUPRODUCTKEY}" "Installer Language"
    DeleteRegKey /ifempty HKCU "${MANUPRODUCTKEY}"
    DeleteRegKey /ifempty HKCU "${MANUKEY}"

    SetShellVarContext current
    RmDir /r "$APPDATA\${BUNDLEID}"
    RmDir /r "$LOCALAPPDATA\${BUNDLEID}"
  ${EndIf}

  !ifmacrodef NSIS_HOOK_POSTUNINSTALL
    !insertmacro NSIS_HOOK_POSTUNINSTALL
  !endif

  ; Auto close if passive mode or updating
  ${If} $PassiveMode = 1
  ${OrIf} $UpdateMode = 1
    SetAutoClose true
  ${EndIf}
SectionEnd

Function RestorePreviousInstallLocation
  ReadRegStr $4 SHCTX "${MANUPRODUCTKEY}" ""
  StrCmp $4 "" +2 0
    StrCpy $INSTDIR $4
FunctionEnd

Function Skip
  Abort
FunctionEnd

Function SkipIfPassive
  ${IfThen} $PassiveMode = 1  ${|} Abort ${|}
FunctionEnd
Function un.SkipIfPassive
  ${IfThen} $PassiveMode = 1  ${|} Abort ${|}
FunctionEnd

Function CreateOrUpdateStartMenuShortcut
  ; We used to use product name as MAINBINARYNAME
  ; migrate old shortcuts to target the new MAINBINARYNAME
  StrCpy $R0 0

  !insertmacro IsShortcutTarget "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk" "$INSTDIR\$OldMainBinaryName"
  Pop $0
  ${If} $0 = 1
    !insertmacro SetShortcutTarget "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    StrCpy $R0 1
  ${EndIf}

  !insertmacro IsShortcutTarget "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\$OldMainBinaryName"
  Pop $0
  ${If} $0 = 1
    !insertmacro SetShortcutTarget "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    StrCpy $R0 1
  ${EndIf}

  ${If} $R0 = 1
    Return
  ${EndIf}

  ; Skip creating shortcut if in update mode or no shortcut mode
  ; but always create if migrating from wix
  ${If} $WixMode = 0
    ${If} $UpdateMode = 1
    ${OrIf} $NoShortcutMode = 1
      Return
    ${EndIf}
  ${EndIf}

  !if "${STARTMENUFOLDER}" != ""
    CreateDirectory "$SMPROGRAMS\$AppStartMenuFolder"
    CreateShortcut "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    !insertmacro SetLnkAppUserModelId "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk"
  !else
    CreateShortcut "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    !insertmacro SetLnkAppUserModelId "$SMPROGRAMS\${PRODUCTNAME}.lnk"
  !endif
FunctionEnd

Function CreateOrUpdateDesktopShortcut
  ; We used to use product name as MAINBINARYNAME
  ; migrate old shortcuts to target the new MAINBINARYNAME
  !insertmacro IsShortcutTarget "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\$OldMainBinaryName"
  Pop $0
  ${If} $0 = 1
    !insertmacro SetShortcutTarget "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    Return
  ${EndIf}

  ; Skip creating shortcut if in update mode or no shortcut mode
  ; but always create if migrating from wix
  ${If} $WixMode = 0
    ${If} $UpdateMode = 1
    ${OrIf} $NoShortcutMode = 1
      Return
    ${EndIf}
  ${EndIf}

  CreateShortcut "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
  !insertmacro SetLnkAppUserModelId "$DESKTOP\${PRODUCTNAME}.lnk"
FunctionEnd

; ---------------------------------------------------------------------------
; Lectus branded window
; One small borderless window: logo, title, subtitle and a thin progress bar.
; Colours follow src/styles/tokens.css (accent #2F6FED).
; ---------------------------------------------------------------------------

; OUT = V device-independent pixels at the window DPI.
!macro LectusScale OUT V
  IntOp ${OUT} ${V} * $LectusDpi
  IntOp ${OUT} ${OUT} / 96
!macroend

; SetCtlColors only takes literal colours, so branch on the theme here.
!macro LectusSetColors HWND MUTED
  ${If} $LectusDark = 1
    !if "${MUTED}" == "1"
      SetCtlColors ${HWND} "9AA0AB" "1B1C22"
    !else
      SetCtlColors ${HWND} "F2F3F5" "1B1C22"
    !endif
  ${Else}
    !if "${MUTED}" == "1"
      SetCtlColors ${HWND} "5F6470" "FFFFFF"
    !else
      SetCtlColors ${HWND} "17181C" "FFFFFF"
    !endif
  ${EndIf}
!macroend

; Hide every direct child of the main window except the current page dialog.
Function LectusHideChrome
  FindWindow $1 "#32770" "" $HWNDPARENT
  StrCpy $0 0
  lectus_hide_next:
    System::Call 'user32::FindWindowExW(p $HWNDPARENT, p r0, p 0, p 0) p .r0'
    IntCmp $0 0 lectus_hide_done
    IntCmp $0 $1 lectus_hide_next
    ShowWindow $0 ${SW_HIDE}
    Goto lectus_hide_next
  lectus_hide_done:
FunctionEnd

Function LectusGuiInit
  ; Theme follows the Windows app mode unless /LECTUSTHEME= overrides it.
  StrCpy $LectusDark 0
  ClearErrors
  ReadRegDWORD $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Themes\Personalize" "AppsUseLightTheme"
  ${IfNot} ${Errors}
    ${If} $0 = 0
      StrCpy $LectusDark 1
    ${EndIf}
  ${EndIf}
  ClearErrors
  ${GetOptions} $CMDLINE "/LECTUSTHEME=" $0
  ${IfNot} ${Errors}
    ${If} $0 == "dark"
      StrCpy $LectusDark 1
    ${ElseIf} $0 == "light"
      StrCpy $LectusDark 0
    ${EndIf}
  ${EndIf}
  ; COLORREF values (0x00BBGGRR): progress track and the Windows 11 border.
  ${If} $LectusDark = 1
    StrCpy $LectusTrack 0x00362E2C
    StrCpy $LectusBorder 0x003F3634
  ${Else}
    StrCpy $LectusTrack 0x00EEE8E6
    StrCpy $LectusBorder 0x00E3DCD9
  ${EndIf}

  ; Segoe UI Variable ships with Windows 11; Windows 10 falls back to Segoe UI.
  StrCpy $LectusFaceTitle "$(lectusFontTitle)"
  StrCpy $LectusFaceBody "$(lectusFontBody)"
  ${IfNot} ${FileExists} "$FONTS\SegUIVar.ttf"
    ${If} $LectusFaceTitle == "Segoe UI Variable Display"
      StrCpy $LectusFaceTitle "Segoe UI"
      StrCpy $LectusFaceBody "Segoe UI"
    ${EndIf}
  ${EndIf}

  ; DPI of the monitor the window is on (PerMonitorV2 manifest); 96 before
  ; Windows 10 1607, where GetDpiForWindow does not exist.
  System::Call 'user32::GetDpiForWindow(p $HWNDPARENT) i .r0'
  ${If} $0 == "error"
  ${OrIf} $0 == ""
  ${OrIf} $0 = 0
    StrCpy $0 96
  ${EndIf}
  StrCpy $LectusDpi $0

  ; No caption, sizing frame, system menu or 3D edges; keep a taskbar button.
  System::Call 'user32::GetWindowLongW(p $HWNDPARENT, i -16) i .r0'
  IntOp $1 0x00CF0080 ~
  IntOp $0 $0 & $1
  System::Call 'user32::SetWindowLongW(p $HWNDPARENT, i -16, i r0)'
  System::Call 'user32::GetWindowLongW(p $HWNDPARENT, i -20) i .r0'
  IntOp $1 0x00020301 ~
  IntOp $0 $0 & $1
  IntOp $0 $0 | 0x00040000
  System::Call 'user32::SetWindowLongW(p $HWNDPARENT, i -20, i r0)'

  ; Size it and centre it in the work area of its monitor.
  !insertmacro LectusScale $2 ${LECTUS_WIN_W}
  !insertmacro LectusScale $3 ${LECTUS_WIN_H}
  System::Call 'user32::MonitorFromWindow(p $HWNDPARENT, i 2) p .r4'
  System::Call '*(i 40, i 0, i 0, i 0, i 0, i 0, i 0, i 0, i 0, i 0) p .r5'
  System::Call 'user32::GetMonitorInfoW(p r4, p r5)'
  System::Call '*$5(i, i, i, i, i, i .r6, i .r7, i .r8, i .r9, i)'
  System::Free $5
  IntOp $8 $8 - $6
  IntOp $8 $8 - $2
  IntOp $8 $8 / 2
  IntOp $8 $8 + $6
  IntOp $9 $9 - $7
  IntOp $9 $9 - $3
  IntOp $9 $9 / 2
  IntOp $9 $9 + $7
  System::Call 'user32::SetWindowPos(p $HWNDPARENT, p 0, i r8, i r9, i r2, i r3, i 0x0024)'

  ; Windows 11: rounded corners and a 1px border (ignored on Windows 10).
  System::Call 'dwmapi::DwmSetWindowAttribute(p $HWNDPARENT, i 33, *i 2, i 4)'
  System::Call 'dwmapi::DwmSetWindowAttribute(p $HWNDPARENT, i 34, *i $LectusBorder, i 4)'

  !insertmacro LectusSetColors $HWNDPARENT 0
  Call LectusHideChrome
FunctionEnd

Function LectusInstFilesShow
  FindWindow $LectusPage "#32770" "" $HWNDPARENT
  ; Pages re-show the stock buttons, so hide the chrome again here.
  Call LectusHideChrome

  ; The page dialog keeps its template size: stretch it over the whole window.
  System::Call '*(i 0, i 0, i 0, i 0) p .r0'
  System::Call 'user32::GetClientRect(p $HWNDPARENT, p r0)'
  System::Call '*$0(i, i, i .r1, i .r2)'
  System::Free $0
  StrCpy $5 $1
  System::Call 'user32::SetWindowPos(p $LectusPage, p 0, i 0, i 0, i r1, i r2, i 0x0014)'
  !insertmacro LectusSetColors $LectusPage 0

  ; Stock page controls: status line, details list and details button.
  GetDlgItem $0 $LectusPage 1006
  ShowWindow $0 ${SW_HIDE}
  GetDlgItem $0 $LectusPage 1016
  ShowWindow $0 ${SW_HIDE}
  GetDlgItem $0 $LectusPage 1027
  ShowWindow $0 ${SW_HIDE}

  ; Logo: the installer's own icon resource (103) at 64 DIP. icon.ico carries
  ; a 256px frame, so it stays sharp at 200%.
  !insertmacro LectusScale $3 64
  System::Call 'kernel32::GetModuleHandleW(p 0) p .r6'
  System::Call 'comctl32::LoadIconWithScaleDown(p r6, p 103, i r3, i r3, *p .r7) i .r8'
  ${If} $8 != 0
    System::Call 'user32::LoadImageW(p r6, p 103, i 1, i r3, i r3, i 0) p .r7'
  ${EndIf}
  IntOp $0 $5 - $3
  IntOp $0 $0 / 2
  !insertmacro LectusScale $4 40
  System::Call 'user32::CreateWindowExW(i 0, w "STATIC", w "", i 0x50000003, i r0, i r4, i r3, i r3, p $LectusPage, p 0, p 0, p 0) p .r9'
  SendMessage $9 ${STM_SETIMAGE} 1 $7
  !insertmacro LectusSetColors $9 0

  ; Fonts: title 18pt semibold, subtitle 10pt (negative height = character height).
  IntOp $0 $LectusDpi * 18
  IntOp $0 $0 / 72
  IntOp $0 0 - $0
  System::Call 'gdi32::CreateFontW(i r0, i 0, i 0, i 0, i 600, i 0, i 0, i 0, i 1, i 0, i 0, i 5, i 0, w "$LectusFaceTitle") p .r6'
  IntOp $0 $LectusDpi * 10
  IntOp $0 $0 / 72
  IntOp $0 0 - $0
  System::Call 'gdi32::CreateFontW(i r0, i 0, i 0, i 0, i 400, i 0, i 0, i 0, i 1, i 0, i 0, i 5, i 0, w "$LectusFaceBody") p .r7'

  ; Text column: full width minus 24 DIP on each side, centred.
  !insertmacro LectusScale $0 24
  IntOp $1 $5 - $0
  IntOp $1 $1 - $0

  ${If} $UpdateMode = 1
    StrCpy $8 "$(lectusUpdating)"
  ${Else}
    StrCpy $8 "$(lectusPreparing)"
  ${EndIf}
  !insertmacro LectusScale $3 118
  !insertmacro LectusScale $4 34
  System::Call 'user32::CreateWindowExW(i 0, w "STATIC", w r8, i 0x50000081, i r0, i r3, i r1, i r4, p $LectusPage, p 0, p 0, p 0) p .r9'
  StrCpy $LectusTitle $9
  SendMessage $LectusTitle ${WM_SETFONT} $6 1
  !insertmacro LectusSetColors $LectusTitle 0

  StrCpy $8 "$(lectusSubtitle)"
  !insertmacro LectusScale $3 154
  !insertmacro LectusScale $4 44
  System::Call 'user32::CreateWindowExW(i 0, w "STATIC", w r8, i 0x50000081, i r0, i r3, i r1, i r4, p $LectusPage, p 0, p 0, p 0) p .r9'
  StrCpy $LectusSubtitle $9
  SendMessage $LectusSubtitle ${WM_SETFONT} $7 1
  !insertmacro LectusSetColors $LectusSubtitle 1

  ; Progress: a flat 4 DIP bar in the accent colour (unthemed, no border).
  GetDlgItem $LectusProgress $LectusPage 1004
  System::Call 'uxtheme::SetWindowTheme(p $LectusProgress, w "", w "")'
  System::Call 'user32::GetWindowLongW(p $LectusProgress, i -16) i .r0'
  IntOp $1 0x00800000 ~
  IntOp $0 $0 & $1
  IntOp $0 $0 | 1
  System::Call 'user32::SetWindowLongW(p $LectusProgress, i -16, i r0)'
  System::Call 'user32::GetWindowLongW(p $LectusProgress, i -20) i .r0'
  IntOp $1 0x00020200 ~
  IntOp $0 $0 & $1
  System::Call 'user32::SetWindowLongW(p $LectusProgress, i -20, i r0)'
  SendMessage $LectusProgress ${PBM_SETBARCOLOR} 0 ${LECTUS_ACCENT}
  SendMessage $LectusProgress ${PBM_SETBKCOLOR} 0 $LectusTrack
  !insertmacro LectusScale $0 200
  IntOp $1 $5 - $0
  IntOp $1 $1 / 2
  !insertmacro LectusScale $3 206
  !insertmacro LectusScale $4 4
  System::Call 'user32::SetWindowPos(p $LectusProgress, p 0, i r1, i r3, i r0, i r4, i 0x0034)'
FunctionEnd

; Runs as soon as the sections end. On failure the window would otherwise stay
; up with no way out, so show what happened and a Close button.
Function LectusInstFilesLeave
  IfAbort 0 lectus_leave_done
    ; A smaller title so longer translations fit on one line.
    IntOp $0 $LectusDpi * 15
    IntOp $0 $0 / 72
    IntOp $0 0 - $0
    System::Call 'gdi32::CreateFontW(i r0, i 0, i 0, i 0, i 600, i 0, i 0, i 0, i 1, i 0, i 0, i 5, i 0, w "$LectusFaceTitle") p .r1'
    SendMessage $LectusTitle ${WM_SETFONT} $1 1
    SendMessage $LectusTitle ${WM_SETTEXT} 0 "STR:$(lectusFailedTitle)"
    SendMessage $LectusSubtitle ${WM_SETTEXT} 0 "STR:$(lectusFailedBody)"
    ShowWindow $LectusProgress ${SW_HIDE}

    ; Shorten the page dialog to make room for the stock Cancel button, centred
    ; at the bottom.
    System::Call '*(i 0, i 0, i 0, i 0) p .r0'
    System::Call 'user32::GetClientRect(p $HWNDPARENT, p r0)'
    System::Call '*$0(i, i, i .r1, i .r2)'
    System::Free $0
    !insertmacro LectusScale $3 56
    IntOp $4 $2 - $3
    System::Call 'user32::SetWindowPos(p $LectusPage, p 0, i 0, i 0, i r1, i r4, i 0x0016)'

    ; NSIS sets this button's (localized) "Cancel" text after this callback, so
    ; the failure text above does not name the button.
    GetDlgItem $0 $HWNDPARENT 2
    EnableWindow $0 1
    !insertmacro LectusScale $5 104
    !insertmacro LectusScale $6 30
    IntOp $7 $1 - $5
    IntOp $7 $7 / 2
    !insertmacro LectusScale $8 44
    IntOp $8 $2 - $8
    System::Call 'user32::SetWindowPos(p r0, p 0, i r7, i r8, i r5, i r6, i 0x0014)'
    ShowWindow $0 ${SW_SHOW}
    SendMessage $HWNDPARENT ${WM_NEXTDLGCTL} $0 1
  lectus_leave_done:
FunctionEnd
