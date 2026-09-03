function WinGetFileAttributes(FileName: String): LongWord; external 'GetFileAttributesW@kernel32.dll stdcall';
// Data operations only touch known children of an application-owned directory.
var
  DataPage: TInputDirWizardPage;
  ModePage: TInputOptionWizardPage;
  UninstallDataRoot: String;
  RemoveUserData, RemoveDocuments: Boolean;

function HasEntries(const Directory: String): Boolean;
var Entry: TFindRec;
begin
  Result := False;
  if FindFirst(AddBackslash(Directory) + '*', Entry) then begin
    try
      repeat
        if (Entry.Name <> '.') and (Entry.Name <> '..') then begin Result := True; Break; end;
      until not FindNext(Entry);
    finally FindClose(Entry); end;
  end;
end;

function NoLinks(const Directory: String): Boolean;
var Entry: TFindRec;
begin
  Result := False;
  if (WinGetFileAttributes(Directory) and $400) <> 0 then Exit;
  Result := True;
  if FindFirst(AddBackslash(Directory) + '*', Entry) then begin
    try
      repeat
        if (Entry.Name <> '.') and (Entry.Name <> '..') then begin
          if (Entry.Attributes and $400) <> 0 then begin Result := False; Break; end;
          if (Entry.Attributes and $10) <> 0 then
            if not NoLinks(AddBackslash(Directory) + Entry.Name) then begin Result := False; Break; end;
        end;
      until not FindNext(Entry);
    finally FindClose(Entry); end;
  end;
end;

function OwnedData(const Directory: String): Boolean;
var Marker: AnsiString;
begin
  Result := False;
  if Length(Directory) <= 3 then Exit;
  if not LoadStringFromFile(AddBackslash(Directory) + '.fastfileocr-data', Marker) then Exit;
  Result := (Trim(String(Marker)) = 'FastFileOCR data v1') and NoLinks(Directory);
end;

function SelectedDirectory: String;
begin
  Result := RemoveBackslashUnlessRoot(ExpandFileName(Trim(DataPage.Values[0])));
end;

function DataDirectory(Param: String): String;
var Selected: String;
begin
  Selected := SelectedDirectory;
  Result := Selected;
  // Never adopt an unrelated folder. Use a dedicated child and display it before install.
  if CompareText(Selected, RemoveBackslashUnlessRoot(ExpandConstant('{app}'))) = 0 then
    Result := AddBackslash(Selected) + 'Data'
  else if not FileExists(AddBackslash(Selected) + '.fastfileocr-data') then
    if (Length(Selected) <= 3) or (DirExists(Selected) and HasEntries(Selected)) then begin
      if CompareText(ExtractFileName(Selected), 'FastFileOCR') = 0 then
        Result := AddBackslash(Selected) + 'Data'
      else
        Result := AddBackslash(Selected) + 'FastFileOCR';
    end;
end;

function InitialLanguage(Param: String): String;
begin
  Result := 'en';
  if ActiveLanguage = 'korean' then Result := 'ko';
  if ActiveLanguage = 'japanese' then Result := 'ja';
end;

procedure InitializeWizard;
var Previous: String;
begin
  Previous := ExpandConstant('{localappdata}\FastFileOCR');
  RegQueryStringValue(HKCU, 'Software\FastFileOCR', 'DataDir', Previous);
  DataPage := CreateInputDirPage(wpSelectDir, CustomMessage('installDataTitle'),
    CustomMessage('installDataDescription'), CustomMessage('installDataHint'), False, 'FastFileOCR');
  DataPage.Add('');
  DataPage.Values[0] := ExpandConstant('{param:DATADIR|' + Previous + '}');
  ModePage := CreateInputOptionPage(DataPage.ID, CustomMessage('installModeTitle'),
    CustomMessage('installModeDescription'), CustomMessage('installModeHint'), True, False);
  ModePage.Add(CustomMessage('installKeep'));
  ModePage.Add(CustomMessage('installFresh'));
  ModePage.SelectedValueIndex := 0;
end;

function ValidateDataDirectory: String;
var Directory, Selected, Probe: String;
begin
  Result := '';
  Selected := SelectedDirectory;
  Directory := DataDirectory('');
  if (Trim(DataPage.Values[0]) = '') or
     (CompareText(Selected, ExpandConstant('{win}')) = 0) or
     (DirExists(Selected) and ((WinGetFileAttributes(Selected) and $400) <> 0)) then begin
    Result := CustomMessage('installUnsafe') + #13#10 + Selected; Exit;
  end;
  if (Length(Directory) <= 3) or
     (CompareText(Directory, ExpandConstant('{win}')) = 0) or
     (CompareText(Directory, ExpandConstant('{%USERPROFILE}')) = 0) or
     (DirExists(Directory) and (not NoLinks(Directory))) or
     (DirExists(Directory) and HasEntries(Directory) and (not OwnedData(Directory))) then begin
    Result := CustomMessage('installUnsafe') + #13#10 + Directory; Exit;
  end;
  if not ForceDirectories(Directory) then begin
    Result := CustomMessage('installWriteError'); Exit;
  end;
  Probe := AddBackslash(Directory) + '.fastfileocr-write-check';
  if not SaveStringToFile(Probe, 'ok', False) then begin
    Result := CustomMessage('installWriteError'); Exit;
  end;
  DeleteFile(Probe);
  DataPage.Values[0] := Directory;
end;

function NextButtonClick(CurPageID: Integer): Boolean;
var Error: String;
begin
  Result := True;
  if CurPageID = DataPage.ID then begin
    Error := ValidateDataDirectory;
    if Error <> '' then begin
      if not WizardSilent then MsgBox(Error, mbError, MB_OK);
      Result := False; Exit;
    end;
  end;
  if (CurPageID = ModePage.ID) and (ModePage.SelectedValueIndex = 1) then
    Result := SuppressibleMsgBox(CustomMessage('installResetConfirm'), mbConfirmation, MB_YESNO or MB_DEFBUTTON2, IDNO) = IDYES;
end;

procedure CurStepChanged(CurStep: TSetupStep);
var Directory, Backup, Error: String;
begin
  if CurStep = ssInstall then begin
    Error := ValidateDataDirectory;
    if Error <> '' then RaiseException(Error);
    Directory := DataDirectory('');
    if not FileExists(AddBackslash(Directory) + '.fastfileocr-data') then
      if not SaveStringToFile(AddBackslash(Directory) + '.fastfileocr-data', 'FastFileOCR data v1', False) then
        RaiseException(CustomMessage('installWriteError'));
    if not OwnedData(Directory) then RaiseException(CustomMessage('installUnsafe'));
    if ModePage.SelectedValueIndex = 1 then begin
      Backup := AddBackslash(Directory) + 'settings-backups';
      ForceDirectories(Backup);
      if FileExists(AddBackslash(Directory) + 'settings.json') then
        if not RenameFile(AddBackslash(Directory) + 'settings.json',
          AddBackslash(Backup) + GetDateTimeString('yyyymmddhhnnsszzz', '-', ':') + '.json') then
            RaiseException(CustomMessage('installWriteError'));
      // This suppresses importing legacy settings after an explicit fresh start.
      SaveStringToFile(AddBackslash(Directory) + '.fresh-settings', '1', False);
    end;
  end;
end;

function UpdateReadyMemo(Space, NewLine, MemoUserInfoInfo, MemoDirInfo, MemoTypeInfo,
  MemoComponentsInfo, MemoGroupInfo, MemoTasksInfo: String): String;
begin
  Result := MemoDirInfo + NewLine + NewLine +
    FmtMessage(CustomMessage('installDataSummary'), [DataDirectory('')]) + NewLine +
    CustomMessage('installKeep');
  if ModePage.SelectedValueIndex = 1 then
    Result := MemoDirInfo + NewLine + NewLine +
      FmtMessage(CustomMessage('installDataSummary'), [DataDirectory('')]) + NewLine +
      CustomMessage('installFresh');
end;

function InitializeUninstall: Boolean;
begin
  UninstallDataRoot := GetIniString('Data', 'Directory', '', ExpandConstant('{app}\data-location.ini'));
  RemoveUserData := False; RemoveDocuments := False;
  Result := True;
  if (UninstallDataRoot <> '') and (not UninstallSilent) then begin
    RemoveUserData := MsgBox(CustomMessage('uninstallData'), mbConfirmation, MB_YESNO or MB_DEFBUTTON2) = IDYES;
    RemoveDocuments := MsgBox(CustomMessage('uninstallDocuments'), mbConfirmation, MB_YESNO or MB_DEFBUTTON2) = IDYES;
  end;
end;

procedure RemoveManaged(const Root, Child: String);
begin
  if not OwnedData(Root) then Exit;
  if DirExists(AddBackslash(Root) + Child) then
    DelTree(AddBackslash(Root) + Child, True, True, True)
  else if FileExists(AddBackslash(Root) + Child) then
    DeleteFile(AddBackslash(Root) + Child);
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then begin
    if not (RemoveUserData or RemoveDocuments) then Exit;
    if not OwnedData(UninstallDataRoot) then begin
      if not UninstallSilent then MsgBox(CustomMessage('uninstallUnsafe'), mbInformation, MB_OK);
      Exit;
    end;
    if RemoveUserData then begin
      RemoveManaged(UninstallDataRoot, 'settings.json');
      RemoveManaged(UninstallDataRoot, 'settings-backups');
      RemoveManaged(UninstallDataRoot, 'models');
      RemoveManaged(UninstallDataRoot, 'logs');
      RemoveManaged(UninstallDataRoot, 'updates');
      RemoveManaged(UninstallDataRoot, '.fresh-settings');
    end;
    if RemoveDocuments then RemoveManaged(UninstallDataRoot, 'workspaces');
    // Keep the location marker and registry hint, allowing a reinstall to find retained data.
  end;
end;
