# Cross-platform shell configuration
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# Environment variable for RUST_LOG
export RUST_LOG := "info"

start:
    cargo run -p sunaba --bin sunaba --release -- --regenerate

load:
    cargo run -p sunaba --bin sunaba --release

# Run with puffin profiling enabled (F3 to toggle profiler)
profile:
    cargo run -p sunaba --bin sunaba --release --features profiling

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
# Default trains ALL archetypes (evolved, spider, snake, worm, flyer) together
# Use --archetype to train a single archetype: just train parcour 100 50 "" evolved
[unix]
train scenario="parcour" generations="100" population="50" simple="" archetype="all":
    rm -rf training_output
    RUST_LOG=warn cargo run -p sunaba --bin sunaba --features headless --release -- --train --scenario {{scenario}} --generations {{generations}} --population {{population}} --archetype {{archetype}} {{simple}}

[windows]
train scenario="parcour" generations="100" population="50" simple="" archetype="all":
    @if (Test-Path training_output) { Remove-Item -Recurse -Force training_output }
    $env:RUST_LOG='warn'; cargo run -p sunaba --bin sunaba --features headless --release -- --train --scenario {{scenario}} --generations {{generations}} --population {{population}} --archetype {{archetype}} {{simple}}

# Quick training with all archetypes (100 generations, 100 population)
train-quick generations="100":
    just train parcour {{generations}} 100

# Full training with all archetypes (larger population, more generations)
train-full:
    just train parcour 500 200

# Train a single archetype (e.g., just train-single spider 100)
train-single archetype="evolved" generations="100":
    just train parcour {{generations}} 50 "" {{archetype}}
