Push-Location "$PSScriptRoot/.."

function Run() {
    $rest = $args[1..($args.Length - 1)]
    & $args[0] @rest

    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

try {
    Run cargo test
    Run cargo build --release --bin elev-run --features require-elevation
    $env:ELEV_RUN_SHA256 = (Get-FileHash "target/release/elev-run.exe" -Algorithm SHA256).Hash.ToLower()
    Run cargo build --release --bin elev

    $zipDir = if ($env:TRAVIS_TAG) { "target/release/elev-$env:TRAVIS_TAG" } else { "target/release/elev" }
    New-Item $zipDir -ItemType Directory -Force
    Copy-Item "target/release/*.exe" $zipDir -Force
    Compress-Archive $zipDir -DestinationPath "$zipDir.zip" -CompressionLevel Optimal -Force
}
finally {
    Pop-Location
}
