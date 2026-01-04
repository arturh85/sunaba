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

# Run multiplayer client (connects to specified SpacetimeDB server)
start-multiplayer server="http://localhost:3000":
    @echo "Starting multiplayer client (connecting to {{server}})..."
    @echo "Logs will appear in the game window - check the console there"
    RUST_LOG=info cargo run -p sunaba --bin sunaba --release --features multiplayer_native -- --server {{server}}

# Join local development server (localhost:3000)
join:
    just start-multiplayer http://localhost:3000

# Join production server (sunaba.app42.blue)
join-prod:
    just start-multiplayer https://sunaba.app42.blue

[unix]
test: fmt clippy
    @cargo test --workspace --quiet 2>&1 | grep -v "running 0 tests" | grep -v "ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s" | awk 'NF{print; blank=1} !NF && blank{print ""; blank=0}'
    cargo build --features "headless,multiplayer_native" -p sunaba --release
    just build-web
    just spacetime-build
    just spacetime-verify-clients
    just spacetime-verify-ts

[windows]
test: fmt clippy
    cargo test --workspace --quiet
    cargo build --features "headless,multiplayer_native" -p sunaba --release
    just build-web
    just spacetime-build
    just spacetime-verify-clients
    just spacetime-verify-ts

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
# Code Coverage Commands
# ============================================================================

# Check code coverage for a specific package and/or path filter
# Usage: just coverage [package] [path_filter]
# Examples:
#   just coverage                           # Whole workspace summary
#   just coverage sunaba-core               # Just sunaba-core package
#   just coverage sunaba-core src/world     # sunaba-core package, only src/world files
#   just coverage "" crates/sunaba-core     # Whole workspace, filter by path
[unix]
coverage package="" path_filter="":
    #!/usr/bin/env bash
    set -euo pipefail
    command -v cargo-llvm-cov >/dev/null 2>&1 || cargo install cargo-llvm-cov
    if [ -z "{{package}}" ] && [ -z "{{path_filter}}" ]; then \
        echo "Running coverage for entire workspace..."; \
        cargo llvm-cov --workspace --all-features --summary-only; \
    else \
        PKG_FLAG=""; \
        if [ -n "{{package}}" ]; then \
            PKG_FLAG="-p {{package}}"; \
            echo "Running coverage for package: {{package}}"; \
        else \
            echo "Running coverage for entire workspace..."; \
        fi; \
        cargo llvm-cov $PKG_FLAG --all-features 2>/dev/null > /tmp/coverage_output.txt; \
        if [ -n "{{path_filter}}" ]; then \
            echo "Filtering results for: {{path_filter}}"; \
            echo ""; \
            head -2 /tmp/coverage_output.txt; \
            grep "{{path_filter}}" /tmp/coverage_output.txt | head -20; \
            echo ""; \
            echo "Summary for {{path_filter}}:"; \
            grep "{{path_filter}}" /tmp/coverage_output.txt | awk '{gsub(/%/,"",$10); gsub(/%/,"",$4); gsub(/%/,"",$7); if($10!="Cover") {lines+=$10; regions+=$4; funcs+=$7; count++}} END {if(count>0) printf "  Lines: %.1f%% | Regions: %.1f%% | Functions: %.1f%%\n", lines/count, regions/count, funcs/count}'; \
        else \
            tail -10 /tmp/coverage_output.txt; \
        fi; \
        rm -f /tmp/coverage_output.txt; \
    fi

[windows]
coverage package="" path_filter="":
    @if (-not (Get-Command cargo-llvm-cov -ErrorAction SilentlyContinue)) { cargo install cargo-llvm-cov }
    @if ("{{package}}" -eq "" -and "{{path_filter}}" -eq "") { \
        Write-Host "Running coverage for entire workspace..."; \
        cargo llvm-cov --workspace --all-features --summary-only; \
    } else { \
        if ("{{package}}" -ne "") { \
            Write-Host "Running coverage for package: {{package}}"; \
            cargo llvm-cov -p {{package}} --all-features --text 2>$null | Select-String "{{path_filter}}"; \
        } else { \
            Write-Host "Running coverage for entire workspace..."; \
            cargo llvm-cov --workspace --all-features --text 2>$null | Select-String "{{path_filter}}"; \
        } \
    }

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

# Publish to SpacetimeDB instance (default: local)
spacetime-publish name="sunaba" server="http://localhost:3000":
    cd crates/sunaba-server && spacetime publish {{name}} -s {{server}}

# View SpacetimeDB logs
spacetime-logs name="sunaba" server="http://localhost:3000":
    spacetime logs {{name}} -s {{server}}

# Follow SpacetimeDB logs (tail -f style)
spacetime-logs-tail name="sunaba" server="http://localhost:3000":
    spacetime logs {{name}} -s {{server}} -f

# Generate TypeScript client SDK from server module
spacetime-generate-ts:
    @echo "Generating TypeScript client from server schema..."
    spacetime generate --lang typescript --project-path crates/sunaba-server --out-dir web/src/spacetime
    @echo "TypeScript client generated successfully"

# Generate Rust client SDK from server module
spacetime-generate-rust:
    @echo "Generating Rust client from server schema..."
    spacetime generate --lang rust --project-path crates/sunaba-server --out-dir crates/sunaba/src/multiplayer/generated
    @echo "Formatting generated Rust code..."
    cargo fmt --manifest-path crates/sunaba/Cargo.toml
    @echo "Rust client generated successfully"

# Verify generated clients are up-to-date (auto-regenerates, gitignored)
[unix]
spacetime-verify-clients:
    @echo "Verifying Rust client is current..."
    @just spacetime-generate-rust > /dev/null 2>&1
    @echo "✅ Rust client regenerated (gitignored)"

[windows]
spacetime-verify-clients:
    @echo "Verifying Rust client is current..."
    @just spacetime-generate-rust | Out-Null
    @echo "✅ Rust client regenerated (gitignored)"

# Verify TypeScript client is up-to-date (auto-regenerates, gitignored)
[unix]
spacetime-verify-ts:
    @echo "Verifying TypeScript client is current..."
    @just spacetime-generate-ts > /dev/null 2>&1
    @echo "✅ TypeScript client regenerated (gitignored)"

[windows]
spacetime-verify-ts:
    @echo "Verifying TypeScript client is current..."
    @just spacetime-generate-ts | Out-Null
    @echo "✅ TypeScript client regenerated (gitignored)"

# Call a reducer manually (for testing)
spacetime-call name="sunaba" reducer="init" server="http://localhost:3000":
    spacetime call -s {{server}} {{name}} {{reducer}}

# Full local development setup: build, start, publish
[unix]
spacetime-dev name="sunaba":
    @echo "Setting up SpacetimeDB local development..."
    just spacetime-build
    just spacetime-start
    just spacetime-publish {{name}}
    @echo "SpacetimeDB ready! Module published as '{{name}}'"
    @echo "Run 'just spacetime-logs-tail {{name}}' to watch logs"

[windows]
spacetime-dev name="sunaba":
    @echo "Setting up SpacetimeDB local development..."
    just spacetime-build
    just spacetime-start
    just spacetime-publish {{name}}
    @echo "SpacetimeDB ready! Module published as '{{name}}'"
    @echo "Run 'just spacetime-logs-tail {{name}}' to watch logs"

# Reset database (delete and republish)
spacetime-reset name="sunaba" server="http://localhost:3000":
    spacetime delete -s {{server}} {{name}} || true
    just spacetime-publish {{name}} {{server}}

# Show database status
spacetime-status name="sunaba" server="http://localhost:3000":
    spacetime describe -s {{server}} {{name}}

# ============================================================================
# Production Server Convenience Commands (sunaba.app42.blue)
# ============================================================================

# Publish to production server
spacetime-publish-prod name="sunaba":
    just spacetime-publish {{name}} https://sunaba.app42.blue

# Tail logs from production server
spacetime-logs-tail-prod name="sunaba":
    just spacetime-logs-tail {{name}} https://sunaba.app42.blue

# Show production database status
spacetime-status-prod name="sunaba":
    just spacetime-status {{name}} https://sunaba.app42.blue

# Reset production database - USE WITH CAUTION
spacetime-reset-prod name="sunaba":
    just spacetime-reset {{name}} https://sunaba.app42.blue
