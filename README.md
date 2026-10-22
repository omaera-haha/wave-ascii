# wave-ascii

a cute little life in your terminal.

A ghost lives in a glass box drawn as text. Its days follow your local clock, so it sleeps at night, and its life is saved between runs. With API keys, Jev (TypeSafe) makes its quick choices and Claude gives it thoughts and words; without them it lives on instinct.

## Run

```
cargo run --release -- [OPTIONS]
```

| Option | |
|---|---|
| `--style ascii\|blocks\|braille` | how pixels become characters (default braille) |
| `--color auto\|truecolor\|256\|never` | color mode; auto follows `NO_COLOR` and `COLORTERM` |
| `--fps N` | frames per second (default 60) |
| `--offline` | don't call any model, even with keys |
| `--peek` | print one line about what the saved ghost is probably doing, for a prompt or status bar |
| `--album-html PATH` | export the saved album as one HTML page (`--force` replaces an existing file) |
| `-h`, `--help` | show help |

Piped, it prints one frame of a fresh ghost and exits, sized by `COLUMNS` and `LINES`. `--time SECONDS`, `--hour HOUR` (default 12), `--date YYYY-MM-DD` (default 2026-06-21), and `--born YYYY-MM-DD` pick the moment.

Press `k` in the app to add API keys, or set `ANTHROPIC_API_KEY` and `TYPESAFE_API_KEY` (these aren't saved). The life is saved in `~/.local/share/wave-ascii/ghost.json`, and settings, keys included, in `~/.config/wave-ascii/config.json`.

## Keys

| Key | |
|---|---|
| `t`, Enter | talk to the ghost |
| arrows, WASD | turn the view |
| `+` / `-` | zoom |
| `r` | reset the view |
| `m`, `c` | style, color mode |
| space | pause |
| `k` | API keys and models |
| `j` | journal |
| `l` | its life and growth |
| `b` | album of moments |
| `p` | save a postcard |
| `P` | ask it to pose, then save a postcard |
| `g`, then `1`–`9` | noughts and crosses |
| `o` | focus together for twenty-five minutes (`o` again ends it) |
| `f` | draw in the fog with left drags (`f` or Esc stops) |
| `x` | hide a treasure: click the floor (`x` or Esc cancels) |
| `e` | letters, or write one when there are none (left/right browse) |
| `w` in letters | write; Ctrl+S sends, Esc keeps the draft; `v` browses sent letters |
| `?` | help |
| `q`, Esc | quit |

## Mouse

- Click the ghost to poke it (or find it when it hides), the ball to flick it, a block to nudge it or a tower to topple it, the window to open or shut it, the music box to wind it, a soap bubble to pop it, and an envelope to read it. Click anywhere else to drop a light mote.
- Drag across the ghost to stroke it; drag elsewhere to turn the view, and scroll to zoom.
- Right-drag to draw in the fog on the glass.
- In the journal, help, life, letter, and album panels, the mouse selects text.
