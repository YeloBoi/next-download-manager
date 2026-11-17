!macro NSIS_HOOK_PREUNINSTALL
  DeleteRegKey HKCU "Software\Google\Chrome\NativeMessagingHosts\local.ndm.bridge"
  DeleteRegKey HKCU "Software\Microsoft\Edge\NativeMessagingHosts\local.ndm.bridge"
  DeleteRegKey HKCU "Software\Mozilla\NativeMessagingHosts\local.ndm.bridge"
!macroend
