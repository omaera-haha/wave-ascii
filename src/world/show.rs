use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::{
    Activity, EventKind, Flower, Heard, Keepsake, Listening, Mood, Understanding, World,
    acquaintance::Color, books, cranes, friend::count_words,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Keepsake(Keepsake),
    Crane,
    Book,
    Song,
    Flower(Flower),
    WrittenBook,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Keepsake(gift) => gift
                .name()
                .strip_prefix("a ")
                .or_else(|| gift.name().strip_prefix("an "))
                .unwrap_or(gift.name()),
            Self::Crane => "paper crane",
            Self::Book => "favorite picture book",
            Self::Song => "own song",
            Self::Flower(flower) => flower.name(),
            Self::WrittenBook => "own written book",
        }
    }

    pub fn korean(self) -> &'static str {
        match self {
            Self::Keepsake(gift) => gift.korean(),
            Self::Crane => "종이학",
            Self::Book => "제일 좋아하는 그림책",
            Self::Song => "직접 만든 노래",
            Self::Flower(flower) => flower.korean(),
            Self::WrittenBook => "직접 쓴 책",
        }
    }

    fn same_family(self, other: Self) -> bool {
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ShowingLife {
    #[serde(default)]
    pub day: Option<String>,
    #[serde(default)]
    pub recent: Vec<Kind>,
}

impl ShowingLife {
    pub fn sanitize(&mut self, today: &str) {
        self.day = self.day.take().filter(|day| {
            day.parse::<jiff::civil::Date>().is_ok_and(|date| {
                date.to_string() == *day
                    && today
                        .parse::<jiff::civil::Date>()
                        .is_ok_and(|today| date <= today)
            })
        });
        if self.recent.len() > 4 {
            self.recent.drain(..self.recent.len() - 4);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Token {
    Keepsake(Keepsake, usize),
    Crane(Color, usize),
    Book(usize),
    Song,
}

#[derive(Clone, Debug)]
struct Treasure {
    kind: Kind,
    source: Vec3,
    token: Option<Token>,
    english: String,
    korean: String,
    freshness: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Fetching,
    Bringing,
    Sharing,
    Returning,
}

#[derive(Clone, Debug)]
pub struct Showing {
    treasure: Treasure,
    phase: Phase,
    glass: Vec3,
    korean: Option<bool>,
}

pub fn requested(text: &str) -> bool {
    let lower = text
        .trim()
        .trim_end_matches(['.', '!', '！'])
        .to_lowercase();
    let lower = lower.strip_prefix("please ").unwrap_or(&lower);
    matches!(
        lower,
        "show me something"
            | "show me your treasures"
            | "뭐 보여줘"
            | "뭐 보여 줘"
            | "보물 보여줘"
            | "보물 보여 줘"
    )
}

impl World {
    fn show_candidates(&self) -> Vec<Treasure> {
        let mut candidates = Vec::new();
        let mut worn = self.wearing.and_then(super::Accessory::keepsake);
        for (index, gift) in self.life.keepsakes.iter().copied().enumerate() {
            if Some(gift) == worn {
                worn = None;
                continue;
            }
            let name = Kind::Keepsake(gift).name();
            candidates.push(Treasure {
                kind: Kind::Keepsake(gift),
                source: super::SHELF_POSITION + Vec3::new(-0.42 + index as f32 * 0.12, 0.3, 0.23),
                token: Some(Token::Keepsake(gift, index)),
                english: format!(
                    "The bird left this {name} on the sill for me. I keep it on my shelf."
                ),
                korean: format!(
                    "새가 창턱에 두고 간 선물이야. 이 {}도 선반에 소중히 둬.",
                    gift.korean()
                ),
                freshness: 1.0 + (index + 1) as f32 / self.life.keepsakes.len() as f32,
            });
        }
        if self.life.cranes.total > 0 {
            let count = self.life.cranes.total;
            let index = (count as usize).min(cranes::VISIBLE) - 1;
            let color = self
                .life
                .cranes
                .colors
                .get(index)
                .copied()
                .unwrap_or(Color::Pink);
            let hope = if count < 1000 {
                "A little closer to a wish for you."
            } else {
                "I made a wish for gentle days for you."
            };
            let korean_hope = if count < 1000 {
                "너를 위한 소원에 조금 더 가까워졌어."
            } else {
                "네 하루가 다정하길 빌었어."
            };
            candidates.push(Treasure {
                kind: Kind::Crane,
                source: cranes::slot(index) + Vec3::new(0.0, 0.12, 0.12),
                token: Some(Token::Crane(color, index)),
                english: format!("I've folded {} cranes. {hope}", count_words(count)),
                korean: format!(
                    "종이학을 {} 마리 접었어. {korean_hope}",
                    cranes::korean_count(count)
                ),
                freshness: if self.life.cranes.day.as_deref() == Some(self.life.day.as_str()) {
                    2.0
                } else {
                    1.0
                },
            });
        }
        if let Some(index) = self.life.books.favorite()
            && let Some(owned) = self
                .life
                .books
                .owned
                .iter()
                .find(|owned| owned.index == index)
        {
            let book = &books::LIBRARY[index];
            candidates.push(Treasure {
                kind: Kind::Book,
                source: books::POSITION + Vec3::Y * 0.65,
                token: Some(Token::Book(index)),
                english: if owned.finished == 0 {
                    format!(
                        "This is my favorite book, “{}”. I haven't finished it yet.",
                        book.title(false)
                    )
                } else {
                    format!(
                        "This is my favorite book, “{}”. I've read it {} {}.",
                        book.title(false),
                        count_words(owned.finished),
                        if owned.finished == 1 { "time" } else { "times" }
                    )
                },
                korean: if owned.finished == 0 {
                    format!(
                        "내가 제일 좋아하는 책, 《{}》이야. 아직 끝까지 읽진 못했어.",
                        book.title(true)
                    )
                } else {
                    format!(
                        "내가 제일 좋아하는 책, 《{}》이야. {} 번 읽었어.",
                        book.title(true),
                        cranes::korean_count(owned.finished)
                    )
                },
                freshness: 1.0,
            });
        }
        if let Some(song) = self.life.compositions.last() {
            candidates.push(Treasure {
                kind: Kind::Song,
                source: super::music::POSITION + Vec3::Y * 0.8,
                token: Some(Token::Song),
                english: format!("I wrote this song myself. It's called “{}”.", song.title),
                korean: format!(
                    "내가 직접 만든 노래야. 제목은 《{}》이야.",
                    song.korean_title
                ),
                freshness: freshness(&song.day, self.date),
            });
        }
        for (index, plant) in self
            .seedlings
            .iter()
            .enumerate()
            .filter(|(_, plant)| plant.flowering(self.season()))
        {
            candidates.push(Treasure {
                kind: Kind::Flower(plant.kind),
                source: super::garden::SPOTS[index] + Vec3::new(0.0, 0.85, 0.35),
                token: None,
                english: format!(
                    "Look, my {} is blooming! I grew it in that little pot.",
                    plant.kind.name()
                ),
                korean: format!(
                    "봐, 내 {} 꽃이 피었어! 저 작은 화분에서 키웠어.",
                    plant.kind.korean()
                ),
                freshness: if plant.age < 7.0 * 86400.0 { 2.0 } else { 1.0 },
            });
        }
        if let Some((index, book)) = self
            .life
            .written
            .iter()
            .enumerate()
            .rev()
            .find(|(_, book)| book.finished)
        {
            candidates.push(Treasure {
                kind: Kind::WrittenBook,
                source: books::POSITION + Vec3::Y * 0.65,
                token: Some(Token::Book(books::LIBRARY.len() + index)),
                english: format!(
                    "I wrote this book from my memories. It's called “{}”.",
                    book.title.0
                ),
                korean: format!(
                    "내 추억을 모아서 쓴 책이야. 제목은 《{}》이야.",
                    book.title.1
                ),
                freshness: freshness(&book.day, self.date),
            });
        }
        candidates
    }

    fn show_weights(&self, candidates: &[Treasure]) -> Vec<f32> {
        candidates
            .iter()
            .map(|candidate| {
                let family_count = candidates
                    .iter()
                    .filter(|other| other.kind.same_family(candidate.kind))
                    .count();
                let recent = self
                    .life
                    .showing
                    .recent
                    .iter()
                    .rev()
                    .position(|kind| kind.same_family(candidate.kind));
                candidate.freshness / family_count as f32
                    * recent.map_or(1.0, |index| 0.08 + index as f32 * 0.08)
            })
            .collect()
    }

    fn show_is_quiet(&self) -> bool {
        self.ghost.asleep < 0.3
            && self.viewer_present()
            && self.quiet_for_question()
            && self.asking.is_none()
            && !self.awaiting_plan_answer()
            && self.feeling_follow_up.is_none()
            && self.comforting.is_none()
            && !self.pending_crane
            && self.pending_tune.is_none()
            && !self.reading_viewer_letter()
            && self.bubble_session.is_none()
            && self.showing.is_none()
    }

    fn has_show_treasure(&self) -> bool {
        let worn = self.wearing.and_then(super::Accessory::keepsake);
        let borrowed = usize::from(worn.is_some_and(|gift| self.life.keepsakes.contains(&gift)));
        self.life.keepsakes.len() > borrowed
            || self.life.cranes.total > 0
            || !self.life.books.owned.is_empty()
            || !self.life.compositions.is_empty()
            || self
                .seedlings
                .iter()
                .any(|plant| plant.flowering(self.season()))
            || self.life.written.iter().any(|book| book.finished)
    }

    pub(super) fn show_can_begin(&self) -> bool {
        self.show_is_quiet() && self.has_show_treasure()
    }

    pub fn show_is_open(&self) -> bool {
        // Typing holds back its own offer, not a request just sent.
        self.ghost.plan.activity == Activity::Visit
            && !self.ghost.plan.done
            && !self.viewer.typing
            && self.show_is_quiet()
            && !self.life.wish.as_ref().is_some_and(|wishing| {
                wishing.wish.with_viewer()
                    && wishing.said.as_deref() != Some(self.life.day.as_str())
            })
            && self.life.bond >= 0.4
            && self
                .day_of_life()
                .map_or(self.life.diary.len() >= 3, |day| day >= 4)
            && super::creation::elapsed_days(self.life.showing.day.as_deref(), self.date)
                .is_none_or(|days| days > 0)
            && (0.0..=600.0).contains(&(self.time - self.viewer.active_at))
            && self
                .ghost
                .plan
                .arrived
                .is_some_and(|at| self.time - at >= 5.0)
            && self
                .ghost
                .speech
                .as_ref()
                .is_none_or(|line| self.time - line.time >= 12.0)
            && self.has_show_treasure()
    }

    pub(super) fn offer_show(&mut self, seconds: f32) {
        if self.show_is_open() && self.random.unit() < seconds.min(60.0) / 300.0 {
            self.begin(Activity::Show);
        }
    }

    pub(super) fn hear_show(&mut self, text: &str, korean: bool) -> Option<Heard> {
        if !requested(text) {
            return None;
        }
        self.listening = Listening::default();
        self.notice_viewer();
        self.record(EventKind::Heard(text.to_owned()));
        if self.show_can_begin() {
            self.begin(Activity::Show);
            if let Some(showing) = &mut self.showing {
                showing.korean = Some(korean);
            }
            self.say(if korean {
                "내 보물 하나 보여줄게. 잠깐만!"
            } else {
                "I'll bring one of my treasures. Just a moment!"
            });
        } else if self.ghost.asleep < 0.3 {
            self.say(if korean {
                "조금 한가해지면 보여줄게."
            } else {
                "I'll show you when things are quiet."
            });
        }
        Some(Heard {
            tone: None,
            understanding: Understanding::Showing,
            named: false,
            learned: Vec::new(),
            stopped: false,
            learned_name: None,
            given_name: None,
            korean,
        })
    }

    pub(super) fn begin_show(&mut self) {
        let candidates = self.show_candidates();
        let weights = self.show_weights(&candidates);
        let Some(index) = self.pick(&weights) else {
            self.ghost.plan.done = true;
            return;
        };
        let treasure = candidates.into_iter().nth(index).unwrap();
        self.life.showing.day = Some(self.date.to_string());
        self.life.showing.recent.push(treasure.kind);
        if self.life.showing.recent.len() > 4 {
            self.life.showing.recent.remove(0);
        }
        self.think(&format!("I want to show them my {}.", treasure.kind.name()));
        self.ghost.plan.target = treasure.source;
        self.ghost.plan.length = self.random.range(8.0, 12.0);
        let mut glass = self.glass_spot();
        glass.y = 1.3;
        self.showing = Some(Showing {
            treasure,
            phase: Phase::Fetching,
            glass,
            korean: None,
        });
    }

    pub(super) fn stop_show(&mut self) {
        if self.showing.take().is_some() && self.ghost.plan.activity == Activity::Show {
            self.ghost.plan.done = true;
        }
    }

    pub(super) fn carry_out_show(&mut self) -> Option<Vec3> {
        if !self.viewer_present()
            || self.focus.is_some()
            || self.ghost.asleep >= 0.3
            || self.ghost.plan.elapsed(self.time) > 150.0
        {
            self.stop_show();
        }
        let Some(showing) = &self.showing else {
            self.ghost.plan.done = true;
            return None;
        };
        let source = showing.treasure.source;
        let glass = showing.glass;
        match showing.phase {
            Phase::Fetching if self.ghost.plan.arrived.is_some() => {
                self.showing.as_mut().unwrap().phase = Phase::Bringing;
                self.ghost.plan.target = glass;
                self.ghost.plan.arrived = None;
            }
            Phase::Bringing if self.ghost.plan.arrived.is_some() => {
                let line = if showing.korean.unwrap_or(self.speaks_korean()) {
                    &showing.treasure.korean
                } else {
                    &showing.treasure.english
                }
                .clone();
                self.showing.as_mut().unwrap().phase = Phase::Sharing;
                self.show_praise_until = Some(self.time + 60.0);
                self.say(&line);
                self.suggest_mood(Mood::Cheerful, 12.0);
            }
            Phase::Sharing if self.ghost.plan.stayed(self.time) >= self.ghost.plan.length => {
                self.showing.as_mut().unwrap().phase = Phase::Returning;
                self.ghost.plan.target = source;
                self.ghost.plan.arrived = None;
            }
            Phase::Returning if self.ghost.plan.arrived.is_some() => {
                let kind = showing.treasure.kind;
                self.stop_show();
                self.ghost.plan.done = true;
                self.ghost.needs.change(0.0, 0.04, 0.04);
                self.record(EventKind::ShowedThing(kind));
            }
            _ => {}
        }
        Some(
            if self
                .showing
                .as_ref()
                .is_some_and(|showing| showing.phase == Phase::Sharing)
            {
                if self.showing.as_ref().unwrap().treasure.token.is_none() {
                    source
                } else {
                    glass + Vec3::Z * 3.0
                }
            } else {
                self.ghost.plan.target
            },
        )
    }

    pub fn shown_token(&self) -> Option<(Token, Vec3)> {
        let showing = self.showing.as_ref()?;
        if showing.phase == Phase::Fetching || self.ghost.plan.done {
            return None;
        }
        Some((
            showing.treasure.token?,
            self.ghost.position - Vec3::Y * (0.36 * self.size()),
        ))
    }

    pub fn show_bob(&self) -> f32 {
        if self
            .showing
            .as_ref()
            .is_some_and(|showing| showing.phase == Phase::Sharing)
        {
            (self.ghost.plan.stayed(self.time) * 2.4).sin() * 0.5 + 0.5
        } else {
            0.0
        }
    }

    pub fn show_words(&self) -> Option<String> {
        let showing = self.showing.as_ref()?;
        let action = match showing.phase {
            Phase::Fetching => "fetching",
            Phase::Bringing | Phase::Sharing => "showing the viewer",
            Phase::Returning => "putting away",
        };
        Some(format!(
            "you are {action} your {}",
            showing.treasure.kind.name()
        ))
    }
}

fn freshness(day: &str, today: jiff::civil::Date) -> f32 {
    if super::creation::elapsed_days(Some(day), today).is_some_and(|days| (0..=7).contains(&days)) {
        2.0
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Life, Plan, Seedling, creation::WrittenBook, music};

    fn grown() -> World {
        let mut world = World::new(7, 12.0);
        world.begin_life(&world.date.to_string());
        world.birthday = Some(world.date.checked_sub(jiff::Span::new().days(10)).unwrap());
        world.time = 100.0;
        world.life.bond = 0.55;
        world.life.books.owned.clear();
        world.life.keepsakes.push(Keepsake::Acorn);
        world.notice_viewer();
        settle(&mut world);
        world
    }

    fn settle(world: &mut World) {
        world.ghost.plan = Plan::new(
            Activity::Visit,
            world.time - 10.0,
            world.ghost.position,
            40.0,
        );
        world.ghost.plan.arrived = Some(world.time - 10.0);
        world.ghost.speech = None;
    }

    fn possessions(world: &mut World) {
        world.life.books = books::Books::default();
        world.life.cranes.total = 12;
        world.life.cranes.colors = vec![Color::Blue; 12];
        world.life.books.owned[0].finished = 5;
        world.life.compositions.push(music::Composition {
            title: "Little Light".to_owned(),
            korean_title: "작은 빛".to_owned(),
            tune: music::built_in(0),
            day: world.date.to_string(),
        });
        let mut plant = Seedling::new(Flower::Tulip);
        plant.growth = 1.0;
        world.seedlings = vec![plant];
        world.life.written.push(WrittenBook {
            title: ("Our Days".to_owned(), "우리의 하루".to_owned()),
            pages: vec![("A little light.".to_owned(), "작은 빛.".to_owned()); 5],
            finished: true,
            day: world.date.to_string(),
            ..WrittenBook::default()
        });
    }

    fn reach_target(world: &mut World) {
        world.ghost.position = world.ghost.plan.target;
        world.ghost.plan.arrived = Some(world.time);
        world.carry_out_show();
    }

    fn share(world: &mut World) {
        reach_target(world);
        reach_target(world);
        assert_eq!(world.showing.as_ref().unwrap().phase, Phase::Sharing);
    }

    fn finish(world: &mut World) {
        share(world);
        world.time += f64::from(world.ghost.plan.length) + 0.1;
        world.carry_out_show();
        reach_target(world);
    }

    #[test]
    fn candidates_only_offer_owned_treasures_and_a_bloom_that_stays_in_its_pot() {
        let mut world = grown();
        world.life.keepsakes.clear();
        assert!(world.show_candidates().is_empty());
        assert!(!world.options().contains(&Activity::Show));
        world.life.keepsakes.push(Keepsake::Acorn);
        possessions(&mut world);
        let candidates = world.show_candidates();
        assert_eq!(candidates.len(), 6);
        for kind in [
            Kind::Keepsake(Keepsake::Acorn),
            Kind::Crane,
            Kind::Book,
            Kind::Song,
            Kind::Flower(Flower::Tulip),
            Kind::WrittenBook,
        ] {
            assert!(candidates.iter().any(|candidate| candidate.kind == kind));
        }
        let book = candidates
            .iter()
            .find(|candidate| candidate.kind == Kind::Book)
            .unwrap();
        assert!(book.english.contains("five times"));
        assert!(book.korean.contains(books::LIBRARY[0].title(true)));
        let flower = candidates
            .iter()
            .find(|candidate| matches!(candidate.kind, Kind::Flower(_)))
            .unwrap();
        assert!(flower.token.is_none());
        world.seedlings[0].droop = 0.1;
        assert!(
            !world
                .show_candidates()
                .iter()
                .any(|candidate| matches!(candidate.kind, Kind::Flower(_)))
        );
        world.life.keepsakes = vec![Keepsake::Feather];
        world.wearing = Some(super::super::Accessory::Feather);
        assert!(
            !world
                .show_candidates()
                .iter()
                .any(|candidate| matches!(candidate.kind, Kind::Keepsake(_)))
        );
    }

    #[test]
    fn selection_is_seeded_and_recent_families_yield_to_newer_unshown_treasures() {
        let mut first = grown();
        possessions(&mut first);
        let mut second = first.clone();
        let mut sequence = Vec::new();
        for _ in 0..30 {
            first.begin(Activity::Show);
            second.begin(Activity::Show);
            let kind = first.showing.as_ref().unwrap().treasure.kind;
            assert_eq!(kind, second.showing.as_ref().unwrap().treasure.kind);
            sequence.push(kind);
            first.stop_show();
            second.stop_show();
            first.date = first.date.tomorrow().unwrap();
            second.date = second.date.tomorrow().unwrap();
            first.begin_day();
            second.begin_day();
            first.notice_viewer();
            second.notice_viewer();
            settle(&mut first);
            settle(&mut second);
        }
        assert!(
            sequence
                .windows(2)
                .filter(|pair| !pair[0].same_family(pair[1]))
                .count()
                >= 20
        );
        assert_eq!(first.life.showing.recent.len(), 4);
        first.life.showing.recent = vec![Kind::Book];
        let candidates = first.show_candidates();
        let weights = first.show_weights(&candidates);
        let weight = |kind| {
            weights[candidates
                .iter()
                .position(|candidate| candidate.kind == kind)
                .unwrap()]
        };
        assert!(weight(Kind::Song) > weight(Kind::Book));
        let mut candidates = candidates;
        let index = candidates
            .iter()
            .position(|candidate| candidate.kind == Kind::Song)
            .unwrap();
        candidates[index].freshness = 2.0;
        let newer = first.show_weights(&candidates)[index];
        candidates[index].freshness = 1.0;
        assert!(newer > first.show_weights(&candidates)[index]);
    }

    #[test]
    fn daily_limit_survives_an_interruption_and_a_saved_life() {
        let mut world = grown();
        assert!(world.show_is_open());
        world.begin(Activity::Show);
        world.begin(Activity::Visit);
        settle(&mut world);
        assert!(!world.show_is_open());
        let saved = serde_json::to_string(&world.saved_life()).unwrap();
        let mut restored = grown();
        restored.life = serde_json::from_str::<Life>(&saved).unwrap().sanitized();
        assert!(!restored.show_is_open());
        restored.date = restored.date.yesterday().unwrap();
        assert!(!restored.show_is_open());
        restored.date = world.date.tomorrow().unwrap();
        restored.begin_day();
        assert!(restored.show_is_open());
    }

    #[test]
    fn offers_wait_for_a_settled_warm_visit_with_recent_attention() {
        let base = grown();
        assert!(base.show_is_open());
        for condition in 0..11 {
            let mut world = base.clone();
            match condition {
                0 => world.life.bond = 0.39,
                1 => world.birthday = Some(world.date.yesterday().unwrap()),
                2 => world.viewer.active_at = 0.0,
                3 => world.viewer.typing = true,
                4 => world.ghost.asleep = 0.5,
                5 => world.ghost.plan.activity = Activity::Float,
                6 => world.ghost.plan.arrived = Some(world.time - 4.0),
                7 => world.ghost.plan.done = true,
                8 => world.say("Just a moment."),
                9 => world.viewer.looked_away_at = Some(world.time - 60.0),
                _ => {
                    world.time += 601.0;
                }
            }
            assert!(!world.show_is_open(), "condition {condition}");
        }
    }

    #[test]
    fn pending_questions_follow_ups_and_quiet_activities_keep_their_turn() {
        for condition in 0..12 {
            let mut world = grown();
            match condition {
                0 => {
                    world.asking =
                        Some((super::super::acquaintance::Topic::Color, world.time + 60.0))
                }
                1 => {
                    world.feeling_follow_up = Some((super::super::Feeling::Sad, world.time + 60.0))
                }
                2 => {
                    world.agenda_answer = Some((
                        super::super::agenda::Errand::Exam,
                        world.date.to_string(),
                        world.time + 60.0,
                    ))
                }
                3 => world.rhythm.reply_until = Some(world.time + 60.0),
                4 => {
                    world.start_focus(25);
                }
                5 => {
                    world.tell(vec!["A little light.".to_owned()]);
                }
                6 => world.begin(Activity::Game),
                7 => world.begin(Activity::Puzzle),
                8 => {
                    world.start_hiding().unwrap();
                }
                9 => world.pending_wish = Some(super::super::Wish::Petting),
                10 => world.pending_crane = true,
                _ => {
                    world.life.wish = Some(super::super::Wishing {
                        wish: super::super::Wish::Petting,
                        day: world.date.to_string(),
                        said: None,
                    });
                }
            }
            assert!(!world.show_is_open(), "condition {condition}");
            world.offer_show(60.0);
            assert!(world.showing.is_none());
        }
    }

    #[test]
    fn requests_in_both_languages_bypass_the_daily_limit_and_teach_no_words() {
        for text in [
            "show me something",
            "show me your treasures",
            "뭐 보여줘",
            "보물 보여줘",
        ] {
            let mut world = grown();
            world.life.showing.day = Some(world.date.to_string());
            world.life.bond = 0.0;
            world.birthday = Some(world.date);
            let vocabulary = serde_json::to_value(&world.life.vocabulary).unwrap();
            let heard = world.hear(text);
            assert_eq!(heard.understanding, Understanding::Showing);
            assert!(heard.learned.is_empty());
            assert_eq!(
                vocabulary,
                serde_json::to_value(&world.life.vocabulary).unwrap()
            );
            assert_eq!(world.ghost.plan.activity, Activity::Show);
            share(&mut world);
            let line = &world.ghost.speech.as_ref().unwrap().text;
            assert_eq!(line.contains("새가"), heard.korean);
        }
        // A request typed across frames was still being typed a moment ago.
        let mut world = grown();
        world.viewer.typing = true;
        world.hear("show me something");
        assert_eq!(world.ghost.plan.activity, Activity::Show);
        for text in [
            "don't show me something",
            "I will show you something",
            "show me your treasures?",
            "보물 보여주지 마",
            "보물 보여줘?",
            "can you show me something else",
        ] {
            assert!(!requested(text), "{text}");
        }
    }

    #[test]
    fn requests_leave_sleep_work_stories_and_hunts_undisturbed() {
        for condition in 0..5 {
            let mut world = grown();
            match condition {
                0 => {
                    world.hour = 23.0;
                    world.tuck_in();
                }
                // The small hours' sleepy nudge mustn't wake it either.
                4 => {
                    world.hour = 1.0;
                    world.tuck_in();
                }
                1 => world.start_focus(25),
                2 => world.tell(vec!["A little light.".to_owned()]),
                _ => {
                    world.start_hiding().unwrap();
                }
            }
            let plan = world.ghost.plan.clone();
            let asleep = world.ghost.asleep;
            assert_eq!(
                world.hear("show me something").understanding,
                Understanding::Showing
            );
            assert_eq!(world.ghost.plan, plan);
            assert_eq!(world.ghost.asleep, asleep);
            assert!(world.showing.is_none());
        }
    }

    #[test]
    fn finishing_returns_the_treasure_then_counts_one_event_and_first_milestone() {
        let mut world = grown();
        let before = world.life.bond;
        world.begin(Activity::Show);
        assert!(world.shown_token().is_none());
        finish(&mut world);
        assert!(world.showing.is_none());
        assert!(world.shown_token().is_none());
        assert_eq!(world.life.keepsakes, [Keepsake::Acorn]);
        assert_eq!(world.life.today.things_shown, 1);
        assert_eq!(world.life.total.things_shown, 1);
        assert!(world.life.bond > before);
        assert!(
            world
                .life
                .today
                .words()
                .to_lowercase()
                .contains("showed you my acorn")
        );
        assert_eq!(
            world
                .life
                .milestones
                .iter()
                .filter(|entry| entry.text.contains("treasures for the first time"))
                .count(),
            1
        );
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::ShowedThing(Kind::Keepsake(Keepsake::Acorn)))
        );
        world.carry_out_show();
        assert_eq!(world.life.total.things_shown, 1);
        for event in &world.events {
            assert!(
                !event
                    .kind
                    .words()
                    .chars()
                    .any(|character| character.is_ascii_digit())
            );
        }
    }

    #[test]
    fn interruptions_and_saves_return_borrowed_things_without_creating_gifts_or_shows() {
        for condition in 0..6 {
            let mut world = grown();
            world.begin(Activity::Show);
            reach_target(&mut world);
            assert!(world.shown_token().is_some());
            let saved = world.saved_life();
            assert_eq!(saved.keepsakes, [Keepsake::Acorn]);
            assert_eq!(saved.total.gifts, 0);
            assert_eq!(saved.total.things_shown, 0);
            match condition {
                0 => world.begin(Activity::Rest),
                1 => world.start_focus(25),
                2 => {
                    world.hear("no");
                }
                3 => world.poke(),
                4 => world.record(EventKind::ViewerLeft),
                _ => {
                    world.hour = 23.0;
                    world.tuck_in();
                }
            }
            assert!(world.shown_token().is_none());
            assert!(world.showing.is_none());
            assert_eq!(world.life.keepsakes, [Keepsake::Acorn]);
            assert_eq!(world.life.total.things_shown, 0);
        }
    }

    #[test]
    fn praise_during_the_story_uses_the_existing_heart_and_delights_without_abandoning_it() {
        for text in ["cute!", "wow", "예쁘다", "멋지다"] {
            let mut world = grown();
            world.begin(Activity::Show);
            share(&mut world);
            assert_eq!(world.hear(text).tone, Some(super::super::Tone::Praise));
            assert!(world.showing.is_some());
            assert_eq!(world.ghost.suggested_mood.unwrap().0, Mood::Excited);
            assert!(
                world
                    .events
                    .iter()
                    .any(|event| event.kind == EventKind::Praised)
            );
            assert_eq!(world.life.today.praises, 1);
        }
    }

    #[test]
    fn actual_flights_reach_every_source_and_return_in_under_two_minutes() {
        let mut base = grown();
        possessions(&mut base);
        base.calendar_at = Some(base.date);
        for treasure in base.show_candidates() {
            let mut world = base.clone();
            world.begin(Activity::Show);
            world.ghost.plan.target = treasure.source;
            world.showing.as_mut().unwrap().treasure = treasure.clone();
            for _ in 0..1200 {
                world.notice_viewer();
                world.step(0.1);
                if world.ghost.plan.done {
                    break;
                }
            }
            assert!(
                world.ghost.plan.done,
                "{:?}: {:?} {:?}",
                treasure.kind, world.ghost.plan, world.ghost.position
            );
            assert_eq!(world.life.total.things_shown, 1, "{:?}", treasure.kind);
            assert_eq!(world.life.keepsakes, base.life.keepsakes);
            assert_eq!(world.life.cranes, base.life.cranes);
            assert_eq!(world.life.books, base.life.books);
        }
    }

    #[test]
    fn old_saves_load_and_damaged_show_history_and_tallies_are_bounded() {
        let life: Life = serde_json::from_str("{}").unwrap();
        assert_eq!(life.showing, ShowingLife::default());
        assert_eq!(life.total.things_shown, 0);
        let mut life = grown().life;
        life.showing.day = Some("damaged".to_owned());
        life.showing.recent = vec![Kind::Book; 100];
        life.today.things_shown = u32::MAX;
        life.today.shown_kinds = vec![Kind::Book; 100];
        life = life.sanitized();
        assert!(life.showing.day.is_none());
        assert_eq!(life.showing.recent.len(), 4);
        assert_eq!(life.today.things_shown, 100_000_000);
        assert_eq!(life.today.shown_kinds, [Kind::Book]);
        life.showing.day = Some("2027-06-21".to_owned());
        assert!(life.sanitized().showing.day.is_none());
    }

    #[test]
    fn a_request_does_not_answer_a_pending_personal_question_or_learn_an_animal() {
        let mut world = grown();
        world.asking = Some((super::super::acquaintance::Topic::Animal, world.time + 60.0));
        assert_eq!(
            world.hear("show me something").understanding,
            Understanding::Showing
        );
        assert!(world.asking.is_some());
        assert!(world.life.viewer.animal.is_none());
        assert!(world.showing.is_none());
    }

    #[test]
    fn spontaneous_chances_eventually_offer_once_and_a_second_request_counts_no_second_first() {
        let mut world = grown();
        for _ in 0..100 {
            world.offer_show(60.0);
            if world.showing.is_some() {
                break;
            }
        }
        assert!(world.showing.is_some());
        finish(&mut world);
        settle(&mut world);
        for _ in 0..100 {
            world.offer_show(60.0);
        }
        assert!(world.showing.is_none());
        world.hear("show me something");
        finish(&mut world);
        assert_eq!(world.life.total.things_shown, 2);
        assert_eq!(
            world
                .life
                .milestones
                .iter()
                .filter(|entry| entry.text.contains("treasures for the first time"))
                .count(),
            1
        );
    }

    #[test]
    fn a_new_ghost_can_show_an_owned_book_without_claiming_to_have_finished_it() {
        for text in ["show me something", "보물 보여줘"] {
            let mut world = World::new(7, 12.0);
            assert_eq!(world.hear(text).understanding, Understanding::Showing);
            assert_eq!(world.showing.as_ref().unwrap().treasure.kind, Kind::Book);
            share(&mut world);
            let line = &world.ghost.speech.as_ref().unwrap().text;
            assert!(line.contains("haven't finished") || line.contains("아직 끝까지 읽진 못했어"));
        }
    }
}
