# wave-ascii

a cute little life in your terminal.

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
| `--peek` | print one line about what's probably happening, for a prompt or status bar |
| `--album-html PATH` | export the saved album as one HTML page (`--force` replaces an existing file) |
| `-h`, `--help` | show help |

Piped, it prints one frame of a fresh start and exits, sized by `COLUMNS` and `LINES`. `--time SECONDS`, `--hour HOUR` (default 12), `--date YYYY-MM-DD` (default 2026-06-21), and `--born YYYY-MM-DD` pick the moment.

It runs on instinct alone. Press `k` in the app to add API keys for Jev (TypeSafe) and Claude, or set `ANTHROPIC_API_KEY` and `TYPESAFE_API_KEY` (these aren't saved). The life is saved under `~/.local/share/wave-ascii/`, and settings, keys included, in `~/.config/wave-ascii/config.json`.

## Keys

| Key | |
|---|---|
| `t`, Enter | talk |
| arrows, WASD | turn the view |
| `+` / `-` | zoom |
| `r` | reset the view |
| `m`, `c` | style, color mode |
| space | pause |
| `k` | API keys and models |
| `j` | journal |
| `l` | life and growth |
| `b` | album of moments |
| `p` | save a postcard |
| `P` | ask for a pose, then save a postcard |
| `g`, then `1`–`9` | noughts and crosses |
| `o` | focus together for twenty-five minutes (`o` again ends it) |
| `f` | draw in the fog with left drags (`f` or Esc stops) |
| `x` | hide a treasure: click the floor (`x` or Esc cancels) |
| `e` | letters, or write one when there are none (left/right browse) |
| `w` in letters | write; Ctrl+S sends, Esc keeps the draft; `v` browses sent letters |
| `?` | help |
| `q`, Esc | quit |

## Mouse

- Click things to see what they do; click empty floor or lawn to drop a light mote.
- Drag across someone to stroke them; drag elsewhere to turn the view, and scroll to zoom.
- Right-drag to draw in the fog on the glass.
- In the journal, help, life, letter, and album panels, the mouse selects text.
