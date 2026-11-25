# Exercises only the editor process and temporary documents created by this test.
param([Parameter(Mandatory=$true)][string]$Engine, [string]$Report)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$root = Join-Path $env:TEMP ('crexe-editor-ui-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$file = Join-Path $root 'edição com espaços.crexe'
[IO.File]::WriteAllText($file, "Primeiro pedido`r`n", [Text.UTF8Encoding]::new($true))
$start = [Diagnostics.ProcessStartInfo]::new((Resolve-Path -LiteralPath $Engine).Path)
$start.UseShellExecute = $false
$start.CreateNoWindow = $true
$start.Environment['CREXE_HOME'] = Join-Path $root 'home'
foreach ($argument in @('edit', $file)) { $start.ArgumentList.Add($argument) }
$process = [Diagnostics.Process]::Start($start)
function Wait-Element($parent, $condition, $scope = [Windows.Automation.TreeScope]::Descendants) {
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    do {
        $element = $parent.FindFirst($scope, $condition)
        if ($element) { return $element }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Owned accessible control not found in $($parent.Current.Name)"
}
function Named($name) { [Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::NameProperty, $name) }
function Click($name) {
    $control = Wait-Element $window (Named $name)
    $control.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke()
    Start-Sleep -Milliseconds 150
}
try {
    $condition = [Windows.Automation.AndCondition]::new(
        [Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::ProcessIdProperty, $process.Id),
        (Named 'edição com espaços.crexe — CREXE Editor'))
    $window = Wait-Element ([Windows.Automation.AutomationElement]::RootElement) $condition ([Windows.Automation.TreeScope]::Children)
    $editCondition = [Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::ControlTypeProperty, [Windows.Automation.ControlType]::Edit)
    $editor = Wait-Element $window $editCondition
    $value = $editor.GetCurrentPattern([Windows.Automation.ValuePattern]::Pattern)
    $value.SetValue("Pedido alterado: ação`n")
    Click 'Salvar'
    $deadline = [DateTime]::UtcNow.AddSeconds(10)
    while ([IO.File]::ReadAllText($file) -ne "Pedido alterado: ação`r`n" -and [DateTime]::UtcNow -lt $deadline) { Start-Sleep -Milliseconds 100 }
    if ([IO.File]::ReadAllText($file) -ne "Pedido alterado: ação`r`n") { throw 'Editor did not save exact content' }
    $bytes = [IO.File]::ReadAllBytes($file)
    if ($bytes[0] -ne 239 -or $bytes[1] -ne 187 -or $bytes[2] -ne 191) { throw 'BOM lost' }
    $value.SetValue('Texto não salvo')
    Click 'Fechar'
    Click 'Cancelar'
    if ($process.HasExited) { throw 'Cancel closed the editor' }
    Click 'Recarregar'
    Click 'Descartar'
    if ($value.Current.Value -notmatch 'Pedido alterado') { throw 'Reload failed' }
    $value.SetValue('Minha alteração')
    [IO.File]::WriteAllText($file, 'Alteração externa', [Text.UTF8Encoding]::new($true))
    Click 'Salvar'
    if ([IO.File]::ReadAllText($file) -ne 'Alteração externa') { throw 'External change overwritten' }
    if ($value.Current.Value -ne 'Minha alteração') { throw 'Editor buffer lost on conflict' }
    Click 'Fechar'
    Click 'Descartar'
    if (-not $process.WaitForExit(10000) -or $process.ExitCode -ne 0) { throw 'Editor failed to close' }
    if (Test-Path (Join-Path $root 'home/config.toml')) { throw 'Editor loaded configuration' }
    if (Test-Path (Join-Path $root 'home/cache')) { throw 'Editor generated a program' }
    $result = @{ result='PASS'; native_editor=$true; save_and_reload=$true; unsaved_cancel_and_discard=$true; external_conflict=$true; utf8_bom_crlf_preserved=$true; generation_started=$false; real_api_calls=0 } | ConvertTo-Json
    if ($Report) { [IO.File]::WriteAllText($Report, $result + "`n") }
    $result
} finally {
    if (-not $process.HasExited) { $process.Kill(); $process.WaitForExit() }
}
