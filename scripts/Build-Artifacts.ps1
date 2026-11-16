$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
function Assert-NativeExit { if ($LASTEXITCODE -ne 0) { throw "Command failed with exit code $LASTEXITCODE" } }
$version = (Get-Content apps/desktop/src-tauri/tauri.conf.json | ConvertFrom-Json).version
$out = Join-Path (Get-Location) '.artifacts'
New-Item -ItemType Directory -Path $out -Force | Out-Null
cargo build --release --locked -p ndm-browser-bridge
Assert-NativeExit
$bundleConfig = @{ bundle = @{ resources = @{
  '../../../target/release/ndm-native-host.exe' = 'ndm-native-host.exe'
  '../../../scripts/Register-NativeHost.ps1' = 'Register-NativeHost.ps1'
  '../../../scripts/Unregister-NativeHost.ps1' = 'Unregister-NativeHost.ps1'
} } } | ConvertTo-Json -Depth 5 -Compress
$bundleConfigPath = Join-Path (Get-Location) 'apps/desktop/src-tauri/tauri.ci.conf.json'
[IO.File]::WriteAllText($bundleConfigPath,$bundleConfig,[Text.UTF8Encoding]::new($false))
Push-Location apps/desktop
try {
  & ../../node_modules/.bin/tauri.cmd build --config src-tauri/tauri.ci.conf.json
  Assert-NativeExit
} finally {
  Pop-Location
  Remove-Item -LiteralPath $bundleConfigPath -ErrorAction SilentlyContinue
}
$portable = Join-Path $out 'portable'
New-Item -ItemType Directory -Path $portable -Force | Out-Null
Copy-Item target/release/ndm.exe,target/release/ndm-native-host.exe,scripts/Register-NativeHost.ps1,scripts/Unregister-NativeHost.ps1,README.md,LICENSE,THIRD_PARTY_NOTICES.md -Destination $portable
Copy-Item docs -Destination (Join-Path $portable 'docs') -Recurse -Force
Compress-Archive -Path "$portable/*" -DestinationPath "$out/NDM-$version-windows-x64-portable.zip" -Force
Copy-Item target/release/bundle/nsis/*.exe -Destination $out
Compress-Archive -Path extensions/chromium/* -DestinationPath "$out/NDM-$version-chromium.zip" -Force
Copy-Item docs/RELEASE_NOTES.md -Destination $out
$hashes = Get-ChildItem $out -File | Where-Object Name -ne 'SHA256SUMS' | ForEach-Object {
  $hash = Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256
  "$($hash.Hash.ToLowerInvariant())  $($_.Name)"
}
[IO.File]::WriteAllLines((Join-Path $out 'SHA256SUMS'), $hashes, [Text.UTF8Encoding]::new($false))
