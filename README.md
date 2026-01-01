# 砂場 Sunaba

![sunaba.jpg](sunaba.jpg)

A 2D falling-sand physics sandbox survival game featuring ML-evolved creatures with articulated bodies and emergent behaviors, inspired by Noita and Terraria.

## Features (Planned)

- **Emergent Physics**: Every pixel is simulated with material properties
- **Chemistry System**: Materials react with each other (fire spreads, water evaporates, acid dissolves)
- **ML-Evolved Creatures**: Pre-evolved populations with diverse morphologies and behaviors
  - Articulated bodies controlled by neural networks (CPPN-NEAT + MAP-Elites)
  - Emergent survival strategies: hunting, building, tool use, social behaviors
  - Taming and selective breeding with genetic crossover
  - Meaningful world interactions: mining, construction, combat
- **Persistent World**: Changes persist across sessions
- **Survival Gameplay**: Crafting, building, exploration, creature management

## Building

### Native Builds

```bash
# Debug build
cargo run

# Release build (faster simulation)
cargo run --release
```

### Web Build (WASM)

The game can run in browsers that support WebGPU (Chrome 113+, Edge 113+, Firefox/Safari with WebGPU enabled).

```bash
# Linux/macOS
./build-web.sh

# Windows
build-web.bat

# Test locally
cd web && python3 -m http.server 8080
# Then open http://localhost:8080
```

### CI/CD

The project includes GitHub Actions workflows for:

- **CI** (`ci.yml`): Automated builds and tests on Linux, Windows, and macOS for every push
- **Release** (`release.yml`): Creates release binaries for all platforms when you push a version tag
- **Pages** (`pages.yml`): Automatically deploys the web version to GitHub Pages on every push to main

To create a new release:
```bash
git tag v0.1.0
git push origin v0.1.0
```

## Downloads

- **Native Builds**: Download from [GitHub Releases](https://github.com/arturh85/sunaba/releases)
- **Web Version**: Play at [https://arturh85.github.io/sunaba](https://arturh85.github.io/sunuba)

## Development Status

🚧 **In Development**
- ✅ Core physics simulation (falling sand, temperature, chemistry, structural integrity)
- ✅ Persistent world with procedural generation
- 🔨 World enhancement for creature interactions (resources, light, advanced materials)
- 📋 ML creature system (morphology, neural control, evolution pipeline)

## License

MIT
