# Sunaba World Generation V2 - Implementation Plan

*Inspired by Hytale's world generation approach, adapted for 2D falling-sand physics*

---

## Current Status (January 2026)

**Phase 1: World Generation Editor - COMPLETE ✅**

The core editor is fully functional:
- Press **F7** to open the World Generation Editor
- All parameter tabs working: World, Terrain, Caves, Biomes, Ores, Vegetation, Features, Presets
- Live 7x7 chunk preview with panning controls
- "Apply to World" regenerates the world with new parameters
- Builtin presets: Default, Cave Heavy, Flat, Desert World, Mountain World

**Files Created:**
- `crates/sunaba-core/src/world/worldgen_config.rs` (~760 lines) - Configuration system
- `crates/sunaba/src/ui/worldgen_editor/mod.rs` - Main editor with tabs
- `crates/sunaba/src/ui/worldgen_editor/noise_widget.rs` - Reusable noise editor
- `crates/sunaba/src/ui/worldgen_editor/preview.rs` - Live preview system

**Files Modified:**
- `crates/sunaba-core/src/world/generation.rs` - `from_config()`, `update_config()`
- `crates/sunaba-core/src/world/persistence_system.rs` - Config access methods
- `crates/sunaba-core/src/world/world.rs` - `update_generator_config()`
- `crates/sunaba/src/ui/ui_state.rs` - WorldGenEditor integration
- `crates/sunaba/src/app.rs` - F7 shortcut + apply handling

**Remaining for Phase 1:**
- Preset save/load to disk (RON files)

**Next Phase:** Phase 2 - Hytale-Inspired Features (context scanner, biome transitions, structures)

---

## Vision

Transform Sunaba's world generation from "functional but boring" to a rich, designer-controlled system where:
- **Curated + Procedural**: Parameters control generation, infinite variety from seeds
- **Context-Aware Placement**: Features relate to surroundings (trees above caves, bridges over gaps)
- **Physics-Stable**: Generated structures respect falling-sand physics
- **Training Integration**: Evolved creatures generalize across varied terrain

## Implementation Phases

### Phase 1: World Generation Editor (Priority: First)

**Goal**: Parameter-based editor with live preview, designed for future node editor expansion.

#### 1.1 Core Data Structures
Create `crates/sunaba-core/src/world/worldgen_config.rs`:

```rust
pub struct WorldGenConfig {
    pub name: String,
    pub world: WorldParams,           // Surface/bedrock Y, layer depths
    pub terrain: TerrainParams,       // Height noise, scale
    pub caves: CaveParams,            // Large/tunnel noise, thresholds
    pub ores: Vec<OreConfig>,         // Per-ore noise, depth ranges
    pub biomes: BiomeParams,          // Temp/moisture noise, biome definitions
    pub vegetation: VegetationParams, // Tree/plant noise
    pub features: FeatureParams,      // Lava pools, structures
}

pub struct NoiseLayerConfig {
    pub seed_offset: i32,
    pub noise_type: NoiseType,        // OpenSimplex2, Perlin, Cellular
    pub frequency: f32,
    pub fractal_type: FractalType,    // FBm, Ridged, PingPong
    pub octaves: u8,
    pub lacunarity: f32,
    pub gain: f32,
}
```

**Files to create:**
- `crates/sunaba-core/src/world/worldgen_config.rs` - All configuration structs
- Serialization: RON format for human-readable presets

#### 1.2 Refactor WorldGenerator
Modify `crates/sunaba-core/src/world/generation.rs`:
- Add `WorldGenerator::from_config(seed, config)` constructor
- Add `update_config(&mut self, config)` for live preview
- Extract hardcoded values into config defaults
- Keep `WorldGenerator::new(seed)` for backward compatibility

#### 1.3 Editor UI
Create `crates/sunaba/src/ui/worldgen_editor/`:

```
worldgen_editor/
├── mod.rs              # WorldGenEditorState, main panel
├── terrain_tab.rs      # Terrain height parameters
├── caves_tab.rs        # Cave generation parameters
├── biomes_tab.rs       # Biome list with per-biome config
├── ores_tab.rs         # Ore depth ranges, thresholds
├── vegetation_tab.rs   # Tree/plant density
├── presets_tab.rs      # Save/load presets
├── noise_widget.rs     # Reusable noise layer editor
└── preview_panel.rs    # Live preview rendering
```

**UI Features:**
- Tab-based parameter editing with sliders/dropdowns
- Reusable `noise_layer_editor()` widget for all noise params
- Live preview (7x7 chunks around origin, throttled 10fps)
- Seed control with randomize button
- Apply to world button

#### 1.4 Preset System
- Save: `WorldGenConfig` → RON file in `~/.sunaba/presets/`
- Load: RON file → `WorldGenConfig`
- Builtin presets: Default, Desert, Cave-Heavy, Mountain, Ocean
- WASM: Use localStorage for web builds

#### 1.5 Keyboard Shortcut
- F7: Toggle World Generation Editor panel

---

### Phase 2: Hytale-Inspired Features (2D Adaptation)

**Goal**: Pattern scanning, biome zones, procedural structures - all physics-aware.

#### 2.1 Context Scanner System
Create `crates/sunaba-core/src/world/context_scanner.rs`:

```rust
pub struct PlacementContext {
    pub ground_distance: Option<i32>,   // Distance to solid below
    pub ceiling_distance: Option<i32>,  // Distance to solid above
    pub air_above: i32,                 // Air blocks above
    pub is_enclosed: bool,              // Cave interior detection
    pub light_level: u8,
    pub biome_type: BiomeType,
}

pub enum PlacementPredicate {
    MinAirAbove(i32),        // "Grass where 10+ air above"
    IsCaveInterior,          // "Mushrooms in caves"
    IsSurface,               // "Trees on surface"
    MinGapWidth(i32),        // "Bridges over 20+ pixel gaps"
    DepthRange { min, max }, // "Gold below -1500"
}
```

**Example Rules:**
- Stalactites: `IsCaveInterior + at ceiling + air below`
- Marker trees: `IsSurface + cave detected below`
- Bridges: `MinGapWidth(20) + ground within 100`
- Glowing mushrooms: `IsCaveInterior + IsSurface + light < 5`

#### 2.2 Biome Transition System
Create `crates/sunaba-core/src/world/biome_transition.rs`:

**Challenge**: Powder materials (sand) can't sit at angles - they fall!

**Solution**: Stability-aware blending
- Blend subsurface stone depth, not surface materials
- Use noise for natural-looking material boundaries
- Prefer solid materials at transition edges
- Apply angle-of-repose constraints for powder piles

```rust
pub enum BlendMode {
    Sharp,                              // No blending
    Gradient { noise_scale: f32 },      // Noise-based transition
    StableLayer,                        // Heavier materials below
}
```

#### 2.3 Biome Zones
Create `crates/sunaba-core/src/world/biome_zones.rs`:

```rust
pub struct BiomeZone {
    pub biomes: Vec<BiomeType>,         // Allowed biomes
    pub depth_range: (i32, i32),        // Y-range for zone
    pub transitions: Vec<TransitionRule>,
}
```

**Zones:**
- Surface (y > -500): Desert, Plains, Forest, Mountains, Ocean
- Underground (-500 to -1500): Crystal Caves, Mushroom Grotto
- Deep (-1500 to -2500): Lava Caverns, Ore-rich zones

#### 2.4 Structure System
Create `crates/sunaba-core/src/world/structures.rs`:

```rust
pub struct StructureTemplate {
    pub pixels: Vec<(u8, u8, u16)>,     // Sparse pixel data
    pub anchor: AnchorPoint,             // BottomCenter, TopCenter, BridgeEnds
    pub placement: PlacementRule,
    pub requires_support: bool,          // Auto-add support columns?
}
```

**Anchor Types:**
- `BottomCenter`: Trees, towers - trace down to bedrock/stone
- `TopCenter`: Stalactites - attached to ceiling
- `BridgeEnds`: Support at both horizontal ends

**Example Structures:**
- Stalactites (procedural cone shape)
- Wooden bridges (deck + end supports)
- Tree variants (normal vs marker trees above caves)
- Underground ruins (stone brick walls)

#### 2.5 Material Providers
Create `crates/sunaba-core/src/world/material_provider.rs`:

```rust
pub enum MaterialProvider {
    Constant(u16),
    BiomeBased { default, overrides },
    DepthBased { layers: Vec<(i32, u16)> },
    ContextBased { rules, default },
    PhysicsStable { preferred, fallback }, // Use fallback if preferred unstable
}
```

---

### Phase 3: Creature Training Integration

**Goal**: Creatures evolve robust behaviors across varied terrain.

#### 3.1 Training Terrain Config
Create `crates/sunaba/src/headless/terrain_gen.rs`:

```rust
pub struct TrainingTerrainConfig {
    pub base_seed: u64,
    pub width: i32,
    pub height: i32,
    pub biome: Option<BiomeType>,
    pub difficulty: DifficultyConfig,
}

pub struct DifficultyConfig {
    pub terrain_roughness: f32,    // 0.0 = flat, 1.0 = max variance
    pub obstacle_density: f32,
    pub hazard_density: f32,
    pub cave_density: f32,
    pub gap_frequency: f32,
}
```

#### 3.2 Environment Distribution
Create `crates/sunaba/src/headless/env_distribution.rs`:

```rust
pub struct EnvironmentDistribution {
    pub biome_weights: HashMap<BiomeType, f32>,
    pub difficulty_range: (DifficultyConfig, DifficultyConfig),
}

impl EnvironmentDistribution {
    pub fn sample(&self, eval_id: u64) -> TrainingTerrainConfig;
    pub fn sample_batch(&self, eval_id: u64, count: usize) -> Vec<TrainingTerrainConfig>;
}
```

**Key insight**: Seeded sampling ensures reproducibility - same creature sees same terrain each evaluation.

#### 3.3 Curriculum Learning
Create `crates/sunaba/src/headless/curriculum.rs`:

```rust
pub struct CurriculumConfig {
    pub stages: Vec<CurriculumStage>,
}

pub struct CurriculumStage {
    pub name: String,
    pub env_distribution: EnvironmentDistribution,
    pub min_generations: usize,
    pub target_fitness: f32,
    pub target_coverage: f32,
}
```

**Standard Curriculum:**
1. Flat Ground (gens 1-20): Basic locomotion
2. Gentle Hills (gens 21-50): Slope handling
3. Obstacles (gens 51-90): Navigation
4. Hazards (gens 91-140): Survival
5. Full Random (gens 141+): Generalization

#### 3.4 Biome Specialists
Create `crates/sunaba/src/headless/biome_scenarios.rs`:

Train separate populations per biome:
- **Desert**: Sand dunes, sparse food, lava hazards
- **Cave**: Narrow passages, limited light, ore deposits
- **Forest**: Dense vegetation, abundant food, water
- **Mountain**: Steep terrain, gaps, high platforms

#### 3.5 Multi-Environment Evaluation
Create `crates/sunaba/src/headless/multi_env_eval.rs`:

```rust
pub struct MultiEnvironmentEvaluator {
    pub num_environments: usize,        // Environments per evaluation
    pub aggregation: FitnessAggregation,
}

pub enum FitnessAggregation {
    Mean,                               // Average performance
    Min,                                // Worst-case (robust)
    Percentile(f32),                    // e.g., 25th percentile
}
```

#### 3.6 Fast Training World
Create `crates/sunaba/src/headless/training_world.rs`:

Optimized world for training (no temperature/light simulation):
- Single-pass noise-based generation
- Pre-cached food/hazard positions
- LRU cache for repeated terrain configs
- Partial generation (active radius only)

#### 3.7 Enhanced Sensors
Extend `crates/sunaba-creature/src/sensors.rs`:

```rust
pub struct TerrainSensoryInput {
    pub ground_slope: f32,              // -1 to 1 (downhill to uphill)
    pub vertical_clearance: f32,        // Air above for jumping
    pub gap_distance: f32,              // Distance to next gap
    pub gap_width: f32,                 // Width of gap
    pub surface_material: u16,          // Material underfoot
}
```

---

## File Summary

### New Files

| File | Purpose |
|------|---------|
| `crates/sunaba-core/src/world/worldgen_config.rs` | Configuration data structures |
| `crates/sunaba-core/src/world/context_scanner.rs` | Context queries for placement |
| `crates/sunaba-core/src/world/biome_transition.rs` | Physics-stable biome blending |
| `crates/sunaba-core/src/world/biome_zones.rs` | Depth-based zone system |
| `crates/sunaba-core/src/world/structures.rs` | Structure templates + placer |
| `crates/sunaba-core/src/world/material_provider.rs` | Context-based material selection |
| `crates/sunaba/src/ui/worldgen_editor/*.rs` | Editor UI (8 files) |
| `crates/sunaba/src/headless/terrain_gen.rs` | Training terrain generation |
| `crates/sunaba/src/headless/env_distribution.rs` | Environment sampling |
| `crates/sunaba/src/headless/curriculum.rs` | Curriculum learning |
| `crates/sunaba/src/headless/biome_scenarios.rs` | Biome-specific scenarios |
| `crates/sunaba/src/headless/training_world.rs` | Optimized training world |
| `crates/sunaba/src/headless/multi_env_eval.rs` | Multi-environment evaluation |

### Modified Files

| File | Changes |
|------|---------|
| `crates/sunaba-core/src/world/generation.rs` | Config-driven, `from_config()` |
| `crates/sunaba-core/src/world/biome.rs` | Extend with transition rules |
| `crates/sunaba-core/src/world/mod.rs` | Export new modules |
| `crates/sunaba/src/ui/mod.rs` | Add worldgen_editor module |
| `crates/sunaba/src/ui/dock.rs` | Add WorldGenEditor tab |
| `crates/sunaba/src/app.rs` | F7 shortcut, editor integration |
| `crates/sunaba/src/headless/training_env.rs` | Environment distribution |
| `crates/sunaba-creature/src/sensors.rs` | Terrain-aware sensors |

---

## Implementation Order

### Sprint 1: Editor Foundation - COMPLETE ✅
1. [x] `WorldGenConfig` + nested structs with defaults
2. [x] RON serialization round-trip tests
3. [x] Refactor `WorldGenerator` to use config
4. [x] Basic editor UI with terrain tab
5. [x] Live preview (throttled chunk generation)

### Sprint 2: Editor Completion - MOSTLY COMPLETE ✅
1. [x] All parameter tabs (caves, biomes, ores, vegetation)
2. [x] Noise layer widget component
3. [ ] Preset save/load system (builtin presets work, file save/load pending)
4. [x] Apply to world functionality
5. [x] F7 keyboard shortcut

### Sprint 3: Context-Aware Generation (5-6 days)
1. [ ] `ContextScanner` + `PlacementContext`
2. [ ] `PlacementPredicate` evaluation
3. [ ] Stalactite generation (proof of concept)
4. [ ] Biome transition system (stability-aware)
5. [ ] Structure template system

### Sprint 4: Training Integration (5-6 days)
1. [ ] `TrainingTerrainConfig` + `TrainingWorld`
2. [ ] `EnvironmentDistribution` with seeded sampling
3. [ ] Multi-environment evaluation
4. [ ] Curriculum learning system
5. [ ] Extended terrain sensors

### Sprint 5: Polish & Biome Content (4-5 days)
1. [ ] Biome zone system (underground biomes)
2. [ ] Biome specialist training mode
3. [ ] More structures (bridges, ruins)
4. [ ] Builtin presets (5+ varied configs)
5. [ ] Training report enhancements

---

## Future: Visual Node Editor

The architecture supports eventual node editor expansion:

```rust
pub trait GenNode {
    fn inputs(&self) -> &[NodePort];
    fn outputs(&self) -> &[NodePort];
    fn evaluate(&self, x: i32, y: i32, inputs: &[f64]) -> Vec<f64>;
}
```

**Migration path:**
- `NoiseLayerConfig` → `NoiseNode`
- `threshold` params → `ThresholdNode`
- `BiomeParams` → `BiomeSelectionNode`
- `OreConfig` → `DepthMaskNode` + `ThresholdNode`

Parameter-based config can serialize to/from node graph for bi-directional editing.

---

## Success Criteria

1. **Editor**: Can create visually distinct worlds by adjusting parameters
2. **Preview**: Changes reflect in < 200ms
3. **Presets**: Save/load works on desktop and WASM
4. **Structures**: Stalactites, bridges generate in appropriate locations
5. **Stability**: No physics artifacts at biome transitions
6. **Training**: Creatures trained on varied terrain generalize to unseen terrain
7. **Performance**: Training world generation < 50ms per scenario
