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

    /// How far from a wall a boid starts steering back.
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
                perception: 4.5,
                separation_dist: 1.6,
                separation_weight: 1.8,
                alignment_weight: 1.0,
                cohesion_weight: 0.9,
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
            },

            flee_radius: 8.5,
            flee_weight: 3.5,
            // Commit to a bait ball from far away and chase it hard.
            hunt_radius: 24.0,
            hunt_weight: 2.6,
            // Extra speed while a bait ball is in range: the charge/sprint.
            hunt_speed_boost: 1.4,
            eat_radius: 1.4,

            boundary_margin: 4.0,
            boundary_weight: 4.0,
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
