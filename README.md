# Black Splat

A 3d renderer built with **Rust** and **wgpu**

## Setup

1. Install Rust (includes cargo) via [rustup](https://rustup.rs).
2. Verify the toolchain:
   ```sh
   cargo --version
   ```

For browser (wasm) builds, also:

3. Add the wasm target:
   ```sh
   rustup target add wasm32-unknown-unknown
   ```
4. Install the `wasm-bindgen` CLI (must match the `wasm-bindgen` crate version in `Cargo.lock`):
   ```sh
   cargo install wasm-bindgen-cli --version 0.2.126
   ```
5. Install [Python 3](https://www.python.org/) to serve the build locally.

For testing the **Gaussian-splat** demo on a phone (it needs WebGPU over HTTPS — see below), also:

6. Install `cloudflared` for an HTTPS tunnel:
   ```sh
   scoop install cloudflared
   ```

## Building and Debugging

Run the examples from a command prompt:

| Demo | Directory | Command |
| --- | --- | --- |
| 2D demo #1 | this directory | `cargo run` |
| 2D demo #2 | `examples/2d/` | `cargo run` |
| 3D demo #3 | `examples/3d/` | `cargo run` |
| Gaussian splats | `examples/splat/` | `cargo run --release` |

> **Note:** Set your working directory to the root of the `black_splat` folder when debugging in Visual Studio, running from RenderDoc, etc.

## Running in a Browser (WASM)

The build system lives in `launcher/build.py` (the single source of truth for
compiling to wasm, running `wasm-bindgen`, and copying `index.html` + runtime
assets into the serve directory). Run any example's browser build with:

```sh
python launcher/build.py run <example> wasm    # e.g. run splat wasm
```

Each example serves on its own port, assigned in discovery order from 8000
(`2d`→8000, `3d`→8001, `mujoco_test`→8002, `splat`→8003); `serve.py` sends
no-cache headers so you always get the fresh build — no incognito needed. The
splat demo also has a `build_wasm.bat` convenience wrapper that just invokes the
launcher for it. Re-run the build after editing `index.html` — the browser
loads the copy in the serve directory, which only a build refreshes.

> The splat demo can talk to a local policy server for robot-policy control;
> that server runs separately on port 8000 — see `policy_server/README.md`.

**Which backend a demo uses matters for the browser:**

| Demo | Browser backend | Needs |
| --- | --- | --- |
| 2D / 3D | WebGL2 | Any modern browser; works over plain http |
| Gaussian splats | WebGPU | A WebGPU browser (iOS 18.2+ / Chrome / Safari 18+) **and an HTTPS (secure) context** — the GPU radix sort uses compute shaders + storage buffers, which WebGL2 lacks |

### Testing on a phone

The `serve.py` server binds all interfaces, so any device on your Wi-Fi can reach it.

- **2D / 3D (WebGL2):** run `build_wasm.bat`, then on the phone open
  `http://<your-PC-LAN-IP>:8000/` (find the IP with `ipconfig`). Plain http is
  fine — WebGL2 doesn't require a secure context. You may need to allow inbound
  port 8000 through Windows Firewall the first time.

- **Gaussian splats (WebGPU):** WebGPU (`navigator.gpu`) is only exposed in a
  *secure context*, and a plain `http://<LAN-IP>` is not one — so LAN http gives
  a black canvas. Run `build_wasm_tunneled.bat` instead: it builds, serves, and
  starts a Cloudflare quick tunnel that gives a real `https://…trycloudflare.com`
  URL (requires `cloudflared` — see Setup). It prints a **QR code** — scan it with
  the phone camera to skip typing the URL. The tunnel URL stays valid across
  rebuilds, so you can bookmark it once and just refresh; it only changes when you
  restart the tunnel.

## Config File

Each example uses a config file that controls several parameters. There is an example at `GameAssets/game_config.txt`:

```json
{
    "enemy_spawn_delay": 0.3,
    "enemy_move_speed": 1.0,
    "max_instances": 2000,

    "window_width": 1920,
    "window_height": 1080,

    "_comment": "Valid values for 'graphics_back_end' are default, vulkan, or dx12",
    "graphics_back_end": "default",

    "_comment2": "Valid values for 'graphics_power_pref' are default, low, and high",
    "graphics_power_pref": "default",

    "_comment3": "Valid values for 'vsync' are true and false",
    "vsync": true

   "start_position": [-2.78, 2.27, 1.81],
    "start_rotation": [-243.4, 13.0, 0.0]
}
```

### TODO MUJOCO
$ npm install @mujoco/mujoco

## Resources

- [Project repository](https://github.com/bennywilson/kbEngine3)
- [The Rust Book](https://doc.rust-lang.org/book/ch01-00-getting-started.html)
- [Learn wgpu](https://sotrh.github.io/learn-wgpu/#what-is-wgpu)
- [Vulkan 1.3 specification](https://registry.khronos.org/vulkan/specs/1.3/html/vkspec.html)
- [Tracy profiler](https://github.com/wolfpld/tracy)
- [3D Gaussian Splatting for Real-Time Radiance Field Rendering + datasets](https://repo-sam.inria.fr/fungraph/3d-gaussian-splatting/)
---

Benny Wilson
bennywilson@benny-wilson.com
