# 砂場 Sunaba

![sanuba.jpg](sanuba.jpg)

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

```bash
# Debug build
cargo run

# Release build (faster simulation)
cargo run --release
```

## Development Status

🚧 **In Development**
- ✅ Core physics simulation (falling sand, temperature, chemistry, structural integrity)
- ✅ Persistent world with procedural generation
- 🔨 World enhancement for creature interactions (resources, light, advanced materials)
- 📋 ML creature system (morphology, neural control, evolution pipeline)

## License

MIT
