//! Core boids behaviour: separation, alignment, cohesion, plus the
//! predator/prey interaction between tuna (hunters) and squid (prey).

use bevy::prelude::*;
use rand::Rng;

use crate::components::{Species, Squid, Tuna, Velocity};
use crate::config::{Score, SimConfig};
use crate::water::current_at;

/// A lightweight, immutable snapshot of every boid for the current frame, so
/// the O(n^2) neighbour scan doesn't fight the borrow checker.
struct Agent {
    entity: Entity,
    species: Species,
    pos: Vec3,
    vel: Vec3,
}

/// Reynolds-style steering: head towards `desired_dir` at full speed, but only
/// apply a limited correction force relative to the current velocity.
fn steer_towards(desired_dir: Vec3, vel: Vec3, max_speed: f32, max_force: f32) -> Vec3 {
    if desired_dir.length_squared() < 1e-6 {
        return Vec3::ZERO;
    }
    let desired = desired_dir.normalize() * max_speed;
    (desired - vel).clamp_length_max(max_force)
}

/// Rotate `new_vel` so its heading is at most `max_angle` radians away from the
/// heading of `old_vel`, preserving the new speed. This caps the turn rate.
fn limit_turn(old_vel: Vec3, new_vel: Vec3, max_angle: f32) -> Vec3 {
    let new_speed = new_vel.length();
    if old_vel.length_squared() < 1e-8 || new_speed < 1e-6 {
        return new_vel;
    }
    let old_dir = old_vel.normalize();
    let new_dir = new_vel / new_speed;
    let angle = old_dir.angle_between(new_dir);
    if angle <= max_angle {
        return new_vel;
    }
    let axis = old_dir.cross(new_dir);
    if axis.length_squared() < 1e-8 {
        // Nearly opposite headings: pick any perpendicular axis to turn around.
        let fallback = if old_dir.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
        let axis = old_dir.cross(fallback).normalize();
        return Quat::from_axis_angle(axis, max_angle) * old_dir * new_speed;
    }
    Quat::from_axis_angle(axis.normalize(), max_angle) * old_dir * new_speed
}

/// Recompute every boid's velocity from its neighbours and the other species.
pub fn flocking_system(
    time: Res<Time>,
    cfg: Res<SimConfig>,
    mut query: Query<(Entity, &Species, &Transform, &mut Velocity)>,
) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();

    let agents: Vec<Agent> = query
        .iter()
        .map(|(entity, species, transform, vel)| Agent {
            entity,
            species: *species,
            pos: transform.translation,
            vel: vel.0,
        })
        .collect();

    for (entity, species, transform, mut velocity) in query.iter_mut() {
        let species = *species;
        let p = cfg.params(species);
        let pos = transform.translation;
        let vel = velocity.0;

        // Same-species flocking accumulators.
        let mut separation = Vec3::ZERO;
        let mut align_sum = Vec3::ZERO;
        let mut center_sum = Vec3::ZERO;
        let mut neighbours = 0u32;

        // Interaction accumulators.
        let mut flee = Vec3::ZERO; // squid only
        let mut prey_center_sum = Vec3::ZERO; // tuna only
        let mut prey_count = 0u32; // tuna only

        for other in &agents {
            if other.entity == entity {
                continue;
            }
            let offset = pos - other.pos;
            let dist = offset.length();
            if dist <= f32::EPSILON {
                continue;
            }

            if other.species == species {
                if dist < p.perception {
                    align_sum += other.vel;
                    center_sum += other.pos;
                    neighbours += 1;
                    if dist < p.separation_dist {
                        // Push away, weighted by inverse square distance.
                        separation += offset / (dist * dist);
                    }
                }
            } else {
                match species {
                    Species::Squid => {
                        if dist < cfg.flee_radius {
                            flee += offset / (dist * dist);
                        }
                    }
                    Species::Tuna => {
                        // Aim at the centre of mass of the squid school in
                        // range, not just the nearest squid, so a pack of tuna
                        // converges on the same bait ball and charges together.
                        if dist < cfg.hunt_radius {
                            prey_center_sum += other.pos;
                            prey_count += 1;
                        }
                    }
                }
            }
        }

        // Tuna sprint when a bait ball is in range, so they can run down the
        // slower, fleeing squid and actually break into the school.
        let max_speed = if species == Species::Tuna && prey_count > 0 {
            p.max_speed * cfg.hunt_speed_boost
        } else {
            p.max_speed
        };

        let mut accel = Vec3::ZERO;

        if neighbours > 0 {
            let inv = 1.0 / neighbours as f32;
            let cohesion = steer_towards(center_sum * inv - pos, vel, max_speed, p.max_force);
            let alignment = steer_towards(align_sum * inv, vel, max_speed, p.max_force);
            accel += cohesion * p.cohesion_weight;
            accel += alignment * p.alignment_weight;
        }
        if separation != Vec3::ZERO {
            accel += steer_towards(separation, vel, max_speed, p.max_force) * p.separation_weight;
        }

        match species {
            Species::Squid => {
                if flee != Vec3::ZERO {
                    accel += steer_towards(flee, vel, max_speed, p.max_force) * cfg.flee_weight;
                }
            }
            Species::Tuna => {
                if prey_count > 0 {
                    let target = prey_center_sum / prey_count as f32;
                    accel +=
                        steer_towards(target - pos, vel, max_speed, p.max_force) * cfg.hunt_weight;
                }
            }
        }

        // Soft boundary steering disabled — containment is left to the hard
        // position clamp in `movement_system`.
        // accel += boundary_force(pos, vel, &cfg, max_speed, p.max_force);

        // The surrounding water pushes the boid along the local current, so the
        // whole school drifts and curls with the flow. Treated as an external
        // force, so it's still bounded by the speed clamp and turn-rate limit
        // applied below.
        accel += current_at(pos, t, &cfg.water) * cfg.water.current_push;

        // Integrate and clamp to the species' speed envelope.
        let mut new_vel = vel + accel * dt;
        let speed = new_vel.length();
        if speed > max_speed {
            new_vel = new_vel / speed * max_speed;
        } else if speed < p.min_speed {
            new_vel = if speed > 1e-4 {
                new_vel / speed * p.min_speed
            } else {
                Vec3::new(p.min_speed, 0.0, 0.0)
            };
        }

        // Limit how fast the heading can change, giving each species a real
        // turning radius (radius ~= speed / max_turn_rate). Big fast tuna sweep
        // through wide arcs; nimble squid pivot sharply.
        new_vel = limit_turn(vel, new_vel, p.max_turn_rate * dt);

        velocity.0 = new_vel;
    }
}

/// A restoring force that grows as a boid approaches a wall of the tank.
/// (Currently unused — see `flocking_system`; kept for easy re-enabling.)
#[allow(dead_code)]
fn boundary_force(pos: Vec3, vel: Vec3, cfg: &SimConfig, max_speed: f32, max_force: f32) -> Vec3 {
    let mut dir = Vec3::ZERO;
    let m = cfg.boundary_margin;
    let b = cfg.bounds;

    for axis in 0..3 {
        let v = pos[axis];
        let limit = b[axis];
        if v > limit - m {
            dir[axis] -= 1.0;
        } else if v < -limit + m {
            dir[axis] += 1.0;
        }
    }

    if dir == Vec3::ZERO {
        Vec3::ZERO
    } else {
        steer_towards(dir, vel, max_speed, max_force) * cfg.boundary_weight
    }
}

/// Move every boid along its velocity and smoothly turn it to face the way it
/// is heading. A hard clamp to the tank is the sole containment now (the soft
/// boundary steering is disabled): fish swim up to a wall and slide along it.
pub fn movement_system(
    time: Res<Time>,
    cfg: Res<SimConfig>,
    mut query: Query<(&mut Velocity, &mut Transform)>,
) {
    let dt = time.delta_secs();
    let b = cfg.bounds;
    for (mut velocity, mut transform) in query.iter_mut() {
        transform.translation += velocity.0 * dt;

        // Keep every fish inside the tank: clamp the position to the walls and
        // cancel any outward velocity so it slides along the wall instead of
        // punching through it.
        for axis in 0..3 {
            if transform.translation[axis] > b[axis] {
                transform.translation[axis] = b[axis];
                velocity.0[axis] = velocity.0[axis].min(0.0);
            } else if transform.translation[axis] < -b[axis] {
                transform.translation[axis] = -b[axis];
                velocity.0[axis] = velocity.0[axis].max(0.0);
            }
        }

        if velocity.0.length_squared() > 1e-4 {
            let mut target = *transform;
            target.look_to(velocity.0.normalize(), Vec3::Y);
            let t = (dt * 6.0).min(1.0);
            transform.rotation = transform.rotation.slerp(target.rotation, t);
        }
    }
}

/// When a tuna catches a squid, the squid is "eaten": it respawns elsewhere in
/// the tank so the population stays constant, and the score ticks up.
pub fn hunting_system(
    cfg: Res<SimConfig>,
    mut score: ResMut<Score>,
    tuna: Query<&Transform, With<Tuna>>,
    mut squid: Query<(&mut Transform, &mut Velocity), (With<Squid>, Without<Tuna>)>,
) {
    let eat_sq = cfg.eat_radius * cfg.eat_radius;
    let mut rng = rand::thread_rng();

    for (mut squid_tf, mut squid_vel) in squid.iter_mut() {
        let caught = tuna
            .iter()
            .any(|t| t.translation.distance_squared(squid_tf.translation) < eat_sq);
        if caught {
            squid_tf.translation = random_point(&cfg, &mut rng);
            squid_vel.0 = random_velocity(cfg.squid.min_speed, cfg.squid.max_speed, &mut rng);
            score.eaten += 1;
        }
    }
}

pub fn random_point(cfg: &SimConfig, rng: &mut impl Rng) -> Vec3 {
    Vec3::new(
        rng.gen_range(-cfg.bounds.x..cfg.bounds.x),
        rng.gen_range(-cfg.bounds.y..cfg.bounds.y),
        rng.gen_range(-cfg.bounds.z..cfg.bounds.z),
    )
}

pub fn random_velocity(min: f32, max: f32, rng: &mut impl Rng) -> Vec3 {
    let dir = Vec3::new(
        rng.gen_range(-1.0..1.0),
        rng.gen_range(-0.4..0.4),
        rng.gen_range(-1.0..1.0),
    );
    let dir = dir.normalize_or_zero();
    let speed = rng.gen_range(min..max.max(min + 0.1));
    if dir == Vec3::ZERO {
        Vec3::new(speed, 0.0, 0.0)
    } else {
        dir * speed
    }
}
