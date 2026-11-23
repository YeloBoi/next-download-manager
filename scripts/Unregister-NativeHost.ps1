param([string]$HostPath = (Join-Path $PSScriptRoot 'ndm-native-host.exe'))
$ErrorActionPreference = 'Stop'
$resolvedHost = (Resolve-Path -LiteralPath $HostPath).Path
if ([IO.Path]::GetFileName($resolvedHost) -ne 'ndm-native-host.exe') { throw 'Select the NDM native-host executable.' }
& $resolvedHost --unregister
if ($LASTEXITCODE -ne 0) { throw 'NDM native-host unregistration failed.' }
Write-Output 'Browser registration for this NDM copy was removed. Downloads and queue data were retained.'
