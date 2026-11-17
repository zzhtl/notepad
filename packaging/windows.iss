[Setup]
AppId=io.github.zzhtl.notepad
AppName=Notepad
AppVersion={#AppVersion}
AppPublisher=zzhtl
AppPublisherURL=https://github.com/zzhtl/notepad
AppSupportURL=https://github.com/zzhtl/notepad/issues
DefaultDirName={localappdata}\Programs\Notepad
DefaultGroupName=Notepad
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
DisableProgramGroupPage=yes
DisableDirPage=no
WizardStyle=modern
UninstallDisplayIcon={app}\notepad.ico
SetupIconFile={#SourceDir}\notepad.ico
LicenseFile={#SourceDir}\LICENSE
OutputDir={#DestinationDir}
OutputBaseFilename=notepad-v{#AppVersion}-windows-x86_64-setup
Compression=lzma2
SolidCompression=yes
CloseApplications=yes
RestartApplications=no

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Files]
Source: "{#SourceDir}\notepad.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\notepad.install"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\notepad.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Notepad"; Filename: "{app}\notepad.exe"; WorkingDir: "{app}"; IconFilename: "{app}\notepad.ico"
Name: "{autodesktop}\Notepad"; Filename: "{app}\notepad.exe"; WorkingDir: "{app}"; IconFilename: "{app}\notepad.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\notepad.exe"; Description: "Launch Notepad"; Flags: nowait postinstall skipifsilent
