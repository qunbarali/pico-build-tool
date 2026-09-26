$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
$Embedded = Join-Path $Root "embedded"
$Binaries = Join-Path $Root "src-tauri\binaries"
New-Item -ItemType Directory -Force $Embedded | Out-Null
New-Item -ItemType Directory -Force $Binaries | Out-Null

$items = @(
  @{ Name="cmake.zip"; Url="https://github.com/Kitware/CMake/releases/download/v4.4.3/cmake-4.4.3-windows-x86_64.zip"; Sha="4d52ebab7193a698651639ed80d8d04fd903358843572cf44c7fd234cb7c26ab" },
  @{ Name="ninja.zip"; Url="https://github.com/ninja-build/ninja/releases/download/v1.13.2/ninja-win.zip"; Sha="07fc8261b42b20e71d1720b39068c2e14ffcee6396b76fb7a795fb460b78dc65" },
  @{ Name="arm-gcc.zip"; Url="https://developer.arm.com/-/media/Files/downloads/gnu/15.2.rel1/binrel/arm-gnu-toolchain-15.2.rel1-mingw-w64-i686-arm-none-eabi.zip"; Sha="b40db54536d2fdf0ff21f4316b56c1fc4d3b782b792c5b298bcbeaf5eccedb96" },
  @{ Name="pico-sdk.zip"; Url="https://github.com/raspberrypi/pico-sdk/archive/refs/tags/2.3.1.zip"; Sha=$null },
  @{ Name="picotool.zip"; Url="https://github.com/raspberrypi/pico-sdk-tools/releases/download/v2.3.1-0/picotool-2.3.1-x64-win.zip"; Sha="68730be0813f8f35be2cca147cf7f1572662d5dcf0f5ba468e02a6dd9e85db2b" }
)

foreach ($item in $items) {
  $path = Join-Path $Embedded $item.Name
  $valid = Test-Path $path
  if ($valid -and $item.Sha) {
    $actual = (Get-FileHash $path -Algorithm SHA256).Hash.ToLower()
    $valid = $actual -eq $item.Sha
  }
  if (-not $valid) {
    Write-Host "Downloading $($item.Name) ..."
    Invoke-WebRequest -Uri $item.Url -OutFile $path
    if ($item.Sha) {
      $actual = (Get-FileHash $path -Algorithm SHA256).Hash.ToLower()
      if ($actual -ne $item.Sha) { throw "Checksum mismatch for $($item.Name)" }
    }
  } else {
    Write-Host "Using cached $($item.Name)"
  }
}

$env:PICO_BUNDLE_DIR = $Embedded
Write-Host "Building self-contained pico-build engine ..."
cargo build --release --bin pico-build
if ($LASTEXITCODE -ne 0) { throw "pico-build engine compilation failed" }

$target = (& rustc -vV | Select-String '^host:' | ForEach-Object { ($_ -split ':')[1].Trim() })
if (-not $target) { throw "Unable to determine Rust host target" }

$sidecar = Join-Path $Binaries "pico-build-$target.exe"
Copy-Item (Join-Path $Root "target\release\pico-build.exe") $sidecar -Force
Write-Host "Prepared Tauri sidecar: $sidecar"
