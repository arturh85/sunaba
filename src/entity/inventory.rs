use serde::{Deserialize, Serialize};

/// A stack of items (materials) in an inventory slot
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ItemStack {
    pub material_id: u16,
    pub count: u32,
}

impl ItemStack {
    /// Create a new item stack
    pub fn new(material_id: u16, count: u32) -> Self {
        ItemStack { material_id, count }
    }

    /// Get the maximum stack size for this material
    /// Most materials stack to 999, special cases can be handled here
    pub fn max_stack_size(&self) -> u32 {
        999 // Default stack size
    }

    /// Check if this stack can accept more items
    pub fn can_add(&self, amount: u32) -> bool {
        self.count + amount <= self.max_stack_size()
    }

    /// Add items to this stack, returns amount that didn't fit
    pub fn add(&mut self, amount: u32) -> u32 {
        let max = self.max_stack_size();
        let space = max.saturating_sub(self.count);
        let to_add = amount.min(space);
        self.count += to_add;
        amount - to_add
    }

    /// Remove items from this stack, returns amount actually removed
    pub fn remove(&mut self, amount: u32) -> u32 {
        let to_remove = amount.min(self.count);
        self.count -= to_remove;
        to_remove
    }

    /// Check if this stack is empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Check if this stack is full
    pub fn is_full(&self) -> bool {
        self.count >= self.max_stack_size()
    }
}

/// Player inventory system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>,
    pub max_slots: usize,
}

impl Inventory {
    /// Create a new inventory with the specified number of slots
    pub fn new(max_slots: usize) -> Self {
        Inventory {
            slots: vec![None; max_slots],
            max_slots,
        }
    }

    /// Try to add an item to the inventory
    /// Returns the amount that couldn't be added (0 if all added successfully)
    pub fn add_item(&mut self, material_id: u16, mut amount: u32) -> u32 {
        // First, try to add to existing stacks of the same material
        for stack in self.slots.iter_mut().flatten() {
            if stack.material_id == material_id && !stack.is_full() {
                amount = stack.add(amount);
                if amount == 0 {
                    return 0;
                }
            }
        }

        // Then, try to create new stacks in empty slots
        while amount > 0 {
            match self.find_empty_slot() {
                Some(index) => {
                    let mut new_stack = ItemStack::new(material_id, 0);
                    let max_stack = new_stack.max_stack_size();
                    let to_add = amount.min(max_stack);
                    new_stack.count = to_add;
                    self.slots[index] = Some(new_stack);
                    amount -= to_add;
                }
                None => break, // No empty slots, return remaining amount
            }
        }

        amount
    }

    /// Try to remove an item from the inventory
    /// Returns the amount actually removed
    pub fn remove_item(&mut self, material_id: u16, mut amount: u32) -> u32 {
        let mut removed = 0;

        for slot in &mut self.slots {
            if let Some(stack) = slot {
                if stack.material_id == material_id {
                    let to_remove = stack.remove(amount);
                    removed += to_remove;
                    amount -= to_remove;

                    // Remove empty stacks
                    if stack.is_empty() {
                        *slot = None;
                    }

                    if amount == 0 {
                        break;
                    }
                }
            }
        }

        removed
    }

    /// Check if the inventory contains at least the specified amount of a material
    pub fn has_item(&self, material_id: u16, amount: u32) -> bool {
        self.count_item(material_id) >= amount
    }

    /// Count how many of a specific material are in the inventory
    pub fn count_item(&self, material_id: u16) -> u32 {
        self.slots
            .iter()
            .filter_map(|slot| slot.as_ref())
            .filter(|stack| stack.material_id == material_id)
            .map(|stack| stack.count)
            .sum()
    }

    /// Find the first empty slot index
    fn find_empty_slot(&self) -> Option<usize> {
        self.slots.iter().position(|slot| slot.is_none())
    }

    /// Get the number of empty slots
    pub fn empty_slot_count(&self) -> usize {
        self.slots.iter().filter(|slot| slot.is_none()).count()
    }

    /// Get the number of used slots
    pub fn used_slot_count(&self) -> usize {
        self.max_slots - self.empty_slot_count()
    }

    /// Clear all items from the inventory
    pub fn clear(&mut self) {
        self.slots.fill(None);
    }

    /// Get a reference to a slot
    pub fn get_slot(&self, index: usize) -> Option<&Option<ItemStack>> {
        self.slots.get(index)
    }

    /// Get a mutable reference to a slot
    pub fn get_slot_mut(&mut self, index: usize) -> Option<&mut Option<ItemStack>> {
        self.slots.get_mut(index)
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new(50) // Default 50 slots
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_stack_basic() {
        let mut stack = ItemStack::new(1, 10);
        assert_eq!(stack.count, 10);
        assert!(!stack.is_empty());
        assert!(!stack.is_full());

        stack.add(5);
        assert_eq!(stack.count, 15);

        let removed = stack.remove(7);
        assert_eq!(removed, 7);
        assert_eq!(stack.count, 8);
    }

    #[test]
    fn test_item_stack_overflow() {
        let mut stack = ItemStack::new(1, 990);
        let overflow = stack.add(20);
        assert_eq!(stack.count, 999);
        assert_eq!(overflow, 11);
        assert!(stack.is_full());
    }

    #[test]
    fn test_inventory_add_single() {
        let mut inv = Inventory::new(10);
        let remaining = inv.add_item(1, 50);
        assert_eq!(remaining, 0);
        assert_eq!(inv.count_item(1), 50);
        assert_eq!(inv.used_slot_count(), 1);
    }

    #[test]
    fn test_inventory_add_multiple_stacks() {
        let mut inv = Inventory::new(10);
        let remaining = inv.add_item(1, 2000);
        assert_eq!(remaining, 0);
        assert_eq!(inv.count_item(1), 2000);
        assert_eq!(inv.used_slot_count(), 3); // 999 + 999 + 2
    }

    #[test]
    fn test_inventory_add_to_existing() {
        let mut inv = Inventory::new(10);
        inv.add_item(1, 100);
        inv.add_item(1, 50);
        assert_eq!(inv.count_item(1), 150);
        assert_eq!(inv.used_slot_count(), 1); // Should stack together
    }

    #[test]
    fn test_inventory_remove() {
        let mut inv = Inventory::new(10);
        inv.add_item(1, 100);
        let removed = inv.remove_item(1, 30);
        assert_eq!(removed, 30);
        assert_eq!(inv.count_item(1), 70);
    }

    #[test]
    fn test_inventory_remove_multiple_stacks() {
        let mut inv = Inventory::new(10);
        inv.add_item(1, 1500); // Creates 2 stacks (999 + 501)
        let removed = inv.remove_item(1, 1200);
        assert_eq!(removed, 1200);
        assert_eq!(inv.count_item(1), 300);
        assert_eq!(inv.used_slot_count(), 1); // First stack removed
    }

    #[test]
    fn test_inventory_full() {
        let mut inv = Inventory::new(2);
        inv.add_item(1, 999);
        inv.add_item(2, 999);
        let remaining = inv.add_item(3, 100);
        assert_eq!(remaining, 100); // No space
        assert_eq!(inv.empty_slot_count(), 0);
    }

    #[test]
    fn test_inventory_has_item() {
        let mut inv = Inventory::new(10);
        inv.add_item(1, 100);
        assert!(inv.has_item(1, 50));
        assert!(inv.has_item(1, 100));
        assert!(!inv.has_item(1, 101));
        assert!(!inv.has_item(2, 1));
    }
}
