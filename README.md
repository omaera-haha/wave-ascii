# wave-ascii

A ghost lives in a glass box in your terminal.

The box is a small room drawn in dots: a floor lamp, a potted plant, a ball, a
cushion to sleep on, and specks of light that drift in. The ghost is a point in
that room with a body, needs, and a mood. You see it as an abstract shape made
of dots, and its glow shows how it is. It floats about, chases and eats light
motes, plays with the ball, waters the plant, switches the lamp, naps when
tired, and comes to the glass to look at you.

Two models give it a mind:

- **Jev**, TypeSafe's System One model, makes its quick choices. Each time
  the ghost finishes something, it asks Jev which of the things it could do
  next fits the moment, and how it feels. Jev answers with calibrated probabilities, and the ghost picks among
  them weighted by its own needs.
- **Claude** is its slower System Two. Every few minutes, and whenever you type
  to it, Claude thinks the moment over in the ghost's voice. It sets a goal,
  says something if there's something to say, and remembers what matters, such
  as your name.

Without API keys the ghost lives on instinct: its needs alone decide what it
does, and it can't talk.

## Run

Requires a stable Rust toolchain and a Unix terminal. Mouse input needs a
terminal that reports the mouse; most do.

```bash
cargo run --release
cargo run --release -- --style blocks
cargo run --release -- --offline        # never call a model
```

Press `k` in the app to enter API keys, or set `ANTHROPIC_API_KEY` and
`TYPESAFE_API_KEY` in the environment, which take precedence over saved keys.
Get a Claude key from the [Claude Console](https://console.anthropic.com) and
a Jev key from the [TypeSafe console](https://console.typesafe.ai).

| Key            | Action                                        |
| -------------- | --------------------------------------------- |
| `t`, Enter     | Talk to the ghost (Enter sends, Esc cancels)  |
| `k`            | Enter API keys                                |
| `j`            | Journal: what it remembers, what was said     |
| `?`            | Help                                          |
| Arrows, WASD   | Turn the view                                 |
| `+`, `-`       | Zoom                                          |
| `r`            | Reset the view                                |
| `m`            | Next style: ascii, blocks, braille            |
| `c`            | Next color mode: truecolor, 256 colors, plain |
| Space          | Pause                                         |
| `q`, Esc       | Quit                                          |

With the mouse, click the ghost to poke it (poke it a few times and it gets
grumpy), click the ball to flick it, and click anywhere else in the room to
drop in a light mote. Drag to turn the view and scroll to zoom. The ghost's
eyes follow the pointer. Hold Shift (Option in iTerm2) to select text while the
app has the mouse.

### What you see

- **The glow around the ghost** takes the color of its mood: teal when
  content, gold when cheerful, pink when excited, blue when curious, indigo
  when sleepy, pale blue when lonely, lilac when bored, and red when grumpy.
  It shines and beats faster with more energy.
- **Its face** shows the mood too. It makes its own expressions at irregular
  moments, reacts to what happens, and shuts its eyes in sleep.
- **The status bar** shows its name, mood, and needs (energy, fun, company),
  what it's doing, and the state of each mind: `●` ready, `◐` thinking,
  `✕` failed (the journal says why), and `○` off.
- **The line above the status bar** shows what it said, a thought, or its
  goal. What it says also appears in a bubble over its head.

The room follows your local time: daylight by day, warm at dawn and dusk, and
dark at night unless the lamp is on.

### Its life between runs

The ghost's name, needs, the plant, the lamp, its goal, its memories, and the
recent conversation are saved in `~/.local/share/wave-ascii/ghost.json`
(`$XDG_DATA_HOME` if set) every minute and on quit. While the app is closed,
time passes: the plant gets thirsty and the ghost lonely, and it notices how
long you were away. Delete the file to start a new ghost.

Settings live in `~/.config/wave-ascii/config.json` (`$XDG_CONFIG_HOME` if set),
readable only by you since it may hold keys. Besides the keys it accepts:

| Setting           | Default           | Meaning                                         |
| ----------------- | ----------------- | ----------------------------------------------- |
| `claude_model`    | `claude-opus-5-5` | The Claude model                                |
| `jev_model`       | `jev-latest`      | The Jev model or alias                          |
| `reflect_every`   | `180`             | Seconds between reflections while you're around |
| `claude_base_url` | none              | Another address for the Anthropic API           |
| `jev_base_url`    | none              | Another address for the TypeSafe API            |

### What is sent, and what it costs

Each request describes the moment in words, for example "tired", "the plant is
a little dry", "the viewer poked you (just now)". Requests to Claude also carry
the ghost's memories and the last few lines of conversation. Nothing else
leaves your machine.

Jev costs very little: each decision is a few hundred input tokens, billed per
million, and output is free. Claude reflects every three minutes while you're
around and every fifteen while you're away, and once for each message you
type. With Claude Opus 5.5, a reflection costs a cent or two.
Raise `reflect_every`, or set `claude_model`, to spend less.

Claude requests turn on Anthropic's server-side fallbacks
(`fallbacks: "default"`), so a request the model's safety classifiers decline
is retried on another model. A declined reply leaves the ghost as it was.

## Piped output

When stdout or stdin isn't a terminal, the app prints one frame of a fresh
ghost and exits. It never reads or writes the saved life or calls a model. The
frame size comes from `COLUMNS` and `LINES` (default 80x24), and output is
plain unless `--color` is given. `--time` runs the world that many seconds
first, deciding on instinct, and `--hour` sets the time of day:

```bash
COLUMNS=100 LINES=32 cargo run -q -- --time 20 < /dev/null | cat
COLUMNS=100 LINES=32 cargo run -q -- --style blocks --color truecolor --hour 21 < /dev/null | cat
```

The same `--time` and `--hour` give the same frame.

## Development

```bash
cargo check
cargo test
cargo fmt --check
cargo clippy --all-targets
```

The API clients are tested against local mock servers, so the tests need no
keys or network.

Dependencies: `crossterm` for the terminal, `glam` for vector math, `tokio`
and `reqwest` for the API calls, `serde` and `serde_json`, `jiff` for the local
time, and `unicode-width` for wide characters such as Hangul.

## License

MIT; see `LICENSE`.
