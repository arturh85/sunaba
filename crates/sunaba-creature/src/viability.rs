//! Movement viability testing for creature morphologies
//!
//! Tests whether a creature's morphology can produce effective movement
//! by analyzing morphological structure.

use glam::Vec2;

use crate::morphology::{CreatureMorphology, JointType};

/// Viability score for a creature morphology
#[derive(Debug, Clone)]
pub struct ViabilityScore {
    /// Overall viability score (0.0-1.0)
    pub overall: f32,
    /// Number of motorized joints that can produce rotation
    pub motor_count: usize,
    /// Whether the morphology has connected locomotion-capable limbs
    pub has_locomotion: bool,
    /// Range of motion achievable by motors (radians)
    pub range_of_motion: f32,
    /// Asymmetry factor (0.0 = symmetric, 1.0 = asymmetric)
    pub asymmetry: f32,
    /// Whether morphology is suitable for walking (has ground-contact limbs)
    pub suitable_for_walking: bool,
    /// Whether morphology is suitable for mining (has forward-facing limbs)
    pub suitable_for_mining: bool,
    /// Detailed diagnosis of issues
    pub issues: Vec<String>,
}

impl ViabilityScore {
    /// Check if this morphology is viable for basic movement
    pub fn is_viable(&self) -> bool {
        self.overall > 0.3 && self.motor_count > 0 && self.has_locomotion
    }

    /// Check if this morphology is excellent for movement
    pub fn is_excellent(&self) -> bool {
        self.overall > 0.7 && self.motor_count >= 2 && self.has_locomotion
    }
}

/// Analyze a morphology for movement viability
pub fn analyze_viability(morphology: &CreatureMorphology) -> ViabilityScore {
    let mut issues = Vec::new();

    // Count motorized joints (revolute)
    let motor_count = morphology
        .joints
        .iter()
        .filter(|j| matches!(j.joint_type, JointType::Revolute { .. }))
        .count();

    if motor_count == 0 {
        issues.push("No motorized joints - creature cannot move limbs".to_string());
    }

    // Calculate total range of motion
    let range_of_motion: f32 = morphology
        .joints
        .iter()
        .filter_map(|j| match &j.joint_type {
            JointType::Revolute {
                min_angle,
                max_angle,
            } => Some(max_angle - min_angle),
            JointType::Fixed => None,
        })
        .sum();

    if range_of_motion < std::f32::consts::PI / 4.0 {
        issues.push("Limited range of motion - joints constrained".to_string());
    }

    // Check for locomotion capability (limbs that can reach ground)
    let root_pos = morphology
        .body_parts
        .get(morphology.root_part_index)
        .map(|p| p.local_position)
        .unwrap_or(Vec2::ZERO);

    let mut has_downward_limbs = false;
    let mut has_lateral_limbs = false;
    let mut lowest_limb_y = 0.0f32;

    for (i, part) in morphology.body_parts.iter().enumerate() {
        if i == morphology.root_part_index {
            continue;
        }

        let offset = part.local_position - root_pos;

        // Check if this limb is below the root (can contact ground)
        if offset.y > 2.0 {
            // Y is positive downward in our coord system
            has_downward_limbs = true;
            lowest_limb_y = lowest_limb_y.max(offset.y);
        }

        // Check for lateral limbs (for walking)
        if offset.x.abs() > 3.0 {
            has_lateral_limbs = true;
        }
    }

    let has_locomotion = has_downward_limbs && motor_count > 0;

    if !has_downward_limbs {
        issues.push("No downward-facing limbs - cannot contact ground".to_string());
    }

    // Calculate asymmetry (symmetric creatures walk better)
    let asymmetry = calculate_asymmetry(morphology);

    if asymmetry > 0.7 {
        issues.push("Highly asymmetric body - may have difficulty walking".to_string());
    }

    // Determine locomotion capabilities
    let suitable_for_walking = has_downward_limbs
        && has_lateral_limbs
        && motor_count >= 2
        && range_of_motion > std::f32::consts::PI / 3.0;

    // Check for mining capability (forward-facing limbs)
    let mut has_forward_limbs = false;
    for part in &morphology.body_parts {
        let offset = part.local_position - root_pos;
        if offset.x.abs() > 5.0 && offset.y.abs() < 5.0 {
            has_forward_limbs = true;
            break;
        }
    }

    let suitable_for_mining = has_forward_limbs && motor_count >= 1;

    // Calculate overall viability score
    let motor_score = (motor_count as f32 / 4.0).min(1.0);
    let rom_score = (range_of_motion / std::f32::consts::PI).min(1.0);
    let symmetry_score = 1.0 - asymmetry;
    let locomotion_bonus = if has_locomotion { 0.3 } else { 0.0 };

    let overall =
        (motor_score * 0.3 + rom_score * 0.2 + symmetry_score * 0.2 + locomotion_bonus).min(1.0);

    ViabilityScore {
        overall,
        motor_count,
        has_locomotion,
        range_of_motion,
        asymmetry,
        suitable_for_walking,
        suitable_for_mining,
        issues,
    }
}

/// Calculate body asymmetry (0.0 = perfectly symmetric, 1.0 = completely asymmetric)
fn calculate_asymmetry(morphology: &CreatureMorphology) -> f32 {
    if morphology.body_parts.len() <= 1 {
        return 0.0;
    }

    let root_pos = morphology
        .body_parts
        .get(morphology.root_part_index)
        .map(|p| p.local_position)
        .unwrap_or(Vec2::ZERO);

    // Check horizontal symmetry around root
    let mut left_mass = 0.0f32;
    let mut right_mass = 0.0f32;

    for part in &morphology.body_parts {
        let offset = part.local_position - root_pos;
        let mass = std::f32::consts::PI * part.radius * part.radius * part.density;

        if offset.x < -1.0 {
            left_mass += mass;
        } else if offset.x > 1.0 {
            right_mass += mass;
        }
    }

    let total_mass = left_mass + right_mass;
    if total_mass < 0.01 {
        return 0.0;
    }

    (left_mass - right_mass).abs() / total_mass
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::morphology::CreatureMorphology;

    #[test]
    fn test_biped_viability() {
        let morphology = CreatureMorphology::test_biped();
        let score = analyze_viability(&morphology);

        // Biped should be viable
        assert!(score.is_viable(), "Biped should be viable");
        assert_eq!(score.motor_count, 2, "Biped should have 2 motors");
        assert!(
            score.has_locomotion,
            "Biped should have locomotion capability"
        );
    }

    #[test]
    fn test_quadruped_viability() {
        let morphology = CreatureMorphology::test_quadruped();
        let score = analyze_viability(&morphology);

        // Quadruped should be excellent
        assert!(score.is_viable(), "Quadruped should be viable");
        assert_eq!(score.motor_count, 4, "Quadruped should have 4 motors");
    }

    #[test]
    fn test_single_body_not_viable() {
        let morphology = CreatureMorphology {
            body_parts: vec![crate::morphology::BodyPart {
                local_position: Vec2::ZERO,
                radius: 5.0,
                density: 1.0,
                index: 0,
                is_wing: false,
            }],
            joints: vec![],
            root_part_index: 0,
            total_mass: 78.5,
        };

        let score = analyze_viability(&morphology);

        // Single body with no joints should not be viable
        assert!(!score.is_viable(), "Single body should not be viable");
        assert_eq!(score.motor_count, 0);
        assert!(!score.has_locomotion);
        assert!(!score.issues.is_empty());
    }

    #[test]
    fn test_symmetry_calculation() {
        let morphology = CreatureMorphology::test_biped();
        let score = analyze_viability(&morphology);

        // Biped should be relatively symmetric
        assert!(
            score.asymmetry < 0.5,
            "Biped should be roughly symmetric, got {}",
            score.asymmetry
        );
    }
}
