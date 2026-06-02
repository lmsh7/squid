//! Scene construction: camera, lights, the tank, and the two schools of boids.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use rand::Rng;

use crate::components::{Species, Squid, StatsText, Tuna, Velocity};
use crate::config::SimConfig;
use crate::flocking::{random_point, random_velocity};

/// Pre-built mesh & material handles so spawning is cheap.
struct CreatureAssets {
    squid_body: Handle<Mesh>,
    squid_fin: Handle<Mesh>,
    squid_mat: Handle<StandardMaterial>,
    tuna_body: Handle<Mesh>,
    tuna_tail: Handle<Mesh>,
    tuna_mat: Handle<StandardMaterial>,
    eye_mesh: Handle<Mesh>,
    eye_mat: Handle<StandardMaterial>,
}

/// Spawn the windowed gameplay camera. (The headless capture binary spawns its
/// own camera that renders to an offscreen image instead.)
pub fn spawn_window_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 25.0, 72.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Spawn the lights, both schools of boids, and the HUD. Shared by the game and
/// the headless capture binary; the camera is added separately by each.
pub fn setup_scene(
    mut commands: Commands,
    cfg: Res<SimConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // --- Lighting -------------------------------------------------------
    commands.spawn((
        DirectionalLight {
            illuminance: 9000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(30.0, 60.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // A second, cooler fill light from below for an underwater feel.
    commands.spawn((
        PointLight {
            intensity: 2_000_000.0,
            range: 200.0,
            color: Color::srgb(0.4, 0.7, 1.0),
            ..default()
        },
        Transform::from_xyz(0.0, -40.0, 0.0),
    ));

    // --- Creature assets ------------------------------------------------
    let assets = CreatureAssets {
        squid_body: meshes.add(Capsule3d::new(0.18, 0.55)),
        squid_fin: meshes.add(Cone {
            radius: 0.28,
            height: 0.5,
        }),
        squid_mat: materials.add(StandardMaterial {
            base_color: Color::srgb(0.78, 0.32, 0.62),
            perceptual_roughness: 0.5,
            metallic: 0.1,
            ..default()
        }),
        tuna_body: meshes.add(Capsule3d::new(0.32, 1.1)),
        tuna_tail: meshes.add(Cone {
            radius: 0.4,
            height: 0.55,
        }),
        tuna_mat: materials.add(StandardMaterial {
            base_color: Color::srgb(0.32, 0.45, 0.62),
            perceptual_roughness: 0.35,
            metallic: 0.55,
            ..default()
        }),
        eye_mesh: meshes.add(Sphere::new(0.05)),
        eye_mat: materials.add(StandardMaterial {
            base_color: Color::srgb(0.02, 0.02, 0.02),
            ..default()
        }),
    };

    // --- Schools --------------------------------------------------------
    let mut rng = rand::thread_rng();
    for _ in 0..cfg.squid_count {
        spawn_squid(&mut commands, &assets, &cfg, &mut rng);
    }
    for _ in 0..cfg.tuna_count {
        spawn_tuna(&mut commands, &assets, &cfg, &mut rng);
    }

    // --- HUD ------------------------------------------------------------
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.93, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        StatsText,
    ));
}

fn spawn_squid(
    commands: &mut Commands,
    assets: &CreatureAssets,
    cfg: &SimConfig,
    rng: &mut impl Rng,
) {
    let pos = random_point(cfg, rng);
    let vel = random_velocity(cfg.squid.min_speed, cfg.squid.max_speed, rng);

    commands
        .spawn((
            Species::Squid,
            Squid,
            Velocity(vel),
            Transform::from_translation(pos),
            Visibility::default(),
        ))
        .with_children(|p| {
            // Mantle / body, lying along the local Z (forward) axis.
            p.spawn((
                Mesh3d(assets.squid_body.clone()),
                MeshMaterial3d(assets.squid_mat.clone()),
                Transform::from_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
            ));
            // Rear fin, apex pointing backward (+Z).
            p.spawn((
                Mesh3d(assets.squid_fin.clone()),
                MeshMaterial3d(assets.squid_mat.clone()),
                Transform::from_xyz(0.0, 0.0, 0.42)
                    .with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
            ));
            // Eyes near the front (-Z).
            for sx in [-1.0_f32, 1.0] {
                p.spawn((
                    Mesh3d(assets.eye_mesh.clone()),
                    MeshMaterial3d(assets.eye_mat.clone()),
                    Transform::from_xyz(sx * 0.11, 0.05, -0.18),
                ));
            }
        });
}

fn spawn_tuna(
    commands: &mut Commands,
    assets: &CreatureAssets,
    cfg: &SimConfig,
    rng: &mut impl Rng,
) {
    let pos = random_point(cfg, rng);
    let vel = random_velocity(cfg.tuna.min_speed, cfg.tuna.max_speed, rng);

    commands
        .spawn((
            Species::Tuna,
            Tuna,
            Velocity(vel),
            Transform::from_translation(pos),
            Visibility::default(),
        ))
        .with_children(|p| {
            // Torpedo body along local Z.
            p.spawn((
                Mesh3d(assets.tuna_body.clone()),
                MeshMaterial3d(assets.tuna_mat.clone()),
                Transform::from_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
            ));
            // Tail fin, flattened, apex backward (+Z).
            p.spawn((
                Mesh3d(assets.tuna_tail.clone()),
                MeshMaterial3d(assets.tuna_mat.clone()),
                Transform::from_xyz(0.0, 0.0, 0.85)
                    .with_rotation(Quat::from_rotation_x(FRAC_PI_2))
                    .with_scale(Vec3::new(1.4, 1.0, 0.4)),
            ));
            // Eyes near the front (-Z).
            for sx in [-1.0_f32, 1.0] {
                p.spawn((
                    Mesh3d(assets.eye_mesh.clone()),
                    MeshMaterial3d(assets.eye_mat.clone()),
                    Transform::from_xyz(sx * 0.18, 0.08, -0.55),
                ));
            }
        });
}
