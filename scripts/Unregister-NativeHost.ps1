$ErrorActionPreference = 'Stop'
foreach ($browser in @('Google\Chrome','Microsoft\Edge')) {
  $key = "HKCU:\Software\$browser\NativeMessagingHosts\local.ndm.bridge"
  if (Test-Path -LiteralPath $key) { Remove-Item -LiteralPath $key }
}
$bridgeDirectory = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'NDM\bridge'
foreach ($name in @('allowed-origins.json','local.ndm.bridge.json')) {
  $path = Join-Path $bridgeDirectory $name
  if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path }
}
Write-Output 'NDM native-host registration removed. Downloads and queue data were retained.'
