# 🦑 Squid vs Tuna — 3D Boids

A 3D flocking simulation built with the [Bevy](https://bevyengine.org) game
engine, inspired by Jonas Lindstrøm's [Boids](https://github.com/jonas-lj/Boids)
project and reimagined in three dimensions with a predator/prey twist.

![Squid vs Tuna preview](assets/preview.gif)

> The preview above is rendered by the app itself running **headless** on
> software Vulkan (lavapipe) — see [Rendering a preview](#rendering-a-preview).

Two schools share a tank filled with **moving water**: a swirling current drags
the schools around and stirs them into eddies, drifting "marine snow" particles
sink through the volume, and distance fog dissolves far-off fish into the murky
blue — so the tank reads as a body of water, not empty space.

Two schools share that tank:

- **🦑 Squid (prey)** — flock tightly using classic Reynolds rules and **ball up**
  into a tight bait ball, then **flee** as one unit when tuna close in. They are
  nimble and pivot sharply to dodge.
- **🐟 Tuna (predators)** — bigger and faster, they **hunt as a pack**: the whole
  group locks onto the centre of a squid school and **charges** it with a speed
  burst. Their wide turning radius means a charge overshoots and sweeps around
  for another pass. When a tuna catches a squid, the squid is eaten and respawns
  elsewhere, so the populations stay constant and the chase never ends.

## The boids model

Every fish steers using three local rules over its same-species neighbours,
plus an inter-species interaction and a soft tank boundary:

| Rule | Description |
| --- | --- |
| **Separation** | Steer away from crowding neighbours (inverse-square weighted). |
| **Alignment** | Match the average heading of nearby neighbours. |
| **Cohesion** | Steer toward the average position of nearby neighbours. |
| **Flee / Hunt** | Squid steer away from nearby tuna; tuna steer toward the closest squid. |
| **Boundary** | A restoring force keeps each fish inside the tank. |
| **Current** | The surrounding water pushes each fish along the local flow, so the whole school drifts and curls with the eddies. |

All tuning lives in [`src/config.rs`](src/config.rs) (`SimConfig`): tank size,
school sizes, per-species speeds and forces, perception radii, the
flee/hunt/eat distances, and the water (`WaterParams`: current strength/scale,
bulk drift, and the drifting particles). Tweak and re-run to change the dynamics.

## The water body

The water is a lightweight, dependency-free effect built on what Bevy 0.16
already ships, so it also runs under the headless software renderer:

- **Current** — an analytic velocity field (a slow bulk drift plus a few
  out-of-phase, slowly animating sinusoidal swirls) sampled per fish in
  [`src/water.rs`](src/water.rs). It's added to each boid as an external force,
  so it's still bounded by the species' speed envelope and turning radius.
- **Marine snow** — a few hundred faint particles that the current advects and
  that slowly sink, wrapping back in from the top so the volume stays populated.
- **Distance fog** — an exponential `DistanceFog` on the camera tints and fades
  distant fish into the water colour.

## Running

Requires a recent Rust toolchain.

```bash
cargo run --release
```

On Linux you'll need the usual Bevy build dependencies (ALSA + udev), e.g. on
Debian/Ubuntu:

```bash
sudo apt-get install -y libasound2-dev libudev-dev pkg-config
```

## Controls

| Key | Action |
| --- | --- |
| `W` `A` `S` `D` / Arrow keys | Orbit the camera |
| `Q` / `E` | Zoom in / out |
| `Space` | Toggle automatic orbiting |

## Project layout

| File | Responsibility |
| --- | --- |
| `src/main.rs` | The game binary: windowed app, plugins, and system schedule. |
| `src/lib.rs` | Shared library re-exporting the simulation modules. |
| `src/bin/capture.rs` | Headless renderer that saves a PNG sequence (no window). |
| `src/components.rs` | ECS components (`Species`, `Velocity`, markers). |
| `src/config.rs` | `SimConfig` tuning resource and the `Score` tally. |
| `src/setup.rs` | Spawns the lights, tank, and both schools. |
| `src/flocking.rs` | The boids rules, movement, and the hunting/eating logic. |
| `src/water.rs` | The water current field plus the drifting "marine snow" particles. |
| `src/camera.rs` | Orbit camera state and input. |
| `src/ui.rs` | Heads-up display and the wireframe tank boundary. |

## Rendering a preview

The `capture` binary runs the simulation with **no window**, rendering each
frame to an offscreen image and saving it as a PNG. This works even without a
GPU by using a software Vulkan driver such as Mesa's lavapipe:

```bash
# One-time: software Vulkan + an encoder (Debian/Ubuntu)
sudo apt-get install -y mesa-vulkan-drivers ffmpeg

# Render a PNG sequence to $CAPTURE_DIR
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json WGPU_BACKEND=vulkan \
CAPTURE_DIR=/tmp/squidcap CAPTURE_WARMUP=50 CAPTURE_FRAMES=160 \
cargo run --bin capture

# Encode the frames into a GIF
ffmpeg -y -framerate 24 -i /tmp/squidcap/frame_%04d.png \
  -vf "fps=20,scale=600:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse" \
  assets/preview.gif
```

`CAPTURE_WARMUP` skips initial frames so the schools settle before capture, and
`CAPTURE_FRAMES` sets how many frames to save.

## License

MIT.
