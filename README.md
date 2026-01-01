# Sunaba 砂場（すなば）

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

## Downloads

- **Native Builds**: Download from [GitHub Releases](https://github.com/arturh85/sunaba/releases/)
- **Web Version**: Play at [https://arturh85.github.io/sunaba](https://arturh85.github.io/sunaba/)

## Building

Development works on Windows, Linux and MacOS.

Requires [Rust](https://www.rust-lang.org/tools/install) 1.56 or later and [Just](https://github.com/casey/just) as a command runner (optional).

### Native Builds

```bash
# Debug build
cargo run

# Release build (faster simulation)
cargo run --release

# Using just command runner
just start
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

# Using just command runner
just web
```

### Tests

To run all checks and tests, run before submitting a PR:

```bash
# Using just command runner
just test
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

## Name

砂場 (Sunaba) is traditionally written as “sand + place,” meaning sandbox in Japanese. 
In this project, the name is intentionally layered: while the reading remains Sunaba (すなば), 
the second kanji may shift between 庭 (garden), 生 (life), and 層 (stratum), each offering a different lens on the same world. 
Together, they reflect a space where simple particles form ecosystems, life emerges from matter, and deep layers of simulation accumulate over time. 
Like a sandbox in the truest sense, 砂場 is a place for experimentation and discovery, 
where complex behavior arises naturally from fundamental rules rather than scripted design.

## Development Status

🚧 **In Development**
- ✅ Core physics simulation (falling sand, temperature, chemistry, structural integrity)
- ✅ Persistent world with procedural generation
- 🔨 World enhancement for creature interactions (resources, light, advanced materials)
- 📋 ML creature system (morphology, neural control, evolution pipeline)

## License

MIT
