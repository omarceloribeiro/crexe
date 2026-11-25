# Windows native settings test. Uses only the process started below and synthetic configuration.
param(
    [Parameter(Mandatory=$true)][string]$Engine,
    [string]$Report
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$root = Join-Path $env:TEMP ('crexe-configuration-ui-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$active = Join-Path $root 'config.toml'
$import = Join-Path $root 'import.toml'
$text = [IO.File]::ReadAllText((Join-Path $PSScriptRoot '..\config.example.toml'))
[IO.File]::WriteAllText($import, $text.Replace('default_provider = "local"', 'default_provider = "openai"').Replace('gpt-4o-mini', 'synthetic-ui-model'))
$info = [Diagnostics.ProcessStartInfo]::new((Resolve-Path -LiteralPath $Engine).Path)
$info.UseShellExecute = $false
$info.CreateNoWindow = $true
foreach ($arg in @('configure', '--config', $active, '--import', $import)) { $info.ArgumentList.Add($arg) }
$process = [Diagnostics.Process]::Start($info)
$window = $null
try {
    $condition = [Windows.Automation.AndCondition]::new(
        [Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::ProcessIdProperty, $process.Id),
        [Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::NameProperty, 'CREXE — Configurações'))
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    while (-not $window -and [DateTime]::UtcNow -lt $deadline) {
        $window = [Windows.Automation.AutomationElement]::RootElement.FindFirst([Windows.Automation.TreeScope]::Children, $condition)
        Start-Sleep -Milliseconds 100
    }
    if (-not $window) { throw 'Owned settings window not found' }
    $buttonCondition = [Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::NameProperty, 'Salvar')
    $button = $null
    $deadline = [DateTime]::UtcNow.AddSeconds(10)
    while (-not $button -and [DateTime]::UtcNow -lt $deadline) {
        $button = $window.FindFirst([Windows.Automation.TreeScope]::Descendants, $buttonCondition)
        Start-Sleep -Milliseconds 100
    }
    if (-not $button) { throw 'Accessible Save button not found' }
    $pattern = $button.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern)
    $pattern.Invoke()
    $deadline = [DateTime]::UtcNow.AddSeconds(10)
    while (-not [IO.File]::Exists($active) -and [DateTime]::UtcNow -lt $deadline) { Start-Sleep -Milliseconds 100 }
    if (-not [IO.File]::Exists($active)) { throw 'Save did not produce active configuration' }
    $saved = [IO.File]::ReadAllText($active)
    if (-not $saved.Contains('default_provider = "openai"') -or -not $saved.Contains('synthetic-ui-model')) { throw 'Saved configuration differs from imported draft' }
    $window.GetCurrentPattern([Windows.Automation.WindowPattern]::Pattern).Close()
    if (-not $process.WaitForExit(10000) -or $process.ExitCode -ne 0) { throw 'Settings did not close normally' }
    $diagnostic = & $Engine doctor --config $active
    if ($LASTEXITCODE -ne 0 -or -not ($diagnostic -match 'Provider: openai; model: synthetic-ui-model')) { throw 'CLI did not use saved settings' }
    $result = @{ result='PASS'; imported_provider_saved_by_native_button=$true; cli_reads_saved_provider=$true; accessibility_invoke=$true; real_api_calls=0 } | ConvertTo-Json
    if ($Report) { [IO.File]::WriteAllText($Report, $result + "`n") }
    $result
} finally {
    if (-not $process.HasExited) { $process.Kill(); $process.WaitForExit() }
}
