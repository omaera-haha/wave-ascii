use serde::{Deserialize, Serialize};

use super::{Activity, EventKind, Life, Passerby, Season, Stage, World};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Wish {
    Rainbow,
    Snow,
    ShootingStar,
    Constellation,
    Passerby(Passerby),
    BirdGift,
    Flower,
    Friend {
        name: String,
    },
    Tower {
        height: u32,
    },
    ViewerName,
    GivenName,
    Game,
    WordGame,
    Drawing,
    Petting,
    Fetch,
    Story,
    Company,
    Breeze,
    /// To finish reading one of its picture books for the first time.
    Book {
        index: usize,
    },
    /// For the viewer to teach it a tune.
    Tune,
    /// For the viewer to tell it about themselves when it asks.
    AboutYou,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wishing {
    pub wish: Wish,
    pub day: String,
    #[serde(default)]
    pub said: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Granted {
    pub wish: Wish,
    pub day: String,
}

impl Wish {
    pub fn words(&self, life: &Life) -> String {
        match self {
            Self::Rainbow => "to see a rainbow".to_owned(),
            Self::Snow => "to see falling snow".to_owned(),
            Self::ShootingStar => "to see a shooting star".to_owned(),
            Self::Constellation => "to find a constellation".to_owned(),
            Self::Passerby(kind) => format!("to see {}", kind.name()),
            Self::BirdGift => "to receive a gift from the bird".to_owned(),
            Self::Flower => "to see a flower in my garden".to_owned(),
            Self::Friend { name } => format!(
                "to see {} again",
                life.friend
                    .as_ref()
                    .map_or(name.as_str(), |friend| &friend.name)
            ),
            Self::Tower { height } => format!(
                "to build a tower {} blocks high",
                super::friend::count_words(*height)
            ),
            Self::ViewerName => "to learn your name".to_owned(),
            Self::GivenName => "for you to give me a name".to_owned(),
            Self::Game => "to play noughts and crosses with you".to_owned(),
            Self::WordGame => "to play a word game with you".to_owned(),
            Self::Drawing => "for you to draw me something in the fog".to_owned(),
            Self::Petting => "for you to pet me gently".to_owned(),
            Self::Fetch => "to play fetch with you".to_owned(),
            Self::Story => "to tell you a story".to_owned(),
            Self::Company => "to keep you company while you work".to_owned(),
            Self::Breeze => "to feel the breeze with you".to_owned(),
            Self::Book { index } => format!("to finish reading {}", book_title(*index, false)),
            Self::Tune => "for you to teach me a tune".to_owned(),
            Self::AboutYou => "to learn more about you".to_owned(),
        }
    }

    pub fn korean(&self, life: &Life) -> String {
        format!("{}는 것", self.korean_action(life))
    }

    pub(super) fn korean_wish(&self, life: &Life) -> String {
        match self {
            Self::Flower => "내 정원에 꽃이 피면 좋겠다는 소원".to_owned(),
            Self::GivenName => "네가 내 이름을 지어 주면 좋겠다는 소원".to_owned(),
            Self::Drawing => "네가 김 서린 유리에 그림을 그려 주면 좋겠다는 소원".to_owned(),
            Self::Petting => "네가 나를 부드럽게 쓰다듬어 주면 좋겠다는 소원".to_owned(),
            Self::Tune => "네가 노래를 가르쳐 주면 좋겠다는 소원".to_owned(),
            _ => format!("{}고 싶다는 소원", self.korean_action(life)),
        }
    }

    fn korean_action(&self, life: &Life) -> String {
        match self {
            Self::Rainbow => "무지개를 보".to_owned(),
            Self::Snow => "내리는 눈을 보".to_owned(),
            Self::ShootingStar => "별똥별을 보".to_owned(),
            Self::Constellation => "별자리를 찾".to_owned(),
            Self::Passerby(kind) => format!(
                "{} 보",
                match kind {
                    Passerby::Geese => "날아가는 기러기를",
                    Passerby::Balloon => "열기구를",
                    Passerby::Kite => "바람에 날리는 연을",
                    Passerby::Squirrel => "나무 위의 다람쥐를",
                    Passerby::Plane => "비행기의 반짝이는 불빛을",
                    Passerby::Snowman => "언덕 위의 눈사람을",
                    Passerby::Lanterns => "밤하늘로 떠오르는 풍등을",
                }
            ),
            Self::BirdGift => "새에게 선물을 받".to_owned(),
            Self::Flower => "내 정원에 꽃이 피".to_owned(),
            Self::Friend { name } => format!(
                "{} 다시 만나",
                object_name(
                    life.friend
                        .as_ref()
                        .map_or(name.as_str(), |friend| &friend.name)
                )
            ),
            Self::Tower { height } => format!(
                "블록 {} 높이의 탑을 쌓",
                super::writing::count(*height, "개")
            ),
            Self::ViewerName => "네 이름을 배우".to_owned(),
            Self::GivenName => "네가 내 이름을 지어 주".to_owned(),
            Self::Game => "너와 유리창에서 틱택토를 하".to_owned(),
            Self::WordGame => "너와 단어 맞히기 놀이를 하".to_owned(),
            Self::Drawing => "네가 김 서린 유리에 그림을 그려 주".to_owned(),
            Self::Petting => "네가 나를 부드럽게 쓰다듬어 주".to_owned(),
            Self::Fetch => "너와 공 가져오기 놀이를 하".to_owned(),
            Self::Story => "너에게 이야기를 들려주".to_owned(),
            Self::Company => "네가 일하는 동안 곁에 있어 주".to_owned(),
            Self::Breeze => "너와 산들바람을 느끼".to_owned(),
            Self::Book { index } => {
                let title = book_title(*index, true);
                format!("《{title}》{} 끝까지 읽", object_particle(title))
            }
            Self::Tune => "네가 노래를 가르쳐 주".to_owned(),
            Self::AboutYou => "너에 대해 더 알아 가".to_owned(),
        }
    }

    pub fn granted_by(&self, kind: &EventKind) -> bool {
        match (self, kind) {
            (Self::Rainbow, EventKind::Noticed(thing)) => thing.starts_with("the rainbow"),
            (Self::Snow, EventKind::Noticed(thing)) => thing == "the falling snow",
            (Self::ShootingStar, EventKind::ShootingStar)
            | (Self::Constellation, EventKind::FoundConstellation(_))
            | (Self::BirdGift, EventKind::Kept(_))
            | (Self::Flower, EventKind::Flowered(_))
            | (Self::Friend { .. }, EventKind::FriendArrived(_))
            | (Self::ViewerName, EventKind::LearnedName(_))
            | (Self::GivenName, EventKind::Named(_))
            | (Self::WordGame, EventKind::WordGame { .. })
            | (Self::Drawing, EventKind::SawDrawing { .. })
            | (Self::Petting, EventKind::Petted)
            | (Self::Fetch, EventKind::Fetched)
            | (Self::Story, EventKind::ToldStory)
            | (
                Self::Company,
                EventKind::FocusEnded {
                    completed: true, ..
                },
            )
            | (Self::Breeze, EventKind::OpenedWindow)
            | (Self::Tune, EventKind::LearnedTune(_))
            | (Self::AboutYou, EventKind::LearnedAboutViewer(_)) => true,
            (Self::Book { index }, EventKind::FinishedBook(title)) => {
                book_title(*index, false) == title
            }
            (Self::Tower { height: target }, EventKind::Built { height }) => height >= target,
            (Self::Passerby(wanted), EventKind::SawPasserby(seen)) => wanted == seen,
            (Self::Game, EventKind::GameOver(outcome)) => outcome.finished(),
            _ => false,
        }
    }

    pub fn with_viewer(&self) -> bool {
        matches!(
            self,
            Self::ViewerName
                | Self::GivenName
                | Self::Game
                | Self::WordGame
                | Self::Drawing
                | Self::Petting
                | Self::Fetch
                | Self::Story
                | Self::Company
                | Self::Breeze
                | Self::Tune
                | Self::AboutYou
        )
    }

    pub fn hint(&self) -> Option<&'static str> {
        Some(match self {
            Self::ViewerName => "tell it your name",
            Self::GivenName => "say \"your name is Pip\"",
            Self::Game => "press g",
            Self::WordGame => "say \"let's play a word game\"",
            Self::Drawing => "right-drag on the glass, or press f",
            Self::Petting => "drag across it gently",
            Self::Fetch => "click the ball",
            Self::Story => "ask it for a story",
            Self::Company => "press o",
            Self::Breeze => "click the window",
            Self::Tune => "type notes like \"do re mi\"",
            Self::AboutYou => "answer when it asks about you",
            _ => return None,
        })
    }

    pub fn serves(&self, activity: Activity, world: &World) -> bool {
        if self.with_viewer() && activity == Activity::Visit && world.viewer_present() {
            return true;
        }
        match self {
            Self::Rainbow | Self::Snow | Self::Passerby(_) | Self::BirdGift => {
                activity == Activity::Window
            }
            Self::ShootingStar | Self::Constellation => {
                matches!(activity, Activity::Window | Activity::Yard)
                    && super::meteor::dark(world.hour)
            }
            Self::Tower { .. } => activity == Activity::Build,
            Self::Book { .. } => activity == Activity::Read,
            Self::Flower => activity == Activity::Tend,
            Self::Breeze => activity == Activity::Air && !world.window_open,
            _ => false,
        }
    }

    fn season_allows(&self, world: &World) -> bool {
        match self {
            Self::Rainbow | Self::BirdGift | Self::Flower | Self::Breeze => {
                world.season() != Season::Winter
            }
            Self::Snow => world.season() == Season::Winter,
            Self::Passerby(kind) => kind.in_season(world.date),
            _ => true,
        }
    }

    fn eligible(&self, world: &World) -> bool {
        if world.age_days().is_some_and(|days| days < 1.0) || !self.season_allows(world) {
            return false;
        }
        if self.with_viewer() && *self != Self::ViewerName && world.life.bond < 0.3 {
            return false;
        }
        if world.stage() == Stage::Newborn
            && !matches!(
                self,
                Self::Rainbow | Self::ShootingStar | Self::Petting | Self::Tower { height: 2 }
            )
        {
            return false;
        }
        match self {
            Self::Constellation => world.stage() != Stage::Newborn,
            Self::BirdGift => world.life.bird_trust >= 1.0 / 3.0,
            Self::Flower => {
                world.seed_ready() || world.seedlings.iter().any(|seedling| seedling.growth < 1.0)
            }
            Self::Friend { .. } => world.life.friend.is_some(),
            Self::ViewerName => world.life.viewer_name.is_none(),
            Self::GivenName => world.life.total.names == 0,
            // Only while something about the viewer is still unknown, and
            // only once it is close enough to ask.
            Self::AboutYou => {
                world.life.bond >= 0.35
                    && super::acquaintance::Topic::ALL
                        .into_iter()
                        .any(|topic| world.life.viewer.answer(topic, false).is_none())
            }
            Self::Book { index } => world
                .life
                .books
                .owned
                .iter()
                .any(|owned| owned.index == *index && owned.finished == 0),
            Self::Breeze => world.fresh_air_is_fair(),
            _ => true,
        }
    }

    fn same_kind(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }

    fn weight(&self, world: &World) -> f32 {
        let nature = world.life.temperament();
        let mut weight = 1.0;
        if matches!(
            self,
            Self::Rainbow
                | Self::Snow
                | Self::ShootingStar
                | Self::Constellation
                | Self::Passerby(_)
        ) {
            weight *= 0.5 + nature.curious;
        }
        if matches!(
            self,
            Self::Tower { .. } | Self::Game | Self::Fetch | Self::WordGame
        ) {
            weight *= 0.5 + nature.playful;
        }
        if matches!(
            self,
            Self::Petting | Self::Story | Self::Flower | Self::BirdGift | Self::Tune
        ) {
            weight *= 0.5 + nature.tender;
        }
        if matches!(self, Self::Book { .. } | Self::AboutYou) {
            weight *= 0.5 + 0.5 * (nature.curious + nature.tender);
        }
        if matches!(self, Self::ShootingStar | Self::Constellation | Self::Story) {
            weight *= 0.5 + nature.sleepy;
        }
        if self.with_viewer() {
            weight *= 1.0 - 0.5 * nature.shy;
        }
        weight
    }

    pub(super) fn valid(&self, life: &Life) -> bool {
        match self {
            // Five blocks make the tallest tower the room allows.
            Self::Tower { height } => (2..=5).contains(height),
            Self::Book { index } => *index < super::books::LIBRARY.len(),
            Self::Friend { name } => {
                life.friend.is_some()
                    && !name.trim().is_empty()
                    && name.chars().count() <= 24
                    && !name.chars().any(char::is_control)
            }
            _ => true,
        }
    }

    pub fn spoken(&self, korean: bool) -> &'static str {
        let (english, hangul) = match self {
            Self::ViewerName => ("I wish I knew your name.", "네 이름을 알고 싶어."),
            Self::GivenName => (
                "I wish you would give me a name.",
                "네가 내 이름을 지어 주면 좋겠어.",
            ),
            Self::Game => (
                "I wish we could play a game on the glass.",
                "유리창에서 같이 게임하고 싶어.",
            ),
            Self::WordGame => (
                "I wish we could play a word game.",
                "같이 단어 맞히기 놀이를 하고 싶어.",
            ),
            Self::Drawing => (
                "I wish you would draw me something in the fog.",
                "김 서린 유리에 그림을 그려 주면 좋겠어.",
            ),
            Self::Petting => (
                "I wish you would pet me gently.",
                "나를 부드럽게 쓰다듬어 주면 좋겠어.",
            ),
            Self::Fetch => (
                "I wish we could play fetch.",
                "같이 공 가져오기 놀이를 하고 싶어.",
            ),
            Self::Story => (
                "I wish I could tell you a story.",
                "너에게 이야기를 들려주고 싶어.",
            ),
            Self::Company => (
                "I wish I could keep you company while you work.",
                "네가 일하는 동안 곁에 있고 싶어.",
            ),
            Self::Breeze => (
                "I wish we could feel the breeze together.",
                "같이 산들바람을 느끼고 싶어.",
            ),
            Self::Tune => (
                "I wish you'd teach me a tune.",
                "노래를 하나 가르쳐 주면 좋겠어.",
            ),
            Self::AboutYou => ("I wish I knew more about you.", "너에 대해 더 알고 싶어."),
            _ => return "",
        };
        if korean { hangul } else { english }
    }

    pub fn notice(&self, name: &str) -> Option<String> {
        let action = match self {
            Self::ViewerName => "tell it your name",
            Self::GivenName => "give it a name",
            Self::Game => "play noughts and crosses",
            Self::WordGame => "play a word game",
            Self::Drawing => "draw it a picture",
            Self::Petting => "pet it gently",
            Self::Fetch => "play fetch",
            Self::Story => "ask for a story",
            Self::Company => "work with it beside you",
            Self::Breeze => "open the window together",
            Self::Tune => "teach it a tune",
            Self::AboutYou => "tell it about yourself",
            _ => return None,
        };
        Some(format!("{name} wishes you'd {action} ({})", self.hint()?))
    }

    pub(super) fn joy(&self, world: &World) -> String {
        if !self.with_viewer() {
            let delight = match self {
                Self::Rainbow => "A rainbow!".to_owned(),
                Self::Snow => "Snowflakes at my window!".to_owned(),
                Self::ShootingStar => "A shooting star!".to_owned(),
                Self::Constellation => "I found a picture in the stars!".to_owned(),
                Self::Passerby(kind) => format!("I saw {}!", kind.name()),
                Self::BirdGift => "The bird brought me a gift!".to_owned(),
                Self::Flower => "A flower in my garden!".to_owned(),
                Self::Friend { name } => format!(
                    "{} came back!",
                    world
                        .life
                        .friend
                        .as_ref()
                        .map_or(name.as_str(), |friend| &friend.name)
                ),
                Self::Tower { height } => format!(
                    "My tower is {} blocks high!",
                    super::friend::count_words(*height)
                ),
                Self::Book { index } => format!("I finished {}!", book_title(*index, false)),
                _ => unreachable!(),
            };
            return format!("{delight} My wish came true.");
        }
        let (english, korean) = match self {
            Self::ViewerName => (
                "Now I know your name! My wish came true.",
                "네 이름을 알게 됐어! 소원이 이루어졌어.",
            ),
            Self::GivenName => (
                "You gave me a name! My wish came true.",
                "내 이름을 지어 줬네! 소원이 이루어졌어.",
            ),
            Self::Drawing => (
                "You drew for me! My wish came true.",
                "그림을 그려 줬네! 소원이 이루어졌어.",
            ),
            Self::Petting => (
                "Your hand is so gentle! My wish came true.",
                "손길이 참 다정해! 소원이 이루어졌어.",
            ),
            Self::Story => (
                "I told you my story! My wish came true.",
                "이야기를 들려줬어! 소원이 이루어졌어.",
            ),
            Self::Company => (
                "We worked together! My wish came true.",
                "함께 일했어! 소원이 이루어졌어.",
            ),
            Self::Breeze => (
                "We felt the breeze! My wish came true.",
                "같이 산들바람을 느꼈어! 소원이 이루어졌어.",
            ),
            Self::Tune => (
                "You taught me a tune! My wish came true.",
                "노래를 가르쳐 줬네! 소원이 이루어졌어.",
            ),
            Self::AboutYou => (
                "Now I know you a little better! My wish came true.",
                "너를 조금 더 알게 됐어! 소원이 이루어졌어.",
            ),
            _ => (
                "You played with me! My wish came true.",
                "같이 놀아 줬네! 소원이 이루어졌어.",
            ),
        };
        if world.speaks_korean() {
            korean
        } else {
            english
        }
        .to_owned()
    }

    pub(super) fn celebrate(&self, world: &mut World) {
        if world.ghost.asleep >= 0.3 {
            return;
        }
        let joy = self.joy(world);
        if self.with_viewer() {
            world.say(&joy);
        } else {
            world.think(&joy);
        }
    }
}

impl Life {
    /// Moves a wish that came true into the granted history, keeping the
    /// newest twelve.
    pub(super) fn grant(&mut self, wish: Wish, day: &str) {
        self.wish = None;
        self.granted.push(Granted {
            wish,
            day: day.to_owned(),
        });
        if self.granted.len() > 12 {
            self.granted.drain(..self.granted.len() - 12);
        }
    }
}

impl World {
    fn wish_options(&self) -> Vec<Wish> {
        let height = if self.stage() == Stage::Newborn {
            2
        } else {
            self.life.total.tallest.saturating_add(1).clamp(3, 5)
        };
        let mut wishes = vec![
            Wish::Rainbow,
            Wish::Snow,
            Wish::ShootingStar,
            Wish::Constellation,
            Wish::BirdGift,
            Wish::Flower,
            Wish::Tower { height },
            Wish::ViewerName,
            Wish::GivenName,
            Wish::Game,
            Wish::WordGame,
            Wish::Drawing,
            Wish::Petting,
            Wish::Fetch,
            Wish::Story,
            Wish::Company,
            Wish::Breeze,
        ];
        if let Some(friend) = &self.life.friend {
            wishes.push(Wish::Friend {
                name: friend.name.clone(),
            });
        }
        wishes.push(Wish::Tune);
        wishes.push(Wish::AboutYou);
        // The first unread book on the pile.
        if let Some(owned) = self
            .life
            .books
            .owned
            .iter()
            .find(|owned| owned.finished == 0)
        {
            wishes.push(Wish::Book { index: owned.index });
        }
        wishes.extend(Passerby::ALL.map(Wish::Passerby));
        wishes.retain(|wish| {
            wish.eligible(self)
                && !self
                    .life
                    .granted
                    .iter()
                    .rev()
                    .take(2)
                    .any(|granted| wish.same_kind(&granted.wish))
        });
        wishes
    }

    pub(super) fn make_wish(&mut self, upon_star: bool) {
        if self.life.wish.is_some()
            || self.ghost.asleep >= 0.3
            || (!upon_star
                && (self.life.today.wishes_granted > 0 || self.life.today.wishes_faded > 0))
        {
            return;
        }
        let wishes = self.wish_options();
        let weights: Vec<f32> = wishes.iter().map(|wish| wish.weight(self)).collect();
        if let Some(index) = self.pick(&weights) {
            let wish = wishes[index].clone();
            self.life.wish = Some(Wishing {
                wish: wish.clone(),
                day: self.date.to_string(),
                said: None,
            });
            self.record(EventKind::MadeWish(wish));
        }
    }

    pub(crate) fn celebrate_wish(&mut self) {
        if let Some(wish) = self.pending_wish.take() {
            wish.celebrate(self);
        }
    }

    pub(super) fn check_wish(&mut self) {
        let faded = self.life.wish.as_ref().is_some_and(|wishing| {
            let age = wishing
                .day
                .parse::<jiff::civil::Date>()
                .map(|day| (self.date - day).get_days())
                .unwrap_or(7);
            age >= 7 || !wishing.wish.season_allows(self)
        });
        if faded {
            let wishing = self.life.wish.take().unwrap();
            self.record(EventKind::WishFaded(wishing.wish));
        }
        self.make_wish(false);
    }

    pub(super) fn wish_on_star(&mut self) {
        let renewing = self.life.wish.is_some();
        self.make_wish(true);
        if let Some(wishing) = &self.life.wish {
            self.think(&format!(
                "I wished{}: {}.",
                if renewing { " again" } else { "" },
                wishing.wish.words(&self.life)
            ));
        }
    }

    /// Says a viewer wish once a local day; the app posts the accompanying hint.
    pub fn share_wish(&mut self) -> Option<String> {
        if (self.asking.is_some() || self.awaiting_plan_answer())
            || !self.viewer_present()
            || self.ghost.asleep >= 0.3
            || self.focus.is_some()
            || self.telling.is_some()
            || self.ghost.plan.activity != Activity::Visit
            || self.ghost.plan.arrived.is_none()
            || self.ghost.plan.done
            || self.game.as_ref().is_some_and(|game| game.ended.is_none())
            || self.puzzle_active()
        {
            return None;
        }
        if self
            .ghost
            .speech
            .as_ref()
            .is_some_and(|line| self.time - line.time < 5.0)
        {
            return None;
        }
        let day = self.date.to_string();
        let wishing = self.life.wish.as_mut()?;
        if !wishing.wish.with_viewer() || wishing.said.as_ref() == Some(&day) {
            return None;
        }
        wishing.said = Some(day);
        let wish = wishing.wish.clone();
        self.say(wish.spoken(self.speaks_korean()));
        wish.notice(&self.ghost.name)
    }

    pub fn wish_observation(&self) -> String {
        let mut words = self.life.wish.as_ref().map_or_else(
            || "It has no wish just now.".to_owned(),
            |wishing| {
                let days = wishing
                    .day
                    .parse::<jiff::civil::Date>()
                    .map(|day| (self.date - day).get_days().max(0) as u32)
                    .unwrap_or(0);
                let ago = match days {
                    0 => "today".to_owned(),
                    1 => "yesterday".to_owned(),
                    days => format!("{} days ago", super::friend::count_words(days)),
                };
                format!(
                    "It wishes {}; it made the wish {ago}.",
                    wishing.wish.words(&self.life)
                )
            },
        );
        for granted in self
            .life
            .granted
            .iter()
            .filter(|granted| granted.day == self.life.day)
        {
            words.push_str(&format!(
                " A wish came true today: {}.",
                granted.wish.words(&self.life)
            ));
        }
        words
    }
}

fn object_name(name: &str) -> String {
    format!("{name}{}", object_particle(name))
}

/// The object particle after `name`: 을 after a final consonant, else 를.
fn object_particle(name: &str) -> &'static str {
    super::writing::particle(name, "을", "를")
}

fn book_title(index: usize, korean: bool) -> &'static str {
    super::books::LIBRARY
        .get(index)
        .map_or("my picture book", |book| book.title(korean))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Companion, Flower, Keepsake, Outcome, Seedling, Sky};
    use jiff::civil::Date;

    fn awake() -> World {
        let mut world = World::new(17, 12.0);
        world.life.begin_day(&world.date.to_string());
        world
    }

    fn hold(world: &mut World, wish: Wish) {
        world.life.wish = Some(Wishing {
            wish,
            day: world.date.to_string(),
            said: None,
        });
    }

    fn companion() -> Companion {
        Companion {
            name: "Mallow".to_owned(),
            hue: 0.5,
            visits: 1,
            closeness: 0.2,
        }
    }

    #[test]
    fn seasons_filter_wishes_using_the_passers_own_calendar() {
        let mut world = awake();
        for month in 1..=12 {
            world.date = Date::new(2026, month, 15).unwrap();
            assert_eq!(
                Wish::Rainbow.eligible(&world),
                world.season() != Season::Winter
            );
            // Only winter brings snow, so a snow wish waits for it.
            assert_eq!(
                Wish::Snow.eligible(&world),
                world.season() == Season::Winter
            );
            for kind in Passerby::ALL {
                assert_eq!(
                    Wish::Passerby(kind).eligible(&world),
                    kind.in_season(world.date)
                );
            }
        }
        world.date = Date::constant(2026, 9, 25);
        assert!(Wish::Passerby(Passerby::Lanterns).eligible(&world));
        world.date = Date::constant(2026, 9, 29);
        assert!(!Wish::Passerby(Passerby::Lanterns).eligible(&world));
    }

    #[test]
    fn age_bond_names_friend_trust_and_garden_filter_the_choices() {
        let mut world = awake();
        assert!(Wish::ViewerName.eligible(&world));
        for wish in [
            Wish::GivenName,
            Wish::Game,
            Wish::WordGame,
            Wish::Drawing,
            Wish::Petting,
            Wish::Fetch,
            Wish::Story,
            Wish::Company,
            Wish::Breeze,
        ] {
            assert!(!wish.eligible(&world), "{wish:?}");
        }
        world.life.bond = 0.3;
        for wish in [
            Wish::GivenName,
            Wish::Game,
            Wish::WordGame,
            Wish::Drawing,
            Wish::Petting,
            Wish::Fetch,
            Wish::Story,
            Wish::Company,
            Wish::Breeze,
        ] {
            assert!(wish.eligible(&world), "{wish:?}");
        }
        world.life.viewer_name = Some("Robin".to_owned());
        assert!(!Wish::ViewerName.eligible(&world));
        world.life.note(&EventKind::Named("Pip".to_owned()));
        assert!(!Wish::GivenName.eligible(&world));
        let friend = Wish::Friend {
            name: "Mallow".to_owned(),
        };
        assert!(!friend.eligible(&world));
        world.life.friend = Some(companion());
        assert!(friend.eligible(&world));
        assert!(!Wish::BirdGift.eligible(&world));
        world.life.bird_trust = 1.0 / 3.0;
        assert!(Wish::BirdGift.eligible(&world));
        assert!(!Wish::Flower.eligible(&world));
        world.plant.seeds = 1;
        assert!(Wish::Flower.eligible(&world));
        world.plant.seeds = 0;
        world.seedlings.push(Seedling::new(Flower::Daisy));
        assert!(Wish::Flower.eligible(&world));
        world.seedlings[0].growth = 1.0;
        assert!(!Wish::Flower.eligible(&world));
        world.weather.sky = Sky::Rain;
        assert!(!Wish::Breeze.eligible(&world));
        world.weather.sky = Sky::Clear;
        world.hour = 23.0;
        assert!(!Wish::Breeze.eligible(&world));
        world.date = Date::constant(2026, 12, 10);
        world.hour = 12.0;
        assert!(!Wish::BirdGift.eligible(&world));
        assert!(!Wish::Breeze.eligible(&world));
        world.birthday = Some(world.date);
        assert!(world.wish_options().is_empty());
        world.date = Date::constant(2026, 12, 11);
        let choices = world.wish_options();
        assert!(choices.contains(&Wish::Tower { height: 2 }));
        assert!(choices.iter().all(|wish| matches!(
            wish,
            Wish::ShootingStar | Wish::Rainbow | Wish::Petting | Wish::Tower { height: 2 }
        )));
        world.date = Date::constant(2026, 12, 13);
        assert!(world.wish_options().contains(&Wish::Constellation));
        for tallest in 0..=5 {
            world.life.total.tallest = tallest;
            assert!(world.wish_options().contains(&Wish::Tower {
                height: (tallest + 1).clamp(3, 5)
            }));
        }
    }

    #[test]
    fn choices_use_one_world_random_draw_and_exclude_the_last_two_kinds() {
        let mut selected = Vec::new();
        for seed in 0..64 {
            let mut world = World::new(seed, 12.0);
            let mut same = world.clone();
            let mut random = world.random.clone();
            random.unit();
            world.make_wish(false);
            same.make_wish(false);
            assert_eq!(world.life.wish, same.life.wish);
            assert_eq!(world.random.unit(), random.unit());
            selected.push(world.life.wish.unwrap().wish);
        }
        assert!(selected.windows(2).any(|pair| pair[0] != pair[1]));
        let mut world = awake();
        world.life.granted = vec![
            Granted {
                wish: Wish::Rainbow,
                day: world.date.to_string(),
            },
            Granted {
                wish: Wish::Tower { height: 3 },
                day: world.date.to_string(),
            },
            Granted {
                wish: Wish::Passerby(Passerby::Kite),
                day: world.date.to_string(),
            },
        ];
        let choices = world.wish_options();
        assert!(choices.contains(&Wish::Rainbow));
        assert!(
            !choices
                .iter()
                .any(|wish| matches!(wish, Wish::Tower { .. } | Wish::Passerby(_)))
        );
    }

    #[test]
    fn nature_weights_hope_without_changing_eligibility() {
        let mut world = awake();
        world.life.bond = 0.5;
        world.life.nature = Some(super::super::Nature {
            curious: 0.0,
            playful: 0.0,
            tender: 0.0,
            sleepy: 0.0,
            shy: 0.0,
        });
        let choices = world.wish_options();
        let wishes = [
            Wish::Constellation,
            Wish::Tower { height: 3 },
            Wish::Petting,
            Wish::Story,
        ];
        let before = wishes.each_ref().map(|wish| wish.weight(&world));
        world.life.nature = Some(super::super::Nature {
            curious: 1.0,
            playful: 1.0,
            tender: 1.0,
            sleepy: 1.0,
            shy: 0.0,
        });
        for (wish, weight) in wishes.iter().zip(before) {
            assert!(wish.weight(&world) > weight);
        }
        let bold = Wish::Game.weight(&world);
        world.life.nature.as_mut().unwrap().shy = 1.0;
        assert_eq!(Wish::Game.weight(&world), bold * 0.5);
        assert_eq!(world.wish_options(), choices);
    }

    fn grants() -> Vec<(Wish, EventKind)> {
        vec![
            (
                Wish::Rainbow,
                EventKind::Noticed("the rainbow through the window".to_owned()),
            ),
            (
                Wish::Snow,
                EventKind::Noticed("the falling snow".to_owned()),
            ),
            (Wish::ShootingStar, EventKind::ShootingStar),
            (
                Wish::Constellation,
                EventKind::FoundConstellation("a heart".to_owned()),
            ),
            (
                Wish::Passerby(Passerby::Kite),
                EventKind::SawPasserby(Passerby::Kite),
            ),
            (Wish::BirdGift, EventKind::Kept(Keepsake::Feather)),
            (Wish::Flower, EventKind::Flowered(Flower::Daisy)),
            (
                Wish::Friend {
                    name: "Mallow".to_owned(),
                },
                EventKind::FriendArrived("Mallow".to_owned()),
            ),
            (Wish::Tower { height: 4 }, EventKind::Built { height: 4 }),
            (Wish::ViewerName, EventKind::LearnedName("Robin".to_owned())),
            (Wish::GivenName, EventKind::Named("Pip".to_owned())),
            (Wish::Game, EventKind::GameOver(Outcome::Draw)),
            (
                Wish::WordGame,
                EventKind::WordGame {
                    word: "MOON".to_owned(),
                    solved: false,
                },
            ),
            (Wish::Drawing, EventKind::SawDrawing { name: None }),
            (Wish::Petting, EventKind::Petted),
            (Wish::Fetch, EventKind::Fetched),
            (Wish::Story, EventKind::ToldStory),
            (
                Wish::Company,
                EventKind::FocusEnded {
                    minutes: 5,
                    completed: true,
                },
            ),
            (Wish::Breeze, EventKind::OpenedWindow),
            (
                Wish::Book { index: 1 },
                EventKind::FinishedBook(super::super::books::LIBRARY[1].title.0.to_owned()),
            ),
            (Wish::Tune, EventKind::LearnedTune("do-re-mi".to_owned())),
            (
                Wish::AboutYou,
                EventKind::LearnedAboutViewer(super::super::acquaintance::Topic::Color),
            ),
        ]
    }

    #[test]
    fn an_unread_book_and_a_tune_from_the_viewer_can_be_wished_for() {
        let mut world = awake();
        world.date = Date::constant(2026, 12, 13);
        world.life.bond = 0.6;
        let unread = world
            .life
            .books
            .owned
            .iter()
            .find(|owned| owned.finished == 0)
            .unwrap()
            .index;
        let choices = world.wish_options();
        assert!(choices.contains(&Wish::Book { index: unread }));
        assert!(choices.contains(&Wish::Tune));
        let book = Wish::Book { index: unread };
        assert!(book.serves(Activity::Read, &world));
        assert!(!book.with_viewer());
        assert!(Wish::Tune.with_viewer());
        assert!(Wish::Tune.hint().unwrap().contains("do re mi"));
        let korean = book.korean(&world.life);
        assert!(korean.contains(super::super::books::LIBRARY[unread].title(true)));
        assert!(korean.ends_with("끝까지 읽는 것"));
        for owned in &mut world.life.books.owned {
            owned.finished = 1;
        }
        assert!(
            !world
                .wish_options()
                .iter()
                .any(|wish| matches!(wish, Wish::Book { .. }))
        );
        assert!(choices.contains(&Wish::AboutYou));
        assert!(Wish::AboutYou.with_viewer());
        world.life.bond = 0.2;
        assert!(!world.wish_options().contains(&Wish::AboutYou));
        assert!(!Wish::Book { index: 99 }.valid(&world.life));
        assert!(!Wish::Tower { height: 6 }.valid(&world.life));
    }

    #[test]
    fn every_wish_matches_its_actual_event_and_rejects_unrelated_events() {
        let pairs = grants();
        for (wish, event) in &pairs {
            assert!(wish.granted_by(event), "{wish:?}");
            for (_, other) in &pairs {
                if other != event {
                    assert!(!wish.granted_by(other), "{wish:?}: {other:?}");
                }
            }
        }
        assert!(!Wish::Game.granted_by(&EventKind::GameOver(Outcome::Abandoned)));
        assert!(!Wish::Company.granted_by(&EventKind::FocusEnded {
            minutes: 5,
            completed: false
        }));
        assert!(!Wish::Rainbow.granted_by(&EventKind::RainbowAppeared));
        assert!(!Wish::Flower.granted_by(&EventKind::Bloomed));
        assert!(!Wish::BirdGift.granted_by(&EventKind::GiftLeft(Keepsake::Feather)));
        assert!(!Wish::Tower { height: 4 }.granted_by(&EventKind::Built { height: 3 }));
        assert!(Wish::Tower { height: 4 }.granted_by(&EventKind::Built { height: 5 }));
        for outcome in [
            Outcome::GhostWon { line: [0, 1, 2] },
            Outcome::ViewerWon { line: [0, 1, 2] },
        ] {
            assert!(Wish::Game.granted_by(&EventKind::GameOver(outcome)));
        }
        assert!(Wish::WordGame.granted_by(&EventKind::WordGame {
            word: "STAR".to_owned(),
            solved: true
        }));
    }

    #[test]
    fn a_grant_follows_its_cause_counts_once_and_keeps_its_day() {
        for (wish, cause) in grants() {
            let mut world = awake();
            hold(&mut world, wish.clone());
            world.record(cause.clone());
            assert_eq!(world.events[0].kind, cause);
            assert_eq!(world.events[1].kind, EventKind::WishGranted(wish.clone()));
            assert!(world.life.wish.is_none());
            assert_eq!(
                world.life.granted,
                vec![Granted {
                    wish,
                    day: world.date.to_string()
                }]
            );
            assert_eq!(world.life.total.wishes_granted, 1);
            assert_eq!(world.life.today.wishes_granted, 1);
            assert!(
                world
                    .life
                    .today
                    .words()
                    .to_lowercase()
                    .contains("a wish came true")
            );
            world.record(cause);
            assert_eq!(world.life.total.wishes_granted, 1);
            assert_eq!(
                world
                    .events
                    .iter()
                    .filter(|event| matches!(event.kind, EventKind::WishGranted(_)))
                    .count(),
                1
            );
        }
    }

    #[test]
    fn first_and_fifth_grants_mark_milestones_once_and_only_viewer_wishes_warm_the_bond() {
        let mut world = awake();
        for _ in 0..16 {
            hold(&mut world, Wish::Tower { height: 3 });
            world.record(EventKind::Built { height: 3 });
        }
        assert_eq!(world.life.total.wishes_granted, 16);
        assert_eq!(world.life.granted.len(), 12);
        for text in [
            "My first wish came true.",
            "Five of my wishes have come true.",
        ] {
            assert_eq!(
                world
                    .life
                    .milestones
                    .iter()
                    .filter(|entry| entry.text == text)
                    .count(),
                1
            );
            assert_eq!(
                world
                    .events
                    .iter()
                    .filter(|event| event.kind == EventKind::Milestone(text.to_owned()))
                    .count(),
                1
            );
        }
        for (wish, _) in grants() {
            let mut life = Life {
                bond: 0.4,
                closest: 0.4,
                ..Life::default()
            };
            life.note(&EventKind::WishGranted(wish.clone()));
            // A bond already this deep warms by the rest of the way squared.
            let expected = if wish.with_viewer() {
                0.4 + 0.01 * 0.6 * 0.6
            } else {
                0.4
            };
            assert!((life.bond - expected).abs() < 0.00001);
        }
    }

    #[test]
    fn calendar_and_morning_make_one_wish_but_not_during_the_first_day_or_sleep() {
        let mut world = awake();
        world.birthday = Some(world.date);
        world.step(0.0);
        world.record(EventKind::Morning);
        assert!(world.life.wish.is_none());
        world.date = Date::constant(2026, 6, 22);
        world.step(0.0);
        assert!(world.life.wish.is_some());
        assert_eq!(world.life.total.wishes_made, 1);
        assert!(
            world
                .life
                .today
                .words()
                .to_lowercase()
                .contains("made a wish ")
        );
        world.record(EventKind::Morning);
        world.step(0.0);
        assert_eq!(world.life.total.wishes_made, 1);
        let mut sleeping = awake();
        sleeping.ghost.asleep = 1.0;
        sleeping.step(0.0);
        assert!(sleeping.life.wish.is_none());
        sleeping.ghost.asleep = 0.0;
        sleeping.record(EventKind::Morning);
        assert!(sleeping.life.wish.is_some());
    }

    #[test]
    fn granting_waits_for_the_next_day_but_a_later_star_can_prompt_a_new_wish() {
        let mut world = awake();
        hold(&mut world, Wish::Petting);
        world.record(EventKind::Petted);
        world.step(0.0);
        world.record(EventKind::Morning);
        assert!(world.life.wish.is_none());
        world.record(EventKind::ShootingStar);
        assert!(world.life.wish.is_some());
        assert_eq!(world.life.total.wishes_made, 1);
        let wish = world.life.wish.clone();
        world.record(EventKind::ShootingStar);
        if wish.as_ref().unwrap().wish != Wish::ShootingStar {
            assert_eq!(world.life.wish, wish);
            assert!(
                world
                    .ghost
                    .thought
                    .as_ref()
                    .unwrap()
                    .text
                    .starts_with("I wished again:")
            );
            assert_eq!(world.life.total.wishes_made, 1);
        }
        hold(&mut world, Wish::Tower { height: 3 });
        world.record(EventKind::Built { height: 3 });
        let saved = world.life.clone();
        let mut restored = awake();
        restored.life = saved;
        restored.step(0.0);
        assert!(restored.life.wish.is_none());
        restored.date = Date::constant(2026, 6, 22);
        restored.step(0.0);
        assert!(restored.life.wish.is_some());
    }

    #[test]
    fn a_star_without_a_wish_makes_one_and_renewing_names_the_held_wish() {
        let mut world = awake();
        world.record(EventKind::ShootingStar);
        assert!(world.life.wish.is_some());
        assert_eq!(world.life.total.wishes_made, 1);
        hold(&mut world, Wish::Rainbow);
        let held = world.life.wish.clone();
        world.record(EventKind::ShootingStar);
        assert_eq!(world.life.wish, held);
        assert_eq!(
            world.ghost.thought.as_ref().unwrap().text,
            "I wished again: to see a rainbow."
        );
        assert_eq!(world.life.total.wishes_made, 1);
        let mut newborn = awake();
        newborn.birthday = Some(newborn.date);
        newborn.record(EventKind::ShootingStar);
        assert!(newborn.life.wish.is_none());
    }

    #[test]
    fn wishes_fade_after_seven_days_or_a_season_change_and_wait_until_tomorrow() {
        let mut world = awake();
        hold(&mut world, Wish::Rainbow);
        world.date = Date::constant(2026, 6, 27);
        world.step(0.0);
        assert!(world.life.wish.is_some());
        world.date = Date::constant(2026, 6, 28);
        world.step(0.0);
        assert!(world.life.wish.is_none());
        assert_eq!(world.life.today.wishes_faded, 1);
        assert!(
            world
                .life
                .today
                .words()
                .to_lowercase()
                .contains("let go of a wish: to see a rainbow")
        );
        world.record(EventKind::Morning);
        assert!(world.life.wish.is_none());
        world.date = Date::constant(2026, 6, 29);
        world.step(0.0);
        assert!(world.life.wish.is_some());
        world.date = Date::constant(2027, 2, 28);
        hold(&mut world, Wish::Snow);
        world.date = Date::constant(2027, 3, 1);
        world.step(0.0);
        assert!(world.life.wish.is_none());
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::WishFaded(Wish::Snow))
        );
    }

    #[test]
    fn serving_activities_follow_the_wish_and_night_sky_waits_for_darkness() {
        let mut world = awake();
        world.notice_viewer();
        assert!(Wish::Rainbow.serves(Activity::Window, &world));
        assert!(!Wish::ShootingStar.serves(Activity::Window, &world));
        world.hour = 23.0;
        assert!(Wish::ShootingStar.serves(Activity::Window, &world));
        assert!(Wish::Constellation.serves(Activity::Window, &world));
        assert!(Wish::Flower.serves(Activity::Tend, &world));
        assert!(Wish::Game.serves(Activity::Visit, &world));
        assert!(Wish::Breeze.serves(Activity::Air, &world));
        world.start_focus(5);
        hold(&mut world, Wish::Game);
        assert!(!world.options().contains(&Activity::Visit));
    }

    #[test]
    fn wishes_are_shared_once_a_day_at_the_glass_with_a_hint_but_never_during_focus_or_play() {
        let mut world = awake();
        world.notice_viewer();
        hold(&mut world, Wish::Game);
        world.begin(Activity::Visit);
        assert!(world.share_wish().is_none());
        world.ghost.plan.arrived = Some(world.time);
        let notice = world.share_wish().unwrap();
        assert!(notice.contains("press g"));
        assert!(world.ghost.speech.as_ref().unwrap().text.contains("I wish"));
        assert!(world.share_wish().is_none());
        world.date = Date::constant(2026, 6, 22);
        world.time += 10.0;
        world.life.korean = 1.0;
        assert!(world.share_wish().is_some());
        assert!(world.ghost.speech.as_ref().unwrap().text.contains("싶어"));
        world.start_focus(5);
        hold(&mut world, Wish::Petting);
        world.ghost.plan.activity = Activity::Visit;
        world.ghost.plan.arrived = Some(world.time);
        assert!(world.share_wish().is_none());
        assert!(world.life.wish.as_ref().unwrap().said.is_none());
        world.focus = None;
        world.ghost.asleep = 1.0;
        assert!(world.share_wish().is_none());
        world.ghost.asleep = 0.0;
        world.hear("tell me a story");
        assert!(world.share_wish().is_none());
        world.telling = None;
        world.challenge();
        world.ghost.plan.activity = Activity::Visit;
        world.ghost.plan.arrived = Some(world.time);
        world.time += 10.0;
        assert!(world.share_wish().is_none());
    }

    #[test]
    fn celebrations_keep_the_viewers_language_and_survive_other_completion_lines() {
        for korean in [false, true] {
            let mut world = awake();
            world.life.korean = if korean { 1.0 } else { 0.0 };
            hold(&mut world, Wish::Company);
            world.start_focus(5);
            world.time += 300.0;
            world.end_focus();
            let speech = &world.ghost.speech.as_ref().unwrap().text;
            assert!(speech.contains(if korean {
                "소원이 이루어졌어"
            } else {
                "My wish came true"
            }));
            assert!(speech.contains(if korean {
                "쉬는 시간이야"
            } else {
                "Time for a break"
            }));
        }
        let mut world = awake();
        hold(&mut world, Wish::Rainbow);
        world.record(EventKind::Noticed(
            "the rainbow through the window".to_owned(),
        ));
        world.celebrate_wish();
        assert!(world.ghost.speech.is_none());
        assert!(
            world
                .ghost
                .thought
                .as_ref()
                .unwrap()
                .text
                .contains("My wish came true")
        );
        hold(&mut world, Wish::Petting);
        world.ghost.asleep = 1.0;
        world.record(EventKind::Petted);
        world.celebrate_wish();
        assert!(world.ghost.speech.is_none());
    }

    #[test]
    fn save_round_trips_migrate_and_drop_damaged_wishes() {
        let mut world = awake();
        world.life.friend = Some(companion());
        hold(
            &mut world,
            Wish::Friend {
                name: "Mallow".to_owned(),
            },
        );
        world.life.wish.as_mut().unwrap().said = Some(world.date.to_string());
        world.life.granted = grants()
            .into_iter()
            .map(|(wish, _)| Granted {
                wish,
                day: world.date.to_string(),
            })
            .collect();
        let encoded = serde_json::to_string(&world.life).unwrap();
        let loaded: Life = serde_json::from_str(&encoded).unwrap();
        assert_eq!(loaded, world.life);
        assert_eq!(loaded.sanitized().granted.len(), 12);
        let old: Life = serde_json::from_str("{}").unwrap();
        assert!(old.wish.is_none());
        assert!(old.granted.is_empty());
        assert_eq!(old.total.wishes_made, 0);
        for height in [0, 1, 7, u32::MAX] {
            hold(&mut world, Wish::Tower { height });
            assert!(world.life.clone().sanitized().wish.is_none());
        }
        hold(
            &mut world,
            Wish::Friend {
                name: "Mallow".to_owned(),
            },
        );
        world.life.friend = None;
        assert!(world.life.clone().sanitized().wish.is_none());
        hold(&mut world, Wish::Rainbow);
        world.life.wish.as_mut().unwrap().day = "broken".to_owned();
        assert!(world.life.clone().sanitized().wish.is_none());
        hold(&mut world, Wish::Rainbow);
        world.life.wish.as_mut().unwrap().said = Some("broken".to_owned());
        assert!(world.life.clone().sanitized().wish.unwrap().said.is_none());
        world.life.total.names = 0;
        world.life.milestones.push(super::super::Entry {
            day: world.date.to_string(),
            text: "You gave me my name: Pip.".to_owned(),
            korean: None,
        });
        assert_eq!(world.life.sanitized().total.names, 1);
    }

    #[test]
    fn wish_observations_recall_its_age_and_grants_and_all_wishes_use_words() {
        let mut world = awake();
        for (wish, _) in grants() {
            assert!(
                !wish
                    .words(&world.life)
                    .chars()
                    .any(|letter| letter.is_ascii_digit())
            );
            assert!(!wish.korean(&world.life).is_empty());
            let korean = wish.korean_wish(&world.life);
            assert!(korean.ends_with("다는 소원"), "{wish:?}: {korean}");
            let mut life = Life::default();
            for event in [
                EventKind::MadeWish(wish.clone()),
                EventKind::WishGranted(wish.clone()),
                EventKind::WishFaded(wish.clone()),
            ] {
                life.note(&event);
            }
            assert!(!life.today.words_korean().contains(": "));
            assert_eq!(wish.with_viewer(), wish.hint().is_some());
        }
        assert!(
            Wish::Tower { height: 5 }
                .words(&world.life)
                .contains("five")
        );
        hold(&mut world, Wish::Rainbow);
        world.date = Date::constant(2026, 6, 23);
        world.life.begin_day(&world.date.to_string());
        assert!(world.wish_observation().contains("two days ago"));
        world.record(EventKind::Noticed(
            "the rainbow through the window".to_owned(),
        ));
        assert!(
            world
                .wish_observation()
                .contains("A wish came true today: to see a rainbow")
        );
    }
    #[test]
    fn a_garden_flowering_during_an_absence_does_not_grant_a_wish() {
        let mut world = awake();
        hold(&mut world, Wish::Flower);
        let mut seedling = Seedling::new(Flower::Daisy);
        seedling.growth = 0.9999;
        seedling.age = 8.0 * 86_400.0;
        world.seedlings.push(seedling);
        world.age_garden(3600.0);
        assert!(
            world
                .events
                .iter()
                .any(|event| matches!(event.kind, EventKind::Flowered(_)))
        );
        assert!(world.life.wish.is_some());
        assert_eq!(world.life.total.wishes_granted, 0);
        assert!(world.life.granted.is_empty());
    }
}
