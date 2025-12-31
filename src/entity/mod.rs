pub mod inventory;
pub mod health;
pub mod player;

use std::sync::atomic::{AtomicU64, Ordering};
use serde::{Deserialize, Serialize};

/// Unique identifier for entities in the world
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(u64);

static NEXT_ENTITY_ID: AtomicU64 = AtomicU64::new(1);

impl EntityId {
    /// Generate a new unique entity ID
    pub fn new() -> Self {
        EntityId(NEXT_ENTITY_ID.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the raw u64 value (useful for debugging/serialization)
    pub fn raw(&self) -> u64 {
        self.0
    }

    /// Create an EntityId from a raw u64 (for deserialization)
    pub fn from_raw(id: u64) -> Self {
        // Update the counter if this ID is higher than current
        NEXT_ENTITY_ID.fetch_max(id + 1, Ordering::Relaxed);
        EntityId(id)
    }
}

impl Default for EntityId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Entity({})", self.0)
    }
}

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
