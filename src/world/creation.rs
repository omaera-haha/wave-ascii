//! Saved creations, with paired prose chosen once for both languages.
use jiff::civil::Date;
use serde::{Deserialize, Serialize};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::{EventKind, Life, Random, Season, Stage, World, books::LIBRARY};

type Page = (String, String);

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WrittenBook {
    pub title: Page,
    /// Only pages the ghost has actually written.
    pub pages: Vec<Page>,
    pub day: String,
    pub finished: bool,
    pub read: u32,
    /// The remaining paired story survives interruptions and restarts.
    pub outline: Vec<Page>,
    pub page: usize,
}

pub fn elapsed_days(day: Option<&str>, today: Date) -> Option<i32> {
    day.and_then(|day| day.parse::<Date>().ok())
        .map(|day| (today - day).get_days())
}

pub fn valid_text(text: &str, cells: usize) -> bool {
    !text.trim().is_empty()
        && (1..=cells).contains(&text.width())
        && text.chars().count() <= cells * 2
        && !text.chars().any(char::is_control)
}

fn valid_day(day: &str, today: &str) -> bool {
    day.parse::<Date>().is_ok_and(|date| {
        date.to_string() == day && today.parse::<Date>().ok().is_none_or(|today| date <= today)
    })
}

pub fn sanitize(life: &mut Life) {
    let mut compositions = Vec::new();
    for mut composition in std::mem::take(&mut life.compositions).into_iter().rev() {
        if !valid_day(&composition.day, &life.day)
            || !valid_text(&composition.title, 80)
            || !valid_text(&composition.korean_title, 80)
        {
            continue;
        }
        let Some(tune) = composition.tune.sanitized() else {
            continue;
        };
        composition.tune = tune;
        composition.tune.name = composition.title.clone();
        if compositions.len() < 8
            && !compositions
                .iter()
                .any(|known: &super::music::Composition| known.tune.notes == composition.tune.notes)
        {
            compositions.push(composition);
        }
    }
    compositions.reverse();
    life.compositions = compositions;
    life.composition_day = life
        .composition_day
        .take()
        .filter(|day| valid_day(day, &life.day))
        .into_iter()
        .chain(life.compositions.iter().map(|song| song.day.clone()))
        .max();
    life.written.retain_mut(|book| {
        let valid_pair = |pair: &Page| {
            [&pair.0, &pair.1]
                .into_iter()
                .all(|line| valid_text(line, 80) && !line.chars().any(char::is_numeric))
        };
        book.read = book.read.min(1_000_000);
        book.page = book.page.min(book.pages.len());
        valid_day(&book.day, &life.day)
            && valid_pair(&book.title)
            && book.pages.iter().chain(&book.outline).all(valid_pair)
            && if book.finished {
                book.pages.len() == 5 && book.outline.is_empty()
            } else {
                book.pages.len() < 5 && book.pages.len() + book.outline.len() == 5
            }
    });
    let mut draft_kept = false;
    life.written.reverse();
    life.written.retain(|book| {
        if book.finished {
            true
        } else if draft_kept {
            false
        } else {
            draft_kept = true;
            true
        }
    });
    life.written.reverse();
    if life.written.len() > 6 {
        life.written.drain(..life.written.len() - 6);
    }
    // A saved draft also establishes the cooldown if loading an earlier schema.
    life.written_day = life
        .written_day
        .take()
        .filter(|day| valid_day(day, &life.day))
        .into_iter()
        .chain(life.written.iter().map(|book| book.day.clone()))
        .max();
    life.last_friend_visit = life
        .last_friend_visit
        .take()
        .filter(|day| valid_day(day, &life.day));
    if let Ok(today) = life.day.parse::<Date>() {
        abandon_old_drafts(life, today);
    }
}

fn abandon_old_drafts(life: &mut Life, today: Date) {
    life.written.retain(|book| {
        book.finished
            || elapsed_days(Some(&book.day), today).is_some_and(|days| (0..7).contains(&days))
    });
}

fn pair(english: &str, korean: &str) -> Page {
    (english.to_owned(), korean.to_owned())
}

/// Keep inserted memory names short enough for either language's page.
fn name(text: &str) -> String {
    let mut cells = 0;
    text.chars()
        .filter(|character| !character.is_control() && !character.is_numeric())
        .take_while(|character| {
            cells += character.width().unwrap_or(0);
            cells <= 20
        })
        .collect()
}

/// Every random choice picks a paired thread, never a separate translation.
pub fn compose_book(random: &mut Random, life: &Life, season: Season, day: String) -> WrittenBook {
    let mut threads = vec![(
        pair("The Little Glass Room", "작은 유리 방"),
        [
            pair(
                "The little ghost woke inside a glass room.",
                "꼬마 유령이 유리 방 안에서 눈을 떴어.",
            ),
            pair(
                "Even a little room had space for a whole story.",
                "작은 방에도 이야기 하나가 들어갈 자리는 있었어.",
            ),
        ],
    )];
    if life.bird_trust > 0.0 || life.total.birds > 0 {
        threads.push((
            pair("The Little Ghost and the Bird", "꼬마 유령과 새"),
            [
                pair(
                    "A bird waited on the sill without a sound.",
                    "새가 창턱에서 소리 없이 기다렸어.",
                ),
                pair(
                    "The ghost sat quietly, and the bird stayed.",
                    "유령이 조용히 앉자 새도 곁에 머물렀어.",
                ),
            ],
        ));
    }
    if let Some(friend) = &life.friend {
        let friend = name(&friend.name);
        threads.push((
            (
                format!("{friend} Comes to Visit"),
                format!("{friend} 친구가 놀러 왔어요"),
            ),
            [
                (
                    format!("{friend} slipped through the window to visit."),
                    format!("{friend} 친구가 창문으로 놀러 왔어."),
                ),
                pair(
                    "They played until the moon peeked into the room.",
                    "달이 방 안을 들여다볼 때까지 함께 놀았어.",
                ),
            ],
        ));
    }
    if life.total.flowered > 0 || life.total.blooms > 0 {
        threads.push((
            pair("A Flower in the Little Room", "작은 방에 핀 꽃"),
            [
                pair(
                    "The ghost gave a thirsty flower a little water.",
                    "유령이 목마른 꽃에게 물을 주었어.",
                ),
                pair(
                    "A bright petal opened, as if to say thank you.",
                    "고맙다고 말하듯 밝은 꽃잎이 펼쳐졌어.",
                ),
            ],
        ));
    }
    if let Some(stars) = life.constellations.last() {
        threads.push((
            pair("A Picture in the Stars", "별들이 그린 그림"),
            [
                (
                    format!("The ghost found {} in the stars.", name(&stars.name)),
                    format!("유령은 별들 속에서 {} 그림을 찾았어.", name(&stars.korean)),
                ),
                pair(
                    "The sky kept the picture safe until morning.",
                    "하늘은 아침까지 그림을 소중히 간직했어.",
                ),
            ],
        ));
    }
    if life.total.shooting_stars > 0 {
        threads.push((
            pair("A Night of Shooting Stars", "별똥별이 내린 밤"),
            [
                pair(
                    "A shooting star stitched light across the dark.",
                    "별똥별이 어둠 위에 빛으로 수를 놓았어.",
                ),
                pair(
                    "The ghost tucked a little wish into its glow.",
                    "유령은 그 빛 속에 작은 소원을 넣었어.",
                ),
            ],
        ));
    }
    if life.mirror.sessions > 0 {
        threads.push((
            pair("A Friend in the Mirror", "거울 속 친구"),
            [
                pair(
                    "A ghost in the mirror copied a little wave.",
                    "거울 속 유령이 작은 손짓을 따라 했어.",
                ),
                if life.mirror.recognizes_self() {
                    pair(
                        "It was the little ghost all along!",
                        "알고 보니 바로 자기 자신이었어!",
                    )
                } else {
                    pair(
                        "They smiled together through the silver glass.",
                        "둘은 은빛 유리를 사이에 두고 함께 웃었어.",
                    )
                },
            ],
        ));
    }
    if !life.keepsakes.is_empty() {
        threads.push((
            pair("The Bird's Little Treasure", "새가 준 작은 보물"),
            [
                pair(
                    "The bird left a tiny treasure on the sill.",
                    "새가 창턱에 작은 보물을 두고 갔어.",
                ),
                pair(
                    "The ghost made a safe place for it on the shelf.",
                    "유령은 선반에 보물을 위한 자리를 만들었어.",
                ),
            ],
        ));
    }
    if let Some(song) = life.compositions.last() {
        threads.push((
            pair("A Song of My Own", "내가 만든 노래"),
            [
                (
                    format!("The ghost hummed {}.", name(&song.title)),
                    format!("유령은 {} 노래를 흥얼거렸어.", name(&song.korean_title)),
                ),
                pair(
                    "The music box carried its song around the room.",
                    "오르골이 그 노래를 방 안 가득 퍼뜨렸어.",
                ),
            ],
        ));
    } else if !life.tunes.is_empty() {
        threads.push((
            pair("The Tune You Taught Me", "네가 가르쳐 준 노래"),
            [
                pair(
                    "The ghost remembered the tune you taught it.",
                    "유령은 네가 가르쳐 준 노래를 기억했어.",
                ),
                pair(
                    "It wound the little box and played it for you.",
                    "작은 오르골을 감아 너에게 들려주었어.",
                ),
            ],
        ));
    }
    let chosen = random.range(0.0, threads.len() as f32) as usize;
    let (title, beats) = threads.swap_remove(chosen);
    let opening = match season {
        Season::Spring => pair(
            "Spring petals drifted past a little glass room.",
            "작은 유리 방 밖으로 봄꽃잎이 흘러갔어.",
        ),
        Season::Summer => pair(
            "Summer light warmed a little glass room.",
            "여름 햇살이 작은 유리 방을 데웠어.",
        ),
        Season::Autumn => pair(
            "Autumn leaves danced past a little glass room.",
            "작은 유리 방 밖으로 가을 잎이 춤추었어.",
        ),
        Season::Winter => pair(
            "Winter snow hushed a little glass room.",
            "겨울눈이 작은 유리 방을 고요하게 감쌌어.",
        ),
    };
    let companion = match (&life.friend, &life.viewer_name) {
        (Some(friend), Some(viewer)) => (
            format!(
                "{} and {} listened to the little ghost.",
                name(&friend.name),
                name(viewer)
            ),
            format!(
                "{} 친구와 {} 모두 유령에게 귀 기울였어.",
                name(&friend.name),
                name(viewer)
            ),
        ),
        (Some(friend), None) => (
            format!("{} came to listen to the little ghost.", name(&friend.name)),
            format!("{} 친구가 유령의 이야기를 들으러 왔어.", name(&friend.name)),
        ),
        (None, Some(viewer)) => (
            format!("{} came to listen to the little ghost.", name(viewer)),
            format!("{} 친구가 유령의 이야기를 들으러 왔어.", name(viewer)),
        ),
        (None, None) => pair(
            "The little ghost had a story to share.",
            "꼬마 유령에게 나누고 싶은 이야기가 있었어.",
        ),
    };
    WrittenBook {
        title,
        pages: Vec::new(),
        day,
        finished: false,
        read: 0,
        outline: vec![
            opening,
            companion,
            beats[0].clone(),
            beats[1].clone(),
            pair(
                "The ghost kept that day inside a little book.",
                "유령은 그날을 작은 책 속에 간직했어.",
            ),
        ],
        page: 0,
    }
}

impl World {
    pub(super) fn expire_drafts(&mut self) {
        let old_length = self.life.written.len();
        abandon_old_drafts(&mut self.life, self.date);
        if old_length != self.life.written.len() {
            self.last_read = None;
            if self
                .reading
                .as_ref()
                .is_some_and(|reading| reading.index >= LIBRARY.len() + self.life.written.len())
            {
                self.reading = None;
            }
        }
    }

    pub fn can_write_book(&self) -> bool {
        self.stage() != Stage::Newborn
            && !self.life.books.owned.is_empty()
            && self.life.books.owned.iter().all(|owned| owned.finished > 0)
            && self.life.milestones.len() >= 8
            && self.life.written.len() < 6
            && elapsed_days(self.life.written_day.as_deref(), self.date)
                .is_none_or(|days| days >= 10)
            && self.hour >= 18.0
            && !self.bedtime()
            && self.focus.is_none()
            && self.ghost.asleep < 0.3
            && self.telling.is_none()
    }

    pub(super) fn writing_choice(&mut self) -> Option<usize> {
        if self.stage() == Stage::Newborn
            || self.focus.is_some()
            || self.hour < 18.0
            || self.bedtime()
            || self.ghost.asleep >= 0.3
            || self.telling.is_some()
        {
            return None;
        }
        if let Some(index) = self.life.written.iter().position(|book| !book.finished) {
            return Some(LIBRARY.len() + index);
        }
        if !self.can_write_book() || self.random.unit() >= 1.0 / 3.0 {
            return None;
        }
        let season = self.season();
        let book = compose_book(&mut self.random, &self.life, season, self.date.to_string());
        self.life.written_day = Some(self.date.to_string());
        self.life.written.push(book);
        Some(LIBRARY.len() + self.life.written.len() - 1)
    }

    pub(super) fn write_page(&mut self, index: usize) -> bool {
        let book = &mut self.life.written[index - LIBRARY.len()];
        if book.finished || book.outline.is_empty() {
            return true;
        }
        book.pages.push(book.outline.remove(0));
        let finished = book.outline.is_empty();
        book.finished = finished;
        if finished {
            book.day = self.date.to_string();
        }
        let title = book.title.0.clone();
        let count = super::friend::count_words(book.pages.len() as u32);
        self.think(&if finished {
            "There. My own little book.".to_owned()
        } else {
            format!("There. Page {count}. What happens next…")
        });
        if finished {
            self.life.written_day = Some(self.date.to_string());
            self.record(EventKind::WroteBook(title));
            self.suggest_mood(super::Mood::Excited, 12.0);
            if self.viewer_present() {
                self.say(self.spoken(
                    "I wrote a book! Want me to read it to you?",
                    "책을 썼어! 읽어줄까?",
                ));
            }
        }
        finished
    }

    pub fn written_words(&self, korean: bool) -> String {
        self.life
            .written
            .iter()
            .map(|book| {
                format!(
                    "{}{}",
                    if korean { &book.title.1 } else { &book.title.0 },
                    if book.finished {
                        ""
                    } else {
                        " (unfinished draft)"
                    }
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn books_words(&self) -> String {
        let mut words = self.life.books.words();
        if !self.life.written.is_empty() {
            words.push_str(&format!("; wrote {}", self.written_words(false)));
        }
        words
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{
        Activity, Companion, Entry, Mood, Nature, Question, Understanding,
        music::{Composition, Note, Tune},
    };

    fn memories() -> Life {
        Life {
            day: "2026-06-21".to_owned(),
            viewer_name: Some("Mina".to_owned()),
            friend: Some(Companion {
                name: "Pell".to_owned(),
                visits: 1,
                ..Default::default()
            }),
            bird_trust: 0.7,
            keepsakes: vec![super::super::Keepsake::Feather],
            ..Default::default()
        }
    }

    fn finished_book() -> WrittenBook {
        let mut book = compose_book(
            &mut Random::new(7),
            &memories(),
            Season::Summer,
            "2026-06-21".to_owned(),
        );
        book.pages = std::mem::take(&mut book.outline);
        book.finished = true;
        book
    }

    fn eligible_writer(seed: u64) -> World {
        let mut world = World::new(seed, 19.0);
        world.step(0.0);
        world.lamp.on = true;
        for owned in &mut world.life.books.owned {
            owned.finished = 1;
            owned.page = LIBRARY[owned.index].pages.len();
        }
        for text in [
            "A bird came.",
            "A flower grew.",
            "A friend visited.",
            "A star fell.",
            "I learned a song.",
            "I saw the moon.",
            "We played.",
            "I read a book.",
        ] {
            world.life.milestones.push(Entry {
                day: world.date.to_string(),
                text: text.to_owned(),
            });
        }
        world
    }

    fn writing_sitting() -> World {
        (0..100)
            .find_map(|seed| {
                let mut world = eligible_writer(seed);
                world.begin(Activity::Read);
                world.reading.as_ref().unwrap().writing.then_some(world)
            })
            .unwrap()
    }

    #[test]
    fn paired_books_keep_five_short_pages_and_the_same_names_in_both_languages() {
        for season in [
            Season::Spring,
            Season::Summer,
            Season::Autumn,
            Season::Winter,
        ] {
            for seed in 0..64 {
                let life = memories();
                let book = compose_book(&mut Random::new(seed), &life, season, life.day.clone());
                assert_eq!(
                    book,
                    compose_book(&mut Random::new(seed), &life, season, life.day.clone())
                );
                assert!(book.pages.is_empty());
                assert_eq!(book.outline.len(), 5);
                for pair in std::iter::once(&book.title).chain(&book.outline) {
                    for text in [&pair.0, &pair.1] {
                        assert!(valid_text(text, 80), "{text}");
                        assert!(!text.chars().any(char::is_numeric), "{text}");
                    }
                }
                for name in ["Pell", "Mina"] {
                    assert!(book.outline.iter().any(|page| page.0.contains(name)));
                    assert!(book.outline.iter().any(|page| page.1.contains(name)));
                }
            }
        }
    }

    #[test]
    fn memory_threads_only_recount_life_details_the_ghost_has() {
        let mut life = Life::default();
        let plain: Vec<_> = (0..64)
            .map(|seed| {
                compose_book(
                    &mut Random::new(seed),
                    &life,
                    Season::Summer,
                    "2026-06-21".to_owned(),
                )
                .title
                .0
            })
            .collect();
        assert!(plain.iter().all(|title| title == "The Little Glass Room"));
        life.total.shooting_stars = 1;
        let remembered: Vec<_> = (0..64)
            .map(|seed| {
                compose_book(
                    &mut Random::new(seed),
                    &life,
                    Season::Summer,
                    "2026-06-21".to_owned(),
                )
                .title
                .0
            })
            .collect();
        assert!(
            remembered
                .iter()
                .any(|title| title == "A Night of Shooting Stars")
        );
        assert!(
            remembered
                .iter()
                .all(|title| title == "The Little Glass Room"
                    || title == "A Night of Shooting Stars")
        );
    }

    #[test]
    fn writing_requires_practice_memories_a_quiet_evening_and_a_local_day_gap() {
        let mut world = eligible_writer(7);
        assert!(world.can_write_book());
        world.life.books.owned[0].finished = 0;
        let mut expected = world.random.clone();
        assert!(world.writing_choice().is_none());
        assert_eq!(world.random.next_u64(), expected.next_u64());
        world.life.books.owned[0].finished = 1;
        world.birthday = Some(world.date);
        assert!(!world.can_write_book());
        world.birthday = None;
        world.life.written_day = Some(world.date.to_string());
        world.date = world.date.checked_add(jiff::Span::new().days(9)).unwrap();
        assert!(!world.can_write_book());
        world.date = world.date.checked_add(jiff::Span::new().days(1)).unwrap();
        assert!(world.can_write_book());
        world.hour = 12.0;
        assert!(!world.can_write_book());
        world.hour = 23.0;
        assert!(!world.can_write_book());
        world.hour = 19.0;
        world.start_focus(25);
        assert!(!world.can_write_book());
    }

    #[test]
    fn writing_waits_for_arrival_and_finishes_five_pages_through_the_reading_sitting() {
        let mut world = writing_sitting();
        world.notice_viewer();
        assert!(world.life.written[0].pages.is_empty());
        world.step(0.1);
        assert!(world.life.written[0].pages.is_empty());
        world.ghost.position = world.ghost.plan.target;
        world.ghost.velocity = glam::Vec3::ZERO;
        world.step(0.1);
        let next = world.reading.as_ref().unwrap().next_at.unwrap();
        assert!((15.0..=25.0).contains(&(next - world.time)));
        world.step((next - world.time + 0.1) as f32);
        assert_eq!(world.life.written[0].pages.len(), 1);
        assert_eq!(world.life.total.books_written, 0);
        for _ in 0..1100 {
            if world.life.written[0].finished {
                break;
            }
            world.notice_viewer();
            world.step(0.1);
        }
        let book = &world.life.written[0];
        assert!(book.finished);
        assert_eq!(book.pages.len(), 5);
        assert!(book.outline.is_empty());
        assert_eq!(world.life.total.books_written, 1);
        assert_eq!(world.life.total.pages, 0);
        assert!(
            world
                .ghost
                .speech
                .as_ref()
                .unwrap()
                .text
                .contains("I wrote a book!")
        );
        assert!(
            world
                .life
                .today
                .diary()
                .to_lowercase()
                .contains("wrote my own picture book")
        );
        assert!(
            world
                .life
                .milestones
                .iter()
                .any(|entry| entry.text.starts_with("I wrote a picture book of my own:"))
        );
        assert!(world.reading.is_none());
    }

    #[test]
    fn an_interrupted_draft_keeps_only_written_pages_and_resumes_after_loading() {
        let mut world = writing_sitting();
        world.ghost.position = world.ghost.plan.target;
        world.step(26.0);
        assert_eq!(world.life.written[0].pages.len(), 1);
        let before = world.life.written[0].clone();
        world.poke();
        assert!(world.reading.is_none());
        assert_eq!(world.life.written[0], before);
        let saved = serde_json::to_string(&world.saved_life()).unwrap();
        let mut restored = eligible_writer(19);
        restored.life = serde_json::from_str::<Life>(&saved).unwrap().sanitized();
        restored.begin(Activity::Read);
        assert!(restored.reading.as_ref().unwrap().writing);
        restored.ghost.position = restored.ghost.plan.target;
        restored.step(110.0);
        assert!(restored.life.written[0].finished);
        assert_eq!(restored.life.written[0].pages[0], before.pages[0]);
        assert_eq!(restored.life.total.books_written, 1);
    }

    #[test]
    fn drafts_expire_after_seven_local_days_without_finishing_or_counting() {
        let mut world = writing_sitting();
        world.date = world.date.checked_add(jiff::Span::new().days(6)).unwrap();
        world.expire_drafts();
        assert_eq!(world.life.written.len(), 1);
        world.date = world.date.checked_add(jiff::Span::new().days(1)).unwrap();
        world.expire_drafts();
        assert!(world.life.written.is_empty());
        assert!(world.reading.is_none());
        assert_eq!(world.life.total.books_written, 0);
        assert!(!world.can_write_book());
    }

    #[test]
    fn own_books_are_read_aloud_in_both_languages_and_only_spoken_pages_advance() {
        for (request, korean) in [
            ("read me your book", false),
            ("read the book you wrote", false),
            ("네가 쓴 책 읽어줘", true),
        ] {
            let mut world = World::new(7, 12.0);
            world.life.written.push(finished_book());
            let heard = world.hear(request);
            assert!(
                matches!(
                    heard.understanding,
                    Understanding::Asked {
                        question: Question::OwnBook,
                        ..
                    }
                ),
                "{request}"
            );
            assert!(heard.learned.is_empty());
            assert_eq!(world.telling.as_ref().unwrap().book, Some(LIBRARY.len()));
            world.step(0.1);
            assert_eq!(world.life.written[0].page, 1);
            assert_eq!(world.life.total.pages, 1);
            let text = if korean {
                &world.life.written[0].pages[0].1
            } else {
                &world.life.written[0].pages[0].0
            };
            assert_eq!(&world.ghost.speech.as_ref().unwrap().text, text);
            world.poke();
            world.step(10.0);
            assert_eq!(world.life.written[0].page, 1);
            assert_eq!(world.life.written[0].read, 0);
            world.hear(request);
            world.step(35.0);
            assert_eq!(world.life.written[0].read, 1);
            assert_eq!(world.life.total.pages, 5);
            assert_eq!(world.life.total.stories, 1);
            assert_eq!(world.life.total.books_finished, 1);
        }
    }

    #[test]
    fn an_unfinished_draft_is_kept_private_when_asked_to_read_aloud() {
        let mut world = writing_sitting();
        world.read_aloud(false);
        let telling = world.telling.as_ref().unwrap();
        assert!(telling.book.unwrap() < LIBRARY.len());
        assert!(!telling.lines.is_empty());
        assert!(!world.life.written[0].finished);
        assert!(world.life.written[0].pages.is_empty());
    }

    #[test]
    fn own_books_join_silent_rereading_without_requiring_writing_eligibility() {
        let mut chosen = 0;
        for seed in 0..32 {
            let mut world = World::new(seed, 12.0);
            world.life.written.push(finished_book());
            for owned in &mut world.life.books.owned {
                owned.finished = 1;
                owned.page = LIBRARY[owned.index].pages.len();
            }
            world.begin(Activity::Read);
            if world.reading.as_ref().unwrap().index >= LIBRARY.len() {
                chosen += 1;
                assert!(!world.reading.as_ref().unwrap().writing);
                world.ghost.position = world.ghost.plan.target;
                world.step(26.0);
                assert!(world.life.written[0].page > 0);
            }
        }
        assert!(chosen >= 6, "{chosen}");
    }

    #[test]
    fn saved_creations_are_bounded_validated_and_deduplicated_without_losing_titles() {
        let mut life = memories();
        for seed in 0..12 {
            let notes = super::super::music::compose(
                &mut Random::new(seed),
                Mood::Cheerful,
                Nature::default(),
            );
            life.compositions.push(Composition {
                tune: Tune {
                    name: "anything".to_owned(),
                    notes,
                },
                title: format!("Song {}", super::super::friend::count_words(seed as u32)),
                korean_title: "작은 노래".to_owned(),
                day: life.day.clone(),
            });
        }
        life.compositions.push(life.compositions[11].clone());
        let mut damaged = life.compositions[0].clone();
        damaged.day = "broken".to_owned();
        life.compositions.push(damaged);
        let mut damaged = life.compositions[0].clone();
        damaged.title = "bad\nname".to_owned();
        life.compositions.push(damaged);
        life.compositions[11].tune.notes[0] = Note {
            pitch: 127,
            beats: 99.0,
        };
        life.written = vec![finished_book(); 9];
        life.written.push(WrittenBook {
            day: life.day.clone(),
            ..Default::default()
        });
        let clean = life.sanitized();
        assert_eq!(clean.compositions.len(), 8);
        assert_eq!(clean.written.len(), 6);
        assert!(
            clean
                .compositions
                .iter()
                .all(|song| song.tune.name == song.title)
        );
        assert!(
            clean
                .compositions
                .iter()
                .flat_map(|song| &song.tune.notes)
                .all(|note| (0..=7).contains(&note.pitch) && (0.5..=4.0).contains(&note.beats))
        );
        let loaded: Life = serde_json::from_str(&serde_json::to_string(&clean).unwrap()).unwrap();
        assert_eq!(loaded.sanitized(), clean);
    }

    #[test]
    fn a_practiced_season_spaces_books_by_ten_days_and_stops_at_six() {
        let mut world = eligible_writer(42);
        let mut completed = Vec::new();
        for day in 0..70 {
            for _ in 0..12 {
                if let Some(index) = world.writing_choice() {
                    while !world.write_page(index) {}
                    completed.push(day);
                }
            }
            world.date = world.date.checked_add(jiff::Span::new().days(1)).unwrap();
        }
        assert_eq!(completed.len(), 6, "{completed:?}");
        assert!(completed.windows(2).all(|days| days[1] - days[0] >= 10));
        assert_eq!(world.life.total.books_written, 6);
    }

    #[test]
    fn maximum_saved_names_still_produce_short_paired_pages() {
        let mut life = memories();
        life.viewer_name = Some("가".repeat(16));
        life.friend.as_mut().unwrap().name = "나".repeat(24);
        for seed in 0..64 {
            let book = compose_book(
                &mut Random::new(seed),
                &life,
                Season::Summer,
                life.day.clone(),
            );
            for pair in std::iter::once(&book.title).chain(&book.outline) {
                assert!(valid_text(&pair.0, 80), "{}", pair.0);
                assert!(valid_text(&pair.1, 80), "{}", pair.1);
            }
        }
    }

    #[test]
    fn damaged_dates_control_text_and_partial_books_are_rejected_on_restore() {
        let mut life = memories();
        let valid = finished_book();
        life.written.push(valid.clone());
        for fault in 0..5 {
            let mut book = valid.clone();
            match fault {
                0 => book.day = "not a date".to_owned(),
                1 => book.pages[0].0 = "A bad\npage.".to_owned(),
                2 => book.title.1 = "제목 둘2".to_owned(),
                3 => {
                    book.pages.pop();
                }
                _ => book.day = "2026-06-22".to_owned(),
            }
            life.written.push(book);
        }
        let clean = life.sanitized();
        assert_eq!(clean.written, vec![valid]);
    }

    #[test]
    fn a_practiced_ghost_cannot_keep_more_than_six_books() {
        let mut world = eligible_writer(4);
        world.life.written = vec![finished_book(); 6];
        assert!(!world.can_write_book());
        assert!(world.writing_choice().is_none());
    }
}
