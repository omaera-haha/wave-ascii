# wave-ascii

A ghost lives in a glass box in your terminal.

The box is a small room drawn in dots: a floor lamp, a potted plant and the
little pots its seeds grow in, a ball, five bright toy blocks, a wind-up music box, a cushion to
sleep on, a pile of picture books, a little shelf of keepsakes, specks of
light that drift in, and a window onto hills where the seasons turn, birds come to perch, storms roll
through, snow falls in winter, rain can leave a rainbow, and shooting stars
fall on clear nights. Migrating geese honk past, hot-air balloons drift,
kites tug in gusts, squirrels scamper in the tree, and planes blink overhead.
A snowman keeps the snowy hill company as it thaws; paper lanterns rise on
Chuseok and Seollal nights. The ghost follows passing life with its gaze and
remembers each first sighting. On lit nights, most of all with the window open, a little moth flutters in and circles the lamp. The ghost is a point in that room with a body, needs, a
mood, and a gaze: it sees what lies within sixty degrees of where it faces,
and furniture hides what's behind it. You see it as an abstract shape made of
dots, and its glow shows how it is.

In suitable weather and hours, each moving kind passes about three times
an hour (kites about twice, during seeded gusts), for twenty to ninety
seconds. Geese migrate in spring and autumn; balloons favor clear summer
and autumn mornings and late afternoons; kites fly on spring and summer
days; squirrels visit in autumn; planes cross clear nights. Lanterns rise
only on Chuseok and Seollal nights. At most two appear together. A snowman
waits for deep snowfall to settle and stays as the remaining cover melts.

It floats about, chases and eats the light motes it has spotted, plays with
the ball and brings it back after a throw, stacks towers of blocks and knocks
them down, pounces on your pointer through the glass, waters the plant,
switches the lamp, watches the birds from the window, naps when tired, and
comes to the glass to look at you. When delighted it dances a little twirl in
the air. It plays hide-and-seek, draws little pictures and writes words in
the fog on the glass, and challenges you to noughts and crosses there. Draw
in the fog yourself and it comes to look, names what it recognizes, and
learns the pictures you teach it. Stroke it and it melts. At night it goes
to bed and sleeps until morning. On some moonlit evenings a smaller ghost,
its friend, drifts in through the window to play.
On clear nights it finds and names pictures in the stars, keeping their faint
outlines in its sky.

A wooden mirror on the right wall reflects the room and its ghosts. At first
it greets the stranger inside, then tests how the reflection copies it.
After enough completed visits, a young or grown ghost recognizes itself;
an accessory helps it make the connection sooner. Later it makes faces,
admires what it wears, and practices little twirls. Ask “who is in the
mirror?” or “거울 속에 누구야?” to hear what it thinks.

Little involuntary quirks follow the room: hiccups after too many motes or
much delight, sneezes around spring blossom or dusty corners, sleepy yawns,
cold shivers, and contented sighs after a long stroke or settling on the
cushion. Poke away the hiccups, or send “yawn”, “*yawns*”, “하품”, or “하암”
and it may yawn back. Shivers draw it toward warmth or a shut window. These
moments move its face and body and leave small diary memories, while sleep,
stories, and games stay quiet.

A small wooden music box sits by the right wall. The ghost winds its crank,
then listens with closed eyes, sways, or dances little loops to the beat.
Golden notes rise from its open lid and fade as the tune winds down. There
is no audio. It knows a lullaby, a waltz, a tiny march, and a sparkling little
melody; sometimes it winds the lullaby on its way to bed and falls asleep
before the music ends. Clicking the box winds it, and a sleeper sleeps on.

Teach it a tune by typing only three to twenty-four solfège syllables:
`do re mi fa sol la ti` (also `so` and `si`), `do-re-mi`, or `도레미` /
`도 레 미`. Add `~` or a trailing dash before a space or the end to hold a
note: `do- re mi~~`. Holds last up to four beats. It plays your tune back
and remembers the newest eight, named by their first notes, in its life
panel. Repeating a lesson keeps one copy. Later it may play your tune on its
own or dream of it. Ask “do you like music?” or “what songs do you know?”
(“음악 좋아해?” / “무슨 노래 알아?”) to hear about them.

During focus sessions the box stays quiet and lessons wait until work ends.
You can explicitly ask for `play a lullaby` or `자장가 들려줘` during work;
it plays without drawing the ghost away from its quiet company. Lessons
heard while sleeping wait until it wakes.

Each ghost is born a small wisp with a nature of its own, and grows up over
its first month, getting better at what it practices. Over days it grows
close to you, learns your words, your name, and the name you give it, learns
when you usually come and waits for you, befriends the bird, which brings it
little gifts, plants a garden from its plant's seeds, forms habits, keeps a
diary, and marks its special days, Seollal and Chuseok among them. Speak
Korean to it and it answers in Korean and writes Hangul in the fog.

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
its day while it sleeps, with glowing pictures in a little thought cloud above its head.

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
| `b`             | Open the album of moments the ghost kept                  |
| `e`             | Read letters; Left/Right browse older/newer                |
| `o`             | Focus together for twenty-five minutes; `o` again ends it |
| `g`             | Invite it to noughts and crosses on the glass              |
| `f`             | Toggle finger drawing in the fog; `f` or Esc stops         |
| Right drag      | Draw on the outside of the glass                          |
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
tower to knock it down, click the window to ask it to open or shut, click
the music box to wind it, and click anywhere else in the room to drop in a
light mote. Press on the ghost and drag back and forth across it to
stroke it. Drag anywhere else to turn the view, and scroll to zoom. The
ghost's eyes follow the pointer. Right-drag on the glass to draw in its fog,
or press `f` to draw with left drags on a trackpad. Left clicks still poke,
flick, or play board moves in this mode; `f` or Esc returns to ordinary drags.

The journal wraps all retained memories, its moments to remember, the diary,
the conversation, the latest ten events, and any model errors. It opens at
the newest lines; arrows on its borders and a line range show where you are.
Other keys leave it open. The help and life panels close on any key. `q`
quits from any of these panels. While one of them is open, the terminal handles
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

### Album of moments

The ghost keeps its own pictures when something memorable happens: every
milestone, the first celebration of each occasion day, the first rainbow or
snowfall it sees through the window that day, shooting star wishes,
constellations found, towers five blocks high, games it wins against you,
its friend's arrival, flowers opening in the main plant or little pots,
finished stories, and completed focus sessions.

It takes a picture after about two seconds, from the next watching frame,
including the scene, speech, symbols, and words line. Panels and pause defer
it for up to a minute; after that the moment passes. Ordinary pictures are
at least five minutes apart. Milestones always earn a picture, and replace
a simultaneous ordinary picture with the milestone's own caption.

Press `b` to open the album, newest first. Left/right flip toward newer/older
pages; Home/End jump to the newest/oldest. Esc or `b` closes it, and `q` quits.
Each page shows its picture, the ghost's caption and name, the local date,
and its place in the album. Smaller terminals crop pictures around the
center. Pictures follow the current color mode, and the mouse selects text
while the album is open. The life panel (`l`) shows how many moments it holds.

The newest sixty ordinary pages, and up to a hundred twenty pages of
milestones besides, live in `~/.local/share/wave-ascii/album/`
(`$XDG_DATA_HOME` if set), beside the saved life, as private JSON files with
local timestamp names and run-length encoded truecolor cells. Older ordinary
pages are deleted as new ones arrive, while a life's firsts stay, back to the
day it came into being; missing, damaged, and foreign files are skipped. Without a saved life, including piped previews, no pictures are kept.

### Letters from the ghost

After at least six hours away, a ghost past its first day leaves a letter
by the front glass. Click the little envelope or press `e` to read it.
Left/Right browse older/newer letters, ↑/↓ scroll a long one on a small
screen; Esc, Enter, or `e` closes the paper,
and `q` quits. The status bar shows `✉ e` while a letter waits unread.
The newest twenty letters stay with its saved life, and the journal lists
their dates and opening lines after the diary.

Letters remember its time alone and the life you share, in English or
Korean. Newborns old enough to write leave a tiny babbling note. They work
without keys; with Claude enabled, a separate request can add its own words
using the away story, skipped diary days, and memories. Opening the letter
keeps its words fixed. Letters stay separate from the conversation. Reading
one warms the bond and can leave a picture in the album. On your birthday,
once it knows it, a short birthday letter waits by the glass too.

### Focus together

Press `o` to work beside the ghost for twenty-five minutes, or tell it
“let's focus”, “focus for 40 minutes”, or “40분 집중”. Requests default to
twenty-five minutes and stay between five and one hundred twenty. It settles
low beside the glass to keep you quiet company, with fewer thoughts and no
unprompted model reflections or invitations to play. It still rests and
cares for its needs; a sleeping ghost stays asleep. You count as nearby even
while another window has focus. The status and window title show the time
left. When time is up it gently wakes, cheers, and suggests a break, then
favors play for five minutes. Press `o` again to end early without a fuss.
Its life panel and diary remember completed sessions and whole minutes
spent together, including early endings. Quitting silently ends the timer.

### Playing with it

- **Picture books.** It starts with three of twelve little books in English and
  Korean. Another arrives every four life days, with an extra birthday gift,
  until its library is full. In quiet moments it reads two to four pages on
  the cushion or under the lamp, turning pages every fifteen to twenty-five
  seconds and keeping its place between visits. Reading fits beside your
  focus session; sometimes it reads a page before bed. Ask "what are you
  reading?" or "무슨 책 읽어?", or ask for its favorite book. "Read to me",
  "read me a book", or "책 읽어줘" brings the next pages to the glass, one
  sentence at a time in your language. A poke or a no puts the book down;
  pages already read stay remembered. Finished books can become retold
  stories, and the life panel lists its library.
- **Stories.** Ask "tell me a story" or "이야기 들려줘" and it tells a little tale
  from its life, a sentence at a time; at bedtime it says goodnight and
  floats to its cushion to sleep. Poking it or saying no stops the tale.
- **Constellations.** After watching a clear, dark sky for twenty seconds, it
  may find a picture among the stars, using familiar shapes and pictures you
  taught it. It searches once a night, remembers its newest twelve finds, and
  lights their faint lines while gazing. Ask "do you know any constellations?"
  or "별자리 알아?" to hear their names; `l` lists its sky.
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
  and settle on their faces. Their positions and standing towers stay between
  visits; its diary remembers towers built, the tallest tower, and your demolitions.
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
- **Word games.** Say "word game", "let's play a word game", "단어 게임 하자",
  or "낱말 맞추기". It thinks of a word from its life and draws a blank for
  each letter beside a flower with six petals. Once it finishes drawing,
  press `t` or Enter to talk, type a letter, and send it with Enter; you can
  also try the whole word. Korean words take whole Hangul syllables. Correct
  guesses fill every matching blank; wrong guesses cost a petal and appear
  below the word. Repeats cost nothing. Solve it and the word glows; lose all
  the petals and it writes the answer for you. After two quiet minutes it
  gently ends the game. Sleeping, saying no, or leaving interrupts silently;
  pokes are fine. It may invite you when playful or bored, but stays quiet
  during focus sessions. Finished games and solved words count in its life;
  a running board is not saved. No model keys are needed.
- **Doodles.** It breathes on the glass and draws a heart, star, smile, spiral,
  sun, flower, wave, or snowflake, chosen from its mood, the weather, and how
  close it feels to you. The stroke takes a few seconds; then it backs away to
  admire it. Up to three pictures linger, fading after about two and a half
  minutes. It also writes "HI" for a friend arriving, "BOO" at Halloween,
  the coming year at New Year, "YAY" on its birthday or life landmarks, and your name once it
  grows close to you, sometimes followed by a heart. It writes Hangul names
  too, composing each syllable from consonant and vowel strokes; names mixing
  scripts or using other scripts get a heart instead. At first it writes as
  it reads from inside the box,
  so the letters look mirrored to you. After three words written while you
  watch, it learns to write your way round and keeps that moment as a milestone.
  Claude can also ask it to write a short word in the fog: up to eight
  characters, including at most five Hangul syllables.
- **Your drawings.** Right-drag, or press `f` and use left drags, to draw on
  the outside of the glass. Lift your finger between strokes. After a short
  pause an awake ghost comes to study the picture, unless it is busy resting,
  tending, carrying something, hiding, or playing a game. It recognizes its
  own eight sketches, plus circles, triangles, squares, crosses, ticks,
  lightning zigzags, houses, crescent moons, and straight lines, without a
  model. Hearts bring affection, stars sparkle, and sometimes it draws an
  answer beside yours. It answers in Korean when your latest message was
  Korean. The traces linger for two and a half minutes, then fade over forty
  seconds. A stroke far away or after a six-second pause starts a new picture.
- **Teaching pictures.** After it looks, name the picture in your next message
  within a minute: "it's a cat", "a cat", "고양이야", or "고양이 그림이야".
  You can correct a wrong guess this way too. Names use letters, at most two
  words and sixteen characters. It remembers the newest eight pictures
  between visits, recognizes them when you draw them again, and sometimes
  draws one itself once you are friends. Teaching the same name replaces
  its example; its first learned picture becomes a milestone. Its diary and
  life panel remember pictures seen and learned. Copies begin mirrored,
  following the same practice as its writing.
- **Asking.** With a Jev key, what you type can ask it to do things: play,
  fetch, build a tower, catch your finger, play noughts and crosses, dance,
  hide, draw, rest, look at your drawing, look out of the window, play with
  its friend while it visits, or come closer.
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
  Claude key, Claude answers instead. It also answers "tell me a story"
  offline, weaving its own memories into a short tale in either language.

### Getting to know you

Once you are friends, the ghost sometimes asks about your favorite color,
season, animal, food, or birthday while you chat at the glass. Answer in
English or Korean; it remembers between visits. It asks at most once a day,
or twice if it is curious, and waits three minutes for an answer. Questions
and requests carry on normally. Dates can be “March 3”, “3월 14일”, or an
unambiguous numeric date such as “3/14”; short animal and food answers can
be up to twenty-four characters.

Ask “what's my favorite color?” or “when is my birthday?” (“내가 좋아하는
색이 뭐야?” / “내 생일 언제야?”) to hear what it remembers. Your favorite
color warms its arrival glow and colors future scarves; your favorite
season gets a greeting on its first morning, and animals and foods wander
into its thoughts and dreams. The life panel's **You** row keeps these details.

Your birthday brings a cake, a greeting, birthday writing in the fog, a
short letter by the glass, and sometimes a party hat. February twenty-ninth is celebrated on the
twenty-eighth in other years. Its own birthday takes precedence if you share
the date.

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

The ghost lifts the lower window sash for fresh air on mild, dry days in
spring, summer, and autumn, and shuts it against rain, snow, blustery wind,
and bedtime. Click the window to ask it to open or shut, or say “open the
window” / “창문 열어” or “close the window” / “창문 닫아”. It refuses to open
in unsuitable weather and lets a sleeping ghost sleep. The sash starts shut
in a new room and stays as it was left between runs.

Wind comes in seasonal spells, gustier in storms and softer in snow and on
summer nights. The tree sways, clouds hurry, and blossom, leaves, and snow
slant in it. An open window billows the light curtains, stirs motes, petals,
plant leaves, and the ghost's hem, and lifts its spirits nearby. Spring
blossom sometimes blows inside; autumn leaves tumble in, settle, and fade.
If rain or snow blows through while it sleeps, a small puddle forms below
the window. It frets and shuts the sash when it wakes; the wet floor dries
over ten minutes. Moths come mostly through an open window (a shut one lets
one slip in now and then), and finish their visit even if it shuts. Its ghost friend can still pass through a shut pane.

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

The ghost naps when tired by day, a few times a day for a quarter hour or
so (newborns and sleepy natures more often); light motes are only snacks. Bedtime runs from ten at night to half past
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
berries, or pebbles, depending on the season. A gift is a small occasion: the
bird decides each morning whether today is a day for one, about two days in
five once it trusts the ghost fully, and never leaves two in a day. The ghost reaches through the
window pane, carries each gift inside, and puts it on a little wooden shelf
above the lamp, keeping the newest eight. The life panel (`l`) shows their
friendship and its keepsakes; gifts and friendship become moments and diary
entries too.

Clear spells last hours, and showers of five to twenty minutes come more
often in spring and less often in summer; about a third of the showers that
end by day leave a rainbow for a few minutes. Some showers become
thunderstorms, especially in summer: jagged lightning flashes across the
window and lights the room, followed a moment later by thunder. The ghost
flinches and may wake from a nap, though a storm wakes it only once. While it barely knows you, it favors the
cushion and shies away from the window; once it trusts you, it comes to the
glass to be near you when thunder rolls, unless it is resting, tending the
plant, or fetching the ball. Winter brings soft falling snow instead,
whitening the hills and dusting the branches. Snow builds in about two
minutes; in winter it lingers for hours, and once spring comes it melts over
about twenty minutes by day or an hour at night. The ghost lights up when it
spots a rainbow or falling snow.

**A friend.** On some clear evenings, between about half past five and a quarter
to ten, a smaller ghost drifts in through the window. A full moon and a lonely
ghost make a visit more likely: the friend decides once each evening whether
to come, about two evenings in five on average, and arrives at some point in
the evening rather than right at dusk. They play tag, twirl together, or float
side by side to watch the moon. Having a friend nearby helps with loneliness. After a
few minutes it waves goodbye and floats away; sleep, bedtime, rain, or snow
bring the visit to an early end. There is at most one visit each evening.

The same friend returns, with its own name and soft glow, and they grow closer
with each visit and game. It is shy near your glass at first and comes closer
as their friendship grows. Click the friend to make it giggle and bob away.
The life panel (`l`), diary, dreams, and thoughts remember it between runs,
though a visit in progress ends when you close the app. You can ask “Who is
your friend?” or “친구 이름은 뭐야?” even without model keys.

### The garden

The plant flowers in spells: buds open over about an hour, flowers last
about a day, petals drift down, and a ripe seed is left behind. After half a
day's rest it can bloom again. The ghost sometimes carries a seed to one of
three little pots below the window and plants it. Daisies, tulips,
sunflowers, bluebells, and poppies sprout, leaf out, bud, and flower with
care over about four to six days (roughly twenty-five waterings). Repeated
watering cannot rush a flower into bloom before four days. The ghost waters
the thirstiest pot first. Spring helps growth; winter is a rest from flowers.
Neglected plants only droop, and lift their heads again after watering;
nothing dies. Its garden and first seeds, sprouts, and flowers are in its
life panel (`l`), thoughts, and diary. Pots and their ages are saved, and
the garden ages while the app is closed, bounded to a week per absence.

### Little things to wear

The ghost sometimes chooses one small accessory from its life and the day:
a flower from a flowering pot in its own garden, a feather borrowed from its
keepsake shelf, or a warm knitted scarf for snow and cold winter evenings.
Birthdays and New Year invite paper party hats; Halloween brings a little
witch hat, Seollal a tiny lucky pouch (복주머니), and the winter holidays a
sprig of holly. Its nature helps it choose, and sometimes it wears nothing.

It visits the pots or shelf before borrowing something, waiting until it is
free. Accessories lean and trail with its body, and fade when it hides.
Flowers last until bedtime; a scarf stays on while it is cold. Its choice
survives a restart on the same day. The visiting ghost friend notices, and
its thoughts, diary, and minds remember what it wore.

Ask “what are you wearing?” / “뭐 입었어?” / “그거 뭐야?” or say
“take off your hat” / “모자 벗어”. These work offline, too.

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

The ghost celebrates its hundredth day, 백일, with layered white rice cake
(백설기), a tiny paper flag, a greeting, and a letter for you. Its thousandth
day, 천일, brings a second little cake. Birth counts as day one, so these fall
ninety-nine and nine hundred ninety-nine days after birth. It also remembers
milestones a week, a calendar month, and each year later, sharing one fond
memory when you visit. Its life panel shows its day of life in words.

Seollal brings a little wooden table with rice-cake soup and a red lucky pouch.
An awake ghost comes to the glass for a deep New Year's bow, once each day,
and wishes you luck. Drop a light mote within twenty seconds afterward and it
delights in its New Year's gift. It sometimes writes "복" in the fog.

At Chuseok, the same table holds half-moon songpyeon and two persimmons.
Clear evenings draw the ghost to the window, where it makes a wish on the
harvest moon once a night, and it sometimes writes "달". Seollal and Chuseok
each last three days, the day before through the day after the holiday;
their dates are included for 2026–2035 only. The first celebration of each
becomes a milestone.

Hangul Day, October 9, brings words such as "한글", "가나다", and "안녕" to the
fog, and a little remark about letters for a Korean-speaking viewer. The
ghost remembers how often you speak Korean and follows that language in its
greetings, games, and offline replies. A Korean message gets a Korean reply
straight away; its private thoughts stay in English.

### Its life between runs

The room stays as you left it, including the ball, blocks, and standing towers.
After more than half an hour away, the ghost has a little story of its own:
naps, weather watching, play, drawing, or familiar visitors. Skipped days go
into its diary; long absences keep a week of detail and a note about the quieter
days before. You may find a new tower, scattered blocks, a moved ball, or the
last fading strokes of a picture. Nights leave it rested, and on your return
it tells you a little about what happened, in English or Korean. You can also
ask “What did you do while I was away?” or “뭐 하고 있었어?” These remembered
stories do not add practice or lifetime counts.

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
you, quickly at first and then more slowly. Closeness takes days, not hours:
each day can hold only so much warmth, so a long day counts for less than the
same hours spread over several. With company through most of each day, it
gets to know you on the first day, grows fond of you within a few, feels close
after about a week and a half, and becomes devoted only after many weeks.
Talking to it, dropping it light motes, and flicking it the ball warm it a
little, within the same daily room; poking it again and again cools it. If you stay away for days, whether the app is closed or left
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

It holds one wish at a time: a rainbow or a shooting star, a bird's gift,
a friend's return, a garden flower, a taller tower (up to five blocks),
finishing an unread picture book, or something to do with you, such as
teaching it a tune or telling it about yourself. Its nature helps choose, and it leans a little toward activities that
could bring the wish about. At the glass it may tell you a wish in English
or Korean, with a short hint on the words line: press `g` for a game,
right-drag a picture, stroke it gently, ask for a story, or type notes such
as “do re mi”. It stays quiet
about wishes while you focus together. Ask “What do you wish for?” or
“소원이 뭐야?” to hear its current hope and the last one that came true.

Wishes appear from the morning or a shooting star after its first day;
newborns have simpler hopes. A wish coming true brings a sparkle, a little
celebration, and sometimes a new album moment. It remembers its newest
twelve granted wishes, avoids the last two kinds, and lets an unfulfilled
wish go after a week or when its season passes. It waits until the next day
for another, unless a shooting star prompts one sooner. Press `l` to see
its wish and how many came true. Wishes, their dates, and the daily spoken
reminder survive restarts; stories about time away never grant them.

It keeps its firsts and landmarks as moments to remember: its first light
mote, its first rainbow, the first time you found it hiding, its hundredth
mote, an hour together, the first word it learns from you, and each time it
grows closer to you. It lights up when it reaches one.

It also grows habits. The more often it chooses something, the more it leans
toward it, so over days each ghost becomes its own: one loves the ball,
another the window. Within a day it tires of repeating itself: a pastime it
has already turned to many times today, like the music box or doodling, draws
it less until tomorrow. At the end of each day it writes a line in its diary
about what stood out most, the most notable first, leaving everyday things
like motes and ball pushes out of a busy day ("Learned your name: Sam, saw a
rainbow, and watered the plant until it flowered. You were here for about an
hour, and we talked."). Press `l` to see its age,
nature, your bond, what it loves, what it has done in all its days, and today so far;
the diary and its moments are in the journal (`j`). Claude reads the last few
days of the diary and its latest moments.

The ghost's name, needs, the plant and its little pots, the lamp, its goal,
its memories, its nature, bond, habits, moments, diary, the words and pictures
it has learned, your name and whether you speak Korean, the hours you usually
come, its practiced skills, its friend, its friendship with the bird,
keepsakes, any gift waiting on the sill, and the recent conversation are saved
in `~/.local/share/wave-ascii/ghost.json` (`$XDG_DATA_HOME` if set) every
minute and on quit. Games, block towers, a friend's visit in progress, and
pictures on the glass are not. While the app is closed, time passes: the plant
gets thirsty and the ghost lonely, the garden ages, and it notices how long you
were away. Delete the file to start a new ghost.

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
`--born YYYY-MM-DD` sets a birth date so you can preview birthdays and 백일:

```bash
COLUMNS=100 LINES=32 cargo run -q -- --time 20 < /dev/null | cat
COLUMNS=100 LINES=32 cargo run -q -- --date 2026-09-26 --hour 20 --time 400 < /dev/null | cat
COLUMNS=100 LINES=32 cargo run -q -- --style blocks --color truecolor --hour 21 < /dev/null | cat
COLUMNS=100 LINES=32 cargo run -q -- --date 2026-12-24 --hour 22 --time 400 < /dev/null | cat
COLUMNS=100 LINES=32 cargo run -q -- --date 2026-10-31 --hour 22 < /dev/null | cat
COLUMNS=100 LINES=32 cargo run -q -- --date 2027-03-01 --born 2026-03-01 < /dev/null | cat
COLUMNS=100 LINES=32 cargo run -q -- --date 2026-06-10 --born 2026-03-03 < /dev/null | cat
COLUMNS=100 LINES=32 cargo run -q -- --date 2027-02-07 < /dev/null | cat
COLUMNS=100 LINES=32 cargo run -q -- --date 2026-09-25 --hour 21 < /dev/null | cat
```

The October evening example has a nearly full moon and a visiting ghost friend
within three simulated minutes.

The same `--time`, `--hour`, `--date`, and `--born` give the same frame.
Without `--born`, piped ghosts have no birthday or day-of-life celebrations. Piped time advances the
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

To check the ghost's rhythm after tuning it, an ignored test lets a ghost
live three days offline beside a simulated viewer and prints how it spent
them: activity shares by hour, how long plans last, every kind of event,
its most repeated thoughts and speech, its diary, and its milestones.

```bash
cargo test --release audit -- --ignored --nocapture
```

Dependencies: `crossterm` for the terminal, `glam` for vector math, `tokio`
and `reqwest` for the API calls, `serde` and `serde_json`, `jiff` for the local
time, and `unicode-width` for wide characters such as Hangul.

## License

MIT; see `LICENSE`.
