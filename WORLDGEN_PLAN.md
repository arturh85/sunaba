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
- **Custom preset save/load**: Save to `worlds/presets/` (native) or localStorage (WASM)

**Files Created:**
- `crates/sunaba-core/src/world/worldgen_config.rs` (~760 lines) - Configuration system
- `crates/sunaba/src/ui/worldgen_editor/mod.rs` (~880 lines) - Main editor with tabs + preset persistence
- `crates/sunaba/src/ui/worldgen_editor/noise_widget.rs` - Reusable noise editor
- `crates/sunaba/src/ui/worldgen_editor/preview.rs` - Live preview system

**Files Modified:**
- `crates/sunaba-core/src/world/generation.rs` - `from_config()`, `update_config()`
- `crates/sunaba-core/src/world/persistence_system.rs` - Config access methods
- `crates/sunaba-core/src/world/world.rs` - `update_generator_config()`
- `crates/sunaba/src/ui/ui_state.rs` - WorldGenEditor integration
- `crates/sunaba/src/app.rs` - F7 shortcut + apply handling

**Phase 2: Context-Aware Generation - COMPLETE ✅**

Context scanner system, stalactite generation, biome transitions, and structure template system:
- `ContextScanner` - Queries placement context from WorldGenerator
- `PlacementContext` - Ground/ceiling distance, air above/below, enclosure detection, biome, light
- `PlacementPredicate` - Composable rules (IsCaveInterior, IsSurface, MinAirAbove, DepthRange, etc.)
- Builder methods for common patterns: `stalactite()`, `stalagmite()`, `surface_tree()`, `cave_mushroom()`
- **Stalactite generation** - Proof-of-concept feature using context scanner, fully configurable via F7 editor
- **Biome transition system** - Physics-stable blending between biomes with 3 modes (Sharp, Gradient, StableLayer)
- **Structure template system** - Flexible template-based placement for bridges, trees, and ruins
- **8 structure variants** - 3 bridge types, 3 tree types, 2 ruin types

**Files Created:**
- `crates/sunaba-core/src/world/context_scanner.rs` (~750 lines) - Context scanning system
- `crates/sunaba-core/src/world/features.rs` (~370 lines) - Feature placement (stalactites, bridges, trees, ruins)
- `crates/sunaba-core/src/world/biome_transition.rs` (~420 lines) - Physics-aware biome blending system
- `crates/sunaba-core/src/world/structures.rs` (~220 lines) - Core structure types (StructureTemplate, AnchorType, StructureVariants)
- `crates/sunaba-core/src/world/structure_templates.rs` (~330 lines) - TemplateBuilder API + all builtin templates
- `crates/sunaba-core/src/world/structure_placement.rs` (~310 lines) - Placement engine + physics validation

**Files Modified:**
- `crates/sunaba-core/src/world/generation.rs` - Biome transition integration, `get_material_with_transition()`
- `crates/sunaba-core/src/world/mod.rs` - Export structure modules and config types
- `crates/sunaba-core/src/world/worldgen_config.rs` - Added StructureConfig, BridgeConfig, TreeConfig, RuinConfig
- `crates/sunaba-core/src/world/context_scanner.rs` - Made `get_material()` public for structure placement
- `crates/sunaba/src/ui/worldgen_editor/mod.rs` - Added UI controls for structures (3 collapsible sections)

**Phase 3: Underground Biome Zones - COMPLETE ✅**

Depth-based themed underground zones with unique materials:
- **5 Underground Zones**: Shallow Caves, Mushroom Grotto, Crystal Caves, Lava Caverns, Abyss
- **5 New Materials**: Mossy Stone, Crystal, Basalt, Glowing Mushroom, Obsidian
- **BiomeZoneRegistry** - Depth-based zone selection with configurable enabled/surface_influence flags
- **Editor Integration** - Underground Zones section in F7 Features tab

**Files Created:**
- `crates/sunaba-core/src/world/biome_zones.rs` (~500 lines) - Zone system with transitions
- `crates/sunaba-simulation/src/materials.rs` (+70 lines) - 5 new underground materials

**Files Modified:**
- `crates/sunaba-core/src/world/generation.rs` - Zone-based stone material selection
- `crates/sunaba-core/src/world/worldgen_config.rs` - UndergroundZonesConfig struct
- `crates/sunaba/src/ui/worldgen_editor/mod.rs` - Underground Zones UI section

**Phase 4: Training Integration (Sprint 4) - IN PROGRESS 🚧**

**Sprint 4, Week 1: Core Infrastructure - COMPLETE ✅ (January 2026)**

Successfully integrated procedural world generation into creature training system:
- **Training Terrain Configuration** - `TrainingTerrainConfig` with 5 difficulty presets (flat → random)
- **Scenario Integration** - Optional procedural terrain in `Scenario::setup_world()`
- **100% Backward Compatible** - All existing scenarios work unchanged (terrain_config = None)
- **Deterministic** - Same seed + config = identical terrain for reproducible training
- **Test Coverage** - 7 new integration tests, all 68 tests passing

**Files Created:**
- `crates/sunaba/src/headless/terrain_config.rs` (~330 lines) - Config system with DifficultyConfig

**Files Modified:**
- `crates/sunaba/src/headless/scenario.rs` (+80 lines) - Procedural world generation support
- `crates/sunaba/src/headless/mod.rs` (+2 lines) - Module exports

**Sprint 4, Week 2: Multi-Environment Evaluation - COMPLETE ✅ (January 2026)**

Implemented multi-environment evaluation system for creature generalization:
- **Environment Distribution** - Deterministic seeded sampling with 3 strategies (Uniform, Discrete, Presets)
- **Multi-Environment Evaluator** - Evaluate creatures on N terrains with 5 aggregation methods
- **TrainingEnv Integration** - Optional multi-env via `TrainingConfig.multi_env`
- **100% Backward Compatible** - None = single environment (existing behavior)
- **Reproducibility** - Same creature sees same terrains each evaluation (deterministic eval_id)
- **Test Coverage** - 18 unit tests passing, all core functionality verified

**Files Created:**
- `crates/sunaba/src/headless/env_distribution.rs` (~346 lines) - Environment sampling system
- `crates/sunaba/src/headless/multi_env_eval.rs` (~315 lines) - Multi-env evaluator + aggregation

**Files Modified:**
- `crates/sunaba/src/headless/training_env.rs` (+140 lines) - Multi-env integration, refactored evaluate_single
- `crates/sunaba/src/headless/scenario.rs` (+5 lines) - Added setup_world_with_terrain() method
- `crates/sunaba/src/headless/mod.rs` (+2 lines) - Module exports

**Sprint 4, Week 3: Curriculum Learning System - COMPLETE ✅ (January 2026)**

Implemented curriculum learning for progressive difficulty training:
- **CurriculumConfig** - Multi-stage training with configurable advancement criteria
- **Advancement Strategies** - Automatic, fitness-based, coverage-based, and combined criteria
- **Standard Curriculum** - 5-stage preset (Flat → Hills → Obstacles → Hazards → Random)
- **Multiple Presets** - Standard, Fast (3 stages), Aggressive (fitness-gated)
- **TrainingEnv Integration** - Automatic stage progression during training with logging
- **Multi-Env Sync** - Updates environment distribution when advancing stages
- **100% Backward Compatible** - None = no curriculum (existing behavior)
- **Test Coverage** - 17 unit tests, comprehensive advancement logic validation

**Files Created:**
- `crates/sunaba/src/headless/curriculum.rs` (~580 lines) - Full curriculum system with tests

**Files Modified:**
- `crates/sunaba/src/headless/training_env.rs` (+90 lines) - Curriculum integration, advancement checking
- `crates/sunaba/src/headless/mod.rs` (+1 line) - Module exports

**Sprint 4, Week 4: Extended Terrain Sensors - COMPLETE ✅ (January 2026)**

Implemented terrain-aware sensory inputs for adaptive locomotion:
- **5 Terrain Sensors** - ground_slope, vertical_clearance, gap_distance, gap_width, surface_material
- **Per-Body-Part Sensing** - Enables limb-specific terrain awareness for sophisticated gaits
- **Neural Integration** - BodyPartFeatures extended from 22 to 27 dimensions per part
- **100% Backward Compatible** - Existing creatures automatically get larger input layer via rebuild_brain()
- **Performance** - ~88 queries per body part (comparable to existing 8×50px raycasts = 400 queries)
- **Test Coverage** - 14 integration tests, all passing, determinism verified

**Files Created:**
- `crates/sunaba-core/tests/creature_terrain_sensors_test.rs` (~430 lines) - Comprehensive integration tests

**Files Modified:**
- `crates/sunaba-creature/src/sensors.rs` (+150 lines) - TerrainSensoryInput struct, 5 sensor functions, SensorConfig extension
- `crates/sunaba-creature/src/neural.rs` (+70 lines) - BodyPartFeatures extended, feature_dim updated, terrain sensing in extract_body_part_features_simple
- `crates/sunaba-creature/src/creature.rs` (~5 lines) - Pass sensor_config to extract_body_part_features_simple
- `crates/sunaba-core/tests/creature_integration_tests.rs` (~10 lines) - Updated SensorConfig usage

**Expected Outcomes:**
After training, creatures will learn:
- **Slope-adaptive gaits** - Lean forward uphill, brake downhill
- **Jump height modulation** - Duck under ceilings, optimize jump power
- **Gap navigation** - Measure gaps, decide if jumpable
- **Surface-specific movement** - Different gaits on sand vs stone

**Next:** Phase 5 - Biome Specialists & Training Report Enhancements

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

| File                                                  | Purpose                                                                 | Status     |
|-------------------------------------------------------|-------------------------------------------------------------------------|------------|
| `crates/sunaba-core/src/world/worldgen_config.rs`     | Configuration data structures                                           | ✅ Complete |
| `crates/sunaba-core/src/world/context_scanner.rs`     | Context queries for placement                                           | ✅ Complete |
| `crates/sunaba-core/src/world/features.rs`            | Post-generation features (stalactites, etc.)                            | ✅ Complete |
| `crates/sunaba/src/ui/worldgen_editor/*.rs`           | Editor UI (8 files)                                                     | ✅ Complete |
| `crates/sunaba-core/src/world/biome_transition.rs`    | Physics-stable biome blending                                           | ✅ Complete |
| `crates/sunaba-core/src/world/structures.rs`          | Core structure types (StructureTemplate, AnchorType, StructureVariants) | ✅ Complete |
| `crates/sunaba-core/src/world/structure_templates.rs` | TemplateBuilder API + builtin templates                                 | ✅ Complete |
| `crates/sunaba-core/src/world/structure_placement.rs` | Placement engine + physics validation                                   | ✅ Complete |
| `crates/sunaba-core/src/world/biome_zones.rs`         | Depth-based zone system                                                 | ✅ Complete |
| `crates/sunaba-core/src/world/material_provider.rs`   | Context-based material selection                                        | Planned    |
| `crates/sunaba/src/headless/terrain_config.rs`        | Training terrain generation (config types + difficulty presets)         | ✅ Complete |
| `crates/sunaba/src/headless/env_distribution.rs`      | Environment sampling (deterministic seeded sampling)                    | ✅ Complete |
| `crates/sunaba/src/headless/multi_env_eval.rs`        | Multi-environment evaluation (aggregation strategies)                   | ✅ Complete |
| `crates/sunaba/src/headless/curriculum.rs`            | Curriculum learning (progressive difficulty stages)                     | ✅ Complete |
| `crates/sunaba/src/headless/biome_scenarios.rs`       | Biome-specific scenarios                                                | Planned    |
| `crates/sunaba/src/headless/training_world.rs`        | Optimized training world                                                | Planned    |

### Modified Files

| File                                              | Changes                                                          | Status     |
|---------------------------------------------------|------------------------------------------------------------------|------------|
| `crates/sunaba-core/src/world/generation.rs`      | Config-driven, `from_config()`, `apply_features()` call          | ✅ Complete |
| `crates/sunaba-core/src/world/mod.rs`             | Export new modules (features, context_scanner, StalactiteConfig) | ✅ Complete |
| `crates/sunaba-core/src/world/worldgen_config.rs` | Added StalactiteConfig struct                                    | ✅ Complete |
| `crates/sunaba/src/ui/mod.rs`                     | Add worldgen_editor module                                       | ✅ Complete |
| `crates/sunaba/src/ui/dock.rs`                    | Add WorldGenEditor tab                                           | ✅ Complete |
| `crates/sunaba/src/app.rs`                        | F7 shortcut, editor integration                                  | ✅ Complete |
| `crates/sunaba/src/headless/training_env.rs`      | Multi-env + curriculum integration (~230 lines added)            | ✅ Complete |
| `crates/sunaba/src/headless/scenario.rs`          | Added setup_world_with_terrain() method                          | ✅ Complete |
| `crates/sunaba/src/headless/mod.rs`               | Export curriculum, multi_env_eval, env_distribution              | ✅ Complete |
| `crates/sunaba-core/src/world/biome.rs`           | Extend with transition rules                                     | Planned    |
| `crates/sunaba-creature/src/sensors.rs`           | Terrain-aware sensors                                            | Planned    |

---

## Implementation Order

### Sprint 1: Editor Foundation - COMPLETE ✅
1. [x] `WorldGenConfig` + nested structs with defaults
2. [x] RON serialization round-trip tests
3. [x] Refactor `WorldGenerator` to use config
4. [x] Basic editor UI with terrain tab
5. [x] Live preview (throttled chunk generation)

### Sprint 2: Editor Completion - COMPLETE ✅
1. [x] All parameter tabs (caves, biomes, ores, vegetation)
2. [x] Noise layer widget component
3. [x] Preset save/load system (builtin presets + custom presets to disk/localStorage)
4. [x] Apply to world functionality
5. [x] F7 keyboard shortcut

### Sprint 3: Context-Aware Generation - COMPLETE ✅
1. [x] `ContextScanner` + `PlacementContext`
2. [x] `PlacementPredicate` evaluation
3. [x] Stalactite generation (proof of concept)
4. [x] Biome transition system (stability-aware)
5. [x] Structure template system (bridges, trees, ruins)

### Sprint 4: Training Integration - COMPLETE ✅ (January 2026)
1. [x] `TrainingTerrainConfig` + `TrainingWorld`
2. [x] `EnvironmentDistribution` with seeded sampling
3. [x] Multi-environment evaluation
4. [x] Curriculum learning system
5. [x] Extended terrain sensors

### Sprint 5: Biome Specialist Training & Enhanced Reports - COMPLETE ✅ (January 2026)

Successfully implemented biome-specialized evolution and comprehensive training analytics:
- **Biome Specialist Training** - Per-biome MAP-Elites grids with biome-specific terrain evaluation
- **Multi-Environment Statistics** - Per-environment-type performance tracking and fitness distributions
- **Curriculum Stage Tracking** - Timeline of stage transitions with advancement reasons
- **Behavior Diversity Metrics** - Shannon entropy, unique niches, density variance
- **Enhanced Data Export** - Extended JSON schema + CSV export for external analysis
- **2 Biome Curriculum Presets** - `biome_progression()` and `biome_quick()` for specialized training

**Files Created:**
- None (extended existing files)

**Files Modified:**
- `crates/sunaba/src/headless/training_env.rs` (+290 lines) - BiomeSpecialistConfig, biome grids, enhanced stats tracking
- `crates/sunaba/src/headless/terrain_config.rs` (+20 lines) - BiomeType field, classify_type() helper
- `crates/sunaba/src/headless/env_distribution.rs` (+40 lines) - sample_for_biome() methods
- `crates/sunaba/src/headless/multi_env_eval.rs` (+30 lines) - sample_terrains_for_biome()
- `crates/sunaba/src/headless/map_elites.rs` (+115 lines) - Behavior diversity calculations (entropy, heatmap, variance)
- `crates/sunaba/src/headless/curriculum.rs` (+108 lines) - Biome curriculum presets
- `crates/sunaba/src/headless/report.rs` (+327 lines) - Extended JSON schema, CSV export

**Test Coverage:**
- 9 new integration tests for biome training
- 7 new unit tests for behavior diversity
- All 118 tests passing ✅

**Key Features:**
1. **Biome Specialist Training**: Train separate populations per biome (Desert, Plains, Forest, Mountains, Ocean)
   ```rust
   let config = TrainingConfig::default()
       .with_biome_specialists(vec![BiomeType::Desert, BiomeType::Mountains]);
   ```
2. **Multi-Environment Stats**: Track mean/best/worst fitness per environment type (flat, hills, obstacles, hazards)
3. **Curriculum Tracking**: Record stage transitions with fitness/coverage at advancement
4. **Behavior Diversity**: Calculate entropy and density variance of MAP-Elites grids
5. **CSV Export**: All training data exportable to `training_data.csv` with dynamic columns
6. **Backward Compatible**: All enhanced stats are optional, existing training unchanged

**Deliverables:**
1. [x] Biome zone system (underground biomes) - **COMPLETE** (Phase 3)
2. [x] Biome specialist training mode - **COMPLETE**
3. [x] More structures (bridges, ruins) - **COMPLETE** (Phase 2)
4. [x] Builtin presets (5+ varied configs) - **COMPLETE** (Phase 1)
5. [x] Training report enhancements - **COMPLETE** (JSON schema + CSV export)

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
