//! State change system
//!
//! Handles material state transitions based on temperature:
//! - Melting (solid → liquid)
//! - Freezing (liquid → solid)
//! - Boiling (liquid → gas)
//! - Condensing (gas → liquid)

use crate::simulation::MaterialDef;
use crate::world::Pixel;

/// System for checking and applying state changes
pub struct StateChangeSystem;

impl StateChangeSystem {
    /// Check if a pixel should change state based on temperature
    ///
    /// Returns true if the pixel was transformed to a different material
    pub fn check_state_change(pixel: &mut Pixel, material: &MaterialDef, temperature: f32) -> bool {
        // Check melting (solid → liquid, e.g., ice → water)
        if let Some(melt_temp) = material.melting_point
            && temperature >= melt_temp
            && let Some(melts_to) = material.melts_to
        {
            pixel.material_id = melts_to;
            return true;
        }

        // Check boiling (liquid → gas, e.g., water → steam)
        if let Some(boil_temp) = material.boiling_point
            && temperature >= boil_temp
            && let Some(boils_to) = material.boils_to
        {
            pixel.material_id = boils_to;
            return true;
        }

        // Check freezing/condensing (liquid/gas → solid, e.g., water → ice, steam → water)
        if let Some(freeze_temp) = material.freezing_point
            && temperature <= freeze_temp
            && let Some(freezes_to) = material.freezes_to
        {
            pixel.material_id = freezes_to;
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::{MaterialId, MaterialType};

    #[test]
    fn test_melting() {
        // Ice melts to water at 0°C
        let ice = MaterialDef {
            id: MaterialId::ICE,
            name: "ice".to_string(),
            material_type: MaterialType::Solid,
            melting_point: Some(0.0),
            melts_to: Some(MaterialId::WATER),
            ..Default::default()
        };

        let mut pixel = Pixel::new(MaterialId::ICE);

        // Below melting point - no change
        assert!(!StateChangeSystem::check_state_change(
            &mut pixel, &ice, -10.0
        ));
        assert_eq!(pixel.material_id, MaterialId::ICE);

        // At melting point - should melt
        assert!(StateChangeSystem::check_state_change(&mut pixel, &ice, 0.0));
        assert_eq!(pixel.material_id, MaterialId::WATER);
    }

    #[test]
    fn test_boiling() {
        // Water boils to steam at 100°C
        let water = MaterialDef {
            id: MaterialId::WATER,
            name: "water".to_string(),
            material_type: MaterialType::Liquid,
            boiling_point: Some(100.0),
            boils_to: Some(MaterialId::STEAM),
            ..Default::default()
        };

        let mut pixel = Pixel::new(MaterialId::WATER);

        // Below boiling point - no change
        assert!(!StateChangeSystem::check_state_change(
            &mut pixel, &water, 50.0
        ));
        assert_eq!(pixel.material_id, MaterialId::WATER);

        // At boiling point - should boil
        assert!(StateChangeSystem::check_state_change(
            &mut pixel, &water, 100.0
        ));
        assert_eq!(pixel.material_id, MaterialId::STEAM);
    }

    #[test]
    fn test_freezing() {
        // Water freezes to ice at 0°C
        let water = MaterialDef {
            id: MaterialId::WATER,
            name: "water".to_string(),
            material_type: MaterialType::Liquid,
            freezing_point: Some(0.0),
            freezes_to: Some(MaterialId::ICE),
            ..Default::default()
        };

        let mut pixel = Pixel::new(MaterialId::WATER);

        // Above freezing point - no change
        assert!(!StateChangeSystem::check_state_change(
            &mut pixel, &water, 10.0
        ));
        assert_eq!(pixel.material_id, MaterialId::WATER);

        // At freezing point - should freeze
        assert!(StateChangeSystem::check_state_change(
            &mut pixel, &water, 0.0
        ));
        assert_eq!(pixel.material_id, MaterialId::ICE);
    }
}
