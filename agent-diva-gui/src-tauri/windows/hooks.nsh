!include "LogicLib.nsh"
!include "nsDialogs.nsh"

Var Dialog
Var ServiceInstallCheckbox
Var InstallServiceSelected

Function ServiceInstallPageCreate
  nsDialogs::Create 1018
  Pop $Dialog

  ${If} $Dialog == error
    Abort
  ${EndIf}

  ${NSD_CreateLabel} 0 0u 100% 24u "Optional Windows service setup for Agent Diva. Use this only when the bundled service bridge is available and you want the gateway to auto-start outside the GUI."
  Pop $0

  ${NSD_CreateCheckbox} 0 34u 100% 12u "Install and start Agent Diva Gateway as a Windows Service"
  Pop $ServiceInstallCheckbox
  ${NSD_SetState} $ServiceInstallCheckbox ${BST_UNCHECKED}

  nsDialogs::Show
FunctionEnd

Function ServiceInstallPageLeave
  ${NSD_GetState} $ServiceInstallCheckbox $InstallServiceSelected
FunctionEnd

Page custom ServiceInstallPageCreate ServiceInstallPageLeave

!macro NSIS_HOOK_POSTINSTALL
  ${If} $InstallServiceSelected == ${BST_CHECKED}
    IfFileExists "$INSTDIR\resources\bin\windows\agent-diva.exe" 0 missing_cli
    IfFileExists "$INSTDIR\resources\bin\windows\agent-diva-service.exe" 0 missing_service

    DetailPrint "Installing Agent Diva Windows Service..."
    ExecWait '"$INSTDIR\resources\bin\windows\agent-diva.exe" service install --auto-start' $0
    ${If} $0 == 0
      ExecWait '"$INSTDIR\resources\bin\windows\agent-diva.exe" service start' $1
      ${If} $1 == 0
        DetailPrint "Agent Diva Windows Service installed and started."
      ${Else}
        MessageBox MB_ICONEXCLAMATION "Agent Diva service was installed but could not be started automatically. Start it later from the GUI or by running agent-diva.exe service start as Administrator."
      ${EndIf}
    ${Else}
      MessageBox MB_ICONEXCLAMATION "Agent Diva service installation failed. Re-run the installer as Administrator or use agent-diva.exe service install --auto-start after installation."
    ${EndIf}
    Goto service_done

missing_cli:
    MessageBox MB_ICONEXCLAMATION "The bundled agent-diva.exe binary was not found under $INSTDIR\resources\bin\windows. The GUI was installed, but Windows service installation was skipped."
    Goto service_done

missing_service:
    MessageBox MB_ICONEXCLAMATION "The optional agent-diva-service.exe binary is not bundled in this build yet. The GUI was installed, but Windows service installation was skipped."

service_done:
  ${EndIf}
!macroend
