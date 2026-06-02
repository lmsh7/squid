//! Heads-up display and the wireframe tank boundary.

use bevy::prelude::*;

use crate::camera::OrbitCamera;
use crate::components::{Species, StatsText};
use crate::config::{Score, SimConfig};

/// Draw the edges of the tank every frame using gizmos.
pub fn draw_bounds(mut gizmos: Gizmos, cfg: Res<SimConfig>) {
    gizmos.cube(
        Transform::from_scale(cfg.bounds * 2.0),
        Color::srgba(0.3, 0.6, 0.9, 0.4),
    );
}

/// Refresh the on-screen statistics text.
pub fn update_stats(
    score: Res<Score>,
    orbit: Res<OrbitCamera>,
    boids: Query<&Species>,
    mut text: Query<&mut Text, With<StatsText>>,
) {
    let mut squid = 0u32;
    let mut tuna = 0u32;
    for s in &boids {
        match s {
            Species::Squid => squid += 1,
            Species::Tuna => tuna += 1,
        }
    }

    let auto = if orbit.auto { "on" } else { "off" };
    if let Ok(mut text) = text.single_mut() {
        **text = format!(
            "🦑 Squid vs Tuna — 3D Boids\n\
             Squid: {squid}    Tuna: {tuna}\n\
             Squid eaten: {}\n\
             \n\
             [WASD/Arrows] orbit   [Q/E] zoom   [Space] toggle auto-orbit ({auto})",
            score.eaten,
        );
    }
}
