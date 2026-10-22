use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::acquaintance::Color;
use super::{Activity, EventKind, Feeling, Heard, Listening, Mood, Stage, Understanding, World};

pub const VISIBLE: usize = 80;
pub const MAXIMUM: u32 = 100_000;
pub const CENTER: Vec3 = Vec3::new(1.52, 1.89, -1.14);
pub const SEAT: Vec3 = Vec3::new(-0.92, 0.72, 0.0);
pub const LANDMARKS: [u32; 6] = [1, 10, 50, 100, 500, 1000];

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Paper {
    #[serde(default = "paper_color")]
    pub color: Color,
    #[serde(default)]
    pub progress: f32,
}

fn paper_color() -> Color {
    Color::Pink
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Cranes {
    #[serde(default)]
    pub total: u32,
    #[serde(default, deserialize_with = "super::life::lenient")]
    pub colors: Vec<Color>,
    #[serde(default)]
    pub day: Option<String>,
    #[serde(default)]
    pub today: u32,
    #[serde(default)]
    pub paper: Option<Paper>,
    /// A day's small allowance is drawn once, so reopening cannot reroll it.
    #[serde(default)]
    pub allowance: u32,
    #[serde(default)]
    pub ready_hour: f32,
    #[serde(default)]
    pub allowance_day: Option<String>,
}

impl Cranes {
    pub fn sanitize(&mut self, day: &str) {
        self.total = self.total.min(MAXIMUM);
        self.colors.truncate(VISIBLE.min(self.total as usize));
        if !self.day.as_ref().is_some_and(|saved| {
            saved.parse::<jiff::civil::Date>().is_ok_and(|saved_day| {
                saved_day.to_string() == *saved
                    && day
                        .parse::<jiff::civil::Date>()
                        .is_ok_and(|today| saved_day <= today)
            })
        }) {
            self.day = None;
            self.today = 0;
            self.ready_hour = 0.0;
        }
        self.today = self.today.min(3).min(self.total);
        self.allowance = self.allowance.min(3);
        self.allowance_day = self.allowance_day.take().filter(|saved| {
            saved.parse::<jiff::civil::Date>().is_ok_and(|saved_day| {
                saved_day.to_string() == *saved
                    && day
                        .parse::<jiff::civil::Date>()
                        .is_ok_and(|today| saved_day <= today)
            })
        });
        if self.allowance_day.is_none() {
            self.allowance = 0;
        }
        self.ready_hour = if self.ready_hour.is_finite() {
            self.ready_hour.clamp(0.0, 24.0)
        } else {
            24.0
        };
        self.paper = self.paper.filter(|paper| paper.progress.is_finite());
        if let Some(paper) = &mut self.paper {
            paper.progress = paper.progress.clamp(0.0, 1.0);
        }
    }

    pub fn count_today(&self, day: jiff::civil::Date) -> u32 {
        if self
            .day
            .as_deref()
            .and_then(|saved| saved.parse::<jiff::civil::Date>().ok())
            .is_some_and(|saved| saved >= day)
        {
            self.today
        } else {
            0
        }
    }

    pub fn words(&self) -> String {
        format!(
            "your string of paper cranes, {} of them",
            super::friend::count_words(self.total)
        )
    }
}

#[derive(Clone, Debug)]
pub struct Folding {
    pub lighting: bool,
    duration: f32,
}

/// The first ten fill one thread; after eighty the picture keeps its first cranes.
pub fn slot(index: usize) -> Vec3 {
    let index = index.min(VISIBLE - 1);
    let thread = index / 10;
    Vec3::new(
        1.25 + (thread % 4) as f32 * 0.18,
        2.12 - (index % 10) as f32 * 0.08,
        -1.23 + (thread / 4) as f32 * 0.18,
    )
}

pub fn requested(text: &str) -> bool {
    let text = text.trim().to_lowercase();
    if text.contains(['?', '？']) {
        return false;
    }
    let words: Vec<_> = text
        .split(|character: char| !character.is_alphabetic() && character != '\'')
        .filter(|word| !word.is_empty())
        .collect();
    let words = words.strip_prefix(&["please"]).unwrap_or(&words);
    matches!(
        words,
        ["fold" | "make", "a", "paper", "crane"]
            | ["fold", "a", "crane"]
            | ["fold" | "make", "me", "a", "paper", "crane"]
            | ["종이학" | "학", "접어줘" | "접어"]
    )
}

pub fn korean_count(count: u32) -> String {
    const ONES: [&str; 10] = [
        "", "한", "두", "세", "네", "다섯", "여섯", "일곱", "여덟", "아홉",
    ];
    const TENS: [&str; 10] = [
        "", "열", "스물", "서른", "마흔", "쉰", "예순", "일흔", "여든", "아흔",
    ];
    if count == 0 {
        return "영".to_owned();
    }
    if count == 20 {
        return "스무".to_owned();
    }
    if count < 100 {
        return format!(
            "{}{}",
            TENS[(count / 10) as usize],
            ONES[(count % 10) as usize]
        );
    }
    const DIGITS: [&str; 10] = ["", "일", "이", "삼", "사", "오", "육", "칠", "팔", "구"];
    let mut words = String::new();
    let mut remaining = count.min(MAXIMUM);
    for (size, unit) in [(10_000, "만"), (1000, "천"), (100, "백"), (10, "십")] {
        let amount = remaining / size;
        if amount > 0 {
            if amount > 1 {
                words.push_str(&korean_count_sino(amount));
            }
            words.push_str(unit);
        }
        remaining %= size;
    }
    words.push_str(DIGITS[remaining as usize]);
    words
}

fn korean_count_sino(count: u32) -> String {
    const DIGITS: [&str; 10] = ["", "일", "이", "삼", "사", "오", "육", "칠", "팔", "구"];
    if count == 10 {
        "십".to_owned()
    } else {
        DIGITS[count as usize].to_owned()
    }
}

impl World {
    pub fn can_fold(&self) -> bool {
        self.stage() != Stage::Newborn
            && self.life.cranes.total < MAXIMUM
            && self.life.cranes.count_today(self.date) < 3
    }

    pub fn fold_is_open(&self) -> bool {
        if !self.can_fold()
            || self.focus.is_some()
            || self.ghost.asleep >= 0.3
            || self.ghost.needs.energy <= 0.25
        {
            return false;
        }
        let cranes = &self.life.cranes;
        if cranes.paper.is_some() {
            return true;
        }
        let today = cranes.count_today(self.date);
        if today > 0 && (today >= cranes.allowance || self.hour < cranes.ready_hour) {
            return false;
        }
        if cranes.total >= 1000 {
            return super::creation::elapsed_days(cranes.day.as_deref(), self.date)
                .is_none_or(|days| days >= 3);
        }
        true
    }

    pub fn crane_for_sad_viewer(&self) -> bool {
        let day = self.date.to_string();
        self.life
            .feelings
            .iter()
            .any(|note| note.day == day && note.feeling == Feeling::Sad)
    }

    pub(super) fn hear_cranes(&mut self, text: &str, korean: bool) -> Option<Heard> {
        if !requested(text) {
            return None;
        }
        self.listening = Listening::default();
        self.notice_viewer();
        self.record(EventKind::Heard(text.to_owned()));
        if self.life.cranes.count_today(self.date) >= 3 || self.life.cranes.total >= MAXIMUM {
            self.say(if korean {
                "오늘은 손이 지쳤어. 내일 또 접을게."
            } else {
                "My fingers are tired; more tomorrow."
            });
        } else if self.stage() == Stage::Newborn {
            self.say(if korean {
                "조금 더 크면 너를 위해 종이학을 접어 줄게."
            } else {
                "When I'm a little bigger, I'll fold a crane for you."
            });
        } else {
            self.pending_crane = self.ghost.plan.activity != Activity::Fold || self.ghost.plan.done;
            self.advance_cranes();
            self.say(if korean {
                "너를 위해 한 마리 접을게. 조용해지면 마저 접자."
            } else {
                "I'll fold one for you, when it's quiet."
            });
        }
        Some(Heard {
            tone: None,
            understanding: Understanding::Folding,
            named: false,
            learned: Vec::new(),
            stopped: false,
            learned_name: None,
            given_name: None,
            korean,
        })
    }

    pub(super) fn advance_cranes(&mut self) {
        // A saved finished paper settles once after restore, like a carried gift.
        if self.folding.is_none()
            && self
                .life
                .cranes
                .paper
                .is_some_and(|paper| paper.progress >= 1.0)
        {
            self.hang_crane();
        }
        if self.pending_crane
            && self.can_fold()
            && self.ghost.asleep < 0.3
            && self.focus.is_none()
            && self.telling.is_none()
            && (self.ghost.plan.activity != Activity::Fold || self.ghost.plan.done)
            && (self.ghost.plan.done
                || matches!(
                    self.ghost.plan.activity,
                    Activity::Float | Activity::Visit | Activity::Company
                ))
        {
            self.begin(Activity::Fold);
        }
    }

    pub(super) fn begin_folding(&mut self) {
        self.pending_crane = false;
        let day = self.date.to_string();
        if self.life.cranes.allowance_day.as_deref() != Some(&day) {
            self.life.cranes.allowance_day = Some(day);
            self.life.cranes.allowance = 0;
        }
        if self.life.cranes.allowance == 0 {
            let chance = self.random.unit();
            self.life.cranes.allowance = if chance < 0.02 {
                3
            } else if chance < 0.32 {
                2
            } else {
                1
            };
        }
        if self.life.cranes.paper.is_none() {
            const PALETTE: [Color; 6] = [
                Color::Pink,
                Color::Blue,
                Color::Yellow,
                Color::Green,
                Color::Purple,
                Color::Orange,
            ];
            let color = if let Some(color) = self.life.viewer.color
                && self.random.unit() < 0.3
            {
                color
            } else {
                PALETTE[self.random.pick(&[1.0; PALETTE.len()]).unwrap()]
            };
            self.life.cranes.paper = Some(Paper {
                color,
                progress: 0.0,
            });
        }
        self.folding = Some(Folding {
            lighting: self.is_night() && !self.lamp.on,
            duration: self.random.range(20.0, 35.0),
        });
        if self.folding.as_ref().unwrap().lighting {
            self.ghost.plan.target = self.lamp.bulb() + Vec3::new(0.35, -0.15, 0.25);
        }
    }

    pub(super) fn stop_folding(&mut self) {
        // What the house shares is the host's to end.
        if self.companion {
            return;
        }
        self.folding = None;
        if self
            .life
            .cranes
            .paper
            .is_some_and(|paper| paper.progress >= 1.0)
        {
            self.hang_crane();
        }
    }

    fn hang_crane(&mut self) {
        if !self.can_fold()
            || !self
                .life
                .cranes
                .paper
                .is_some_and(|paper| paper.progress >= 1.0)
        {
            return;
        }
        let paper = self.life.cranes.paper.take().unwrap();
        let today = self.life.cranes.count_today(self.date);
        let day = self
            .life
            .cranes
            .day
            .as_deref()
            .and_then(|day| day.parse::<jiff::civil::Date>().ok())
            .map_or(self.date, |day| day.max(self.date));
        self.life.cranes.day = Some(day.to_string());
        self.life.cranes.today = today + 1;
        self.life.cranes.ready_hour = (self.hour + 3.0).min(24.0);
        self.life.cranes.total += 1;
        if self.life.cranes.colors.len() < VISIBLE {
            self.life.cranes.colors.push(paper.color);
        }
        let count = self.life.cranes.total;
        self.record(EventKind::FoldedCrane { count });
        self.ghost.needs.change(0.0, 0.04, 0.01);
        self.suggest_mood(Mood::Content, 8.0);
        if self.crane_for_sad_viewer() && self.viewer_present() {
            self.say(self.spoken("I folded a crane for you.", "너를 위해 종이학을 접었어."));
        }
        if count == 1000 {
            self.say(self.spoken(
                "A thousand cranes. My wish is for you to have gentle days.",
                "종이학 천 마리야. 네 하루하루가 다정하길 빌었어.",
            ));
            let korean = self.speaks_korean();
            let pairs = [
                ("Dear you,", "소중한 너에게,"),
                (
                    "A thousand little folds, all for you.",
                    "작은 종이학 천 마리를 모두 너를 위해 접었어.",
                ),
                (
                    "I wished for your days to be gentle, and for you to feel less alone.",
                    "네 하루가 다정하고, 네가 덜 외롭기를 빌었어.",
                ),
                (
                    "There is always a little light waiting for you here.",
                    "여기에는 언제나 너를 기다리는 작은 빛이 있어.",
                ),
            ];
            self.deliver(
                pairs
                    .into_iter()
                    .map(|(english, korean_line)| {
                        if korean { korean_line } else { english }.to_owned()
                    })
                    .collect(),
                korean,
            );
        }
    }

    pub(super) fn carry_out_fold(&mut self, seconds: f32) -> Option<Vec3> {
        let Some(folding) = &self.folding else {
            self.ghost.plan.done = true;
            return None;
        };
        if folding.lighting {
            if self.light_for_reading() {
                self.folding.as_mut().unwrap().lighting = false;
            }
            return Some(self.lamp.bulb());
        }
        match self.ghost.plan.count {
            0 if self.ghost.plan.arrived.is_some() => {
                let paper = self.life.cranes.paper.as_mut()?;
                paper.progress = (paper.progress + seconds / folding.duration).min(1.0);
                if paper.progress >= 1.0 {
                    self.ghost.plan.count = 1;
                    self.ghost.plan.arrived = None;
                    self.ghost.plan.target = slot(self.life.cranes.total as usize) + Vec3::Y * 0.2;
                }
            }
            1 if self.ghost.position.distance(self.ghost.plan.target) < super::ARRIVED => {
                self.hang_crane();
                self.ghost.plan.count = 2;
                self.ghost.plan.arrived = Some(self.time);
            }
            2 if self.ghost.plan.stayed(self.time) > 3.0 => {
                self.folding = None;
                self.ghost.plan.done = true;
            }
            _ => {}
        }
        Some(if self.ghost.plan.count == 0 {
            self.ghost.position - Vec3::Y * 0.3 * self.size()
        } else {
            slot(
                self.life
                    .cranes
                    .total
                    .saturating_sub(u32::from(self.ghost.plan.count == 2)) as usize,
            )
        })
    }

    pub fn held_paper(&self) -> Option<(Paper, Vec3)> {
        let folding = self.folding.as_ref()?;
        if folding.lighting || (self.ghost.plan.arrived.is_none() && self.ghost.plan.count == 0) {
            return None;
        }
        Some((self.life.cranes.paper?, self.ghost.position - Vec3::Y * 0.2))
    }

    pub fn folding_words(&self) -> &'static str {
        if self.folding.is_none() {
            return "no paper in your hands just now";
        }
        match self.life.cranes.paper.map(|paper| paper.progress) {
            Some(progress) if progress >= 1.0 => {
                "you are carrying a finished paper crane to its thread"
            }
            Some(progress) if progress > 0.75 => "you are folding a paper crane, nearly done",
            _ => "you are folding a colored square into a paper crane",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grown() -> World {
        let mut world = World::new(7, 18.0);
        world.begin_life(&world.date.to_string());
        world
    }

    fn finished_paper(world: &mut World) {
        world.life.cranes.paper = Some(Paper {
            color: Color::Blue,
            progress: 1.0,
        });
    }

    #[test]
    fn folding_requests_recognize_both_languages_but_not_questions_or_negation() {
        for text in [
            "fold a crane",
            "fold me a paper crane",
            "make a paper crane",
            "Please fold a paper crane!",
            "종이학 접어줘",
            "학 접어줘",
            "종이학 접어",
        ] {
            assert!(requested(text), "{text}");
        }
        for text in [
            "don't fold a crane",
            "do not make a paper crane",
            "fold a crane?",
            "can you fold a crane",
            "how many cranes have you folded?",
            "종이학 접지 마",
            "종이학 접어?",
            "종이학 접었어",
            "I will fold a crane",
            "make a crane fly",
        ] {
            assert!(!requested(text), "{text}");
        }
    }

    #[test]
    fn newborns_do_not_fold_and_requests_never_exceed_three_per_local_day() {
        let mut world = grown();
        world.birthday = Some(world.date);
        assert!(!world.options().contains(&Activity::Fold));
        world.hear("fold a crane");
        world.begin(Activity::Fold);
        assert_ne!(world.ghost.plan.activity, Activity::Fold);
        world.birthday = Some(world.date.checked_sub(jiff::Span::new().days(3)).unwrap());
        for count in 1..=3 {
            finished_paper(&mut world);
            world.hang_crane();
            assert_eq!(world.life.cranes.total, count);
        }
        let heard = world.hear("종이학 접어줘");
        assert_eq!(heard.understanding, Understanding::Folding);
        assert_eq!(
            world.ghost.speech.as_ref().unwrap().text,
            "오늘은 손이 지쳤어. 내일 또 접을게."
        );
        finished_paper(&mut world);
        world.hang_crane();
        assert_eq!(world.life.cranes.total, 3);
        assert!(!world.fold_is_open());
        world.date = world.date.tomorrow().unwrap();
        world.begin_day();
        world.hang_crane();
        assert_eq!(world.life.cranes.total, 4);
        assert_eq!(world.life.cranes.today, 1);
    }

    #[test]
    fn requests_teach_no_words_and_wait_through_sleep_work_and_stories() {
        for condition in 0..3 {
            let mut world = grown();
            match condition {
                0 => {
                    world.hour = 23.0;
                    world.tuck_in();
                }
                1 => world.start_focus(25),
                _ => world.tell(vec!["A little bird came home.".to_owned()]),
            }
            let activity = world.ghost.plan.activity;
            let vocabulary = serde_json::to_value(&world.life.vocabulary).unwrap();
            assert_eq!(
                world.hear("fold a crane").understanding,
                Understanding::Folding
            );
            assert_eq!(world.ghost.plan.activity, activity);
            assert_eq!(
                serde_json::to_value(&world.life.vocabulary).unwrap(),
                vocabulary
            );
            assert!(world.pending_crane);
            world.focus = None;
            world.telling = None;
            world.ghost.asleep = 0.0;
            world.ghost.plan.done = true;
            world.advance_cranes();
            assert_eq!(world.ghost.plan.activity, Activity::Fold);
            assert!(!world.pending_crane);
        }
    }

    #[test]
    fn partial_paper_and_daily_allowance_survive_interruptions_and_saving() {
        let mut world = grown();
        world.begin(Activity::Fold);
        world.ghost.position = SEAT;
        world.ghost.plan.arrived = Some(world.time);
        world.carry_out_fold(10.0);
        let paper = world.life.cranes.paper.unwrap();
        assert!(paper.progress > 0.0 && paper.progress < 1.0);
        let allowance = world.life.cranes.allowance;
        world.begin(Activity::Visit);
        assert!(world.folding.is_none());
        assert_eq!(world.life.cranes.paper, Some(paper));
        let saved = world.saved_life();
        let mut restored = grown();
        restored.life =
            serde_json::from_str::<super::super::Life>(&serde_json::to_string(&saved).unwrap())
                .unwrap()
                .sanitized();
        restored.begin(Activity::Fold);
        assert_eq!(restored.life.cranes.paper, Some(paper));
        assert_eq!(restored.life.cranes.allowance, allowance);
    }

    #[test]
    fn interrupted_and_restored_finished_paper_hangs_exactly_once() {
        let mut world = grown();
        world.begin(Activity::Fold);
        finished_paper(&mut world);
        world.begin(Activity::Visit);
        world.stop_folding();
        assert_eq!(world.life.cranes.total, 1);
        assert!(world.life.cranes.paper.is_none());
        world.begin(Activity::Fold);
        finished_paper(&mut world);
        let saved = world.saved_life();
        let mut restored = grown();
        restored.life = saved.sanitized();
        restored.advance_cranes();
        restored.advance_cranes();
        assert_eq!(restored.life.cranes.total, 2);
        assert_eq!(restored.life.total.cranes, 2);
    }

    #[test]
    fn folding_waits_for_arrival_lights_the_lamp_and_flies_to_hang_the_crane() {
        let mut world = grown();
        world.hour = 21.0;
        world.lamp.on = false;
        world.begin(Activity::Fold);
        world.carry_out_fold(5.0);
        assert_eq!(world.life.cranes.paper.unwrap().progress, 0.0);
        for _ in 0..2000 {
            world.notice_viewer();
            world.step(0.1);
            if world.ghost.plan.done {
                break;
            }
        }
        assert!(world.lamp.on);
        assert_eq!(
            world.life.cranes.total, 1,
            "{:?} {:?}",
            world.ghost.plan, world.ghost.position
        );
        assert!(world.ghost.plan.done);
        assert!(world.life.cranes.paper.is_none());
        assert_eq!(world.life.today.cranes, 1);
    }

    #[test]
    fn all_slots_fill_threads_in_order_and_clear_the_room_furniture() {
        let world = grown();
        for index in 0..VISIBLE {
            let position = slot(index);
            let radius = 0.055;
            assert!(
                (position - Vec3::splat(radius))
                    .cmpge(super::super::ROOM_MIN)
                    .all()
            );
            assert!(
                (position + Vec3::splat(radius))
                    .cmple(super::super::ROOM_MAX)
                    .all()
            );
            let mirror = super::super::mirror::CENTER;
            assert!(
                (position.z - mirror.z).abs()
                    > super::super::mirror::WIDTH * 0.5
                        + super::super::mirror::FRAME_WIDTH
                        + radius
            );
            // The window glass and curtains end well left of the garland.
            assert!(position.x - radius > 1.0);
            let plant_top = world.plant.position.y + 1.06 + 0.28;
            assert!(position.y - radius > plant_top);
            assert!(position.y - radius > 0.68);
            if index % 10 > 0 {
                assert_eq!(position.x, slot(index - 1).x);
                assert_eq!(position.z, slot(index - 1).z);
                assert!(position.y < slot(index - 1).y);
            } else if index > 0 {
                assert_ne!(position, slot(index - 10));
                assert_eq!(position.y, slot(0).y);
            }
        }
        assert_eq!(slot(VISIBLE + 100), slot(VISIBLE - 1));
    }

    #[test]
    fn milestones_happen_only_at_the_six_crane_landmarks_and_use_words() {
        let mut life = super::super::Life::default();
        for count in 1..=1001 {
            let milestone = life.note(&EventKind::FoldedCrane { count });
            assert_eq!(milestone.is_some(), LANDMARKS.contains(&count), "{count}");
            if let Some(text) = milestone {
                assert!(!text.chars().any(|letter| letter.is_ascii_digit()));
            }
        }
        assert_eq!(life.milestones.len(), 6);
        assert!(life.note(&EventKind::FoldedCrane { count: 1000 }).is_none());
        assert_eq!(
            EventKind::FoldedCrane { count: 12 }.words(),
            "folded the twelfth paper crane"
        );
    }

    #[test]
    fn the_thousandth_crane_delivers_one_wish_letter_in_the_viewers_language() {
        for korean in [false, true] {
            let mut world = grown();
            world.life.korean = if korean { 1.0 } else { 0.0 };
            world.life.cranes.total = 999;
            finished_paper(&mut world);
            world.hang_crane();
            assert_eq!(world.life.letters.len(), 1);
            assert_eq!(world.life.letters[0].korean, korean);
            assert!(world.life.letters[0].valid());
            finished_paper(&mut world);
            world.hang_crane();
            world.stop_folding();
            let mut restored = grown();
            restored.life = world.saved_life().sanitized();
            restored.advance_cranes();
            assert_eq!(restored.life.cranes.total, 1001);
            assert_eq!(restored.life.letters.len(), 1);
        }
    }

    #[test]
    fn daily_pacing_has_a_saved_allowance_a_long_pause_and_slows_after_a_thousand() {
        let mut world = grown();
        world.begin(Activity::Fold);
        finished_paper(&mut world);
        world.hang_crane();
        world.folding = None;
        world.life.cranes.allowance = 2;
        assert!(!world.fold_is_open());
        world.hour += 3.0;
        assert!(world.fold_is_open());
        finished_paper(&mut world);
        world.hang_crane();
        world.hour = 23.9;
        assert!(!world.fold_is_open());
        world.life.cranes.total = 1000;
        world.date = world.date.tomorrow().unwrap();
        assert!(!world.fold_is_open());
        world.date = world.date.checked_add(jiff::Span::new().days(2)).unwrap();
        assert!(world.fold_is_open());
    }

    #[test]
    fn old_and_damaged_saved_cranes_are_defaulted_bounded_and_repaired() {
        let mut life: super::super::Life = serde_json::from_str("{}").unwrap();
        assert_eq!(life.cranes, Cranes::default());
        life.day = "2026-06-21".to_owned();
        life.cranes = Cranes {
            total: u32::MAX,
            colors: vec![Color::Pink; 100],
            day: Some("damaged".to_owned()),
            today: u32::MAX,
            allowance: u32::MAX,
            ready_hour: f32::INFINITY,
            paper: Some(Paper {
                color: Color::Green,
                progress: f32::NAN,
            }),
            allowance_day: Some("damaged".to_owned()),
        };
        life.today.cranes = u32::MAX;
        life.today.cranes_for_sad_viewer = u32::MAX;
        life = life.sanitized();
        assert_eq!(life.cranes.total, MAXIMUM);
        assert_eq!(life.cranes.colors.len(), VISIBLE);
        assert!(life.cranes.day.is_none());
        assert_eq!(life.cranes.today, 0);
        assert!(life.cranes.paper.is_none());
        assert_eq!(life.today.cranes, 3);
        life.cranes.day = Some("2027-06-21".to_owned());
        life.cranes.paper = Some(Paper {
            color: Color::Green,
            progress: -3.0,
        });
        life = life.sanitized();
        assert!(life.cranes.day.is_none());
        assert_eq!(life.cranes.paper.unwrap().progress, 0.0);
    }

    #[test]
    fn the_garland_keeps_only_its_first_eighty_colors_but_counts_every_crane() {
        let mut world = grown();
        for _ in 0..40 {
            for _ in 0..3 {
                finished_paper(&mut world);
                world.hang_crane();
            }
            world.date = world.date.tomorrow().unwrap();
        }
        assert_eq!(world.life.cranes.total, 120);
        assert_eq!(world.life.cranes.colors, vec![Color::Blue; VISIBLE]);
    }

    #[test]
    fn a_crane_on_a_sad_day_counts_as_care_in_the_diary() {
        let mut world = grown();
        world.hear("I'm sad");
        finished_paper(&mut world);
        world.hang_crane();
        assert_eq!(world.life.today.cranes_for_sad_viewer, 1);
        assert!(
            world
                .life
                .today
                .diary()
                .contains("folded a crane for you when you were sad")
        );
    }

    #[test]
    fn crane_questions_and_korean_counts_are_answered_without_learning_words() {
        let mut world = grown();
        world.life.cranes.total = 12;
        for text in ["how many cranes have you folded?", "종이학 몇 마리 접었어?"] {
            let heard = world.hear(text);
            assert!(matches!(
                heard.understanding,
                Understanding::Asked {
                    question: super::super::Question::Cranes,
                    ..
                }
            ));
            assert!(heard.learned.is_empty());
        }
        for (count, words) in [
            (1, "한"),
            (12, "열두"),
            (20, "스무"),
            (21, "스물한"),
            (100, "백"),
            (1000, "천"),
            (100_000, "십만"),
        ] {
            assert_eq!(korean_count(count), words);
        }
    }

    #[test]
    fn restarting_a_finished_third_fold_does_not_start_a_fourth_paper() {
        let mut world = grown();
        world.begin(Activity::Fold);
        world.life.cranes.total = 2;
        world.life.cranes.day = Some(world.date.to_string());
        world.life.cranes.today = 2;
        finished_paper(&mut world);
        world.begin(Activity::Fold);
        assert_eq!(world.life.cranes.total, 3);
        assert!(world.life.cranes.paper.is_none());
        assert!(world.folding.is_none());
        assert!(world.ghost.plan.done);
    }
}
