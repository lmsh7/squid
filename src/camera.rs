//! A simple orbit camera that circles the tank and responds to the keyboard.

use bevy::prelude::*;

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
    orbit.radius = orbit.radius.clamp(20.0, 160.0);
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
