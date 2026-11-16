param(
  [Parameter(Mandatory=$true)][ValidatePattern('^[a-p]{32}$')][string]$ExtensionId,
  [Parameter(Mandatory=$true)][string]$HostPath
)
$ErrorActionPreference = 'Stop'
$resolvedHost = (Resolve-Path -LiteralPath $HostPath).Path
if ([System.IO.Path]::GetFileName($resolvedHost) -ne 'ndm-native-host.exe') { throw 'Select the NDM native-host executable.' }
$bridgeDirectory = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'NDM\bridge'
New-Item -ItemType Directory -Path $bridgeDirectory -Force | Out-Null
$origin = "chrome-extension://$ExtensionId/"
$manifest = @{ name = 'local.ndm.bridge'; description = 'NDM native messaging host'; path = $resolvedHost; type = 'stdio'; allowed_origins = @($origin) }
$manifestPath = Join-Path $bridgeDirectory 'local.ndm.bridge.json'
$utf8 = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($manifestPath, ($manifest | ConvertTo-Json), $utf8)
[System.IO.File]::WriteAllText((Join-Path $bridgeDirectory 'allowed-origins.json'), (ConvertTo-Json -InputObject @($origin)), $utf8)
foreach ($browser in @('Google\Chrome','Microsoft\Edge')) {
  $key = "HKCU:\Software\$browser\NativeMessagingHosts\local.ndm.bridge"
  New-Item -Path $key -Force | Out-Null
  Set-Item -LiteralPath $key -Value $manifestPath
}
Write-Output 'NDM native host registered for the selected extension in Chrome and Edge.'
