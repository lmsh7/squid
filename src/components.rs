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

/// Per-fish swim-animation state, stored on the boid root. A random phase
/// offset and tempo multiplier keep a whole school from beating its fins in
/// lockstep, so the shoal looks alive rather than mechanical.
#[derive(Component, Clone, Copy)]
pub struct SwimGait {
    /// Phase offset in radians, randomised per fish.
    pub phase: f32,
    /// Per-fish multiplier on the beat frequency (~1.0), randomised so faster
    /// and slower swimmers mingle in the same school.
    pub tempo: f32,
}

/// The role of an animated body part within a fish — the procedural "skeleton".
/// Each tagged child is a pivot the [`animate_fish`](crate::animation::animate_fish)
/// system drives every frame, applying the motion appropriate to its role
/// relative to its [`RestPose`].
#[derive(Component, Clone, Copy)]
pub enum FishPart {
    /// Tuna body: a gentle carangiform S-curve that the tail rides on. Its
    /// children (the tail and dorsal fin) undulate with it.
    Body,
    /// Tuna caudal (tail) fin: yaws side to side — the main thrust.
    TailFin,
    /// A pectoral / side fin that flaps gently. `side` is -1 (left) or +1.
    PectoralFin { side: f32 },
    /// Squid mantle: pulses in and out, evoking jet propulsion.
    Mantle,
    /// A squid lateral fin that rolls in a travelling ripple. `side` is -1/+1.
    SquidFin { side: f32 },
    /// The squid's trailing arm/tentacle bundle: sways and curls as it swims.
    Tentacles,
}

/// A part's rest-pose local transform. The animation system applies motion
/// relative to this so repeated frames don't accumulate drift.
#[derive(Component, Clone, Copy)]
pub struct RestPose(pub Transform);

/// A kelp blade anchored to the seabed. Stores its rest rotation plus a phase
/// and amplitude so each blade sways in the current on its own rhythm.
#[derive(Component)]
pub struct Kelp {
    pub phase: f32,
    pub amp: f32,
    pub rest: Quat,
}

/// Marker for the on-screen statistics text.
#[derive(Component)]
pub struct StatsText;

/// Marker for the translucent blue water-tint box, so a system can fade it out
/// as the camera zooms inside the tank (where the tint would otherwise smear
/// across the view and never resolve no matter how far you zoom in).
#[derive(Component)]
pub struct WaterTint;
