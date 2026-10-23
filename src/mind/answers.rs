use crate::world::{Activity, Mood, Question, Sky, World};

// Plain answers to the viewer's questions, from what the ghost knows of
// itself, for the draft of a letter that Claude then writes properly.

/// Spoken once on returning, or again when the viewer asks about the absence.
pub fn returned(world: &World, korean: bool) -> Vec<String> {
    let Some(happening) = world.away.as_ref().and_then(|away| away.notable()) else {
        return Vec::new();
    };

    happening
        .lines(korean)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

// What the ghost says to `question`, from what it knows of itself: its
// mood, needs, plan, name, age, habits, the viewer's name, and the weather,
// in Korean when it was asked in Korean. `doing` is what it was doing when
// asked, or `None` if the question woke it.
pub(crate) fn answer(
    world: &World,
    question: Question,
    doing: Option<Activity>,
    korean: bool,
) -> Vec<String> {
    let lines = |english: &[&str], hangul: &[&str]| {
        let items = if korean { hangul } else { english };
        items.iter().map(|line| (*line).to_owned()).collect()
    };
    let ghost = &world.ghost;
    let needs = ghost.needs;
    let bond = world.life.bond;
    match question {
        Question::Mirror if world.life.mirror.recognizes_self() => {
            lines(&["That's me!"], &["그건 나야!"])
        }
        Question::Mirror => lines(
            &["Someone just like me! It copies me."],
            &["나랑 똑같이 생겼어! 나를 따라 해."],
        ),
        Question::Wearing => match world.wearing {
            Some(accessory) if korean => vec![format!("{}! 마음에 들어.", accessory.korean())],
            Some(accessory) => vec![format!("I’m wearing {}!", accessory.words())],
            None => lines(
                &["Just my usual glow today."],
                &["오늘은 아무것도 안 걸쳤어."],
            ),
        },
        Question::ReadAloud => Vec::new(),
        Question::OwnBook => world
            .ghost
            .speech
            .as_ref()
            .map(|line| vec![line.text.clone()])
            .unwrap_or_default(),
        Question::Reading => match world.current_book() {
            Some(index) => {
                if index >= crate::world::books::LIBRARY.len() {
                    return vec![if korean {
                        format!("《{}》, 내가 쓴 책이야.", world.book_title(index, true))
                    } else {
                        format!("{}. I wrote it myself.", world.book_title(index, false))
                    }];
                }
                let book = crate::world::books::LIBRARY[index];
                vec![if korean {
                    format!("《{}》, {}.", book.title(true), book.about(true))
                } else {
                    format!("{}. It's about {}.", book.title(false), book.about(false))
                }]
            }
            None => lines(&["My book pile is waiting."], &["책들이 기다리고 있어."]),
        },
        Question::FavoriteBook => match world.favorite_book() {
            Some(index) => {
                let title = world.book_title(index, korean);
                vec![if korean {
                    format!("《{}》! 자꾸 다시 읽게 돼.", title)
                } else {
                    format!("{}! I keep coming back to it.", title)
                }]
            }
            None => lines(
                &["I'm still finding my favorite."],
                &["아직 제일 좋은 책을 찾는 중이야."],
            ),
        },
        Question::Music => lines(
            &["Yes! I like swaying with my little music box."],
            &["응! 오르골에 맞춰 살랑이는 게 좋아."],
        ),
        Question::Songs => {
            let learned = world.learned_tune_words();
            let own = world.composition_words(korean);
            let own = if own.is_empty() {
                String::new()
            } else if korean {
                format!(" 내가 만든 노래도 있어: {own}.")
            } else {
                format!(" I made my own songs too: {own}.")
            };
            if korean {
                let extra = if world.life.tunes.is_empty() {
                    "아직 네게 배운 곡은 없어.".to_owned()
                } else {
                    format!("네가 가르쳐 준 곡도 알아: {learned}.")
                };
                vec![format!(
                    "작은 자장가, 빙글 왈츠, 꼬마 행진곡, 빛 알갱이 반짝임을 알아. {extra}{own}"
                )]
            } else {
                let extra = if world.life.tunes.is_empty() {
                    "You can teach me a tune, too.".to_owned()
                } else {
                    format!("And the tunes you taught me: {learned}.")
                };
                vec![format!(
                    "I know little lullaby, turning waltz, tiny march, and mote sparkle. {extra}{own}"
                )]
            }
        }
        Question::Wish => {
            let mut words = match (&world.life.wish, korean) {
                (Some(wishing), true) => {
                    format!("내 소원은 {}이야.", wishing.wish.korean(&world.life))
                }
                (Some(wishing), false) => format!("I wish {}.", wishing.wish.words(&world.life)),
                (None, true) => "지금은 없어. 행복해.".to_owned(),
                (None, false) => "Nothing right now. I'm happy.".to_owned(),
            };
            if let Some(granted) = world.life.granted.last() {
                words.push_str(&if korean {
                    format!(
                        " 지난번 소원은 {}이었어. 이루어졌어!",
                        granted.wish.korean(&world.life)
                    )
                } else {
                    format!(
                        " My last wish came true: {}.",
                        granted.wish.words(&world.life)
                    )
                });
            }
            vec![words]
        }
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
                &["I'm good, thanks.", "Content. Just being me."],
                &["좋아. 그냥 여기 있어.", "괜찮아, 고마워."],
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
        Question::Viewer(topic) => vec![world.life.viewer.reply(topic, korean)],
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
            for gift in world.life.shelved_keepsakes() {
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
        Question::Cranes => vec![if korean {
            if world.life.cranes.total == 0 {
                "아직 한 마리도 없어. 천천히 너를 위해 접을 거야.".to_owned()
            } else {
                format!(
                    "지금까지 종이학 {} 마리를 접었어. 모두 너를 위한 거야.",
                    crate::world::cranes::korean_count(world.life.cranes.total)
                )
            }
        } else {
            format!(
                "I've folded {} paper cranes. Every one is for you.",
                crate::world::friend::count_words(world.life.cranes.total)
            )
        }],
        Question::Gallery => vec![world.life.gallery_answer(korean)],
        Question::Yesterday => vec![world.life.recall_yesterday(world.date, korean)],
        Question::Today => vec![world.life.today.recall(korean)],
        Question::Time => vec![crate::world::writing::time_answer(world, korean)],
        Question::Date => vec![crate::world::writing::date_answer(world, korean)],
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
        Activity::Friend => "유령 친구와 노는 중",
        Activity::Look => "네가 그린 그림을 보는 중",
        Activity::Air => "창문을 여닫는 중",
        Activity::Comfort => "네 마음을 보살피는 중",
        Activity::Company => "네가 일하는 동안 옆에 있어 주는 중",
        Activity::Puzzle => "유리에 낱말 맞추기 하는 중",
        Activity::Read => "그림책 읽는 중",
        Activity::Music => "오르골 음악을 듣는 중",
        Activity::Letter => "네 편지를 읽는 중",
        Activity::Mirror => "거울 보는 중",
        Activity::Seek => "네 보물 찾는 중",
        Activity::Fold => "종이학 접는 중",
        Activity::Bubbles => "비눗방울 부는 중",
        Activity::Pin => "벽에 그림 붙이는 중",
        Activity::Show => "보물을 보여주는 중",
        Activity::Tidy => "방 정리하는 중",
        Activity::Yard => "마당에서 노는 중",
        Activity::Together => "친구랑 노는 중",
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
        Activity::Air => "신선한 바람 쐬기",
        Activity::Comfort => "네 곁에서 마음 보살피기",
        Activity::Company => "일하는 네 옆에 있기",
        Activity::Puzzle => "낱말 맞추기",
        Activity::Read => "그림책 읽기",
        Activity::Music => "오르골 듣기",
        Activity::Letter => "네 편지 읽기",
        Activity::Mirror => "거울 보기",
        Activity::Seek => "네 보물 찾기",
        Activity::Fold => "종이학 접기",
        Activity::Bubbles => "비눗방울 불기",
        Activity::Pin => "벽에 그림 붙이기",
        Activity::Show => "보물 보여주기",
        Activity::Tidy => "방 정리하기",
        Activity::Yard => "마당에서 놀기",
        Activity::Together => "친구들이랑 놀기",
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

fn capitalized(text: &str) -> String {
    let mut characters = text.chars();
    characters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(characters).collect()
    })
}
