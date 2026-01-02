//! Material definitions and registry

use serde::{Deserialize, Serialize};

/// Built-in material IDs
pub struct MaterialId;

impl MaterialId {
    // Original materials (0-14)
    pub const AIR: u16 = 0;
    pub const STONE: u16 = 1;
    pub const SAND: u16 = 2;
    pub const WATER: u16 = 3;
    pub const WOOD: u16 = 4;
    pub const FIRE: u16 = 5;
    pub const SMOKE: u16 = 6;
    pub const STEAM: u16 = 7;
    pub const LAVA: u16 = 8;
    pub const OIL: u16 = 9;
    pub const ACID: u16 = 10;
    pub const ICE: u16 = 11;
    pub const GLASS: u16 = 12;
    pub const METAL: u16 = 13;
    pub const BEDROCK: u16 = 14;

    // Phase 5: New materials (15-30+)
    // Organic materials
    pub const DIRT: u16 = 15;
    pub const PLANT_MATTER: u16 = 16;
    pub const FRUIT: u16 = 17;
    pub const FLESH: u16 = 18;
    pub const BONE: u16 = 19;
    pub const ASH: u16 = 20;

    // Ore materials
    pub const COAL_ORE: u16 = 21;
    pub const IRON_ORE: u16 = 22;
    pub const COPPER_ORE: u16 = 23;
    pub const GOLD_ORE: u16 = 24;

    // Refined materials
    pub const COPPER_INGOT: u16 = 25;
    pub const IRON_INGOT: u16 = 26;
    pub const BRONZE_INGOT: u16 = 27;
    pub const STEEL_INGOT: u16 = 28;

    // Special materials
    pub const GUNPOWDER: u16 = 29;
    pub const POISON_GAS: u16 = 30;
    pub const FERTILIZER: u16 = 31;
}

/// How a material behaves physically
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaterialType {
    /// Doesn't move (stone, wood, metal)
    Solid,
    /// Falls, piles up (sand, gravel, ash)
    Powder,
    /// Flows, seeks level (water, oil, lava)
    Liquid,
    /// Rises, disperses (steam, smoke)
    Gas,
}

/// Tags for material categorization and behavior
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaterialTag {
    /// Organic matter (wood, flesh, plant)
    Organic,
    /// Metallic materials (iron, copper, gold)
    Metallic,
    /// Edible by creatures
    Edible,
    /// Mineable ore
    Ore,
    /// Combustible fuel
    Fuel,
    /// Toxic/poisonous
    Toxic,
    /// Natural stone/mineral
    Mineral,
    /// Manufactured/refined
    Refined,
}

/// Definition of a material's properties
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaterialDef {
    pub id: u16,
    pub name: String,
    pub material_type: MaterialType,

    /// Base color (RGBA)
    pub color: [u8; 4],

    /// Density (g/cm³) - affects sinking/floating
    pub density: f32,

    // Physical properties
    /// Mining/breaking resistance (None = unbreakable)
    pub hardness: Option<u8>,
    /// Sliding coefficient (powders)
    pub friction: f32,
    /// Flow speed (liquids)
    pub viscosity: f32,

    // Thermal properties
    /// Temperature at which this melts (Celsius)
    pub melting_point: Option<f32>,
    /// Temperature at which this boils/evaporates
    pub boiling_point: Option<f32>,
    /// Temperature at which this freezes
    pub freezing_point: Option<f32>,
    /// Temperature at which this ignites
    pub ignition_temp: Option<f32>,
    /// Heat conductivity (0.0 - 1.0)
    pub heat_conductivity: f32,

    // State transitions
    /// What this becomes when melted
    pub melts_to: Option<u16>,
    /// What this becomes when boiled
    pub boils_to: Option<u16>,
    /// What this becomes when frozen
    pub freezes_to: Option<u16>,
    /// What this becomes when burned
    pub burns_to: Option<u16>,
    /// How fast this burns (0.0 - 1.0)
    pub burn_rate: f32,

    // Flags
    pub flammable: bool,
    /// Can support other solid pixels
    pub structural: bool,
    pub conducts_electricity: bool,

    // Creature interaction properties (Phase 5)
    /// Food value when consumed (0-100 calories)
    pub nutritional_value: Option<f32>,
    /// Poison damage per second when consumed
    pub toxicity: Option<f32>,
    /// Mining speed multiplier (1.0 = normal, 2.0 = twice as hard)
    pub hardness_multiplier: f32,
    /// Maximum weight this material can support before collapse
    pub structural_strength: Option<f32>,
    /// Energy released when burned (for smelting/cooking)
    pub fuel_value: Option<f32>,
    /// Material category tags
    pub tags: Vec<MaterialTag>,
}

impl Default for MaterialDef {
    fn default() -> Self {
        Self {
            id: 0,
            name: "unknown".to_string(),
            material_type: MaterialType::Solid,
            color: [255, 0, 255, 255], // Magenta for missing materials
            density: 1.0,
            hardness: Some(1),
            friction: 0.5,
            viscosity: 0.5,
            melting_point: None,
            boiling_point: None,
            freezing_point: None,
            ignition_temp: None,
            heat_conductivity: 0.5,
            melts_to: None,
            boils_to: None,
            freezes_to: None,
            burns_to: None,
            burn_rate: 0.0,
            flammable: false,
            structural: false,
            conducts_electricity: false,
            nutritional_value: None,
            toxicity: None,
            hardness_multiplier: 1.0,
            structural_strength: None,
            fuel_value: None,
            tags: Vec::new(),
        }
    }
}

/// Registry of all materials
pub struct Materials {
    materials: Vec<MaterialDef>,
}

impl Materials {
    pub fn new() -> Self {
        let mut materials = Self {
            materials: Vec::new(),
        };
        materials.register_defaults();
        materials
    }

    fn register_defaults(&mut self) {
        // Air (empty space)
        self.register(MaterialDef {
            id: MaterialId::AIR,
            name: "air".to_string(),
            material_type: MaterialType::Gas,
            color: [0, 0, 0, 0], // Transparent
            density: 0.001,
            hardness: None,
            ..Default::default()
        });

        // Stone
        self.register(MaterialDef {
            id: MaterialId::STONE,
            name: "stone".to_string(),
            material_type: MaterialType::Solid,
            color: [128, 128, 128, 255],
            density: 2.5,
            hardness: Some(5),
            structural: true,
            melting_point: Some(1200.0),
            melts_to: Some(MaterialId::LAVA),
            ..Default::default()
        });

        // Sand
        self.register(MaterialDef {
            id: MaterialId::SAND,
            name: "sand".to_string(),
            material_type: MaterialType::Powder,
            color: [194, 178, 128, 255],
            density: 1.5,
            hardness: Some(1),
            friction: 0.3,
            melting_point: Some(1700.0),
            melts_to: Some(MaterialId::GLASS),
            ..Default::default()
        });

        // Water
        self.register(MaterialDef {
            id: MaterialId::WATER,
            name: "water".to_string(),
            material_type: MaterialType::Liquid,
            color: [64, 164, 223, 200],
            density: 1.0,
            hardness: None,
            viscosity: 0.1,
            boiling_point: Some(100.0),
            boils_to: Some(MaterialId::STEAM),
            freezing_point: Some(0.0),
            freezes_to: Some(MaterialId::ICE),
            heat_conductivity: 0.6,
            ..Default::default()
        });

        // Wood
        self.register(MaterialDef {
            id: MaterialId::WOOD,
            name: "wood".to_string(),
            material_type: MaterialType::Solid,
            color: [139, 90, 43, 255],
            density: 0.6,
            hardness: Some(2),
            structural: true,
            flammable: true,
            ignition_temp: Some(300.0),
            burns_to: Some(MaterialId::ASH),
            burn_rate: 0.02,
            fuel_value: Some(15.0),
            tags: vec![MaterialTag::Organic, MaterialTag::Fuel],
            ..Default::default()
        });

        // Fire
        self.register(MaterialDef {
            id: MaterialId::FIRE,
            name: "fire".to_string(),
            material_type: MaterialType::Gas,
            color: [255, 100, 0, 255],
            density: 0.0001,
            hardness: None,
            ..Default::default()
        });

        // Smoke
        self.register(MaterialDef {
            id: MaterialId::SMOKE,
            name: "smoke".to_string(),
            material_type: MaterialType::Gas,
            color: [60, 60, 60, 150],
            density: 0.001,
            hardness: None,
            ..Default::default()
        });

        // Steam
        self.register(MaterialDef {
            id: MaterialId::STEAM,
            name: "steam".to_string(),
            material_type: MaterialType::Gas,
            color: [200, 200, 200, 100],
            density: 0.0006,
            hardness: None,
            freezing_point: Some(100.0), // Condenses below boiling point
            freezes_to: Some(MaterialId::WATER),
            ..Default::default()
        });

        // Lava
        self.register(MaterialDef {
            id: MaterialId::LAVA,
            name: "lava".to_string(),
            material_type: MaterialType::Liquid,
            color: [255, 80, 0, 255],
            density: 3.0,
            hardness: None,
            viscosity: 0.8, // Very viscous
            freezing_point: Some(700.0),
            freezes_to: Some(MaterialId::STONE),
            heat_conductivity: 0.8,
            ..Default::default()
        });

        // Oil
        self.register(MaterialDef {
            id: MaterialId::OIL,
            name: "oil".to_string(),
            material_type: MaterialType::Liquid,
            color: [50, 40, 30, 255],
            density: 0.8, // Floats on water
            hardness: None,
            viscosity: 0.3,
            flammable: true,
            ignition_temp: Some(200.0),
            burns_to: Some(MaterialId::SMOKE),
            burn_rate: 0.05,
            ..Default::default()
        });

        // Acid
        self.register(MaterialDef {
            id: MaterialId::ACID,
            name: "acid".to_string(),
            material_type: MaterialType::Liquid,
            color: [0, 255, 0, 200],
            density: 1.1,
            hardness: None,
            viscosity: 0.2,
            ..Default::default()
        });

        // Ice
        self.register(MaterialDef {
            id: MaterialId::ICE,
            name: "ice".to_string(),
            material_type: MaterialType::Solid,
            color: [200, 230, 255, 200],
            density: 0.9,
            hardness: Some(2),
            structural: true,
            melting_point: Some(0.0),
            melts_to: Some(MaterialId::WATER),
            ..Default::default()
        });

        // Glass
        self.register(MaterialDef {
            id: MaterialId::GLASS,
            name: "glass".to_string(),
            material_type: MaterialType::Solid,
            color: [200, 220, 255, 150],
            density: 2.5,
            hardness: Some(3),
            structural: true,
            melting_point: Some(1400.0),
            melts_to: Some(MaterialId::LAVA), // Molten glass
            ..Default::default()
        });

        // Metal
        self.register(MaterialDef {
            id: MaterialId::METAL,
            name: "metal".to_string(),
            material_type: MaterialType::Solid,
            color: [180, 180, 190, 255],
            density: 7.8,
            hardness: Some(7),
            structural: true,
            melting_point: Some(1500.0),
            melts_to: Some(MaterialId::LAVA), // Molten metal
            heat_conductivity: 0.9,
            conducts_electricity: true,
            ..Default::default()
        });

        // Bedrock - indestructible foundation
        self.register(MaterialDef {
            id: MaterialId::BEDROCK,
            name: "bedrock".to_string(),
            material_type: MaterialType::Solid,
            color: [40, 40, 50, 255], // Dark gray
            density: 100.0,
            hardness: None, // None = indestructible
            structural: true,
            heat_conductivity: 0.1,
            ..Default::default()
        });

        // ===== PHASE 5: NEW MATERIALS =====

        // ORGANIC MATERIALS

        // Dirt - mineable ground material
        self.register(MaterialDef {
            id: MaterialId::DIRT,
            name: "dirt".to_string(),
            material_type: MaterialType::Powder,
            color: [101, 67, 33, 255], // Brown
            density: 1.3,
            hardness: Some(1),
            friction: 0.4,
            hardness_multiplier: 0.5, // Easy to mine
            tags: vec![MaterialTag::Organic, MaterialTag::Mineral],
            ..Default::default()
        });

        // Plant Matter - grows, provides food
        self.register(MaterialDef {
            id: MaterialId::PLANT_MATTER,
            name: "plant_matter".to_string(),
            material_type: MaterialType::Solid,
            color: [34, 139, 34, 255], // Forest green
            density: 0.4,
            hardness: Some(1),
            flammable: true,
            ignition_temp: Some(250.0),
            burns_to: Some(MaterialId::ASH),
            burn_rate: 0.03,
            nutritional_value: Some(10.0), // Low calories
            fuel_value: Some(5.0),
            hardness_multiplier: 0.3, // Very easy to harvest
            tags: vec![MaterialTag::Organic, MaterialTag::Edible, MaterialTag::Fuel],
            ..Default::default()
        });

        // Fruit - high nutrition
        self.register(MaterialDef {
            id: MaterialId::FRUIT,
            name: "fruit".to_string(),
            material_type: MaterialType::Powder,
            color: [255, 69, 0, 255], // Red-orange
            density: 0.6,
            hardness: Some(1),
            friction: 0.2,
            nutritional_value: Some(40.0), // High calories
            hardness_multiplier: 0.1,      // Almost instant harvest
            tags: vec![MaterialTag::Organic, MaterialTag::Edible],
            ..Default::default()
        });

        // Flesh - creature material, high nutrition but toxic when rotten
        self.register(MaterialDef {
            id: MaterialId::FLESH,
            name: "flesh".to_string(),
            material_type: MaterialType::Powder,
            color: [205, 92, 92, 255], // Indian red
            density: 1.0,
            hardness: Some(1),
            friction: 0.3,
            nutritional_value: Some(50.0), // Very high calories
            flammable: true,
            ignition_temp: Some(200.0),
            burns_to: Some(MaterialId::ASH),
            burn_rate: 0.04,
            tags: vec![MaterialTag::Organic, MaterialTag::Edible],
            ..Default::default()
        });

        // Bone - structural organic material
        self.register(MaterialDef {
            id: MaterialId::BONE,
            name: "bone".to_string(),
            material_type: MaterialType::Solid,
            color: [245, 245, 220, 255], // Beige
            density: 1.8,
            hardness: Some(4),
            structural: true,
            structural_strength: Some(50.0),
            hardness_multiplier: 1.5,
            tags: vec![MaterialTag::Organic],
            ..Default::default()
        });

        // Ash - burn product, can be fertilizer
        self.register(MaterialDef {
            id: MaterialId::ASH,
            name: "ash".to_string(),
            material_type: MaterialType::Powder,
            color: [128, 128, 128, 200], // Gray, slightly transparent
            density: 0.5,
            hardness: Some(1),
            friction: 0.1, // Very slippery
            tags: vec![MaterialTag::Mineral],
            ..Default::default()
        });

        // ORE MATERIALS

        // Coal Ore - fuel source
        self.register(MaterialDef {
            id: MaterialId::COAL_ORE,
            name: "coal_ore".to_string(),
            material_type: MaterialType::Solid,
            color: [25, 25, 25, 255], // Almost black
            density: 2.3,
            hardness: Some(3),
            structural: true,
            flammable: true,
            ignition_temp: Some(400.0),
            burns_to: Some(MaterialId::ASH),
            burn_rate: 0.01,
            fuel_value: Some(30.0), // High energy
            hardness_multiplier: 1.2,
            tags: vec![MaterialTag::Ore, MaterialTag::Fuel, MaterialTag::Mineral],
            ..Default::default()
        });

        // Iron Ore - common metal ore
        self.register(MaterialDef {
            id: MaterialId::IRON_ORE,
            name: "iron_ore".to_string(),
            material_type: MaterialType::Solid,
            color: [139, 90, 90, 255], // Rusty brown
            density: 5.0,
            hardness: Some(5),
            structural: true,
            melting_point: Some(1200.0),
            melts_to: Some(MaterialId::IRON_INGOT),
            hardness_multiplier: 2.0, // Harder to mine
            tags: vec![MaterialTag::Ore, MaterialTag::Mineral],
            ..Default::default()
        });

        // Copper Ore - conductive metal ore
        self.register(MaterialDef {
            id: MaterialId::COPPER_ORE,
            name: "copper_ore".to_string(),
            material_type: MaterialType::Solid,
            color: [184, 115, 51, 255], // Copper brown
            density: 4.5,
            hardness: Some(4),
            structural: true,
            melting_point: Some(1000.0),
            melts_to: Some(MaterialId::COPPER_INGOT),
            hardness_multiplier: 1.8,
            conducts_electricity: true,
            tags: vec![MaterialTag::Ore, MaterialTag::Mineral],
            ..Default::default()
        });

        // Gold Ore - valuable rare ore
        self.register(MaterialDef {
            id: MaterialId::GOLD_ORE,
            name: "gold_ore".to_string(),
            material_type: MaterialType::Solid,
            color: [255, 215, 0, 255], // Gold
            density: 8.0,
            hardness: Some(4),
            structural: true,
            melting_point: Some(1064.0),
            melts_to: Some(MaterialId::COPPER_INGOT), // FIXME: Should be gold_ingot
            hardness_multiplier: 1.5,
            conducts_electricity: true,
            tags: vec![MaterialTag::Ore, MaterialTag::Mineral],
            ..Default::default()
        });

        // REFINED MATERIALS

        // Copper Ingot - smelted copper
        self.register(MaterialDef {
            id: MaterialId::COPPER_INGOT,
            name: "copper_ingot".to_string(),
            material_type: MaterialType::Solid,
            color: [205, 127, 50, 255], // Bronze
            density: 8.9,
            hardness: Some(6),
            structural: true,
            melting_point: Some(1084.0),
            melts_to: Some(MaterialId::LAVA),
            heat_conductivity: 0.9,
            conducts_electricity: true,
            structural_strength: Some(100.0),
            hardness_multiplier: 2.5,
            tags: vec![MaterialTag::Metallic, MaterialTag::Refined],
            ..Default::default()
        });

        // Iron Ingot - smelted iron
        self.register(MaterialDef {
            id: MaterialId::IRON_INGOT,
            name: "iron_ingot".to_string(),
            material_type: MaterialType::Solid,
            color: [169, 169, 169, 255], // Dark gray
            density: 7.8,
            hardness: Some(7),
            structural: true,
            melting_point: Some(1538.0),
            melts_to: Some(MaterialId::LAVA),
            heat_conductivity: 0.8,
            conducts_electricity: true,
            structural_strength: Some(150.0),
            hardness_multiplier: 3.0,
            tags: vec![MaterialTag::Metallic, MaterialTag::Refined],
            ..Default::default()
        });

        // Bronze Ingot - copper + tin alloy (using copper for now)
        self.register(MaterialDef {
            id: MaterialId::BRONZE_INGOT,
            name: "bronze_ingot".to_string(),
            material_type: MaterialType::Solid,
            color: [140, 120, 83, 255], // Bronze brown
            density: 8.7,
            hardness: Some(6),
            structural: true,
            melting_point: Some(950.0),
            melts_to: Some(MaterialId::LAVA),
            heat_conductivity: 0.7,
            structural_strength: Some(120.0),
            hardness_multiplier: 2.8,
            tags: vec![MaterialTag::Metallic, MaterialTag::Refined],
            ..Default::default()
        });

        // Steel Ingot - iron + carbon alloy
        self.register(MaterialDef {
            id: MaterialId::STEEL_INGOT,
            name: "steel_ingot".to_string(),
            material_type: MaterialType::Solid,
            color: [192, 192, 192, 255], // Silver
            density: 7.85,
            hardness: Some(8),
            structural: true,
            melting_point: Some(1370.0),
            melts_to: Some(MaterialId::LAVA),
            heat_conductivity: 0.75,
            structural_strength: Some(200.0),
            hardness_multiplier: 3.5, // Very hard to mine
            tags: vec![MaterialTag::Metallic, MaterialTag::Refined],
            ..Default::default()
        });

        // SPECIAL MATERIALS

        // Gunpowder - explosive powder
        self.register(MaterialDef {
            id: MaterialId::GUNPOWDER,
            name: "gunpowder".to_string(),
            material_type: MaterialType::Powder,
            color: [64, 64, 64, 255], // Dark gray
            density: 1.7,
            hardness: Some(1),
            friction: 0.2,
            flammable: true,
            ignition_temp: Some(150.0),
            burns_to: Some(MaterialId::SMOKE),
            burn_rate: 0.9,         // Burns VERY fast (explosion)
            fuel_value: Some(50.0), // High energy
            tags: vec![MaterialTag::Fuel],
            ..Default::default()
        });

        // Poison Gas - toxic gas
        self.register(MaterialDef {
            id: MaterialId::POISON_GAS,
            name: "poison_gas".to_string(),
            material_type: MaterialType::Gas,
            color: [50, 205, 50, 150], // Lime green, transparent
            density: 0.002,
            hardness: None,
            toxicity: Some(5.0), // 5 damage per second
            tags: vec![MaterialTag::Toxic],
            ..Default::default()
        });

        // Fertilizer - enhances plant growth
        self.register(MaterialDef {
            id: MaterialId::FERTILIZER,
            name: "fertilizer".to_string(),
            material_type: MaterialType::Powder,
            color: [101, 67, 33, 255], // Dark brown
            density: 0.9,
            hardness: Some(1),
            friction: 0.3,
            tags: vec![MaterialTag::Organic],
            ..Default::default()
        });
    }

    fn register(&mut self, material: MaterialDef) {
        let id = material.id as usize;

        // Ensure vec is large enough
        if self.materials.len() <= id {
            self.materials.resize(id + 1, MaterialDef::default());
        }

        self.materials[id] = material;
    }

    /// Get material definition by ID
    pub fn get(&self, id: u16) -> &MaterialDef {
        self.materials
            .get(id as usize)
            .unwrap_or(&self.materials[0])
    }

    /// Get color for a material
    pub fn get_color(&self, id: u16) -> [u8; 4] {
        self.get(id).color
    }
}

impl Default for Materials {
    fn default() -> Self {
        Self::new()
    }
}
