# Generate the Windows NSIS bitmaps from the Lectus master mark.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$iconRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../src-tauri/icons'))
$masterImage = [System.Drawing.Image]::FromFile((Join-Path $iconRoot 'lectus-master.png'))
$surface = [System.Drawing.Color]::FromArgb(20, 21, 26)
$foreground = [System.Drawing.Color]::FromArgb(238, 240, 244)

function Draw-Mark($graphics, $image, [single]$x, [single]$y, [single]$width, [single]$height) {
    $scale = [Math]::Min($width / $image.Width, $height / $image.Height)
    $drawWidth = [single]($image.Width * $scale)
    $drawHeight = [single]($image.Height * $scale)
    $bounds = [System.Drawing.RectangleF]::new(($x + ($width - $drawWidth) / 2), ($y + ($height - $drawHeight) / 2), $drawWidth, $drawHeight)
    $graphics.DrawImage($image, $bounds)
}

try {
    foreach ($art in @(@{ Name='installer-header.bmp'; Width=150; Height=57 }, @{ Name='installer-sidebar.bmp'; Width=164; Height=314 })) {
        $bitmap = [System.Drawing.Bitmap]::new($art.Width, $art.Height, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        try {
            $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
            $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
            $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
            $graphics.Clear($surface)
            if ($art.Height -eq 57) {
                Draw-Mark $graphics $masterImage 100 7.5 42 42
            } else {
                Draw-Mark $graphics $masterImage 32 44 100 100
                $wordFont = [System.Drawing.Font]::new('Segoe UI', 20, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
                $wordBrush = [System.Drawing.SolidBrush]::new($foreground)
                $wordFormat = [System.Drawing.StringFormat]::new()
                try {
                    $graphics.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
                    $wordFormat.Alignment = [System.Drawing.StringAlignment]::Center
                    $wordFormat.LineAlignment = [System.Drawing.StringAlignment]::Center
                    $wordBounds = [System.Drawing.RectangleF]::new(0, 152, 164, 36)
                    $graphics.DrawString('Lectus', $wordFont, $wordBrush, $wordBounds, $wordFormat)
                } finally { $wordFormat.Dispose(); $wordBrush.Dispose(); $wordFont.Dispose() }
            }
            $bitmap.Save((Join-Path $iconRoot $art.Name), [System.Drawing.Imaging.ImageFormat]::Bmp)
            Write-Output ('Generated ' + $art.Name + ' (' + $art.Width + 'x' + $art.Height + ', 24-bit).')
        } finally { $graphics.Dispose(); $bitmap.Dispose() }
    }
} finally { $masterImage.Dispose() }
