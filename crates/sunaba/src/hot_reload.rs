//! Hot-reloading support for configuration and data files.
//!
//! This module provides file watching and reload detection for:
//! - `config.ron` - Game configuration
//! - `materials.ron` - Material definitions (when implemented)
//!
//! On WASM, hot-reloading is disabled (no filesystem access).

use instant::Instant;
use std::time::Duration;

/// Flags indicating which files need reloading.
#[derive(Debug, Clone, Copy, Default)]
pub struct ReloadFlags {
    /// True if config.ron was modified
    pub config_changed: bool,
    /// True if materials.ron was modified
    pub materials_changed: bool,
}

impl ReloadFlags {
    /// Check if any flags are set.
    pub fn any(&self) -> bool {
        self.config_changed || self.materials_changed
    }
}

/// Manages periodic checking and reloading of configuration files.
///
/// On native platforms, checks file modification times periodically.
/// On WASM, this is a no-op (returns empty flags).
#[derive(Debug)]
pub struct HotReloadManager {
    /// Last time we checked for changes
    last_check: Instant,
    /// Minimum time between checks (avoid hammering filesystem)
    check_interval: Duration,
    /// Last known modification time for config.ron
    #[cfg(not(target_arch = "wasm32"))]
    config_modified: Option<std::time::SystemTime>,
    /// Last known modification time for materials.ron
    #[cfg(not(target_arch = "wasm32"))]
    materials_modified: Option<std::time::SystemTime>,
}

impl Default for HotReloadManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HotReloadManager {
    /// Create a new hot reload manager.
    pub fn new() -> Self {
        Self {
            last_check: Instant::now(),
            check_interval: Duration::from_secs(2), // Check every 2 seconds
            #[cfg(not(target_arch = "wasm32"))]
            config_modified: None,
            #[cfg(not(target_arch = "wasm32"))]
            materials_modified: None,
        }
    }

    /// Set the check interval.
    pub fn set_check_interval(&mut self, interval: Duration) {
        self.check_interval = interval;
    }

    /// Check if any files need reloading.
    ///
    /// This is rate-limited by `check_interval` to avoid filesystem spam.
    /// Returns flags indicating which files have changed.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn check_for_changes(&mut self) -> ReloadFlags {
        // Rate limit checks
        if self.last_check.elapsed() < self.check_interval {
            return ReloadFlags::default();
        }
        self.last_check = Instant::now();

        let mut flags = ReloadFlags::default();

        // Check config.ron
        if let Some(modified) = Self::get_modified_time("config.ron")
            && self.config_modified.map(|m| modified > m).unwrap_or(true)
        {
            // First check or file changed
            if self.config_modified.is_some() {
                // Only flag as changed if we've seen it before
                // (avoid triggering reload on first startup)
                flags.config_changed = true;
                log::info!("Detected config.ron modification");
            }
            self.config_modified = Some(modified);
        }

        // Check materials.ron
        if let Some(modified) = Self::get_modified_time("materials.ron")
            && self
                .materials_modified
                .map(|m| modified > m)
                .unwrap_or(true)
        {
            if self.materials_modified.is_some() {
                flags.materials_changed = true;
                log::info!("Detected materials.ron modification");
            }
            self.materials_modified = Some(modified);
        }

        flags
    }

    /// WASM: No hot reloading, always returns empty flags.
    #[cfg(target_arch = "wasm32")]
    pub fn check_for_changes(&mut self) -> ReloadFlags {
        ReloadFlags::default()
    }

    /// Get the modification time of a file, if it exists.
    #[cfg(not(target_arch = "wasm32"))]
    fn get_modified_time(path: &str) -> Option<std::time::SystemTime> {
        std::fs::metadata(path).ok().and_then(|m| m.modified().ok())
    }

    /// Force a reload check on next call (resets the rate limit timer).
    pub fn force_check(&mut self) {
        self.last_check = Instant::now() - self.check_interval - Duration::from_secs(1);
    }

    /// Mark a file as needing initial load (clear cached modification time).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn invalidate_config(&mut self) {
        self.config_modified = None;
    }

    /// Mark materials as needing initial load.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn invalidate_materials(&mut self) {
        self.materials_modified = None;
    }

    #[cfg(target_arch = "wasm32")]
    pub fn invalidate_config(&mut self) {}

    #[cfg(target_arch = "wasm32")]
    pub fn invalidate_materials(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reload_flags_default() {
        let flags = ReloadFlags::default();
        assert!(!flags.config_changed);
        assert!(!flags.materials_changed);
        assert!(!flags.any());
    }

    #[test]
    fn test_reload_flags_any() {
        let flags = ReloadFlags {
            config_changed: true,
            ..Default::default()
        };
        assert!(flags.any());
    }

    #[test]
    fn test_manager_rate_limiting() {
        let mut manager = HotReloadManager::new();
        manager.set_check_interval(Duration::from_secs(10));

        // First check should work
        let _ = manager.check_for_changes();

        // Second check immediately should be rate limited
        let flags = manager.check_for_changes();
        assert!(!flags.any());
    }

    #[test]
    fn test_force_check() {
        let mut manager = HotReloadManager::new();
        manager.set_check_interval(Duration::from_secs(10));

        // Use up the first check
        let _ = manager.check_for_changes();

        // Force a check
        manager.force_check();

        // Now check should work (not rate limited)
        // Note: actual file changes won't be detected since no test files exist
        let _flags = manager.check_for_changes();
        // Just verify it doesn't panic
    }
}
