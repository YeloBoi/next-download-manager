param(
  [ValidatePattern('^[a-p]{32}$')][string]$ExtensionId,
  [string]$HostPath = (Join-Path $PSScriptRoot 'ndm-native-host.exe'),
  # Accepted for callers of older releases. Registration now covers all browsers.
  [ValidateSet('Chromium','Firefox')][string]$Browser = 'Chromium'
)
$ErrorActionPreference = 'Stop'
$resolvedHost = (Resolve-Path -LiteralPath $HostPath).Path
if ([IO.Path]::GetFileName($resolvedHost) -ne 'ndm-native-host.exe') { throw 'Select the NDM native-host executable.' }
if ($ExtensionId) { & $resolvedHost --register $ExtensionId }
else { & $resolvedHost --register }
if ($LASTEXITCODE -ne 0) { throw 'NDM native-host registration failed.' }
Write-Output 'NDM native host registered for Firefox, Chrome and Edge.'
