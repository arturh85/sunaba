//! World generation configuration - serializable parameters for world generation
//!
//! This module provides data structures that capture all world generation parameters,
//! enabling:
//! - Parameter-based editor UI with live preview
//! - Serialization to RON format for presets
//! - Configuration-driven world generation
//! - Future expansion to visual node editor

use crate::simulation::MaterialId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete world generation configuration
///
/// All parameters needed to generate a world. Serializable to RON for presets.
/// The seed is NOT part of the config - same config + different seed = different world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldGenConfig {
    /// Display name for this configuration
    pub name: String,

    /// Global world parameters (boundaries, layers)
    pub world: WorldParams,

    /// Terrain height generation
    pub terrain: TerrainParams,

    /// Cave system generation
    pub caves: CaveParams,

    /// Ore generation (per-ore settings)
    pub ores: Vec<OreConfig>,

    /// Biome selection and definitions
    pub biomes: BiomeParams,

    /// Vegetation placement
    pub vegetation: VegetationParams,

    /// Special features (lava pools, etc.)
    pub features: FeatureParams,

    /// Underground biome zones (depth-based themed caves)
    pub underground_zones: UndergroundZonesConfig,
}

/// Global world boundaries and layer depths
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldParams {
    /// Sea level baseline (default: 0)
    pub surface_y: i32,
    /// Top of atmosphere (default: 1000)
    pub sky_height: i32,
    /// Bedrock layer starts here (default: -3500)
    pub bedrock_y: i32,
    /// Underground layer boundaries
    pub underground_layers: UndergroundLayers,
}

/// Underground layer depth boundaries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndergroundLayers {
    /// Shallow underground: common ores, shallow caves (default: -500)
    pub shallow: i32,
    /// Deep underground: better ores, larger caves (default: -1500)
    pub deep: i32,
    /// Cavern layer: rare ores, huge caverns, lava (default: -2500)
    pub cavern: i32,
}

/// Terrain height generation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainParams {
    /// Noise layer for terrain height variation
    pub height_noise: NoiseLayerConfig,
    /// Height multiplier (affects hill amplitude, default: 100.0)
    pub height_scale: f32,
}

/// Cave system generation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaveParams {
    /// Large cavern noise layer
    pub large_caves: NoiseLayerConfig,
    /// Tunnel network noise layer
    pub tunnels: NoiseLayerConfig,
    /// Threshold for large caves (lower = more caves, default: 0.15)
    pub large_threshold: f32,
    /// Threshold for tunnels (lower = more tunnels, default: 0.25)
    pub tunnel_threshold: f32,
    /// Minimum depth below surface for caves to appear (default: 10)
    pub min_cave_depth: i32,
}

/// Individual ore generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OreConfig {
    /// Material ID for this ore
    pub material_id: u16,
    /// Display name
    pub name: String,
    /// Noise layer for ore placement
    pub noise: NoiseLayerConfig,
    /// Threshold for ore generation (higher = rarer, default varies by ore)
    pub threshold: f32,
    /// Shallowest depth where this ore appears
    pub min_depth: i32,
    /// Deepest depth where this ore appears
    pub max_depth: i32,
    /// Noise scale for ore generation (default: 0.08)
    pub noise_scale: f32,
}

/// Biome selection and definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeParams {
    /// Temperature noise for biome selection
    pub temperature_noise: NoiseLayerConfig,
    /// Moisture noise for biome selection
    pub moisture_noise: NoiseLayerConfig,
    /// Individual biome definitions
    pub biomes: Vec<BiomeConfig>,
    /// Biome transition configuration
    pub transition: BiomeTransitionConfig,
}

/// Single biome configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeConfig {
    /// Biome name
    pub name: String,
    /// Biome type identifier
    pub biome_type: String,

    // Selection thresholds (temperature/moisture ranges)
    /// Minimum temperature for this biome
    pub temp_min: f32,
    /// Maximum temperature for this biome
    pub temp_max: f32,
    /// Minimum moisture for this biome
    pub moisture_min: f32,
    /// Maximum moisture for this biome
    pub moisture_max: f32,
    /// Selection priority (higher = checked first)
    pub priority: i32,

    // Surface generation
    /// Top layer material
    pub surface_material: u16,
    /// Below surface material
    pub subsurface_material: u16,
    /// Depth before reaching stone
    pub stone_depth: i32,

    // Terrain modifiers
    /// Hilliness multiplier (0.0-2.0)
    pub height_variance: f32,
    /// Base elevation offset from sea level
    pub height_offset: i32,

    // Vegetation
    /// Tree spawn probability (0.0-1.0)
    pub tree_density: f32,
    /// Plant spawn probability (0.0-1.0)
    pub plant_density: f32,

    // Underground
    /// Cave frequency multiplier (1.0 = normal)
    pub cave_density_multiplier: f32,

    // Ore multipliers (material_id -> multiplier)
    /// Per-ore abundance multipliers
    pub ore_multipliers: HashMap<u16, f32>,
}

/// Biome transition configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeTransitionConfig {
    /// Enable biome transitions (false = sharp boundaries)
    pub enabled: bool,
    /// Transition width in pixels (default: 32)
    pub width: i32,
    /// Blend mode (0=Sharp, 1=Gradient, 2=StableLayer)
    pub mode: BiomeBlendModeConfig,
    /// Noise scale for gradient mode (0-100, controls boundary roughness)
    pub noise_scale_percent: u32,
    /// Enforce physics stability (prevent powder collapse)
    pub enforce_stability: bool,
}

/// Blend mode configuration (serializable version)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BiomeBlendModeConfig {
    Sharp,
    Gradient,
    StableLayer,
}

/// Vegetation placement parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VegetationParams {
    /// Tree placement noise
    pub tree_noise: NoiseLayerConfig,
    /// Plant placement noise
    pub plant_noise: NoiseLayerConfig,
    /// Tree noise frequency multiplier (default: 0.03)
    pub tree_noise_scale: f32,
    /// Plant noise frequency multiplier (default: 0.05)
    pub plant_noise_scale: f32,
}

/// Wire network generation for Circuit Ruins zone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireNetworkConfig {
    /// Enable wire network generation
    pub enabled: bool,
    /// Minimum depth for wire networks (Circuit Ruins zone)
    pub min_depth: i32,
    /// Maximum depth for wire networks
    pub max_depth: i32,
    /// Horizontal wire spacing (default: 200)
    pub h_spacing: i32,
    /// Vertical wire spacing (default: 300)
    pub v_spacing: i32,
    /// Probability of battery at intersections (0.0-1.0)
    pub battery_chance: f32,
    /// Probability of spark near batteries (0.0-1.0)
    pub spark_chance: f32,
    /// Noise threshold for horizontal wires
    pub h_wire_threshold: f32,
    /// Noise threshold for vertical wires
    pub v_wire_threshold: f32,
    /// Noise seed offset
    pub seed_offset: i32,
}

/// Thunder zone generation for Thunder Caverns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThunderZoneConfig {
    /// Enable thunder zone generation
    pub enabled: bool,
    /// Minimum depth for thunder zones
    pub min_depth: i32,
    /// Maximum depth for thunder zones
    pub max_depth: i32,
    /// Noise threshold for thunder clusters
    pub thunder_threshold: f32,
    /// Probability of spark in cave air
    pub spark_chance: f32,
    /// Wire lightning rod chance below thunder
    pub wire_rod_chance: f32,
    /// Noise seed offset
    pub seed_offset: i32,
}

/// Volatile pool generation for Volatile Lakes zone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolatilePoolConfig {
    /// Enable volatile pool generation
    pub enabled: bool,
    /// Minimum depth for volatile pools
    pub min_depth: i32,
    /// Maximum depth for volatile pools
    pub max_depth: i32,
    /// Noise threshold for pool placement
    pub pool_threshold: f32,
    /// Probability of nitro vs oil vs water (nitro weight)
    pub nitro_weight: f32,
    /// Oil weight
    pub oil_weight: f32,
    /// Probability of gunpowder near pool edges
    pub gunpowder_chance: f32,
    /// Probability of lava vein near pools (danger!)
    pub lava_vein_threshold: f32,
    /// Noise seed offset
    pub seed_offset: i32,
}

/// Toxic vent generation for Toxic Depths zone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToxicVentConfig {
    /// Enable toxic vent generation
    pub enabled: bool,
    /// Minimum depth for toxic vents
    pub min_depth: i32,
    /// Maximum depth for toxic vents
    pub max_depth: i32,
    /// Noise threshold for poison gas vents
    pub vent_threshold: f32,
    /// Maximum height of gas columns
    pub max_gas_height: i32,
    /// Noise threshold for virus patches
    pub virus_threshold: f32,
    /// Maximum virus patch radius
    pub max_virus_radius: i32,
    /// Noise threshold for mercury pools
    pub mercury_threshold: f32,
    /// Noise seed offset
    pub seed_offset: i32,
}

/// Special feature generation parameters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FeatureParams {
    /// Lava pool generation
    pub lava_pools: LavaPoolConfig,
    /// Stalactite generation
    pub stalactites: StalactiteConfig,
    /// Structure generation (bridges, trees, ruins)
    pub structures: StructureConfig,
    /// Wire network generation for Circuit Ruins zone
    pub wire_networks: WireNetworkConfig,
    /// Thunder zone generation for Thunder Caverns
    pub thunder_zones: ThunderZoneConfig,
    /// Volatile pool generation for Volatile Lakes zone
    pub volatile_pools: VolatilePoolConfig,
    /// Toxic vent generation for Toxic Depths zone
    pub toxic_vents: ToxicVentConfig,
}

/// Lava pool generation in deep caverns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LavaPoolConfig {
    /// Enable lava pool generation
    pub enabled: bool,
    /// Minimum depth for lava pools (default: -2500)
    pub min_depth: i32,
    /// Threshold for lava generation (higher = less lava, default: 0.6)
    pub threshold: f32,
    /// Noise scale for lava distribution (default: 0.05)
    pub noise_scale: f32,
}

/// Stalactite generation in caves
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StalactiteConfig {
    /// Enable stalactite generation
    pub enabled: bool,
    /// Minimum depth below surface for stalactites (default: -50)
    /// Prevents stalactites near surface caves
    pub min_depth: i32,
    /// Spacing between stalactites (grid spacing in pixels, default: 16)
    /// Higher = sparser stalactites
    pub spacing: i32,
    /// Minimum length of stalactites in pixels (default: 3)
    pub min_length: i32,
    /// Maximum length of stalactites in pixels (default: 12)
    pub max_length: i32,
    /// Base width at ceiling (default: 3)
    pub base_width: i32,
    /// Minimum air space required below ceiling (default: 5)
    pub min_air_below: i32,
    /// Noise seed offset for stalactite placement variation
    pub seed_offset: i32,
    /// Placement probability (0.0-1.0, default: 0.5)
    /// After finding valid position, this determines if stalactite is placed
    pub placement_chance: f32,
    /// Whether stalactites should taper to a point (default: true)
    pub taper: bool,
}

/// Structure generation configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructureConfig {
    /// Wooden bridges over gaps
    pub bridges: BridgeConfig,
    /// Surface trees (normal and marker variants)
    pub trees: TreeConfig,
    /// Underground ruins
    pub ruins: RuinConfig,
}

/// Wooden bridge generation over gaps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    /// Enable bridge generation
    pub enabled: bool,
    /// Grid sampling spacing (default: 32)
    pub spacing: i32,
    /// Minimum gap width to place bridge (default: 16)
    pub min_gap_width: i32,
    /// Maximum gap width to bridge (default: 48)
    pub max_gap_width: i32,
    /// Placement probability (0.0-1.0, default: 0.3)
    pub placement_chance: f32,
    /// Minimum depth for bridge placement (default: -100)
    pub min_depth: i32,
    /// Noise seed offset
    pub seed_offset: i32,
}

/// Tree generation (normal and marker trees)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeConfig {
    /// Enable tree generation
    pub enabled: bool,
    /// Grid spacing for tree placement (default: 24)
    pub spacing: i32,
    /// Base placement probability (default: 0.2)
    pub placement_chance: f32,
    /// Marker tree chance when cave detected below (default: 0.6)
    pub marker_tree_chance: f32,
    /// Minimum air above for tree placement (default: 15)
    pub min_air_above: i32,
    /// Depth to scan below for cave detection (default: 50)
    pub cave_scan_depth: i32,
    /// Noise seed offset
    pub seed_offset: i32,
}

/// Underground ruin generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuinConfig {
    /// Enable ruin generation
    pub enabled: bool,
    /// Grid spacing (default: 64)
    pub spacing: i32,
    /// Placement probability (default: 0.1)
    pub placement_chance: f32,
    /// Minimum depth (default: -500)
    pub min_depth: i32,
    /// Maximum depth (default: -50)
    pub max_depth: i32,
    /// Noise seed offset
    pub seed_offset: i32,
}

/// Underground biome zones configuration
///
/// Controls depth-based underground biomes like Mushroom Grotto, Crystal Caves, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndergroundZonesConfig {
    /// Enable underground zone system (replaces stone with zone-specific materials)
    pub enabled: bool,
    /// Whether surface biomes should influence underground zones
    pub surface_influence: bool,
    /// Individual zone overrides (optional, uses defaults if not specified)
    pub zone_overrides: Vec<ZoneOverrideConfig>,
}

/// Override configuration for a specific zone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneOverrideConfig {
    /// Zone name ("ShallowCaves", "MushroomGrotto", "CrystalCaves", "LavaCaverns", "Abyss")
    pub zone_name: String,
    /// Override minimum Y (top of zone)
    pub min_y: Option<i32>,
    /// Override maximum Y (bottom of zone)
    pub max_y: Option<i32>,
    /// Override primary stone material
    pub primary_stone: Option<u16>,
    /// Override feature density (0.0-1.0)
    pub feature_density: Option<f32>,
}

/// Reusable noise layer configuration
///
/// Abstracts FastNoiseLite settings for UI editing and serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseLayerConfig {
    /// Offset added to world seed for this layer
    pub seed_offset: i32,
    /// Noise algorithm type
    pub noise_type: NoiseTypeConfig,
    /// Base frequency (lower = larger features)
    pub frequency: f32,
    /// Fractal combination type
    pub fractal_type: FractalTypeConfig,
    /// Number of fractal octaves (1-8)
    pub octaves: u8,
    /// Frequency multiplier per octave (default: 2.0)
    pub lacunarity: f32,
    /// Amplitude multiplier per octave / persistence (default: 0.5)
    pub gain: f32,
}

/// Noise algorithm types (maps to FastNoiseLite::NoiseType)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoiseTypeConfig {
    OpenSimplex2,
    OpenSimplex2S,
    Cellular,
    Perlin,
    ValueCubic,
    Value,
}

/// Fractal combination types (maps to FastNoiseLite::FractalType)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FractalTypeConfig {
    None,
    FBm,
    Ridged,
    PingPong,
    DomainWarpProgressive,
    DomainWarpIndependent,
}

// ============================================================================
// Default implementations
// ============================================================================

impl Default for WorldGenConfig {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            world: WorldParams::default(),
            terrain: TerrainParams::default(),
            caves: CaveParams::default(),
            ores: default_ore_configs(),
            biomes: BiomeParams::default(),
            vegetation: VegetationParams::default(),
            features: FeatureParams::default(),
            underground_zones: UndergroundZonesConfig::default(),
        }
    }
}

impl Default for WorldParams {
    fn default() -> Self {
        Self {
            surface_y: 0,
            sky_height: 1000,
            bedrock_y: -70000, // 20× deeper (was -3500) - Noita-scale world depth
            underground_layers: UndergroundLayers::default(),
        }
    }
}

impl Default for UndergroundLayers {
    fn default() -> Self {
        Self {
            shallow: -10000, // 20× deeper (was -500)
            deep: -30000,    // 20× deeper (was -1500)
            cavern: -50000,  // 20× deeper (was -2500)
        }
    }
}

impl Default for TerrainParams {
    fn default() -> Self {
        Self {
            height_noise: NoiseLayerConfig {
                seed_offset: 2,
                noise_type: NoiseTypeConfig::OpenSimplex2,
                frequency: 0.0005, // 2× larger hills (was 0.001)
                fractal_type: FractalTypeConfig::FBm,
                octaves: 4,
                lacunarity: 2.0,
                gain: 0.5,
            },
            height_scale: 200.0, // More dramatic terrain (was 100.0)
        }
    }
}

impl Default for CaveParams {
    fn default() -> Self {
        Self {
            large_caves: NoiseLayerConfig {
                seed_offset: 3,
                noise_type: NoiseTypeConfig::OpenSimplex2,
                frequency: 0.00167, // 3× larger caverns (was 0.005) → ~600px features
                fractal_type: FractalTypeConfig::FBm,
                octaves: 3,
                lacunarity: 2.0,
                gain: 0.5,
            },
            tunnels: NoiseLayerConfig {
                seed_offset: 4,
                noise_type: NoiseTypeConfig::OpenSimplex2,
                frequency: 0.00267, // 3× larger tunnels (was 0.008) → ~375px features
                fractal_type: FractalTypeConfig::FBm,
                octaves: 4,
                lacunarity: 2.0,
                gain: 0.5,
            },
            large_threshold: 0.15,
            tunnel_threshold: 0.25,
            min_cave_depth: 10,
        }
    }
}

impl Default for BiomeParams {
    fn default() -> Self {
        Self {
            temperature_noise: NoiseLayerConfig {
                seed_offset: 0,
                noise_type: NoiseTypeConfig::OpenSimplex2,
                frequency: 0.0003,
                fractal_type: FractalTypeConfig::FBm,
                octaves: 2,
                lacunarity: 2.0,
                gain: 0.5,
            },
            moisture_noise: NoiseLayerConfig {
                seed_offset: 1,
                noise_type: NoiseTypeConfig::OpenSimplex2,
                frequency: 0.0003,
                fractal_type: FractalTypeConfig::FBm,
                octaves: 2,
                lacunarity: 2.0,
                gain: 0.5,
            },
            biomes: default_biome_configs(),
            transition: BiomeTransitionConfig::default(),
        }
    }
}

impl Default for BiomeTransitionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            width: 32,
            mode: BiomeBlendModeConfig::StableLayer,
            noise_scale_percent: 30,
            enforce_stability: true,
        }
    }
}

impl Default for VegetationParams {
    fn default() -> Self {
        Self {
            tree_noise: NoiseLayerConfig {
                seed_offset: 9,
                noise_type: NoiseTypeConfig::OpenSimplex2,
                frequency: 1.0, // Scaled by tree_noise_scale
                fractal_type: FractalTypeConfig::None,
                octaves: 1,
                lacunarity: 2.0,
                gain: 0.5,
            },
            plant_noise: NoiseLayerConfig {
                seed_offset: 10,
                noise_type: NoiseTypeConfig::OpenSimplex2,
                frequency: 1.0, // Scaled by plant_noise_scale
                fractal_type: FractalTypeConfig::None,
                octaves: 1,
                lacunarity: 2.0,
                gain: 0.5,
            },
            tree_noise_scale: 0.03,
            plant_noise_scale: 0.05,
        }
    }
}

impl Default for LavaPoolConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_depth: -50000, // 20× deeper (was -2500)
            threshold: 0.6,
            noise_scale: 0.05,
        }
    }
}

impl Default for StalactiteConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_depth: -1000,  // Deeper (was -50)
            spacing: 48,       // 3× larger spacing (was 16)
            min_length: 9,     // 3× longer (was 3)
            max_length: 36,    // 3× longer (was 12)
            base_width: 5,     // Slightly wider (was 3)
            min_air_below: 15, // 3× more clearance (was 5)
            seed_offset: 100,
            placement_chance: 0.5,
            taper: true,
        }
    }
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            spacing: 96,        // 3× larger spacing (was 32)
            min_gap_width: 48,  // 3× wider gaps (was 16)
            max_gap_width: 144, // 3× wider gaps (was 48)
            placement_chance: 0.3,
            min_depth: -2000, // Deeper (was -100)
            seed_offset: 200,
        }
    }
}

impl Default for TreeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            spacing: 24,
            placement_chance: 0.2,
            marker_tree_chance: 0.6,
            min_air_above: 15,
            cave_scan_depth: 50,
            seed_offset: 300,
        }
    }
}

impl Default for RuinConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            spacing: 192, // 3× larger spacing (was 64)
            placement_chance: 0.1,
            min_depth: -10000, // 20× deeper (was -500)
            max_depth: -1000,  // 20× deeper (was -50)
            seed_offset: 400,
        }
    }
}

impl Default for WireNetworkConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_depth: -15000, // Circuit Ruins zone start
            max_depth: -22000, // Circuit Ruins zone end
            h_spacing: 200,
            v_spacing: 300,
            battery_chance: 0.05,
            spark_chance: 0.10,
            h_wire_threshold: 0.85,
            v_wire_threshold: 0.88,
            seed_offset: 500,
        }
    }
}

impl Default for ThunderZoneConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_depth: -45000, // Thunder Caverns zone start
            max_depth: -52000, // Thunder Caverns zone end
            thunder_threshold: 0.90,
            spark_chance: 0.05,
            wire_rod_chance: 0.3,
            seed_offset: 600,
        }
    }
}

impl Default for VolatilePoolConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_depth: -30000, // Volatile Lakes zone start
            max_depth: -38000, // Volatile Lakes zone end
            pool_threshold: 0.70,
            nitro_weight: 0.6,
            oil_weight: 0.3,
            gunpowder_chance: 0.15,
            lava_vein_threshold: 0.92,
            seed_offset: 700,
        }
    }
}

impl Default for ToxicVentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_depth: -58000, // Toxic Depths zone start
            max_depth: -65000, // Toxic Depths zone end
            vent_threshold: 0.93,
            max_gas_height: 30,
            virus_threshold: 0.95,
            max_virus_radius: 8,
            mercury_threshold: 0.85,
            seed_offset: 800,
        }
    }
}

impl Default for UndergroundZonesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            surface_influence: false,
            zone_overrides: Vec::new(),
        }
    }
}

impl Default for NoiseLayerConfig {
    fn default() -> Self {
        Self {
            seed_offset: 0,
            noise_type: NoiseTypeConfig::OpenSimplex2,
            frequency: 0.01,
            fractal_type: FractalTypeConfig::FBm,
            octaves: 3,
            lacunarity: 2.0,
            gain: 0.5,
        }
    }
}

/// Create default ore configurations matching current generation
///
/// Ore veins are 5× larger than before (noise_scale 0.016 vs 0.08) and
/// depth ranges are 20× deeper to match the expanded world depth.
fn default_ore_configs() -> Vec<OreConfig> {
    vec![
        OreConfig {
            material_id: MaterialId::COAL_ORE,
            name: "Coal".to_string(),
            noise: NoiseLayerConfig {
                seed_offset: 5,
                noise_type: NoiseTypeConfig::OpenSimplex2,
                frequency: 1.0,
                fractal_type: FractalTypeConfig::None,
                octaves: 1,
                lacunarity: 2.0,
                gain: 0.5,
            },
            threshold: 0.70,    // Slightly lower for similar density (was 0.75)
            min_depth: -10000,  // 20× deeper (was -500)
            max_depth: -1000,   // 20× deeper (was -50)
            noise_scale: 0.016, // 5× larger veins (was 0.08) → 150-400px
        },
        OreConfig {
            material_id: MaterialId::COPPER_ORE,
            name: "Copper".to_string(),
            noise: NoiseLayerConfig {
                seed_offset: 7,
                noise_type: NoiseTypeConfig::OpenSimplex2,
                frequency: 1.0,
                fractal_type: FractalTypeConfig::None,
                octaves: 1,
                lacunarity: 2.0,
                gain: 0.5,
            },
            threshold: 0.72,    // Slightly lower for similar density (was 0.77)
            min_depth: -20000,  // 20× deeper (was -1000)
            max_depth: -4000,   // 20× deeper (was -200)
            noise_scale: 0.016, // 5× larger veins (was 0.08)
        },
        OreConfig {
            material_id: MaterialId::IRON_ORE,
            name: "Iron".to_string(),
            noise: NoiseLayerConfig {
                seed_offset: 6,
                noise_type: NoiseTypeConfig::OpenSimplex2,
                frequency: 1.0,
                fractal_type: FractalTypeConfig::None,
                octaves: 1,
                lacunarity: 2.0,
                gain: 0.5,
            },
            threshold: 0.71,    // Slightly lower for similar density (was 0.76)
            min_depth: -40000,  // 20× deeper (was -2000)
            max_depth: -10000,  // 20× deeper (was -500)
            noise_scale: 0.016, // 5× larger veins (was 0.08)
        },
        OreConfig {
            material_id: MaterialId::GOLD_ORE,
            name: "Gold".to_string(),
            noise: NoiseLayerConfig {
                seed_offset: 8,
                noise_type: NoiseTypeConfig::OpenSimplex2,
                frequency: 1.0,
                fractal_type: FractalTypeConfig::None,
                octaves: 1,
                lacunarity: 2.0,
                gain: 0.5,
            },
            threshold: 0.75,    // Slightly lower for similar density (was 0.80)
            min_depth: -60000,  // 20× deeper (was -3000)
            max_depth: -30000,  // 20× deeper (was -1500)
            noise_scale: 0.016, // 5× larger veins (was 0.08)
        },
    ]
}

/// Create default biome configurations matching current BiomeDefinitions
fn default_biome_configs() -> Vec<BiomeConfig> {
    vec![
        // Ocean: temp < -0.3 && moisture < -0.3 (checked first)
        BiomeConfig {
            name: "Ocean".to_string(),
            biome_type: "Ocean".to_string(),
            temp_min: -1.0,
            temp_max: -0.3,
            moisture_min: -1.0,
            moisture_max: -0.3,
            priority: 100, // Highest priority
            surface_material: MaterialId::WATER,
            subsurface_material: MaterialId::SAND,
            stone_depth: 30,
            height_variance: 0.2,
            height_offset: -40,
            tree_density: 0.0,
            plant_density: 0.0,
            cave_density_multiplier: 0.5,
            ore_multipliers: HashMap::new(),
        },
        // Mountains: temp > 0.5
        BiomeConfig {
            name: "Mountains".to_string(),
            biome_type: "Mountains".to_string(),
            temp_min: 0.5,
            temp_max: 1.0,
            moisture_min: -1.0,
            moisture_max: 1.0,
            priority: 90,
            surface_material: MaterialId::STONE,
            subsurface_material: MaterialId::STONE,
            stone_depth: 5,
            height_variance: 1.2,
            height_offset: 30,
            tree_density: 0.02,
            plant_density: 0.1,
            cave_density_multiplier: 0.8,
            ore_multipliers: {
                let mut m = HashMap::new();
                m.insert(MaterialId::IRON_ORE, 1.5);
                m.insert(MaterialId::COPPER_ORE, 1.3);
                m
            },
        },
        // Desert: temp > 0.0 && moisture < -0.2
        BiomeConfig {
            name: "Desert".to_string(),
            biome_type: "Desert".to_string(),
            temp_min: 0.0,
            temp_max: 0.5,
            moisture_min: -1.0,
            moisture_max: -0.2,
            priority: 80,
            surface_material: MaterialId::SAND,
            subsurface_material: MaterialId::SAND,
            stone_depth: 15,
            height_variance: 0.3,
            height_offset: 5,
            tree_density: 0.0,
            plant_density: 0.05,
            cave_density_multiplier: 1.2,
            ore_multipliers: {
                let mut m = HashMap::new();
                m.insert(MaterialId::GOLD_ORE, 1.5);
                m.insert(MaterialId::COAL_ORE, 0.5);
                m
            },
        },
        // Forest: moisture > 0.3
        BiomeConfig {
            name: "Forest".to_string(),
            biome_type: "Forest".to_string(),
            temp_min: -0.3,
            temp_max: 0.5,
            moisture_min: 0.3,
            moisture_max: 1.0,
            priority: 70,
            surface_material: MaterialId::DIRT,
            subsurface_material: MaterialId::DIRT,
            stone_depth: 25,
            height_variance: 0.6,
            height_offset: 0,
            tree_density: 0.15,
            plant_density: 0.4,
            cave_density_multiplier: 1.0,
            ore_multipliers: HashMap::new(),
        },
        // Plains: default (lowest priority)
        BiomeConfig {
            name: "Plains".to_string(),
            biome_type: "Plains".to_string(),
            temp_min: -1.0,
            temp_max: 1.0,
            moisture_min: -1.0,
            moisture_max: 1.0,
            priority: 0, // Lowest - catch-all
            surface_material: MaterialId::DIRT,
            subsurface_material: MaterialId::DIRT,
            stone_depth: 20,
            height_variance: 0.4,
            height_offset: 0,
            tree_density: 0.05,
            plant_density: 0.3,
            cave_density_multiplier: 1.0,
            ore_multipliers: {
                let mut m = HashMap::new();
                m.insert(MaterialId::COAL_ORE, 1.2);
                m
            },
        },
    ]
}

// ============================================================================
// Conversion helpers
// ============================================================================

impl NoiseTypeConfig {
    /// Convert to fastnoise_lite::NoiseType
    pub fn to_fastnoise(&self) -> fastnoise_lite::NoiseType {
        match self {
            NoiseTypeConfig::OpenSimplex2 => fastnoise_lite::NoiseType::OpenSimplex2,
            NoiseTypeConfig::OpenSimplex2S => fastnoise_lite::NoiseType::OpenSimplex2S,
            NoiseTypeConfig::Cellular => fastnoise_lite::NoiseType::Cellular,
            NoiseTypeConfig::Perlin => fastnoise_lite::NoiseType::Perlin,
            NoiseTypeConfig::ValueCubic => fastnoise_lite::NoiseType::ValueCubic,
            NoiseTypeConfig::Value => fastnoise_lite::NoiseType::Value,
        }
    }
}

impl FractalTypeConfig {
    /// Convert to fastnoise_lite::FractalType
    pub fn to_fastnoise(&self) -> fastnoise_lite::FractalType {
        match self {
            FractalTypeConfig::None => fastnoise_lite::FractalType::None,
            FractalTypeConfig::FBm => fastnoise_lite::FractalType::FBm,
            FractalTypeConfig::Ridged => fastnoise_lite::FractalType::Ridged,
            FractalTypeConfig::PingPong => fastnoise_lite::FractalType::PingPong,
            FractalTypeConfig::DomainWarpProgressive => {
                fastnoise_lite::FractalType::DomainWarpProgressive
            }
            FractalTypeConfig::DomainWarpIndependent => {
                fastnoise_lite::FractalType::DomainWarpIndependent
            }
        }
    }
}

impl NoiseLayerConfig {
    /// Create a FastNoiseLite instance from this config
    pub fn to_fastnoise(&self, base_seed: u64) -> fastnoise_lite::FastNoiseLite {
        let mut noise =
            fastnoise_lite::FastNoiseLite::with_seed((base_seed as i32) + self.seed_offset);
        noise.set_noise_type(Some(self.noise_type.to_fastnoise()));
        noise.set_frequency(Some(self.frequency));
        noise.set_fractal_type(Some(self.fractal_type.to_fastnoise()));
        noise.set_fractal_octaves(Some(self.octaves as i32));
        noise.set_fractal_lacunarity(Some(self.lacunarity));
        noise.set_fractal_gain(Some(self.gain));
        noise
    }
}

impl BiomeConfig {
    /// Check if this biome matches the given temperature and moisture values
    pub fn matches(&self, temperature: f64, moisture: f64) -> bool {
        temperature >= self.temp_min as f64
            && temperature <= self.temp_max as f64
            && moisture >= self.moisture_min as f64
            && moisture <= self.moisture_max as f64
    }

    /// Get ore multiplier for a specific ore type (1.0 if not specified)
    pub fn get_ore_multiplier(&self, ore_material: u16) -> f32 {
        *self.ore_multipliers.get(&ore_material).unwrap_or(&1.0)
    }
}

impl BiomeParams {
    /// Select the best matching biome for given temperature and moisture
    pub fn select_biome(&self, temperature: f64, moisture: f64) -> Option<&BiomeConfig> {
        // Sort by priority (highest first) and find first match
        let mut matching: Vec<&BiomeConfig> = self
            .biomes
            .iter()
            .filter(|b| b.matches(temperature, moisture))
            .collect();

        matching.sort_by(|a, b| b.priority.cmp(&a.priority));
        matching.first().copied()
    }
}

// ============================================================================
// Preset helpers
// ============================================================================

impl WorldGenConfig {
    /// Create a preset with more caves
    pub fn preset_cave_heavy() -> Self {
        Self {
            name: "Cave Heavy".to_string(),
            caves: CaveParams {
                large_threshold: 0.10, // Lower = more caves
                tunnel_threshold: 0.15,
                ..CaveParams::default()
            },
            ..Self::default()
        }
    }

    /// Create a preset with desert-dominated world
    pub fn preset_desert_world() -> Self {
        let mut biomes = default_biome_configs();
        // Shift biome thresholds to favor desert
        for biome in &mut biomes {
            if biome.name == "Desert" {
                biome.temp_min = -0.3;
                biome.moisture_max = 0.3;
            }
        }
        Self {
            name: "Desert World".to_string(),
            biomes: BiomeParams {
                biomes,
                ..BiomeParams::default()
            },
            ..Self::default()
        }
    }

    /// Create a preset with mountain-dominated world
    pub fn preset_mountain_world() -> Self {
        let mut biomes = default_biome_configs();
        // Shift biome thresholds to favor mountains
        for biome in &mut biomes {
            if biome.name == "Mountains" {
                biome.temp_min = 0.0;
            }
        }
        Self {
            name: "Mountain World".to_string(),
            terrain: TerrainParams {
                height_scale: 150.0, // Taller terrain
                ..TerrainParams::default()
            },
            biomes: BiomeParams {
                biomes,
                ..BiomeParams::default()
            },
            ..Self::default()
        }
    }

    /// Create a preset with flat terrain (good for building)
    pub fn preset_flat() -> Self {
        let mut biomes = default_biome_configs();
        for biome in &mut biomes {
            biome.height_variance = 0.1;
            biome.height_offset = 0;
        }
        Self {
            name: "Flat World".to_string(),
            terrain: TerrainParams {
                height_scale: 10.0, // Very flat
                ..TerrainParams::default()
            },
            biomes: BiomeParams {
                biomes,
                ..BiomeParams::default()
            },
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WorldGenConfig::default();
        assert_eq!(config.name, "Default");
        assert_eq!(config.world.surface_y, 0);
        assert_eq!(config.world.bedrock_y, -70000); // Noita-scale depth
        assert!(!config.ores.is_empty());
        assert!(!config.biomes.biomes.is_empty());
    }

    #[test]
    fn test_biome_selection() {
        let config = WorldGenConfig::default();

        // Ocean: temp < -0.3 && moisture < -0.3
        let ocean = config.biomes.select_biome(-0.5, -0.5);
        assert!(ocean.is_some());
        assert_eq!(ocean.unwrap().name, "Ocean");

        // Mountains: temp > 0.5
        let mountains = config.biomes.select_biome(0.7, 0.0);
        assert!(mountains.is_some());
        assert_eq!(mountains.unwrap().name, "Mountains");

        // Desert: temp > 0.0 && moisture < -0.2
        let desert = config.biomes.select_biome(0.3, -0.5);
        assert!(desert.is_some());
        assert_eq!(desert.unwrap().name, "Desert");

        // Forest: moisture > 0.3
        let forest = config.biomes.select_biome(0.0, 0.5);
        assert!(forest.is_some());
        assert_eq!(forest.unwrap().name, "Forest");

        // Plains: default
        let plains = config.biomes.select_biome(0.0, 0.0);
        assert!(plains.is_some());
        assert_eq!(plains.unwrap().name, "Plains");
    }

    #[test]
    fn test_noise_config_to_fastnoise() {
        let config = NoiseLayerConfig::default();
        let noise = config.to_fastnoise(42);
        // Just verify it doesn't panic
        let _ = noise.get_noise_2d(0.0, 0.0);
    }

    #[test]
    fn test_ron_serialization() {
        let config = WorldGenConfig::default();
        let serialized = ron::to_string(&config).expect("Failed to serialize");
        let deserialized: WorldGenConfig =
            ron::from_str(&serialized).expect("Failed to deserialize");
        assert_eq!(config.name, deserialized.name);
        assert_eq!(config.world.surface_y, deserialized.world.surface_y);
    }

    #[test]
    fn test_presets() {
        let cave_heavy = WorldGenConfig::preset_cave_heavy();
        assert_eq!(cave_heavy.name, "Cave Heavy");
        assert!(cave_heavy.caves.large_threshold < 0.15);

        let flat = WorldGenConfig::preset_flat();
        assert_eq!(flat.name, "Flat World");
        assert!(flat.terrain.height_scale < 20.0);
    }
}
