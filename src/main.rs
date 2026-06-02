//! Squid vs Tuna — a 3D boids simulation built with Bevy.
//!
//! Two schools share a tank: squid (prey) flock tightly and flee from tuna,
//! while tuna (predators) school loosely and hunt the nearest squid. The
//! design is inspired by Jonas Lindstrøm's `Boids` project, reimagined in 3D.

mod camera;
mod components;
mod config;
mod flocking;
mod setup;
mod ui;

use bevy::prelude::*;

use camera::{camera_apply, camera_input, OrbitCamera};
use config::{Score, SimConfig};
use flocking::{flocking_system, hunting_system, movement_system};
use setup::setup;
use ui::{draw_bounds, update_stats};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Squid vs Tuna — 3D Boids".into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.02, 0.05, 0.12)))
        .insert_resource(AmbientLight {
            color: Color::srgb(0.5, 0.7, 1.0),
            brightness: 220.0,
            ..default()
        })
        .init_resource::<SimConfig>()
        .init_resource::<Score>()
        .init_resource::<OrbitCamera>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                flocking_system,
                movement_system,
                hunting_system,
                camera_input,
                camera_apply,
                update_stats,
                draw_bounds,
            )
                .chain(),
        )
        .run();
}
