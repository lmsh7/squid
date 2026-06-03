//! A simple orbit camera that circles the tank and responds to the keyboard.

use bevy::prelude::*;

use crate::components::WaterTint;
use crate::config::SimConfig;

/// Orbit state: spherical coordinates around `target`.
#[derive(Resource)]
pub struct OrbitCamera {
    pub yaw: f32,
    pub pitch: f32,
    pub radius: f32,
    pub target: Vec3,
    /// When true the camera slowly rotates on its own.
    pub auto: bool,
}

impl OrbitCamera {
    /// Closest the camera may orbit to its target. Low enough to dip inside the
    /// tank and swim among the fish (the water tint fades out as it crosses in).
    pub const MIN_RADIUS: f32 = 8.0;
    /// Furthest the camera may orbit from its target. Used both to clamp zoom and
    /// to size the sun's shadow cascade so it always covers the whole tank.
    pub const MAX_RADIUS: f32 = 160.0;
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            yaw: 0.6,
            pitch: 0.35,
            radius: 72.0,
            target: Vec3::ZERO,
            auto: true,
        }
    }
}

/// Read keyboard input and update the orbit state.
pub fn camera_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut orbit: ResMut<OrbitCamera>,
) {
    let dt = time.delta_secs();
    let rot_speed = 1.2;
    let zoom_speed = 40.0;

    if keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA) {
        orbit.yaw -= rot_speed * dt;
    }
    if keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD) {
        orbit.yaw += rot_speed * dt;
    }
    if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::KeyW) {
        orbit.pitch += rot_speed * dt;
    }
    if keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::KeyS) {
        orbit.pitch -= rot_speed * dt;
    }
    if keys.pressed(KeyCode::KeyQ) || keys.pressed(KeyCode::Equal) {
        orbit.radius -= zoom_speed * dt;
    }
    if keys.pressed(KeyCode::KeyE) || keys.pressed(KeyCode::Minus) {
        orbit.radius += zoom_speed * dt;
    }
    if keys.just_pressed(KeyCode::Space) {
        orbit.auto = !orbit.auto;
    }

    if orbit.auto {
        orbit.yaw += 0.15 * dt;
    }

    orbit.pitch = orbit.pitch.clamp(-1.45, 1.45);
    // Lower bound lets the camera dip inside the tank to swim among the fish; the
    // water-tint box fades out as it crosses in (see `fade_water_tint`) so the
    // close-up view stays clean.
    orbit.radius = orbit.radius.clamp(OrbitCamera::MIN_RADIUS, OrbitCamera::MAX_RADIUS);
}

/// Apply the orbit state to the camera transform.
pub fn camera_apply(orbit: Res<OrbitCamera>, mut cam: Query<&mut Transform, With<Camera3d>>) {
    let Ok(mut transform) = cam.single_mut() else {
        return;
    };
    let (sy, cy) = orbit.yaw.sin_cos();
    let (sp, cp) = orbit.pitch.sin_cos();
    let offset = Vec3::new(orbit.radius * cp * sy, orbit.radius * sp, orbit.radius * cp * cy);
    transform.translation = orbit.target + offset;
    transform.look_at(orbit.target, Vec3::Y);
}

/// Fade the blue water-tint box out as the camera zooms inside the tank.
///
/// The tint reads as the blue body of water *when viewed from outside*. Once the
/// camera dips inside the box its far wall smears a flat blue sheet across the
/// view that no amount of zooming resolves — so we ramp the tint's alpha down to
/// zero as the camera crosses from outside the tank to within it, leaving a clean
/// view of the fish when you're swimming among them.
pub fn fade_water_tint(
    orbit: Res<OrbitCamera>,
    cfg: Res<SimConfig>,
    tint: Query<&MeshMaterial3d<StandardMaterial>, With<WaterTint>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // The box's horizontal half-diagonal: the camera (which orbits in a sphere)
    // is fully outside the box only once its radius clears this. Fade across a
    // band ending here so the tint is gone by the time the camera is inside.
    let half = cfg.bounds;
    let outside_radius = (half.x * half.x + half.z * half.z).sqrt();
    let fade_start = outside_radius + 8.0;

    // 0 when inside (radius <= outside_radius), 1 when comfortably outside.
    let t = ((orbit.radius - outside_radius) / (fade_start - outside_radius)).clamp(0.0, 1.0);
    let alpha = 0.28 * t;

    for handle in &tint {
        if let Some(mut material) = materials.get_mut(&handle.0) {
            material.base_color.set_alpha(alpha);
        }
    }
}
