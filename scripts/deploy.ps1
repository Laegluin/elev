Push-Location "$PSScriptRoot/.."

try {
    cargo build --release --bin elev-run --features require-elevation
    $env:ELEV_RUN_SHA256 = (Get-FileHash "target/release/elev-run.exe" -Algorithm SHA256).Hash.ToLower()
    cargo build --release --bin elev

    $zipDir = "target/release/elev-$env:TRAVIS_TAG"
    New-Item $zipDir -ItemType Directory
    Copy-Item "target/release/*.exe" $zipDir
    Compress-Archive $zipDir -DestinationPath "$zipDir.zip" -CompressionLevel Optimal
}
finally {
    Pop-Location
}
