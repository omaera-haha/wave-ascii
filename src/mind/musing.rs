use crate::world::{Activity, Heard, Mood, Occasion, Question, Sky, Tone, Understanding, World};

// Thoughts the ghost has on its own while it has no System Two to think
// with: a few words that fit the moment, so a ghost without keys still has
// an inner life. Each function gives every line that fits; the caller picks
// one.

/// What crosses its mind as it sets out to do `activity`.
pub fn setting_out(world: &World, activity: Activity) -> Vec<&'static str> {
    let present = world.viewer_present();
    match activity {
        Activity::Float => Vec::new(),
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
        Activity::Tend if world.plant.thirst > 0.7 => {
            vec![
                "Oh no, the plant is drooping.",
                "Hold on, plant, I'm coming!",
            ]
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
    }
}

/// What it muses about while nothing in particular happens.
pub fn idle(world: &World) -> Vec<&'static str> {
    let ghost = &world.ghost;
    let needs = ghost.needs;
    let present = world.viewer_present();
    let mut lines = Vec::new();
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
            Activity::Float => "Floating is nice.",
            Activity::Build => "How high could I stack those blocks?",
            Activity::Fetch => "Throw the ball again! Please?",
            Activity::Pounce => "Move your finger again!",
            Activity::Dance => "I've been working on a new twirl.",
            Activity::Hide => "Nobody hides like a ghost.",
            Activity::Game => "Another game on the glass?",
            Activity::Doodle => "I should draw on the glass again.",
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
        None => {}
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
    if today.waterings > 0 {
        lines.push("…the plant grows right up through the ceiling…");
    }
    if today.snowfalls > 0 {
        lines.push("…the snow falls inside the glass, soft as light…");
    }
    if today.storms > 0 {
        lines.push("…thunder far away, like a drum…");
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
    let lines = |items: &[&str]| items.iter().map(|line| (*line).to_owned()).collect();
    if let Some(name) = &heard.learned_name {
        return if heard.korean {
            vec![format!("반가워, {name}!"), format!("{name}! 기억할게.")]
        } else {
            vec![
                format!("Nice to meet you, {name}!"),
                format!("{name}! I'll remember."),
            ]
        };
    }
    if let Some((word, _)) = heard.learned.last() {
        return vec![
            format!("So “{word}” means this!"),
            format!("Oh! “{word}”! I get it now."),
        ];
    }
    if guessed.is_some()
        && let Understanding::Puzzled { word, .. } = &heard.understanding
    {
        return vec![
            "Like this?".to_owned(),
            format!("Is this “{word}”?"),
            format!("{}… this?", capitalized(word)),
        ];
    }
    match &heard.understanding {
        Understanding::Doing { word, .. } => {
            let word = capitalized(word);
            return vec![
                format!("{word}? Okay!"),
                format!("{word}! I know that one."),
                format!("{word}! Watch me."),
            ];
        }
        Understanding::Unable { activity, .. } => return unable(world, *activity),
        Understanding::Asked { question, doing } => {
            let answers = answer(world, *question, *doing, heard.korean);
            let hello = if heard.korean { "안녕! " } else { "Hi! " };
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
        Some(Tone::Scolding) if heard.stopped => return lines(&["Oh… okay, I'll stop.", "Sorry…"]),
        Some(Tone::Scolding) => return lines(&["Oh… sorry.", "Okay, okay…"]),
        Some(Tone::Praise) if bond < 0.12 => return lines(&["Oh! Thank you.", "Hehe."]),
        Some(Tone::Praise) => return lines(&["Hehe!", "Yay!", "You think so?"]),
        _ => {}
    }
    if let Understanding::Puzzled { word, again } = &heard.understanding {
        return if *again {
            vec![
                format!("“{word}” again… what does it mean?"),
                format!("Hm, “{word}”… show me?"),
            ]
        } else {
            vec![
                format!("“{word}”?"),
                format!("What's “{word}”?"),
                format!("Hm? {}?", capitalized(word)),
            ]
        };
    }
    if heard.tone == Some(Tone::Greeting)
        && let Some(name) = &world.life.viewer_name
    {
        return vec![format!("Hi, {name}!"), "Hello! You're here!".to_owned()];
    }
    match heard.tone {
        Some(Tone::Farewell) if world.is_night() => lines(&["Goodnight!", "Sleep well."]),
        Some(Tone::Farewell) => lines(&["Bye… come back soon.", "See you!"]),
        Some(Tone::Greeting) if bond < 0.12 => lines(&["Oh! Hello…", "Hi there."]),
        Some(Tone::Greeting) if bond < 0.55 => lines(&["Hello!", "Hi! It's you."]),
        Some(Tone::Greeting) => lines(&["Hi! You're here!", "Hello, you!"]),
        _ if heard.named => lines(&["Yes? That's me!", "You called?"]),
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
    }
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
fn unable(world: &World, activity: Activity) -> Vec<String> {
    let line = match activity {
        Activity::Rest => "I'm not sleepy!",
        Activity::Chase if world.motes.is_empty() => "I don't see any light motes.",
        Activity::Chase => "I don't know where a light mote is.",
        Activity::Tend => "The plant doesn't need water yet.",
        Activity::Hide if !world.viewer_present() => "Nobody's here to find me.",
        _ if world.ghost.needs.energy < 0.4 => "I'm too tired for that.",
        _ => "Not right now.",
    };
    vec![line.to_owned()]
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
    use crate::world::EventKind;

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
}
