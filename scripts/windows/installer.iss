; Build through scripts/build_windows_installer.py, using the validated CI ZIP.
; Directives are compatible with Inno Setup 6.3 and later.
; https://github.com/jrsoftware/issrc/blob/is-6_3_3/ISHelp/isetup.xml
; Inno's original runtime, About box, copyrights and URLs are retained unchanged.
; Binary redistribution terms: https://github.com/jrsoftware/issrc/blob/is-6_7_1/license.txt
#if VER < EncodeVer(6, 3, 0)
  #error Inno Setup 6.3 or newer is required.
#endif
#ifndef PayloadDir
  #error PayloadDir must point to the validated, extracted Windows package.
#endif
#ifndef AppVersion
  #error AppVersion must come from BUILD-INFO.json.
#endif
#ifndef NumericVersion
  #error NumericVersion must contain the numeric version components.
#endif
#ifndef InstallerOutputDir
  #error InstallerOutputDir is required.
#endif
#ifndef InstallerIcon
  #error InstallerIcon is required.
#endif

#define ApplicationMutex "Local\Spotiamp.Application.4A36A2DD-2701-4EF4-A3B3-5C1EF1A8A1D0"
#define InstallationMutex "Local\Spotiamp.Setup.4A36A2DD-2701-4EF4-A3B3-5C1EF1A8A1D0"

[Setup]
; This identity is permanent across application versions and ZIP updates.
AppId={{4A36A2DD-2701-4EF4-A3B3-5C1EF1A8A1D0}
AppName=Spotiamp
AppVersion={#AppVersion}
AppVerName=Spotiamp {#AppVersion}
AppPublisher=roukmoute
AppPublisherURL=https://github.com/roukmoute/spotiamp
AppSupportURL=https://github.com/roukmoute/spotiamp/issues
AppUpdatesURL=https://github.com/roukmoute/spotiamp/releases
DefaultDirName={localappdata}\Programs\Spotiamp
DefaultGroupName=Spotiamp
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
UsePreviousAppDir=yes
UsePreviousTasks=yes
AppMutex={#ApplicationMutex}
SetupMutex={#InstallationMutex}
CloseApplications=yes
CloseApplicationsFilter=spotiamp.exe
RestartApplications=no
Uninstallable=yes
CreateUninstallRegKey=yes
UninstallDisplayName=Spotiamp
UninstallDisplayIcon={app}\spotiamp.exe
WizardStyle=modern
SetupIconFile={#InstallerIcon}
Compression=lzma2
SolidCompression=yes
OutputDir={#InstallerOutputDir}
OutputBaseFilename=Spotiamp-Setup-Windows-x64
VersionInfoVersion={#NumericVersion}
VersionInfoTextVersion={#AppVersion}
VersionInfoProductTextVersion={#AppVersion}
VersionInfoDescription=Spotiamp Setup

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "french"; MessagesFile: "compiler:Languages\French.isl"
Name: "german"; MessagesFile: "compiler:Languages\German.isl"
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "italian"; MessagesFile: "compiler:Languages\Italian.isl"
Name: "portuguese"; MessagesFile: "compiler:Languages\Portuguese.isl"

[CustomMessages]
english.InstallationBusy=Spotiamp is already being installed or removed. Close the other installer and try again.
french.InstallationBusy=Une installation ou une désinstallation de Spotiamp est déjà en cours. Fermez l'autre assistant puis réessayez.
german.InstallationBusy=Spotiamp wird bereits installiert oder entfernt. Schließen Sie das andere Installationsprogramm und versuchen Sie es erneut.
spanish.InstallationBusy=Ya hay una instalación o desinstalación de Spotiamp en curso. Cierre el otro asistente e inténtelo de nuevo.
italian.InstallationBusy=È già in corso un'installazione o disinstallazione di Spotiamp. Chiudere l'altra procedura e riprovare.
portuguese.InstallationBusy=Já está em curso uma instalação ou desinstalação do Spotiamp. Feche o outro assistente e tente novamente.
english.ApplicationRunning=Close Spotiamp before continuing the installation.
french.ApplicationRunning=Fermez Spotiamp avant de poursuivre l'installation.
german.ApplicationRunning=Schließen Sie Spotiamp, bevor Sie die Installation fortsetzen.
spanish.ApplicationRunning=Cierre Spotiamp antes de continuar la instalación.
italian.ApplicationRunning=Chiudere Spotiamp prima di continuare l'installazione.
portuguese.ApplicationRunning=Feche o Spotiamp antes de continuar a instalação.

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
; No transforms or exclusions: every byte from the portable ZIP is installed.
Source: "{#PayloadDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{userprograms}\Spotiamp\Spotiamp"; Filename: "{app}\spotiamp.exe"; WorkingDir: "{app}"
Name: "{userdesktop}\Spotiamp"; Filename: "{app}\spotiamp.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
; The application briefly waits for SetupMutex to be released after Finish.
Filename: "{app}\spotiamp.exe"; Parameters: "--wait-for-setup"; WorkingDir: "{app}"; Description: "{cm:LaunchProgram,Spotiamp}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
; ZIP updates can add dependency files unknown to the original Inno uninstall log.
; These directories and this lock are owned by the application. Never use {app}\*.
Type: filesandordirs; Name: "{app}\licenses"
Type: filesandordirs; Name: "{app}\mpl-source"
Type: filesandordirs; Name: "{app}\scripts"
Type: files; Name: "{app}\.spotiamp-update.lock"

[Code]
// SetupMutex covers Setup itself. Uninstall must hold the same mutex explicitly.
// CreateMutexW is atomic: two uninstallers cannot pass a check-then-create race.
function NativeCreateMutex(Attributes: LongWord; InitialOwner: LongWord;
  Name: String): LongWord;
  external 'CreateMutexW@kernel32.dll stdcall';
function NativeCloseHandle(Handle: LongWord): LongWord;
  external 'CloseHandle@kernel32.dll stdcall';

var
  UninstallMutex: LongWord;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  // Inno checks AppMutex before creating SetupMutex. Recheck after creation to
  // cover an application that started between these two initialization steps.
  Result := '';
  if CheckForMutexes('{#ApplicationMutex}') then
    Result := CustomMessage('ApplicationRunning');
end;

function InitializeUninstall: Boolean;
var
  ErrorCode: LongWord;
begin
  UninstallMutex := NativeCreateMutex(0, 0, '{#InstallationMutex}');
  ErrorCode := DLLGetLastError;
  Result := (UninstallMutex <> 0) and (ErrorCode <> 183);
  if not Result then begin
    if UninstallMutex <> 0 then begin
      NativeCloseHandle(UninstallMutex);
      UninstallMutex := 0;
    end;
    SuppressibleMsgBox(CustomMessage('InstallationBusy'), mbError, MB_OK, IDOK);
  end;
end;

procedure DeinitializeUninstall;
begin
  if UninstallMutex <> 0 then begin
    NativeCloseHandle(UninstallMutex);
    UninstallMutex := 0;
  end;
end;

