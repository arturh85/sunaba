# Research Progress

## Experiments

| Date | Experiment | Area | Status | Result |
|------|------------|------|--------|--------|
| 2026-01-03 | Neural Mining Output | Neural/Physics | Implemented | Added mining as neural output, expanded world to 700px |
| 2026-01-03 | [Food Collection Research](2026-01-03-food-collection-research.md) | Creature/Neural | In Progress | Investigating food collection behavior |
| 2026-01-03 | [World Scale Investigation](2026-01-03-world-scale-investigation.md) | World/Rendering | Implemented | 640x360, 12px player, larger caves |

## Key Discoveries

### 2026-01-03: Neural Mining Output (Implemented)
- **GIF Background**: Changed from sky blue (RGB 135,206,235) to black for better visibility
- **World Width**: Expanded parcour from 500px to 700px, added 3 more food items
- **Mining Neural Output**: Added mining as neural network output (joints.len() + 1 outputs)
  - Mining threshold: 0.3 (neural output must exceed to trigger)
  - Mines 3x3 area in movement direction
  - Can mine STONE and SAND materials
  - `blocks_mined` counter added to Creature for fitness tracking
- **Fitness Changes**: DirectionalFoodFitness now includes `mining_points: 2.0` per block mined
- **Training Result**: 30 generations, best fitness 2589 (Flyer archetype)
  - Flyers learn to fly very high/far right (y=71531!)
  - Ground-based creatures (Spider, Snake, Worm) achieve ~640-645 fitness
  - Mining behavior not yet evolved (requires more generations with wall pressure)

### 2026-01-03: World Scale (Implemented)
- **Original:** ~240x135 visible, player 16px = 11.9% of screen height
- **Noita reference:** 480x270 visible, player ~10-12px = ~4% of screen height
- **New settings:**
  - Player height: 16px → 12px
  - Default zoom: 0.015 → 0.0055 (shows ~640x360 pixels)
  - Cave noise frequency: halved for larger caverns
  - Cave thresholds: lowered for more open spaces
  - **Background layer added** - Shows darkened rock behind caves (40% brightness)
- **Result:** Player now takes ~3.3% of screen (close to Noita's 4%), caves have visible depth

## Next Research Priorities

1. **Mining system changes** - 4x4 mining patches for Terraria feel
2. **Food collection creature behavior** - Continue neural/behavior research
3. **Background interactions** - Moss growing from background to foreground
