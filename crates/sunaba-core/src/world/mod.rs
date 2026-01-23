//! World management - chunks, loading, saving

pub mod biome;
pub mod biome_transition;
pub mod biome_zones;
mod ca_update;
mod chemistry_system;
mod chunk;
mod chunk_manager;
mod chunk_status;
mod collision;
pub mod context_scanner;
mod debris_system;
pub mod electrical_system;
pub mod features;
pub mod generation;
mod light_system;
mod mining_system;
mod neighbor_queries;
pub mod persistence;
mod persistence_system;
#[cfg(feature = "regeneration")]
pub mod pixel_entity_system;
mod pixel_queries;
mod player_physics;
pub mod pressure_system;
mod raycasting;
pub mod rng_trait;
#[cfg(feature = "regeneration")]
pub mod special_behaviors_system;
pub mod stats;
pub mod structure_placement;
pub mod structure_templates;
pub mod structures;
#[allow(clippy::module_inception)]
mod world;
pub mod worldgen_config;

pub use biome::{BiomeDefinition, BiomeRegistry, BiomeType};
pub use biome_transition::{
    BiomeTransition, BlendMode, MaterialStability, classify_material_stability, find_biome_boundary,
};
pub use biome_zones::{BiomeZoneRegistry, UndergroundZone, ZoneDefinition, ZoneTransition};
pub use chunk::{CHUNK_SIZE, Chunk, Pixel, pixel_flags};
pub use chunk_manager::ChunkManager;
pub use chunk_status::ChunkStatus;
pub use context_scanner::{
    ContextScanner, MAX_SCAN_DISTANCE, PlacementContext, PlacementPredicate,
};
pub use debris_system::DebrisSystem;
pub use electrical_system::ElectricalSystem;
pub use generation::WorldGenerator;
pub use light_system::LightSystem;
pub use mining_system::MiningSystem;
pub use neighbor_queries::NeighborQueries;
pub use persistence::{ChunkPersistence, WorldMetadata};
pub use persistence_system::PersistenceSystem;
#[cfg(feature = "regeneration")]
pub use pixel_entity_system::PixelEntitySystem;
pub use pixel_queries::PixelQueries;
pub use player_physics::PlayerPhysicsSystem;
pub use raycasting::Raycasting;
pub use rng_trait::WorldRng;
#[cfg(feature = "regeneration")]
pub use special_behaviors_system::SpecialBehaviorsSystem;
pub use stats::{NoopStats, SimStats};
pub use structures::{AnchorType, StructureTemplate, StructureVariants};
pub use world::World;
pub use worldgen_config::{
    BiomeBlendModeConfig, BiomeConfig, BiomeParams, BiomeTransitionConfig, BridgeConfig,
    CaveParams, FeatureParams, FractalTypeConfig, LavaPoolConfig, NoiseLayerConfig,
    NoiseTypeConfig, OreConfig, RuinConfig, StalactiteConfig, StructureConfig, TerrainParams,
    TreeConfig, UndergroundLayers, UndergroundZonesConfig, VegetationParams, WorldGenConfig,
    WorldParams, ZoneOverrideConfig,
};
