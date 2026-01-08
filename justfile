# Cross-platform shell configuration
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# Environment variable for RUST_LOG
export RUST_LOG := "info"

# Internal helper: ensure SpacetimeDB CLI is installed and generated files exist
[unix]
_ensure-generated:
    #!/usr/bin/env bash
    set -euo pipefail
    # Check if generated.rs exists
    if [ ! -f "crates/sunaba/src/multiplayer/generated/mod.rs" ]; then
        echo "⚠️  Generated files missing, setting up..."
        # Check if spacetime CLI is installed
        if ! command -v spacetime &> /dev/null; then
            echo "Installing SpacetimeDB CLI..."
            curl --proto '=https' --tlsv1.2 -sSf https://install.spacetimedb.com | sh -s -- -y
            export PATH="$HOME/.local/share/spacetime/bin:$PATH"
        fi
        echo "Generating Rust client..."
        just spacetime-generate-rust > /dev/null 2>&1
        echo "✅ Generated files ready"
    fi

[windows]
_ensure-generated:
    @if (-not (Test-Path "crates/sunaba/src/multiplayer/generated/mod.rs")) { \
        Write-Host "⚠️  Generated files missing, setting up..."; \
        if (-not (Get-Command spacetime -ErrorAction SilentlyContinue)) { \
            Write-Host "Installing SpacetimeDB CLI..."; \
            irm https://windows.spacetimedb.com | iex; \
        } \
        Write-Host "Generating Rust client..."; \
        just spacetime-generate-rust | Out-Null; \
        Write-Host "✅ Generated files ready"; \
    }

start:
    cargo run -p sunaba --bin sunaba --release --no-default-features --features native -- --regenerate

load:
    cargo run -p sunaba --bin sunaba --release

# ============================================================================
# Fast Development Commands (Auto-Rebuild + Instant Launch)
# ============================================================================

# Auto-rebuild on file changes (defaults to build, switch with keys)
# Keyboard shortcuts: b=build, t=test, q=quit
watch:
    bacon

# Direct binary execution (instant launch, ~100ms, bypasses Cargo overhead)
# Use with `just watch` for truly instant hot-reload
run *args='--regenerate':
    ./target/release/sunaba {{args}}

# ============================================================================
# End Fast Development Commands
# ============================================================================

# ============================================================================
# Screenshot Commands
# ============================================================================

# List all available screenshot scenarios (levels + UI panels)
list-scenarios:
    cargo run -p sunaba --bin sunaba --release --features headless -- --list-scenarios

# List all available demo levels for screenshots
list-levels:
    cargo run -p sunaba --bin sunaba --release --features headless -- --list-levels

# Capture a screenshot (level:N, ui:panel, or just N)
# Usage: just screenshot <scenario> [width] [height] [settle_frames]
# Example: just screenshot 0            # Level 0 (backward compatible)
# Example: just screenshot level:3      # Level 3 (explicit)
# Example: just screenshot ui:params    # Parameters panel (coming soon)
# Example: just screenshot 3 800 600    # Custom resolution
[unix]
screenshot scenario width="1920" height="1080" settle="60":
    @mkdir -p screenshots
    cargo run -p sunaba --bin sunaba --release --features headless -- --screenshot {{scenario}} --screenshot-width {{width}} --screenshot-height {{height}} --screenshot-settle {{settle}}
    @echo "Screenshot captured successfully!"

[windows]
screenshot scenario width="1920" height="1080" settle="60":
    @if (-not (Test-Path screenshots)) { New-Item -ItemType Directory -Path screenshots | Out-Null }
    cargo run -p sunaba --bin sunaba --release --features headless -- --screenshot {{scenario}} --screenshot-width {{width}} --screenshot-height {{height}} --screenshot-settle {{settle}}
    @Write-Host "Screenshot captured successfully!"

# Capture screenshots of all demo levels
[unix]
screenshot-all width="1920" height="1080":
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p screenshots
    echo "Capturing screenshots of all levels..."
    for i in {0..20}; do
        echo "  Level $i..."
        just screenshot $i {{width}} {{height}} > /dev/null 2>&1 || true
    done
    echo "✅ All screenshots captured in screenshots/"

[windows]
screenshot-all width="1920" height="1080":
    @if (-not (Test-Path screenshots)) { New-Item -ItemType Directory -Path screenshots | Out-Null }
    @Write-Host "Capturing screenshots of all levels..."
    @for ($i=0; $i -le 20; $i++) { \
        Write-Host "  Level $i..."; \
        just screenshot $i {{width}} {{height}} | Out-Null; \
    }
    @Write-Host "✅ All screenshots captured in screenshots/"

# List available UI panels for screenshots
[unix]
list-ui-panels:
    cargo run -p sunaba --bin sunaba --release --features headless -- --list-ui-panels

[windows]
list-ui-panels:
    cargo run -p sunaba --bin sunaba --release --features headless -- --list-ui-panels

# Capture screenshot of a single UI panel
# Usage: just screenshot-ui inventory
# Available panels: params, inventory, crafting, logger, worldgen, levels, multiplayer
[unix]
screenshot-ui panel width="1920" height="1080":
    @mkdir -p screenshots
    cargo run -p sunaba --bin sunaba --release --features headless -- --screenshot-ui --ui-panel {{panel}} --screenshot-width {{width}} --screenshot-height {{height}}
    @echo "UI screenshot captured: screenshots/ui_{{panel}}.png"

[windows]
screenshot-ui panel width="1920" height="1080":
    @if (-not (Test-Path screenshots)) { New-Item -ItemType Directory -Path screenshots | Out-Null }
    cargo run -p sunaba --bin sunaba --release --features headless -- --screenshot-ui --ui-panel {{panel}} --screenshot-width {{width}} --screenshot-height {{height}}
    @Write-Host "UI screenshot captured: screenshots/ui_{{panel}}.png"

# Capture screenshots of all UI panels
[unix]
screenshot-ui-all width="1920" height="1080":
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p screenshots
    echo "Capturing screenshots of all UI panels..."
    for panel in params inventory crafting logger worldgen levels; do
        echo "  Panel: $panel..."
        just screenshot-ui $panel {{width}} {{height}} > /dev/null 2>&1 || true
    done
    echo "✅ All UI screenshots captured in screenshots/"

[windows]
screenshot-ui-all width="1920" height="1080":
    @if (-not (Test-Path screenshots)) { New-Item -ItemType Directory -Path screenshots | Out-Null }
    @Write-Host "Capturing screenshots of all UI panels..."
    @foreach ($panel in @("params", "inventory", "crafting", "logger", "worldgen", "levels")) { \
        Write-Host "  Panel: $panel..."; \
        just screenshot-ui $panel {{width}} {{height}} | Out-Null; \
    }
    @Write-Host "✅ All UI screenshots captured in screenshots/"

# ============================================================================
# End Screenshot Commands
# ============================================================================

# ============================================================================
# Scenario Testing Commands
# ============================================================================

# Run a single test scenario from RON file
# Usage: just test-scenario <file.ron>
# Example: just test-scenario scenarios/test_mining.ron
test-scenario scenario_file:
    cargo run -p sunaba --bin sunaba --release --features headless -- --test-scenario {{scenario_file}} --scenario-screenshots

# Run a test scenario from stdin (RON format)
# Usage: echo '(name: "Test", ...)' | just test-scenario-stdin
# Example: cat scenarios/test.ron | just test-scenario-stdin
test-scenario-stdin:
    cargo run -p sunaba --bin sunaba --release --features headless -- --test-scenario-stdin --scenario-screenshots

# Run all scenario tests in scenarios/ directory
[unix]
test-scenario-all:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ ! -d "scenarios" ]; then
        echo "⚠️  No scenarios/ directory found"
        exit 0
    fi
    shopt -s nullglob
    scenarios=(scenarios/*.ron)
    if [ ${#scenarios[@]} -eq 0 ]; then
        echo "⚠️  No .ron scenario files found in scenarios/"
        exit 0
    fi
    echo "Running ${#scenarios[@]} scenarios..."
    passed=0
    failed=0
    for scenario in "${scenarios[@]}"; do
        echo ""
        echo "═══════════════════════════════════════════════════"
        echo "Running: $scenario"
        echo "═══════════════════════════════════════════════════"
        if just test-scenario "$scenario"; then
            ((passed++))
        else
            ((failed++))
        fi
    done
    echo ""
    echo "═══════════════════════════════════════════════════"
    echo "Results: $passed passed, $failed failed"
    echo "═══════════════════════════════════════════════════"
    if [ $failed -gt 0 ]; then
        exit 1
    fi

[windows]
test-scenario-all:
    @if (-not (Test-Path scenarios)) { Write-Host "⚠️  No scenarios/ directory found"; exit 0 }
    @$scenarios = Get-ChildItem -Path scenarios -Filter *.ron -File
    @if ($scenarios.Count -eq 0) { Write-Host "⚠️  No .ron scenario files found in scenarios/"; exit 0 }
    @Write-Host "Running $($scenarios.Count) scenarios..."
    @$passed = 0
    @$failed = 0
    @foreach ($scenario in $scenarios) { \
        Write-Host ""; \
        Write-Host "═══════════════════════════════════════════════════"; \
        Write-Host "Running: $($scenario.FullName)"; \
        Write-Host "═══════════════════════════════════════════════════"; \
        if (just test-scenario $scenario.FullName) { $passed++ } else { $failed++ } \
    }
    @Write-Host ""
    @Write-Host "═══════════════════════════════════════════════════"
    @Write-Host "Results: $passed passed, $failed failed"
    @Write-Host "═══════════════════════════════════════════════════"
    @if ($failed -gt 0) { exit 1 }

# ============================================================================
# End Scenario Testing Commands
# ============================================================================

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

# Fast development test: clippy + fmt + tests (no release builds, no WASM)
[unix]
test package="":
    #!/usr/bin/env bash
    set -euo pipefail
    just _ensure-generated
    echo "Running fast test suite..."
    just fmt
    just clippy
    if [ -z "{{package}}" ]; then \
        cargo test --workspace --all-features --quiet 2>&1 | grep -v "running 0 tests" | grep -v "ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s" | awk 'NF{print; blank=1} !NF && blank{print ""; blank=0}'; \
    else \
        cargo test -p {{package}} --all-features --quiet 2>&1 | grep -v "running 0 tests" | grep -v "ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s" | awk 'NF{print; blank=1} !NF && blank{print ""; blank=0}'; \
    fi
    echo "✅ Fast tests passed"

[windows]
test package="":
    @just _ensure-generated
    @Write-Host "Running fast test suite..."
    just fmt
    just clippy
    @if ("{{package}}" -eq "") { cargo test --workspace --all-features --quiet } else { cargo test -p {{package}} --all-features --quiet }
    @Write-Host "✅ Fast tests passed"

# Run scenario integration tests (slow, marked with #[ignore])
[unix]
test-scenarios:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Running scenario integration tests (slow)..."
    cargo test --test scenarios --features headless -- --ignored --test-threads=1 --nocapture
    echo "✅ Scenario tests passed"

[windows]
test-scenarios:
    @Write-Host "Running scenario integration tests (slow)..."
    cargo test --test scenarios --features headless -- --ignored --test-threads=1 --nocapture
    @Write-Host "✅ Scenario tests passed"

# Run all tests including slow scenario integration tests
[unix]
test-all package="":
    #!/usr/bin/env bash
    set -euo pipefail
    just test {{package}}
    just test-scenarios
    echo "✅ All tests passed"

[windows]
test-all package="":
    just test {{package}}
    just test-scenarios
    @Write-Host "✅ All tests passed"

# CI-equivalent: full validation with all builds
[unix]
test-ci:
    #!/usr/bin/env bash
    set -euo pipefail
    just _ensure-generated
    echo "Running CI-equivalent test suite..."
    just fmt
    just clippy
    cargo test --workspace --quiet 2>&1 | grep -v "running 0 tests" | grep -v "ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s" | awk 'NF{print; blank=1} !NF && blank{print ""; blank=0}'
    cargo build --features "headless,multiplayer_native" -p sunaba --release
    just build-web
    just spacetime-build
    just spacetime-verify-clients
    just spacetime-verify-ts
    echo "✅ All CI checks passed"

[windows]
test-ci:
    @just _ensure-generated
    @Write-Host "Running CI-equivalent test suite..."
    just fmt
    just clippy
    cargo test --workspace --quiet
    cargo build --features "headless,multiplayer_native" -p sunaba --release
    just build-web
    just spacetime-build
    just spacetime-verify-clients
    just spacetime-verify-ts
    @Write-Host "✅ All CI checks passed"

fmt:
    @just _ensure-generated
    cargo fmt --all
    cargo fmt --all -- --check

# Format check for CI (ensures generated files exist first)
fmt-check:
    @just _ensure-generated
    cargo fmt --all -- --check

clippy:
    cargo clippy --fix --workspace --tests --allow-dirty

[unix]
check crate="":
    #!/usr/bin/env bash
    set -euo pipefail
    just _ensure-generated
    if [ -z "{{crate}}" ]; then
        echo "Checking workspace..."
        cargo clippy --fix --workspace --tests --allow-dirty
        cargo fmt --all
        cargo fmt --all -- --check
        cargo check --workspace
    else
        echo "Checking crate: {{crate}}..."
        cargo clippy --fix -p {{crate}} --tests --allow-dirty
        cargo fmt --manifest-path crates/{{crate}}/Cargo.toml
        cargo fmt --manifest-path crates/{{crate}}/Cargo.toml -- --check
        cargo check -p {{crate}}
    fi
    echo "✅ Check complete"

[windows]
check crate="":
    @just _ensure-generated
    @if ("{{crate}}" -eq "") { \
        Write-Host "Checking workspace..."; \
        cargo clippy --fix --workspace --tests --allow-dirty; \
        cargo fmt --all; \
        cargo fmt --all -- --check; \
        cargo check --workspace; \
    } else { \
        Write-Host "Checking crate: {{crate}}..."; \
        cargo clippy --fix -p {{crate}} --tests --allow-dirty; \
        cargo fmt --manifest-path crates/{{crate}}/Cargo.toml; \
        cargo fmt --manifest-path crates/{{crate}}/Cargo.toml -- --check; \
        cargo check -p {{crate}}; \
    }
    @Write-Host "✅ Check complete"

# Check sccache compiler cache statistics
profile-cache:
    @if command -v sccache >/dev/null 2>&1; then \
        echo "sccache statistics:"; \
        sccache --show-stats; \
    else \
        echo "sccache not installed. Install with: cargo install sccache"; \
        echo "Then add to .cargo/config.toml:"; \
        echo "  [build]"; \
        echo "  rustc-wrapper = \"sccache\""; \
    fi

[unix]
build-web:
    @echo "Building Sunaba for Web (WASM)..."
    @command -v wasm-bindgen >/dev/null 2>&1 || cargo install wasm-bindgen-cli --version 0.2.106
    @mkdir -p web/pkg
    RUSTFLAGS='--cfg getrandom_backend="wasm_js"' cargo build --lib --release --target wasm32-unknown-unknown -p sunaba --no-default-features --features multiplayer_wasm
    wasm-bindgen --out-dir web/pkg --no-typescript --target web target/wasm32-unknown-unknown/release/sunaba.wasm
    @echo "Build complete! Output in web/pkg/"

[windows]
build-web:
    @echo "Building Sunaba for Web (WASM)..."
    @if (-not (Get-Command wasm-bindgen -ErrorAction SilentlyContinue)) { cargo install wasm-bindgen-cli --version 0.2.106 }
    @if (-not (Test-Path web\pkg)) { New-Item -ItemType Directory -Path web\pkg | Out-Null }
    $env:RUSTFLAGS='--cfg getrandom_backend="wasm_js"'; cargo build --lib --release --target wasm32-unknown-unknown -p sunaba --no-default-features --features multiplayer_wasm
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
        RUSTC_WRAPPER="" RUSTFLAGS="-C link-arg=-framework -C link-arg=Accelerate" cargo llvm-cov --workspace --all-features --summary-only; \
    else \
        PKG_FLAG=""; \
        if [ -n "{{package}}" ]; then \
            PKG_FLAG="-p {{package}}"; \
            echo "Running coverage for package: {{package}}"; \
        else \
            echo "Running coverage for entire workspace..."; \
        fi; \
        RUSTC_WRAPPER="" RUSTFLAGS="-C link-arg=-framework -C link-arg=Accelerate" cargo llvm-cov $PKG_FLAG --all-features 2>/dev/null > /tmp/coverage_output.txt; \
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
        $env:RUSTC_WRAPPER=""; $env:RUSTFLAGS=""; cargo llvm-cov --workspace --all-features --summary-only; \
    } else { \
        if ("{{package}}" -ne "") { \
            Write-Host "Running coverage for package: {{package}}"; \
            $env:RUSTC_WRAPPER=""; $env:RUSTFLAGS=""; cargo llvm-cov -p {{package}} --all-features --text 2>$null | Select-String "{{path_filter}}"; \
        } else { \
            Write-Host "Running coverage for entire workspace..."; \
            $env:RUSTC_WRAPPER=""; $env:RUSTFLAGS=""; cargo llvm-cov --workspace --all-features --text 2>$null | Select-String "{{path_filter}}"; \
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

# Build the SpacetimeDB module (WASM) and regenerate clients
[unix]
spacetime-build:
    @echo "Building SpacetimeDB module..."
    spacetime build -p crates/sunaba-server
    @echo "Regenerating clients from schema..."
    @just spacetime-generate-rust > /dev/null 2>&1
    @just spacetime-generate-ts > /dev/null 2>&1
    @echo "Build complete! (clients auto-generated)"

[windows]
spacetime-build:
    @echo "Building SpacetimeDB module..."
    spacetime build -p crates/sunaba-server
    @echo "Regenerating clients from schema..."
    @just spacetime-generate-rust | Out-Null
    @just spacetime-generate-ts | Out-Null
    @echo "Build complete! (clients auto-generated)"

# Check if SpacetimeDB server is running
[unix]
spacetime-check server="http://localhost:3000":
    #!/usr/bin/env bash
    if curl -s --connect-timeout 2 {{server}}/database/list > /dev/null 2>&1; then
        echo "✅ SpacetimeDB server is running at {{server}}"
        exit 0
    else
        echo "❌ SpacetimeDB server is not running at {{server}}"
        exit 1
    fi

[windows]
spacetime-check server="http://localhost:3000":
    @try { \
        Invoke-WebRequest -Uri "{{server}}/database/list" -TimeoutSec 2 -UseBasicParsing | Out-Null; \
        Write-Host "✅ SpacetimeDB server is running at {{server}}"; \
        exit 0; \
    } catch { \
        Write-Host "❌ SpacetimeDB server is not running at {{server}}"; \
        exit 1; \
    }

# Start local SpacetimeDB instance (checks if already running)
[unix]
spacetime-start:
    @if just spacetime-check > /dev/null 2>&1; then \
        echo "SpacetimeDB server already running"; \
    else \
        echo "Starting SpacetimeDB server..."; \
        spacetime start & \
        sleep 2; \
        echo "SpacetimeDB local instance started"; \
    fi

[windows]
spacetime-start:
    @if (just spacetime-check) { \
        Write-Host "SpacetimeDB server already running"; \
    } else { \
        Write-Host "Starting SpacetimeDB server..."; \
        Start-Process spacetime -ArgumentList "start" -NoNewWindow; \
        Start-Sleep -Seconds 2; \
        Write-Host "SpacetimeDB local instance started"; \
    }

# Stop local SpacetimeDB instance
spacetime-stop:
    killall spacetime

# Publish to SpacetimeDB instance (default: local)
# Note: Authentication must be done separately via `spacetime login`
[unix]
spacetime-publish name="sunaba" server="http://localhost:3000":
    #!/usr/bin/env bash
    cd crates/sunaba-server
    spacetime publish {{name}} -s {{server}} -y

[windows]
spacetime-publish name="sunaba" server="http://localhost:3000":
    @cd crates/sunaba-server
    spacetime publish {{name}} -s {{server}} -y

# View SpacetimeDB logs
spacetime-logs name="sunaba" server="http://localhost:3000":
    spacetime logs {{name}} -s {{server}}

# Follow SpacetimeDB logs (tail -f style)
spacetime-logs-tail name="sunaba" server="http://localhost:3000":
    spacetime logs {{name}} -s {{server}} -f

# Generate TypeScript client SDK from server module
spacetime-generate-ts:
    @echo "Generating TypeScript client from server schema..."
    spacetime generate --lang typescript --project-path crates/sunaba-server --out-dir web/src/generated -y
    @echo "TypeScript client generated successfully"

# Generate Rust client SDK from server module
spacetime-generate-rust:
    @echo "Generating Rust client from server schema..."
    spacetime generate --lang rust --project-path crates/sunaba-server --out-dir crates/sunaba/src/multiplayer/generated -y
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
    spacetime delete -s {{server}} {{name}} -y || true
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
