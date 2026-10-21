# wave-ascii

A Rust terminal app that renders animated scenes as text, in truecolor, 256
colors, or plain ASCII.

It has 34 built-in scenes in four groups:

- **Generated** in code: a rotating square with a moving, pulsing circle
  (`2d`); a shaded 3D cube (`3d`); a ray-marched sunset `ocean`; a rainbow
  torus `knot`; three `lava` lamps on a shelf; the northern lights over a
  lake (`aurora`); and a floating `ghost`, after the one on Ghostty's home
  page, inside a ring of flowing light, which watches the mouse pointer and
  makes faces of its own (see [Ghost](#ghost)).
- **Drawings** from SVG: the `wave` logo, a `koi` pond, Mount `fuji`, a
  `synthwave` sunset, a monarch `butterfly`, a low-poly `fox`, a scarlet
  `macaw`, a `mandala`, and pixel-art `invaders`.
- **Animations** from animated SVG: planets passing behind and in front of
  the sun (`orbits`); desert `dunes` through a day and night; a `bauhaus`
  poster whose tiles turn and slide on one beat; flowing silk `ribbons`; a
  pendulum wave (`pendulums`); comets chasing around an `infinity` loop; and a
  `breathe` guide that opens on the in-breath.
- **Models** from glTF: a textured `duck`, a `lighthouse`, a retro `rocket`, a
  ringed `planet`, a hot-air `balloon`, a flying saucer (`ufo`), a `bonsai`,
  fly-agaric `mushrooms`, gem `crystals`, a glazed `donut`, and a `chess`
  set.

It also plays vector art and 3D models made in other tools: SVG drawings and
SVG animations from Figma, Illustrator, Inkscape, or a text editor, and glTF,
OBJ, or STL models from Blender or any CAD or sculpting app.

## Run

Requires a stable Rust toolchain and a Unix terminal with `stty` available.

```bash
cargo run --release
cargo run --release -- ocean
cargo run --release -- wave --style braille
cargo run --release -- duck --style blocks
cargo run --release -- path/to/model.glb path/to/logo.svg
```

Without a scene name, the app starts with the scene menu open. Otherwise the
first scene or file named is shown first. Files join the end of the scene list.

The app draws 60 frames per second. On a faster display, set `--fps` to its
refresh rate for the smoothest motion, for example `--fps 120` or `--fps 144`.
A terminal program can't see the display's rate itself, since it talks to the
terminal through a pty, often over SSH or from a container. Animation speed
follows the clock, so it is the same at any rate; scenes that take longer to
draw than a frame allows run as fast as they can.

| Key                | Action                                        |
| ------------------ | --------------------------------------------- |
| Enter              | Open the scene menu                           |
| Tab, Shift+Tab     | Next or previous scene                        |
| `1`-`9`            | One of the first nine scenes                  |
| `m`                | Next style: ascii, blocks, braille            |
| `c`                | Next color mode: truecolor, 256 colors, plain |
| Arrows, WASD       | Turn the model, camera, or drawing            |
| `+`, `-`           | Zoom                                          |
| Space              | Pause                                         |
| `r`                | Reset the view                                |
| `q`, Esc, Ctrl+C   | Quit                                          |

The mouse works too. The wheel acts like Up and Down, and a click on a scene
in the menu shows it. Terminals hand mouse input to the app while it runs, so
select text by holding Shift (Option in iTerm2) as you drag.

### Scene menu

The scene menu lists every scene in a panel at the left, grouped under
Generated, Drawings, Animations, Models, and Files, with no search box to
type into. The
highlighted scene plays beside the panel, so you can look through the scenes
as you move.

| Key in the menu       | Action                                           |
| --------------------- | ------------------------------------------------ |
| Up, Down (`k`, `j`)   | Highlight the previous or next scene             |
| Tab, Shift+Tab        | Same as Down and Up                              |
| Home, End, PgUp, PgDn | Jump to the start, end, or by a page             |
| Enter                 | Show the highlighted scene and close the menu    |
| `1`-`9`               | Show that scene and close the menu               |
| Esc                   | Close the menu and go back to the previous scene |
| `q`, Ctrl+C           | Quit                                             |

Left and Right still turn the preview, and `m`, `c`, and Space work as usual.

The view adapts to terminal resizing and assumes characters are approximately
twice as tall as they are wide.

### Ghost

The ghost's face follows what you do, picking expressions at random so it
never repeats on a loop:

- **Left alone**, it glances around, smiles, sparkles with wonder, eyes
  something with suspicion or doubt, worries, winks, nods off for a moment,
  or turns its eyes into a `>_` prompt with a blinking cursor.
- **While the pointer moves** over the scene, its eyes follow it, now and then
  sparkling, narrowing, or smiling. It looks away once the pointer rests.
- **With the pointer on it**, it only watches the pointer and blinks.
- **After 40 seconds without input**, it falls asleep, and any key or mouse
  movement wakes it with a start.

It also reacts to events: it winks when its scene is shown, and a click on it
is a poke that surprises it, then makes it giggle, then annoys it.

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
| `.svg`, `.svgz`       | Figma, Illustrator, Inkscape, Affinity  | Rendered with resvg: gradients, clipping, masks, filters, text in system fonts, and SMIL animation |
| `.glb`, `.gltf`       | Blender, Maya, 3ds Max, Sketchfab       | glTF 2.0 node hierarchy, base color textures and factors, and vertex colors              |
| `.obj` with `.mtl`    | Nearly every 3D tool                    | Polygons, normals, texture coordinates, vertex colors, and material diffuse colors         |
| `.stl`                | CAD and 3D-printing tools               | Binary or ASCII; turned from Z-up to Y-up                                                 |

Drawings are framed by their visible bounds, sway gently, and catch a passing
highlight. Very dark ink is lifted so black artwork stays visible on dark
terminals. Models are centered, scaled to fit, and shown on a turntable.
Missing normals are smoothed except across sharp edges.

SVG files with animation play as they would in a browser, framed and clipped
by their canvas instead of swaying. The app reads SMIL animation:

- `<animate>`, `<set>`, and `<animateTransform>` on any attribute, including
  colors, path data, gradient stops, and `stroke-dashoffset`, with `values`
  or `from`, `to`, and `by`
- `<animateMotion>` along a `path`, an `<mpath>`, or `values`, with
  `rotate="auto"` and `keyPoints`
- `begin` offsets (negative ones start partway through), `dur`, `end`,
  `repeatCount`, `repeatDur`, and `fill="freeze"`
- `calcMode` `linear`, `discrete`, and `spline` with `keyTimes` and
  `keySplines`; `paced` moves evenly along a motion path and is linear
  otherwise
- `additive="sum"`, and targets named with `href`

Animations that begin on an event, such as a click or another animation
ending, never start. CSS animations are not supported, and an animated
attribute doesn't override the same property in a `style` attribute.

glTF meshes compressed with Draco or meshopt are not supported. If a texture
can't be decoded, for example because it isn't PNG or JPEG, the model is drawn
without textures.

## Bundled art

The drawings, animations, and models are compiled into the program from
`assets/`, so the built app needs no asset files. The duck is the Khronos glTF sample model,
© 2006 Sony, under the SCEA Shared Source License (see
`assets/duck.LICENSE.txt`). More sample models are in the
[Khronos glTF Sample Assets](https://github.com/KhronosGroup/glTF-Sample-Assets);
check each model's license. The other art was made for this project.

The drawings and animations are plain SVG files, written to be edited by hand.
A web browser plays the animations too, which helps when timing them.

The other models are generated by Python scripts in `tools/models/`, one per
model, using the small mesh kit in `tools/models/meshkit.py`. They need only
the Python 3 standard library. To change a model, edit its script and run it
from anywhere to rewrite the `.glb`:

```bash
python3 tools/models/rocket.py
```

To see how art looks before adding it, render one frame of the file in blocks
with color, using `--time` to turn a model to other sides or to pick a moment
of an animation:

```bash
COLUMNS=100 LINES=32 cargo run -q -- assets/rocket.glb --style blocks --color truecolor --time 2.6 < /dev/null | cat
```

## Piped output

When stdin or stdout isn't a terminal, the app prints one frame and exits. The
frame is at time 0 unless `--time SECONDS` picks another moment, which also
turns a model to a different side. Its size defaults to 80 columns by 24 rows,
or uses `COLUMNS` and `LINES` if set. Output is plain unless a color mode is
given:

```bash
COLUMNS=100 LINES=30 cargo run -q -- ocean --style blocks --color truecolor < /dev/null | cat
COLUMNS=100 LINES=30 cargo run -q -- duck --time 2.5 < /dev/null | cat
```

## Development

```bash
cargo check
cargo test
cargo fmt --check
cargo clippy
```

Dependencies: `glam` for vector math, `resvg` for SVG, `svgtypes` for colors
and paths in SVG animation, and `gltf` for glTF.

## License

MIT; see `LICENSE`. That covers the bundled art except `assets/duck.glb`,
which is © 2006 Sony under the SCEA Shared Source License, in
`assets/duck.LICENSE.txt`.
