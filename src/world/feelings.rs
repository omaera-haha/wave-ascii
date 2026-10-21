use glam::Vec3;
use jiff::civil::Date;
use serde::{Deserialize, Serialize};

use super::{
    Activity, EventKind, Heard, Listening, Mood, Picture, Sketch, Tone, Understanding, World,
    vocabulary,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Feeling {
    Tired,
    Sad,
    Lonely,
    Stressed,
    Angry,
    Unwell,
    Bored,
    Happy,
    Proud,
}

impl Feeling {
    pub fn name(self) -> &'static str {
        match self {
            Self::Tired => "tired",
            Self::Sad => "sad",
            Self::Lonely => "lonely",
            Self::Stressed => "stressed",
            Self::Angry => "angry",
            Self::Unwell => "unwell",
            Self::Bored => "bored",
            Self::Happy => "happy",
            Self::Proud => "proud",
        }
    }

    pub fn korean(self) -> &'static str {
        match self {
            Self::Tired => "피곤해",
            Self::Sad => "슬퍼",
            Self::Lonely => "외로워",
            Self::Stressed => "불안해",
            Self::Angry => "화나",
            Self::Unwell => "아파",
            Self::Bored => "심심해",
            Self::Happy => "행복해",
            Self::Proud => "뿌듯해",
        }
    }

    pub fn negative(self) -> bool {
        !matches!(self, Self::Happy | Self::Proud)
    }

    pub fn care_words(self) -> &'static str {
        match self {
            Self::Tired | Self::Unwell => "kept you quiet company",
            Self::Sad | Self::Lonely => "drew you a heart",
            Self::Stressed | Self::Angry => "breathed together with you",
            Self::Bored => "invited you to play",
            Self::Happy | Self::Proud => "celebrated with you",
        }
    }
}

/// Only complete, first-person statements count. In particular a question,
/// negation, quoted feeling, or someone else's subject supplies no evidence.
// Whether `text` says no to something, in English or Korean.
fn denies(text: &str) -> bool {
    let text = text.to_lowercase().replace(['\'', '’', '‘'], "");
    text.split(|character: char| !character.is_alphabetic())
        .any(|word| {
            matches!(
                word,
                "not" | "never" | "dont" | "isnt" | "arent" | "no" | "nope" | "안" | "못" | "별로"
            ) || word.contains("않")
                || word.contains("아니")
        })
}

pub fn felt(text: &str) -> Option<Feeling> {
    if text.contains(['?', '？', '"', '“', '”']) || super::question::ask(text).is_some() {
        return None;
    }
    let text = text.to_lowercase().replace(['\'', '’', '‘'], "");
    let words: Vec<&str> = text
        .split(|character: char| !character.is_alphabetic())
        .filter(|word| !word.is_empty())
        .collect();
    if words.iter().any(|word| {
        matches!(
            *word,
            "not" | "never" | "dont" | "isnt" | "arent" | "안" | "못"
        ) || word.contains("않")
            || word.contains("아니")
    }) {
        return None;
    }
    let mut statement = words.as_slice();
    if matches!(statement, ["i", "did", "it"]) {
        return Some(Feeling::Proud);
    }
    if matches!(
        statement,
        ["rough" | "long" | "bad", "day"]
            | ["today", "was", "hard"]
            | ["i", "had", "a", "rough" | "long" | "bad", "day"]
    ) {
        return Some(Feeling::Tired);
    }
    if matches!(
        statement,
        ["good" | "great", "day"] | ["i", "had", "a", "good" | "great", "day"]
    ) {
        return Some(Feeling::Happy);
    }
    if matches!(statement, ["i", "have", "a", "headache"]) {
        return Some(Feeling::Unwell);
    }
    statement = match statement {
        ["im" | "feeling", rest @ ..] | ["i", "am" | "feel", rest @ ..] => rest,
        _ => return korean_feeling(statement),
    };
    if let ["feeling", rest @ ..] = statement {
        statement = rest;
    }
    while let [
        "so" | "very" | "really" | "pretty" | "just" | "quite",
        rest @ ..,
    ] = statement
    {
        statement = rest;
    }
    let (&word, ending) = statement.split_first()?;
    if !matches!(
        ending,
        [] | ["today"] | ["now"] | ["right", "now"] | ["out"] | ["out", "today"]
    ) {
        return None;
    }
    Some(match word {
        "tired" | "exhausted" | "sleepy" => Feeling::Tired,
        "sad" | "down" | "depressed" | "upset" => Feeling::Sad,
        "lonely" => Feeling::Lonely,
        "stressed" | "anxious" | "worried" | "nervous" => Feeling::Stressed,
        "angry" | "annoyed" | "frustrated" => Feeling::Angry,
        "sick" | "ill" | "unwell" => Feeling::Unwell,
        "bored" => Feeling::Bored,
        "happy" | "excited" | "great" => Feeling::Happy,
        "proud" => Feeling::Proud,
        _ => return None,
    })
}

fn korean_feeling(mut words: &[&str]) -> Option<Feeling> {
    let mood = words.contains(&"기분") || words.contains(&"기분이");
    while let [
        "나" | "나는" | "난" | "저" | "저는" | "제가" | "오늘" | "오늘은" | "너무" | "진짜"
        | "정말" | "좀" | "조금" | "기분" | "기분이",
        rest @ ..,
    ] = words
    {
        words = rest;
    }
    if matches!(
        words,
        ["좋은", "하루"] | ["좋은", "하루였어"] | ["좋은", "하루였어요"]
    ) {
        return Some(Feeling::Happy);
    }
    if matches!(words, ["힘든", "하루"] | ["힘든", "하루였어"]) {
        return Some(Feeling::Tired);
    }
    if matches!(
        words,
        [
            "스트레스",
            "받아" | "받아요" | "받았어" | "받았어요" | "받네" | "받네요"
        ]
    ) {
        return Some(Feeling::Stressed);
    }
    if matches!(words, ["감기" | "감기에", "걸렸어" | "걸렸어요"]) {
        return Some(Feeling::Unwell);
    }
    if matches!(words, ["머리" | "머리가", ..]) {
        words = &words[1..];
    }
    let [word] = words else {
        return None;
    };
    let word = word.strip_suffix('요').unwrap_or(word);
    let groups: &[(Feeling, &[&str])] = &[
        (
            Feeling::Tired,
            &[
                "피곤해",
                "피곤하다",
                "피곤하네",
                "피곤하네요",
                "지쳤어",
                "지쳤다",
                "지쳤네",
                "졸려",
                "졸리다",
                "졸리네",
                "힘들었어",
                "힘들었다",
                "힘들어",
                "힘들다",
                "힘드네",
            ],
        ),
        (
            Feeling::Sad,
            &[
                "슬퍼",
                "슬프다",
                "슬프네",
                "우울해",
                "우울하다",
                "우울하네",
                "속상해",
                "속상하다",
                "속상하네",
            ],
        ),
        (Feeling::Lonely, &["외로워", "외롭다", "외롭네"]),
        (
            Feeling::Stressed,
            &[
                "스트레스",
                "불안해",
                "불안하다",
                "불안하네",
                "걱정돼",
                "걱정된다",
                "걱정되네",
                "긴장돼",
                "긴장된다",
                "긴장되네",
            ],
        ),
        (
            Feeling::Angry,
            &["화나", "화난다", "화나네", "짜증나", "짜증난다", "짜증나네"],
        ),
        (Feeling::Unwell, &["아파", "아프다", "아프네", "감기"]),
        (Feeling::Bored, &["심심해", "심심하다", "심심하네"]),
        (
            Feeling::Happy,
            &[
                "행복해",
                "행복하다",
                "행복하네",
                "기뻐",
                "기쁘다",
                "기쁘네",
                "신나",
                "신난다",
                "신나네",
            ],
        ),
        (
            Feeling::Proud,
            &[
                "해냈어",
                "해냈다",
                "해냈네",
                "뿌듯해",
                "뿌듯하다",
                "뿌듯하네",
            ],
        ),
    ];
    if mood && matches!(word, "좋아" | "좋다" | "좋네") {
        return Some(Feeling::Happy);
    }
    groups
        .iter()
        .find_map(|(feeling, forms)| forms.contains(&word).then_some(*feeling))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeelingNote {
    pub feeling: Feeling,
    pub day: String,
    pub hour: f32,
    #[serde(default)]
    pub asked: bool,
}

impl FeelingNote {
    pub fn days_ago(&self, today: Date) -> Option<i32> {
        let day = self.day.parse::<Date>().ok()?;
        (day.to_string() == self.day).then(|| (today - day).get_days())
    }

    pub fn valid(&self, today: &str) -> bool {
        today
            .parse::<Date>()
            .ok()
            .and_then(|today| self.days_ago(today))
            .is_some_and(|days| days >= 0)
            && self.hour.is_finite()
            && (0.0..24.0).contains(&self.hour)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Comforting {
    pub feeling: Feeling,
    pub korean: bool,
    pub cycles: u32,
    cue: u32,
    drawing_finished: bool,
    lullaby: bool,
    invite: bool,
}

/// One slow breath: four seconds in, six out, with eased turns.
pub fn breath(elapsed: f32) -> f32 {
    let phase = elapsed.max(0.0) % 10.0;
    let fraction = if phase < 4.0 {
        phase / 4.0
    } else {
        1.0 - (phase - 4.0) / 6.0
    };
    fraction * fraction * (3.0 - 2.0 * fraction)
}

impl World {
    pub fn recent_feeling(&self, seconds: f64) -> Option<Feeling> {
        let note = self.life.feelings.last()?;
        let age = self.feeling_at.map_or_else(
            || {
                // The clock has moved on with the world since restoring.
                let days = note.days_ago(self.date)?;
                Some(f64::from(days) * 86400.0 + f64::from(self.hour - note.hour) * 3600.0)
            },
            |at| Some(self.time - at),
        )?;
        (0.0..=seconds).contains(&age).then_some(note.feeling)
    }

    pub fn recent_negative_feeling(&self, seconds: f64) -> Option<Feeling> {
        self.recent_feeling(seconds)
            .filter(|feeling| feeling.negative())
    }

    pub fn feeling_words(&self) -> String {
        let mut words = self
            .recent_feeling(3600.0)
            .map_or_else(String::new, |feeling| {
                let mut words = format!(
                    "The viewer said a little while ago that they feel {}.",
                    feeling.name()
                );
                if self.latest_korean {
                    words.push_str(&format!(" They said “{}”.", feeling.korean()));
                }
                words
            });
        if let Some(comforting) = &self.comforting {
            words.push_str(&format!(
                " The ghost is comforting them: it {}.",
                comforting.feeling.care_words()
            ));
        }
        if self.feeling_follow_up.is_some() {
            words.push_str(" Waiting to hear whether the viewer feels better today.");
        }
        words
    }

    fn feeling_heard(&self, understanding: Understanding, korean: bool) -> Heard {
        Heard {
            tone: None,
            understanding,
            named: false,
            learned: Vec::new(),
            stopped: false,
            learned_name: None,
            given_name: None,
            korean,
        }
    }

    pub(super) fn hear_feelings(
        &mut self,
        text: &str,
        korean: bool,
        follow_up: Option<(Feeling, f64)>,
    ) -> Option<Heard> {
        let follow_up = follow_up.filter(|(_, until)| self.time <= *until);
        let feeling = felt(text);
        if let Some((earlier, _)) = follow_up {
            let tone = vocabulary::read(text, &self.ghost.name).tone;
            let negative = feeling.is_some_and(Feeling::negative);
            // "Not any better" carries a praise word, but it is no recovery.
            if feeling.is_none() && denies(text) {
                return Some(self.receive_feeling(text, earlier, korean));
            }
            if !negative
                && (feeling.is_some() || matches!(tone, Some(Tone::Praise | Tone::Greeting)))
            {
                self.listening = Listening::default();
                self.notice_viewer();
                self.begin_day();
                self.record(EventKind::Heard(text.to_owned()));
                if let Some(feeling) = feeling {
                    self.remember_feeling(feeling);
                }
                self.record(EventKind::FeltBetter);
                if self.ghost.asleep < 0.3 {
                    self.say(if korean || self.speaks_korean() {
                        "다행이야. 정말 기뻐!"
                    } else {
                        "I'm so glad you're feeling better!"
                    });
                }
                return Some(self.feeling_heard(Understanding::FeltBetter, korean));
            }
            if feeling.is_none() && tone == Some(Tone::Scolding) {
                return Some(self.receive_feeling(text, earlier, korean));
            }
        }
        feeling.map(|feeling| self.receive_feeling(text, feeling, korean))
    }

    fn receive_feeling(&mut self, text: &str, feeling: Feeling, korean: bool) -> Heard {
        self.begin_day();
        self.listening = Listening::default();
        self.notice_viewer();
        self.record(EventKind::Heard(text.to_owned()));
        self.remember_feeling(feeling);
        self.comfort_viewer(feeling, korean || self.speaks_korean());
        self.feeling_heard(Understanding::Felt(feeling), korean)
    }

    pub(super) fn remember_feeling(&mut self, feeling: Feeling) {
        self.life.feelings.push(FeelingNote {
            feeling,
            day: self.date.to_string(),
            hour: self.hour,
            asked: false,
        });
        if self.life.feelings.len() > 12 {
            self.life.feelings.remove(0);
        }
        self.feeling_at = Some(self.time);
        self.record(EventKind::ViewerFelt(feeling));
    }

    fn comfort_viewer(&mut self, feeling: Feeling, korean: bool) {
        if self.ghost.asleep >= 0.3 || self.focus.is_some() || self.telling.is_some() {
            return;
        }
        let invite = feeling == Feeling::Bored
            && self.invitations_welcome()
            && !self.puzzle_active()
            && self.game.as_ref().is_none_or(|game| game.outcome.is_some());
        if feeling == Feeling::Bored && !invite {
            self.begin(Activity::Visit);
            self.prepare_comfort();
        } else {
            self.begin(Activity::Comfort);
        }
        if let Some(comforting) = &mut self.comforting {
            comforting.korean = korean;
            comforting.invite = invite;
        }
    }

    pub(super) fn prepare_comfort(&mut self) {
        let Some(feeling) = self.recent_feeling(600.0) else {
            return;
        };
        let cycles = 4 + (self.random.unit() * 3.0) as u32;
        self.ghost.plan.length = match feeling {
            Feeling::Stressed | Feeling::Angry => cycles as f32 * 10.0,
            Feeling::Tired | Feeling::Unwell => self.random.range(30.0, 60.0),
            Feeling::Sad | Feeling::Lonely => self.random.range(20.0, 40.0),
            Feeling::Bored => 6.0,
            Feeling::Happy | Feeling::Proud => 6.0,
        };
        if matches!(feeling, Feeling::Sad | Feeling::Lonely) {
            self.ghost.plan.picture = Some(Picture::Sketch(if self.random.unit() < 0.5 {
                Sketch::Heart
            } else {
                Sketch::Sun
            }));
        }
        self.ghost.attention = None;
        self.comforting = Some(Comforting {
            feeling,
            korean: self.latest_korean || self.speaks_korean(),
            cycles,
            cue: 0,
            drawing_finished: false,
            invite: false,
            lullaby: matches!(feeling, Feeling::Tired | Feeling::Unwell)
                && self.hour >= 19.0
                && self.music.is_none()
                && self.random.unit() < 0.5,
        });
        self.suggest_mood(
            if feeling.negative() {
                Mood::Content
            } else {
                Mood::Excited
            },
            65.0,
        );
    }

    pub fn comfort_breath(&self) -> Option<f32> {
        self.comforting
            .as_ref()
            .filter(|comforting| matches!(comforting.feeling, Feeling::Stressed | Feeling::Angry))
            .filter(|_| {
                self.ghost.plan.activity == Activity::Comfort
                    && !self.ghost.plan.done
                    && self.ghost.plan.arrived.is_some()
            })
            .map(|_| breath(self.ghost.plan.stayed(self.time)))
    }

    pub fn quiet_comfort(&self) -> bool {
        self.comforting.as_ref().is_some_and(|comforting| {
            matches!(comforting.feeling, Feeling::Tired | Feeling::Unwell)
        }) && self.ghost.plan.activity == Activity::Comfort
            && !self.ghost.plan.done
    }

    pub(super) fn abandon_comfort(&mut self) {
        if self.comforting.take().is_some()
            && matches!(
                self.ghost.plan.activity,
                Activity::Comfort | Activity::Visit
            )
        {
            self.ghost.plan.done = true;
        }
    }

    pub(super) fn carry_out_comfort(&mut self) -> Option<Vec3> {
        if !self.viewer_present() || self.ghost.asleep >= 0.3 {
            self.abandon_comfort();
            return None;
        }
        let mut comforting = self.comforting.take()?;
        let anchor = self.ghost.plan.anchor;
        let look = anchor + Vec3::new(0.0, 0.3, 3.0);
        if self.ghost.plan.arrived.is_none() {
            self.comforting = Some(comforting);
            return Some(look);
        }
        let elapsed = self.ghost.plan.stayed(self.time);
        let say = |english, korean| if comforting.korean { korean } else { english };
        if comforting.cue == 0 {
            self.say(match comforting.feeling {
                Feeling::Stressed | Feeling::Angry => say("Breathe with me…", "같이 숨 쉬자…"),
                Feeling::Unwell => say("Rest well, okay?", "푹 쉬어, 알겠지?"),
                Feeling::Tired => say("I'll sit quietly with you.", "조용히 곁에 있을게."),
                Feeling::Sad | Feeling::Lonely if self.life.temperament().tender >= 0.6 => {
                    say("I'm here.", "내가 여기 있어.")
                }
                Feeling::Sad | Feeling::Lonely => {
                    say("A little heart for you.", "널 위한 작은 하트야.")
                }
                Feeling::Bored => say("Want to play with me?", "나랑 같이 놀래?"),
                Feeling::Happy => say("Your happiness makes me glow!", "네가 행복하니 나도 빛나!"),
                Feeling::Proud => say("You did it! I knew you could!", "해냈구나! 대단해!"),
            });
            comforting.cue = 1;
        }
        match comforting.feeling {
            Feeling::Stressed | Feeling::Angry => {
                let cycle = (elapsed / 10.0) as u32;
                let out = elapsed % 10.0 >= 4.0;
                let cue = 2 + cycle * 2 + u32::from(out);
                if cycle < 2 && cue > comforting.cue && elapsed >= 1.5 {
                    self.say(if out {
                        say("out…", "내쉬고…")
                    } else {
                        say("in…", "들이쉬고…")
                    });
                    comforting.cue = cue;
                }
                self.ghost.plan.target = anchor + Vec3::Y * (breath(elapsed) * 0.10);
            }
            Feeling::Sad | Feeling::Lonely if !comforting.drawing_finished => {
                let drawing_look = self.carry_out_doodle();
                comforting.drawing_finished = self.ghost.plan.done;
                self.ghost.plan.done = false;
                self.comforting = Some(comforting);
                return drawing_look;
            }
            Feeling::Happy | Feeling::Proud => {
                self.ghost.plan.target = anchor
                    + super::growth::dance_offset(
                        elapsed / 6.0,
                        self.life.skills.level(super::Skill::Dancing),
                    ) * 0.35;
            }
            _ => {
                self.ghost.plan.target = anchor + Vec3::Y * (elapsed * 0.45).sin() * 0.025;
            }
        }
        if elapsed >= self.ghost.plan.length {
            self.ghost.plan.done = true;
            if !comforting.feeling.negative() {
                self.record(EventKind::Danced);
            }
            self.record(EventKind::Comforted(comforting.feeling));
            if comforting.lullaby && self.music.is_none() && self.focus.is_none() {
                self.say(say("Shall I play you a lullaby?", "자장가를 들려줄까?"));
                self.play_comfort_lullaby();
            } else if comforting.invite
                && self.invitations_welcome()
                && self.game.as_ref().is_none_or(|game| game.outcome.is_some())
                && !self.puzzle_active()
            {
                let options = self.options();
                let choices: Vec<Activity> = [Activity::Game, Activity::Puzzle]
                    .into_iter()
                    .filter(|activity| options.contains(activity))
                    .collect();
                if let Some(index) = self.pick(&vec![1.0; choices.len()]) {
                    self.begin(choices[index]);
                }
            }
        } else {
            self.comforting = Some(comforting);
        }
        Some(look)
    }

    pub(super) fn follow_up_feeling(&mut self) {
        if !self.viewer_present() {
            self.feeling_visit_asked = false;
            return;
        }
        if self
            .feeling_follow_up
            .is_some_and(|(_, until)| self.time > until)
        {
            self.feeling_follow_up = None;
        }
        if self.feeling_visit_asked
            || self.feeling_follow_up.is_some()
            || self.asking.is_some()
            || self.ghost.asleep >= 0.3
            || self.focus.is_some()
            || self.telling.is_some()
            || self.puzzle_active()
            || self
                .game
                .as_ref()
                .is_some_and(|game| game.outcome.is_none())
            || self.viewer.typing
        {
            return;
        }
        let Some(index) = self.life.feelings.len().checked_sub(1) else {
            return;
        };
        let note = &self.life.feelings[index];
        if !note.feeling.negative()
            || note.asked
            || !note
                .days_ago(self.date)
                .is_some_and(|days| (1..=3).contains(&days))
        {
            return;
        }
        if self.ghost.plan.done || self.ghost.plan.activity == Activity::Float {
            self.begin(Activity::Visit);
        }
        if self.ghost.plan.activity != Activity::Visit
            || self.ghost.plan.done
            || !self
                .ghost
                .plan
                .arrived
                .is_some_and(|at| self.time - at >= 3.0)
            || self
                .ghost
                .speech
                .as_ref()
                .is_some_and(|line| self.time - line.time < 12.0)
        {
            return;
        }
        let note = &mut self.life.feelings[index];
        note.asked = true;
        self.feeling_follow_up = Some((note.feeling, self.time + 180.0));
        self.feeling_visit_asked = true;
        self.last_question_at = Some(self.time);
        self.ghost.plan.length = self
            .ghost
            .plan
            .length
            .max(self.ghost.plan.stayed(self.time) + 60.0);
        self.say(self.spoken("Are you feeling better today?", "오늘은 좀 괜찮아?"));
    }
}

/// Memories for Claude, with relative days rather than numeric dates.
pub fn memories(notes: &[FeelingNote], day: &str) -> String {
    let Ok(today) = day.parse::<Date>() else {
        return String::new();
    };
    notes
        .iter()
        .filter_map(|note| {
            let when = match note.days_ago(today)? {
                0 => "today",
                1 => "yesterday",
                2 => "two days ago",
                3 => "three days ago",
                _ => return None,
            };
            Some(format!("The viewer felt {} {when}.", note.feeling.name()))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn replies(world: &World, feeling: Feeling, korean: bool) -> Vec<String> {
    let pairs: &[(&str, &str)] = match feeling {
        Feeling::Tired => &[
            ("You can slow down here.", "여기서는 천천히 쉬어도 돼."),
            ("I'll keep you quiet company.", "조용히 곁에 있을게."),
            ("A little rest with me?", "나랑 잠깐 쉴래?"),
        ],
        Feeling::Sad => &[
            ("That sounds hard. I'm here.", "힘들었겠다. 내가 여기 있어."),
            ("A little heart for you.", "널 위한 작은 하트야."),
            (
                "You don't have to smile for me.",
                "내 앞에서는 억지로 웃지 않아도 돼.",
            ),
        ],
        Feeling::Lonely => &[
            ("I'm here with you.", "내가 곁에 있어."),
            (
                "There's room beside my glass.",
                "내 유리창 옆에 네 자리가 있어.",
            ),
            ("We can sit together.", "같이 앉아 있자."),
        ],
        Feeling::Stressed => &[
            ("One slow breath with me?", "나랑 천천히 숨 쉴래?"),
            ("We can take a quiet moment.", "잠깐 조용히 쉬자."),
            (
                "I'll stay close. Breathe slowly.",
                "곁에 있을게. 천천히 숨 쉬자.",
            ),
        ],
        Feeling::Angry => &[
            ("That sounds frustrating.", "정말 답답했겠다."),
            (
                "I'll sit with you while it settles.",
                "마음이 가라앉을 때까지 곁에 있을게.",
            ),
            ("Let's breathe for a little while.", "잠깐 같이 숨 쉬자."),
        ],
        Feeling::Unwell => &[
            ("Rest well, okay?", "푹 쉬어, 알겠지?"),
            ("I'll keep things quiet here.", "여기는 조용히 해 둘게."),
            ("A soft little light for you.", "널 위해 작은 빛을 켜 둘게."),
        ],
        Feeling::Bored => &[
            ("We could play something!", "우리 뭐 하고 놀까!"),
            ("Want a little game?", "작은 놀이 하나 할래?"),
            ("I have a few tiny ideas.", "재밌는 생각이 몇 개 있어."),
        ],
        Feeling::Happy => &[
            ("Your happiness makes me glow!", "네가 행복하니 나도 빛나!"),
            ("I'm happy with you!", "나도 같이 행복해!"),
            ("A happy day for us!", "우리에게 행복한 날이야!"),
        ],
        Feeling::Proud => &[
            ("You did it! I knew you could!", "해냈구나! 대단해!"),
            ("Look at you! I'm cheering!", "정말 멋져! 응원하고 있어!"),
            ("That deserves a little twirl!", "축하하는 춤을 춰야겠어!"),
        ],
    };
    let mut lines: Vec<String> = pairs
        .iter()
        .map(|(english, hangul)| if korean { *hangul } else { *english }.to_owned())
        .collect();
    let nature = world.life.temperament();
    let extra = if nature.tender >= 0.65 {
        if feeling.negative() {
            Some((
                "You matter to me. I'll stay.",
                "너는 내게 소중해. 곁에 있을게.",
            ))
        } else {
            Some((
                "I'm so glad you shared this with me.",
                "나와 나눠 줘서 정말 기뻐.",
            ))
        }
    } else if nature.shy >= 0.65 {
        Some(if feeling.negative() {
            (
                "If you'd like… I'll sit nearby.",
                "괜찮다면… 옆에 앉아 있을게.",
            )
        } else {
            ("A tiny cheer… just for you.", "널 위해… 조그맣게 응원할게.")
        })
    } else if nature.playful >= 0.65 {
        Some(
            if matches!(feeling, Feeling::Happy | Feeling::Proud | Feeling::Bored) {
                (
                    "My whole little room is cheering!",
                    "내 작은 방도 같이 응원해!",
                )
            } else {
                (
                    "Your little ghost is on your side.",
                    "네 작은 유령은 네 편이야.",
                )
            },
        )
    } else {
        None
    };
    if let Some((english, hangul)) = extra {
        lines.push(if korean { hangul } else { english }.to_owned());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Life, Nature};

    #[test]
    fn first_person_english_statements_recognize_feelings_without_a_model() {
        for (feeling, phrases) in [
            (
                Feeling::Tired,
                vec![
                    "I'm tired",
                    "i am so tired",
                    "im exhausted",
                    "I Am Exhausted",
                    "I’m really tired 😴",
                    "I‘m tired",
                    "I am feeling so tired",
                    "I'm sleepy.",
                    "rough day",
                    "long day",
                    "bad day",
                    "today was hard",
                    "I had a rough day",
                ],
            ),
            (
                Feeling::Sad,
                vec!["feeling sad", "I feel down", "I'm so sad 😢", "I am upset"],
            ),
            (Feeling::Lonely, vec!["I'm lonely", "i feel lonely today"]),
            (
                Feeling::Stressed,
                vec![
                    "I'm stressed out",
                    "I'm anxious",
                    "I'm worried",
                    "I'm nervous",
                ],
            ),
            (
                Feeling::Angry,
                vec!["I'm angry", "I'm annoyed", "I'm frustrated"],
            ),
            (
                Feeling::Unwell,
                vec!["I'm sick", "I have a headache", "I feel unwell"],
            ),
            (Feeling::Bored, vec!["I'm bored", "feeling really bored"]),
            (
                Feeling::Happy,
                vec![
                    "I'm happy",
                    "I'm so happy! 😊",
                    "I'm excited",
                    "I'm great",
                    "good day",
                    "great day",
                ],
            ),
            (
                Feeling::Proud,
                vec!["I did it!", "I'm proud", "I am really proud"],
            ),
        ] {
            for phrase in phrases {
                assert_eq!(felt(phrase), Some(feeling), "{phrase}");
                assert_eq!(
                    World::new(1, 12.0).hear(phrase).understanding,
                    Understanding::Felt(feeling),
                    "{phrase}"
                );
            }
        }
    }

    #[test]
    fn korean_statements_accept_modifiers_and_ordinary_endings() {
        for (feeling, phrases) in [
            (
                Feeling::Tired,
                vec![
                    "피곤해",
                    "피곤하다",
                    "피곤하네",
                    "피곤해요",
                    "나는 너무 피곤해요",
                    "지쳤어",
                    "졸려",
                    "오늘 너무 힘들었어",
                    "힘들어",
                    "힘든 하루",
                ],
            ),
            (
                Feeling::Sad,
                vec![
                    "슬퍼",
                    "우울해",
                    "속상해",
                    "나 정말 슬퍼요",
                    "슬프네",
                    "우울하다",
                ],
            ),
            (
                Feeling::Lonely,
                vec!["외로워", "오늘 진짜 외로워요", "외롭다"],
            ),
            (
                Feeling::Stressed,
                vec![
                    "스트레스",
                    "불안해",
                    "걱정돼",
                    "긴장돼",
                    "나는 너무 불안해요",
                ],
            ),
            (Feeling::Angry, vec!["화나", "짜증나", "오늘 정말 화나요"]),
            (
                Feeling::Unwell,
                vec!["아파", "머리 아파", "머리가 아파요", "감기", "감기 걸렸어"],
            ),
            (Feeling::Bored, vec!["심심해", "나 너무 심심해요"]),
            (
                Feeling::Happy,
                vec![
                    "행복해",
                    "기뻐",
                    "신나",
                    "오늘 기분 좋아",
                    "나는 오늘 기분이 좋아요",
                    "오늘 좋은 하루",
                ],
            ),
            (Feeling::Proud, vec!["해냈어", "뿌듯해", "나 정말 뿌듯해요"]),
        ] {
            for phrase in phrases {
                assert_eq!(felt(phrase), Some(feeling), "{phrase}");
                assert_eq!(
                    World::new(1, 12.0).hear(phrase).understanding,
                    Understanding::Felt(feeling),
                    "{phrase}"
                );
            }
        }
    }

    #[test]
    fn questions_negations_other_people_and_names_are_not_the_viewers_feeling() {
        for text in [
            "are you tired?",
            "are you lonely",
            "are you bored?",
            "how do you feel?",
            "I'm tired, are you?",
            "I'm happy?",
            "not tired",
            "I'm not sad",
            "I am never angry",
            "my cat is sick",
            "he is angry",
            "she is tired",
            "you are boring",
            "you're sick",
            "you tired ghost",
            "I think you are sad",
            "I'm tired of you",
            "I'm happy for him",
            "I said I'm sad",
            "\"I'm tired\"",
            "feeling sad for her",
            "I have a headache medicine",
            "hi",
            "good ghost",
            "what a great day",
            "your name is Tired",
            "I'll call you Sad",
            "너 피곤해?",
            "너 피곤해",
            "외로워?",
            "안 피곤해",
            "안 슬퍼",
            "피곤하지 않아",
            "슬프지 않아요",
            "내 고양이 아파",
            "그는 화나",
            "너 짜증나",
            "너는 심심해",
            "너 바보야",
            "좋아",
            "오늘 좋아",
            "네가 좋아",
            "이름은 피곤해",
            "오늘 기분 안 좋아",
            "힘들지 않아",
            "나는 행복하지 않다",
        ] {
            assert_eq!(felt(text), None, "{text}");
        }
    }

    fn settled(feeling: &str) -> World {
        let mut world = World::new(17, 12.0);
        world.begin_life("2026-06-21");
        world.hear(feeling);
        world.ghost.position = world.ghost.plan.target;
        world.ghost.velocity = Vec3::ZERO;
        world
    }

    fn finish(world: &mut World) {
        for _ in 0..800 {
            world.notice_viewer();
            world.step(0.1);
            if world.ghost.plan.done || world.comforting.is_none() {
                return;
            }
        }
        panic!("comfort should finish in a little over a minute");
    }

    #[test]
    fn feelings_bypass_learning_and_record_ordered_events_without_praise() {
        for phrase in ["I'm sad", "I'm happy!", "오늘 너무 힘들었어"] {
            let mut world = World::new(1, 12.0);
            world.hear("spin");
            let words = world.life.vocabulary.clone();
            let heard = world.hear(phrase);
            assert_eq!(
                heard.understanding,
                Understanding::Felt(felt(phrase).unwrap())
            );
            assert_eq!(heard.tone, None);
            assert!(heard.learned.is_empty());
            assert_eq!(world.life.vocabulary, words);
            assert_eq!(world.listening, Listening::default());
            let heard_at = world
                .events
                .iter()
                .rposition(|event| event.kind == EventKind::Heard(phrase.to_owned()))
                .unwrap();
            assert_eq!(
                world.events[heard_at + 1].kind,
                EventKind::ViewerFelt(felt(phrase).unwrap())
            );
            assert_eq!(world.life.today.feelings_heard, 1);
        }
    }

    #[test]
    fn sleepers_focus_and_stories_keep_their_plan_when_the_viewer_feels_sad() {
        for condition in 0..3 {
            let mut world = World::new(1, 12.0);
            match condition {
                0 => {
                    world.hour = 23.0;
                    world.tuck_in();
                }
                1 => world.start_focus(25),
                _ => world.tell(vec!["A little tale.".to_owned()]),
            }
            let plan = world.ghost.plan.clone();
            let asleep = world.ghost.asleep;
            world.hear("I'm sad");
            assert_eq!(world.ghost.plan, plan);
            assert_eq!(world.ghost.asleep, asleep);
            assert!(world.comforting.is_none());
            assert_eq!(world.life.total.feelings_heard, 1);
            assert_eq!(world.life.total.comforts, 0);
        }
    }

    #[test]
    fn completed_care_counts_once_warms_the_bond_and_remembers_each_feeling() {
        for phrase in [
            "I'm tired",
            "I'm sad",
            "I'm lonely",
            "I'm stressed",
            "I'm angry",
            "I'm sick",
            "I'm bored",
            "I'm happy",
            "I'm proud",
        ] {
            let mut world = settled(phrase);
            assert_eq!(world.ghost.plan.activity, Activity::Comfort);
            let bond = world.life.bond;
            finish(&mut world);
            assert_eq!(world.life.total.comforts, 1, "{phrase}");
            assert!(world.life.bond >= bond + 0.006);
            assert_eq!(
                world.life.today.comforted_feelings,
                vec![felt(phrase).unwrap()]
            );
            assert!(
                world
                    .life
                    .milestones
                    .iter()
                    .any(|entry| entry.text == "I comforted you for the first time.")
            );
            world.step(0.1);
            assert_eq!(world.life.total.comforts, 1);
            assert!(!world.options().contains(&Activity::Comfort));
        }
    }

    #[test]
    fn interrupted_care_never_counts_as_a_completed_comfort() {
        for interruption in 0..5 {
            let mut world = settled("I'm stressed");
            world.step(2.0);
            match interruption {
                0 => world.poke(),
                1 => {
                    world.hour = 23.0;
                    world.tuck_in();
                }
                2 => {
                    world.look_away(true);
                    world.step(21.0);
                }
                3 => {
                    world.hear("no stop");
                }
                _ => world.begin(Activity::Explore),
            }
            assert!(world.comforting.is_none());
            world.step(70.0);
            assert_eq!(world.life.total.comforts, 0);
        }
    }

    #[test]
    fn breathing_uses_four_to_six_ten_second_cycles_and_then_goes_quiet() {
        assert_eq!(breath(0.0), 0.0);
        assert_eq!(breath(4.0), 1.0);
        assert_eq!(breath(10.0), 0.0);
        assert!(breath(2.0) < breath(3.0));
        assert!(breath(6.0) > breath(8.0));
        for korean in [false, true] {
            let mut world = settled(if korean { "불안해" } else { "I'm stressed" });
            let cycles = world.comforting.as_ref().unwrap().cycles;
            assert!((4..=6).contains(&cycles));
            assert_eq!(world.ghost.plan.length, cycles as f32 * 10.0);
            finish(&mut world);
            let cues: Vec<&str> = world
                .events
                .iter()
                .filter_map(|event| match &event.kind {
                    EventKind::Said(text) => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            assert_eq!(
                cues,
                if korean {
                    vec![
                        "같이 숨 쉬자…",
                        "들이쉬고…",
                        "내쉬고…",
                        "들이쉬고…",
                        "내쉬고…",
                    ]
                } else {
                    vec!["Breathe with me…", "in…", "out…", "in…", "out…"]
                }
            );
        }
    }

    #[test]
    fn cheering_draws_an_existing_picture_before_finishing_and_celebration_practices_dancing() {
        let mut world = settled("I'm sad");
        finish(&mut world);
        assert_eq!(world.doodles.len(), 1);
        assert!(matches!(
            world.doodles[0].picture,
            Picture::Sketch(Sketch::Heart | Sketch::Sun)
        ));
        assert_eq!(world.life.today.doodles, 1);
        let mut world = settled("I did it!");
        finish(&mut world);
        assert_eq!(world.life.today.dances, 1);
    }

    #[test]
    fn boredom_respects_ignored_invitations_and_existing_games() {
        let mut world = settled("I'm bored");
        finish(&mut world);
        assert!(matches!(
            world.ghost.plan.activity,
            Activity::Game | Activity::Puzzle
        ));
        let mut world = settled("I'm bored");
        world.invitation_ignored();
        finish(&mut world);
        assert!(!matches!(
            world.ghost.plan.activity,
            Activity::Game | Activity::Puzzle
        ));
        let mut world = World::new(1, 12.0);
        world.begin(Activity::Game);
        world.hear("I'm bored");
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
        assert!(
            world
                .game
                .as_ref()
                .is_some_and(|game| game.outcome == Some(super::super::Outcome::Abandoned))
        );
        finish(&mut world);
        assert_eq!(world.life.total.comforts, 1);
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
    }

    #[test]
    fn hearing_a_feeling_after_midnight_counts_and_saves_it_on_the_new_local_day() {
        let mut world = settled("I'm tired");
        world.date = Date::constant(2026, 6, 22);
        world.hear("I'm sad");
        assert_eq!(world.life.day, "2026-06-22");
        assert_eq!(world.life.today.feelings, vec![Feeling::Sad]);
        let life = world.life.sanitized();
        assert_eq!(life.feelings.len(), 2);
        assert_eq!(life.feelings.last().unwrap().day, life.day);
    }

    #[test]
    fn saved_notes_are_bounded_defaulted_and_drop_damaged_or_future_values() {
        let mut world = settled("I'm tired");
        for _ in 0..15 {
            world.hear("I'm sad");
        }
        assert_eq!(world.life.feelings.len(), 12);
        world.life.feelings[0].asked = true;
        let restored: Life =
            serde_json::from_str(&serde_json::to_string(&world.life).unwrap()).unwrap();
        assert!(restored.feelings[0].asked);
        assert!(
            serde_json::from_str::<Life>("{}")
                .unwrap()
                .feelings
                .is_empty()
        );
        let mut life = Life {
            day: "2026-06-21".to_owned(),
            ..Life::default()
        };
        for (day, hour) in [
            ("bad", 12.0),
            ("2026-06-22", 12.0),
            ("2026-06-21", f32::NAN),
            ("2026-06-21", -1.0),
            ("2026-06-21", 24.0),
            ("2026-06-20", 23.0),
        ] {
            life.feelings.push(FeelingNote {
                feeling: Feeling::Tired,
                day: day.to_owned(),
                hour,
                asked: false,
            });
        }
        assert_eq!(life.sanitized().feelings.len(), 1);
    }

    #[test]
    fn care_reaches_the_glass_from_the_room_and_seeded_choices_repeat() {
        for phrase in ["I'm stressed", "I'm sick", "I'm sad", "I'm happy"] {
            let mut first = World::new(29, 12.0);
            let mut second = first.clone();
            first.hear(phrase);
            second.hear(phrase);
            finish(&mut first);
            finish(&mut second);
            assert_eq!(first.events, second.events);
            assert_eq!(first.life.total.comforts, 1, "{phrase}");
            assert_eq!(first.life.total, second.life.total);
            assert!((first.ghost.position.z - super::super::ROOM_MAX.z).abs() < 0.8);
        }
    }

    #[test]
    fn evening_lullabies_use_the_music_plan_and_never_replace_busy_music_or_focus() {
        let mut offers = 0;
        for seed in 1..20 {
            let mut world = World::new(seed, 20.0);
            world.hear("I'm tired");
            world.ghost.position = world.ghost.plan.target;
            finish(&mut world);
            if world.ghost.plan.activity == Activity::Music {
                offers += 1;
                for _ in 0..400 {
                    world.notice_viewer();
                    world.step(0.1);
                    if world.music.is_some() {
                        break;
                    }
                }
                assert!(
                    world
                        .music
                        .as_ref()
                        .is_some_and(|playing| playing.tune.name == "little lullaby")
                );
            }
        }
        assert!(offers > 0 && offers < 19);
        let mut world = World::new(3, 20.0);
        world.wind_music_box();
        let playing = world.music.clone();
        world.hear("I'm sick");
        assert!(!world.comforting.as_ref().unwrap().lullaby);
        assert_eq!(world.music, playing);
        world.start_focus(25);
        world.hear("I'm tired");
        assert_eq!(world.ghost.plan.activity, Activity::Company);
        assert!(world.music.is_none());
    }

    fn following_up() -> World {
        let mut world = settled("I'm tired");
        finish(&mut world);
        world.date = Date::constant(2026, 6, 22);
        world.life.begin_day("2026-06-22");
        world.time += 30.0;
        world.begin(Activity::Visit);
        world.ghost.position = world.ghost.plan.target;
        world.ghost.plan.arrived = Some(world.time - 5.0);
        world.notice_viewer();
        world.follow_up_feeling();
        assert!(world.feeling_follow_up.is_some());
        world
    }

    #[test]
    fn a_restored_feeling_ages_by_the_clock_alone() {
        let mut world = World::new(1, 12.25);
        world.life.feelings.push(FeelingNote {
            feeling: Feeling::Tired,
            day: world.date.to_string(),
            hour: 12.0,
            asked: false,
        });
        // A quarter of an hour of live play after restoring moves both the
        // clock and the world's time.
        world.time = 900.0;
        assert_eq!(world.recent_feeling(1200.0), Some(Feeling::Tired));
        assert_eq!(world.recent_feeling(600.0), None);
    }

    #[test]
    fn a_later_day_follow_up_is_saved_once_and_positive_answers_teach_no_words() {
        for answer in [
            "yes",
            "yeah",
            "better",
            "much better",
            "hello",
            "I'm happy",
            "응",
            "어",
            "괜찮아",
            "나아졌어",
        ] {
            let mut world = following_up();
            assert!(world.life.feelings[0].asked);
            let heard = world.hear(answer);
            assert_eq!(heard.understanding, Understanding::FeltBetter, "{answer}");
            assert_eq!(world.life.today.felt_better, 1);
            assert!(world.life.vocabulary.known().is_empty());
            world.follow_up_feeling();
            assert!(world.feeling_follow_up.is_none());
            let mut restored = World::new(1, 12.0);
            restored.date = world.date;
            restored.life = world.life;
            restored.notice_viewer();
            restored.follow_up_feeling();
            assert!(restored.feeling_follow_up.is_none());
        }
    }

    #[test]
    fn negative_follow_up_answers_restart_care_and_the_window_expires() {
        for answer in [
            "no",
            "not really",
            "아니",
            "별로",
            "I'm sad",
            "아파",
            "I'm not feeling any better",
            "not better",
            "안 괜찮아",
        ] {
            let mut world = following_up();
            let heard = world.hear(answer);
            assert!(
                matches!(heard.understanding, Understanding::Felt(_)),
                "{answer}"
            );
            assert_eq!(world.ghost.plan.activity, Activity::Comfort);
            assert_eq!(world.life.today.feelings_heard, 1);
        }
        let mut world = following_up();
        world.time += 181.0;
        assert_ne!(world.hear("yes").understanding, Understanding::FeltBetter);
        assert_eq!(world.life.today.felt_better, 0);
    }

    #[test]
    fn follow_up_waits_for_a_later_day_presence_sleep_and_quiet_and_expires_after_three_days() {
        for condition in 0..8 {
            let mut world = settled("I'm sad");
            finish(&mut world);
            world.date = Date::constant(2026, 6, 22);
            world.time += 30.0;
            world.notice_viewer();
            world.begin(Activity::Visit);
            world.ghost.plan.arrived = Some(world.time - 5.0);
            match condition {
                0 => world.date = Date::constant(2026, 6, 21),
                1 => world.date = Date::constant(2026, 6, 25),
                2 => world.viewer.active_at = -1000.0,
                3 => world.ghost.asleep = 1.0,
                4 => world.start_focus(25),
                5 => world.tell(vec!["A tale.".to_owned()]),
                6 => world.begin(Activity::Game),
                _ => world.ghost.plan.arrived = None,
            }
            world.follow_up_feeling();
            assert!(world.feeling_follow_up.is_none(), "condition {condition}");
            assert!(!world.life.feelings[0].asked);
        }
    }

    #[test]
    fn a_quiet_viewer_still_has_three_minutes_to_answer_a_follow_up() {
        let mut world = following_up();
        world.step(120.0);
        assert!(!world.viewer_present());
        assert!(world.feeling_follow_up.is_some());
        assert_eq!(
            world.hear("much better").understanding,
            Understanding::FeltBetter
        );
    }

    #[test]
    fn a_direct_command_or_unrelated_message_consumes_the_follow_up_answer_window() {
        for message in [
            "focus",
            "yawn",
            "open the window",
            "my name is Sam",
            "abracadabra",
        ] {
            let mut world = following_up();
            world.hear(message);
            assert!(world.feeling_follow_up.is_none(), "{message}");
            assert_ne!(world.hear("yes").understanding, Understanding::FeltBetter);
        }
        let mut world = settled("I'm sad");
        world.hear("I'm happy");
        world.date = Date::constant(2026, 6, 22);
        world.begin(Activity::Visit);
        world.ghost.plan.arrived = Some(world.time - 5.0);
        world.follow_up_feeling();
        assert!(world.feeling_follow_up.is_none());
    }

    #[test]
    fn recent_feelings_use_elapsed_time_and_saved_civil_time_across_midnight() {
        let mut world = settled("I'm tired");
        assert_eq!(world.recent_negative_feeling(1800.0), Some(Feeling::Tired));
        world.time += 1801.0;
        assert_eq!(world.recent_negative_feeling(1800.0), None);
        world.date = Date::constant(2026, 6, 22);
        world.hour = 0.1;
        world.time = 0.0;
        world.feeling_at = None;
        world.life.feelings[0].hour = 23.9;
        assert_eq!(world.recent_negative_feeling(1800.0), Some(Feeling::Tired));
        world.hour = 2.0;
        assert_eq!(world.recent_negative_feeling(3600.0), None);
    }

    #[test]
    fn replies_follow_language_and_nature_and_memories_use_words_instead_of_dates() {
        let mut world = settled("I'm sad");
        for nature in [
            Nature {
                tender: 1.0,
                ..Nature::default()
            },
            Nature {
                shy: 1.0,
                ..Nature::default()
            },
            Nature {
                playful: 1.0,
                ..Nature::default()
            },
        ] {
            world.life.nature = Some(nature);
            assert!(replies(&world, Feeling::Sad, false).len() >= 4);
            assert!(replies(&world, Feeling::Sad, true).iter().all(|line| {
                line.chars()
                    .any(|character| ('가'..='힣').contains(&character))
            }));
        }
        let words = memories(&world.life.feelings, "2026-06-22");
        assert!(words.contains("sad yesterday"));
        assert!(!words.chars().any(char::is_numeric));
    }
}
