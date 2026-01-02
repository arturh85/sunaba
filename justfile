# Cross-platform shell configuration
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# Environment variable for RUST_LOG
export RUST_LOG := "info"

start:
    cargo run -p sunaba --release -- --regenerate

load:
    cargo run -p sunaba --release

test: fmt clippy
    cargo test --workspace --quiet
    cargo build --workspace --release
    just build-web

fmt:
    cargo fmt --all
    cargo fmt --all -- --check

clippy:
    cargo clippy --fix --workspace --tests --allow-dirty

[unix]
build-web:
    @echo "Building Sunaba for Web (WASM)..."
    @command -v wasm-bindgen >/dev/null 2>&1 || cargo install wasm-bindgen-cli --version 0.2.106
    @mkdir -p web/pkg
    RUSTFLAGS='--cfg getrandom_backend="wasm_js"' cargo build --lib --release --target wasm32-unknown-unknown -p sunaba
    wasm-bindgen --out-dir web/pkg --no-typescript --target web target/wasm32-unknown-unknown/release/sunaba.wasm
    @echo "Build complete! Output in web/pkg/"

[windows]
build-web:
    @echo "Building Sunaba for Web (WASM)..."
    @if (-not (Get-Command wasm-bindgen -ErrorAction SilentlyContinue)) { cargo install wasm-bindgen-cli --version 0.2.106 }
    @if (-not (Test-Path web\pkg)) { New-Item -ItemType Directory -Path web\pkg | Out-Null }
    $env:RUSTFLAGS='--cfg getrandom_backend="wasm_js"'; cargo build --lib --release --target wasm32-unknown-unknown -p sunaba
    wasm-bindgen --out-dir web/pkg --no-typescript --target web target/wasm32-unknown-unknown/release/sunaba.wasm
    @echo "Build complete! Output in web/pkg/"

[unix]
web: build-web
    cd web && python3 -m http.server 8080

[windows]
web: build-web
    cd web; python -m http.server 8080

# Evolution training commands (RUST_LOG=warn for clean progress bar output)
train scenario="parcour" generations="100" population="50":
    rm -rf training_output
    RUST_LOG=warn cargo run -p sunaba --features headless --release -- --train --scenario {{scenario}} --generations {{generations}} --population {{population}}

# Quick training with simple morphology (fewer body parts, viability filter, movement-focused fitness)
train-quick generations="100":
    rm -rf training_output
    RUST_LOG=warn cargo run -p sunaba --features headless --release -- --train --scenario simple --simple --generations {{generations}} --population 50

# Full training with default complex morphology
train-full:
    rm -rf training_output
    RUST_LOG=warn cargo run -p sunaba --features headless --release -- --train --generations 500 --population 100
