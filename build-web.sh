#!/bin/bash
set -e

echo "Building Sunaba for Web (WASM)..."

# Check if wasm-bindgen-cli is installed
if ! command -v wasm-bindgen &> /dev/null; then
    echo "wasm-bindgen-cli not found. Installing..."
    cargo install wasm-bindgen-cli --version 0.2.92
fi

# Create output directory
echo "Creating output directory..."
mkdir -p web/pkg

# Build for wasm32 target
echo "Compiling to WASM..."
cargo build --release --target wasm32-unknown-unknown

# Generate JS bindings
echo "Generating JS bindings..."
wasm-bindgen --out-dir web/pkg --web --no-typescript --target web target/wasm32-unknown-unknown/release/sunaba.wasm

echo "✅ Build complete! Output in web/pkg/"
echo ""
echo "To test locally, run:"
echo "  cd web && python3 -m http.server 8080"
echo "Then open http://localhost:8080 in your browser"
