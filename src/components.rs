//! ECS components shared across the simulation.

use bevy::prelude::*;

/// Which kind of creature a boid is. The flocking system uses this to pick the
/// right tuning parameters and to decide predator/prey behaviour.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Species {
    /// Prey. Flock tightly and flee from tuna.
    Squid,
    /// Predator. Loosely schooled and hunt squid.
    Tuna,
}

/// Marker component for squid, so we can write disjoint queries.
#[derive(Component)]
pub struct Squid;

/// Marker component for tuna, so we can write disjoint queries.
#[derive(Component)]
pub struct Tuna;

/// Current world-space velocity of a boid, in units per second.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Velocity(pub Vec3);

/// Marker for the on-screen statistics text.
#[derive(Component)]
pub struct StatsText;
