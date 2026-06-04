//! Tunable parameters for the simulation, exposed as a Bevy resource.

use bevy::prelude::*;

/// Per-species flocking tuning (classic Reynolds boids weights plus limits).
#[derive(Clone)]
pub struct BoidParams {
    pub max_speed: f32,
    pub min_speed: f32,
    /// Maximum steering force applied per rule (Reynolds style).
    pub max_force: f32,
    /// Radius within which neighbours contribute to alignment & cohesion.
    pub perception: f32,
    /// Distance below which neighbours push each other apart.
    pub separation_dist: f32,
    pub separation_weight: f32,
    pub alignment_weight: f32,
    pub cohesion_weight: f32,
    /// Maximum heading change in radians per second. Smaller = wider turning
    /// radius (turning radius ~= speed / max_turn_rate).
    pub max_turn_rate: f32,
}

/// Tuning for the water body: a swirling current field that pushes the boids
/// around, plus the drifting "marine snow" particles that make the volume read
/// as water.
#[derive(Clone)]
pub struct WaterParams {
    /// Constant bulk flow of the whole tank, in units per second.
    pub drift: Vec3,
    /// Peak speed of the swirling component of the current, in units/second.
    pub current_strength: f32,
    /// Spatial frequency of the swirls: larger = tighter, smaller = broader
    /// eddies (the field is sampled at `pos * current_scale`).
    pub current_scale: f32,
    /// How fast the current field animates over time.
    pub current_time_scale: f32,
    /// How strongly the current accelerates a boid (an external steering force,
    /// clamped like the others by each species' turn rate and speed envelope).
    pub current_push: f32,

    /// How many drifting particles fill the tank.
    pub particle_count: usize,
    /// Baseline sink speed of a particle, in units/second.
    pub particle_fall_speed: f32,
    /// Radius of a single particle sphere.
    pub particle_size: f32,
}

/// Global simulation configuration.
#[derive(Resource, Clone)]
pub struct SimConfig {
    /// Half-extents of the tank (the simulation volume is `2 * bounds`).
    pub bounds: Vec3,
    pub squid_count: usize,
    pub tuna_count: usize,

    pub squid: BoidParams,
    pub tuna: BoidParams,

    /// The surrounding water: current field and drifting particles.
    pub water: WaterParams,

    /// Squid sense and flee from tuna within this radius.
    pub flee_radius: f32,
    pub flee_weight: f32,
    /// Tuna sense and chase the squid school within this radius.
    pub hunt_radius: f32,
    pub hunt_weight: f32,
    /// Speed multiplier applied to a tuna while it is charging a bait ball.
    pub hunt_speed_boost: f32,
    /// A squid this close to a tuna gets eaten.
    pub eat_radius: f32,

    /// How far from a wall a boid starts steering back. Only used if the soft
    /// `boundary_force` is re-enabled in `flocking_system`; containment is
    /// otherwise a hard position clamp in `movement_system`.
    pub boundary_margin: f32,
    pub boundary_weight: f32,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            bounds: Vec3::new(28.0, 16.0, 28.0),
            squid_count: 40,
            tuna_count: 140,

            squid: BoidParams {
                max_speed: 8.0,
                min_speed: 3.0,
                max_force: 12.0,
                // Wide perception so the scattered minority can find each other
                // and gather into a single tight bait ball.
                perception: 22.0,
                separation_dist: 1.6,
                separation_weight: 1.8,
                alignment_weight: 1.2,
                // Strong cohesion keeps the school balled up under attack.
                cohesion_weight: 1.8,
                // Nimble: squid can pivot tightly to dodge.
                max_turn_rate: 6.0,
            },
            tuna: BoidParams {
                // Clearly faster than squid (8.0) so a charge runs them down.
                max_speed: 14.0,
                min_speed: 6.0,
                max_force: 12.0,
                // See far and across the whole pack so the school stays unified.
                perception: 12.0,
                // Wide personal space so the pack spreads into a loose, open
                // formation instead of collapsing into one dense ball.
                separation_dist: 5.0,
                separation_weight: 2.2,
                // Strong alignment keeps them moving as one school; gentle
                // cohesion holds the formation together without crowding.
                alignment_weight: 2.0,
                cohesion_weight: 0.9,
                // Big and fast: a wide turning radius, so a charge overshoots
                // the bait ball and sweeps around for another pass.
                max_turn_rate: 1.2,
            },

            water: WaterParams {
                // Only a whisper of net drift: enough to bias the flow without
                // convecting every fish into a downstream corner. The roaming
                // comes from the swirls below, whose spatial average is ~zero.
                drift: Vec3::new(0.15, 0.0, 0.08),
                // Swirls are a soft nudge next to the 3–14 unit/s swim speeds,
                // enough to bend the schools without overpowering the boids.
                current_strength: 1.5,
                // Broad eddies a few body-lengths across.
                current_scale: 0.06,
                current_time_scale: 0.35,
                current_push: 1.2,

                // A dense drift of marine snow: the suspended motes are what sell
                // the tank as a body of water rather than tinted air, so there are
                // plenty of them, slightly larger, sinking slowly through the volume.
                particle_count: 650,
                particle_fall_speed: 0.6,
                particle_size: 0.07,
            },

            flee_radius: 8.5,
            // Moderate flee so cohesion wins: the whole ball edges away from a
            // charge together instead of exploding apart.
            flee_weight: 2.6,
            // Commit to a bait ball from far away and chase it hard.
            hunt_radius: 24.0,
            hunt_weight: 2.6,
            // Extra speed while a bait ball is in range: the charge/sprint.
            hunt_speed_boost: 1.4,
            eat_radius: 1.4,

            // Tuning for the (currently disabled) soft boundary steering: a wide
            // margin would let the fast, wide-turning tuna bank away early. Kept
            // for if `boundary_force` is re-enabled.
            boundary_margin: 7.0,
            boundary_weight: 5.0,
        }
    }
}

impl SimConfig {
    pub fn params(&self, species: Species) -> &BoidParams {
        match species {
            Species::Squid => &self.squid,
            Species::Tuna => &self.tuna,
        }
    }
}

use crate::components::Species;

/// Running tally of how many squid the tuna have eaten.
#[derive(Resource, Default)]
pub struct Score {
    pub eaten: u32,
}
