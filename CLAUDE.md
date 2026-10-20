# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

`wave-ascii` is a Rust (edition 2024) terminal app that renders animated scenes as text, in color or plain. Its dependencies are `glam` for vector, matrix, and quaternion math, `resvg` (with `usvg` and `tiny-skia`) for SVG, `svgtypes` for colors and paths in SVG animation, and `gltf` for glTF models. It has no terminal crate: terminal control uses raw ANSI escapes and shells out to `stty`, so it runs only on Unix.

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

The rendering pipeline is: `Scene::render(grid, time, view)` → `Frame` → `Style::canvas` → `Canvas` → `Terminal::draw` (or `Canvas::ansi` when piped). While the scene menu is open, `Menu::draw` renders the scene into the columns beside its panel and returns the combined `Canvas`.

- **`frame.rs`** defines `Grid`, `Shade`, and `Frame`. A `Grid` is the pixel lattice a scene is sampled on. Its y axis runs from -1 to 1 across the height, and `aspect` is a pixel's width divided by its height, so a distance covers the same physical length in both directions. `CELL_ASPECT = 0.5` because a terminal cell is about twice as tall as it is wide, and a test checks this. A `Shade` is a brightness plus a hue at full intensity; `hue * brightness` is the lit color, and glyph density shows brightness. `Frame::sample` evaluates a closure at every pixel center, `Frame::build` does the same by column and row, and both split rows across threads for large frames, so closures must be `Fn + Sync`. Values are clamped to `[0, 1]`. `Grid::refine` and `Frame::downsample` supersample for anti-aliasing.
- **`canvas.rs`** turns a `Frame` into terminal `Cell`s. `Style::grid` picks the sampling grid for a style: `Ascii` uses one pixel per cell and the ramp `" .:-=+*#%@"`, `Blocks` uses two pixels per cell drawn as `▀` with foreground and background colors, and `Braille` uses 2x4 dots with ordered dithering. `Canvas::redraw` writes only the cells that differ from the previous canvas, positioning the cursor absolutely, and `Pen` writes color escapes only when colors change.
- **`color.rs`** has `ColorMode` (plain, 256-color palette, truecolor), detection from `NO_COLOR`, `TERM`, and `COLORTERM`, and the SGR encoding.
- **`scene.rs`** has one `Scene` variant per scene and the `View` (yaw, pitch, zoom) set by the arrow and zoom keys. Each scene is a pure function of grid, time, and view:
  - `Shapes` (2D) combines signed distance functions. Anti-aliasing comes from `coverage(distance, edge)`, where `edge` is about one pixel.
  - `Cube` (3D) is a per-pixel ray caster. It keeps the camera fixed and moves the ray into the cube's frame using the inverse of the cube's rotation. `cube_hit` intersects the ray with the cube's slabs, and the normal is rotated back for Lambert shading.
  - `Ocean` (in `ocean.rs`) ray marches a sum of sharp-crested waves, then bisects to the surface, and shades it with Fresnel sky reflection, sun glitter, foam, and haze.
  - `Ghost` (in `ghost.rs`) is built from signed distances: a head circle, a flared skirt, and round hem lobes joined by a smooth union, with the face cut out. Face features change by moving their own points and sizes (an eye bends into `>`, the other tips into a `_` cursor), and the outline ring is the silhouette's distance offset, so it follows the hem. Every motion's period divides the 12-second `CYCLE`, so the loop is seamless.
  - `Knot` and `Model` draw a `Mesh`. `Drawing` draws an SVG, still or animated; animated ones are listed under Animations.
  - The `wave` and `duck` presets are `Drawing` and `Model` scenes parsed from `assets/wave.svg` and `assets/duck.glb`, which `include_bytes!` compiles into the binary. `Drawing::parse` and `model::parse_gltf` read from bytes; `Drawing::load` and `model::load` read files.
- **`mesh.rs`** is a triangle rasterizer. It projects vertices, fills a depth buffer at twice the grid resolution in each direction, records the nearest triangle and perspective-correct weights per pixel, shades each visible pixel once (key, fill, rim, and specular light on the albedo from vertex colors and an optional texture), and downsamples. Surfaces are double-sided. `fill_missing_normals` smooths corners that lack normals except across creases sharper than about 40 degrees. `normalize` fits a mesh in a unit sphere.
- **`model.rs`** loads `.glb`/`.gltf` (through the `gltf` crate, walking the node hierarchy and reading base color factors, textures, and vertex colors), `.obj` with `.mtl` diffuse colors, and binary or ASCII `.stl` (rotated from Z-up to Y-up). It works in sRGB, so glTF's linear colors are converted.
- **`drawing.rs`** parses SVG with `usvg` (loading system fonts only when the file may contain text) and rasterizes it with `resvg` every frame at about 3x3 subpixels per pixel, through an affine transform from `Grid::to_pixels`. Coverage and premultiplied color are averaged per pixel. A still drawing is framed by its visible bounds and gets a sway and a sheen; an animated one is reparsed from `Animation::at(time)` each frame, framed by its canvas, and masked to it.
- **`animation.rs`** plays SMIL animation (`<animate>`, `<set>`, `<animateTransform>`, `<animateMotion>`), which usvg ignores. `Animation::parse` reads the source with `roxmltree` (re-exported by usvg), finds each animated attribute's byte range (or where to insert it), and `Animation::at` writes the still SVG for a moment by splicing in the computed values. Values blend number by number when their text matches (path data, lists, lengths); colors blend as `rgba(…)`; anything else switches discretely. Event-based `begin` values never start.
- **`terminal.rs`** opens `/dev/tty` and runs `stty` with stdin set to that tty. It uses `stty` to save the terminal mode, enable raw non-blocking input (`min 0 time 0`), and read the size. `Key::parse` separates arrow-key sequences from a lone Esc. Output uses the alternate screen, hides the cursor, and wraps each frame in synchronized-output markers (`?2026`). `Terminal::clear` forgets the previous canvas so the next draw is complete; call it after resizing or changing the style or color mode. `Drop` restores the saved mode and the screen, so code must let `Terminal` drop normally instead of calling `process::exit`.
- **`app.rs`** holds what the keys control (scene index, `View`, pause, style, color mode, and the `Menu`). `App::press` gives each key to the open menu first and handles the rest: Enter opens the menu, `1`-`9`/Tab/Shift+Tab switch scenes, `m` style, `c` color mode, arrows/WASD turn, `+`/`-` zoom, space pause, `r` reset, and `q`, Esc, or Ctrl+C quit, because raw mode disables SIGINT. It returns a `Response` telling the loop to continue, redraw completely, or quit. `App::status` builds the status line.
- **`menu.rs`** is the scene menu: a list without a search box, grouped under headings from `Entry::group`. `Menu::handle` moves the highlight (arrows, `j`/`k`, Tab, Home/End, Page Up/Down, wrapping at the ends for single steps) and reports an `Action`; the app shows the highlighted scene as a live preview, Enter or a digit chooses, and Esc goes back to the scene the menu opened on. Keys the menu doesn't use come back as `Action::Ignored`. `Menu::draw` scrolls to keep the highlight (and its group heading) in view and paints the panel with box-drawing borders and colors, plus a `▸` marker so plain output shows the highlight.
- **`main.rs`** parses options (`--style`, `--color`, `--time`, `--fps`) and scene names or file paths, picks the one-frame path when not on a TTY (at `--time`, default 0), and otherwise runs a loop at `--fps` (default 60) that passes keys to `App::press` and checks the terminal size every 250 ms. Frames start on a fixed beat that restarts when drawing falls behind, and animation time follows the clock, so speed doesn't depend on the rate. It opens the menu at the start when no scene was named. It renders at `columns-1` x `rows-1` to leave room for line wrapping and the status line.

### Adding a scene

A built-in scene made in code touches several places:
- `Scene` variant, `GENERATED` name, `builtins` entry, `group`, `label`, and `render` arm in `scene.rs`, usually with its renderer in its own module (like `ocean.rs`, `lava.rs`, `aurora.rs`, `ghost.rs`)
- Help text in `main.rs` lists names from `Scene::groups`, so it needs no change
- README

A preset made from an asset file only needs a `preset!` line in `DRAWINGS`, `ANIMATIONS`, or `MODELS` in `scene.rs`; the macro reads `assets/<name>.svg` or `assets/<name>.glb` with `include_bytes!`. An SVG in `ANIMATIONS` must contain SMIL animation, since the menu groups a drawing by whether it animates. Also:
- README
- For third-party assets, a license file next to the asset in `assets/`
- For generated models, the generator script in `tools/models/` (see below)

`builtins` lists generated scenes, then drawings, then animations, then models, and the menu starts a new heading whenever the group changes. The `every_builtin_scene_produces_visible_animation` test covers every entry in `builtins`. A new file format instead needs a loader in `model.rs` or `drawing.rs` and its extension in that module's `EXTENSIONS`.

### Bundled art

The SVG drawings and animations in `assets/` are their own source; edit them directly (a browser plays the animations too). Animated art is framed by its `viewBox`, so make the canvas fit the motion tightly. Opacity and partial coverage darken a pixel's color instead of letting the terminal background show through, so faint translucent glows come out near black and can look like dark outlines; keep glows strong or leave them out. The models other than the duck are generated by stdlib-only Python scripts in `tools/models/`, one per model, built with `tools/models/meshkit.py` (lathe, tube, extrude, icosphere, and other primitives; transforms; vertex colors; flat or smooth normals; a GLB writer that converts sRGB colors to glTF's linear ones). Running `python3 tools/models/<name>.py` rewrites `assets/<name>.glb`. To check how art looks, render a frame through the app with `--time` and the block style, for example `COLUMNS=100 LINES=32 cargo run -q -- assets/rocket.glb --style blocks --color truecolor --time 2.6 < /dev/null | cat`. Drawings lift ink darker than value 0.45 and models floor albedo at 0.3, so art should use mid-to-bright colors and keep brightness contrast for the plain style.

## Conventions

- Tests are inline `#[cfg(test)] mod tests` blocks. Test names are descriptive sentences in snake_case, for example `a_parallel_ray_outside_the_cube_misses`.
- Identifiers are full words (`columns`, `rows`, `distance`), not abbreviations.
- Commit subjects are concise and imperative, for example `Add wave rendering module`. For changes that affect visible output, include sample terminal output in PRs. If tests could not be run, say so explicitly.
- When adding tooling or commands, document them in `README.md`.
