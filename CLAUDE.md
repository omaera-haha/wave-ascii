# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

`wave-ascii` is a Rust (edition 2024) terminal app that renders animated scenes as text, in color or plain. Its dependencies are `glam` for vector, matrix, and quaternion math, `resvg` (with `usvg` and `tiny-skia`) for SVG, and `gltf` for glTF models. It has no terminal crate: terminal control uses raw ANSI escapes and shells out to `stty`, so it runs only on Unix.

## Commands

The Rust toolchain lives in the dev container (`.devcontainer/`, based on `rust:latest`, which adds rustfmt and clippy). The host may not have `cargo` installed.

```bash
cargo check
cargo test
cargo test cube_hit            # run tests whose name contains a substring
cargo test scene::tests::      # run one module's tests
cargo fmt --check
cargo clippy
cargo run --release -- 2d      # interactive; also 3d, ocean, knot, wave, duck, a file path, or --help
```

You can't watch the interactive animation from a non-TTY shell. To check visible output, pipe it. When stdin or stdout isn't a terminal, the app prints one frame at `time = 0` and exits. The frame size comes from `COLUMNS`/`LINES` (default 80x24). Piped output is plain unless `--color` is given:

```bash
COLUMNS=60 LINES=20 cargo run -q -- 3d < /dev/null | cat
COLUMNS=60 LINES=20 cargo run -q -- ocean --style blocks --color truecolor < /dev/null | cat
```

To exercise the interactive loop, run it under `script` with a pty and feed keys on stdin, for example `(sleep 1; printf q) | script -qfec "stty rows 30 cols 100; ./target/release/wave-ascii" /dev/null`.

## Architecture

The rendering pipeline is: `Scene::render(grid, time, view)` → `Frame` → `Style::canvas` → `Canvas` → `Terminal::draw` (or `Canvas::ansi` when piped).

- **`frame.rs`** defines `Grid`, `Shade`, and `Frame`. A `Grid` is the pixel lattice a scene is sampled on. Its y axis runs from -1 to 1 across the height, and `aspect` is a pixel's width divided by its height, so a distance covers the same physical length in both directions. `CELL_ASPECT = 0.5` because a terminal cell is about twice as tall as it is wide, and a test checks this. A `Shade` is a brightness plus a hue at full intensity; `hue * brightness` is the lit color, and glyph density shows brightness. `Frame::sample` evaluates a closure at every pixel center, `Frame::build` does the same by column and row, and both split rows across threads for large frames, so closures must be `Fn + Sync`. Values are clamped to `[0, 1]`. `Grid::refine` and `Frame::downsample` supersample for anti-aliasing.
- **`canvas.rs`** turns a `Frame` into terminal `Cell`s. `Style::grid` picks the sampling grid for a style: `Ascii` uses one pixel per cell and the ramp `" .:-=+*#%@"`, `Blocks` uses two pixels per cell drawn as `▀` with foreground and background colors, and `Braille` uses 2x4 dots with ordered dithering. `Canvas::redraw` writes only the cells that differ from the previous canvas, positioning the cursor absolutely, and `Pen` writes color escapes only when colors change.
- **`color.rs`** has `ColorMode` (plain, 256-color palette, truecolor), detection from `NO_COLOR`, `TERM`, and `COLORTERM`, and the SGR encoding.
- **`scene.rs`** has one `Scene` variant per scene and the `View` (yaw, pitch, zoom) set by the arrow and zoom keys. Each scene is a pure function of grid, time, and view:
  - `Shapes` (2D) combines signed distance functions. Anti-aliasing comes from `coverage(distance, edge)`, where `edge` is about one pixel.
  - `Cube` (3D) is a per-pixel ray caster. It keeps the camera fixed and moves the ray into the cube's frame using the inverse of the cube's rotation. `cube_hit` intersects the ray with the cube's slabs, and the normal is rotated back for Lambert shading.
  - `Ocean` (in `ocean.rs`) ray marches a sum of sharp-crested waves, then bisects to the surface, and shades it with Fresnel sky reflection, sun glitter, foam, and haze.
  - `Knot` and `Model` draw a `Mesh`. `Drawing` draws an SVG.
  - The `wave` and `duck` presets are `Drawing` and `Model` scenes parsed from `assets/wave.svg` and `assets/duck.glb`, which `include_bytes!` compiles into the binary. `Drawing::parse` and `model::parse_gltf` read from bytes; `Drawing::load` and `model::load` read files.
- **`mesh.rs`** is a triangle rasterizer. It projects vertices, fills a depth buffer at twice the grid resolution in each direction, records the nearest triangle and perspective-correct weights per pixel, shades each visible pixel once (key, fill, rim, and specular light on the albedo from vertex colors and an optional texture), and downsamples. Surfaces are double-sided. `fill_missing_normals` smooths corners that lack normals except across creases sharper than about 40 degrees. `normalize` fits a mesh in a unit sphere.
- **`model.rs`** loads `.glb`/`.gltf` (through the `gltf` crate, walking the node hierarchy and reading base color factors, textures, and vertex colors), `.obj` with `.mtl` diffuse colors, and binary or ASCII `.stl` (rotated from Z-up to Y-up). It works in sRGB, so glTF's linear colors are converted.
- **`drawing.rs`** parses SVG with `usvg` (loading system fonts only when the file may contain text) and rasterizes it with `resvg` every frame at about 3x3 subpixels per pixel, through an affine transform from `Grid::to_pixels`. Coverage and premultiplied color are averaged per pixel.
- **`terminal.rs`** opens `/dev/tty` and runs `stty` with stdin set to that tty. It uses `stty` to save the terminal mode, enable raw non-blocking input (`min 0 time 0`), and read the size. `Key::parse` separates arrow-key sequences from a lone Esc. Output uses the alternate screen, hides the cursor, and wraps each frame in synchronized-output markers (`?2026`). `Terminal::clear` forgets the previous canvas so the next draw is complete; call it after resizing or changing the style or color mode. `Drop` restores the saved mode and the screen, so code must let `Terminal` drop normally instead of calling `process::exit`.
- **`main.rs`** parses options (`--style`, `--color`) and scene names or file paths, picks the one-frame path when not on a TTY, and otherwise runs a 30 fps loop. The loop reads keys (`1`-`9`/Tab switch scenes, `m` style, `c` color mode, arrows/WASD turn, `+`/`-` zoom, space pause, `r` reset; `q`, Esc, or Ctrl+C quit, because raw mode disables SIGINT) and checks the terminal size every 250 ms. It renders at `columns-1` x `rows-1` to leave room for line wrapping and the status line.

### Adding a scene

A new built-in scene touches several places:
- `Scene` variant, `NAMES` entry, `builtins` entry, `label`, and `render` arm in `scene.rs` (a preset made from an asset file only needs the `NAMES` and `builtins` entries and an `include_bytes!` constant)
- Help text in `main.rs`
- README
- For third-party assets, a license file next to the asset in `assets/`

The `every_builtin_scene_produces_visible_animation` test covers every entry in `builtins`. A new file format instead needs a loader in `model.rs` or `drawing.rs` and its extension in that module's `EXTENSIONS`.

## Conventions

- Tests are inline `#[cfg(test)] mod tests` blocks. Test names are descriptive sentences in snake_case, for example `a_parallel_ray_outside_the_cube_misses`.
- Identifiers are full words (`columns`, `rows`, `distance`), not abbreviations.
- Commit subjects are concise and imperative, for example `Add wave rendering module`. For changes that affect visible output, include sample terminal output in PRs. If tests could not be run, say so explicitly.
- When adding tooling or commands, document them in `README.md`.
