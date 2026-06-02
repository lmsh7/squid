# 🦑 Squid vs Tuna — 3D Boids

A 3D flocking simulation built with the [Bevy](https://bevyengine.org) game
engine, inspired by Jonas Lindstrøm's [Boids](https://github.com/jonas-lj/Boids)
project and reimagined in three dimensions with a predator/prey twist.

Two schools share a tank:

- **🦑 Squid (prey)** — flock tightly using classic Reynolds rules and **flee**
  from any tuna that gets too close.
- **🐟 Tuna (predators)** — school loosely and **hunt** the nearest squid. When a
  tuna catches a squid, the squid is eaten and respawns elsewhere, so the
  populations stay constant and the chase never ends.

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

All tuning lives in [`src/config.rs`](src/config.rs) (`SimConfig`): tank size,
school sizes, per-species speeds and forces, perception radii, and the
flee/hunt/eat distances. Tweak and re-run to change the dynamics.

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
| `src/main.rs` | App setup, plugins, and system schedule. |
| `src/components.rs` | ECS components (`Species`, `Velocity`, markers). |
| `src/config.rs` | `SimConfig` tuning resource and the `Score` tally. |
| `src/setup.rs` | Spawns the camera, lights, tank, and both schools. |
| `src/flocking.rs` | The boids rules, movement, and the hunting/eating logic. |
| `src/camera.rs` | Orbit camera state and input. |
| `src/ui.rs` | Heads-up display and the wireframe tank boundary. |

## License

MIT.
