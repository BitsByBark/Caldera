$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if (!(Test-Path "backend/icons/icon.ico")) { throw "Missing backend/icons/icon.ico" }
if (!(Test-Path "backend/icons/512x512.png")) { throw "Missing backend/icons/512x512.png" }
if (!(Test-Path "backend/icons/icon.png")) { throw "Missing backend/icons/icon.png" }

$bytes = [System.IO.File]::ReadAllBytes("backend/icons/512x512.png")
$pngSig = [byte[]](0x89,0x50,0x4E,0x47,0x0D,0x0A,0x1A,0x0A)
for ($i=0; $i -lt 8; $i++) {
  if ($bytes[$i] -ne $pngSig[$i]) { throw "backend/icons/512x512.png is not PNG" }
}

$w = [System.BitConverter]::ToUInt32([byte[]]($bytes[19],$bytes[18],$bytes[17],$bytes[16]),0)
$h = [System.BitConverter]::ToUInt32([byte[]]($bytes[23],$bytes[22],$bytes[21],$bytes[20]),0)
if ($w -ne $h -or $w -lt 256) { throw "Invalid icon size ${w}x${h}; must be square and >=256" }

Write-Host "Icon set OK: ${w}x${h}"
