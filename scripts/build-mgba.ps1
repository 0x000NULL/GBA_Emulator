<#
.SYNOPSIS
  Builds tools/mgba-wav.exe, the headless mGBA audio capture tool (scripts/mgba-wav/).
.DESCRIPTION
  Clones the mGBA source at the pinned tag into tools/mgba-src (gitignored), configures
  scripts/mgba-wav/CMakeLists.txt into tools/mgba-build (libmgba only, static, no dependencies,
  GBA core only) and copies the resulting mgba-wav.exe to tools/. Idempotent: an existing clone
  and build directory are reused; -Clean removes the build directory first.

  Toolchain: Ninja + the MinGW gcc on PATH when both are found, otherwise the Visual Studio 2022
  generator. Nothing is installed.
.PARAMETER MgbaVersion
  mGBA tag to build (default 0.10.5, the same as the portable build fetch-tests.ps1 downloads).
.PARAMETER Clean
  Delete tools/mgba-build before configuring.
#>
[CmdletBinding()]
param(
    [string]$MgbaVersion = '0.10.5',
    [switch]$Clean
)
$ErrorActionPreference = 'Stop'
$repo = Resolve-Path (Join-Path $PSScriptRoot '..')
$src = Join-Path $repo 'tools\mgba-src'
$build = Join-Path $repo 'tools\mgba-build'
$tool = Join-Path $repo 'scripts\mgba-wav'

if (-not (Test-Path (Join-Path $src 'CMakeLists.txt'))) {
    Write-Host "Cloning mGBA $MgbaVersion into $src"
    git clone -q --depth 1 --branch $MgbaVersion https://github.com/mgba-emu/mgba $src
    if ($LASTEXITCODE -ne 0) { throw "git clone failed" }
} else {
    Write-Host "mGBA source: $src"
}

if ($Clean -and (Test-Path $build)) {
    Remove-Item -Recurse -Force $build
}

$gcc = Get-Command gcc -ErrorAction SilentlyContinue
$ninja = Get-Command ninja -ErrorAction SilentlyContinue
if ($gcc -and $ninja) {
    Write-Host "Configuring with Ninja + $($gcc.Source)"
    cmake -S $tool -B $build -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_C_COMPILER=gcc "-DCMAKE_POLICY_VERSION_MINIMUM=3.5" -Wno-dev "-DMGBA_SRC=$src"
} else {
    Write-Host "Configuring with Visual Studio 17 2022"
    cmake -S $tool -B $build -G 'Visual Studio 17 2022' -A x64 "-DCMAKE_POLICY_VERSION_MINIMUM=3.5" -Wno-dev "-DMGBA_SRC=$src"
}
if ($LASTEXITCODE -ne 0) { throw "cmake configure failed" }

foreach ($name in 'mgba-wav', 'mgba-ref') {
    cmake --build $build --config Release --target $name
    if ($LASTEXITCODE -ne 0) { throw "build of $name failed" }
    $exe = Get-ChildItem -Path $build -Recurse -Filter "$name.exe" | Select-Object -First 1
    if (-not $exe) { throw "$name.exe not found under $build" }
    Copy-Item $exe.FullName (Join-Path $repo "tools\$name.exe") -Force
    Write-Host "${name}: $(Join-Path $repo "tools\$name.exe")"
}
