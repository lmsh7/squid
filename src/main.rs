//! Squid vs Tuna — a 3D boids simulation built with Bevy.
//!
//! Two schools share a tank: squid (prey) flock tightly and flee from tuna,
//! while tuna (predators) school loosely and hunt the nearest squid. The
//! design is inspired by Jonas Lindstrøm's `Boids` project, reimagined in 3D.

use bevy::prelude::*;

use squid::animation::animate_fish;
use squid::camera::{camera_apply, camera_input, fade_water_tint, OrbitCamera};
use squid::config::{Score, SimConfig};
use squid::flocking::{flocking_system, hunting_system, movement_system};
use squid::setup::{setup_scene, spawn_window_camera};
use squid::ui::{configure_gizmos, draw_bounds, update_stats};
use squid::water::{drift_particles, spawn_water_particles};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Squid vs Tuna — 3D Boids".into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.06, 0.28, 0.44)))
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.5, 0.7, 1.0),
            brightness: 130.0,
            ..default()
        })
        .init_resource::<SimConfig>()
        .init_resource::<Score>()
        .init_resource::<OrbitCamera>()
        .add_systems(
            Startup,
            (
                spawn_window_camera,
                setup_scene,
                spawn_water_particles,
                configure_gizmos,
            ),
        )
        .add_systems(
            Update,
            (
                flocking_system,
                movement_system,
                animate_fish,
                drift_particles,
                hunting_system,
                camera_input,
                camera_apply,
                fade_water_tint,
                update_stats,
                draw_bounds,
            )
                .chain(),
        )
        .run();
}
