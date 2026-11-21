; Desinstallation propre de MeikiCtrl : on retire le lancement avec Windows.
; (Les reglages dans %APPDATA%\MeikiCtrl sont conserves.)
!macro NSIS_HOOK_PREUNINSTALL
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "MeikiCtrl"
  nsExec::Exec 'schtasks /delete /tn "MeikiCtrl" /f'
  Pop $0
!macroend
