$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$iconDirectory = Join-Path (Split-Path $PSScriptRoot -Parent) 'apps/desktop/src-tauri/icons'
New-Item -ItemType Directory -Path $iconDirectory -Force | Out-Null
$bitmap = [Drawing.Bitmap]::new(256,256)
$graphics = [Drawing.Graphics]::FromImage($bitmap)
$graphics.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
$graphics.Clear([Drawing.ColorTranslator]::FromHtml('#181818'))
$pen = [Drawing.Pen]::new([Drawing.ColorTranslator]::FromHtml('#67acfa'),16)
$pen.StartCap = [Drawing.Drawing2D.LineCap]::Round
$pen.EndCap = [Drawing.Drawing2D.LineCap]::Round
$graphics.DrawLine($pen,56,68,56,196)
$graphics.DrawLine($pen,56,68,152,196)
$graphics.DrawLine($pen,152,68,152,196)
$graphics.DrawLine($pen,208,68,208,196)
$graphics.DrawLine($pen,180,168,208,196)
$graphics.DrawLine($pen,208,196,236,168)
$bitmap.Save((Join-Path $iconDirectory 'icon.png'),[Drawing.Imaging.ImageFormat]::Png)
$pen.Dispose(); $graphics.Dispose(); $bitmap.Dispose()
$png = [IO.File]::ReadAllBytes((Join-Path $iconDirectory 'icon.png'))
$stream = [IO.File]::Create((Join-Path $iconDirectory 'icon.ico'))
$writer = [IO.BinaryWriter]::new($stream)
try {
  $writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]1)
  $writer.Write([byte]0); $writer.Write([byte]0); $writer.Write([byte]0); $writer.Write([byte]0)
  $writer.Write([uint16]1); $writer.Write([uint16]32)
  $writer.Write([uint32]$png.Length); $writer.Write([uint32]22); $writer.Write($png)
} finally { $writer.Dispose(); $stream.Dispose() }
