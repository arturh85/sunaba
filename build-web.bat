@echo off
echo Building Sunaba for Web (WASM)...

REM Check if wasm-bindgen-cli is installed
where wasm-bindgen >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo wasm-bindgen-cli not found. Installing...
    cargo install wasm-bindgen-cli --version 0.2.92
)

REM Create output directory
echo Creating output directory...
if not exist web\pkg mkdir web\pkg

REM Build for wasm32 target
echo Compiling to WASM...
cargo build --release --target wasm32-unknown-unknown
if %ERRORLEVEL% NEQ 0 exit /b %ERRORLEVEL%

REM Generate JS bindings
echo Generating JS bindings...
wasm-bindgen --out-dir web\pkg --web --no-typescript --target web target\wasm32-unknown-unknown\release\sunaba.wasm
if %ERRORLEVEL% NEQ 0 exit /b %ERRORLEVEL%

echo.
echo ✅ Build complete! Output in web\pkg\
echo.
echo To test locally, run:
echo   cd web ^&^& python -m http.server 8080
echo Then open http://localhost:8080 in your browser
