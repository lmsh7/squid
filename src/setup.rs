//! Scene construction: camera, lights, the tank, and the two schools of boids.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use bevy::light::{FogVolume, NotShadowCaster, VolumetricFog, VolumetricLight};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::post_process::bloom::{Bloom, BloomCompositeMode, BloomPrefilter};
use bevy::prelude::*;
use bevy::render::view::Hdr;
use rand::Rng;

use crate::components::{FishPart, RestPose, Species, Squid, StatsText, SwimGait, Tuna, Velocity};
use crate::config::SimConfig;
use crate::flocking::{random_point, random_velocity};

/// Pre-built mesh & material handles so spawning is cheap.
///
/// Each fin/body mesh is baked into its rest orientation (see [`baked`]) so the
/// pivot entity that carries it can sit cleanly at the joint and the animation
/// system only has to add a rotation or a pulse.
struct CreatureAssets {
    // --- Squid ---
    squid_mantle: Handle<Mesh>,
    squid_fin: Handle<Mesh>,
    squid_head: Handle<Mesh>,
    squid_arm: Handle<Mesh>,
    squid_mat: Handle<StandardMaterial>,
    // --- Tuna ---
    tuna_body: Handle<Mesh>,
    tuna_tail: Handle<Mesh>,
    tuna_dorsal: Handle<Mesh>,
    tuna_pectoral: Handle<Mesh>,
    tuna_mat: Handle<StandardMaterial>,
    // --- Shared ---
    eye_mesh: Handle<Mesh>,
    eye_mat: Handle<StandardMaterial>,
}

/// Bake a transform into a primitive's geometry. This lets a fin be modelled in
/// convenient local axes, then baked so its *pivot* (the carrying entity's
/// origin) lands at the joint it swings about — keeping the animation maths
/// trivial (just add a rotation about the origin).
fn baked(mesh: impl Into<Mesh>, transform: Transform) -> Mesh {
    mesh.into().transformed_by(transform)
}

/// A random swim gait so no two fish in a school beat their fins in lockstep.
fn random_gait(rng: &mut impl Rng) -> SwimGait {
    SwimGait {
        phase: rng.gen_range(0.0..TAU),
        tempo: rng.gen_range(0.85..1.2),
    }
}

/// Underwater distance fog: distant fish dissolve into the surrounding water
/// colour, selling the sense of clear blue depth. Shared by the windowed and
/// headless cameras so both look the same.
pub fn water_fog() -> DistanceFog {
    DistanceFog {
        // Matches the clear-water background, so distance reads as translucent
        // blue depth (atmospheric perspective) rather than a murky black void.
        color: Color::srgb(0.05, 0.22, 0.34),
        // Gentle enough that the near water stays clear and see-through, with
        // the far wall fading into blue.
        falloff: FogFalloff::ExponentialSquared { density: 0.011 },
        ..default()
    }
}

/// Enables volumetric lighting on the camera, so the sun's `VolumetricLight`
/// scatters through the `FogVolume` into a soft glow of sunlight. Jitter softens
/// the raymarch banding. Shared by the windowed and headless cameras.
pub fn sun_rays() -> VolumetricFog {
    VolumetricFog {
        jitter: 0.4,
        ..default()
    }
}

/// Bloom for the light shafts: a low threshold so the moderately bright
/// volumetric scattering glows (not just fully blown-out pixels), composited
/// additively so it reads as light on top of the dark water. Shared by both
/// cameras.
pub fn beam_bloom() -> Bloom {
    Bloom {
        intensity: 0.35,
        prefilter: BloomPrefilter {
            threshold: 0.2,
            threshold_softness: 0.2,
        },
        composite_mode: BloomCompositeMode::Additive,
        ..Bloom::OLD_SCHOOL
    }
}

/// Spawn the windowed gameplay camera. (The headless capture binary spawns its
/// own camera that renders to an offscreen image instead.)
pub fn spawn_window_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        // HDR + bloom let the bright volumetric beams glow and bleed, which is
        // what makes them read as shafts of light rather than flat haze.
        Hdr,
        beam_bloom(),
        Transform::from_xyz(0.0, 25.0, 72.0).looking_at(Vec3::ZERO, Vec3::Y),
        water_fog(),
        sun_rays(),
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
    // The "sun": a directional light slanting down into the tank. `VolumetricLight`
    // makes it scatter through the water (see the fog volume below) into a soft
    // glow of sunlight. (Bevy 0.18 only scatters directional lights volumetrically.)
    commands.spawn((
        DirectionalLight {
            // Bright, sun-like: volumetric in-scattering scales with the light's
            // radiance, so a strong sun is what makes the water glow.
            illuminance: 20_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(30.0, 60.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        VolumetricLight,
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

    // The body of water the sun scatters through. A `FogVolume` is a unit cube
    // scaled by its transform, so this fills the whole tank; the sun above
    // scatters through it into a soft volumetric glow of sunlight.
    commands.spawn((
        FogVolume {
            fog_color: Color::srgb(0.12, 0.38, 0.58),
            // Thin and barely-absorbing: this only adds the soft sunlit glow
            // inside the water. The translucent box below is what gives the
            // water its clear blue body and crisp edges at the wireframe.
            density_factor: 0.09,
            scattering: 0.5,
            absorption: 0.03,
            // Forward scattering concentrates a soft halo of glow toward the sun
            // direction, reading as sunlight pouring down through the water.
            scattering_asymmetry: 0.6,
            light_intensity: 30.0,
            light_tint: Color::srgb(0.8, 0.9, 1.0),
            ..default()
        },
        Transform::from_scale(cfg.bounds * 2.0),
    ));

    // A faint translucent blue box, exactly the size of the wireframe tank, so
    // the water reads as a clear body filling the tank right to its edges (通透)
    // — you see straight through it to the fish, with a gentle blue tint. Unlit
    // so it's a pure tint rather than a shaded solid, and not a shadow caster so
    // it doesn't darken the scene. Double-sided (`cull_mode: None`) so every
    // view ray crosses both the entry and exit face: the tint then fills the
    // whole box evenly from any angle, instead of clinging to the faces that
    // happen to face the camera as the view orbits.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(cfg.bounds * 2.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.10, 0.36, 0.52, 0.11),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            ..default()
        })),
        NotShadowCaster,
    ));

    // --- Creature assets ------------------------------------------------
    // Forward is local -Z (the eyes lead); the tail trails at +Z, up is +Y.
    // Every fin is baked into its rest pose so the entity carrying it can pivot
    // cleanly at the joint (see `baked`).
    let assets = CreatureAssets {
        // Squid mantle: a tapered tube, blunt at the head (-Z) and drawn to a
        // point at the tail (+Z) — a cone laid along the body axis.
        squid_mantle: meshes.add(baked(
            Cone {
                radius: 0.3,
                height: 1.0,
            },
            Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
        )),
        // A lateral fin: a thin triangular blade jutting out to +X, pivoting at
        // its inner edge (the origin) so it can roll in a ripple.
        squid_fin: meshes.add(baked(
            Cone {
                radius: 0.22,
                height: 0.5,
            },
            Transform {
                translation: Vec3::new(0.25, 0.0, 0.0),
                rotation: Quat::from_rotation_z(-FRAC_PI_2),
                scale: Vec3::new(0.18, 1.0, 1.0),
            },
        )),
        squid_head: meshes.add(Sphere::new(0.2)),
        // A single arm: a slender tapered cone reaching forward (-Z) from its
        // base at the origin, so a bundle of them trails from the head.
        squid_arm: meshes.add(baked(
            Cone {
                radius: 0.045,
                height: 0.6,
            },
            Transform {
                translation: Vec3::new(0.0, 0.0, -0.3),
                rotation: Quat::from_rotation_x(-FRAC_PI_2),
                scale: Vec3::ONE,
            },
        )),
        squid_mat: materials.add(StandardMaterial {
            base_color: Color::srgb(0.78, 0.32, 0.62),
            perceptual_roughness: 0.5,
            metallic: 0.1,
            ..default()
        }),
        // Tuna body: a smooth fusiform ellipsoid, baked forward so the body
        // pivot can sit near the head and the whole flank undulates behind it.
        tuna_body: meshes.add(baked(
            Sphere::new(1.0),
            Transform {
                translation: Vec3::new(0.0, 0.0, 0.5),
                rotation: Quat::IDENTITY,
                scale: Vec3::new(0.38, 0.5, 1.0),
            },
        )),
        // Caudal fin: a tall vertical fan, thin side to side, pivoting at the
        // peduncle (origin) so it can sweep for thrust.
        tuna_tail: meshes.add(baked(
            Cone {
                radius: 0.5,
                height: 0.9,
            },
            Transform {
                translation: Vec3::new(0.0, 0.0, 0.5),
                rotation: Quat::from_rotation_x(-FRAC_PI_2),
                scale: Vec3::new(0.16, 0.9, 1.4),
            },
        )),
        // Dorsal fin: a small vertical blade standing on the back, baked so its
        // base sits at the entity origin.
        tuna_dorsal: meshes.add(baked(
            Cone {
                radius: 0.28,
                height: 0.45,
            },
            Transform {
                translation: Vec3::new(0.0, 0.225, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::new(0.12, 1.0, 0.9),
            },
        )),
        // Pectoral fin: a small blade out to +X, pivoting at the flank.
        tuna_pectoral: meshes.add(baked(
            Cone {
                radius: 0.18,
                height: 0.4,
            },
            Transform {
                translation: Vec3::new(0.2, 0.0, 0.0),
                rotation: Quat::from_rotation_z(-FRAC_PI_2),
                scale: Vec3::new(0.15, 1.0, 1.0),
            },
        )),
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
            random_gait(rng),
            Transform::from_translation(pos),
            Visibility::default(),
        ))
        .with_children(|p| {
            // Mantle: pulses in and out like a jetting squid.
            let mantle = Transform::IDENTITY;
            p.spawn((
                Mesh3d(assets.squid_mantle.clone()),
                MeshMaterial3d(assets.squid_mat.clone()),
                mantle,
                FishPart::Mantle,
                RestPose(mantle),
            ));
            // Lateral fins near the rear of the mantle, one per side, rolling in
            // a travelling ripple. The left fin is the +X blade flipped to -X.
            for side in [-1.0_f32, 1.0] {
                let rest = Transform {
                    translation: Vec3::new(side * 0.26, 0.0, 0.28),
                    rotation: if side < 0.0 {
                        Quat::from_rotation_y(PI)
                    } else {
                        Quat::IDENTITY
                    },
                    scale: Vec3::ONE,
                };
                p.spawn((
                    Mesh3d(assets.squid_fin.clone()),
                    MeshMaterial3d(assets.squid_mat.clone()),
                    rest,
                    FishPart::SquidFin { side },
                    RestPose(rest),
                ));
            }
            // Head, tucked just ahead of the mantle.
            p.spawn((
                Mesh3d(assets.squid_head.clone()),
                MeshMaterial3d(assets.squid_mat.clone()),
                Transform::from_xyz(0.0, 0.0, -0.5),
            ));
            // Arm/tentacle bundle: a sheaf of slender cones trailing from the
            // head, swaying as one on its pivot.
            let arms = Transform::from_xyz(0.0, -0.02, -0.55);
            p.spawn((arms, RestPose(arms), FishPart::Tentacles, Visibility::default()))
                .with_children(|a| {
                    let splay = [
                        (-0.30_f32, 0.06_f32),
                        (-0.15, -0.10),
                        (0.0, 0.12),
                        (0.15, -0.10),
                        (0.30, 0.06),
                    ];
                    for (yaw, pitch) in splay {
                        a.spawn((
                            Mesh3d(assets.squid_arm.clone()),
                            MeshMaterial3d(assets.squid_mat.clone()),
                            Transform::from_rotation(
                                Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch),
                            ),
                        ));
                    }
                });
            // Eyes on the head (-Z front).
            for sx in [-1.0_f32, 1.0] {
                p.spawn((
                    Mesh3d(assets.eye_mesh.clone()),
                    MeshMaterial3d(assets.eye_mat.clone()),
                    Transform::from_xyz(sx * 0.13, 0.07, -0.6),
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
            random_gait(rng),
            Transform::from_translation(pos),
            Visibility::default(),
        ))
        .with_children(|p| {
            // Body pivot near the head; the flank undulates behind it and the
            // tail + dorsal fin (its children) ride that wave.
            let body = Transform::from_xyz(0.0, 0.0, -0.5);
            p.spawn((
                body,
                RestPose(body),
                FishPart::Body,
                Visibility::default(),
            ))
            .with_children(|b| {
                // The fusiform body mesh, baked to sit centred on the fish.
                b.spawn((
                    Mesh3d(assets.tuna_body.clone()),
                    MeshMaterial3d(assets.tuna_mat.clone()),
                    Transform::IDENTITY,
                ));
                // Caudal fin at the peduncle (body-local +Z), beating for thrust.
                let tail = Transform::from_xyz(0.0, 0.0, 1.4);
                b.spawn((
                    Mesh3d(assets.tuna_tail.clone()),
                    MeshMaterial3d(assets.tuna_mat.clone()),
                    tail,
                    FishPart::TailFin,
                    RestPose(tail),
                ));
                // Dorsal fin standing on the back (static, but rides the body).
                b.spawn((
                    Mesh3d(assets.tuna_dorsal.clone()),
                    MeshMaterial3d(assets.tuna_mat.clone()),
                    Transform::from_xyz(0.0, 0.5, 0.5),
                ));
            });
            // Pectoral fins on the flanks, flapping gently. Left is the +X blade
            // flipped to -X.
            for side in [-1.0_f32, 1.0] {
                let rest = Transform {
                    translation: Vec3::new(side * 0.34, -0.12, -0.4),
                    rotation: if side < 0.0 {
                        Quat::from_rotation_y(PI)
                    } else {
                        Quat::IDENTITY
                    },
                    scale: Vec3::ONE,
                };
                p.spawn((
                    Mesh3d(assets.tuna_pectoral.clone()),
                    MeshMaterial3d(assets.tuna_mat.clone()),
                    rest,
                    FishPart::PectoralFin { side },
                    RestPose(rest),
                ));
            }
            // Eyes near the front (-Z).
            for sx in [-1.0_f32, 1.0] {
                p.spawn((
                    Mesh3d(assets.eye_mesh.clone()),
                    MeshMaterial3d(assets.eye_mat.clone()),
                    Transform::from_xyz(sx * 0.2, 0.12, -0.78),
                ));
            }
        });
}
