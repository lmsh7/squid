//! The water body: an analytic current field that advects everything in the
//! tank, plus drifting "marine snow" particles that give the volume a sense of
//! depth and motion. No shaders or extra dependencies — just ECS and a little
//! trigonometry, so it also runs under the headless software renderer.

use bevy::prelude::*;
use rand::Rng;

use crate::config::{SimConfig, WaterParams};
use crate::flocking::random_point;

/// Marker for a drifting water particle.
#[derive(Component)]
pub struct WaterParticle;

/// Sample the water current velocity (units/second) at a world position and
/// time. It's a smooth, slowly animating, swirling field built from a few
/// out-of-phase sinusoids, plus a constant bulk drift. The vertical component
/// is deliberately small so things mostly sway sideways, like a real tank.
pub fn current_at(pos: Vec3, t: f32, w: &WaterParams) -> Vec3 {
    let p = pos * w.current_scale;
    let ts = t * w.current_time_scale;

    let swirl = Vec3::new(
        (p.y * 1.3 + ts).sin() + (p.z * 0.7 - ts * 0.6).cos(),
        ((p.x * 0.9 - ts * 0.5).sin() + (p.z * 1.1 + ts * 0.4).cos()) * 0.25,
        (p.x * 1.2 - ts).cos() + (p.y * 0.8 + ts * 0.7).sin(),
    );

    w.drift + swirl * w.current_strength
}

/// Spawn the cloud of drifting particles that fills the tank. Shared by the
/// windowed game and the headless capture binary.
pub fn spawn_water_particles(
    mut commands: Commands,
    cfg: Res<SimConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Sphere::new(cfg.water.particle_size));
    // Faintly glowing pale motes so they read even in the dim, fogged water.
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.7, 0.82, 0.95),
        emissive: LinearRgba::rgb(0.25, 0.35, 0.5),
        unlit: true,
        ..default()
    });

    let mut rng = rand::thread_rng();
    for _ in 0..cfg.water.particle_count {
        commands.spawn((
            WaterParticle,
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(random_point(&cfg, &mut rng)),
        ));
    }
}

/// Advect every particle by the current and let it slowly sink, wrapping it
/// back into the tank (toroidally sideways, raining back from the top) so the
/// volume stays evenly populated.
pub fn drift_particles(
    time: Res<Time>,
    cfg: Res<SimConfig>,
    mut particles: Query<&mut Transform, With<WaterParticle>>,
) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    let b = cfg.bounds;
    let mut rng = rand::thread_rng();

    for mut tf in particles.iter_mut() {
        let flow = current_at(tf.translation, t, &cfg.water);
        tf.translation += flow * dt;
        tf.translation.y -= cfg.water.particle_fall_speed * dt;

        let p = &mut tf.translation;
        // Sunk below the floor: rain back in from the top at a fresh column.
        if p.y < -b.y {
            p.y = b.y;
            p.x = rng.gen_range(-b.x..b.x);
            p.z = rng.gen_range(-b.z..b.z);
        }
        // Wrap sideways so the current never sweeps the tank empty.
        if p.x > b.x {
            p.x -= 2.0 * b.x;
        } else if p.x < -b.x {
            p.x += 2.0 * b.x;
        }
        if p.z > b.z {
            p.z -= 2.0 * b.z;
        } else if p.z < -b.z {
            p.z += 2.0 * b.z;
        }
    }
}
