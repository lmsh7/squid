//! Headless renderer: runs the simulation with no window, renders each frame to
//! an offscreen image (works on software Vulkan / lavapipe), and saves a
//! sequence of PNGs that can be assembled into a GIF.
//!
//! Run with e.g. `CAPTURE_DIR=/tmp/squidcap cargo run --release --bin capture`.

use std::time::Duration;

use bevy::app::{AppExit, ScheduleRunnerPlugin};
use bevy::asset::RenderAssetUsages;
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::render::view::Hdr;
use bevy::window::ExitCondition;
use bevy::winit::WinitPlugin;

use squid::camera::{camera_apply, camera_input, OrbitCamera};
use squid::config::{Score, SimConfig};
use squid::flocking::{flocking_system, hunting_system, movement_system};
use squid::setup::{beam_bloom, setup_scene, sun_rays, water_fog};
use squid::ui::{draw_bounds, update_stats};
use squid::water::{drift_particles, spawn_water_particles};

const WIDTH: u32 = 960;
const HEIGHT: u32 = 540;

/// Handle of the offscreen image the camera renders into.
#[derive(Resource)]
struct RenderImage(Handle<Image>);

/// Drives the capture loop: warm up, then save one PNG per frame.
#[derive(Resource)]
struct Capture {
    frame: u32,
    warmup: u32,
    frames: u32,
    out_dir: String,
}

fn main() {
    let out_dir = std::env::var("CAPTURE_DIR").unwrap_or_else(|_| "/tmp/squidcap".into());
    std::fs::create_dir_all(&out_dir).expect("create capture dir");

    let warmup: u32 = env_u32("CAPTURE_WARMUP", 40);
    let frames: u32 = env_u32("CAPTURE_FRAMES", 120);

    App::new()
        .add_plugins(
            DefaultPlugins
                .build()
                .disable::<WinitPlugin>()
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: ExitCondition::DontExit,
                    close_when_requested: false,
                    ..default()
                }),
        )
        .add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            1.0 / 30.0,
        )))
        .insert_resource(ClearColor(Color::srgb(0.05, 0.22, 0.34)))
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.5, 0.7, 1.0),
            brightness: 220.0,
            ..default()
        })
        .init_resource::<SimConfig>()
        .init_resource::<Score>()
        .insert_resource(OrbitCamera::default())
        .insert_resource(Capture {
            frame: 0,
            warmup,
            frames,
            out_dir,
        })
        .add_systems(
            Startup,
            (setup_capture_camera, setup_scene, spawn_water_particles),
        )
        .add_systems(
            Update,
            (
                flocking_system,
                movement_system,
                drift_particles,
                hunting_system,
                camera_input,
                camera_apply,
                update_stats,
                draw_bounds,
                capture_system,
            )
                .chain(),
        )
        .run();
}

fn env_u32(key: &str, default: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn setup_capture_camera(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let size = Extent3d {
        width: WIDTH,
        height: HEIGHT,
        depth_or_array_layers: 1,
    };
    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::all(),
    );
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
        | TextureUsages::COPY_SRC
        | TextureUsages::COPY_DST
        | TextureUsages::RENDER_ATTACHMENT;
    let handle = images.add(image);

    commands.spawn((
        Camera3d::default(),
        RenderTarget::Image(handle.clone().into()),
        Hdr,
        beam_bloom(),
        Transform::from_xyz(0.0, 25.0, 72.0).looking_at(Vec3::ZERO, Vec3::Y),
        water_fog(),
        sun_rays(),
    ));

    commands.insert_resource(RenderImage(handle));
}

fn capture_system(
    mut commands: Commands,
    mut cap: ResMut<Capture>,
    image: Res<RenderImage>,
    mut exit: MessageWriter<AppExit>,
) {
    let f = cap.frame;
    cap.frame += 1;

    if f < cap.warmup {
        return;
    }
    let idx = f - cap.warmup;
    if idx < cap.frames {
        let path = format!("{}/frame_{idx:04}.png", cap.out_dir);
        commands
            .spawn(Screenshot::image(image.0.clone()))
            .observe(save_to_disk(path));
    } else if idx >= cap.frames + 8 {
        // Give the last few screenshots time to flush, then quit.
        exit.write(AppExit::Success);
    }
}
