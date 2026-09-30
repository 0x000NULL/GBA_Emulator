<#
.SYNOPSIS
  Fetch test ROMs and the reference emulator into tools/ (gitignored).

  - jsmolka/gba-tests  -> $GBA_TESTS_DIR/gba-tests   (prebuilt .gba files are committed upstream)
  - mgba-emu/suite     -> $GBA_TESTS_DIR/suite       (source only; needs devkitARM to build suite.gba)
  - mGBA portable      -> tools/mgba/mGBA-<ver>-win64 (reference emulator for trace/PCM diffing)
  - GBATEK             -> tools/gbatek.htm, split into docs/gbatek/*.md by scripts/gbatek-split.py

.PARAMETER TestsDir
  Defaults to $env:GBA_TESTS_DIR, else <repo>/tools/tests.
#>
[CmdletBinding()]
param(
    [string]$TestsDir = $(if ($env:GBA_TESTS_DIR) { $env:GBA_TESTS_DIR } else { Join-Path $PSScriptRoot '..\tools\tests' }),
    [string]$MgbaVersion = '0.10.5',
    [switch]$SkipMgba,
    [switch]$SkipGbatek
)
$ErrorActionPreference = 'Stop'
$repo = Resolve-Path (Join-Path $PSScriptRoot '..')
New-Item -ItemType Directory -Force $TestsDir | Out-Null
$TestsDir = (Resolve-Path $TestsDir).Path

function Clone-OrPull($url, $dest) {
    if (Test-Path (Join-Path $dest '.git')) {
        Write-Host "updating $dest"
        git -C $dest pull -q --ff-only
    } else {
        Write-Host "cloning $url -> $dest"
        git clone -q --depth 1 $url $dest
    }
}

Clone-OrPull 'https://github.com/jsmolka/gba-tests.git' (Join-Path $TestsDir 'gba-tests')
Clone-OrPull 'https://github.com/mgba-emu/suite.git'    (Join-Path $TestsDir 'suite')

$suiteRom = Join-Path $TestsDir 'suite\suite.gba'
if (-not (Test-Path $suiteRom)) {
    Write-Host "note: mgba suite ships source only; build suite.gba with devkitARM (docker-build.sh) and place it at $suiteRom"
}

if (-not $SkipMgba) {
    $mgbaDir = Join-Path $repo 'tools\mgba'
    New-Item -ItemType Directory -Force $mgbaDir | Out-Null
    $archive = Join-Path $mgbaDir "mGBA-$MgbaVersion-win64.7z"
    $extracted = Join-Path $mgbaDir "mGBA-$MgbaVersion-win64"
    if (-not (Test-Path $extracted)) {
        if (-not (Test-Path $archive)) {
            $url = "https://github.com/mgba-emu/mgba/releases/download/$MgbaVersion/mGBA-$MgbaVersion-win64.7z"
            Write-Host "downloading $url"
            Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $archive
        }
        $sevenZip = @("$env:ProgramFiles\7-Zip\7z.exe", '7z') | Where-Object { Get-Command $_ -ErrorAction SilentlyContinue } | Select-Object -First 1
        if (-not $sevenZip) { throw '7-Zip is required to extract the mGBA archive (winget install 7zip.7zip)' }
        & $sevenZip x -y "-o$mgbaDir" $archive | Out-Null
    }
    Write-Host "mGBA: $extracted\mGBA.exe"
}

if (-not $SkipGbatek) {
    $gbatek = Join-Path $repo 'tools\gbatek.htm'
    if (-not (Test-Path $gbatek)) {
        Write-Host 'downloading GBATEK'
        Invoke-WebRequest -UseBasicParsing -Uri 'https://problemkaputt.de/gbatek.htm' -OutFile $gbatek
    }
    Push-Location $repo
    try { python scripts/gbatek-split.py } finally { Pop-Location }
}

Write-Host ''
Write-Host "GBA_TESTS_DIR=$TestsDir"
Write-Host 'Set it for cargo test, e.g.:  $env:GBA_TESTS_DIR = ' + "'$TestsDir'"
