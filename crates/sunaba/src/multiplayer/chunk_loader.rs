//! Progressive chunk loading with spiral pattern for multiplayer
//!
//! This module provides a spiral iterator that generates chunk positions in expanding
//! square rings, and a chunk load queue that rate-limits chunk loading to avoid frame drops.

use glam::IVec2;
use std::collections::HashSet;

/// Iterator that generates chunk positions in a spiral pattern
///
/// Yields chunks in expanding square rings from the center:
/// - Radius 0: center (1 chunk)
/// - Radius 1: 8 chunks around center
/// - Radius 2: 16 chunks in next ring
/// - etc.
///
/// Example order for radius 0→2:
/// ```text
/// 5 6 7 8 9
/// 4 1 2 3 A
/// C 0 * 1 B
/// D 3 2 1 E
/// E F G H I
/// ```
/// Where * = center, 0 = first ring, 1 = second ring, etc.
pub struct SpiralChunkIterator {
    center: IVec2,
    current_radius: i32,
    max_radius: i32,

    // State machine for walking square perimeter
    side: u8,             // 0=right, 1=down, 2=left, 3=up
    steps_in_side: i32,   // How many steps to take on current side
    steps_remaining: i32, // Steps left on current side
    current_pos: IVec2,   // Current position in iteration
    finished: bool,
}

impl SpiralChunkIterator {
    /// Create a new spiral iterator centered at `center` with maximum radius `max_radius`
    ///
    /// # Arguments
    /// * `center` - The center chunk position (e.g., IVec2::ZERO for spawn)
    /// * `max_radius` - Maximum radius to iterate (e.g., 10 for 21x21 grid)
    ///
    /// # Example
    /// ```
    /// let spiral = SpiralChunkIterator::new(IVec2::ZERO, 10);
    /// // Yields 441 chunks in spiral order (21x21 grid)
    /// ```
    pub fn new(center: IVec2, max_radius: i32) -> Self {
        Self {
            center,
            current_radius: 0,
            max_radius,
            side: 0,
            steps_in_side: 0,
            steps_remaining: 0,
            current_pos: center,
            finished: false,
        }
    }

    /// Calculate total number of chunks this iterator will yield
    pub fn total_chunks(&self) -> usize {
        let side_length = (self.max_radius * 2 + 1) as usize;
        side_length * side_length
    }
}

impl Iterator for SpiralChunkIterator {
    type Item = IVec2;

    fn next(&mut self) -> Option<IVec2> {
        if self.finished {
            return None;
        }

        // Yield current position
        let result = self.current_pos;

        // Special case: radius 0 (center only)
        if self.current_radius == 0 {
            if self.max_radius == 0 {
                self.finished = true;
                return Some(result);
            }

            // Move to radius 1
            self.current_radius = 1;
            self.current_pos = self.center + IVec2::new(self.current_radius, 0);
            self.side = 0; // Start on right side
            self.steps_in_side = self.current_radius * 2;
            self.steps_remaining = self.steps_in_side;
            return Some(result);
        }

        // Walk the square perimeter
        self.steps_remaining -= 1;

        if self.steps_remaining > 0 {
            // Continue on current side
            match self.side {
                0 => self.current_pos.y -= 1, // Right side: move down
                1 => self.current_pos.x -= 1, // Bottom side: move left
                2 => self.current_pos.y += 1, // Left side: move up
                3 => self.current_pos.x += 1, // Top side: move right
                _ => unreachable!(),
            }
        } else {
            // Move to next side
            self.side = (self.side + 1) % 4;

            if self.side == 0 {
                // Completed one full ring, move to next radius
                self.current_radius += 1;

                if self.current_radius > self.max_radius {
                    self.finished = true;
                    return Some(result);
                }

                // Start new ring from right side
                self.current_pos = self.center + IVec2::new(self.current_radius, 0);
                self.steps_in_side = self.current_radius * 2;
            } else {
                // Continue on new side of current ring
                self.steps_in_side = self.current_radius * 2;
            }

            self.steps_remaining = self.steps_in_side;

            // Take first step on new side
            match self.side {
                0 => self.current_pos.y -= 1,
                1 => self.current_pos.x -= 1,
                2 => self.current_pos.y += 1,
                3 => self.current_pos.x += 1,
                _ => unreachable!(),
            }
        }

        Some(result)
    }
}

/// Queue for progressive chunk loading with rate limiting
///
/// Controls which chunks to sync to the world each frame, using a spiral pattern
/// and rate limiting to avoid frame drops.
pub struct ChunkLoadQueue {
    spiral: SpiralChunkIterator,
    loaded_this_session: HashSet<IVec2>,
    batch_size: usize,
    total_chunks: usize,
}

impl ChunkLoadQueue {
    /// Create a new chunk load queue
    ///
    /// # Arguments
    /// * `center` - The center chunk position
    /// * `max_radius` - Maximum radius to load
    /// * `batch_size` - Number of chunks to load per frame (typically 2-3)
    pub fn new(center: IVec2, max_radius: i32, batch_size: usize) -> Self {
        let spiral = SpiralChunkIterator::new(center, max_radius);
        let total_chunks = spiral.total_chunks();

        Self {
            spiral,
            loaded_this_session: HashSet::new(),
            batch_size,
            total_chunks,
        }
    }

    /// Get the next batch of chunks to load (up to `batch_size` chunks)
    ///
    /// Returns chunk positions that haven't been loaded yet this session.
    /// Empty vector means all chunks have been loaded.
    pub fn next_batch(&mut self) -> Vec<IVec2> {
        let mut batch = Vec::with_capacity(self.batch_size);

        while batch.len() < self.batch_size {
            match self.spiral.next() {
                Some(pos) => {
                    // Skip if already loaded
                    if !self.loaded_this_session.contains(&pos) {
                        batch.push(pos);
                    }
                }
                None => break, // Spiral exhausted
            }
        }

        batch
    }

    /// Mark a chunk as loaded
    ///
    /// This prevents the chunk from being requested again in future batches.
    pub fn mark_loaded(&mut self, pos: IVec2) {
        self.loaded_this_session.insert(pos);
    }

    /// Get current loading progress
    ///
    /// Returns (loaded_count, total_count) for progress tracking.
    pub fn progress(&self) -> (usize, usize) {
        (self.loaded_this_session.len(), self.total_chunks)
    }

    /// Check if all chunks have been loaded
    pub fn is_complete(&self) -> bool {
        self.loaded_this_session.len() >= self.total_chunks
    }

    /// Reset the queue with a new center position
    ///
    /// Used when player moves and we need to re-subscribe to a new area.
    /// Clears loaded chunks tracking and resets spiral to new center.
    pub fn reset_center(&mut self, new_center: IVec2, max_radius: i32) {
        self.spiral = SpiralChunkIterator::new(new_center, max_radius);
        self.total_chunks = self.spiral.total_chunks();
        self.loaded_this_session.clear();

        log::info!(
            "Reset chunk load queue to center {:?} (radius {}, {} total chunks)",
            new_center,
            max_radius,
            self.total_chunks
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spiral_center_only() {
        let spiral = SpiralChunkIterator::new(IVec2::ZERO, 0);
        let chunks: Vec<IVec2> = spiral.collect();

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], IVec2::ZERO);
    }

    #[test]
    fn test_spiral_radius_1() {
        let spiral = SpiralChunkIterator::new(IVec2::ZERO, 1);
        let chunks: Vec<IVec2> = spiral.collect();

        // Should have 9 chunks (3x3 grid)
        assert_eq!(chunks.len(), 9);

        // First chunk should be center
        assert_eq!(chunks[0], IVec2::ZERO);

        // Check total coverage (no duplicates)
        let unique: HashSet<IVec2> = chunks.iter().copied().collect();
        assert_eq!(unique.len(), 9);

        // Check all positions are within radius 1
        for chunk in chunks {
            assert!(
                chunk.x.abs() <= 1 && chunk.y.abs() <= 1,
                "Chunk {:?} outside radius 1",
                chunk
            );
        }
    }

    #[test]
    fn test_spiral_radius_2() {
        let spiral = SpiralChunkIterator::new(IVec2::ZERO, 2);
        let chunks: Vec<IVec2> = spiral.collect();

        // Should have 25 chunks (5x5 grid)
        assert_eq!(chunks.len(), 25);

        // First chunk should be center
        assert_eq!(chunks[0], IVec2::ZERO);

        // No duplicates
        let unique: HashSet<IVec2> = chunks.iter().copied().collect();
        assert_eq!(unique.len(), 25);
    }

    #[test]
    fn test_spiral_no_duplicates() {
        let spiral = SpiralChunkIterator::new(IVec2::new(5, 5), 10);
        let chunks: Vec<IVec2> = spiral.collect();

        let unique: HashSet<IVec2> = chunks.iter().copied().collect();
        assert_eq!(
            unique.len(),
            chunks.len(),
            "Spiral iterator produced duplicates"
        );
    }

    #[test]
    fn test_spiral_total_chunks() {
        let spiral = SpiralChunkIterator::new(IVec2::ZERO, 10);
        assert_eq!(spiral.total_chunks(), 441); // 21x21 grid

        let spiral = SpiralChunkIterator::new(IVec2::ZERO, 3);
        assert_eq!(spiral.total_chunks(), 49); // 7x7 grid
    }

    #[test]
    fn test_chunk_load_queue_batch_size() {
        let mut queue = ChunkLoadQueue::new(IVec2::ZERO, 2, 3);

        // First batch should have 3 chunks
        let batch1 = queue.next_batch();
        assert!(batch1.len() <= 3);

        // Mark first batch as loaded
        for pos in &batch1 {
            queue.mark_loaded(*pos);
        }

        // Second batch should have different chunks
        let batch2 = queue.next_batch();
        for pos in &batch2 {
            assert!(
                !batch1.contains(pos),
                "Batch 2 contains chunk from batch 1: {:?}",
                pos
            );
        }
    }

    #[test]
    fn test_chunk_load_queue_progress() {
        let mut queue = ChunkLoadQueue::new(IVec2::ZERO, 1, 2);

        let (loaded, total) = queue.progress();
        assert_eq!(loaded, 0);
        assert_eq!(total, 9); // 3x3 grid

        // Load some chunks
        let batch = queue.next_batch();
        for pos in batch {
            queue.mark_loaded(pos);
        }

        let (loaded, _) = queue.progress();
        assert!(loaded > 0);
    }

    #[test]
    fn test_chunk_load_queue_complete() {
        let mut queue = ChunkLoadQueue::new(IVec2::ZERO, 1, 10);

        assert!(!queue.is_complete());

        // Load all chunks
        while let batch = queue.next_batch()
            && !batch.is_empty()
        {
            for pos in batch {
                queue.mark_loaded(pos);
            }
        }

        assert!(queue.is_complete());
    }

    #[test]
    fn test_chunk_load_queue_reset() {
        let mut queue = ChunkLoadQueue::new(IVec2::ZERO, 2, 3);

        // Load some chunks
        let batch = queue.next_batch();
        for pos in batch {
            queue.mark_loaded(pos);
        }

        let (loaded_before, _) = queue.progress();
        assert!(loaded_before > 0);

        // Reset to new center
        queue.reset_center(IVec2::new(10, 10), 2);

        let (loaded_after, _) = queue.progress();
        assert_eq!(loaded_after, 0, "Progress should reset after reset_center");
    }
}
