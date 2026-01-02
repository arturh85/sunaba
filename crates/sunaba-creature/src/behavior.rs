//! GOAP behavior planning for creatures
//!
//! Implements Goal-Oriented Action Planning for high-level decision making.

use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use super::sensors::SensoryInput;

/// High-level needs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatureNeeds {
    pub hunger: f32,       // 0.0 = satisfied, 1.0 = starving
    pub threat_level: f32, // 0.0 = safe, 1.0 = extreme danger
    pub energy: f32,       // 0.0 = exhausted, 1.0 = full
}

impl Default for CreatureNeeds {
    fn default() -> Self {
        Self::new()
    }
}

impl CreatureNeeds {
    /// Create default needs
    pub fn new() -> Self {
        Self {
            hunger: 0.0,
            threat_level: 0.0,
            energy: 1.0,
        }
    }

    /// Update needs based on sensory input
    pub fn update(&mut self, sensory: &SensoryInput, hunger_value: f32) {
        self.hunger = hunger_value;
        self.threat_level = if sensory.nearest_threat.is_some() {
            0.8
        } else {
            0.0
        };
    }

    /// Get most urgent need
    pub fn most_urgent(&self) -> NeedType {
        if self.threat_level > 0.5 {
            NeedType::Safety
        } else if self.hunger > 0.7 {
            NeedType::Hunger
        } else if self.energy < 0.3 {
            NeedType::Energy
        } else {
            NeedType::Exploration
        }
    }
}

/// Type of need
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NeedType {
    Safety,
    Hunger,
    Energy,
    Exploration,
}

/// World state properties (for GOAP)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WorldProperty {
    HasFood,
    NearFood,
    IsHungry,
    InDanger,
    IsSafe,
    HasEnergy,
    AtDestination,
}

/// High-level actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CreatureAction {
    MoveTo { target: Vec2 },
    Wander { duration: f32 },
    Eat { position: Vec2, material_id: u16 },
    Mine { position: Vec2, material_id: u16 },
    Build { position: Vec2, material_id: u16 },
    Flee { from: Vec2 },
    Rest { duration: f32 },
}

impl CreatureAction {
    /// Get action duration
    pub fn duration(&self) -> f32 {
        match self {
            Self::MoveTo { .. } => 2.0,
            Self::Wander { duration } => *duration,
            Self::Eat { .. } => 1.0,
            Self::Mine { .. } => 2.0,
            Self::Build { .. } => 1.5,
            Self::Flee { .. } => 3.0,
            Self::Rest { duration } => *duration,
        }
    }
}

impl std::fmt::Display for CreatureAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MoveTo { .. } => write!(f, "Moving"),
            Self::Wander { .. } => write!(f, "Wandering"),
            Self::Eat { .. } => write!(f, "Eating"),
            Self::Mine { .. } => write!(f, "Mining"),
            Self::Build { .. } => write!(f, "Building"),
            Self::Flee { .. } => write!(f, "Fleeing"),
            Self::Rest { .. } => write!(f, "Resting"),
        }
    }
}

/// Action definition (preconditions + effects)
#[derive(Debug, Clone)]
pub struct ActionDef {
    pub name: String,
    pub preconditions: Vec<WorldProperty>,
    pub effects: Vec<WorldProperty>,
    pub cost: f32,
}

/// GOAP planner
pub struct GoalPlanner {
    pub current_goal: Vec<WorldProperty>,
    pub action_plan: VecDeque<CreatureAction>,
    pub world_state: Vec<WorldProperty>,
}

impl Default for GoalPlanner {
    fn default() -> Self {
        Self::new()
    }
}

impl GoalPlanner {
    /// Create new planner
    pub fn new() -> Self {
        Self {
            current_goal: Vec::new(),
            action_plan: VecDeque::new(),
            world_state: Vec::new(),
        }
    }

    /// Update goal based on needs
    pub fn update_goal(&mut self, needs: &CreatureNeeds) {
        let need_type = needs.most_urgent();
        self.current_goal = match need_type {
            NeedType::Safety => vec![WorldProperty::IsSafe],
            NeedType::Hunger => vec![WorldProperty::HasFood],
            NeedType::Energy => vec![WorldProperty::HasEnergy],
            NeedType::Exploration => vec![WorldProperty::AtDestination],
        };
    }

    /// Evaluate current world state from sensory input
    pub fn evaluate_world_state(&mut self, sensory: &SensoryInput, hunger: f32) {
        self.world_state.clear();

        if hunger < 0.3 {
            self.world_state.push(WorldProperty::HasFood);
        }
        if hunger > 0.7 {
            self.world_state.push(WorldProperty::IsHungry);
        }
        if sensory.nearest_threat.is_none() {
            self.world_state.push(WorldProperty::IsSafe);
        }
        if sensory.nearest_threat.is_some() {
            self.world_state.push(WorldProperty::InDanger);
        }
        if sensory.nearest_food.is_some() {
            self.world_state.push(WorldProperty::NearFood);
        }
    }

    /// Plan actions to achieve goal (greedy algorithm for Phase 6)
    pub fn plan(&mut self, sensory: &SensoryInput, _position: Vec2) {
        // Simple greedy planning based on current goal
        self.action_plan.clear();

        if self.current_goal.contains(&WorldProperty::IsSafe) {
            if let Some(threat_pos) = sensory.nearest_threat {
                self.action_plan
                    .push_back(CreatureAction::Flee { from: threat_pos });
            }
        } else if self.current_goal.contains(&WorldProperty::HasFood) {
            if let Some(food_pos) = sensory.nearest_food {
                self.action_plan
                    .push_back(CreatureAction::MoveTo { target: food_pos });
                // Assume food material ID is determinable from world
                self.action_plan.push_back(CreatureAction::Eat {
                    position: food_pos,
                    material_id: 0, // Placeholder
                });
            } else {
                self.action_plan
                    .push_back(CreatureAction::Wander { duration: 5.0 });
            }
        } else {
            // Default: wander
            self.action_plan
                .push_back(CreatureAction::Wander { duration: 5.0 });
        }
    }

    /// Get next action to execute
    pub fn next_action(&mut self) -> Option<CreatureAction> {
        self.action_plan.pop_front()
    }

    /// Check if plan is still valid
    pub fn is_plan_valid(&self, sensory: &SensoryInput) -> bool {
        // Simple validation: if threat appeared or disappeared, re-plan
        let has_threat = sensory.nearest_threat.is_some();
        let plan_addresses_threat = self
            .action_plan
            .iter()
            .any(|a| matches!(a, CreatureAction::Flee { .. }));

        if has_threat != plan_addresses_threat {
            return false;
        }

        !self.action_plan.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensors::ChemicalGradient;

    #[test]
    fn test_needs_creation() {
        let needs = CreatureNeeds::new();
        assert_eq!(needs.hunger, 0.0);
        assert_eq!(needs.threat_level, 0.0);
        assert_eq!(needs.energy, 1.0);
    }

    #[test]
    fn test_needs_most_urgent() {
        let mut needs = CreatureNeeds::new();
        assert_eq!(needs.most_urgent(), NeedType::Exploration);

        needs.threat_level = 0.9;
        assert_eq!(needs.most_urgent(), NeedType::Safety);

        needs.threat_level = 0.0;
        needs.hunger = 0.8;
        assert_eq!(needs.most_urgent(), NeedType::Hunger);

        needs.hunger = 0.0;
        needs.energy = 0.2;
        assert_eq!(needs.most_urgent(), NeedType::Energy);
    }

    #[test]
    fn test_needs_update() {
        let mut needs = CreatureNeeds::new();
        let sensory = SensoryInput {
            raycasts: vec![],
            contact_materials: vec![],
            gradients: ChemicalGradient {
                food: 0.5,
                danger: 0.0,
                mate: 0.0,
            },
            nearest_food: Some(Vec2::new(10.0, 10.0)),
            nearest_threat: Some(Vec2::new(5.0, 5.0)),
            food_direction: Some(Vec2::new(1.0, 0.0)),
            food_distance: 0.2,
        };

        needs.update(&sensory, 0.6);

        assert_eq!(needs.hunger, 0.6);
        assert_eq!(needs.threat_level, 0.8); // Threat present
    }

    #[test]
    fn test_action_duration() {
        assert_eq!(CreatureAction::Wander { duration: 5.0 }.duration(), 5.0);
        assert_eq!(
            CreatureAction::Eat {
                position: Vec2::ZERO,
                material_id: 1
            }
            .duration(),
            1.0
        );
        assert_eq!(
            CreatureAction::MoveTo { target: Vec2::ZERO }.duration(),
            2.0
        );
    }

    #[test]
    fn test_planner_creation() {
        let planner = GoalPlanner::new();
        assert!(planner.current_goal.is_empty());
        assert!(planner.action_plan.is_empty());
        assert!(planner.world_state.is_empty());
    }

    #[test]
    fn test_planner_update_goal() {
        let mut planner = GoalPlanner::new();
        let mut needs = CreatureNeeds::new();

        // Test safety goal
        needs.threat_level = 0.9;
        planner.update_goal(&needs);
        assert_eq!(planner.current_goal, vec![WorldProperty::IsSafe]);

        // Test hunger goal
        needs.threat_level = 0.0;
        needs.hunger = 0.8;
        planner.update_goal(&needs);
        assert_eq!(planner.current_goal, vec![WorldProperty::HasFood]);
    }

    #[test]
    fn test_planner_evaluate_world_state() {
        let mut planner = GoalPlanner::new();
        let sensory = SensoryInput {
            raycasts: vec![],
            contact_materials: vec![],
            gradients: ChemicalGradient {
                food: 0.5,
                danger: 0.0,
                mate: 0.0,
            },
            nearest_food: Some(Vec2::new(10.0, 10.0)),
            nearest_threat: None,
            food_direction: Some(Vec2::new(1.0, 0.0)),
            food_distance: 0.2,
        };

        planner.evaluate_world_state(&sensory, 0.2);

        assert!(planner.world_state.contains(&WorldProperty::HasFood));
        assert!(planner.world_state.contains(&WorldProperty::IsSafe));
        assert!(planner.world_state.contains(&WorldProperty::NearFood));
    }

    #[test]
    fn test_planner_flee_action() {
        let mut planner = GoalPlanner::new();
        planner.current_goal = vec![WorldProperty::IsSafe];

        let sensory = SensoryInput {
            raycasts: vec![],
            contact_materials: vec![],
            gradients: ChemicalGradient {
                food: 0.0,
                danger: 0.8,
                mate: 0.0,
            },
            nearest_food: None,
            nearest_threat: Some(Vec2::new(5.0, 5.0)),
            food_direction: None,
            food_distance: 1.0,
        };

        planner.plan(&sensory, Vec2::ZERO);

        let action = planner.next_action();
        assert!(matches!(action, Some(CreatureAction::Flee { .. })));
    }

    #[test]
    fn test_planner_eat_action() {
        let mut planner = GoalPlanner::new();
        planner.current_goal = vec![WorldProperty::HasFood];

        let sensory = SensoryInput {
            raycasts: vec![],
            contact_materials: vec![],
            gradients: ChemicalGradient {
                food: 0.8,
                danger: 0.0,
                mate: 0.0,
            },
            nearest_food: Some(Vec2::new(10.0, 10.0)),
            nearest_threat: None,
            food_direction: Some(Vec2::new(1.0, 0.0)),
            food_distance: 0.2,
        };

        planner.plan(&sensory, Vec2::ZERO);

        // Should have MoveTo then Eat
        assert_eq!(planner.action_plan.len(), 2);
        let action1 = planner.next_action();
        assert!(matches!(action1, Some(CreatureAction::MoveTo { .. })));
        let action2 = planner.next_action();
        assert!(matches!(action2, Some(CreatureAction::Eat { .. })));
    }

    #[test]
    fn test_planner_wander_when_no_food() {
        let mut planner = GoalPlanner::new();
        planner.current_goal = vec![WorldProperty::HasFood];

        let sensory = SensoryInput {
            raycasts: vec![],
            contact_materials: vec![],
            gradients: ChemicalGradient {
                food: 0.0,
                danger: 0.0,
                mate: 0.0,
            },
            nearest_food: None,
            nearest_threat: None,
            food_direction: None,
            food_distance: 1.0,
        };

        planner.plan(&sensory, Vec2::ZERO);

        let action = planner.next_action();
        assert!(matches!(action, Some(CreatureAction::Wander { .. })));
    }

    #[test]
    fn test_plan_validation() {
        let mut planner = GoalPlanner::new();
        planner
            .action_plan
            .push_back(CreatureAction::Wander { duration: 5.0 });

        // Plan with no threat, sensory with no threat - valid
        let sensory_safe = SensoryInput {
            raycasts: vec![],
            contact_materials: vec![],
            gradients: ChemicalGradient {
                food: 0.0,
                danger: 0.0,
                mate: 0.0,
            },
            nearest_food: None,
            nearest_threat: None,
            food_direction: None,
            food_distance: 1.0,
        };
        assert!(planner.is_plan_valid(&sensory_safe));

        // Threat appears - plan invalid
        let sensory_danger = SensoryInput {
            raycasts: vec![],
            contact_materials: vec![],
            gradients: ChemicalGradient {
                food: 0.0,
                danger: 0.8,
                mate: 0.0,
            },
            nearest_food: None,
            nearest_threat: Some(Vec2::new(5.0, 5.0)),
            food_direction: None,
            food_distance: 1.0,
        };
        assert!(!planner.is_plan_valid(&sensory_danger));
    }
}
