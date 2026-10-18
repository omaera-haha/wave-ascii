# wave-ascii

A Rust terminal app that renders animated scenes as text, in truecolor, 256
colors, or plain ASCII.

It has six built-in scenes: a rotating square with a moving, pulsing circle; a
shaded 3D cube; a ray-marched sunset ocean; a rainbow torus knot; a wave logo
drawn from SVG; and a textured duck model. It also plays vector art and 3D
models made in other tools: SVG drawings from Figma, Illustrator, or Inkscape,
and glTF, OBJ, or STL models from Blender or any CAD or sculpting app.

## Run

Requires a stable Rust toolchain and a Unix terminal with `stty` available.

```bash
cargo run --release -- 2d
cargo run --release -- 3d
cargo run --release -- ocean
cargo run --release -- knot
cargo run --release -- wave --style braille
cargo run --release -- duck --style blocks
cargo run --release -- path/to/model.glb path/to/logo.svg
```

The first scene or file named is shown first. Files join the built-in scenes,
so `1` to `6` still switch to those.

| Key                | Action                                        |
| ------------------ | --------------------------------------------- |
| `1`-`9`, Tab       | Change scene                                  |
| `m`                | Next style: ascii, blocks, braille            |
| `c`                | Next color mode: truecolor, 256 colors, plain |
| Arrows, WASD       | Turn the model, camera, or drawing            |
| `+`, `-`           | Zoom                                          |
| Space              | Pause                                         |
| `r`                | Reset the view                                |
| `q`, Esc, Ctrl+C   | Quit                                          |

The view adapts to terminal resizing and assumes characters are approximately
twice as tall as they are wide.

## Styles

`--style` (or `m` while running) picks how pixels become characters:

- `ascii` draws one pixel per cell with the ramp ` .:-=+*#%@`.
- `blocks` draws two square pixels per cell with `▀`, using the foreground and
  background colors. Without color, it dithers to `▀▄█`.
- `braille` draws eight dithered dots per cell with braille patterns, for the
  finest detail.

Block and braille styles need a font with those characters. Terminals that
draw East Asian ambiguous-width characters as wide should be set to narrow.

## Color

`--color auto` (the default) uses plain text when `NO_COLOR` is set or
`TERM=dumb`, truecolor when `COLORTERM` is `truecolor` or `24bit` or the
terminal is known to support it, and 256 colors otherwise. Use
`--color truecolor`, `--color 256`, or `--color never` to choose, or press `c`.

Frames are drawn inside synchronized-output markers, and only cells that
changed since the previous frame are rewritten.

## Bring your own art

| Format                | Made with                               | Notes                                                                                     |
| --------------------- | --------------------------------------- | ----------------------------------------------------------------------------------------- |
| `.svg`, `.svgz`       | Figma, Illustrator, Inkscape, Affinity  | Rendered with resvg: gradients, clipping, masks, filters, and text in system fonts       |
| `.glb`, `.gltf`       | Blender, Maya, 3ds Max, Sketchfab       | glTF 2.0 node hierarchy, base color textures and factors, and vertex colors              |
| `.obj` with `.mtl`    | Nearly every 3D tool                    | Polygons, normals, texture coordinates, vertex colors, and material diffuse colors         |
| `.stl`                | CAD and 3D-printing tools               | Binary or ASCII; turned from Z-up to Y-up                                                 |

Drawings are framed by their visible bounds, sway gently, and catch a passing
highlight. Very dark ink is lifted so black artwork stays visible on dark
terminals. Models are centered, scaled to fit, and shown on a turntable.
Missing normals are smoothed except across sharp edges.

glTF meshes compressed with Draco or meshopt are not supported. If a texture
can't be decoded, for example because it isn't PNG or JPEG, the model is drawn
without textures.

The `wave` and `duck` scenes come from `assets/wave.svg` and `assets/duck.glb`,
which are compiled into the program, so the built app needs no asset files. The
duck is the Khronos glTF sample model, © 2006 Sony, under the SCEA Shared Source
License (see `assets/duck.LICENSE.txt`). More sample models are in the
[Khronos glTF Sample Assets](https://github.com/KhronosGroup/glTF-Sample-Assets);
check each model's license.

## Piped output

When stdin or stdout isn't a terminal, the app prints one frame at time 0 and
exits. Its size defaults to 80 columns by 24 rows, or uses `COLUMNS` and
`LINES` if set. Output is plain unless a color mode is given:

```bash
COLUMNS=100 LINES=30 cargo run -q -- ocean --style blocks --color truecolor < /dev/null | cat
```

## Development

```bash
cargo check
cargo test
cargo fmt --check
cargo clippy
```

Dependencies: `glam` for vector math, `resvg` for SVG, and `gltf` for glTF.

## License

MIT; see `LICENSE`. The bundled `assets/duck.glb` is not covered by it: it's
© 2006 Sony under the SCEA Shared Source License, in `assets/duck.LICENSE.txt`.
