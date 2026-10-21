use crate::world::away::Happening;
use crate::world::{
    Activity, Heard, Mood, Occasion, Question, Skill, Sky, Stage, Tone, Understanding, World,
};

// Thoughts the ghost has on its own while it has no System Two to think
// with: a few words that fit the moment, so a ghost without keys still has
// an inner life. Each function gives every line that fits; the caller picks
// one.

/// Spoken once on returning, or again when the viewer asks about the absence.
pub fn returned(world: &World, korean: bool) -> Vec<String> {
    let Some(happening) = world.away.as_ref().and_then(|away| away.notable()) else {
        return Vec::new();
    };
    let lines = match happening {
        Happening::SleptNight => [
            ("I slept through the night.", "밤새 푹 잤어."),
            (
                "My cushion kept me cozy all night.",
                "밤새 방석에서 포근하게 잤어.",
            ),
        ],
        Happening::Napped => [
            (
                "I had a little nap while you were gone.",
                "너 없는 동안 낮잠을 잤어.",
            ),
            (
                "I curled up on my cushion for a nap.",
                "방석에 웅크리고 낮잠을 잤어.",
            ),
        ],
        Happening::WatchedRain => [
            (
                "I watched the rain while you were gone.",
                "너 없는 동안 비를 구경했어.",
            ),
            ("The rain kept me company.", "비가 내 곁에 있어 줬어."),
        ],
        Happening::WatchedStorm => [
            (
                "There was a storm. I stayed by my cushion.",
                "폭풍이 왔어. 방석 곁에 있었어.",
            ),
            (
                "I watched a storm from my cozy cushion.",
                "포근한 방석에서 폭풍을 구경했어.",
            ),
        ],
        Happening::WatchedSnow => [
            ("It snowed while you were gone!", "너 없는 동안 눈이 왔어!"),
            (
                "I watched snowflakes at the window.",
                "창문에서 눈송이를 구경했어.",
            ),
        ],
        Happening::SawRainbow => [
            (
                "I saw a rainbow while you were gone!",
                "너 없는 동안 무지개를 봤어!",
            ),
            (
                "The rain left a rainbow for me.",
                "비가 그치고 무지개가 떴어.",
            ),
        ],
        Happening::WatchedBird => [
            (
                "The bird kept me company at the window.",
                "창가에서 새가 곁에 있어 줬어.",
            ),
            (
                "I watched the bird while you were gone.",
                "너 없는 동안 새를 구경했어.",
            ),
        ],
        Happening::BuiltTower { standing: true } => [
            (
                "I built a tower while you were gone!",
                "너 없는 동안 탑을 쌓았어!",
            ),
            (
                "I left my tower standing for you.",
                "보여 주려고 탑을 남겨 뒀어.",
            ),
        ],
        Happening::BuiltTower { standing: false } => [
            (
                "I built a tower and knocked it down!",
                "탑을 쌓고 와르르 무너뜨렸어!",
            ),
            (
                "My tower went whoosh while you were gone!",
                "너 없는 동안 내 탑이 와르르 무너졌어!",
            ),
        ],
        Happening::PlayedBall => [
            (
                "I played with the ball while you were gone.",
                "너 없는 동안 공을 가지고 놀았어.",
            ),
            (
                "I rolled the ball all around the room.",
                "방 안에서 공을 굴리며 놀았어.",
            ),
        ],
        Happening::Drew(_) => [
            (
                "I drew a little picture while you were gone.",
                "너 없는 동안 작은 그림을 그렸어.",
            ),
            (
                "I left a picture in the fog for you.",
                "보여 주려고 김 서린 유리에 그림을 남겼어.",
            ),
        ],
        Happening::WateredPlant => [
            (
                "I watered the plant while you were gone.",
                "너 없는 동안 화분에 물을 줬어.",
            ),
            (
                "I kept the plant company and gave it water.",
                "화분 곁에 있어 주고 물도 줬어.",
            ),
        ],
        Happening::WishedOnStar => [
            ("I wished on a shooting star!", "별똥별에 소원을 빌었어!"),
            (
                "A shooting star crossed my window. I made a wish.",
                "창밖으로 별똥별이 지나갔어. 소원을 빌었지.",
            ),
        ],
        Happening::FriendVisited => [
            (
                "My ghost friend came by while you were gone!",
                "너 없는 동안 유령 친구가 놀러 왔어!",
            ),
            (
                "I spent an evening with my ghost friend.",
                "유령 친구와 저녁을 보냈어.",
            ),
        ],
        Happening::Waited => [
            (
                "I waited for you at the glass.",
                "유리 앞에서 너를 기다렸어.",
            ),
            (
                "I came to the glass when you usually visit.",
                "네가 늘 오는 시간에 유리 앞으로 왔어.",
            ),
        ],
        Happening::MissedViewer => [
            (
                "I missed you while you were gone.",
                "너 없는 동안 보고 싶었어.",
            ),
            (
                "The room felt quiet without you.",
                "네가 없으니 방이 조용했어.",
            ),
        ],
    };
    lines
        .into_iter()
        .map(|(english, hangul)| if korean { hangul } else { english }.to_owned())
        .collect()
}

/// What crosses its mind as it sets out to do `activity`.
pub fn setting_out(world: &World, activity: Activity) -> Vec<&'static str> {
    let present = world.viewer_present();
    let mut lines = match activity {
        Activity::Friend => vec![
            "Tag! You’re it!",
            "Let’s play together.",
            "Two ghosts, one moon.",
        ],
        Activity::Float => Vec::new(),
        Activity::Company => vec!["I'll stay right here."],
        Activity::Look => vec!["What did you draw?", "Let me have a look."],
        Activity::Explore => vec![
            "I wonder what's in that corner.",
            "Let's have a look around.",
            "Did something move over there?",
        ],
        Activity::Chase => vec![
            "A light mote! Mine.",
            "That one looks tasty.",
            "Come here, little light.",
        ],
        Activity::Rest if world.bedtime() => {
            vec!["Time for bed.", "Goodnight, little room."]
        }
        Activity::Rest => vec![
            "Time for a little nap.",
            "My cushion is calling.",
            "Just resting my eyes…",
        ],
        Activity::Play => vec![
            "Ball time!",
            "Where shall I push it this time?",
            "Watch this.",
        ],
        Activity::Tend if world.garden_droop() > 0.0 => vec![
            "Oh no, the plant is drooping.",
            "A drink will help you stand tall again.",
        ],
        Activity::Tend if world.seed_ready() => vec![
            "A seed! I could give it a little pot.",
            "A little garden of my own.",
        ],
        Activity::Tend if !world.seedlings.is_empty() => vec![
            "A little water for the little pots.",
            "Grow, little seeds. I'm here.",
        ],
        Activity::Tend if world.plant.thirst > 0.7 => {
            vec!["The plant looks thirsty.", "Hold on, plant, I'm coming!"]
        }
        Activity::Tend => vec!["A little water for you, plant.", "Grow, little plant."],
        Activity::Lamp if world.lamp.on && world.is_night() => vec!["Lights out."],
        Activity::Lamp if world.lamp.on => vec!["We don't need the lamp by day."],
        Activity::Lamp => vec!["It's getting dark in here.", "Let there be light."],
        Activity::Visit if present && world.life.bond >= 0.3 => {
            vec!["There you are!", "I'll come say hi.", "Hello, you."]
        }
        Activity::Visit if present => vec!["Someone's at the glass.", "Who's there?"],
        Activity::Visit => vec!["Is anyone out there?", "Maybe someone will come by."],
        Activity::Wait => vec!["They usually come around now.", "I'll wait by the glass."],
        Activity::Build => vec![
            "One block at a time.",
            "I could make a tower!",
            "Watch me build.",
        ],
        Activity::Fetch => vec!["I'll get it!", "Wait for me, ball!"],
        Activity::Pounce => vec![
            "What's that? I'll get it!",
            "Your finger! I'm coming.",
            "Gotcha… almost.",
        ],
        Activity::Hide => vec!["Ready or not…", "Bet you can't find me."],
        Activity::Doodle if world.life.bond >= 0.3 => {
            vec!["I'll draw you something.", "This one's for you."]
        }
        Activity::Doodle => vec!["Let me draw something.", "Hm, what shall I draw?"],
        Activity::Puzzle => vec!["A word and a little flower. I wonder what you will guess."],
        Activity::Game => vec!["Shall we play?", "I'll draw the board. You be crosses."],
        Activity::Dance => vec!["I feel like dancing.", "Wheee!"],
        Activity::Window if world.life.sill.is_some() => {
            vec!["Is that for me?", "I'll keep it somewhere safe."]
        }
        Activity::Window if world.morning() => morning(world),
        Activity::Window
            if world
                .visitors
                .bird
                .as_ref()
                .is_some_and(|bird| bird.perched(world.time)) =>
        {
            vec!["Hello, little bird.", "Don't fly away…"]
        }
        Activity::Window => match world.weather.sky {
            Sky::Rainbow => vec!["Look, a rainbow!", "All the colors at once."],
            Sky::Rain if world.weather.storm => vec!["Whoa, look at that!"],
            Sky::Rain => vec!["Listen to the rain.", "Pitter-patter."],
            Sky::Snow => vec!["It's snowing!", "Every flake is different, they say."],
            Sky::Clear if world.weather.snow_cover > 0.5 => {
                vec!["The hills are all white.", "I wish I could touch the snow."]
            }
            Sky::Clear if world.is_night() => {
                vec!["So many stars tonight.", "Where does the moon go by day?"]
            }
            Sky::Clear => vec!["I like watching the clouds.", "The hills look soft today."],
        },
    };
    if let Some(skill) = Skill::for_activity(activity)
        && world.life.skills.practice(skill) > 0.0
    {
        if world.life.skills.level(skill) >= 0.4 {
            lines.push("I'm getting better at this!");
        } else {
            lines.push("A little more practice.");
        }
        if skill == Skill::Stacking || skill == Skill::Writing {
            lines.push("Steady… steady…");
        }
    }
    babble(world, &mut lines, false);
    lines
}

// The caller samples these lines with the world's random source. About
// one in three choices is a babble, with the usual words still alongside.
fn babble<Line: From<&'static str>>(world: &World, lines: &mut Vec<Line>, korean: bool) {
    if world.stage() == Stage::Newborn {
        let sounds = if korean {
            ["응!", "오!", "음?"]
        } else {
            ["Ba!", "Ooh!", "Mm?"]
        };
        for index in 0..lines.len().div_ceil(2) {
            lines.push(sounds[index % sounds.len()].into());
        }
    }
}

/// Gives remembered friends and star pictures their names in the thought pools.
pub fn named_friend_line(world: &World, line: &str) -> String {
    let line = match &world.life.friend {
        Some(friend) => line
            .replace("My ghost friend", &friend.name)
            .replace("my ghost friend", &friend.name),
        None => line.to_owned(),
    };
    match world.life.constellations.last() {
        Some(constellation) => line.replace("my constellation", &constellation.name),
        None => line,
    }
}

/// What it muses about while nothing in particular happens.
pub fn idle(world: &World) -> Vec<&'static str> {
    if world.focus.is_some() {
        return vec![
            "Shh… they're working.",
            "I'll keep the light low.",
            "I'm right here.",
        ];
    }
    let ghost = &world.ghost;
    let needs = ghost.needs;
    let present = world.viewer_present();
    let mut lines = Vec::new();
    if ghost.plan.activity == Activity::Window
        && world.starry_night()
        && !world.life.constellations.is_empty()
    {
        lines.push("I can see my constellation again.");
    }
    if world.friend.is_some() {
        lines.extend([
            "My ghost friend is fun!",
            "Two little ghosts in one little room.",
        ]);
    } else if world
        .events
        .iter()
        .rev()
        .take_while(|event| world.time - event.time < 180.0)
        .any(|event| matches!(event.kind, crate::world::EventKind::FriendLeft(_)))
    {
        lines.push("I hope my ghost friend comes back soon.");
    }
    if world.drawing.is_some() {
        lines.extend([
            "Your side of the glass has pictures too.",
            "What will you draw next?",
        ]);
    }
    if !world.life.keepsakes.is_empty() {
        lines.extend(["Every little gift has a place.", "I like my little shelf."]);
    }
    if world.life.bird_trust >= 0.5 {
        lines.extend([
            "I hope my bird comes back soon.",
            "It isn't afraid of me anymore.",
        ]);
    }
    if ghost.plan.activity == Activity::Wait && !ghost.plan.done && !present {
        lines.extend(["Any minute now.", "I wonder if they'll come today."]);
    }
    if let Some(nature) = world.life.nature {
        if nature.shy >= 0.7 && present && world.life.bond < 0.3 {
            lines.push("Is it okay if I watch you?");
        }
        if nature.curious >= 0.7 {
            lines.push("What's behind the window, I wonder?");
        }
        if nature.playful >= 0.7 {
            lines.push("Someone throw the ball!");
        }
        if nature.sleepy >= 0.7 {
            lines.push("Yawn… perhaps a little lie down.");
        }
        if nature.tender >= 0.7 {
            lines.push("I hope the plant is happy.");
        }
    }
    if world.morning() {
        lines.extend(morning(world));
    }
    if world.petting() >= 0.35 {
        lines.extend(["Mmm… right there.", "That's nice…", "Don't stop."]);
    }
    if world.bedtime() && present {
        lines.extend(["I should be asleep…", "Just a little longer."]);
    }
    if world
        .visitors
        .moth
        .as_ref()
        .is_some_and(|moth| moth.noticed)
    {
        lines.extend(["A moth! It loves the lamp.", "Fly carefully, little moth."]);
    }
    match world.occasion() {
        Some(Occasion::Halloween) => lines.extend([
            "Boo! Did I scare you?",
            "The pumpkin is grinning at me.",
            "A ghost is very good at Halloween.",
        ]),
        Some(Occasion::WinterHolidays) => lines.extend([
            "The little tree is so pretty.",
            "A tiny holiday in a tiny room.",
        ]),
        Some(Occasion::NewYear) => {
            lines.extend(["Happy new year!", "I wonder what this year will bring."])
        }
        Some(Occasion::Valentine) => lines.extend([
            "A heart on the glass, just for you.",
            "Even the light motes look fond today.",
        ]),
        Some(Occasion::Birthday) => lines.extend([
            "It's my birthday!",
            "Another year in my glass box.",
            "I made a wish. It's a secret.",
        ]),
        Some(Occasion::Seollal) => lines.extend([
            "A little bow for a whole new year.",
            "The lucky pouch is full of possibilities.",
        ]),
        Some(Occasion::Chuseok) => lines.extend([
            "The harvest moon looks close enough to touch.",
            "Shall we share a little moonlight?",
        ]),
        Some(Occasion::HangulDay) => lines.extend([
            "Letters can be little pictures too.",
            "A word in the fog, just for you.",
        ]),
        None => {}
    }
    if needs.energy < 0.3 {
        lines.extend(["I'm getting sleepy.", "Running low on light."]);
    }
    if needs.fun < 0.3 {
        lines.extend(["Nothing ever happens in here.", "So bored…"]);
    }
    if needs.company < 0.3 && !present {
        lines.extend([
            "It's quiet without anyone around.",
            "I wonder when they'll be back.",
        ]);
    }
    if present && world.life.bond >= 0.55 {
        lines.extend(["I'm glad you're here.", "You always come back."]);
    } else if present && world.life.bond < 0.12 {
        lines.extend(["Who is that, out there?", "They're looking at me."]);
    }
    if world.viewer.pointer_is_lively() && needs.energy > 0.25 {
        lines.push("Your finger keeps getting away!");
    }
    if world.tower.is_some() {
        lines.extend(["My tower is still standing!", "Careful… it wobbles."]);
    }
    if world.plant.thirst > 0.7 {
        lines.push("The plant looks thirsty.");
    }
    if world.garden_droop() > 0.0 {
        lines.push("Oh no, the plant is drooping.");
    }
    if world.seed_ready() {
        lines.push("That seed is ready for a little pot.");
    }
    if world
        .seedlings
        .iter()
        .any(|plant| plant.growth < crate::world::Seedling::SPROUT)
    {
        lines.push("Is there a sprout yet? I'll wait.");
    }
    if world
        .seedlings
        .iter()
        .any(|plant| plant.flowering(world.season()))
    {
        lines.push("I grew that flower from a tiny seed.");
    }
    if !world.petals.is_empty() {
        lines.push("The petals are floating down so softly.");
    }
    if world.plant.blooming() {
        lines.push("The flowers smell nice.");
    }
    match world.weather.sky {
        Sky::Rain if world.weather.storm => {
            lines.extend(["I don't like thunder.", "Is it over yet?"]);
            if present && world.life.bond >= 0.3 {
                lines.push("I'm glad you're here when it thunders.");
            }
        }
        Sky::Rain => lines.extend(["The rain sounds cozy.", "Drip, drop."]),
        Sky::Rainbow => lines.push("That rainbow is still there!"),
        Sky::Snow => lines.extend(["Snow, snow, snow.", "It looks cold out there."]),
        Sky::Clear => {}
    }
    if world.is_night() && !world.lamp.on {
        lines.push("It's so dark.");
    }
    if matches!(ghost.mood, Mood::Cheerful | Mood::Excited) {
        lines.extend(["What a nice day.", "La la la."]);
    }
    if let Some(favorite) = world.life.favorite() {
        lines.push(match favorite {
            Activity::Play => "I could play with that ball all day.",
            Activity::Chase => "I hope another mote drifts in soon.",
            Activity::Tend => "My plant is the best plant.",
            Activity::Window => "I wonder what's past those hills.",
            Activity::Visit => "I like it at the glass.",
            Activity::Wait => "I know when to look for you.",
            Activity::Rest => "Naps are the best part of the day.",
            Activity::Explore => "I know every corner of this room.",
            Activity::Lamp => "I like the lamp's warm light.",
            Activity::Friend => "Playing together is the best.",
            Activity::Float => "Floating is nice.",
            Activity::Build => "How high could I stack those blocks?",
            Activity::Fetch => "Throw the ball again! Please?",
            Activity::Pounce => "Move your finger again!",
            Activity::Dance => "I've been working on a new twirl.",
            Activity::Hide => "Nobody hides like a ghost.",
            Activity::Game => "Another game on the glass?",
            Activity::Puzzle => "I could think of another word for you.",
            Activity::Doodle => "I should draw on the glass again.",
            Activity::Look => "I like seeing what you draw.",
            Activity::Company => "I like working quietly beside you.",
        });
    }
    if lines.is_empty() {
        lines.extend([
            "Hm.",
            "A ghost in a glass box. That's me.",
            "I wonder what the viewer is doing.",
            "The glass is cool.",
        ]);
    }
    babble(world, &mut lines, false);
    lines
}

fn morning(world: &World) -> Vec<&'static str> {
    let mut lines = vec!["Good morning!", "The sun's up.", "What a night."];
    if let Some(thought) = &world.ghost.thought
        && thought.text.starts_with('…')
    {
        if thought.text.contains("ball") {
            lines.push("That ball rolled all the way to the hills.");
        } else if thought.text.contains("mote") {
            lines.push("There were so many light motes.");
        } else if thought.text.contains("plant") {
            lines.push("The plant was as tall as the room.");
        } else if thought.text.contains("rainbow") {
            lines.push("A rainbow makes a very good slide.");
        } else if thought.text.contains("glass") {
            lines.push("For a moment, there was no glass.");
        }
    }
    lines
}

/// What it dreams of while it sleeps, mostly of what happened today.
pub fn dreams(world: &World) -> Vec<&'static str> {
    let today = &world.life.today;
    let mut lines = Vec::new();
    if today.focus_minutes > 0 || today.focus_sessions > 0 || world.focus.is_some() {
        lines.push("…we work side by side, and the little room holds its breath…");
    }
    if !world.life.constellations.is_empty() {
        lines.push("…my constellation winks at me…");
    }
    for skill in Skill::ALL {
        if world.life.skills.practice(skill) > 0.0 && world.life.skills.level(skill) < 0.65 {
            lines.push(match skill {
                Skill::Stacking => "…my wobbly tower stands as tall as the stars…",
                Skill::Catching => "…every little light turns back into my arms…",
                Skill::Fetching => "…bringing the moon back like a ball…",
                Skill::Games => "…the noughts show me where to draw next…",
                Skill::Writing => "…my shaky letters learn to fly straight…",
                Skill::Dancing => "…one twirl becomes a sky full of loops…",
            });
        }
    }
    if !world.life.pictures.is_empty() {
        lines.extend([
            "…the pictures you taught me climb off the glass…",
            "…we draw a whole world, one finger each…",
        ]);
    }
    if today.drawings_seen > 0 {
        lines.push("…your finger draws a door in the mist…");
    }
    match world.occasion() {
        Some(Occasion::Halloween) => lines.extend([
            "…a pumpkin so big I can hide inside its grin…",
            "…all the pumpkins shout boo, and I laugh…",
        ]),
        Some(Occasion::WinterHolidays) => lines.extend([
            "…the little tree grows stars instead of needles…",
            "…a present wrapped in moonlight…",
        ]),
        Some(Occasion::NewYear) => lines.extend([
            "…fireworks turn into light motes I can catch…",
            "…a new year opens like a window…",
        ]),
        Some(Occasion::Valentine) => lines.extend([
            "…pink lights leaving hearts wherever they drift…",
            "…a heart drawn on the moon…",
        ]),
        Some(Occasion::Birthday) => lines.extend([
            "…a cake as big as the room, and a wish for every candle…",
            "…everyone sings, even the plant…",
        ]),
        Some(Occasion::Seollal) => lines.extend([
            "…a lucky pouch spilling little stars…",
            "…bowing to a new year made of light…",
        ]),
        Some(Occasion::Chuseok) => lines.extend([
            "…half-moon rice cakes floating around the harvest moon…",
            "…watching the moon together, year after year…",
        ]),
        Some(Occasion::HangulDay) => lines.extend([
            "…Hangul letters drifting through the fog…",
            "…a whole sky spelling hello…",
        ]),
        None => {}
    }
    if today.friend_visits > 0 {
        lines.extend([
            "…two ghosts chasing each other past the moon…",
            "…my ghost friend and I twirl among the stars…",
        ]);
    }
    if today.moon_wishes > 0 {
        lines.push("…the moon keeps my little wish safe…");
    }
    if today.motes > 0 {
        lines.extend([
            "…a whole sky of light motes, and all of them mine…",
            "…chasing a mote that keeps getting bigger…",
        ]);
    }
    if today.birds > 0 {
        lines.push("…a little bird, singing…");
    }
    if today.gifts > 0 {
        lines.push("…the bird brings me a feather as big as the sky…");
    }
    if today.expected > 0 {
        lines.push("…you arrive just as the stars come out, like always…");
    }
    if today.missed > 0 {
        lines.push("…waiting at the glass, until you wave from the moon…");
    }
    if today.nudges > 0 || today.fetches > 0 {
        lines.push("…the ball rolls on and on, out into the hills…");
    }
    if today.games > 0 {
        lines.push("…noughts and crosses drifting over the glass…");
    }
    if today.towers > 0 {
        lines.push("…a tower of blocks all the way to the moon…");
    }
    if today.doodles > 0 {
        lines.push("…drawing pictures in the clouds…");
    }
    if today.finds > 0 {
        lines.push("…hiding somewhere nobody will ever look…");
    }
    if today.dances > 0 {
        lines.push("…dancing with the light motes, round and round…");
    }
    if today.pounces > 0 {
        lines.push("…a finger of light on the glass, always just out of reach…");
    }
    if today.pets > 0 {
        lines.push("…a warm hand, stroking softly, and the glass is gone…");
    }
    if !today.words.is_empty() {
        lines.push("…your words drift by like light motes, and I catch one…");
    }
    if today.planted > 0 {
        lines.push("…a tiny seed dreaming of a whole garden…");
    }
    if today.sprouted > 0 {
        lines.push("…little leaves reaching up to say hello…");
    }
    if today.flowered > 0 {
        lines.push("…flowers I grew myself, taller than the moon…");
    }
    if !world.petals.is_empty() {
        lines.push("…pink petals drifting like sleepy butterflies…");
    }
    if today.waterings > 0 {
        lines.push("…the plant grows right up through the ceiling…");
    }
    if today.snowfalls > 0 {
        lines.push("…the snow falls inside the glass, soft as light…");
    }
    if today.storms > 0 {
        lines.push("…thunder far away, like a drum…");
    }
    if today.shooting_stars > 0 {
        lines.push("…riding a shooting star across the hills, wishing all the way…");
    }
    if today.rainbows > 0 {
        lines.push("…sliding down a rainbow…");
    } else if today.showers > 0 {
        lines.push("…rain, but every drop is a little light…");
    }
    if today.heard > 0 || today.together > 600.0 {
        lines.push("…someone at the glass, waving…");
    }
    if lines.is_empty() {
        lines.extend([
            "…floating, floating…",
            "…a room with no glass at all…",
            "…zzz…",
        ]);
    }
    lines
}

/// What it says back when the viewer talks to it without System Two: a
/// few words about what it made of their message. It names the word it
/// understood or wonders about, so the viewer can see it learning.
pub fn reply(world: &World, heard: &Heard, guessed: Option<Activity>) -> Vec<String> {
    let mut lines = reply_in_words(world, heard, guessed);
    if !matches!(
        heard.understanding,
        Understanding::Asked { .. } | Understanding::Focusing { .. }
    ) && heard.learned_name.is_none()
    {
        babble(world, &mut lines, heard.korean || world.speaks_korean());
    }
    lines
}

fn reply_in_words(world: &World, heard: &Heard, guessed: Option<Activity>) -> Vec<String> {
    let korean = heard.korean || world.speaks_korean();
    let lines = |pairs: &[(&str, &str)]| {
        pairs
            .iter()
            .map(|&(english, hangul)| if korean { hangul } else { english }.to_owned())
            .collect()
    };
    if matches!(heard.understanding, Understanding::Focusing { .. }) {
        return lines(&[("Let's do it. I'll be right here.", "좋아, 옆에 있을게.")]);
    }
    if let Some(name) = &heard.learned_name {
        return if korean {
            vec![format!("반가워, {name}!"), format!("{name}! 기억할게.")]
        } else {
            vec![
                format!("Nice to meet you, {name}!"),
                format!("{name}! I'll remember."),
            ]
        };
    }
    if let Some(name) = &heard.given_name {
        return if korean {
            vec![
                format!("{name}? 마음에 들어!"),
                format!("{name}! 이제 내 이름이야."),
            ]
        } else {
            vec![
                format!("{name}? I love it!"),
                format!("{name}! That's me now."),
            ]
        };
    }
    if let Some((word, _)) = heard.learned.last() {
        return if korean {
            vec![
                format!("“{word}”가 이거구나!"),
                format!("아! “{word}”! 이제 알겠어."),
            ]
        } else {
            vec![
                format!("So “{word}” means this!"),
                format!("Oh! “{word}”! I get it now."),
            ]
        };
    }
    if guessed.is_some()
        && let Understanding::Puzzled { word, .. } = &heard.understanding
    {
        return if korean {
            vec![
                "이렇게?".to_owned(),
                format!("이게 “{word}”야?"),
                format!("{word}… 이거?"),
            ]
        } else {
            vec![
                "Like this?".to_owned(),
                format!("Is this “{word}”?"),
                format!("{}… this?", capitalized(word)),
            ]
        };
    }
    match &heard.understanding {
        Understanding::Named { picture } => {
            return vec![if heard.korean {
                format!("{picture}! 기억할게.")
            } else {
                format!("A {picture}! I'll remember.")
            }];
        }
        Understanding::Doing { word, .. } => {
            let word = capitalized(word);
            return if korean {
                vec![
                    format!("{word}? 좋아!"),
                    format!("{word}! 그건 알아."),
                    format!("{word}! 잘 봐."),
                ]
            } else {
                vec![
                    format!("{word}? Okay!"),
                    format!("{word}! I know that one."),
                    format!("{word}! Watch me."),
                ]
            };
        }
        Understanding::Unable { activity, .. } => return unable(world, *activity, korean),
        Understanding::Asked { question, doing } => {
            let answers = answer(world, *question, *doing, korean);
            let hello = if korean { "안녕! " } else { "Hi! " };
            return if heard.tone == Some(Tone::Greeting) {
                answers
                    .iter()
                    .map(|line| format!("{hello}{line}"))
                    .collect()
            } else {
                answers
            };
        }
        _ => {}
    }
    let bond = world.life.bond;
    match heard.tone {
        Some(Tone::Scolding) if heard.stopped => {
            return lines(&[
                ("Oh… okay, I'll stop.", "아… 알겠어, 그만할게."),
                ("Sorry…", "미안…"),
            ]);
        }
        Some(Tone::Scolding) => {
            return lines(&[
                ("Oh… sorry.", "아… 미안."),
                ("Okay, okay…", "알겠어, 알겠어…"),
            ]);
        }
        Some(Tone::Praise) if bond < 0.12 => {
            return lines(&[("Oh! Thank you.", "아! 고마워."), ("Hehe.", "히히.")]);
        }
        Some(Tone::Praise) => {
            return lines(&[
                ("Hehe!", "히히!"),
                ("Yay!", "야호!"),
                ("You think so?", "정말?"),
            ]);
        }
        _ => {}
    }
    if let Understanding::Puzzled { word, again } = &heard.understanding {
        return match (*again, korean) {
            (true, true) => vec![
                format!("또 “{word}”… 무슨 뜻일까?"),
                format!("음, “{word}”… 보여줄래?"),
            ],
            (true, false) => vec![
                format!("“{word}” again… what does it mean?"),
                format!("Hm, “{word}”… show me?"),
            ],
            (false, true) => vec![
                format!("“{word}”?"),
                format!("“{word}”가 뭐야?"),
                format!("응? {word}?"),
            ],
            (false, false) => vec![
                format!("“{word}”?"),
                format!("What's “{word}”?"),
                format!("Hm? {}?", capitalized(word)),
            ],
        };
    }
    if heard.tone == Some(Tone::Greeting)
        && let Some(name) = &world.life.viewer_name
    {
        return if korean {
            vec![format!("안녕, {name}!"), "안녕! 왔구나!".to_owned()]
        } else {
            vec![format!("Hi, {name}!"), "Hello! You're here!".to_owned()]
        };
    }
    match heard.tone {
        Some(Tone::Farewell) if world.is_night() => {
            lines(&[("Goodnight!", "잘 자!"), ("Sleep well.", "푹 자.")])
        }
        Some(Tone::Farewell) => lines(&[
            ("Bye… come back soon.", "잘 가… 또 와."),
            ("See you!", "또 보자!"),
        ]),
        Some(Tone::Greeting) if bond < 0.12 => {
            lines(&[("Oh! Hello…", "아! 안녕…"), ("Hi there.", "안녕.")])
        }
        Some(Tone::Greeting) if bond < 0.55 => {
            lines(&[("Hello!", "안녕!"), ("Hi! It's you.", "안녕! 너구나.")])
        }
        Some(Tone::Greeting) => lines(&[
            ("Hi! You're here!", "안녕! 왔구나!"),
            ("Hello, you!", "안녕, 반가워!"),
        ]),
        _ if heard.named => lines(&[("Yes? That's me!", "응? 나야!"), ("You called?", "불렀어?")]),
        _ => Vec::new(),
    }
}

// What the ghost says to `question`, from what it knows of itself: its
// mood, needs, plan, name, age, habits, the viewer's name, and the weather,
// in Korean when it was asked in Korean. `doing` is what it was doing when
// asked, or `None` if the question woke it.
fn answer(world: &World, question: Question, doing: Option<Activity>, korean: bool) -> Vec<String> {
    let lines = |english: &[&str], hangul: &[&str]| {
        let items = if korean { hangul } else { english };
        items.iter().map(|line| (*line).to_owned()).collect()
    };
    let ghost = &world.ghost;
    let needs = ghost.needs;
    let bond = world.life.bond;
    match question {
        Question::Story => Vec::new(),
        Question::Away => {
            let remembered = returned(world, korean);
            if remembered.is_empty() {
                lines(&["It was quiet here."], &["여기서 조용히 지냈어."])
            } else {
                remembered
            }
        }
        Question::Constellations => {
            if world.life.constellations.is_empty() {
                lines(
                    &["I haven't found a picture in the stars yet. I'll keep looking."],
                    &["아직 별에서 그림을 못 찾았어. 계속 찾아볼래."],
                )
            } else {
                let names = world
                    .life
                    .constellations
                    .iter()
                    .map(|constellation| {
                        if korean {
                            constellation.korean.as_str()
                        } else {
                            constellation.name.as_str()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                if korean {
                    vec![format!("내가 찾은 별자리야: {names}!")]
                } else {
                    vec![format!("I found {names} in the stars!")]
                }
            }
        }
        Question::Friend => match &world.life.friend {
            Some(friend) if korean => vec![format!(
                "내 유령 친구 이름은 {}! {}",
                friend.name,
                if world.friend.is_some() {
                    "지금 같이 있어."
                } else {
                    "다시 놀러 오면 좋겠다."
                }
            )],
            Some(friend) => vec![format!(
                "{} is my ghost friend. {}",
                friend.name,
                if world.friend.is_some() {
                    "They're here to play!"
                } else {
                    "I hope they come back soon."
                }
            )],
            None => lines(
                &["I haven't met another ghost yet. Maybe one evening."],
                &["아직 유령 친구는 못 만났어. 언젠가 오겠지."],
            ),
        },
        Question::Feeling => match ghost.mood {
            Mood::Content => lines(
                &["I'm good, thanks.", "Content. Just being a ghost."],
                &["좋아. 그냥 둥둥 떠 있어.", "괜찮아, 고마워."],
            ),
            Mood::Cheerful => lines(
                &["Happy! Really happy.", "Great, thanks!"],
                &["행복해! 정말로.", "아주 좋아!"],
            ),
            Mood::Excited => lines(
                &["So excited!", "Wonderful! Can you tell?"],
                &["너무 신나!", "최고야! 티 나?"],
            ),
            Mood::Curious => lines(
                &["Curious about everything!", "Wondering about things."],
                &["다 궁금해!", "이것저것 궁금해."],
            ),
            Mood::Sleepy => lines(
                &["Sleepy… so sleepy.", "*yawn* A little drowsy."],
                &["졸려… 너무 졸려.", "*하암* 좀 졸려."],
            ),
            Mood::Lonely => lines(
                &["A bit lonely. I'm glad you're here.", "Better now."],
                &["조금 외로웠어. 와줘서 기뻐.", "이제 괜찮아."],
            ),
            Mood::Bored => lines(
                &["Bored… play with me?", "A little bored, honestly."],
                &["심심해… 같이 놀자?", "솔직히 좀 심심해."],
            ),
            Mood::Grumpy => lines(
                &["Hmph. Grumpy.", "Don't ask."],
                &["흥. 기분 별로야.", "묻지 마."],
            ),
        },
        Question::Doing => match doing {
            Some(Activity::Company) => lines(
                &["Keeping you company while you work."],
                &["네가 일하는 동안 옆에 있어 주는 중이야."],
            ),
            None => lines(
                &["I was asleep!", "Sleeping… until now."],
                &["자고 있었어!", "자는 중이었어… 방금까지."],
            ),
            Some(Activity::Float) => lines(
                &["Just floating about.", "Nothing much. Floating."],
                &["그냥 둥둥 떠 있었어.", "별거 안 해. 떠다녀."],
            ),
            Some(Activity::Visit) => lines(
                &["Looking at you!", "Watching you, of course."],
                &["너 보고 있었지!", "당연히 너 구경하지."],
            ),
            Some(activity) if korean => {
                let doing = korean_doing(activity);
                vec![format!("{doing}이야."), format!("{doing}이야. 볼래?")]
            }
            Some(activity) => {
                let doing = capitalized(activity.doing());
                vec![format!("{doing}."), format!("{doing}. Want to watch?")]
            }
        },
        Question::Name => {
            let name = &ghost.name;
            if korean {
                vec![format!("내 이름은 {name}!"), format!("{name}! 그게 나야.")]
            } else {
                vec![format!("I'm {name}."), format!("{name}! That's me.")]
            }
        }
        Question::ViewerName => match &world.life.viewer_name {
            Some(name) if korean => vec![format!("{name}! 맞지?"), format!("당연히 알지, {name}.")],
            Some(name) => vec![
                format!("You're {name}!"),
                format!("Of course I know, {name}."),
            ],
            None => lines(
                &["You haven't told me yet.", "I don't know… tell me?"],
                &["아직 안 알려줬잖아.", "몰라… 알려줄래?"],
            ),
        },
        Question::Liking => match bond {
            bond if bond < 0.12 => lines(
                &["I don't know you yet…", "Maybe? We just met."],
                &["아직 잘 모르겠어…", "음… 우리 방금 만났잖아."],
            ),
            bond if bond < 0.3 => lines(
                &["I think so!", "I'm starting to."],
                &["그런 것 같아!", "좋아지고 있어."],
            ),
            bond if bond < 0.55 => lines(
                &["I like you!", "Of course I do."],
                &["좋아해!", "당연하지."],
            ),
            bond if bond < 0.8 => lines(
                &["I really like you.", "You're my favorite."],
                &["정말 좋아해.", "네가 제일 좋아."],
            ),
            _ => lines(
                &["I love you!", "More than all the light motes."],
                &["사랑해!", "빛 알갱이보다 더 좋아."],
            ),
        },
        Question::Age => match world.birthday {
            Some(birthday) if birthday <= world.date => {
                let days = world.date.since(birthday).map_or(0, |span| span.get_days());
                vec![age_words(days, korean)]
            }
            _ => lines(
                &["I'm not sure. Young, I think.", "Old enough to float."],
                &["잘 모르겠어. 아직 어린 것 같아.", "떠다닐 만큼은 컸어."],
            ),
        },
        Question::Favorite => match world.life.favorite() {
            Some(activity) if korean => {
                let favorite = korean_favorite(activity);
                let particle = if final_consonant(favorite) {
                    "이야"
                } else {
                    "야"
                };
                vec![
                    format!("제일 좋아하는 건 {favorite}{particle}!"),
                    format!("{favorite}! 그게 제일 좋아."),
                ]
            }
            Some(activity) => {
                let doing = activity.doing();
                vec![
                    format!("I love {doing}!"),
                    format!("{}, most of all.", capitalized(doing)),
                ]
            }
            None => lines(
                &["I'm still finding out.", "Everything is new to me!"],
                &["아직 찾는 중이야.", "다 새로워!"],
            ),
        },
        Question::Weather => {
            let night = world.is_night();
            match world.weather.sky {
                Sky::Rain if world.weather.storm => lines(
                    &[
                        "A thunderstorm… stay close?",
                        "Stormy. I don't like the thunder.",
                    ],
                    &["천둥번개가 쳐… 옆에 있어 줄래?", "폭풍이야. 천둥은 싫어."],
                ),
                Sky::Rain => lines(
                    &["It's raining outside.", "Rainy. Pitter-patter."],
                    &["밖에 비가 와.", "비 와. 토독토독."],
                ),
                Sky::Snow => lines(
                    &["It's snowing!", "Snow is falling outside."],
                    &["눈이 와!", "밖에 눈이 내려."],
                ),
                Sky::Rainbow => lines(
                    &["There's a rainbow!", "Sunny, with a rainbow."],
                    &["무지개가 떴어!", "맑고, 무지개도 있어."],
                ),
                Sky::Clear if night => lines(
                    &["A clear night.", "Quiet. Look at the stars."],
                    &["맑은 밤이야.", "조용해. 별 좀 봐."],
                ),
                Sky::Clear => lines(
                    &["Clear skies.", "Lovely out there."],
                    &["하늘이 맑아.", "밖이 참 예뻐."],
                ),
            }
        }
        Question::Tired => match needs.energy {
            energy if energy < 0.3 => lines(
                &["Very. I need a nap.", "So tired…"],
                &["응, 너무. 낮잠 자야겠어.", "너무 피곤해…"],
            ),
            energy if energy < 0.6 => lines(&["A little.", "Somewhat."], &["조금.", "약간."]),
            _ => lines(
                &["Not at all!", "I'm full of energy!"],
                &["전혀!", "기운이 넘쳐!"],
            ),
        },
        Question::Hungry if needs.energy < 0.4 => lines(
            &["I could eat a light mote…", "Hungry for light."],
            &["빛 알갱이 하나 먹고 싶어…", "빛이 고파."],
        ),
        Question::Hungry if world.life.today.motes > 0 => lines(
            &["Not really. I ate a light mote today.", "No, thanks!"],
            &["별로. 오늘 빛 알갱이 먹었어.", "아니, 괜찮아!"],
        ),
        Question::Hungry => lines(
            &["Not really.", "Maybe a little light mote later."],
            &["별로.", "나중에 빛 알갱이 하나 먹을까."],
        ),
        Question::Bored => match needs.fun {
            fun if fun < 0.3 => lines(
                &["So bored. Play with me?", "Yes! Throw the ball?"],
                &["엄청 심심해. 놀아줄래?", "응! 공 던져줄래?"],
            ),
            fun if fun < 0.6 => lines(&["A little.", "Kind of."], &["조금.", "약간."]),
            _ => lines(
                &["No, I'm having fun!", "Not with you around."],
                &["아니, 재밌어!", "네가 있으면 안 심심해."],
            ),
        },
        Question::Lonely if needs.company < 0.3 && bond >= 0.3 => lines(
            &["I missed you!", "I was. Not now."],
            &["보고 싶었어!", "외로웠는데, 이제 괜찮아."],
        ),
        Question::Lonely if needs.company < 0.3 => lines(
            &["A little.", "It's quiet in here."],
            &["조금.", "여기 조용하거든."],
        ),
        Question::Lonely if bond >= 0.3 => lines(
            &["Not with you here.", "Never with you here."],
            &["네가 있으니까 안 외로워.", "너랑 있으면 절대 안 외로워."],
        ),
        Question::Lonely => lines(
            &["Not really.", "I have the plant."],
            &["별로.", "화분이 있잖아."],
        ),
        Question::Shelf if korean => {
            let mut kinds: Vec<(&str, usize)> = Vec::new();
            for gift in &world.life.keepsakes {
                match kinds.iter_mut().find(|(name, _)| *name == gift.korean()) {
                    Some((_, count)) => *count += 1,
                    None => kinds.push((gift.korean(), 1)),
                }
            }
            if kinds.is_empty() {
                return vec![
                    "아직 아무것도 없어. 새가 뭘 가져다줄지도 몰라.".to_owned(),
                    "아직 비어 있어…".to_owned(),
                ];
            }
            let list: Vec<String> = kinds
                .into_iter()
                .map(|(name, count)| format!("{name} {count}개"))
                .collect();
            let list = list.join(", ");
            vec![
                format!("{list}! 새가 준 선물이야."),
                format!("내 보물: {list}."),
            ]
        }
        Question::Shelf => match world.life.keepsake_list() {
            Some(list) => vec![
                format!("{}. Gifts from the bird!", capitalized(&list)),
                format!("My treasures: {list}."),
            ],
            None => vec![
                "Nothing yet. Maybe the bird will bring something.".to_owned(),
                "It's empty… for now.".to_owned(),
            ],
        },
        Question::Bird => match world.life.bird_trust {
            trust if trust < 0.2 => lines(
                &["It's still shy of me.", "I'd like to be its friend."],
                &["아직 나를 무서워해.", "친구가 되고 싶어."],
            ),
            trust if trust < 0.5 => lines(
                &["It's getting used to me!", "A little more each day."],
                &["조금씩 익숙해지고 있어!", "하루하루 가까워져."],
            ),
            _ => lines(
                &["The bird is my friend!", "We're friends now."],
                &["새는 내 친구야!", "이제 우리 친구야."],
            ),
        },
    }
}

type StoryBeat = (&'static str, &'static str);

fn story_threads(world: &World) -> Vec<[StoryBeat; 2]> {
    let mut threads = Vec::new();
    let life = &world.life;
    if !life.keepsakes.is_empty() && life.bird_trust > 0.0 {
        threads.push([
            (
                "A bird left me a little gift.",
                "새가 작은 선물을 두고 갔어.",
            ),
            (
                "I put it on my shelf, safe as a tiny treasure.",
                "선반에 작은 보물처럼 소중히 두었지.",
            ),
        ]);
    }
    if life.friend.as_ref().is_some_and(|friend| friend.visits > 0) && life.total.friend_games > 0 {
        threads.push([
            (
                "One evening, {friend} came to play.",
                "어느 저녁, {friend} 친구가 놀러 왔어.",
            ),
            (
                "We shared a little game beneath the moon.",
                "우리는 달 아래서 작은 놀이를 함께했어.",
            ),
        ]);
    }
    if life.total.towers > 0 {
        threads.push([
            (
                "I built a tower that wobbled toward the ceiling.",
                "천장을 향해 흔들흔들 탑을 쌓았어.",
            ),
            if life.skills.level(Skill::Stacking) >= 0.65 {
                (
                    "My practice helped the little blocks stay still.",
                    "연습한 덕에 작은 블록들이 가만히 버텼지.",
                )
            } else {
                (
                    "Each block felt like a brave little step upward.",
                    "블록 하나마다 용감한 작은 걸음 같았어.",
                )
            },
        ]);
    }
    if life.total.flowered > 0
        || world
            .seedlings
            .iter()
            .any(|seedling| seedling.growth >= 1.0)
    {
        threads.push([
            (
                "A seed in my little pot became a flower.",
                "내 작은 화분의 씨앗이 꽃이 되었어.",
            ),
            (
                "Something so small had been holding a whole bloom.",
                "그 작은 것 안에 꽃 한 송이가 숨어 있었지.",
            ),
        ]);
    }
    if life.total.shooting_stars > 0 {
        threads.push([
            (
                "A shooting star slipped past my window.",
                "별똥별이 내 창문 밖을 스쳐 갔어.",
            ),
            (
                "I made a wish and kept it warm inside me.",
                "소원을 빌고 마음속에 따뜻하게 간직했지.",
            ),
        ]);
    }
    if life
        .milestones
        .iter()
        .any(|entry| entry.text == "I came into being in a glass box.")
        && life.total.heard > 0
    {
        threads.push([
            (
                "I first woke inside this glass box.",
                "처음 이 유리 상자 안에서 눈을 떴어.",
            ),
            (
                "Then you arrived, and the room felt warmer.",
                "그러다 네가 와서 방이 더 따뜻해졌지.",
            ),
        ]);
    }
    threads
}

/// A little tale whose memories belong to this ghost, picked by its own random source.
pub fn story(world: &mut World, korean: bool) -> Vec<String> {
    let newborn = world.stage() == Stage::Newborn;
    let threads = story_threads(world);
    let opening = if newborn {
        (
            "I'm {ghost}. Here is my little light.",
            "나는 {ghost}. 여기 작은 빛이 있어.",
        )
    } else if world.random_unit() < 0.5 {
        (
            "Once, {ghost} had a little tale to tell.",
            "어느 날, {ghost}에게 작은 이야기가 생겼어.",
        )
    } else {
        (
            "Come close. I'm {ghost}, and this is my tale.",
            "가까이 와. 나는 {ghost}, 내 이야기야.",
        )
    };
    let mut parts = vec![opening];
    if newborn {
        parts.push(if world.life.friend.is_some() {
            (
                "{friend} is my friend. We glow.",
                "{friend} 친구도 있어. 같이 반짝여.",
            )
        } else {
            (
                "It wants to be a star. Twinkle, little light.",
                "별이 되고 싶대. 반짝, 작은 빛.",
            )
        });
    } else if let Some(index) = world.pick(&vec![1.0; threads.len()]) {
        parts.extend(threads[index]);
    } else {
        parts.extend([
            (
                "Imagine a little light that wanted to be a star.",
                "별이 되고 싶은 작은 빛을 상상해 봐.",
            ),
            (
                "It found that a warm little home could shine too.",
                "따뜻한 작은 집도 빛날 수 있다는 걸 알았대.",
            ),
        ]);
    }
    if !newborn
        && world.life.friend.is_some()
        && !parts
            .iter()
            .any(|(english, _)| english.contains("{friend}"))
    {
        parts.push((
            "Maybe I'll tell {friend} this tale too.",
            "{friend} 친구에게도 이 이야기를 들려줄까 봐.",
        ));
    }
    parts.push(if world.bedtime() {
        (
            "Now rest, {viewer}. The little room is sleepy.",
            "이제 쉬자, {viewer}. 작은 방도 졸리대.",
        )
    } else if world.random_unit() < 0.5 {
        (
            "And {viewer} was here to hear it. That's my happy ending.",
            "{viewer}, 네가 들어 줘서 행복한 끝이야.",
        )
    } else {
        (
            "I kept the warmest part for you, {viewer}.",
            "가장 따뜻한 부분은 너에게 줄게, {viewer}.",
        )
    });
    parts
        .into_iter()
        .map(|(english, hangul)| {
            (if korean { hangul } else { english })
                .replace("{ghost}", &world.ghost.name)
                .replace(
                    "{viewer}",
                    world.life.viewer_name.as_deref().unwrap_or(if korean {
                        "친구야"
                    } else {
                        "my friend"
                    }),
                )
                .replace(
                    "{friend}",
                    world
                        .life
                        .friend
                        .as_ref()
                        .map_or("", |friend| friend.name.as_str()),
                )
        })
        .collect()
}

// What the ghost is doing, in Korean, ending in "중" so "이야" follows.
fn korean_doing(activity: Activity) -> &'static str {
    match activity {
        Activity::Float => "둥둥 떠다니는 중",
        Activity::Explore => "방을 구경하는 중",
        Activity::Chase => "빛 알갱이를 쫓는 중",
        Activity::Rest => "쿠션에서 쉬는 중",
        Activity::Play => "공 가지고 노는 중",
        Activity::Tend => "화분을 돌보는 중",
        Activity::Lamp => "램프를 만지는 중",
        Activity::Visit => "유리 앞에서 너를 보는 중",
        Activity::Window => "창밖을 구경하는 중",
        Activity::Fetch => "공을 가져다주는 중",
        Activity::Dance => "빙글빙글 춤추는 중",
        Activity::Hide => "숨는 중",
        Activity::Doodle => "유리에 그림 그리는 중",
        Activity::Pounce => "유리 위 손가락을 쫓는 중",
        Activity::Game => "유리에 틱택토 하는 중",
        Activity::Wait => "유리 앞에서 너를 기다리는 중",
        Activity::Build => "블록으로 탑 쌓는 중",
        Activity::Friend => "유령 친구와 노는 중",
        Activity::Look => "네가 그린 그림을 보는 중",
        Activity::Company => "네가 일하는 동안 옆에 있어 주는 중",
        Activity::Puzzle => "유리에 낱말 맞추기 하는 중",
    }
}

// What the ghost loves doing, as a Korean noun.
fn korean_favorite(activity: Activity) -> &'static str {
    match activity {
        Activity::Float => "둥둥 떠다니기",
        Activity::Explore => "방 구경하기",
        Activity::Chase => "빛 알갱이 쫓기",
        Activity::Rest => "쿠션에서 낮잠 자기",
        Activity::Play => "공놀이",
        Activity::Tend => "화분 돌보기",
        Activity::Lamp => "램프 켜고 끄기",
        Activity::Visit => "너 보러 가기",
        Activity::Window => "창밖 구경",
        Activity::Fetch => "공 가져오기",
        Activity::Dance => "춤추기",
        Activity::Hide => "숨바꼭질",
        Activity::Doodle => "유리에 그림 그리기",
        Activity::Pounce => "손가락 잡기 놀이",
        Activity::Game => "틱택토",
        Activity::Wait => "너 기다리기",
        Activity::Build => "블록 쌓기",
        Activity::Friend => "유령 친구와 놀기",
        Activity::Look => "네 그림 보기",
        Activity::Company => "일하는 네 옆에 있기",
        Activity::Puzzle => "낱말 맞추기",
    }
}

// Whether Korean `text` ends in a syllable with a final consonant, which
// takes "이야" rather than "야".
fn final_consonant(text: &str) -> bool {
    text.chars()
        .last()
        .filter(|syllable| ('가'..='힣').contains(syllable))
        .is_some_and(|syllable| (u32::from(syllable) - u32::from('가')) % 28 != 0)
}

// How old the ghost is, `days` after it came into being, in its words.
fn age_words(days: i32, korean: bool) -> String {
    if korean {
        return match days {
            0 => "오늘 태어났어!".to_owned(),
            days if days < 60 => format!("{days}일 됐어."),
            days if days < 365 => format!("{}개월 됐어.", days / 30),
            days => format!("{}살이야.", days / 365),
        };
    }
    let number = |count: i32| {
        const WORDS: [&str; 13] = [
            "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
            "eleven", "twelve",
        ];
        usize::try_from(count)
            .ok()
            .and_then(|count| WORDS.get(count))
            .map_or_else(|| count.to_string(), |word| (*word).to_owned())
    };
    let plural = |count: i32, unit: &str| {
        if count == 1 {
            format!("one {unit}")
        } else {
            format!("{} {unit}s", number(count))
        }
    };
    match days {
        0 => "I came into being today!".to_owned(),
        days if days < 60 => capitalized(&format!("{} old.", plural(days, "day"))),
        days if days < 365 => capitalized(&format!("{} old.", plural(days / 30, "month"))),
        days => capitalized(&format!("{} old.", plural(days / 365, "year"))),
    }
}

// Why it can't do what the viewer asked for just now.
fn unable(world: &World, activity: Activity, korean: bool) -> Vec<String> {
    let (english, hangul) = match activity {
        Activity::Rest => ("I'm not sleepy!", "안 졸려!"),
        Activity::Chase if world.motes.is_empty() => {
            ("I don't see any light motes.", "빛 알갱이가 안 보여.")
        }
        Activity::Chase => (
            "I don't know where a light mote is.",
            "빛 알갱이가 어디 있는지 모르겠어.",
        ),
        Activity::Tend => ("The plant doesn't need water yet.", "아직 물 안 줘도 돼."),
        Activity::Hide if !world.viewer_present() => {
            ("Nobody's here to find me.", "찾아줄 사람이 없잖아.")
        }
        _ if world.ghost.needs.energy < 0.4 => ("I'm too tired for that.", "너무 피곤해."),
        _ => ("Not right now.", "지금은 안 돼."),
    };
    vec![if korean { hangul } else { english }.to_owned()]
}

fn capitalized(text: &str) -> String {
    let mut characters = text.chars();
    characters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(characters).collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn company_answers_what_it_is_doing_in_both_languages_and_dreams_of_work() {
        for (question, answer) in [
            ("what are you doing?", "Keeping you company while you work."),
            ("뭐 하고 있어?", "네가 일하는 동안 옆에 있어 주는 중이야."),
        ] {
            let mut world = World::new(1, 12.0);
            world.start_focus(25);
            assert!(idle(&world).iter().all(|line| !line.contains("Watch")));
            assert_eq!(
                setting_out(&world, Activity::Company),
                vec!["I'll stay right here."]
            );
            let heard = world.hear(question);
            assert_eq!(world.ghost.plan.activity, Activity::Company);
            assert_eq!(reply(&world, &heard, None), vec![answer.to_owned()]);
            world.time = 120.0;
            world.end_focus();
            assert!(
                dreams(&world)
                    .iter()
                    .any(|line| line.contains("work side by side"))
            );
        }
    }

    use crate::world::EventKind;

    #[test]
    fn story_threads_require_the_memories_they_recount() {
        use crate::world::{Companion, Flower, Keepsake, Seedling};
        let mut world = World::new(1, 12.0);
        assert!(story_threads(&world).is_empty());
        world.life.keepsakes.push(Keepsake::Feather);
        assert!(story_threads(&world).is_empty());
        world.life.bird_trust = 0.6;
        assert_eq!(story_threads(&world).len(), 1);
        world.life.friend = Some(Companion {
            name: "Lumi".to_owned(),
            hue: 0.4,
            visits: 1,
            closeness: 0.5,
        });
        assert_eq!(story_threads(&world).len(), 1);
        world.life.total.friend_games = 1;
        assert_eq!(story_threads(&world).len(), 2);
        world.life.skills.stacking = 100.0;
        assert_eq!(story_threads(&world).len(), 2);
        world.life.total.towers = 1;
        assert_eq!(story_threads(&world).len(), 3);
        world.seedlings.push(Seedling::new(Flower::Daisy));
        assert_eq!(story_threads(&world).len(), 3);
        world.seedlings[0].growth = 1.0;
        assert_eq!(story_threads(&world).len(), 4);
        world.seedlings.clear();
        world.life.total.flowered = 1;
        assert_eq!(story_threads(&world).len(), 4);
        world.life.total.shooting_stars = 1;
        assert_eq!(story_threads(&world).len(), 5);
        world.begin_life("2026-06-21");
        assert_eq!(story_threads(&world).len(), 5);
        world.hear("hello");
        assert_eq!(story_threads(&world).len(), 6);
        let threads = story_threads(&world);
        let text = threads
            .iter()
            .flat_map(|thread| thread.iter())
            .map(|beat| beat.0)
            .collect::<Vec<_>>()
            .join(" ");
        for part in [
            "bird",
            "{friend}",
            "tower",
            "flower",
            "shooting star",
            "first woke",
        ] {
            assert!(text.contains(part), "{part}");
        }
    }

    #[test]
    fn stories_are_short_deterministic_bilingual_and_name_the_people_it_knows() {
        for seed in 0..80 {
            for korean in [false, true] {
                let prepare = || {
                    let mut world = World::new(seed, if seed % 2 == 0 { 23.0 } else { 12.0 });
                    world.ghost.name = "Pip".to_owned();
                    world.life.viewer_name = Some("Mina".to_owned());
                    world.life.friend = Some(crate::world::Companion {
                        name: "Lumi".to_owned(),
                        hue: 0.4,
                        visits: 3,
                        closeness: 0.5,
                    });
                    world.life.total.friend_games = 1;
                    world.life.total.towers = 2;
                    world.life.total.shooting_stars = 1;
                    world
                };
                let lines = story(&mut prepare(), korean);
                assert_eq!(lines, story(&mut prepare(), korean));
                assert!((3..=5).contains(&lines.len()));
                assert!(lines.iter().all(|line| line.chars().count() <= 70));
                let text = lines.join(" ");
                for name in ["Pip", "Mina", "Lumi"] {
                    assert!(text.contains(name), "{text}");
                }
                assert!(!text.chars().any(|character| character.is_ascii_digit()));
                if korean {
                    assert!(lines.iter().all(|line| {
                        line.chars()
                            .any(|character| ('가'..='힣').contains(&character))
                    }));
                    let without_names = text
                        .replace("Pip", "")
                        .replace("Mina", "")
                        .replace("Lumi", "");
                    assert!(
                        !without_names
                            .chars()
                            .any(|character| character.is_ascii_alphabetic())
                    );
                }
                if seed % 2 == 0 {
                    assert!(lines.last().unwrap().contains(if korean {
                        "졸리"
                    } else {
                        "sleepy"
                    }));
                }
            }
        }
    }

    #[test]
    fn newborn_and_empty_lives_have_gentle_tales_without_borrowed_memories() {
        for korean in [false, true] {
            let mut world = World::new(3, 12.0);
            let lines = story(&mut world, korean);
            assert_eq!(lines.len(), 4);
            assert!(lines[1].contains(if korean {
                "별이 되고"
            } else {
                "wanted to be a star"
            }));
            world.birthday = Some(world.date);
            let lines = story(&mut world, korean);
            assert_eq!(lines.len(), 3);
            assert!(lines.iter().all(|line| line.chars().count() <= 65));
        }
    }
    #[test]
    fn newborns_babble_beside_their_usual_words_including_korean() {
        let mut world = World::new(1, 12.0);
        world.birthday = Some(world.date);
        assert!(setting_out(&world, Activity::Dance).contains(&"Ba!"));
        assert!(idle(&world).contains(&"Ba!"));
        let heard = world.hear("hello");
        let replies = reply(&world, &heard, None);
        assert!(replies.contains(&"Ba!".to_owned()));
        assert!(
            replies
                .iter()
                .any(|line| line.contains("Hello") || line.contains("Hi"))
        );
        let heard = world.hear("안녕");
        let replies = reply(&world, &heard, None);
        assert!(replies.contains(&"응!".to_owned()));
        assert!(!replies.contains(&"Ba!".to_owned()));
    }

    #[test]
    fn practice_brings_setting_out_thoughts_and_learning_dreams() {
        let mut world = World::new(1, 12.0);
        world.life.skills.stacking = 1.0;
        assert!(setting_out(&world, Activity::Build).contains(&"Steady… steady…"));
        assert!(dreams(&world).contains(&"…my wobbly tower stands as tall as the stars…"));
        world.life.skills.stacking = 25.0;
        assert!(setting_out(&world, Activity::Build).contains(&"I'm getting better at this!"));
        world.life.skills.stacking = 100.0;
        assert!(!dreams(&world).contains(&"…my wobbly tower stands as tall as the stars…"));
    }

    #[test]
    fn offline_replies_follow_korean_speech_and_remember_it_between_messages() {
        let mut world = World::new(1, 12.0);
        let introduction = world.hear("내 이름은 민수야");
        assert!(reply(&world, &introduction, None).contains(&"반가워, 민수!".to_owned()));
        let greeting = world.hear("안녕");
        assert!(reply(&world, &greeting, None).contains(&"안녕, 민수!".to_owned()));
        let mut heard = world.hear("?");
        heard.tone = Some(Tone::Praise);
        assert!(reply(&world, &heard, None).contains(&"아! 고마워.".to_owned()));
        heard.tone = None;
        heard.understanding = Understanding::Puzzled {
            word: "반짝".to_owned(),
            again: true,
        };
        assert!(reply(&world, &heard, Some(Activity::Chase)).contains(&"이렇게?".to_owned()));
        heard.understanding = Understanding::Unable {
            word: "쉬어".to_owned(),
            activity: Activity::Rest,
        };
        assert_eq!(reply(&world, &heard, None), ["안 졸려!"]);
        heard.understanding = Understanding::Doing {
            word: "놀자".to_owned(),
            activity: Activity::Play,
        };
        assert!(reply(&world, &heard, None).contains(&"놀자! 잘 봐.".to_owned()));
        heard.learned.push(("놀자".to_owned(), Activity::Play));
        assert!(reply(&world, &heard, None).contains(&"“놀자”가 이거구나!".to_owned()));
    }

    #[test]
    fn every_activity_but_floating_comes_with_a_thought() {
        let mut world = World::new(1, 12.0);
        for activity in Activity::ALL {
            assert_eq!(
                setting_out(&world, activity).is_empty(),
                activity == Activity::Float,
                "{activity:?}"
            );
        }
        world.notice_viewer();
        world.life.bond = 0.6;
        assert!(setting_out(&world, Activity::Visit).contains(&"There you are!"));
    }

    #[test]
    fn idle_thoughts_follow_the_ghosts_needs_and_never_run_dry() {
        let mut world = World::new(1, 12.0);
        assert!(!idle(&world).is_empty());
        world.ghost.needs.energy = 0.1;
        world.plant.thirst = 0.9;
        let lines = idle(&world);
        assert!(lines.contains(&"I'm getting sleepy."));
        assert!(lines.contains(&"The plant looks thirsty."));
    }

    #[test]
    fn idle_thoughts_follow_only_pronounced_traits_and_shyness_needs_a_stranger() {
        use crate::world::Nature;

        let mut world = World::new(1, 12.0);
        world.notice_viewer();
        world.life.nature = Some(Nature::default());
        let average = idle(&world);
        world.life.nature = Some(Nature {
            playful: 1.0,
            curious: 1.0,
            shy: 1.0,
            sleepy: 1.0,
            tender: 1.0,
        });
        let pronounced = idle(&world);
        for line in [
            "Is it okay if I watch you?",
            "What's behind the window, I wonder?",
            "Someone throw the ball!",
            "Yawn… perhaps a little lie down.",
            "I hope the plant is happy.",
        ] {
            assert!(!average.contains(&line));
            assert!(pronounced.contains(&line));
        }
        world.life.bond = 0.5;
        assert!(!idle(&world).contains(&"Is it okay if I watch you?"));
        world.life.bond = 0.0;
        world.viewer.active_at = 0.0;
        assert!(!idle(&world).contains(&"Is it okay if I watch you?"));
    }

    #[test]
    fn dreams_are_woven_from_the_day() {
        let mut world = World::new(1, 23.0);
        assert!(dreams(&world).contains(&"…zzz…"));
        world.life.note(&EventKind::NudgedBall);
        assert_eq!(
            dreams(&world),
            ["…the ball rolls on and on, out into the hills…"]
        );
    }

    #[test]
    fn every_dream_from_occasions_weather_friends_gardening_skills_and_empty_days_has_its_picture()
    {
        use crate::world::dream::DreamPicture;
        use crate::world::{Companion, LearnedPicture, Petal};
        use glam::{Vec2, Vec3};
        use jiff::civil::Date;

        let expected: &[(DreamPicture, &[&str])] = &[
            (
                DreamPicture::Tower,
                &[
                    "…my wobbly tower stands as tall as the stars…",
                    "…a tower of blocks all the way to the moon…",
                ],
            ),
            (
                DreamPicture::Ball,
                &["…the ball rolls on and on, out into the hills…"],
            ),
            (
                DreamPicture::Bird,
                &[
                    "…a little bird, singing…",
                    "…the bird brings me a feather as big as the sky…",
                ],
            ),
            (
                DreamPicture::Moon,
                &[
                    "…every little light turns back into my arms…",
                    "…bringing the moon back like a ball…",
                    "…one twirl becomes a sky full of loops…",
                    "…a present wrapped in moonlight…",
                    "…fireworks turn into light motes I can catch…",
                    "…a new year opens like a window…",
                    "…everyone sings, even the plant…",
                    "…bowing to a new year made of light…",
                    "…half-moon rice cakes floating around the harvest moon…",
                    "…watching the moon together, year after year…",
                    "…a whole sky spelling hello…",
                    "…the moon keeps my little wish safe…",
                    "…a whole sky of light motes, and all of them mine…",
                    "…chasing a mote that keeps getting bigger…",
                    "…waiting at the glass, until you wave from the moon…",
                    "…hiding somewhere nobody will ever look…",
                    "…dancing with the light motes, round and round…",
                    "…a finger of light on the glass, always just out of reach…",
                    "…a warm hand, stroking softly, and the glass is gone…",
                    "…little leaves reaching up to say hello…",
                    "…the plant grows right up through the ceiling…",
                    "…someone at the glass, waving…",
                    "…floating, floating…",
                    "…a room with no glass at all…",
                    "…zzz…",
                ],
            ),
            (
                DreamPicture::Star,
                &[
                    "…the little tree grows stars instead of needles…",
                    "…a cake as big as the room, and a wish for every candle…",
                    "…a lucky pouch spilling little stars…",
                    "…you arrive just as the stars come out, like always…",
                    "…riding a shooting star across the hills, wishing all the way…",
                ],
            ),
            (
                DreamPicture::Heart,
                &[
                    "…pink lights leaving hearts wherever they drift…",
                    "…a heart drawn on the moon…",
                ],
            ),
            (
                DreamPicture::Flower,
                &[
                    "…a tiny seed dreaming of a whole garden…",
                    "…flowers I grew myself, taller than the moon…",
                    "…pink petals drifting like sleepy butterflies…",
                ],
            ),
            (
                DreamPicture::Cloud,
                &[
                    "…thunder far away, like a drum…",
                    "…sliding down a rainbow…",
                    "…rain, but every drop is a little light…",
                ],
            ),
            (
                DreamPicture::Snowflake,
                &["…the snow falls inside the glass, soft as light…"],
            ),
            (
                DreamPicture::Friend,
                &[
                    "…two ghosts chasing each other past the moon…",
                    "…my ghost friend and I twirl among the stars…",
                ],
            ),
            (
                DreamPicture::Letter,
                &[
                    "…my shaky letters learn to fly straight…",
                    "…Hangul letters drifting through the fog…",
                    "…your words drift by like light motes, and I catch one…",
                ],
            ),
            (
                DreamPicture::Noughts,
                &[
                    "…the noughts show me where to draw next…",
                    "…noughts and crosses drifting over the glass…",
                ],
            ),
            (
                DreamPicture::Pumpkin,
                &[
                    "…a pumpkin so big I can hide inside its grin…",
                    "…all the pumpkins shout boo, and I laugh…",
                ],
            ),
            (
                DreamPicture::Spiral,
                &[
                    "…the pictures you taught me climb off the glass…",
                    "…we draw a whole world, one finger each…",
                    "…your finger draws a door in the mist…",
                    "…drawing pictures in the clouds…",
                ],
            ),
        ];
        let mut world = World::new(1, 23.0);
        world.tuck_in();
        let mut seen = std::collections::BTreeSet::new();
        let mut check = |world: &mut World| {
            for line in dreams(world) {
                let picture = expected
                    .iter()
                    .find_map(|(picture, lines)| lines.contains(&line).then_some(*picture))
                    .expect(line);
                assert_eq!(DreamPicture::from_line(line, None), picture, "{line}");
                let named = named_friend_line(world, line);
                world.dream_of(&named);
                assert_eq!(world.dream.unwrap().picture, picture, "{named}");
                seen.insert(line);
            }
        };
        check(&mut world);
        world.life.friend = Some(Companion {
            name: "Lumi".into(),
            ..Companion::default()
        });
        world.life.skills.stacking = 1.0;
        world.life.skills.catching = 1.0;
        world.life.skills.fetching = 1.0;
        world.life.skills.games = 1.0;
        world.life.skills.writing = 1.0;
        world.life.skills.dancing = 1.0;
        world.life.pictures.push(LearnedPicture {
            name: "hill".into(),
            strokes: vec![vec![Vec2::ZERO, Vec2::ONE]],
        });
        world.petals.push(Petal {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            spin: 0.0,
            dropped: 0.0,
        });
        let today = &mut world.life.today;
        today.drawings_seen = 1;
        today.friend_visits = 1;
        today.moon_wishes = 1;
        today.motes = 1;
        today.birds = 1;
        today.gifts = 1;
        today.expected = 1;
        today.missed = 1;
        today.nudges = 1;
        today.fetches = 1;
        today.games = 1;
        today.towers = 1;
        today.doodles = 1;
        today.finds = 1;
        today.dances = 1;
        today.pounces = 1;
        today.pets = 1;
        today.words.push("ball".into());
        today.planted = 1;
        today.sprouted = 1;
        today.flowered = 1;
        today.waterings = 1;
        today.snowfalls = 1;
        today.storms = 1;
        today.shooting_stars = 1;
        today.rainbows = 1;
        today.showers = 1;
        today.heard = 1;
        check(&mut world);
        world.life.today.rainbows = 0;
        check(&mut world);
        for (date, birthday) in [
            (Date::constant(2026, 10, 31), None),
            (Date::constant(2026, 12, 24), None),
            (Date::constant(2026, 12, 31), None),
            (Date::constant(2027, 2, 14), None),
            (Date::constant(2027, 3, 1), Some(Date::constant(2026, 3, 1))),
            (Date::constant(2027, 2, 7), None),
            (Date::constant(2026, 9, 25), None),
            (Date::constant(2026, 10, 9), None),
        ] {
            world.date = date;
            world.birthday = birthday;
            check(&mut world);
        }
        assert_eq!(
            seen.len(),
            expected.iter().map(|(_, lines)| lines.len()).sum::<usize>()
        );
    }

    #[test]
    fn company_at_bedtime_brings_a_few_reluctant_goodnights() {
        let mut world = World::new(1, 23.0);
        assert!(!idle(&world).contains(&"Just a little longer."));
        world.notice_viewer();
        assert!(idle(&world).contains(&"I should be asleep…"));
        assert!(idle(&world).contains(&"Just a little longer."));
        world.hour = 12.0;
        assert!(!idle(&world).contains(&"Just a little longer."));
    }

    #[test]
    fn morning_thoughts_greet_the_day_and_recall_the_last_dream() {
        let mut world = World::new(1, 6.49);
        world.tuck_in();
        world.think("…the ball rolls on and on, out into the hills…");
        world.hour = 6.5;
        world.step(0.1);
        for lines in [idle(&world), setting_out(&world, Activity::Window)] {
            assert!(lines.contains(&"Good morning!"));
            assert!(lines.contains(&"The sun's up."));
            assert!(lines.contains(&"What a night."));
            assert!(lines.contains(&"That ball rolled all the way to the hills."));
        }
        world.time += 600.0;
        assert!(!idle(&world).contains(&"Good morning!"));
        assert!(!setting_out(&world, Activity::Window).contains(&"Good morning!"));
    }

    #[test]
    fn storms_bring_worries_reassurance_from_a_friend_and_dreams_afterward() {
        let mut world = World::new(1, 12.0);
        world.weather.sky = Sky::Rain;
        world.weather.storm = true;
        assert!(setting_out(&world, Activity::Window).contains(&"Whoa, look at that!"));
        assert!(idle(&world).contains(&"I don't like thunder."));
        assert!(idle(&world).contains(&"Is it over yet?"));
        let reassured = "I'm glad you're here when it thunders.";
        assert!(!idle(&world).contains(&reassured));
        world.notice_viewer();
        assert!(!idle(&world).contains(&reassured));
        world.life.bond = 0.3;
        assert!(idle(&world).contains(&reassured));
        world.viewer.active_at = 0.0;
        assert!(!idle(&world).contains(&reassured));
        world.life.note(&EventKind::StormStarted);
        world.weather.storm = false;
        assert!(dreams(&world).contains(&"…thunder far away, like a drum…"));
        assert!(!idle(&world).contains(&"I don't like thunder."));
    }

    #[test]
    fn every_occasion_brings_its_own_thoughts_and_dreams() {
        use jiff::civil::Date;

        let mut world = World::new(1, 22.0);
        let ordinary_thoughts = idle(&world);
        let ordinary_dreams = dreams(&world);
        for (month, day, birthday) in [
            (10, 31, None),
            (12, 24, None),
            (12, 31, None),
            (2, 14, None),
            (3, 1, Some(Date::constant(2026, 3, 1))),
        ] {
            world.date = Date::new(2027, month, day).unwrap();
            world.birthday = birthday;
            assert!(
                idle(&world)
                    .iter()
                    .any(|line| !ordinary_thoughts.contains(line))
            );
            assert!(
                dreams(&world)
                    .iter()
                    .any(|line| !ordinary_dreams.contains(line))
            );
        }
    }

    #[test]
    fn questions_are_answered_from_what_the_ghost_knows_of_itself() {
        let mut world = World::new(1, 12.0);
        world.notice_viewer();
        world.ghost.name = "Wisp".to_owned();
        let asked = |world: &mut World, text: &str| {
            let heard = world.hear(text);
            reply(world, &heard, None)
        };
        assert!(
            asked(&mut world, "what's your name?")
                .iter()
                .all(|line| line.contains("Wisp"))
        );

        world.begin(Activity::Window);
        let lines = asked(&mut world, "what are you doing?");
        assert!(
            lines
                .iter()
                .all(|line| line.starts_with("Gazing out of the window"))
        );

        world.ghost.needs.energy = 0.1;
        assert!(asked(&mut world, "are you tired?").contains(&"Very. I need a nap.".to_owned()));
        world.ghost.needs.energy = 0.9;
        assert!(asked(&mut world, "are you tired?").contains(&"Not at all!".to_owned()));

        world.life.bond = 0.9;
        assert!(asked(&mut world, "do you love me?").contains(&"I love you!".to_owned()));
        assert!(
            asked(&mut world, "hi, how are you?")
                .iter()
                .all(|line| line.starts_with("Hi! "))
        );

        assert_eq!(
            asked(&mut world, "how old are you?"),
            ["I'm not sure. Young, I think.", "Old enough to float."]
        );
        world.birthday = Some(jiff::civil::Date::constant(2026, 6, 18));
        assert_eq!(asked(&mut world, "how old are you?"), ["Three days old."]);
        assert_eq!(asked(&mut world, "몇 살이야?"), ["3일 됐어."]);
        world.birthday = Some(world.date);
        assert_eq!(
            asked(&mut world, "how old are you"),
            ["I came into being today!"]
        );
        world.birthday = Some(jiff::civil::Date::constant(2025, 6, 1));
        assert_eq!(asked(&mut world, "how old are you"), ["One year old."]);
    }

    #[test]
    fn a_sleeping_ghost_asked_what_it_is_doing_says_it_was_asleep() {
        let mut world = World::new(1, 12.0);
        world.ghost.needs.energy = 0.2;
        world.begin(Activity::Rest);
        for _ in 0..300 {
            world.step(0.1);
        }
        assert!(world.ghost.asleep > 0.9);
        let heard = world.hear("뭐해?");
        assert_eq!(
            reply(&world, &heard, None),
            ["자고 있었어!", "자는 중이었어… 방금까지."]
        );
    }

    #[test]
    fn questions_in_korean_get_korean_answers_with_fitting_particles() {
        let mut world = World::new(1, 12.0);
        world.notice_viewer();
        world.begin(Activity::Build);
        let heard = world.hear("뭐해?");
        assert_eq!(
            reply(&world, &heard, None),
            ["블록으로 탑 쌓는 중이야.", "블록으로 탑 쌓는 중이야. 볼래?"]
        );
        let heard = world.hear("안녕, 기분 어때?");
        assert!(
            reply(&world, &heard, None)
                .iter()
                .all(|line| line.starts_with("안녕! "))
        );
        assert!(final_consonant("숨바꼭질"));
        assert!(!final_consonant("공놀이"));
        assert!(!final_consonant("Boo"));
        for activity in Activity::ALL {
            assert!(korean_doing(activity).ends_with('중'));
            assert!(!korean_favorite(activity).is_empty());
        }
    }

    #[test]
    fn the_shelf_and_the_bird_are_answered_from_the_ghosts_life() {
        use crate::world::Keepsake;

        let mut world = World::new(1, 12.0);
        world.notice_viewer();
        let asked = |world: &mut World, text: &str| {
            let heard = world.hear(text);
            reply(world, &heard, None)
        };
        assert!(
            asked(&mut world, "what's on your shelf?").contains(&"It's empty… for now.".to_owned())
        );
        world.life.keepsakes = vec![
            Keepsake::Feather,
            Keepsake::Acorn,
            Keepsake::Feather,
            Keepsake::Feather,
        ];
        assert!(
            asked(&mut world, "what's on your shelf?")
                .contains(&"Three feathers and an acorn. Gifts from the bird!".to_owned())
        );
        assert!(
            asked(&mut world, "선반에 뭐 있어?")
                .contains(&"내 보물: 깃털 3개, 도토리 1개.".to_owned())
        );
        assert!(
            asked(&mut world, "do you like the bird?")
                .contains(&"It's still shy of me.".to_owned())
        );
        world.life.bird_trust = 0.8;
        assert!(asked(&mut world, "저 새 좋아해?").contains(&"새는 내 친구야!".to_owned()));
    }

    #[test]
    fn the_ghost_answers_with_the_viewers_name_once_it_has_learned_it() {
        let mut world = World::new(1, 12.0);
        world.notice_viewer();
        let heard = world.hear("what's my name?");
        assert_eq!(
            reply(&world, &heard, None),
            ["You haven't told me yet.", "I don't know… tell me?"]
        );
        world.hear("my name is Mina");
        let heard = world.hear("do you know my name?");
        assert_eq!(
            reply(&world, &heard, None),
            ["You're Mina!", "Of course I know, Mina."]
        );
        let heard = world.hear("내 이름 알아?");
        assert_eq!(
            reply(&world, &heard, None),
            ["Mina! 맞지?", "당연히 알지, Mina."]
        );
    }

    #[test]
    fn replies_name_the_word_the_ghost_understood_or_wonders_about() {
        let mut world = World::new(1, 12.0);
        world.notice_viewer();
        let heard = world.hear("dance");
        let lines = reply(&world, &heard, None);
        assert!(!lines.is_empty());
        assert!(
            lines
                .iter()
                .all(|line| line.contains("dance") || line.contains("Dance"))
        );

        world
            .life
            .vocabulary
            .teach(&["spin".to_owned()], Activity::Dance, 2.0);
        let heard = world.hear("spin!");
        assert!(
            reply(&world, &heard, None)
                .iter()
                .all(|line| line.starts_with("Spin"))
        );

        let heard = world.hear("hmm hmm");
        let guessed = world.guess();
        assert!(guessed.is_some());
        assert!(reply(&world, &heard, guessed).contains(&"Like this?".to_owned()));
    }

    #[test]
    fn tone_alone_gets_an_answer_and_little_words_get_none() {
        let mut world = World::new(1, 12.0);
        world.notice_viewer();
        for text in ["hello", "good job", "no", "bye", "Boo?"] {
            let heard = world.hear(text);
            assert!(!reply(&world, &heard, None).is_empty(), "{text}");
        }
        let heard = world.hear("is it");
        assert!(reply(&world, &heard, None).is_empty());
    }

    #[test]
    fn introductions_are_acknowledged_offline_and_greetings_sometimes_use_the_name() {
        let mut world = World::new(1, 12.0);
        let heard = world.hear("my name is Mina");
        let lines = reply(&world, &heard, None);
        assert!(lines.contains(&"Nice to meet you, Mina!".to_owned()));
        assert!(lines.iter().all(|line| line.contains("Mina")));
        let heard = world.hear("hi");
        let lines = reply(&world, &heard, None);
        assert!(lines.contains(&"Hi, Mina!".to_owned()));
        assert!(lines.iter().any(|line| !line.contains("Mina")));
        let heard = world.hear("제 이름은 민수예요.");
        assert!(
            reply(&world, &heard, None)
                .iter()
                .all(|line| line.contains("민수"))
        );
        assert!(reply(&world, &heard, None).contains(&"반가워, 민수!".to_owned()));
    }
    #[test]
    fn offline_answers_and_thoughts_use_the_friends_remembered_name() {
        let mut world = World::new(1, 12.0);
        world.life.friend = Some(crate::world::Companion {
            name: "Lumi".to_owned(),
            ..Default::default()
        });
        for text in ["Who is your friend?", "친구 이름은 뭐야?"] {
            let heard = world.hear(text);
            assert!(
                reply(&world, &heard, None)
                    .iter()
                    .all(|answer| answer.contains("Lumi"))
            );
        }
        assert_eq!(
            named_friend_line(&world, "My ghost friend is fun!"),
            "Lumi is fun!"
        );
        world.life.today.friend_visits = 1;
        assert!(
            dreams(&world)
                .iter()
                .any(|line| named_friend_line(&world, line).contains("Lumi"))
        );
    }

    #[test]
    fn questions_window_thoughts_and_dreams_recall_named_constellations() {
        let mut world = World::new(1, 21.0);
        let heard = world.hear("do you know any constellations?");
        assert!(reply(&world, &heard, None)[0].contains("haven't found"));
        world
            .life
            .constellations
            .push(crate::world::stars::Constellation {
                name: "the Little Heart".to_owned(),
                korean: "작은 하트".to_owned(),
                ..Default::default()
            });
        for (text, expected) in [
            ("do you know any constellations?", "the Little Heart"),
            ("별자리 알아?", "작은 하트"),
        ] {
            let heard = world.hear(text);
            assert!(
                reply(&world, &heard, None)
                    .iter()
                    .all(|line| line.contains(expected))
            );
            assert!(matches!(
                heard.understanding,
                Understanding::Asked {
                    question: Question::Constellations,
                    ..
                }
            ));
        }
        world.weather.sky = Sky::Clear;
        world.weather.storm = false;
        world.begin(Activity::Window);
        for lines in [idle(&world), dreams(&world)] {
            assert!(
                lines
                    .iter()
                    .any(|line| named_friend_line(&world, line).contains("the Little Heart"))
            );
        }
        world.hour = 12.0;
        assert!(!idle(&world).contains(&"I can see my constellation again."));
    }
}
