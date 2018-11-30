Push-Location $PSScriptRoot

try {
    cargo build --release --bin elev-run --features require-elevation
    $env:ELEV_RUN_SHA256 = (Get-FileHash ".\target\release\elev-run.exe" -Algorithm SHA256).Hash.ToLower()
    cargo build --release --bin elev
}
finally {
    Pop-Location
}
