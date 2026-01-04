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

[unix]
test: fmt clippy
    @cargo test --workspace --quiet 2>&1 | grep -v "running 0 tests" | grep -v "ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s" | awk 'NF{print; blank=1} !NF && blank{print ""; blank=0}'
    cargo build --workspace --release
    just build-web
    just spacetime-build

[windows]
test: fmt clippy
    cargo test --workspace --quiet
    cargo build --workspace --release
    just build-web
    just spacetime-build

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
    RUSTFLAGS='--cfg getrandom_backend="wasm_js"' cargo build --lib --release --target wasm32-unknown-unknown -p sunaba --no-default-features
    wasm-bindgen --out-dir web/pkg --no-typescript --target web target/wasm32-unknown-unknown/release/sunaba.wasm
    @echo "Build complete! Output in web/pkg/"

[windows]
build-web:
    @echo "Building Sunaba for Web (WASM)..."
    @if (-not (Get-Command wasm-bindgen -ErrorAction SilentlyContinue)) { cargo install wasm-bindgen-cli --version 0.2.106 }
    @if (-not (Test-Path web\pkg)) { New-Item -ItemType Directory -Path web\pkg | Out-Null }
    $env:RUSTFLAGS='--cfg getrandom_backend="wasm_js"'; cargo build --lib --release --target wasm32-unknown-unknown -p sunaba --no-default-features
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

# ============================================================================
# SpacetimeDB Commands
# ============================================================================

# Install SpacetimeDB CLI
[unix]
spacetime-install:
    curl --proto '=https' --tlsv1.2 -sSf https://install.spacetimedb.com | sh

[windows]
spacetime-install:
    irm https://windows.spacetimedb.com | iex

# Check SpacetimeDB CLI version
spacetime-version:
    spacetime version

# Build the SpacetimeDB module (WASM)
[unix]
spacetime-build:
    @echo "Building SpacetimeDB module..."
    spacetime build -p crates/sunaba-server
    @echo "Build complete!"

[windows]
spacetime-build:
    @echo "Building SpacetimeDB module..."
    spacetime build -p crates/sunaba-server
    @echo "Build complete!"

# Start local SpacetimeDB instance
[unix]
spacetime-start:
    spacetime start &
    @sleep 2
    @echo "SpacetimeDB local instance started"

[windows]
spacetime-start:
    Start-Process spacetime -ArgumentList "start" -NoNewWindow
    Start-Sleep -Seconds 2
    @echo "SpacetimeDB local instance started"

# Stop local SpacetimeDB instance
spacetime-stop:
    spacetime stop

# Publish to local SpacetimeDB instance
spacetime-publish-local name="sunaba":
    spacetime publish --skip-clippy -c local {{name}} crates/sunaba-server

# Publish to SpacetimeDB cloud (requires auth)
spacetime-publish-cloud name="sunaba":
    spacetime publish --skip-clippy {{name}} crates/sunaba-server

# View SpacetimeDB logs
spacetime-logs name="sunaba":
    spacetime logs -c local {{name}}

# Follow SpacetimeDB logs (tail -f style)
spacetime-logs-tail name="sunaba":
    spacetime logs -c local {{name}} -f

# Generate TypeScript client SDK
spacetime-generate-ts name="sunaba" output="web/src/spacetime":
    spacetime generate -c local --lang typescript --out-dir {{output}} {{name}}

# Generate Rust client SDK
spacetime-generate-rust name="sunaba" output="crates/sunaba/src/spacetime_client":
    spacetime generate -c local --lang rust --out-dir {{output}} {{name}}

# Call a reducer manually (for testing)
spacetime-call name="sunaba" reducer="init":
    spacetime call -c local {{name}} {{reducer}}

# Full local development setup: build, start, publish
[unix]
spacetime-dev name="sunaba":
    @echo "Setting up SpacetimeDB local development..."
    just spacetime-build
    just spacetime-start
    just spacetime-publish-local {{name}}
    @echo "SpacetimeDB ready! Module published as '{{name}}'"
    @echo "Run 'just spacetime-logs-tail {{name}}' to watch logs"

[windows]
spacetime-dev name="sunaba":
    @echo "Setting up SpacetimeDB local development..."
    just spacetime-build
    just spacetime-start
    just spacetime-publish-local {{name}}
    @echo "SpacetimeDB ready! Module published as '{{name}}'"
    @echo "Run 'just spacetime-logs-tail {{name}}' to watch logs"

# Reset database (delete and republish)
spacetime-reset name="sunaba":
    spacetime delete -c local {{name}} || true
    just spacetime-publish-local {{name}}

# Show database status
spacetime-status name="sunaba":
    spacetime describe -c local {{name}}
