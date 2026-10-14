# Operates only windows of child processes and temporary files created here.
param([Parameter(Mandatory=$true)][string]$Engine, [string]$Report)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class EmptyTestWindow {
    [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr hwnd, uint message, UIntPtr wp, IntPtr lp);
}
'@
$root = Join-Path $env:TEMP ('crexe-empty-ui-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$file = Join-Path $root 'vazio.crexe'
[IO.File]::WriteAllText($file, " `r`n", [Text.UTF8Encoding]::new($true))
function Named($name) { [Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::NameProperty, $name) }
function Wait-Control($parent, $condition, $scope = [Windows.Automation.TreeScope]::Descendants) {
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    do {
        $element = $parent.FindFirst($scope, $condition)
        if ($element) { return $element }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw 'Owned accessible control not found'
}
function Wait-Pattern($control, $pattern) {
    $deadline = [DateTime]::UtcNow.AddSeconds(5)
    do {
        try { return $control.GetCurrentPattern($pattern) } catch {
            if ([DateTime]::UtcNow -ge $deadline) { throw }
            Start-Sleep -Milliseconds 100
        }
    } while ($true)
}
foreach ($action in @('Cancelar', 'Close', 'Sim')) {
    $start = [Diagnostics.ProcessStartInfo]::new((Resolve-Path -LiteralPath $Engine).Path)
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.Environment['CREXE_HOME'] = Join-Path $root 'home'
    foreach ($argument in @($file, '--ui')) { $start.ArgumentList.Add($argument) }
    $process = [Diagnostics.Process]::Start($start)
    try {
        $owned = [Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::ProcessIdProperty, $process.Id)
        $dialog = Wait-Control ([Windows.Automation.AutomationElement]::RootElement) ([Windows.Automation.AndCondition]::new($owned,
            [Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::ClassNameProperty, 'CREXE.EmptyDocument.v1'))) ([Windows.Automation.TreeScope]::Children)
        $null = Wait-Control $dialog (Named 'O arquivo está vazio. Abrir para edição?')
        if ($action -eq 'Close') {
            $null = [EmptyTestWindow]::PostMessageW([IntPtr]$dialog.Current.NativeWindowHandle, 0x0010, [UIntPtr]::Zero, [IntPtr]::Zero)
        } else {
            (Wait-Pattern (Wait-Control $dialog (Named $action)) ([Windows.Automation.InvokePattern]::Pattern)).Invoke()
        }
        if ($action -eq 'Sim') {
            $editor = Wait-Control ([Windows.Automation.AutomationElement]::RootElement) ([Windows.Automation.AndCondition]::new($owned, (Named 'vazio.crexe — CREXE Editor'))) ([Windows.Automation.TreeScope]::Children)
            $edit = Wait-Control $editor ([Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::ControlTypeProperty, [Windows.Automation.ControlType]::Edit))
            (Wait-Pattern $edit ([Windows.Automation.ValuePattern]::Pattern)).SetValue('Uma calculadora')
            Start-Sleep -Milliseconds 200
            (Wait-Pattern (Wait-Control $editor (Named 'Salvar')) ([Windows.Automation.InvokePattern]::Pattern)).Invoke()
            Start-Sleep -Milliseconds 200
            (Wait-Pattern (Wait-Control $editor (Named 'Fechar')) ([Windows.Automation.InvokePattern]::Pattern)).Invoke()
        }
        if (-not $process.WaitForExit(10000) -or $process.ExitCode -ne 0) { throw "Empty dialog failed: $action" }
    } finally {
        if (-not $process.HasExited) { $process.Kill(); $process.WaitForExit() }
    }
}
if ([IO.File]::ReadAllText($file) -ne 'Uma calculadora') { throw 'Editor did not save' }
if (Test-Path (Join-Path $root 'home/config.toml')) { throw 'Empty document loaded configuration' }
if (Test-Path (Join-Path $root 'home/cache')) { throw 'Empty document started generation' }
$result = @{ result='PASS'; empty_bom_whitespace=$true; yes_opens_editor=$true; cancel_and_close=$true; save_does_not_generate=$true; real_api_calls=0 } | ConvertTo-Json
if ($Report) { [IO.File]::WriteAllText($Report, $result + "`n") }
$result
