//! Scenario execution engine

use anyhow::{Context, Result, bail};
use glam::Vec2;
use rand::thread_rng;
use sunaba_core::entity::InputState;
use sunaba_core::entity::inventory::ItemStack;
use sunaba_core::simulation::MaterialId;
use sunaba_core::world::{NoopStats, World};

use super::actions::{MouseButton, ScenarioAction};
use super::definition::ScenarioDefinition;
use super::results::ExecutionReport;
use super::verification::VerificationCondition;

/// Configuration for scenario executor
#[derive(Debug, Clone)]
pub struct ScenarioExecutorConfig {
    /// Enable screenshot capture
    pub capture_screenshots: bool,

    /// Screenshot output directory
    pub screenshot_dir: String,

    /// Verbose logging
    pub verbose: bool,
}

impl Default for ScenarioExecutorConfig {
    fn default() -> Self {
        Self {
            capture_screenshots: false,
            screenshot_dir: "screenshots".to_string(),
            verbose: false,
        }
    }
}

/// Executes scenario actions against game state
pub struct ScenarioExecutor {
    /// Configuration
    config: ScenarioExecutorConfig,

    /// Current input state
    input_state: InputState,

    /// Current frame counter (for timing)
    frame_count: usize,

    /// Action execution log
    log: Vec<String>,

    /// Screenshots captured
    screenshots: Vec<String>,
}

impl ScenarioExecutor {
    /// Create new executor with default config
    pub fn new() -> Self {
        Self::with_config(ScenarioExecutorConfig::default())
    }

    /// Create new executor with custom config
    pub fn with_config(config: ScenarioExecutorConfig) -> Self {
        Self {
            config,
            input_state: InputState::default(),
            frame_count: 0,
            log: Vec::new(),
            screenshots: Vec::new(),
        }
    }

    /// Execute a complete scenario
    pub fn execute_scenario(
        &mut self,
        scenario: &ScenarioDefinition,
        world: &mut World,
    ) -> Result<ExecutionReport> {
        let mut report = ExecutionReport::new(scenario.name.clone());

        self.log.clear();
        self.screenshots.clear();
        self.frame_count = 0;

        self.log(&format!("Starting scenario: {}", scenario.name));
        self.log(&format!("Description: {}", scenario.description));

        // Execute setup actions
        if !scenario.setup.is_empty() {
            self.log(&format!("Running {} setup actions", scenario.setup.len()));
            for (idx, action) in scenario.setup.iter().enumerate() {
                if let Err(e) = self.execute_action(action, world) {
                    let msg = format!("Setup action {} failed: {}", idx, e);
                    self.log(&msg);
                    report.log = self.log.clone();
                    return Err(anyhow::anyhow!(msg));
                }
            }
        }

        // Execute main actions
        self.log(&format!("Running {} main actions", scenario.actions.len()));
        for (idx, action) in scenario.actions.iter().enumerate() {
            if let Err(e) = self.execute_action(action, world) {
                let msg = format!("Action {} failed: {}", idx, e);
                self.log(&msg);
                report.log = self.log.clone();
                return Err(anyhow::anyhow!(msg));
            }
        }

        report.actions_executed = scenario.setup.len() + scenario.actions.len();

        // Run verifications
        if !scenario.verify.is_empty() {
            self.log(&format!("Running {} verifications", scenario.verify.len()));
            for condition in &scenario.verify {
                let result = condition.evaluate(world);
                self.log(&format!(
                    "  {} {}",
                    if result.passed { "✓" } else { "✗" },
                    result.message
                ));

                if !result.passed {
                    report.verification_failures.push(result);
                }
            }
        }

        // Cleanup actions (always run)
        if !scenario.cleanup.is_empty() {
            self.log(&format!(
                "Running {} cleanup actions",
                scenario.cleanup.len()
            ));
            for (idx, action) in scenario.cleanup.iter().enumerate() {
                if let Err(e) = self.execute_action(action, world) {
                    self.log(&format!("Warning: Cleanup action {} failed: {}", idx, e));
                }
            }
        }

        // Finalize report
        report.frames_executed = self.frame_count;
        report.log = self.log.clone();
        report.screenshots = self.screenshots.clone();
        report.passed = report.verification_failures.is_empty();

        self.log(&format!(
            "Scenario complete: {} ({})",
            if report.passed { "PASSED" } else { "FAILED" },
            self.frame_count
        ));

        Ok(report)
    }

    /// Execute a single action
    fn execute_action(&mut self, action: &ScenarioAction, world: &mut World) -> Result<()> {
        if self.config.verbose {
            self.log(&format!("[Frame {}] {:?}", self.frame_count, action));
        }

        match action {
            // High-level commands
            ScenarioAction::TeleportPlayer { x, y } => {
                world.player.position = Vec2::new(*x, *y);
                world.player.velocity = Vec2::ZERO;
                self.log(&format!("  Teleported player to ({}, {})", x, y));
            }

            ScenarioAction::MovePlayerTo { x, y, timeout } => {
                self.move_player_to(world, Vec2::new(*x, *y), *timeout)?;
            }

            ScenarioAction::MineCircle {
                center_x,
                center_y,
                radius,
            } => {
                world.debug_mine_circle(*center_x, *center_y, *radius);
                self.log(&format!(
                    "  Mined circle at ({}, {}) r={}",
                    center_x, center_y, radius
                ));
            }

            ScenarioAction::MineRect {
                min_x,
                min_y,
                max_x,
                max_y,
            } => {
                // Mine rectangle pixel by pixel
                for y in *min_y..=*max_y {
                    for x in *min_x..=*max_x {
                        if world.get_pixel(x, y).is_some() {
                            world.set_pixel(x, y, MaterialId::AIR);
                        }
                    }
                }
                self.log(&format!(
                    "  Mined rect ({},{}) to ({},{})",
                    min_x, min_y, max_x, max_y
                ));
            }

            ScenarioAction::PlaceMaterial {
                x,
                y,
                material,
                radius,
            } => {
                // Ensure chunks exist in the area first
                let r = *radius as i32;
                world.ensure_chunks_for_area(x - r, y - r, x + r, y + r);

                world.place_material_debug(*x, *y, *material, *radius);
                self.log(&format!(
                    "  Placed {:?} at ({}, {}) r={}",
                    material, x, y, radius
                ));
            }

            ScenarioAction::FillRect {
                min_x,
                min_y,
                max_x,
                max_y,
                material,
            } => {
                // Ensure chunks exist in the area first
                world.ensure_chunks_for_area(*min_x, *min_y, *max_x, *max_y);

                // Fill rectangle
                for y in *min_y..=*max_y {
                    for x in *min_x..=*max_x {
                        world.set_pixel(x, y, *material);
                    }
                }
                self.log(&format!(
                    "  Filled rect ({},{}) to ({},{}) with {:?}",
                    min_x, min_y, max_x, max_y, material
                ));
            }

            ScenarioAction::GiveItem { item, slot } => {
                if let Some(slot_idx) = slot {
                    if let Some(slot_ref) = world.player.inventory.get_slot_mut(*slot_idx) {
                        *slot_ref = Some(item.clone());
                        self.log(&format!("  Gave {:?} to slot {}", item, slot_idx));
                    } else {
                        bail!("Invalid slot index: {}", slot_idx);
                    }
                } else {
                    // Add item based on type
                    match item {
                        ItemStack::Material { material_id, count } => {
                            let remaining = world.player.inventory.add_item(*material_id, *count);
                            if remaining == 0 {
                                self.log(&format!(
                                    "  Gave {} {:?} to inventory",
                                    count, material_id
                                ));
                            } else {
                                bail!("Failed to add all items - {} remaining", remaining);
                            }
                        }
                        ItemStack::Tool { .. } => {
                            bail!("Auto-placement of tools not yet implemented - specify a slot");
                        }
                    }
                }
            }

            ScenarioAction::RemoveItem { slot } => {
                if let Some(slot_ref) = world.player.inventory.get_slot_mut(*slot) {
                    *slot_ref = None;
                    self.log(&format!("  Removed item from slot {}", slot));
                } else {
                    bail!("Invalid slot index: {}", slot);
                }
            }

            ScenarioAction::SetPlayerHealth { health } => {
                world.player.health.current = *health;
                self.log(&format!("  Set player health to {}", health));
            }

            ScenarioAction::SetPlayerHunger { hunger } => {
                world.player.hunger.current = *hunger;
                self.log(&format!("  Set player hunger to {}", hunger));
            }

            ScenarioAction::LoadLevel { level_id } => {
                use crate::levels::LevelManager;
                let mut level_manager = LevelManager::new();
                level_manager.load_level(*level_id, world);
                self.log(&format!("  Loaded level {}", level_id));
            }

            // Low-level input simulation
            ScenarioAction::SimulateKey { key, frames } => {
                self.simulate_key_input(world, key, *frames)?;
            }

            ScenarioAction::SimulateMouseClick {
                world_x,
                world_y,
                button,
                frames,
            } => {
                self.simulate_mouse_click(world, *world_x, *world_y, *button, *frames)?;
            }

            ScenarioAction::SimulateMouseMove { world_x, world_y } => {
                self.input_state.mouse_world_pos = Some((*world_x, *world_y));
                self.log(&format!("  Moved mouse to ({}, {})", world_x, world_y));
            }

            // Control flow
            ScenarioAction::WaitFrames { frames } => {
                self.simulate_frames(world, *frames)?;
                self.log(&format!("  Waited {} frames", frames));
            }

            ScenarioAction::WaitUntil {
                condition,
                timeout_frames,
            } => {
                self.wait_until(world, condition, *timeout_frames)?;
            }

            ScenarioAction::CaptureScreenshot {
                filename,
                width,
                height,
            } => {
                if self.config.capture_screenshots {
                    self.capture_screenshot(world, filename, *width, *height)?;
                } else {
                    self.log(&format!("  Screenshot capture disabled: {}", filename));
                }
            }

            ScenarioAction::Log { message } => {
                self.log(&format!("  [USER] {}", message));
            }

            ScenarioAction::Sequence { actions } => {
                self.log(&format!("  Sequence: {} actions", actions.len()));
                for action in actions {
                    self.execute_action(action, world)?;
                }
            }

            // Creature management
            ScenarioAction::SpawnCreature { genome_type, x, y } => {
                use sunaba_core::creature::{CreatureArchetype, CreatureGenome};

                // Parse archetype from string
                let archetype = match genome_type.to_lowercase().as_str() {
                    "spider" => CreatureArchetype::Spider,
                    "snake" => CreatureArchetype::Snake,
                    "worm" => CreatureArchetype::Worm,
                    "flyer" => CreatureArchetype::Flyer,
                    "evolved" => CreatureArchetype::Evolved,
                    _ => bail!("Unknown creature archetype: {}", genome_type),
                };

                // Create genome for this archetype
                let genome = match archetype {
                    CreatureArchetype::Spider => CreatureGenome::archetype_spider(),
                    CreatureArchetype::Snake => CreatureGenome::archetype_snake(),
                    CreatureArchetype::Worm => CreatureGenome::archetype_worm(),
                    CreatureArchetype::Flyer => CreatureGenome::archetype_flyer(),
                    CreatureArchetype::Evolved => {
                        bail!(
                            "Evolved archetype not supported in scenarios - use specific archetype"
                        )
                    }
                };

                // Spawn creature
                let position = Vec2::new(*x, *y);
                let _creature_id = world
                    .creature_manager
                    .spawn_creature_with_archetype_and_hunger(
                        genome,
                        position,
                        1.0, // Full hunger
                        &sunaba_core::creature::MorphologyConfig::default(),
                        archetype,
                    );

                self.log(&format!("  Spawned {} at ({}, {})", genome_type, x, y));
            }

            ScenarioAction::ClearCreatures => {
                let count = world.creature_manager.count();
                world.creature_manager.clear();
                self.log(&format!("  Cleared {} creatures", count));
            }

            // Unimplemented actions
            _ => {
                bail!("Action not implemented: {:?}", action);
            }
        }

        Ok(())
    }

    /// Move player smoothly to target position
    fn move_player_to(&mut self, world: &mut World, target: Vec2, timeout: f32) -> Result<()> {
        let max_frames = (timeout * 60.0) as usize;

        for frame in 0..max_frames {
            let current_pos = world.player.position;
            let distance = (target - current_pos).length();

            // Reached target
            if distance < 5.0 {
                self.log(&format!(
                    "  Reached target ({:.1}, {:.1}) in {} frames",
                    target.x, target.y, frame
                ));
                return Ok(());
            }

            // Calculate input direction
            let dir = (target - current_pos).normalize_or_zero();

            // Set input state
            self.input_state.w_pressed = dir.y > 0.3;
            self.input_state.s_pressed = dir.y < -0.3;
            self.input_state.a_pressed = dir.x < -0.3;
            self.input_state.d_pressed = dir.x > 0.3;

            // Update player
            world.update_player(&self.input_state, 1.0 / 60.0);
            self.frame_count += 1;

            // Update world (let physics settle)
            if frame % 10 == 0 {
                self.simulate_frames(world, 1)?;
            }
        }

        bail!(
            "MovePlayerTo timed out after {} frames (distance: {:.1}px from ({:.1}, {:.1}))",
            max_frames,
            (target - world.player.position).length(),
            target.x,
            target.y
        );
    }

    /// Simulate key input for N frames
    fn simulate_key_input(&mut self, world: &mut World, key: &str, frames: usize) -> Result<()> {
        // Reset input state
        self.input_state = InputState::default();

        // Map key string to input state
        match key.to_lowercase().as_str() {
            "w" => self.input_state.w_pressed = true,
            "a" => self.input_state.a_pressed = true,
            "s" => self.input_state.s_pressed = true,
            "d" => self.input_state.d_pressed = true,
            "space" => self.input_state.jump_pressed = true,
            _ => bail!("Unknown key: {}", key),
        }

        for _ in 0..frames {
            world.update_player(&self.input_state, 1.0 / 60.0);
            self.frame_count += 1;
        }

        self.log(&format!("  Simulated key '{}' for {} frames", key, frames));

        // Reset input
        self.input_state = InputState::default();
        Ok(())
    }

    /// Simulate mouse click at world coordinates
    fn simulate_mouse_click(
        &mut self,
        world: &mut World,
        world_x: i32,
        world_y: i32,
        button: MouseButton,
        frames: usize,
    ) -> Result<()> {
        self.input_state.mouse_world_pos = Some((world_x, world_y));

        match button {
            MouseButton::Left => self.input_state.left_mouse_pressed = true,
            MouseButton::Right => self.input_state.right_mouse_pressed = true,
            MouseButton::Middle => bail!("Middle mouse not supported"),
        }

        for _ in 0..frames {
            world.update_player(&self.input_state, 1.0 / 60.0);
            self.frame_count += 1;
        }

        self.log(&format!(
            "  Simulated {:?} click at ({}, {}) for {} frames",
            button, world_x, world_y, frames
        ));

        // Reset input
        self.input_state = InputState::default();
        Ok(())
    }

    /// Wait until condition is met
    fn wait_until(
        &mut self,
        world: &mut World,
        condition: &VerificationCondition,
        timeout_frames: usize,
    ) -> Result<()> {
        for frame in 0..timeout_frames {
            let result = condition.evaluate(world);

            if result.passed {
                self.log(&format!(
                    "  Condition met after {} frames: {}",
                    frame, result.message
                ));
                return Ok(());
            }

            // Simulate one frame
            self.simulate_frames(world, 1)?;
        }

        let result = condition.evaluate(world);
        bail!(
            "WaitUntil timed out after {} frames: {}",
            timeout_frames,
            result.message
        );
    }

    /// Simulate N frames of physics
    fn simulate_frames(&mut self, world: &mut World, frames: usize) -> Result<()> {
        let mut stats = NoopStats;
        let mut rng = thread_rng();

        for _ in 0..frames {
            world.update(1.0 / 60.0, &mut stats, &mut rng, false);
            self.frame_count += 1;
        }

        Ok(())
    }

    /// Capture screenshot to file
    fn capture_screenshot(
        &mut self,
        world: &World,
        filename: &str,
        width: Option<usize>,
        height: Option<usize>,
    ) -> Result<()> {
        #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
        {
            use crate::headless::PixelRenderer;

            let w = width.unwrap_or(1920);
            let h = height.unwrap_or(1080);

            let mut renderer = PixelRenderer::new(w, h);
            renderer.render(world, world.materials(), world.player.position, &[], 1.0);

            // Ensure screenshot directory exists
            std::fs::create_dir_all(&self.config.screenshot_dir)?;

            let path = format!("{}/{}", self.config.screenshot_dir, filename);
            save_buffer_as_png(&renderer.buffer, w, h, &path)?;

            self.screenshots.push(path.clone());
            self.log(&format!("  Screenshot saved: {}", path));

            Ok(())
        }

        #[cfg(not(all(not(target_arch = "wasm32"), feature = "headless")))]
        {
            bail!("Screenshot capture requires headless feature");
        }
    }

    fn log(&mut self, message: &str) {
        log::info!("{}", message);
        self.log.push(message.to_string());
    }
}

impl Default for ScenarioExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Save RGBA buffer as PNG
#[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
fn save_buffer_as_png(buffer: &[u8], width: usize, height: usize, path: &str) -> Result<()> {
    use image::{ImageBuffer, Rgba};

    let img: ImageBuffer<Rgba<u8>, _> =
        ImageBuffer::from_raw(width as u32, height as u32, buffer.to_vec())
            .context("Failed to create image from buffer")?;

    img.save(path)
        .with_context(|| format!("Failed to save screenshot: {}", path))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_basic() {
        let mut world = World::new(false);
        let mut executor = ScenarioExecutor::new();

        // Teleport player
        let action = ScenarioAction::TeleportPlayer { x: 100.0, y: 200.0 };
        executor.execute_action(&action, &mut world).unwrap();

        assert_eq!(world.player.position.x, 100.0);
        assert_eq!(world.player.position.y, 200.0);
    }
}
