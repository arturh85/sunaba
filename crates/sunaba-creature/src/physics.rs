//! Rigid body physics for falling debris

use glam::{IVec2, Vec2};
use rapier2d::prelude::*;
use std::collections::HashMap;

/// Falling debris tracked as a rigid body
pub struct FallingDebris {
    /// Rigid body handle in rapier world
    pub body_handle: RigidBodyHandle,

    /// Collider handle
    pub collider_handle: ColliderHandle,

    /// Pixels relative to center of mass and their materials
    pub pixels: HashMap<IVec2, u16>,

    /// Original center of mass position (for debugging)
    pub original_center: Vec2,
}

/// Debris data for rendering
pub struct DebrisRenderData {
    /// Current position (center of mass)
    pub position: Vec2,

    /// Current rotation in radians
    pub rotation: f32,

    /// Pixel offsets from center + material IDs
    pub pixels: HashMap<IVec2, u16>,
}

/// Manages rapier2d physics world
pub struct PhysicsWorld {
    /// Rapier rigid body set
    rigid_body_set: RigidBodySet,

    /// Rapier collider set
    collider_set: ColliderSet,

    /// Physics pipeline
    pipeline: PhysicsPipeline,

    /// Integration parameters
    integration_parameters: IntegrationParameters,

    /// Island manager
    island_manager: IslandManager,

    /// Broad phase
    broad_phase: BroadPhase,

    /// Narrow phase
    narrow_phase: NarrowPhase,

    /// Impulse joint set
    impulse_joint_set: ImpulseJointSet,

    /// Multibody joint set
    multibody_joint_set: MultibodyJointSet,

    /// CCD solver
    ccd_solver: CCDSolver,

    /// Query pipeline
    query_pipeline: QueryPipeline,

    /// Active falling debris
    debris: HashMap<RigidBodyHandle, FallingDebris>,
}

impl PhysicsWorld {
    pub fn new() -> Self {
        let integration_parameters = IntegrationParameters {
            dt: 1.0 / 60.0, // 60 FPS
            ..Default::default()
        };

        let mut collider_set = ColliderSet::new();

        // IMPORTANT: Create static ground plane below world to stop infinite falling
        // Bedrock ends at y=-57, so ground at y=-200 is safely below
        let ground_half_width = 5000.0; // Wide enough for any debris
        let ground_half_height = 50.0; // Thick enough to be solid
        let ground_y = -200.0; // Well below bedrock

        let ground = ColliderBuilder::cuboid(ground_half_width, ground_half_height)
            .translation(vector![0.0, ground_y])
            .friction(0.8) // High friction so debris don't slide much
            .restitution(0.1) // Low bounce
            .build();

        collider_set.insert(ground);

        log::debug!(
            "Physics: Created ground plane at y={} ({}x{} pixels)",
            ground_y,
            ground_half_width * 2.0,
            ground_half_height * 2.0
        );

        // Optional: Add side walls to prevent debris from flying off-screen
        let wall_half_width = 50.0;
        let wall_half_height = 2000.0;

        let left_wall = ColliderBuilder::cuboid(wall_half_width, wall_half_height)
            .translation(vector![-2000.0, 0.0])
            .build();

        let right_wall = ColliderBuilder::cuboid(wall_half_width, wall_half_height)
            .translation(vector![2000.0, 0.0])
            .build();

        collider_set.insert(left_wall);
        collider_set.insert(right_wall);

        Self {
            rigid_body_set: RigidBodySet::new(),
            collider_set, // Use the collider_set with ground plane and walls
            pipeline: PhysicsPipeline::new(),
            integration_parameters,
            island_manager: IslandManager::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
            debris: HashMap::new(),
        }
    }

    /// Create an empty physics world for use as a temporary placeholder
    /// This avoids the overhead of creating ground planes and walls when
    /// we just need a dummy value for std::mem::replace
    pub fn empty() -> Self {
        let integration_parameters = IntegrationParameters {
            dt: 1.0 / 60.0,
            ..Default::default()
        };

        Self {
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            pipeline: PhysicsPipeline::new(),
            integration_parameters,
            island_manager: IslandManager::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
            debris: HashMap::new(),
        }
    }

    /// Create falling debris from a pixel region
    /// Returns the rigid body handle
    pub fn create_debris(&mut self, pixels: HashMap<IVec2, u16>) -> RigidBodyHandle {
        // Calculate center of mass from absolute coords
        let center = self.calculate_center_of_mass(&pixels);

        log::info!(
            "Physics: Creating rigid body debris: {} pixels at center ({:.1}, {:.1})",
            pixels.len(),
            center.x,
            center.y
        );

        // Convert pixels to relative coordinates (offset from center)
        let mut relative_pixels = HashMap::new();
        let center_i = IVec2::new(center.x.round() as i32, center.y.round() as i32);
        for (world_pos, material_id) in pixels {
            let relative_pos = world_pos - center_i;
            relative_pixels.insert(relative_pos, material_id);
        }

        // Create rigid body at center
        let rigid_body = RigidBodyBuilder::dynamic()
            .translation(vector![center.x, center.y])
            .build();

        let body_handle = self.rigid_body_set.insert(rigid_body);

        // Create collider from pixel shape (use relative pixels)
        let collider = self.create_collider_from_pixels(&relative_pixels, Vec2::ZERO);
        let collider_handle =
            self.collider_set
                .insert_with_parent(collider, body_handle, &mut self.rigid_body_set);

        // Store debris data
        let debris = FallingDebris {
            body_handle,
            collider_handle,
            pixels: relative_pixels, // Store relative coords
            original_center: center, // For debugging
        };

        self.debris.insert(body_handle, debris);

        log::debug!("Created rigid body with handle {:?}", body_handle);

        body_handle
    }

    /// Update physics simulation
    pub fn step(&mut self) {
        let gravity = vector![0.0, -300.0]; // Downward gravity (pixels/s^2)
        let physics_hooks = ();
        let event_handler = ();

        self.pipeline.step(
            &gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &physics_hooks,
            &event_handler,
        );
    }

    /// Check for debris that have stopped moving (should be converted back to pixels)
    /// Returns list of debris handles that need to be converted
    pub fn get_settled_debris(&self) -> Vec<RigidBodyHandle> {
        let mut settled = Vec::new();

        for handle in self.debris.keys() {
            let body = &self.rigid_body_set[*handle];

            // Check if velocity is very low (came to rest)
            let linvel = body.linvel();
            let velocity_magnitude = linvel.magnitude();

            if velocity_magnitude < 5.0 {
                // Settled - not moving much
                log::info!(
                    "Physics: Debris settled: handle={:?}, velocity={:.2} px/s",
                    handle,
                    velocity_magnitude
                );
                settled.push(*handle);
            }
        }

        settled
    }

    /// Remove debris and return its data for reconstruction
    pub fn remove_debris(&mut self, handle: RigidBodyHandle) -> Option<FallingDebris> {
        if let Some(debris) = self.debris.remove(&handle) {
            // Remove collider
            self.collider_set.remove(
                debris.collider_handle,
                &mut self.island_manager,
                &mut self.rigid_body_set,
                false,
            );

            // Remove rigid body
            self.rigid_body_set.remove(
                handle,
                &mut self.island_manager,
                &mut self.collider_set,
                &mut self.impulse_joint_set,
                &mut self.multibody_joint_set,
                false,
            );

            Some(debris)
        } else {
            None
        }
    }

    /// Get the current position and rotation of a debris
    pub fn get_debris_transform(&self, handle: RigidBodyHandle) -> Option<(Vec2, f32)> {
        if let Some(body) = self.rigid_body_set.get(handle) {
            let translation = body.translation();
            let rotation = body.rotation().angle();
            Some((Vec2::new(translation.x, translation.y), rotation))
        } else {
            None
        }
    }

    /// Calculate center of mass from pixel positions (uniform density)
    fn calculate_center_of_mass(&self, pixels: &HashMap<IVec2, u16>) -> Vec2 {
        let mut sum = Vec2::ZERO;
        for pos in pixels.keys() {
            sum += Vec2::new(pos.x as f32, pos.y as f32);
        }
        sum / pixels.len() as f32
    }

    /// Create collider from pixel shape
    /// Uses bounding box approximation for now (can be improved with convex hull)
    fn create_collider_from_pixels(&self, pixels: &HashMap<IVec2, u16>, _center: Vec2) -> Collider {
        // Find bounding box
        let mut min = IVec2::new(i32::MAX, i32::MAX);
        let mut max = IVec2::new(i32::MIN, i32::MIN);

        for pos in pixels.keys() {
            min = min.min(*pos);
            max = max.max(*pos);
        }

        // Calculate half extents relative to center
        let width = (max.x - min.x + 1) as f32;
        let height = (max.y - min.y + 1) as f32;
        let half_extents = vector![width * 0.5, height * 0.5];

        log::debug!(
            "Physics: Created collider {}x{} pixels, density=2.0",
            width,
            height
        );

        ColliderBuilder::cuboid(half_extents.x, half_extents.y)
            .density(2.0) // Average material density
            .friction(0.5)
            .restitution(0.1) // Small bounce
            .build()
    }

    /// Get number of active debris
    pub fn debris_count(&self) -> usize {
        self.debris.len()
    }

    /// Get debris render data for all active debris
    pub fn get_debris_render_data(&self) -> Vec<DebrisRenderData> {
        let mut render_data = Vec::new();

        for (handle, debris) in &self.debris {
            if let Some((position, rotation)) = self.get_debris_transform(*handle) {
                render_data.push(DebrisRenderData {
                    position,
                    rotation,
                    pixels: debris.pixels.clone(),
                });
            }
        }

        render_data
    }

    /// Add a static bedrock chunk collider
    /// This creates a physics collision box for a chunk to prevent debris from falling through
    pub fn add_bedrock_collider(&mut self, chunk_x: i32, chunk_y: i32) {
        // Create a static cuboid for this chunk
        const CHUNK_SIZE: f32 = 64.0; // Must match world::CHUNK_SIZE
        let world_x = (chunk_x as f32 * CHUNK_SIZE) + (CHUNK_SIZE / 2.0);
        let world_y = (chunk_y as f32 * CHUNK_SIZE) + (CHUNK_SIZE / 2.0);

        let collider = ColliderBuilder::cuboid(CHUNK_SIZE / 2.0, CHUNK_SIZE / 2.0)
            .translation(vector![world_x, world_y])
            .friction(0.8)
            .restitution(0.1)
            .build();

        self.collider_set.insert(collider);

        log::debug!(
            "Physics: Added bedrock collider at chunk ({}, {})",
            chunk_x,
            chunk_y
        );
    }

    // ===== Multibody/Creature Physics Methods =====

    /// Get mutable access to multibody joint set (for creature articulated bodies)
    pub fn multibody_joint_set_mut(&mut self) -> &mut MultibodyJointSet {
        &mut self.multibody_joint_set
    }

    /// Get reference to rigid body set
    pub fn rigid_body_set(&self) -> &RigidBodySet {
        &self.rigid_body_set
    }

    /// Get mutable reference to rigid body set
    pub fn rigid_body_set_mut(&mut self) -> &mut RigidBodySet {
        &mut self.rigid_body_set
    }

    /// Get mutable reference to collider set
    pub fn collider_set_mut(&mut self) -> &mut ColliderSet {
        &mut self.collider_set
    }

    /// Get mutable reference to island manager
    pub fn island_manager_mut(&mut self) -> &mut IslandManager {
        &mut self.island_manager
    }

    /// Remove a multibody (for creature cleanup)
    pub fn remove_multibody(&mut self, handle: MultibodyJointHandle) {
        self.multibody_joint_set.remove(handle, true);
    }

    /// Get multibody root position
    pub fn get_multibody_position(&self, handle: MultibodyJointHandle) -> Option<Vec2> {
        // For now, return None as we're using placeholder handles
        // This will be properly implemented when we use real multibody joints
        let _ = handle;
        None
    }

    /// Remove a rigid body (helper for creatures)
    pub fn remove_rigid_body(&mut self, handle: RigidBodyHandle) {
        let mut impulse_joints = Default::default();
        self.rigid_body_set.remove(
            handle,
            &mut self.island_manager,
            &mut self.collider_set,
            &mut impulse_joints,
            &mut self.multibody_joint_set,
            false,
        );
    }

    /// Add collider with parent (helper for creatures)
    pub fn add_collider_with_parent(
        &mut self,
        collider: Collider,
        parent_handle: RigidBodyHandle,
    ) -> ColliderHandle {
        self.collider_set
            .insert_with_parent(collider, parent_handle, &mut self.rigid_body_set)
    }
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self::new()
    }
}
