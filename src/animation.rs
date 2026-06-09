//! Procedural swim animation for the fish "skeletons".
//!
//! The boids systems only move and turn each fish as a whole; nothing in there
//! makes a fish *look* like it is swimming. That life comes from here: every
//! creature is built (in [`crate::setup`]) as a small rig of pivot entities
//! tagged with [`FishPart`], and this system flexes those pivots every frame.
//!
//! The motion is a handful of sinusoids whose frequency rises with swimming
//! speed and whose phase is jittered per fish (via [`SwimGait`]), so a charging
//! tuna beats its tail hard and fast while a drifting one barely idles, and no
//! two fish in a school are ever quite in step.

use std::f32::consts::TAU;

use bevy::prelude::*;

use crate::components::{FishPart, RestPose, SwimGait, Velocity};

/// Base tail-beat frequency (Hz) at a standstill; speed adds to it.
const BASE_FREQ: f32 = 1.3;
/// How much each unit/second of swimming speed raises the beat frequency.
const FREQ_PER_SPEED: f32 = 0.11;

/// Flex every fish's body parts for the current frame.
///
/// Walks each fish's subtree (parts can be nested — the tuna's tail and dorsal
/// fin hang off its undulating body) and re-poses every [`FishPart`] relative
/// to its [`RestPose`].
pub fn animate_fish(
    time: Res<Time>,
    fish: Query<(&Velocity, &SwimGait, &Children)>,
    children: Query<&Children>,
    mut parts: Query<(&FishPart, &RestPose, &mut Transform)>,
) {
    let t = time.elapsed_secs();

    for (vel, gait, roots) in &fish {
        let speed = vel.0.length();
        // Faster swimming → faster, harder beats; the per-fish tempo keeps the
        // school from moving as one rigid body.
        let freq = (BASE_FREQ + speed * FREQ_PER_SPEED) * gait.tempo;
        let phase = t * freq * TAU + gait.phase;
        // Effort ramps the amplitude up a little as the fish accelerates, so a
        // sprint reads as a more frantic, full-bodied swim than a cruise.
        let effort = (0.5 + speed * 0.06).min(1.3);

        // Depth-first walk so nested parts (tail/dorsal under the body) animate
        // too. Parts are a tiny fraction of all entities, so this is cheap.
        let mut stack: Vec<Entity> = roots.to_vec();
        while let Some(entity) = stack.pop() {
            if let Ok(grandkids) = children.get(entity) {
                stack.extend_from_slice(grandkids);
            }
            if let Ok((part, rest, mut transform)) = parts.get_mut(entity) {
                *transform = pose_part(*part, rest.0, phase, effort);
            }
        }
    }
}

/// Compute a part's animated transform from its rest pose, the fish's current
/// phase, and an effort factor that scales the amplitude with speed.
fn pose_part(part: FishPart, rest: Transform, phase: f32, effort: f32) -> Transform {
    let mut t = rest;
    match part {
        // Carangiform body wave: a small yaw that the tail (a child pivot) rides
        // on, so thrust looks like it travels down the body to the tail, plus a
        // slight counter-roll — a stiff-bodied fish rocks about its long axis as
        // it beats, and without it the swim read as a flat metronome.
        FishPart::Body => {
            let yaw = 0.09 * effort * phase.sin();
            let roll = 0.05 * effort * (phase - 0.4).sin();
            t.rotation = rest.rotation * Quat::from_rotation_y(yaw) * Quat::from_rotation_z(roll);
        }
        // The tail sweeps a quarter-cycle behind the body wave, the way a real
        // fish's tail lags the bend running down its flank.
        FishPart::TailFin => {
            let yaw = 0.55 * effort * (phase - 0.6).sin();
            t.rotation = rest.rotation * Quat::from_rotation_y(yaw);
        }
        // Pectorals paddle slowly and out of phase with the tail, mirrored side
        // to side so the pair beats symmetrically.
        FishPart::PectoralFin { side } => {
            let flap = 0.35 * (0.5 * phase + 0.8).sin();
            t.rotation = rest.rotation * Quat::from_rotation_z(side * flap);
        }
        // Mantle jet: squeeze the radius and stretch lengthwise on the pulse,
        // then relax — a soft breathing that suggests jet propulsion. Roughly
        // volume-preserving so it doesn't just balloon.
        FishPart::Mantle => {
            let pulse = (phase * 0.8).sin();
            let squeeze = 1.0 - 0.10 * effort * pulse;
            let stretch = 1.0 + 0.14 * effort * pulse;
            t.scale = rest.scale * Vec3::new(squeeze, squeeze, stretch);
        }
        // Lateral fins roll in a travelling ripple; the two sides run a half
        // cycle apart so the squid looks like it's sculling, not flapping.
        FishPart::SquidFin { side } => {
            let ripple = 0.5 * effort * (phase + side.max(0.0) * std::f32::consts::PI).sin();
            t.rotation = rest.rotation * Quat::from_rotation_x(side * ripple);
        }
        // The arm bundle sways and nods, trailing the body like loose tentacles.
        FishPart::Tentacles => {
            let sway = 0.3 * (phase * 0.7).sin();
            let nod = 0.18 * (phase * 0.7 + 1.2).cos();
            t.rotation = rest.rotation
                * Quat::from_rotation_y(sway)
                * Quat::from_rotation_x(nod);
        }
    }
    t
}
