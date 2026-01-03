//! Creature execution state machine.
//!
//! This module provides a compile-time checked state machine for tracking
//! creature execution state. It complements the GOAP planner:
//! - GOAP decides *what* action to take
//! - FSM tracks *what the creature is currently doing*

use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Execution state enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ExecutionState {
    /// Creature is idle, waiting for next action
    #[default]
    Idle,
    /// Creature is moving toward a target
    Moving,
    /// Creature is eating at a position
    Eating,
    /// Creature is mining at a position
    Mining,
    /// Creature is building at a position
    Building,
    /// Creature is fleeing from a threat
    Fleeing,
    /// Creature is dead (terminal state)
    Dead,
}

impl ExecutionState {
    /// Check if this state can be interrupted by a flee command.
    pub fn can_flee(&self) -> bool {
        matches!(
            self,
            ExecutionState::Idle
                | ExecutionState::Moving
                | ExecutionState::Eating
                | ExecutionState::Mining
                | ExecutionState::Building
        )
    }

    /// Check if this state is a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, ExecutionState::Dead)
    }
}

/// Input events for state transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionInput {
    /// Start moving toward a target
    StartMoving,
    /// Start eating
    StartEating,
    /// Start mining
    StartMining,
    /// Start building
    StartBuilding,
    /// Start fleeing from threat
    StartFleeing,
    /// Arrived at movement target
    Arrive,
    /// Finished current action (eating, mining, building)
    Finish,
    /// Action was interrupted
    Interrupt,
    /// Threat has passed, safe to stop fleeing
    Safe,
    /// Creature has died
    Die,
}

/// State-specific data for the current execution.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ExecutionStateData {
    /// No state-specific data
    #[default]
    Idle,
    /// Moving toward a target position
    Moving { target: Vec2 },
    /// Eating at a position
    Eating {
        position: Vec2,
        nutrition_remaining: f32,
    },
    /// Mining at a position
    Mining { position: Vec2, progress: f32 },
    /// Building at a position
    Building { position: Vec2, material_id: u16 },
    /// Fleeing from a threat
    Fleeing { from: Vec2 },
    /// Dead
    Dead,
}

/// Transition table for the state machine.
/// Returns Some(new_state) if transition is valid, None if invalid.
fn transition(current: ExecutionState, input: ExecutionInput) -> Option<ExecutionState> {
    use ExecutionInput::*;
    use ExecutionState::*;

    match (current, input) {
        // From Idle
        (Idle, StartMoving) => Some(Moving),
        (Idle, StartEating) => Some(Eating),
        (Idle, StartMining) => Some(Mining),
        (Idle, StartBuilding) => Some(Building),
        (Idle, StartFleeing) => Some(Fleeing),
        (Idle, Die) => Some(Dead),

        // From Moving
        (Moving, Arrive) => Some(Idle),
        (Moving, StartEating) => Some(Eating),
        (Moving, StartMining) => Some(Mining),
        (Moving, StartBuilding) => Some(Building),
        (Moving, StartFleeing) => Some(Fleeing),
        (Moving, Die) => Some(Dead),

        // From Eating
        (Eating, Finish) => Some(Idle),
        (Eating, Interrupt) => Some(Idle),
        (Eating, StartFleeing) => Some(Fleeing),
        (Eating, Die) => Some(Dead),

        // From Mining
        (Mining, Finish) => Some(Idle),
        (Mining, Interrupt) => Some(Idle),
        (Mining, StartFleeing) => Some(Fleeing),
        (Mining, Die) => Some(Dead),

        // From Building
        (Building, Finish) => Some(Idle),
        (Building, Interrupt) => Some(Idle),
        (Building, StartFleeing) => Some(Fleeing),
        (Building, Die) => Some(Dead),

        // From Fleeing
        (Fleeing, Safe) => Some(Idle),
        (Fleeing, Die) => Some(Dead),

        // Dead is terminal - no transitions allowed
        (Dead, _) => None,

        // All other transitions are invalid
        _ => None,
    }
}

/// Wrapper around the FSM that provides additional functionality.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreatureExecutionState {
    /// Current state
    state: ExecutionState,
    /// Time spent in current state (seconds)
    state_timer: f32,
    /// State-specific data
    data: ExecutionStateData,
}

impl CreatureExecutionState {
    /// Create a new execution state starting at Idle.
    pub fn new() -> Self {
        Self {
            state: ExecutionState::Idle,
            state_timer: 0.0,
            data: ExecutionStateData::Idle,
        }
    }

    /// Get the current execution state.
    pub fn current(&self) -> ExecutionState {
        self.state
    }

    /// Attempt a state transition.
    ///
    /// Returns `true` if the transition was valid and occurred,
    /// `false` if the transition was invalid from the current state.
    pub fn transition(&mut self, input: ExecutionInput) -> bool {
        if let Some(new_state) = transition(self.state, input) {
            self.state = new_state;
            self.state_timer = 0.0;
            true
        } else {
            log::trace!(
                "Invalid transition: {:?} + {:?} -> rejected",
                self.state,
                input
            );
            false
        }
    }

    /// Update the state timer.
    pub fn update(&mut self, dt: f32) {
        self.state_timer += dt;
    }

    /// Get time spent in current state.
    pub fn time_in_state(&self) -> f32 {
        self.state_timer
    }

    /// Set state-specific data.
    pub fn set_data(&mut self, data: ExecutionStateData) {
        self.data = data;
    }

    /// Get reference to state-specific data.
    pub fn data(&self) -> &ExecutionStateData {
        &self.data
    }

    /// Get mutable reference to state-specific data.
    pub fn data_mut(&mut self) -> &mut ExecutionStateData {
        &mut self.data
    }

    /// Check if the creature is dead.
    pub fn is_dead(&self) -> bool {
        self.state == ExecutionState::Dead
    }

    /// Check if the creature is idle (ready for new action).
    pub fn is_idle(&self) -> bool {
        self.state == ExecutionState::Idle
    }

    /// Check if the creature is moving.
    pub fn is_moving(&self) -> bool {
        self.state == ExecutionState::Moving
    }

    /// Check if the creature is fleeing.
    pub fn is_fleeing(&self) -> bool {
        self.state == ExecutionState::Fleeing
    }

    /// Check if the creature can be interrupted (for priority actions).
    pub fn can_interrupt(&self) -> bool {
        matches!(
            self.state,
            ExecutionState::Moving
                | ExecutionState::Eating
                | ExecutionState::Mining
                | ExecutionState::Building
        )
    }

    /// Force transition to fleeing state (for threat response).
    /// Returns true if transition succeeded.
    pub fn force_flee(&mut self, from: Vec2) -> bool {
        if self.transition(ExecutionInput::StartFleeing) {
            self.set_data(ExecutionStateData::Fleeing { from });
            true
        } else {
            false
        }
    }

    /// Transition to moving state with target.
    pub fn start_moving(&mut self, target: Vec2) -> bool {
        if self.transition(ExecutionInput::StartMoving) {
            self.set_data(ExecutionStateData::Moving { target });
            true
        } else {
            false
        }
    }

    /// Transition to eating state.
    pub fn start_eating(&mut self, position: Vec2, nutrition: f32) -> bool {
        if self.transition(ExecutionInput::StartEating) {
            self.set_data(ExecutionStateData::Eating {
                position,
                nutrition_remaining: nutrition,
            });
            true
        } else {
            false
        }
    }

    /// Transition to mining state.
    pub fn start_mining(&mut self, position: Vec2) -> bool {
        if self.transition(ExecutionInput::StartMining) {
            self.set_data(ExecutionStateData::Mining {
                position,
                progress: 0.0,
            });
            true
        } else {
            false
        }
    }

    /// Transition to building state.
    pub fn start_building(&mut self, position: Vec2, material_id: u16) -> bool {
        if self.transition(ExecutionInput::StartBuilding) {
            self.set_data(ExecutionStateData::Building {
                position,
                material_id,
            });
            true
        } else {
            false
        }
    }

    /// Mark current action as finished.
    pub fn finish(&mut self) -> bool {
        self.transition(ExecutionInput::Finish)
    }

    /// Mark that creature arrived at destination.
    pub fn arrive(&mut self) -> bool {
        self.transition(ExecutionInput::Arrive)
    }

    /// Mark that creature is now safe from threat.
    pub fn safe(&mut self) -> bool {
        self.transition(ExecutionInput::Safe)
    }

    /// Mark creature as dead.
    pub fn die(&mut self) -> bool {
        self.transition(ExecutionInput::Die)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let state = CreatureExecutionState::new();
        assert_eq!(state.current(), ExecutionState::Idle);
        assert!(state.is_idle());
    }

    #[test]
    fn test_valid_transitions() {
        let mut state = CreatureExecutionState::new();

        // Idle -> Moving
        assert!(state.start_moving(Vec2::new(10.0, 20.0)));
        assert_eq!(state.current(), ExecutionState::Moving);

        // Moving -> Eating
        assert!(state.start_eating(Vec2::new(10.0, 20.0), 50.0));
        assert_eq!(state.current(), ExecutionState::Eating);

        // Eating -> Idle (finish)
        assert!(state.finish());
        assert_eq!(state.current(), ExecutionState::Idle);
    }

    #[test]
    fn test_invalid_transition() {
        let mut state = CreatureExecutionState::new();

        // Idle -> Arrive is invalid (we're not moving)
        assert!(!state.arrive());
        assert_eq!(state.current(), ExecutionState::Idle);
    }

    #[test]
    fn test_flee_from_any_interruptible_state() {
        let mut state = CreatureExecutionState::new();

        // Go to Mining
        assert!(state.start_mining(Vec2::new(5.0, 5.0)));
        assert_eq!(state.current(), ExecutionState::Mining);

        // Flee should work from Mining
        assert!(state.force_flee(Vec2::new(10.0, 20.0)));
        assert_eq!(state.current(), ExecutionState::Fleeing);
    }

    #[test]
    fn test_dead_is_terminal() {
        let mut state = CreatureExecutionState::new();

        assert!(state.die());
        assert!(state.is_dead());

        // No transitions should work from Dead
        assert!(!state.start_moving(Vec2::ZERO));
        assert!(!state.safe());
        assert!(state.is_dead());
    }

    #[test]
    fn test_state_timer() {
        let mut state = CreatureExecutionState::new();
        state.update(0.5);
        assert!((state.time_in_state() - 0.5).abs() < 0.001);

        // Timer resets on transition
        state.start_moving(Vec2::ZERO);
        assert!((state.time_in_state() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_serialization() {
        let mut state = CreatureExecutionState::new();
        state.start_mining(Vec2::new(10.0, 20.0));
        state.update(1.5);

        // Update progress manually
        if let ExecutionStateData::Mining { progress, .. } = state.data_mut() {
            *progress = 0.5;
        }

        // Serialize and deserialize using bincode_next (bincode-next crate)
        let encoded =
            bincode_next::serde::encode_to_vec(&state, bincode_next::config::standard()).unwrap();
        let (restored, _): (CreatureExecutionState, _) =
            bincode_next::serde::decode_from_slice(&encoded, bincode_next::config::standard())
                .unwrap();

        assert_eq!(restored.current(), ExecutionState::Mining);
        assert!((restored.time_in_state() - 1.5).abs() < 0.001);

        if let ExecutionStateData::Mining { position, progress } = restored.data() {
            assert!((position.x - 10.0).abs() < 0.001);
            assert!((progress - 0.5).abs() < 0.001);
        } else {
            panic!("Wrong data type");
        }
    }

    #[test]
    fn test_transition_table_completeness() {
        // Verify all states have at least Die transition (except Dead)
        use ExecutionInput::*;
        use ExecutionState::*;

        for state in [Idle, Moving, Eating, Mining, Building, Fleeing] {
            assert!(
                transition(state, Die).is_some(),
                "{:?} should accept Die input",
                state
            );
        }

        // Dead accepts nothing
        for input in [
            StartMoving,
            StartEating,
            StartMining,
            StartBuilding,
            StartFleeing,
            Arrive,
            Finish,
            Interrupt,
            Safe,
            Die,
        ] {
            assert!(
                transition(Dead, input).is_none(),
                "Dead should reject {:?}",
                input
            );
        }
    }
}
