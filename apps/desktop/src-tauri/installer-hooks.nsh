!macro NSIS_HOOK_POSTINSTALL
  ExecWait '"$INSTDIR\ndm-native-host.exe" --register' $0
  ${If} $0 != 0
    DetailPrint "NDM could not register the browser connection. Open NDM and choose Check connection to retry."
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ExecWait '"$INSTDIR\ndm-native-host.exe" --unregister'
!macroend
