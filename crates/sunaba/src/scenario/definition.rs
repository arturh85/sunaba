//! Scenario definition and RON file loading

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::actions::ScenarioAction;
use super::verification::VerificationCondition;

/// Top-level scenario definition loaded from RON files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioDefinition {
    /// Scenario name
    pub name: String,

    /// Description
    pub description: String,

    /// Initial setup actions (run before main scenario)
    #[serde(default)]
    pub setup: Vec<ScenarioAction>,

    /// Main scenario actions
    pub actions: Vec<ScenarioAction>,

    /// Verification checks to run after scenario
    #[serde(default)]
    pub verify: Vec<VerificationCondition>,

    /// Cleanup actions (run even if scenario fails)
    #[serde(default)]
    pub cleanup: Vec<ScenarioAction>,
}

impl ScenarioDefinition {
    /// Load scenario from RON file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read scenario file: {}", path.display()))?;

        let scenario = ron::from_str(&content)
            .with_context(|| format!("Failed to parse RON scenario: {}", path.display()))?;

        Ok(scenario)
    }

    /// Save scenario to RON file
    pub fn to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let ron = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .context("Failed to serialize scenario to RON")?;

        std::fs::write(path.as_ref(), ron).with_context(|| {
            format!("Failed to write scenario file: {}", path.as_ref().display())
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_serialization() {
        let scenario = ScenarioDefinition {
            name: "Test Scenario".to_string(),
            description: "A test scenario".to_string(),
            setup: vec![ScenarioAction::TeleportPlayer { x: 0.0, y: 100.0 }],
            actions: vec![
                ScenarioAction::WaitFrames { frames: 60 },
                ScenarioAction::Log {
                    message: "Test message".to_string(),
                },
            ],
            verify: vec![VerificationCondition::PlayerPosition {
                x: 0.0,
                y: 100.0,
                tolerance: 5.0,
            }],
            cleanup: vec![],
        };

        // Test RON serialization
        let ron = ron::ser::to_string_pretty(&scenario, ron::ser::PrettyConfig::default()).unwrap();
        assert!(ron.contains("Test Scenario"));
        assert!(ron.contains("TeleportPlayer"));

        // Test round-trip
        let deserialized: ScenarioDefinition = ron::from_str(&ron).unwrap();
        assert_eq!(deserialized.name, scenario.name);
        assert_eq!(deserialized.actions.len(), scenario.actions.len());
    }
}
