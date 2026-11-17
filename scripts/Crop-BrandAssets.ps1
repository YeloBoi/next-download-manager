param([Parameter(Mandatory)][string]$Source)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Split-Path $PSScriptRoot -Parent
$brand = Join-Path $root 'apps/desktop/public/brand'
$extension = Join-Path $root 'extensions/chromium/icons'
New-Item -ItemType Directory -Path $brand,$extension -Force | Out-Null
$image = [Drawing.Bitmap]::FromFile((Resolve-Path -LiteralPath $Source))
if ($image.Width -ne 1024 -or $image.Height -ne 1024) { throw 'Expected the original 1024 x 1024 logo sheet.' }
function Save-Crop([string]$Name,[int]$X,[int]$Y,[int]$W,[int]$H,[bool]$ClearWhite) {
  $crop = $image.Clone([Drawing.Rectangle]::new($X,$Y,$W,$H),[Drawing.Imaging.PixelFormat]::Format32bppArgb)
  if ($ClearWhite) {
    for ($cy=0; $cy -lt $H; $cy++) { for ($cx=0; $cx -lt $W; $cx++) {
      $c = $crop.GetPixel($cx,$cy)
      $minimum = [Math]::Min($c.R,[Math]::Min($c.G,$c.B))
      if ($minimum -lt 200) { continue }
      $alpha = 255 - $minimum
      if ($alpha -lt 24) { $crop.SetPixel($cx,$cy,[Drawing.Color]::Transparent) }
      else {
        $r = [Math]::Max(0,[Math]::Min(255,[int](255-((255-$c.R)*255/$alpha))))
        $g = [Math]::Max(0,[Math]::Min(255,[int](255-((255-$c.G)*255/$alpha))))
        $b = [Math]::Max(0,[Math]::Min(255,[int](255-((255-$c.B)*255/$alpha))))
        $crop.SetPixel($cx,$cy,[Drawing.Color]::FromArgb($alpha,$r,$g,$b))
      }
    } }
  }
  $crop.Save((Join-Path $brand $Name),[Drawing.Imaging.ImageFormat]::Png)
  return $crop
}
function Resize-Square([Drawing.Bitmap]$InputImage,[int]$Size) {
  $result = [Drawing.Bitmap]::new($Size,$Size,[Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $graphics = [Drawing.Graphics]::FromImage($result)
  $graphics.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
  $graphics.DrawImage($InputImage,0,0,$Size,$Size)
  $graphics.Dispose()
  return $result
}
try {
  $mark = Save-Crop 'mark-blue.png' 102 202 306 312 $true
  $wordmark = Save-Crop 'wordmark-color.png' 102 202 822 312 $false
  $dark = Save-Crop 'app-dark.png' 54 674 230 228 $true
  $light = Save-Crop 'app-light.png' 335 673 236 232 $false
  $mono = Save-Crop 'wordmark-mono.png' 626 718 363 143 $true
  $icon = Resize-Square $dark 256
  $icon.Save((Join-Path $root 'apps/desktop/src-tauri/icons/icon.png'),[Drawing.Imaging.ImageFormat]::Png)
  foreach ($size in @(16,32,48,128)) {
    $small = Resize-Square $dark $size
    $small.Save((Join-Path $extension "$size.png"),[Drawing.Imaging.ImageFormat]::Png)
    $small.Dispose()
  }
  # ICO stores PNG entries for sharp Windows taskbar and installer icons at every DPI.
  $frames = @(16,24,32,48,64,128,256) | ForEach-Object {
    $bitmap = Resize-Square $dark $_
    $stream = [IO.MemoryStream]::new()
    $bitmap.Save($stream,[Drawing.Imaging.ImageFormat]::Png)
    $bytes = $stream.ToArray()
    $bitmap.Dispose(); $stream.Dispose()
    @{ Size=$_; Bytes=$bytes }
  }
  $file = [IO.File]::Create((Join-Path $root 'apps/desktop/src-tauri/icons/icon.ico'))
  $writer = [IO.BinaryWriter]::new($file)
  try {
    $writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]$frames.Count)
    $offset = 6 + 16*$frames.Count
    foreach ($frame in $frames) {
      $dimension = if ($frame.Size -eq 256) { 0 } else { $frame.Size }
      $writer.Write([byte]$dimension); $writer.Write([byte]$dimension)
      $writer.Write([byte]0); $writer.Write([byte]0)
      $writer.Write([uint16]1); $writer.Write([uint16]32)
      $writer.Write([uint32]$frame.Bytes.Length); $writer.Write([uint32]$offset)
      $offset += $frame.Bytes.Length
    }
    foreach ($frame in $frames) { $writer.Write([byte[]]$frame.Bytes) }
  } finally { $writer.Dispose(); $file.Dispose() }
  $icon.Dispose()
  Write-Output 'Cropped five original brand variants; generated Windows and Chromium icons.'
} finally {
  foreach ($bitmap in @($mark,$wordmark,$dark,$light,$mono)) { if ($bitmap) { $bitmap.Dispose() } }
  $image.Dispose()
}
