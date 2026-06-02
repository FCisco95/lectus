# Generates tray-state PNGs from the Eclectus master logo.
# tray-idle = 64x64 logo; recording = + red dot; transcribing = + amber dot.
Add-Type -AssemblyName System.Drawing
$icons = Join-Path $PSScriptRoot '..\src-tauri\icons'
$master = Join-Path $icons 'lectus-master.png'
$src = [System.Drawing.Image]::FromFile($master)

function Save-Tray($name, $dotColor) {
    $bmp = New-Object System.Drawing.Bitmap 64, 64
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = 'AntiAlias'
    $g.InterpolationMode = 'HighQualityBicubic'
    $g.Clear([System.Drawing.Color]::Transparent)
    $g.DrawImage($src, 0, 0, 64, 64)
    if ($dotColor) {
        $brush = New-Object System.Drawing.SolidBrush $dotColor
        $g.FillEllipse($brush, 40, 40, 22, 22)
        $pen = New-Object System.Drawing.Pen ([System.Drawing.Color]::White), 3
        $g.DrawEllipse($pen, 40, 40, 22, 22)
        $brush.Dispose(); $pen.Dispose()
    }
    $bmp.Save((Join-Path $icons $name), [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose()
}

Save-Tray 'tray-idle.png' $null
Save-Tray 'tray-recording.png' ([System.Drawing.Color]::FromArgb(255, 230, 50, 50))
Save-Tray 'tray-transcribing.png' ([System.Drawing.Color]::FromArgb(255, 245, 170, 30))
$src.Dispose()
Write-Host 'Tray icons generated.'
