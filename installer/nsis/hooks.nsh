; FastFileOCR extensions to Tauri's version-pinned NSIS template.
!include nsDialogs.nsh
!include LogicLib.nsh
!include WinVer.nsh
!ifndef FFO_REGKEY
!define FFO_REGKEY "Software\FastFileOCR"
!endif
!ifndef FFO_HELPER
!define FFO_HELPER "${__FILEDIR__}\..\..\src-tauri\resources\installer\fastfileocr-setup-helper.exe"
!endif
Var FfoExplicitLanguage
Var FfoRoot
Var FfoFresh
Var FfoInput
Var FfoRadio
Var FfoKeepRadio
Var FfoDialog
Var FfoResult
Var FfoStatus
Var FfoRemoveData
Var FfoRemoveDocs
Var FfoDataCheck
Var FfoDocsCheck
Var FfoUnRoot

Function FfoSelectLanguage
  StrCpy $FfoExplicitLanguage 1
  ClearErrors
  ${GetOptions} $CMDLINE "/LANGUAGE=" $0
  ${If} $0 == "1033"
  ${OrIf} $0 == "1041"
  ${OrIf} $0 == "1042"
    StrCpy $LANGUAGE $0
    Return
  ${EndIf}
  IfSilent language_silent
  StrCpy $FfoExplicitLanguage 0
  Return
  language_silent:
    ReadRegStr $0 HKCU "${FFO_REGKEY}" "Language"
    StrCpy $LANGUAGE 1033
    ${If} $0 == "ko"
      StrCpy $LANGUAGE 1042
    ${ElseIf} $0 == "ja"
      StrCpy $LANGUAGE 1041
    ${EndIf}
FunctionEnd

Function FfoInit
  ${IfNot} ${AtLeastWin10}
    IfSilent +2
    MessageBox MB_OK|MB_ICONSTOP "$(installWindowsRequired)"
    SetErrorLevel 1
    Quit
  ${EndIf}
  InitPluginsDir
  StrCpy $FfoResult "$PLUGINSDIR\fastfileocr-result.ini"
  File /oname=$PLUGINSDIR\fastfileocr-setup-helper.exe "${FFO_HELPER}"
  ReadRegStr $FfoRoot HKCU "${FFO_REGKEY}" "DataDir"
  ${If} $FfoRoot == ""
    StrCpy $FfoRoot "$LOCALAPPDATA\FastFileOCR"
  ${EndIf}
  ${GetOptions} $CMDLINE "/DATADIR=" $0
  ${If} $0 != ""
    StrCpy $FfoRoot $0
  ${EndIf}
  StrCpy $FfoFresh 0
  ${GetOptions} $CMDLINE "/FRESH=" $0
  ${If} $0 == "1"
    StrCpy $FfoFresh 1
  ${EndIf}
FunctionEnd

Function FfoResolve
  Delete "$FfoResult"
  nsExec::ExecToStack '"$PLUGINSDIR\fastfileocr-setup-helper.exe" resolve --root "$FfoRoot" --app "$INSTDIR" --result "$FfoResult"'
  Pop $0
  Pop $1
  ReadINIStr $FfoStatus "$FfoResult" "Result" "Status"
  ${If} $0 == 0
  ${AndIf} $FfoStatus == "ok"
    ReadINIStr $FfoRoot "$FfoResult" "Result" "Root"
  ${Else}
    StrCpy $FfoStatus "unsafe"
  ${EndIf}
FunctionEnd

Function FfoError
  IfSilent silent_error
  ${If} $FfoStatus == "write"
    MessageBox MB_OK|MB_ICONSTOP "$(installWriteError)"
  ${Else}
    MessageBox MB_OK|MB_ICONSTOP "$(installUnsafe)$\r$\n$FfoRoot"
  ${EndIf}
  silent_error:
FunctionEnd

Function FfoBrowse
  Pop $0 ; nsDialogs callback control handle
  nsDialogs::SelectFolderDialog "$(installDataTitle)" "$FfoRoot"
  Pop $0
  ${If} $0 != "error"
    ${NSD_SetText} $FfoInput $0
  ${EndIf}
FunctionEnd

Function FfoDataPage
  ${If} $PassiveMode == 1
    Abort
  ${EndIf}
  !insertmacro MUI_HEADER_TEXT "$(installDataTitle)" "$(installDataDescription)"
  nsDialogs::Create 1018
  Pop $FfoDialog
  ${NSD_CreateLabel} 0 0 100% 48u "$(installDataHint)"
  Pop $0
  ${NSD_CreateDirRequest} 0 56u 78% 14u "$FfoRoot"
  Pop $FfoInput
  ${NSD_CreateBrowseButton} 80% 56u 20% 14u "$(installBrowse)"
  Pop $0
  ${NSD_OnClick} $0 FfoBrowse
  nsDialogs::Show
FunctionEnd

Function FfoDataLeave
  ${NSD_GetText} $FfoInput $FfoRoot
  Call FfoResolve
  ${If} $FfoStatus != "ok"
    Call FfoError
    Abort
  ${EndIf}
FunctionEnd

Function FfoModePage
  ${If} $PassiveMode == 1
    Abort
  ${EndIf}
  !insertmacro MUI_HEADER_TEXT "$(installModeTitle)" "$(installModeDescription)"
  nsDialogs::Create 1018
  Pop $FfoDialog
  ${NSD_CreateLabel} 0 0 100% 28u "$(installDataResolved)$\r$\n$FfoRoot"
  Pop $0
  ${NSD_CreateLabel} 0 36u 100% 28u "$(installModeHint)"
  Pop $0
  ${NSD_CreateRadioButton} 0 74u 100% 24u "$(installKeep)"
  Pop $FfoKeepRadio
  ${NSD_CreateRadioButton} 0 106u 100% 24u "$(installFresh)"
  Pop $FfoRadio
  ${If} $FfoFresh == 1
    ${NSD_Check} $FfoRadio
  ${Else}
    ${NSD_Check} $FfoKeepRadio
  ${EndIf}
  nsDialogs::Show
FunctionEnd

Function FfoModeLeave
  ${NSD_GetState} $FfoRadio $FfoFresh
  ${If} $FfoFresh == 1
    MessageBox MB_YESNO|MB_ICONQUESTION|MB_DEFBUTTON2 "$(installResetConfirm)" IDYES fresh_confirmed
    Abort
    fresh_confirmed:
  ${EndIf}
FunctionEnd

!macro NSIS_HOOK_PREINSTALL
  Call FfoResolve
  ${If} $FfoStatus != "ok"
    Call FfoError
    SetErrorLevel 1
    Quit
  ${EndIf}
  Delete "$FfoResult"
  nsExec::ExecToStack '"$PLUGINSDIR\fastfileocr-setup-helper.exe" prepare --root "$FfoRoot" --app "$INSTDIR" --fresh "$FfoFresh" --result "$FfoResult"'
  Pop $0
  Pop $1
  ReadINIStr $FfoStatus "$FfoResult" "Result" "Status"
  ${If} $0 != 0
  ${OrIf} $FfoStatus != "ok"
    Call FfoError
    SetErrorLevel 1
    Quit
  ${EndIf}
  ReadINIStr $FfoRoot "$FfoResult" "Result" "Root"
  CreateDirectory "$INSTDIR"
  SetOutPath "$INSTDIR"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  WriteRegStr HKCU "${FFO_REGKEY}" "DataDir" "$FfoRoot"
  StrCpy $0 "en"
  ${If} $LANGUAGE == 1042
    StrCpy $0 "ko"
  ${ElseIf} $LANGUAGE == 1041
    StrCpy $0 "ja"
  ${EndIf}
  WriteRegStr HKCU "${FFO_REGKEY}" "Language" $0
  WriteINIStr "$INSTDIR\data-location.ini" "Data" "Directory" "$FfoRoot"
!macroend

Function un.FfoDataPage
  ${If} $UpdateMode == 1
  ${OrIf} $PassiveMode == 1
    Abort
  ${EndIf}
  ReadINIStr $FfoUnRoot "$INSTDIR\data-location.ini" "Data" "Directory"
  !insertmacro MUI_HEADER_TEXT "$(uninstallDataTitle)" "$(uninstallDataHint)"
  nsDialogs::Create 1018
  Pop $FfoDialog
  ${NSD_CreateLabel} 0 0 100% 28u "$FfoUnRoot"
  Pop $0
  ${NSD_CreateCheckbox} 0 34u 100% 48u "$(uninstallData)"
  Pop $FfoDataCheck
  ${NSD_CreateCheckbox} 0 92u 100% 48u "$(uninstallDocuments)"
  Pop $FfoDocsCheck
  ${If} $FfoRemoveData == 1
    ${NSD_Check} $FfoDataCheck
  ${EndIf}
  ${If} $FfoRemoveDocs == 1
    ${NSD_Check} $FfoDocsCheck
  ${EndIf}
  nsDialogs::Show
FunctionEnd

Function un.FfoDataLeave
  ${NSD_GetState} $FfoDataCheck $FfoRemoveData
  ${NSD_GetState} $FfoDocsCheck $FfoRemoveDocs
FunctionEnd

!macro NSIS_HOOK_PREUNINSTALL
  ReadINIStr $FfoUnRoot "$INSTDIR\data-location.ini" "Data" "Directory"
  ${If} $UpdateMode == 1
  ${OrIf} $PassiveMode == 1
    StrCpy $FfoRemoveData 0
    StrCpy $FfoRemoveDocs 0
  ${Else}
    ; Explicit flags allow an intentional silent cleanup; updates always retain data.
    ${GetOptions} $CMDLINE "/REMOVEUSERDATA=" $0
    ${If} $0 == "1"
      StrCpy $FfoRemoveData 1
    ${EndIf}
    ${GetOptions} $CMDLINE "/REMOVEDOCUMENTS=" $0
    ${If} $0 == "1"
      StrCpy $FfoRemoveDocs 1
    ${EndIf}
  ${EndIf}
  ${If} $FfoRemoveData == 1
  ${OrIf} $FfoRemoveDocs == 1
    InitPluginsDir
    Delete "$PLUGINSDIR\remove-result.ini"
    nsExec::ExecToStack '"$INSTDIR\resources\installer\fastfileocr-setup-helper.exe" remove --root "$FfoUnRoot" --data "$FfoRemoveData" --documents "$FfoRemoveDocs" --result "$PLUGINSDIR\remove-result.ini"'
    Pop $0
    Pop $1
    ReadINIStr $FfoStatus "$PLUGINSDIR\remove-result.ini" "Result" "Status"
    ${If} $0 != 0
    ${OrIf} $FfoStatus != "ok"
      IfSilent +2
      MessageBox MB_OK|MB_ICONSTOP "$(uninstallUnsafe)"
      SetErrorLevel 1
      Quit
    ${EndIf}
  ${EndIf}
  Delete "$INSTDIR\data-location.ini"
!macroend
