pub mod crafting;
pub mod health;
pub mod input;
pub mod inventory;
pub mod player;
pub mod tools;

pub use input::InputState;

// Re-export EntityId from sunaba-creature for consistency
pub use sunaba_creature::EntityId;

// Keep Health and Hunger local (player uses them differently than creatures)
pub use health::{Health, Hunger};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_id_uniqueness() {
        let id1 = EntityId::new();
        let id2 = EntityId::new();
        assert_ne!(id1, id2);
        assert!(id2.raw() > id1.raw());
    }

    #[test]
    fn test_entity_id_from_raw() {
        let id = EntityId::from_raw(42);
        assert_eq!(id.raw(), 42);

        let next_id = EntityId::new();
        assert!(next_id.raw() > 42);
    }
}
