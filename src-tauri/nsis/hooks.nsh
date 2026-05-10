; ModuTone NSIS installer hooks
; Phase: 10
;
; NSIS_HOOK_POSTINSTALL: Copy bundled model files from the installer's
; source directory into the install directory. GGUF model files cannot
; be embedded in the NSIS archive (2 GB limit), so they are packaged
; alongside this installer inside a self-extracting 7z wrapper (SFX).
; When the SFX runs, it extracts models/ next to setup.exe in a temp
; dir, and this hook copies them into the install directory.
;
; Expected layout next to setup.exe (in SFX temp dir):
;   models/qwen2.5-3b-instruct-q5_k_m.gguf
;   models/qwen2.5-14b-instruct-q5_k_m-00001-of-00003.gguf
;   models/qwen2.5-14b-instruct-q5_k_m-00002-of-00003.gguf
;   models/qwen2.5-14b-instruct-q5_k_m-00003-of-00003.gguf
;
; NSIS_HOOK_PREUNINSTALL: Ask user whether to remove application data.
; Default: No (preserves user data).
; App data path: $APPDATA\com.modutone.desktop
; (Derived from the app identifier, NOT the product name)

!macro NSIS_HOOK_POSTINSTALL
  ; Copy any GGUF model files from the installer's source directory
  ; ($EXEDIR is the directory containing the setup.exe)
  IfFileExists "$EXEDIR\models\*.gguf" 0 postinstall_no_models
    CreateDirectory "$INSTDIR\models"
    CopyFiles /SILENT "$EXEDIR\models\*.gguf" "$INSTDIR\models"
  postinstall_no_models:
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Remove bundled model files (copied by POSTINSTALL hook, not tracked by NSIS)
  RMDir /r "$INSTDIR\models"

  MessageBox MB_YESNO|MB_ICONQUESTION \
    "Do you want to remove your ModuTone user data (settings, profiles, tags)?$\n$\nThis cannot be undone." \
    /SD IDNO \
    IDYES removedata IDNO keepdata

  removedata:
    RMDir /r "$APPDATA\com.modutone.desktop"
    Goto done

  keepdata:
    ; User data preserved (default)

  done:
!macroend
