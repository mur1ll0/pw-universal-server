# Compila o pw-launcher para rodar do Windows 7 ao 11 (B275).
#
# Desde o Rust 1.78 os alvos *-pc-windows-msvc exigem Windows 10 (a std importa WaitOnAddress,
# ProcessPrng, GetSystemTimePreciseAsFileTime). Os alvos *-win7-windows-msvc trocam essas chamadas
# por equivalentes do Windows 7, mas são Tier 3: a std é recompilada do fonte (-Zbuild-std), o que
# pede o componente rust-src e RUSTC_BOOTSTRAP=1 no Rust estável. O CRT vai estático
# (+crt-static): o exe não depende do Universal CRT, que o Windows 7 só tem com o KB2999226.
#
# Uso:  .\compilar.ps1            -> 64 bits  (target\x86_64-win7-windows-msvc\release\pw-launcher.exe)
#       .\compilar.ps1 -Bits 32   -> 32 bits  (target\i686-win7-windows-msvc\release\pw-launcher.exe)
param([ValidateSet(32, 64)][int]$Bits = 64)
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
$alvo = if ($Bits -eq 32) { 'i686-win7-windows-msvc' } else { 'x86_64-win7-windows-msvc' }
rustup component add rust-src | Out-Null
$env:RUSTC_BOOTSTRAP = '1'
$env:RUSTFLAGS = '-C target-feature=+crt-static'
cargo build --release --target $alvo -Zbuild-std=std
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host "pronto: target\$alvo\release\pw-launcher.exe"
