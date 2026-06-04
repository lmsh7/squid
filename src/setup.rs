//! Scene construction: camera, lights, the tank, and the two schools of boids.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use bevy::light::{FogVolume, NotShadowCaster, VolumetricFog, VolumetricLight};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::post_process::bloom::{Bloom, BloomCompositeMode, BloomPrefilter};
use bevy::prelude::*;
use bevy::render::render_resource::Face;
use bevy::camera::Hdr;
use rand::Rng;

use crate::components::{
    FishPart, RestPose, Species, Squid, StatsText, SwimGait, Tuna, Velocity, WaterTint,
};
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
    tuna_finlet: Handle<Mesh>,
    tuna_pectoral: Handle<Mesh>,
    tuna_belly: Handle<Mesh>,
    tuna_mat: Handle<StandardMaterial>,
    tuna_belly_mat: Handle<StandardMaterial>,
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
    // `Atmospheric` falloff attenuates each R/G/B channel independently — the
    // real-water cue. Water absorbs red light within metres but lets blue-green
    // travel far, so an `extinction_color` that keeps blue-green and an
    // `inscattering_color` of the same water blue make distance read as genuine
    // underwater depth (warm tones drop out first, the volume turns teal-blue and
    // fades into the matching background) rather than a flat blue tint. Unlike a
    // `FogVolume` this is purely per-distance, so it has no hard boundary to flare
    // along and never jumps as the camera orbits. `visibility` is the clear-water
    // sight distance in world units; the tank is ~56 across.
    DistanceFog {
        falloff: FogFalloff::from_visibility_colors(
            48.0,
            // Extinction: blue-green survives, red is scrubbed out with distance.
            Color::srgb(0.42, 0.62, 0.70),
            // In-scattering: the water's own blue-teal glow added back over distance.
            Color::srgb(0.06, 0.30, 0.46),
        ),
        ..default()
    }
}

/// Bloom for the light shafts: a moderate threshold so only the genuinely bright
/// shaft highlights bloom, not the whole moderately-lit water volume — a low
/// threshold plus additive compositing was pushing the entire tank to a washed-out
/// white. Shared by both cameras.
pub fn beam_bloom() -> Bloom {
    Bloom {
        intensity: 0.35,
        prefilter: BloomPrefilter {
            threshold: 0.7,
            threshold_softness: 0.2,
        },
        composite_mode: BloomCompositeMode::Additive,
        ..Bloom::OLD_SCHOOL
    }
}

/// TRIAL (bevy 0.19): camera-side volumetric fog so the `FogVolume` renders.
/// Jitter softens the raymarch banding. Shared by both cameras.
pub fn sun_rays() -> VolumetricFog {
    VolumetricFog {
        jitter: 0.4,
        ..default()
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
        // Roll the bright HDR water back to clear blue instead of letting it
        // clip to white. Without a tonemapper the ambient + volumetric glow
        // overexposes the whole tank to a washed-out haze.
        Tonemapping::TonyMcMapface,
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
    // The "sun": a directional light straight overhead, shining vertically down
    // into the tank so it lights the fish from above evenly rather than from one
    // side. Shadows are off: with a single enclosing tank there is nothing to
    // cast a meaningful shadow, and a shadowed sun only complicated the water.
    commands.spawn((
        DirectionalLight {
            // Bright, sun-like: lights the fish surfaces from above.
            illuminance: 20_000.0,
            // TRIAL (bevy 0.19): shadows back on to feed volumetric scatter, to
            // test whether the 0.19 fog-volume fix (#22574) removes the orbiting
            // bright-edge / jumping-focus artifact.
            shadow_maps_enabled: true,
            ..default()
        },
        // Straight down (-Y). The `up` reference for `look_at` must be horizontal
        // (`Vec3::Z`): with a vertical look direction, `Vec3::Y` would be parallel
        // to the view and the rotation would be undefined.
        Transform::from_xyz(0.0, 60.0, 0.0).looking_at(Vec3::ZERO, Vec3::Z),
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

    // Volumetric fog, sized just a touch larger than the tank (1.1× the
    // half-extent) so its hard AABB boundary — where the raymarch flares a
    // bright, view-dependent edge (a known FogVolume artifact, bevy #22574 /
    // #18371) — sits just *past* the wireframe instead of on it. The tank
    // interior then shows the volume's even haze without the jumping edge, while
    // the fog still hugs the water body closely.
    commands.spawn((
        FogVolume {
            fog_color: Color::srgb(0.12, 0.38, 0.58),
            density_factor: 0.028,
            scattering: 0.5,
            absorption: 0.02,
            scattering_asymmetry: 0.3,
            light_intensity: 10.0,
            light_tint: Color::srgb(0.8, 0.9, 1.0),
            ..default()
        },
        Transform::from_scale(cfg.bounds * 2.2),
    ));

    // A faint translucent blue box, exactly the size of the wireframe tank, so
    // the water reads as a clear body filling the tank right to its edges (通透)
    // — you see straight through it to the fish, with a gentle blue tint. Unlit
    // so it's a pure tint rather than a shaded solid, and not a shadow caster so
    // it doesn't darken the scene.
    //
    // A `Blend` material writes no depth and is sorted as a whole by its centre's
    // distance to the camera. This box is centred on the origin and encloses the
    // entire tank, so most fish sit *closer* to the camera than the box centre —
    // the renderer then draws the box *over* them and the blue tint clings to the
    // camera-facing side like a film stuck to the lens. Culling the front faces
    // (`Face::Front`) draws only the box's *far* wall, behind every fish, so the
    // tint reads as the blue depth of the water rather than a sheet on the lens.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(cfg.bounds * 2.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.10, 0.40, 0.58, 0.28),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: Some(Face::Front),
            ..default()
        })),
        NotShadowCaster,
        WaterTint,
    ));

    // --- Creature assets ------------------------------------------------
    // Forward is local -Z (the eyes lead); the tail trails at +Z, up is +Y.
    // Every fin is baked into its rest pose so the entity carrying it can pivot
    // cleanly at the joint (see `baked`).
    let assets = CreatureAssets {
        // Squid mantle: a long, slender cone tapering to a point at the tail (+Z),
        // ~4:1 longer than wide. The cone's flat base sits at the head end (-Z);
        // the head dome (below) caps it so the body reads as a rounded bullet
        // rather than an open cone.
        squid_mantle: meshes.add(baked(
            Cone {
                radius: 0.18,
                height: 1.5,
            },
            Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
        )),
        // A lateral fin: the rhomboid "wing" along the rear third of the mantle
        // (as in the reference) — a flat triangular blade jutting out to +X,
        // stretched along the body axis (Z) so the pair reads as the squid's
        // arrowhead tail fins. Pivots at its inner edge (origin) to roll.
        squid_fin: meshes.add(baked(
            Cone {
                radius: 0.16,
                height: 0.42,
            },
            Transform {
                translation: Vec3::new(0.16, 0.0, 0.0),
                rotation: Quat::from_rotation_z(-FRAC_PI_2),
                // Thin (X), normal span (Y), stretched along the body (Z) into a
                // long rhomboid rather than a short paddle.
                scale: Vec3::new(0.12, 1.0, 2.2),
            },
        )),
        // Head: a rounded dome the same radius as the mantle base, so it caps the
        // cone's flat front into a smooth bullet nose and tapers slightly forward
        // into the short neck that the arm crown hangs from. Radius matches the
        // mantle base (0.18); stretched a little along the body axis.
        squid_head: meshes.add(baked(
            Sphere::new(0.18),
            Transform::from_scale(Vec3::new(0.92, 0.92, 1.15)),
        )),
        // A single arm: a long, very slender tapered cone reaching forward (-Z)
        // from its base at the origin. Thinner and a touch longer than before so
        // the bundle reads as the squid's fine crown of arms. A unit-height cone
        // here; each arm is scaled per-instance so the two tentacles can be made
        // longer than the eight arms.
        squid_arm: meshes.add(baked(
            Cone {
                radius: 0.03,
                height: 1.0,
            },
            Transform {
                translation: Vec3::new(0.0, 0.0, -0.5),
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
        // Tuna body: a sleek fusiform ellipsoid. A bluefin is laterally compressed
        // (taller than wide) and roughly 3× longer than deep, so the half-axes are
        // narrow in X, deeper in Y, and long in Z — a far more streamlined torpedo
        // than the earlier near-round body. Baked forward so the pivot sits near
        // the head and the flank undulates behind it.
        tuna_body: meshes.add(baked(
            Sphere::new(1.0),
            Transform {
                translation: Vec3::new(0.0, 0.0, 0.5),
                rotation: Quat::IDENTITY,
                scale: Vec3::new(0.30, 0.46, 1.35),
            },
        )),
        // Caudal fin LOBE: one blade of the bluefin's forked lunate tail. A thin,
        // narrow cone with its base at the peduncle (origin) and its point swept
        // outward; two of these, angled up and down, make the deep V/crescent.
        // Thin side-to-side (X), long along its own reach (post-rotation Y here is
        // the body Z), narrow fore-aft. The lobe points along +Z before tilting.
        tuna_tail: meshes.add(baked(
            Cone {
                radius: 0.26,
                height: 1.05,
            },
            Transform {
                // Lay the cone along +Z (point trailing aft), thin in X and narrow
                // fore-aft so each lobe is a slender, stiff blade.
                translation: Vec3::new(0.0, 0.0, 0.525),
                rotation: Quat::from_rotation_x(-FRAC_PI_2),
                scale: Vec3::new(0.11, 1.0, 0.42),
            },
        )),
        // First dorsal fin: a tall blade standing on the back, baked so its base
        // sits at the entity origin. Thin side-to-side, raised, swept back a touch.
        tuna_dorsal: meshes.add(baked(
            Cone {
                radius: 0.26,
                height: 0.5,
            },
            Transform {
                translation: Vec3::new(0.0, 0.25, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::new(0.1, 1.0, 0.8),
            },
        )),
        // A small finlet: the row of tiny triangular fins running along the back
        // and belly toward the tail on tunas. A flat little blade, base at origin.
        tuna_finlet: meshes.add(baked(
            Cone {
                radius: 0.07,
                height: 0.12,
            },
            Transform {
                translation: Vec3::new(0.0, 0.06, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::new(0.08, 1.0, 0.7),
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
        // Bluefin back: a dark steel blue with a metallic sheen, as in the
        // reference. The belly is a separate pale material (see `tuna_belly_mat`)
        // for the classic countershading.
        tuna_mat: materials.add(StandardMaterial {
            base_color: Color::srgb(0.13, 0.22, 0.40),
            perceptual_roughness: 0.3,
            metallic: 0.6,
            ..default()
        }),
        // Belly: a slightly smaller fusiform shell hugging the lower body, in pale
        // silver — the bright underside of the bluefin's countershading. Sits just
        // below the body centre so only the lower flank reads as silver.
        tuna_belly: meshes.add(baked(
            Sphere::new(1.0),
            Transform {
                translation: Vec3::new(0.0, -0.12, 0.5),
                rotation: Quat::IDENTITY,
                scale: Vec3::new(0.29, 0.34, 1.3),
            },
        )),
        tuna_belly_mat: materials.add(StandardMaterial {
            base_color: Color::srgb(0.82, 0.85, 0.88),
            perceptual_roughness: 0.25,
            metallic: 0.5,
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
            font_size: FontSize::Px(18.0),
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
                    // Rear third of the longer mantle (tail end is +Z at 0.75).
                    translation: Vec3::new(side * 0.16, 0.0, 0.5),
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
            // Head dome capping the mantle's flat front (cone base at -Z 0.75),
            // overlapping it slightly for a seamless bullet nose.
            p.spawn((
                Mesh3d(assets.squid_head.clone()),
                MeshMaterial3d(assets.squid_mat.clone()),
                Transform::from_xyz(0.0, 0.0, -0.7),
            ));
            // Arm/tentacle crown: eight shorter arms splayed in a ring, plus two
            // longer feeding tentacles reaching further forward (as in the
            // reference). The whole crown sways as one on its pivot. Each entry is
            // (yaw, pitch, length) — the arm mesh is a unit-length cone scaled by
            // `length` along its reach (local Z, which becomes -Z after the pivot
            // rotation), so the two tentacles stretch past the eight arms.
            let arms = Transform::from_xyz(0.0, -0.02, -0.7);
            p.spawn((arms, RestPose(arms), FishPart::Tentacles, Visibility::default()))
                .with_children(|a| {
                    // Eight arms ringed around the crown, ~0.55 long.
                    let crown = [
                        (-0.42_f32, 0.10_f32),
                        (-0.30, -0.06),
                        (-0.16, 0.14),
                        (-0.05, -0.12),
                        (0.05, 0.12),
                        (0.16, -0.14),
                        (0.30, 0.06),
                        (0.42, -0.10),
                    ];
                    for (yaw, pitch) in crown {
                        a.spawn((
                            Mesh3d(assets.squid_arm.clone()),
                            MeshMaterial3d(assets.squid_mat.clone()),
                            Transform {
                                rotation: Quat::from_rotation_y(yaw)
                                    * Quat::from_rotation_x(pitch),
                                scale: Vec3::new(1.0, 1.0, 0.55),
                                ..default()
                            },
                        ));
                    }
                    // Two long feeding tentacles, slightly thinner and ~1.5× the
                    // arm length, reaching straight ahead.
                    for sx in [-0.07_f32, 0.07] {
                        a.spawn((
                            Mesh3d(assets.squid_arm.clone()),
                            MeshMaterial3d(assets.squid_mat.clone()),
                            Transform {
                                rotation: Quat::from_rotation_y(sx),
                                scale: Vec3::new(0.7, 0.7, 0.95),
                                ..default()
                            },
                        ));
                    }
                });
            // Eyes on the head (-Z front), set on the sides of the head sphere.
            for sx in [-1.0_f32, 1.0] {
                p.spawn((
                    Mesh3d(assets.eye_mesh.clone()),
                    MeshMaterial3d(assets.eye_mat.clone()),
                    Transform::from_xyz(sx * 0.15, 0.06, -0.78),
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
                // Silver belly shell for countershading, riding the same wave.
                b.spawn((
                    Mesh3d(assets.tuna_belly.clone()),
                    MeshMaterial3d(assets.tuna_belly_mat.clone()),
                    Transform::IDENTITY,
                ));
                // Caudal fin: a forked, lunate tail. The pivot at the peduncle
                // (body-local +Z) carries the swimming yaw; two lobes hang off it
                // angled up and down so they splay into the deep V of a bluefin's
                // tail. The pivot beats for thrust (FishPart::TailFin), sweeping
                // both lobes together.
                let tail = Transform::from_xyz(0.0, 0.0, 1.75);
                b.spawn((tail, FishPart::TailFin, RestPose(tail), Visibility::default()))
                    .with_children(|t| {
                        // Upper and lower lobes, only slightly splayed (~16°) so the
                        // pair reads as a tall, narrow, deep crescent — not a wide
                        // open scissor. A tuna's tail is nearly vertical with just a
                        // shallow fork.
                        for dir in [1.0_f32, -1.0] {
                            t.spawn((
                                Mesh3d(assets.tuna_tail.clone()),
                                MeshMaterial3d(assets.tuna_mat.clone()),
                                Transform::from_rotation(Quat::from_rotation_x(
                                    dir * 0.28,
                                )),
                            ));
                        }
                    });
                // First (front) dorsal fin, taller, over the mid-back.
                b.spawn((
                    Mesh3d(assets.tuna_dorsal.clone()),
                    MeshMaterial3d(assets.tuna_mat.clone()),
                    Transform::from_xyz(0.0, 0.5, 0.35),
                ));
                // Second (rear) dorsal fin, a touch smaller and set further back.
                b.spawn((
                    Mesh3d(assets.tuna_dorsal.clone()),
                    MeshMaterial3d(assets.tuna_mat.clone()),
                    Transform {
                        translation: Vec3::new(0.0, 0.46, 0.95),
                        scale: Vec3::splat(0.7),
                        ..default()
                    },
                ));
                // Finlet rows running from behind the second dorsal toward the
                // tail, on both the back (+Y) and the belly (-Y).
                for i in 0..4 {
                    let z = 1.15 + i as f32 * 0.16;
                    let taper = 1.0 - i as f32 * 0.12;
                    // Dorsal (top) finlet.
                    b.spawn((
                        Mesh3d(assets.tuna_finlet.clone()),
                        MeshMaterial3d(assets.tuna_mat.clone()),
                        Transform {
                            translation: Vec3::new(0.0, 0.34, z),
                            scale: Vec3::splat(taper),
                            ..default()
                        },
                    ));
                    // Ventral (bottom) finlet, flipped under the body.
                    b.spawn((
                        Mesh3d(assets.tuna_finlet.clone()),
                        MeshMaterial3d(assets.tuna_mat.clone()),
                        Transform {
                            translation: Vec3::new(0.0, -0.34, z),
                            rotation: Quat::from_rotation_z(PI),
                            scale: Vec3::splat(taper),
                        },
                    ));
                }
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
