# wave-ascii

A ghost lives in a glass box in your terminal.

The box is a small room drawn in dots: a floor lamp, a potted plant, a ball,
five bright toy blocks, a cushion to sleep on, a little shelf of keepsakes,
specks of light that drift in, and a window onto hills where the seasons
turn, birds come to perch, storms roll through, snow falls in winter, and
rain can leave a rainbow. On lit nights, a little moth sometimes flutters in
and circles the lamp. The ghost is a point in that room with a body, needs, a
mood, and a gaze: it sees what lies within sixty degrees of where it faces,
and furniture hides what's behind it. You see it as an abstract shape made of
dots, and its glow shows how it is.

It floats about, chases and eats the light motes it has spotted, plays with
the ball and brings it back after a throw, stacks towers of blocks and knocks
them down, pounces on your pointer through the glass, waters the plant,
switches the lamp, watches the birds from the window, naps when tired, and
comes to the glass to look at you. When delighted it dances a little twirl in
the air. It plays hide-and-seek, draws little pictures and writes words in
the fog on the glass, and challenges you to noughts and crosses there. Stroke
it and it melts. At night it goes to bed and sleeps until morning. Each ghost
is born with a nature of its own, and over days it grows close to you, learns
your words and your name, learns when you usually come and waits for you,
befriends the bird, which brings it little gifts, forms habits, keeps a
diary, and marks its special days.

Two models give it a mind:

- **Jev**, TypeSafe's System One model, makes its quick choices. Each time
  the ghost finishes something, it asks Jev which of the things it could do
  next fits the moment, and how it feels. Jev answers with calibrated
  probabilities, and the ghost picks among them weighted by its own needs.
- **Claude** is its slower System Two. Every few minutes, and whenever you type
  to it, Claude thinks the moment over in the ghost's voice. It sets a goal,
  says something if there's something to say, and remembers what matters, such
  as your name.

Without API keys the ghost lives on instinct: its needs alone decide what it
does. It can't hold a conversation, but it reads your tone the way a pet does,
answers in a few words of its own, answers simple questions about itself, and
slowly learns what your words mean. It
still thinks to itself now and then ("The plant looks thirsty."), and dreams of
its day while it sleeps.

## Run

Requires a stable Rust toolchain and a Unix terminal. Mouse input needs a
terminal that reports the mouse; most do.

```bash
cargo run --release
cargo run --release -- --style blocks
cargo run --release -- --offline        # never call a model
cargo run --release -- --fps 30         # frames per second (default 60)
```

In terminals that report focus, it draws at most 24 frames a second while
another window has the focus, so a ghost kept beside your work stays light.
When you quit, it sees you off with a line left in your terminal, such as
"Pip waves from the glass. See you soon."

Press `k` in the app to open the settings and paste API keys, or set
`ANTHROPIC_API_KEY` and `TYPESAFE_API_KEY` in the environment, which take
precedence over saved keys. Get a Claude key from the
[Claude Console](https://console.anthropic.com) and a Jev key from the
[TypeSafe console](https://console.typesafe.ai). Keys entered in the app are
saved, so you enter them once.

The same panel changes the Claude model, Claude's effort, the prompt cache,
and the Jev model while the app runs. Use ↑↓ to choose a row and ←→ to step
through the choices, or type any model name. Enter saves the settings, and
the next requests use them at once.

| Key             | Action                                                     |
| --------------- | ---------------------------------------------------------- |
| `t`, Enter      | Talk to the ghost (Enter sends, Esc cancels)               |
| `k`             | Settings: keys, models, effort, cache                      |
| `j`             | Journal: memories, moments, diary, conversation            |
| `l`             | Its life: age, growth, skills, bond, today so far           |
| `p`             | Save a postcard of the moment                             |
| `g`             | Invite it to noughts and crosses on the glass              |
| `1`–`9`         | Choose a square, from top left to bottom right             |
| `?`             | Help                                                       |
| Arrows, WASD    | Turn the view; ↑/↓ scroll the journal one line             |
| PageUp/PageDown | Scroll the journal by a page                               |
| Home/End        | Top/bottom of the journal                                  |
| Esc, Enter, `j` | Close the journal                                          |
| `+`, `-`        | Zoom                                                       |
| `r`             | Reset the view                                             |
| `m`             | Next style: ascii, blocks, braille                         |
| `c`             | Next color mode: truecolor, 256 colors, plain              |
| Space           | Pause                                                      |
| `q`, Esc        | Quit (Esc closes panels or cancels typing)                 |

With the mouse, click the ghost to poke it (poke it a few times and it gets
grumpy), click the ball to flick it, click a loose block to nudge it or a
tower to knock it down, and click anywhere else in the room to drop in a
light mote. Press on the ghost and drag back and forth across it to
stroke it. Drag anywhere else to turn the view, and scroll to zoom. The
ghost's eyes follow the pointer.

The journal wraps all retained memories, its moments to remember, the diary,
the conversation, the latest ten events, and any model errors. It opens at
the newest lines; arrows on its borders and a line range show where you are.
Other keys leave it open. The help and life panels close on any key. `q`
quits from any of the three. While one of them is open, the terminal handles
the mouse so you can select and copy text directly. Many terminals send wheel
movements as arrow keys here, which scroll the journal. Elsewhere, hold Shift
(Option in iTerm2) to select text while the app has the mouse.

### Postcards

Press `p` while watching to save the scene and the words line, including its
speech bubble and little symbols, with a caption naming the ghost, the local
date and time, and what it is doing. The status bar and panels stay out of the
picture. Each postcard is a plain `.txt` file and a truecolor `.ans` file in
`~/.local/share/wave-ascii/postcards/` (`$XDG_DATA_HOME` if set). Names follow
the moment's local time, such as `2026-10-21-153012.txt`; another in the same
second gets `-2`, then `-3`. Use `cat` to view an `.ans` file in a truecolor
terminal. In plain block mode, the ANSI copy uses colored blocks from the
same rendered moment. A ghost without a saved life cannot save postcards;
piped output never saves them.

### Playing with it

- **Stroking.** Press on the ghost and drag gently back and forth across it.
  It slows down, closes its eyes in happy arcs, leans into your hand, and
  puffs little hearts. Strokes soothe even a grumpy ghost, keep it company,
  and warm its bond with you. A sleeping ghost stays asleep, and a hiding one
  is found.
- **Fetch.** A flick of the ball can start a game: an awake ghost with energy
  to spare goes after it, carries it back to the glass, lets it drop in front
  of you, and waits a few seconds for another throw. It lets go of the ball if
  you poke it or it starts something else.
- **Blocks.** It gathers five bright toy blocks one at a time, stacks a
  slightly wobbly tower, then backs away to admire it. Depending on its nature,
  it leaves the tower standing or swoops in to knock it down. Click a loose
  block to nudge it, or any block in the tower to topple it; a playful ghost or
  a close friend laughs, while a quieter ghost may sulk over its fallen work.
  A fast ball knocks the tower over too. Loose blocks fall, tumble, bounce,
  and settle on their faces. The blocks start scattered each session; its
  diary remembers towers built, the tallest tower, and your demolitions.
- **Catch your finger.** Move the mouse quickly over the room and a playful
  ghost chases your pointer along the inside of the glass and pounces on it.
  It tires of the game after a few catches. Hold still and it lingers at the
  last spot, or let it rest if it's sleepy or grumpy. No API keys needed.
- **Hide-and-seek.** While you're around, it may slip behind the furniture or
  into a back corner and turn faint. Look for the glimmer of its eyes and click
  it to find it. If you don't find it after a while, it jumps toward the glass
  with a "Boo!"
- **Noughts and crosses.** Press `g` to invite an awake ghost to a game in
  the fog on the glass, with you going first. It may also draw a board and
  open a game itself while you're around. Once the grid is drawn, play crosses
  by clicking a free square or pressing `1`–`9` in reading order (top left to
  bottom right); it floats over to draw its noughts. It thinks ahead, but can
  be beaten: curiosity sharpens its play, playfulness and tiredness invite
  mistakes, and a tender ghost with a close bond sometimes lets you win. It
  remembers your games, wins, losses, and draws. Leave a turn unanswered for
  forty-five seconds and it says "Maybe later." The board fades over forty
  seconds. Games in progress aren't kept when the app closes.
- **Doodles.** It breathes on the glass and draws a heart, star, smile, spiral,
  sun, flower, wave, or snowflake, chosen from its mood, the weather, and how
  close it feels to you. The stroke takes a few seconds; then it backs away to
  admire it. Up to three pictures linger, fading after about two and a half
  minutes. It also writes "HI" for a friend arriving, "BOO" at Halloween,
  the coming year at New Year, "YAY" on its birthday, and your name once it
  grows close to you, sometimes followed by a heart. Names in other scripts
  get a heart instead. At first it writes as it reads from inside the box,
  so the letters look mirrored to you. After three words written while you
  watch, it learns to write your way round and keeps that moment as a milestone.
  Claude can also ask it to write a short word in the fog.
- **Asking.** With a Jev key, what you type can ask it to do things: play,
  fetch, build a tower, catch your finger, play noughts and crosses, dance, hide, draw, rest,
  look out of
  the window, or come closer.
- **Teaching it words.** Even without keys, the ghost hears whether you greet
  it, praise it ("good", "yay", "잘했어"), say goodbye, or tell it no ("no",
  "stop", "그만"), which stops what it's doing. Other words it has to learn,
  the way a pet does: from what it was doing when you said them (say "twirl"
  while it dances), from what you do right after (say "catch" and flick the
  ball), and from your praise. Say a new word twice and it tries something to
  see if that's what you mean ("Like this?"); praise the right guess and it
  remembers. Once it knows a word, saying it is enough ("Twirl? Okay!"), as
  long as it can do that just now. With a Jev key, what Jev understands
  teaches it too, so it keeps those words if you go without. The life panel
  lists the words it knows, and Claude hears them. It learns your name without
  a model too: say "my name is Mina", "call me Sam", or "내 이름은 민수야".
  It remembers the name between visits, greets you by it sometimes, and shows
  it in the life panel. A later "call me …" changes it. You can name the
  ghost too: "your name is Pip", "I'll call you Pip", "너의 이름은 뽀삐야",
  or "뽀삐라고 부를게". It takes the name gladly, and the first name you give
  it becomes a moment to remember.
- **Asking it things.** Without keys it still answers simple questions about
  itself, in English or Korean: how it feels ("how are you?", "기분 어때?"),
  what it's doing ("뭐해?"), its name, your name once you've told it, its
  age, whether it likes you, what it loves most, what's on its shelf, how it
  gets on with the bird, the weather outside, and whether it's tired, hungry,
  bored, or lonely. It answers from its mood, needs, habits, and bond, in
  Korean when you ask in Korean, and learns no words from a question. With a
  Claude key, Claude answers instead.

### What you see

- **The glow around the ghost** takes the color of its mood: teal when
  content, gold when cheerful, pink when excited, blue when curious, indigo
  when sleepy, pale blue when lonely, lilac when bored, and red when grumpy.
  It shines and beats faster with more energy.
- **Its face** shows the mood too. It makes its own expressions at irregular
  moments, reacts to what happens, and shuts its eyes in sleep.
- **Its eyes** follow where it faces, so its face shows what it's looking at,
  except while your pointer moves, which it watches instead.
- **Little symbols** rise above its head, even without API keys: `z`/`Z`
  for sleep, `♪` for humming, `♥` for affection, `♡` for loneliness, `!` for
  surprise, `?` for curiosity, `#` for grumpiness, `…` for boredom, and `✦`
  for delight. They drift away after a couple of seconds and move beside
  speech bubbles so the words stay readable.
- **The status bar** shows its name, mood, and needs (energy, fun, company),
  what it's doing, and the state of each mind: `●` ready, `◐` thinking,
  `✕` failed (the journal says why), and `○` off.
- **The line above the status bar** shows what it said, a thought, or its
  goal. What it says also appears in a bubble over its head.
- **The window title** shows the ghost's name and what it is doing. Your
  previous title comes back on quit in terminals that support title saving.

### The world outside

The room follows your local time and date: daylight by day, warm at dawn and
dusk, and dark at night unless the lamp is on. A wooden window in the back
wall looks over rolling hills and a tree that changes with the northern
hemisphere seasons: spring blossom, summer green, autumn gold and drifting
leaves, and bare winter branches. The sun crosses the sky, clouds drift,
stars twinkle, and the moon waxes and wanes with the calendar. A few fireflies
blink over the hills on summer nights. Late on clear nights, a shooting star
now and then streaks across the window, and many more fall on the best nights
of the year's meteor showers: the Quadrantids in early January, the Lyrids in
April, the Eta Aquariids in May, the Perseids in August, the Orionids in
October, the Leonids in November, and the Geminids in December. On those nights
the ghost likes to watch the sky, and if it sees one fall, it makes a wish.

The ghost naps when tired by day. Bedtime runs from ten at night to half past
six in the morning: it turns the lamp off if you're away and sleeps through
until morning. Keep it company and it stays up a while longer, until you
wish it goodnight ("goodnight", "잘자"), which sends it to bed. Opening the
app at bedtime finds it asleep on its cushion; poking or talking wakes it,
and leaves it drowsy for a couple of minutes. In the morning it stretches,
looks out of the window, and comes to the glass to greet you.

By day, a little bird sometimes lands on a branch or the outer sill, hops,
pecks, and sings. Winter brings a robin, even in snow; rain sends it away.
The ghost likes to watch quietly. Coming too close or rushing at the window
startles the bird, and the ghost wishes it hadn't scared its visitor. Quiet
watching slowly earns its trust across days; startling it costs some. As they
become friends the bird visits more often, sings more, and sits closer. In time
it leaves small gifts on the outer sill: feathers, petals, acorns, leaves,
berries, or pebbles, depending on the season. The ghost reaches through the
window pane, carries each gift inside, and puts it on a little wooden shelf
above the lamp, keeping the newest eight. The life panel (`l`) shows their
friendship and its keepsakes; gifts and friendship become moments and diary
entries too.

Showers come more often in spring and less often in summer; rain that ends by
day leaves a rainbow for a couple of minutes. Some showers become
thunderstorms, especially in summer: jagged lightning flashes across the
window and lights the room, followed a moment later by thunder. The ghost
flinches and may wake from a nap. While it barely knows you, it favors the
cushion and shies away from the window; once it trusts you, it comes to the
glass to be near you when thunder rolls, unless it is resting, tending the
plant, or fetching the ball. Winter brings soft falling snow instead,
whitening the hills and dusting the branches. Snow builds in about two
minutes and melts over about twenty minutes by day or an hour at night. The
ghost lights up when it spots a rainbow or falling snow.

### Special days

From October 24–31, a grinning jack-o'-lantern warms the room with candlelight.
The ghost loves hide-and-seek that week, and shouts "BOO!" on Halloween itself.
A little fir tree with twinkling lights stands for the winter holidays from
December 18–31. At eight on December 31, the New Year celebration takes over
through New Year's Day; fireworks burst outside from half past eleven until one
in the morning, briefly lighting the room. The ghost marks midnight, too. On
Valentine's Day, the motes glow pink and every doodle is a heart.

Starting with its first anniversary, the ghost's birthday brings a cake with
one candle per year, up to five, and a cheerful urge to dance. Its birth date
comes from its saved life in your local time zone; a February 29 birthday falls
on February 28 in other years. Birthdays take precedence over other occasions.
Special days find their way into its thoughts, dreams, and diary, even offline.

### Its life between runs

**Growing up.** A newborn is a small wisp, a young ghost after three days,
and fully grown after its first month. Its body grows smoothly with age,
even while the app is closed. Newborns tire faster, nap more, and sometimes
babble. Practice stays with it: stacking blocks, catching motes, fetching,
noughts and crosses, writing, and dancing each get better as it finishes
them. Towers grow taller and steadier, catches and fetches grow quicker,
game mistakes become rarer, shaky letters settle, and twirls grow into
larger loops. It remembers becoming young, growing up, and getting good at
each skill. Press `l` to see its stage and what it is getting good at, from
clumsy through learning to masterful. Older saved ghosts keep their practice
from their lifetime counts; ordinary seeded pictures remain full size unless
`--born` supplies a birth date.

Each ghost is born with its own nature: playful, curious, shy, sleepy, tender,
or some quiet mix of these. It keeps that nature for life. A playful ghost
gets bored sooner and likes the ball; a sleepy one naps more and longer.
A shy one takes longer to come and greet you, but holds on to your bond.
Its nature also colors its thoughts and words. Press `l` to see it.

The ghost grows close to you over time. Each minute you spend with it (it
knows you're there while you use the keys or the mouse, and in terminals that
report focus, twenty seconds in another window count as away) deepens its bond with
you, quickly at first and then more slowly: about a third of the way after an
hour together, most of the way after a dozen. Talking to it, dropping it light
motes, and flicking it the ball warm it a little; poking it again and again
cools it. If you stay away for days, whether the app is closed or left
running, the bond fades a little each day, but never below half the closest
it has been. Once it knows you, it leaves
whatever it was idly doing to come to the glass when you arrive, and it seeks
you out more often. Both minds hear how close it feels to you, and Claude lets
that color its words: shy with a stranger, warm with a friend.

After seeing you arrive on several different days, it learns the hours you
usually come. Around then, if you're away and it is awake, it drifts to the
glass and waits, shifting from side to side and glancing at the window.
Come at your usual time and it lights up: "There you are!" If it waited and
you didn't come, it feels a little lonely and notes that in its diary.
Older days gradually fade from its routine, so it learns when your hours
change. Press `l` to see when it usually expects you. This needs no model.

It keeps its firsts and landmarks as moments to remember: its first light
mote, its first rainbow, the first time you found it hiding, its hundredth
mote, an hour together, the first word it learns from you, and each time it
grows closer to you. It lights up when it reaches one.

It also grows habits. The more often it chooses something, the more it leans
toward it, so over days each ghost becomes its own: one loves the ball,
another the window. At the end of each day it writes a line in its diary
("Ate 4 light motes, watered the plant until it flowered, and saw a rainbow.
You were here for about an hour, and we talked."). Press `l` to see its age,
nature, your bond, what it loves, what it has done in all its days, and today so far;
the diary and its moments are in the journal (`j`). Claude reads the last few
days of the diary and its latest moments.

The ghost's name, needs, the plant, the lamp, its goal, its memories, its
nature, bond, habits, moments, diary, the words it has learned, your name, the
hours you usually come, its practiced skills, its friendship with the bird, keepsakes, any gift
waiting on the sill, and the recent conversation are saved in
`~/.local/share/wave-ascii/ghost.json` (`$XDG_DATA_HOME` if set) every minute
and on quit. Games, block towers, and pictures on the glass are not. While the
app is closed,
time passes: the plant gets thirsty and the ghost lonely, and it notices how
long you were away. Delete the file to start a new ghost.

Settings live in `~/.config/wave-ascii/config.json` (`$XDG_CONFIG_HOME` if set),
readable only by you since it may hold keys. Not next to the binary, so moving
or rebuilding the app keeps them. Besides the keys it holds:

| Setting           | Default           | Meaning                                              |
| ----------------- | ----------------- | ---------------------------------------------------- |
| `claude_model`    | `claude-opus-5-5` | The Claude model                                     |
| `claude_effort`   | `low`             | `low` to `max`, or `null` for the model's default    |
| `cache`           | `1h`              | Prompt cache lifetime: `1h`, `5m`, or `null` for off |
| `jev_model`       | `jev-latest`      | The Jev model or alias                               |
| `reflect_every`   | `180`             | Seconds between reflections while you're around      |
| `claude_base_url` | none              | Another address for the Anthropic API                |
| `jev_base_url`    | none              | Another address for the TypeSafe API                 |

All but the last three can be changed in the app with `k`.

### What is sent, and what it costs

Each request describes the moment in words, for example "tired", "the plant is
a little dry", "the viewer poked you (just now)", and what the ghost sees from
where it faces, such as "the window, showing a rainbow: across the room,
straight ahead", and how close it feels to you ("fond of them"). Requests to
Claude also carry the ghost's memories, its latest moments to remember, the
last three days of its diary, and the last few lines of conversation. Nothing
else leaves your machine. Claude can answer by turning the ghost's gaze to
something, so it can choose to look.

Jev costs very little: each decision is a few hundred input tokens, billed per
million, and output is free. Claude reflects every three minutes while you're
around and every fifteen while you're away, and once for each message you
type. With Claude Opus 5.5 a reflection costs a cent or two; with Claude Haiku
5.5 it costs a few hundredths of a cent. Raise `reflect_every`, or pick a
cheaper model, to spend less.

Each Claude request puts what changes least first: the ghost's persona,
memories, moments, and diary, then the conversation, and last the moment
itself. The first two are marked for Anthropic's prompt cache, so later
requests read them at a tenth of the input price. Writing an hour-long cache
entry costs twice the input price (five minutes: 1.25 times), so it pays off
when requests come more than once an hour, as they do while you're around.
Prompts shorter than the model's minimum (512 tokens on current models) aren't
cached, and nothing breaks.

Requests to models that offer them turn on Anthropic's server-side fallbacks
(`fallbacks: "default"`), so a request the model's safety classifiers decline
is retried on another model; Haiku models don't offer them. A declined reply
leaves the ghost as it was. If the API rejects a request because a model
doesn't take an option, it is sent once more without the effort, fallbacks,
and cache markers, so any model name works.

## Piped output

When stdout or stdin isn't a terminal, the app prints one frame of a fresh
ghost and exits. It never reads or writes the saved life or calls a model. The
frame size comes from `COLUMNS` and `LINES` (default 80x24), and output is
plain unless `--color` is given. `--time` runs the world that many seconds
first, deciding on instinct. `--hour` sets the time of day (default noon),
`--date YYYY-MM-DD` sets the date (default 2026-06-21), and optional
`--born YYYY-MM-DD` sets a birth date so you can preview birthdays:

```bash
COLUMNS=100 LINES=32 cargo run -q -- --time 20 < /dev/null | cat
COLUMNS=100 LINES=32 cargo run -q -- --style blocks --color truecolor --hour 21 < /dev/null | cat
COLUMNS=100 LINES=32 cargo run -q -- --date 2026-12-24 --hour 22 --time 400 < /dev/null | cat
COLUMNS=100 LINES=32 cargo run -q -- --date 2026-10-31 --hour 22 < /dev/null | cat
COLUMNS=100 LINES=32 cargo run -q -- --date 2027-03-01 --born 2026-03-01 < /dev/null | cat
```

The same `--time`, `--hour`, `--date`, and `--born` give the same frame.
Without `--born`, piped ghosts have no birthday. Piped time advances the
simulation while the chosen date and hour stay fixed.

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
