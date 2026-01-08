//! Biome transition system - physics-stable blending between biomes
//!
//! Key challenge: Powder materials (sand) can't sit at angles in a falling-sand simulation.
//! Solution: Stability-aware blending that considers material physics properties.

use crate::simulation::{MaterialId, Materials};
use crate::world::biome::{BiomeDefinition, BiomeType};
use fastnoise_lite::FastNoiseLite;

/// Position context for biome blending
#[derive(Debug, Clone, Copy)]
pub struct BlendContext {
    pub world_x: i32,
    pub world_y: i32,
}

/// Biome transition configuration
pub struct BiomeTransition {
    /// Transition width in pixels (default: 32)
    pub transition_width: i32,
    /// Noise for natural-looking boundaries
    boundary_noise: FastNoiseLite,
    /// Whether to enforce stability constraints
    pub enforce_stability: bool,
}

/// Blend mode for biome transitions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    /// No blending - sharp biome boundaries
    Sharp,
    /// Noise-based gradient transition
    Gradient { noise_scale_percent: u32 }, // 0-100, controls boundary roughness
    /// Stability-aware layering (heavier materials below)
    StableLayer,
}

/// Material stability classification for physics-aware transitions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialStability {
    /// Solid, self-supporting (stone, wood, dirt when compacted)
    Solid,
    /// Powder, falls freely (sand, gravel)
    Powder,
    /// Liquid, flows (water, lava)
    Liquid,
    /// Gas, rises (air, steam)
    Gas,
}

impl BiomeTransition {
    /// Create a new biome transition system
    pub fn new(seed: i32) -> Self {
        let mut boundary_noise = FastNoiseLite::with_seed(seed);
        boundary_noise.set_noise_type(Some(fastnoise_lite::NoiseType::OpenSimplex2));
        boundary_noise.set_frequency(Some(0.1));
        boundary_noise.set_fractal_type(Some(fastnoise_lite::FractalType::FBm));
        boundary_noise.set_fractal_octaves(Some(3));
        boundary_noise.set_fractal_lacunarity(Some(2.0));
        boundary_noise.set_fractal_gain(Some(0.5));

        Self {
            transition_width: 32,
            boundary_noise,
            enforce_stability: true,
        }
    }

    /// Calculate blend factor between two biomes at a given position
    ///
    /// Returns (biome1_weight, biome2_weight) where weights sum to 1.0
    /// The blend is centered at `boundary_x` with `transition_width` on each side.
    pub fn calculate_blend_factor(
        &self,
        world_x: i32,
        world_y: i32,
        boundary_x: i32,
        mode: BlendMode,
    ) -> (f32, f32) {
        match mode {
            BlendMode::Sharp => {
                // Hard cutoff at boundary
                if world_x < boundary_x {
                    (1.0, 0.0)
                } else {
                    (0.0, 1.0)
                }
            }
            BlendMode::Gradient {
                noise_scale_percent,
            } => {
                // Smooth gradient with noise
                let distance = (world_x - boundary_x) as f32;
                let half_width = self.transition_width as f32 / 2.0;

                // Base linear blend
                let base_blend =
                    ((distance + half_width) / (self.transition_width as f32)).clamp(0.0, 1.0);

                // Add noise for natural boundaries
                let noise_scale = noise_scale_percent as f32 / 100.0;
                let noise = self
                    .boundary_noise
                    .get_noise_2d(world_x as f32, world_y as f32);
                let noise_offset = noise * noise_scale * 0.5; // ±noise_scale/2

                let blend = (base_blend + noise_offset).clamp(0.0, 1.0);

                (1.0 - blend, blend)
            }
            BlendMode::StableLayer => {
                // Similar to gradient, but will be used with stability-aware material selection
                let distance = (world_x - boundary_x) as f32;
                let half_width = self.transition_width as f32 / 2.0;
                let blend =
                    ((distance + half_width) / (self.transition_width as f32)).clamp(0.0, 1.0);

                (1.0 - blend, blend)
            }
        }
    }

    /// Blend materials between two biomes, considering physics stability
    ///
    /// This is the main entry point for biome transition logic.
    pub fn blend_material(
        &self,
        ctx: BlendContext,
        depth: i32,
        biome1: &BiomeDefinition,
        biome2: &BiomeDefinition,
        blend_factor: (f32, f32),
        materials: &Materials,
    ) -> u16 {
        let (weight1, weight2) = blend_factor;

        // Get candidate materials from both biomes
        let mat1 = self.get_biome_material_at_depth(biome1, depth);
        let mat2 = self.get_biome_material_at_depth(biome2, depth);

        // If same material, no blending needed
        if mat1 == mat2 {
            return mat1;
        }

        // If one side dominates (>90%), use that material
        if weight1 > 0.9 {
            return mat1;
        }
        if weight2 > 0.9 {
            return mat2;
        }

        // Physics-aware blending
        if self.enforce_stability {
            self.blend_stable(ctx, mat1, mat2, weight1, weight2, materials)
        } else {
            // Simple probabilistic blend
            if weight1 > 0.5 { mat1 } else { mat2 }
        }
    }

    /// Get material at a specific depth within a biome
    fn get_biome_material_at_depth(&self, biome: &BiomeDefinition, depth: i32) -> u16 {
        if depth <= 1 {
            biome.surface_material
        } else if depth <= biome.stone_depth {
            biome.subsurface_material
        } else {
            MaterialId::STONE
        }
    }

    /// Stability-aware material blending
    ///
    /// Ensures powder materials don't create unstable configurations
    fn blend_stable(
        &self,
        ctx: BlendContext,
        mat1: u16,
        mat2: u16,
        weight1: f32,
        weight2: f32,
        materials: &Materials,
    ) -> u16 {
        let stability1 = classify_material_stability(mat1, materials);
        let stability2 = classify_material_stability(mat2, materials);

        // Use noise to create natural boundaries
        let noise = self
            .boundary_noise
            .get_noise_2d(ctx.world_x as f32, ctx.world_y as f32);
        let threshold = 0.5 - (weight1 - 0.5); // Bias threshold based on weights

        match (stability1, stability2) {
            // Both solid: simple probabilistic blend
            (MaterialStability::Solid, MaterialStability::Solid) => {
                if noise > threshold {
                    mat1
                } else {
                    mat2
                }
            }

            // Solid vs Powder: prefer solid in transition zone to prevent powder collapse
            (MaterialStability::Solid, MaterialStability::Powder) => {
                // Bias heavily toward solid material in transition
                if weight1 > 0.3 || noise > threshold {
                    mat1
                } else {
                    mat2
                }
            }
            (MaterialStability::Powder, MaterialStability::Solid) => {
                if weight2 > 0.3 || noise < threshold {
                    mat2
                } else {
                    mat1
                }
            }

            // Both powder: allow mixing but use noise for natural patches
            (MaterialStability::Powder, MaterialStability::Powder) => {
                if noise > threshold {
                    mat1
                } else {
                    mat2
                }
            }

            // Liquid transitions: sharp boundaries work better for liquids
            (_, MaterialStability::Liquid) | (MaterialStability::Liquid, _) => {
                if weight1 > 0.6 {
                    mat1
                } else {
                    mat2
                }
            }

            // Gas: always use gas if present (air pockets)
            (MaterialStability::Gas, _) => mat1,
            (_, MaterialStability::Gas) => mat2,
        }
    }
}

/// Classify material stability for transition logic
pub fn classify_material_stability(material_id: u16, materials: &Materials) -> MaterialStability {
    // Handle common cases first for performance
    if material_id == MaterialId::AIR {
        return MaterialStability::Gas;
    }
    if material_id == MaterialId::WATER {
        return MaterialStability::Liquid;
    }
    if material_id == MaterialId::LAVA {
        return MaterialStability::Liquid;
    }

    // Check material properties
    let def = materials.get(material_id);
    if def.id == material_id {
        match def.name.as_str() {
            // Explicitly classify common materials
            "sand" => MaterialStability::Powder,
            "stone" | "dirt" | "wood" | "coal_ore" | "iron_ore" | "copper_ore" | "gold_ore"
            | "bedrock" | "metal" | "glass" | "bone" | "ash" | "ice" => MaterialStability::Solid,
            "oil" | "acid" => MaterialStability::Liquid,
            "steam" | "smoke" | "poison_gas" => MaterialStability::Gas,
            _ => {
                // Fallback: classify by material type/behavior
                // For now, assume most materials are solid
                // TODO: Add material property flags for stability classification
                MaterialStability::Solid
            }
        }
    } else {
        // Unknown material, treat as solid to be safe
        MaterialStability::Solid
    }
}

/// Helper to find biome boundaries for a given X coordinate
///
/// Scans horizontally to detect where biomes change. Returns the X coordinate
/// of the nearest boundary and the biomes on either side.
pub fn find_biome_boundary<F>(
    world_x: i32,
    get_biome: F,
    search_radius: i32,
) -> Option<(i32, BiomeType, BiomeType)>
where
    F: Fn(i32) -> BiomeType,
{
    let center_biome = get_biome(world_x);

    // Search left and right for biome change
    for offset in 1..=search_radius {
        // Check right
        let right_biome = get_biome(world_x + offset);
        if right_biome != center_biome {
            return Some((world_x + offset, center_biome, right_biome));
        }

        // Check left
        let left_biome = get_biome(world_x - offset);
        if left_biome != center_biome {
            return Some((world_x - offset, left_biome, center_biome));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sharp_blend() {
        let transition = BiomeTransition::new(12345);
        let boundary_x = 100;

        // Left of boundary should be 100% biome1
        let (w1, w2) = transition.calculate_blend_factor(50, 0, boundary_x, BlendMode::Sharp);
        assert_eq!(w1, 1.0);
        assert_eq!(w2, 0.0);

        // Right of boundary should be 100% biome2
        let (w1, w2) = transition.calculate_blend_factor(150, 0, boundary_x, BlendMode::Sharp);
        assert_eq!(w1, 0.0);
        assert_eq!(w2, 1.0);
    }

    #[test]
    fn test_gradient_blend() {
        let transition = BiomeTransition::new(12345);
        let boundary_x = 100;

        // Center of boundary should be roughly 50/50 (allowing for noise)
        let (w1, w2) = transition.calculate_blend_factor(
            boundary_x,
            0,
            boundary_x,
            BlendMode::Gradient {
                noise_scale_percent: 10,
            },
        );
        assert!((w1 - 0.5).abs() < 0.2, "Center blend should be ~0.5");
        assert!((w2 - 0.5).abs() < 0.2, "Center blend should be ~0.5");
        assert!((w1 + w2 - 1.0).abs() < 0.01, "Weights should sum to 1.0");

        // Far left should be mostly biome1
        let (w1, _) = transition.calculate_blend_factor(
            boundary_x - 50,
            0,
            boundary_x,
            BlendMode::Gradient {
                noise_scale_percent: 10,
            },
        );
        assert!(w1 > 0.8, "Far left should be mostly biome1");

        // Far right should be mostly biome2
        let (_, w2) = transition.calculate_blend_factor(
            boundary_x + 50,
            0,
            boundary_x,
            BlendMode::Gradient {
                noise_scale_percent: 10,
            },
        );
        assert!(w2 > 0.8, "Far right should be mostly biome2");
    }

    #[test]
    fn test_stability_classification() {
        let materials = Materials::new();

        assert_eq!(
            classify_material_stability(MaterialId::AIR, &materials),
            MaterialStability::Gas
        );
        assert_eq!(
            classify_material_stability(MaterialId::WATER, &materials),
            MaterialStability::Liquid
        );
        assert_eq!(
            classify_material_stability(MaterialId::SAND, &materials),
            MaterialStability::Powder
        );
        assert_eq!(
            classify_material_stability(MaterialId::STONE, &materials),
            MaterialStability::Solid
        );
    }

    #[test]
    fn test_find_boundary() {
        // Create a mock biome function: Plains from 0-99, Desert from 100+
        let get_biome = |x: i32| {
            if x < 100 {
                BiomeType::Plains
            } else {
                BiomeType::Desert
            }
        };

        // Search from center of plains
        let result = find_biome_boundary(50, get_biome, 60);
        assert!(result.is_some());
        let (boundary_x, biome1, biome2) = result.unwrap();
        assert_eq!(boundary_x, 100);
        assert_eq!(biome1, BiomeType::Plains);
        assert_eq!(biome2, BiomeType::Desert);

        // Search from far left (no boundary in range)
        let result = find_biome_boundary(10, get_biome, 30);
        assert!(result.is_none());
    }
}
