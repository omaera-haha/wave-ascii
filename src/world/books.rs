use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::{Activity, EventKind, Mood, Occasion, Random, World};

pub const POSITION: Vec3 = Vec3::new(-1.76, 0.0, -0.25);
pub const RADIUS: f32 = 0.25;
pub const DELIVERY_DAYS: u32 = 4;

type Line = (&'static str, &'static str);

#[derive(Clone, Copy, Debug)]
pub struct Book {
    pub title: Line,
    pub about: Line,
    pub pages: &'static [Line],
}

impl Book {
    pub fn title(self, korean: bool) -> &'static str {
        if korean { self.title.1 } else { self.title.0 }
    }

    pub fn about(self, korean: bool) -> &'static str {
        if korean { self.about.1 } else { self.about.0 }
    }
}

pub const LIBRARY: [Book; 12] = [
    Book {
        title: ("The Lantern Fox", "등불 여우"),
        about: (
            "a fox sharing a little light",
            "작은 빛을 나누는 여우 이야기야",
        ),
        pages: &[
            (
                "A fox carried a lantern through the snow.",
                "여우가 눈 속에서 등불을 들고 걸었어.",
            ),
            (
                "The wind blew its little flame away.",
                "바람이 작은 불꽃을 꺼 버렸어.",
            ),
            (
                "A lost moth offered one golden spark.",
                "길 잃은 나방이 금빛 불씨를 건넸어.",
            ),
            (
                "Together they lit the path to the burrow.",
                "둘은 함께 굴로 가는 길을 밝혔어.",
            ),
            (
                "Inside, there was warm soup for two.",
                "안에는 둘이 먹을 따뜻한 수프가 있었어.",
            ),
        ],
    },
    Book {
        title: ("Seven Small Clouds", "작은 구름 일곱"),
        about: (
            "clouds finding room for a friend",
            "친구 자리를 만드는 구름들 이야기야",
        ),
        pages: &[
            (
                "Seven small clouds slept on a blue hill.",
                "작은 구름 일곱이 파란 언덕에서 잤어.",
            ),
            (
                "A new cloud found no room to lie down.",
                "새 구름은 누울 자리가 없었어.",
            ),
            (
                "The smallest cloud tried to fold itself.",
                "가장 작은 구름이 몸을 접어 봤어.",
            ),
            (
                "Then all seven made a soft cloudy ring.",
                "그러자 일곱 모두 포근한 고리를 만들었어.",
            ),
            (
                "Their new friend dreamed in the middle.",
                "새 친구는 가운데서 꿈을 꾸었어.",
            ),
        ],
    },
    Book {
        title: ("The Moon Who Wanted a Hat", "모자가 갖고 싶은 달"),
        about: (
            "the moon finding a hat that fits",
            "꼭 맞는 모자를 찾는 달 이야기야",
        ),
        pages: &[
            (
                "The moon wished for a cozy hat.",
                "달은 포근한 모자가 갖고 싶었어.",
            ),
            (
                "Every hat on Earth was much too small.",
                "땅 위의 모자는 모두 너무 작았어.",
            ),
            (
                "A spider spun a silver thread upward.",
                "거미가 은빛 실을 하늘로 뽑았어.",
            ),
            (
                "The stars knitted until the morning.",
                "별들이 아침까지 뜨개질을 했어.",
            ),
            (
                "The moon wore a hat full of tiny lights.",
                "달은 작은 빛이 가득한 모자를 썼어.",
            ),
        ],
    },
    Book {
        title: ("A Whale in the Sky", "하늘 고래"),
        about: (
            "a flying whale finding its song",
            "노래를 되찾는 하늘 고래 이야기야",
        ),
        pages: &[
            (
                "A whale sailed between the clouds.",
                "고래가 구름 사이를 헤엄쳤어.",
            ),
            (
                "Its song fell into a sleeping sea.",
                "고래의 노래가 잠든 바다에 떨어졌어.",
            ),
            (
                "It listened, but the sky was quiet.",
                "귀를 기울여도 하늘은 조용했어.",
            ),
            (
                "A shell sent the song back on a breeze.",
                "조개가 바람에 노래를 실어 보냈어.",
            ),
            (
                "The whale sang softly to the sea below.",
                "고래는 아래 바다에 다정하게 노래했어.",
            ),
        ],
    },
    Book {
        title: ("The Pocket Garden", "주머니 정원"),
        about: (
            "a seed growing a home in a pocket",
            "주머니에 집을 짓는 씨앗 이야기야",
        ),
        pages: &[
            (
                "A rabbit kept a seed in its pocket.",
                "토끼가 주머니에 씨앗을 넣었어.",
            ),
            (
                "There was no garden left in the town.",
                "마을에는 정원이 하나도 없었어.",
            ),
            (
                "The rabbit filled its pocket with soil.",
                "토끼가 주머니를 흙으로 채웠어.",
            ),
            (
                "A shy green leaf peeked over the seam.",
                "수줍은 초록 잎이 솔기 위로 나왔어.",
            ),
            (
                "Soon every pocket carried a flower.",
                "곧 모든 주머니에 꽃이 피었어.",
            ),
        ],
    },
    Book {
        title: ("The Sleepy Lighthouse", "졸린 등대"),
        about: (
            "friends helping a tired lighthouse",
            "지친 등대를 돕는 친구들 이야기야",
        ),
        pages: &[
            (
                "A lighthouse blinked beside the sea.",
                "등대가 바닷가에서 깜빡였어.",
            ),
            (
                "One night it was too tired to shine.",
                "어느 밤에는 너무 피곤해 빛을 못 냈어.",
            ),
            (
                "The little boats huddled in the dark.",
                "작은 배들이 어둠 속에 모였어.",
            ),
            (
                "Fireflies gathered at its window.",
                "반딧불이들이 등대 창에 모였어.",
            ),
            (
                "Their glow brought every boat home.",
                "그 빛이 모든 배를 집으로 데려왔어.",
            ),
            (
                "The lighthouse slept until sunrise.",
                "등대는 해가 뜰 때까지 푹 잤어.",
            ),
        ],
    },
    Book {
        title: ("A Scarf for the River", "강물의 목도리"),
        about: (
            "an otter warming a chilly river",
            "추운 강물을 감싸 주는 수달 이야기야",
        ),
        pages: &[
            (
                "An otter heard the river shiver.",
                "수달이 강물 떠는 소리를 들었어.",
            ),
            (
                "Its wool scarf floated far away.",
                "수달의 털목도리는 멀리 떠내려갔어.",
            ),
            (
                "The reeds whispered of fallen leaves.",
                "갈대가 낙엽 이야기를 속삭였어.",
            ),
            (
                "The otter wove a long golden scarf.",
                "수달은 긴 금빛 목도리를 엮었어.",
            ),
            (
                "The river wore it gently into spring.",
                "강물은 봄까지 목도리를 살며시 둘렀어.",
            ),
        ],
    },
    Book {
        title: ("The Star in the Teacup", "찻잔 속 별"),
        about: (
            "a small star learning to shine nearby",
            "곁에서 빛나는 법을 배우는 별 이야기야",
        ),
        pages: &[
            (
                "A small star landed in a teacup.",
                "작은 별이 찻잔에 내려앉았어.",
            ),
            (
                "It could not jump back to the sky.",
                "아무리 뛰어도 하늘에 닿지 못했어.",
            ),
            (
                "A mouse brought it a blanket of steam.",
                "생쥐가 김으로 만든 이불을 가져왔어.",
            ),
            (
                "The star warmed the mouse's cold paws.",
                "별이 생쥐의 찬 발을 녹여 주었어.",
            ),
            (
                "That night the table had its own sky.",
                "그날 밤 식탁에도 하늘이 생겼어.",
            ),
        ],
    },
    Book {
        title: ("The Snail's Little Post", "달팽이 우체국"),
        about: (
            "a slow letter arriving just in time",
            "꼭 맞는 때에 도착한 느린 편지 이야기야",
        ),
        pages: &[
            (
                "A snail carried a letter up a hill.",
                "달팽이가 편지를 지고 언덕을 올랐어.",
            ),
            (
                "The birthday picnic ended before noon.",
                "생일 소풍은 점심 전에 끝나 버렸어.",
            ),
            (
                "The snail worried that it was too late.",
                "달팽이는 너무 늦었다고 걱정했어.",
            ),
            (
                "At dusk a lonely mole opened the door.",
                "해 질 녘 외로운 두더지가 문을 열었어.",
            ),
            (
                "The letter said, I saved you some cake.",
                "편지에는 네 케이크를 남겨 뒀다고 쓰였어.",
            ),
            (
                "They shared the last slice under a star.",
                "둘은 별 아래서 마지막 조각을 나눴어.",
            ),
        ],
    },
    Book {
        title: ("The Umbrella Tree", "우산 나무"),
        about: (
            "a bare tree making shelter together",
            "함께 쉼터를 만드는 나무 이야기야",
        ),
        pages: &[
            (
                "A bare tree wanted to shelter the birds.",
                "잎 없는 나무가 새들을 품고 싶었어.",
            ),
            (
                "Rain slipped through every branch.",
                "가지 사이마다 빗방울이 떨어졌어.",
            ),
            (
                "A sparrow found an old red umbrella.",
                "참새가 낡은 빨간 우산을 찾았어.",
            ),
            (
                "The birds lifted it into the branches.",
                "새들이 우산을 나뭇가지 위로 올렸어.",
            ),
            (
                "Under their roof, the tree felt leafy.",
                "그 지붕 아래서 나무는 잎이 난 듯했어.",
            ),
        ],
    },
    Book {
        title: ("The Quietest Bell", "가장 조용한 종"),
        about: (
            "a quiet bell finding someone to hear it",
            "귀 기울일 친구를 찾는 작은 종 이야기야",
        ),
        pages: &[
            (
                "A little bell lived in a busy station.",
                "작은 종이 북적이는 역에 살았어.",
            ),
            (
                "No one could hear its tiny chime.",
                "아무도 작은 종소리를 못 들었어.",
            ),
            (
                "It rolled away to an empty garden.",
                "종은 텅 빈 정원으로 굴러갔어.",
            ),
            (
                "A sleeping seed heard it through the soil.",
                "잠든 씨앗이 흙 너머로 그 소리를 들었어.",
            ),
            (
                "A flower opened to listen every day.",
                "꽃은 날마다 귀 기울이려고 피어났어.",
            ),
        ],
    },
    Book {
        title: ("A Boat for the Morning", "아침을 위한 배"),
        about: (
            "a paper boat bringing the morning home",
            "아침을 데려오는 종이배 이야기야",
        ),
        pages: &[
            (
                "A paper boat waited in a dark puddle.",
                "종이배가 어두운 웅덩이에서 기다렸어.",
            ),
            (
                "It feared the morning had lost its way.",
                "아침이 길을 잃었을까 봐 걱정했어.",
            ),
            (
                "A frog placed a dewdrop on its bow.",
                "개구리가 뱃머리에 이슬을 올렸어.",
            ),
            (
                "The first sunbeam sparkled in the drop.",
                "첫 햇살이 이슬 속에서 반짝였어.",
            ),
            (
                "The boat brought morning to the reeds.",
                "종이배가 갈대에게 아침을 데려왔어.",
            ),
        ],
    },
];

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct OwnedBook {
    #[serde(default)]
    pub index: usize,
    #[serde(default)]
    pub page: usize,
    #[serde(default)]
    pub finished: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Books {
    #[serde(default)]
    pub owned: Vec<OwnedBook>,
    #[serde(default)]
    pub deliveries: u32,
    #[serde(default)]
    pub birthday_year: Option<i16>,
}

impl Default for Books {
    fn default() -> Self {
        Self {
            owned: (0..3)
                .map(|index| OwnedBook {
                    index,
                    ..OwnedBook::default()
                })
                .collect(),
            deliveries: 0,
            birthday_year: None,
        }
    }
}

impl Books {
    pub fn sanitize(&mut self) {
        let mut seen = [false; LIBRARY.len()];
        self.owned.retain_mut(|owned| {
            if owned.index >= LIBRARY.len() || seen[owned.index] {
                return false;
            }
            seen[owned.index] = true;
            owned.page = owned.page.min(LIBRARY[owned.index].pages.len());
            owned.finished = owned.finished.min(1_000_000);
            true
        });
        if self.owned.is_empty() {
            self.owned = Self::default().owned;
        }
        self.deliveries = self.deliveries.min(9);
        self.birthday_year = self
            .birthday_year
            .filter(|year| (-9999..=9999).contains(year));
    }

    pub fn favorite(&self) -> Option<usize> {
        self.owned
            .iter()
            .max_by_key(|owned| owned.finished)
            .map(|owned| owned.index)
    }

    pub fn titles(&self, finished: bool, korean: bool) -> Vec<&'static str> {
        self.owned
            .iter()
            .filter(|owned| !finished || owned.finished > 0)
            .map(|owned| LIBRARY[owned.index].title(korean))
            .collect()
    }

    pub fn words(&self) -> String {
        let owned = self.titles(false, false).join(", ");
        let finished = self.titles(true, false);
        format!(
            "owns {owned}; finished {}",
            if finished.is_empty() {
                "none yet".to_owned()
            } else {
                finished.join(", ")
            }
        )
    }
}

#[derive(Clone, Debug)]
pub struct Reading {
    pub index: usize,
    pub next_at: Option<f64>,
    pub remaining: u32,
    pub turned_at: Option<f64>,
    pub lighting: bool,
}

impl World {
    pub(super) fn deliver_books(&mut self) {
        let due = self
            .age_days()
            .map_or(0, |days| days as u32 / DELIVERY_DAYS)
            .min(9);
        while self.life.books.deliveries < due {
            self.life.books.deliveries += 1;
            self.deliver_book();
        }
        if self.occasion() == Some(Occasion::Birthday)
            && self.life.books.birthday_year != Some(self.date.year())
        {
            self.life.books.birthday_year = Some(self.date.year());
            self.deliver_book();
        }
    }

    fn deliver_book(&mut self) {
        let mut order: Vec<usize> = (3..LIBRARY.len()).collect();
        let mut random = Random::new(self.meteor_seed ^ 0x0062_6f6f_6b73);
        for end in (1..order.len()).rev() {
            let chosen = (random.unit() * (end + 1) as f32) as usize;
            order.swap(end, chosen);
        }
        if let Some(index) = order.into_iter().find(|index| {
            !self
                .life
                .books
                .owned
                .iter()
                .any(|owned| owned.index == *index)
        }) {
            self.life.books.owned.push(OwnedBook {
                index,
                ..OwnedBook::default()
            });
            self.record(EventKind::NewBook(LIBRARY[index].title.0.to_owned()));
        }
    }

    pub fn current_book(&self) -> Option<usize> {
        self.reading
            .as_ref()
            .map(|reading| reading.index)
            .or_else(|| self.telling.as_ref().and_then(|telling| telling.book))
            .or(self.last_read)
            .or_else(|| {
                self.life
                    .books
                    .owned
                    .iter()
                    .find(|owned| owned.page < LIBRARY[owned.index].pages.len())
                    .map(|owned| owned.index)
            })
            .or_else(|| self.life.books.favorite())
    }

    pub fn reading_words(&self) -> String {
        let Some(index) = self
            .reading
            .as_ref()
            .map(|reading| reading.index)
            .or_else(|| self.telling.as_ref().and_then(|telling| telling.book))
        else {
            return "no book open just now".to_owned();
        };
        let owned = self
            .life
            .books
            .owned
            .iter()
            .find(|owned| owned.index == index)
            .unwrap();
        format!(
            "reading {}; {} of {} pages read",
            LIBRARY[index].title.0,
            super::friend::count_words(owned.page as u32),
            super::friend::count_words(LIBRARY[index].pages.len() as u32)
        )
    }

    pub(super) fn begin_reading(&mut self, bedtime: bool) {
        let unfinished: Vec<usize> = self
            .life
            .books
            .owned
            .iter()
            .filter(|owned| owned.page < LIBRARY[owned.index].pages.len())
            .map(|owned| owned.index)
            .collect();
        let index = self
            .pick(&vec![1.0; unfinished.len()])
            .map(|chosen| unfinished[chosen])
            .or_else(|| self.life.books.favorite());
        let Some(index) = index else {
            self.ghost.plan.done = true;
            return;
        };
        self.prepare_book(index);
        self.reading = Some(Reading {
            index,
            next_at: None,
            remaining: if bedtime {
                1
            } else {
                self.random.range(2.0, 5.0) as u32
            },
            turned_at: None,
            lighting: self.is_night() && !self.lamp.on,
        });
        if self.reading.as_ref().unwrap().lighting {
            self.ghost.plan.target = self.lamp.bulb() + Vec3::new(0.35, -0.15, 0.25);
            self.ghost.plan.arrived = None;
        }
    }

    fn prepare_book(&mut self, index: usize) {
        self.last_read = Some(index);
        if let Some(owned) = self
            .life
            .books
            .owned
            .iter_mut()
            .find(|owned| owned.index == index)
            && owned.page >= LIBRARY[index].pages.len()
        {
            owned.page = 0;
        }
    }

    pub(super) fn read_page(&mut self, index: usize, thought: bool) -> bool {
        let Some(owned) = self
            .life
            .books
            .owned
            .iter_mut()
            .find(|owned| owned.index == index)
        else {
            return true;
        };
        let book = LIBRARY[index];
        let Some(page) = book.pages.get(owned.page) else {
            return true;
        };
        owned.page += 1;
        let finished = owned.page == book.pages.len();
        if finished {
            owned.finished = owned.finished.saturating_add(1);
        }
        let owned_finished = owned.finished;
        if thought {
            self.think(&format!("\"{}\"", page.0));
        }
        self.record(EventKind::ReadPage);
        if finished {
            // Rereading a favorite is a quiet pleasure; only the first time
            // through is an event worth telling.
            if owned_finished == 1 {
                self.record(EventKind::FinishedBook(book.title.0.to_owned()));
            }
            self.suggest_mood(Mood::Content, 10.0);
        }
        self.ghost.needs.change(0.0, 0.025, 0.0);
        finished
    }

    pub(super) fn carry_out_read(&mut self) -> Option<Vec3> {
        let Some(reading) = self.reading.as_ref() else {
            self.ghost.plan.done = true;
            return None;
        };
        if reading.lighting {
            if self.light_for_reading() {
                self.reading.as_mut().unwrap().lighting = false;
            }
            return Some(self.lamp.bulb());
        }
        if self.ghost.plan.arrived.is_none() {
            return Some(self.ghost.plan.target);
        }
        if reading.next_at.is_none() {
            let next_at = self.time + f64::from(self.random.range(15.0, 25.0));
            self.reading.as_mut().unwrap().next_at = Some(next_at);
        }
        let reading = self.reading.as_ref().unwrap();
        if self.time >= reading.next_at.unwrap() {
            let finished = self.read_page(reading.index, true);
            let reading = self.reading.as_mut().unwrap();
            reading.remaining -= 1;
            reading.turned_at = Some(self.time);
            reading.next_at = Some(self.time + f64::from(self.random.range(15.0, 25.0)));
            if finished || reading.remaining == 0 {
                self.reading = None;
                if self.ghost.plan.activity == Activity::Read {
                    self.ghost.plan.done = true;
                }
            }
        }
        Some(self.ghost.position - Vec3::Y * 0.3 * self.size())
    }

    pub(super) fn light_for_reading(&mut self) -> bool {
        if self.ghost.plan.stayed(self.time) <= 0.6 {
            return false;
        }
        if !self.lamp.on {
            self.lamp.on = true;
            self.record(EventKind::SwitchedLamp { on: true });
        }
        self.ghost.plan.target = self.ghost.plan.anchor;
        self.ghost.plan.arrived = None;
        true
    }

    pub fn held_book(&self) -> Option<(usize, Vec3, f32)> {
        let (index, turned_at) = if let Some(reading) = &self.reading {
            if reading.lighting || reading.next_at.is_none() {
                return None;
            }
            (reading.index, reading.turned_at)
        } else {
            let telling = self.telling.as_ref()?;
            if telling.lighting {
                return None;
            }
            (
                telling.book?,
                (telling.next > 0).then(|| {
                    self.ghost
                        .speech
                        .as_ref()
                        .map_or(self.time, |line| line.time)
                }),
            )
        };
        let turning = turned_at.map_or(0.0, |turned| {
            let elapsed = (self.time - turned) as f32;
            if (0.0..1.2).contains(&elapsed) {
                elapsed / 1.2
            } else {
                0.0
            }
        });
        Some((
            index,
            self.ghost.position - Vec3::Y * 0.28 * self.size(),
            turning,
        ))
    }

    pub(super) fn read_aloud(&mut self, korean: bool) {
        if self.telling.is_some() {
            return;
        }
        let Some(index) = self.current_book() else {
            return;
        };
        self.prepare_book(index);
        let page = self
            .life
            .books
            .owned
            .iter()
            .find(|owned| owned.index == index)
            .unwrap()
            .page;
        let lines = LIBRARY[index].pages[page..]
            .iter()
            .map(|line| if korean { line.1 } else { line.0 }.to_owned())
            .collect();
        self.tell(lines);
        let lighting = self.is_night() && !self.lamp.on;
        let telling = self.telling.as_mut().unwrap();
        telling.book = Some(index);
        telling.lighting = lighting;
        if lighting {
            self.ghost.plan.target = self.lamp.bulb() + Vec3::new(0.35, -0.15, 0.25);
            self.ghost.plan.arrived = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Life, Question, Thing, Understanding};
    use unicode_width::UnicodeWidthStr;

    #[test]
    fn twelve_unique_bilingual_books_have_short_complete_pages() {
        assert_eq!(LIBRARY.len(), 12);
        let mut english = std::collections::BTreeSet::new();
        let mut korean = std::collections::BTreeSet::new();
        for book in LIBRARY {
            assert!(english.insert(book.title.0));
            assert!(korean.insert(book.title.1));
            assert!((5..=6).contains(&book.pages.len()));
            assert!(!book.about.0.is_empty() && !book.about.1.is_empty());
            for (english, korean) in book.pages {
                assert!(!english.is_empty() && !korean.is_empty());
                assert!(english.width() <= 52, "{english}");
                assert!(korean.width() <= 52, "{korean}");
                assert!(english.ends_with('.') && korean.ends_with('.'));
            }
        }
    }

    #[test]
    fn old_lives_start_with_three_books_and_damaged_books_are_repaired() {
        let old: Life = serde_json::from_str("{}").unwrap();
        assert_eq!(old.books, Books::default());
        assert_eq!(
            old.books
                .owned
                .iter()
                .map(|owned| owned.index)
                .collect::<Vec<_>>(),
            [0, 1, 2]
        );
        let mut life = old;
        life.books.owned.extend([
            OwnedBook {
                index: 0,
                page: 3,
                finished: 2,
            },
            OwnedBook {
                index: 12,
                page: 0,
                finished: 0,
            },
            OwnedBook {
                index: 3,
                page: usize::MAX,
                finished: u32::MAX,
            },
        ]);
        life.books.deliveries = u32::MAX;
        let clean = life.sanitized();
        assert_eq!(clean.books.owned.len(), 4);
        let last = clean.books.owned.last().unwrap();
        assert_eq!(last.page, LIBRARY[3].pages.len());
        assert_eq!(last.finished, 1_000_000);
        assert_eq!(clean.books.deliveries, 9);
        let saved = serde_json::to_string(&clean).unwrap();
        assert_eq!(
            serde_json::from_str::<Life>(&saved).unwrap().sanitized(),
            clean
        );
    }

    #[test]
    fn deliveries_follow_life_days_in_seeded_order_and_survive_loading() {
        let mut world = World::new(8, 12.0);
        world.birthday = Some(world.date);
        world.step(0.0);
        assert_eq!(world.life.books.owned.len(), 3);
        world.date = world.date.checked_add(jiff::Span::new().days(3)).unwrap();
        world.step(0.0);
        assert_eq!(world.life.books.owned.len(), 3);
        world.date = world.date.checked_add(jiff::Span::new().days(1)).unwrap();
        world.step(0.0);
        assert_eq!(world.life.books.owned.len(), 4);
        assert_eq!(world.life.total.books_received, 1);
        world.step(0.0);
        assert_eq!(world.life.books.owned.len(), 4);
        let saved = serde_json::to_string(&world.life).unwrap();
        let mut restored = World::new(8, 12.0);
        restored.birthday = world.birthday;
        restored.date = world.date;
        restored.life = serde_json::from_str::<Life>(&saved).unwrap().sanitized();
        restored.step(0.0);
        assert_eq!(restored.life.books, world.life.books);
        for days in [4, 4, 40] {
            world.date = world
                .date
                .checked_add(jiff::Span::new().days(days))
                .unwrap();
            restored.date = world.date;
            world.step(0.0);
            restored.step(0.0);
            assert_eq!(world.life.books, restored.life.books);
        }
        assert_eq!(world.life.books.owned.len(), 12);
        assert_eq!(world.life.total.books_received, 9);
    }

    #[test]
    fn birthdays_bring_one_extra_book_once_even_across_restarts() {
        let mut world = World::new(19, 12.0);
        world.date = jiff::civil::Date::constant(2027, 2, 28);
        world.birthday = Some(jiff::civil::Date::constant(2024, 2, 29));
        world.life.books.deliveries = 9;
        world.step(0.0);
        assert_eq!(world.life.books.owned.len(), 4);
        assert!(world.events.iter().any(|event| matches!(&event.kind, EventKind::NewBook(title) if LIBRARY.iter().any(|book| book.title.0 == title))));
        world.calendar_at = None;
        world.step(0.0);
        assert_eq!(world.life.books.owned.len(), 4);
        for year in 2028..2045 {
            world.date = jiff::civil::Date::new(year, 2, 28).unwrap();
            // The leap-day anniversary is the next day in leap years.
            if world.occasion() != Some(Occasion::Birthday) {
                world.date = jiff::civil::Date::new(year, 2, 29).unwrap();
            }
            world.step(0.0);
        }
        assert_eq!(world.life.books.owned.len(), 12);
    }

    fn settled_reader() -> World {
        let mut world = World::new(5, 12.0);
        world.begin(Activity::Read);
        world.ghost.position = world.ghost.plan.target;
        world.step(0.1);
        world
    }

    #[test]
    fn reading_waits_for_arrival_turns_pages_on_time_and_keeps_saved_progress() {
        let mut world = World::new(5, 12.0);
        world.begin(Activity::Read);
        world.step(0.1);
        assert!(world.reading.as_ref().unwrap().next_at.is_none());
        world.ghost.position = world.ghost.plan.target;
        world.ghost.velocity = Vec3::ZERO;
        world.step(0.1);
        let reading = world.reading.as_ref().unwrap();
        assert!((2..=4).contains(&reading.remaining));
        let index = reading.index;
        let next_at = reading.next_at.unwrap();
        assert!((15.0..=25.0).contains(&(next_at - world.time)));
        assert!(world.held_book().is_some());
        world.step((next_at - world.time - 0.2) as f32);
        assert_eq!(world.life.total.pages, 0);
        world.step(0.3);
        let owned = world
            .life
            .books
            .owned
            .iter()
            .find(|owned| owned.index == index)
            .unwrap();
        assert_eq!(owned.page, 1);
        assert_eq!(
            world.ghost.thought.as_ref().unwrap().text,
            format!("\"{}\"", LIBRARY[index].pages[0].0)
        );
        let saved = serde_json::to_string(&world.life).unwrap();
        let restored: Life = serde_json::from_str(&saved).unwrap();
        assert_eq!(restored.books, world.life.books);
        let sitting = world.reading.as_ref().unwrap().remaining + 1;
        world.step(110.0);
        assert!(world.ghost.plan.done);
        assert_eq!(world.life.total.pages, sitting);
        assert!(world.held_book().is_none());
    }

    #[test]
    fn finishing_counts_once_per_reading_and_marks_the_first_book_in_words() {
        let mut world = settled_reader();
        let index = world.reading.as_ref().unwrap().index;
        world
            .life
            .books
            .owned
            .iter_mut()
            .find(|owned| owned.index == index)
            .unwrap()
            .page = LIBRARY[index].pages.len() - 1;
        world.step(26.0);
        assert_eq!(world.life.total.books_finished, 1);
        assert_eq!(world.life.today.pages, 1);
        assert!(
            world
                .life
                .today
                .words()
                .to_lowercase()
                .contains("read one page")
        );
        assert!(
            world
                .life
                .today
                .words()
                .to_lowercase()
                .contains("finished one picture book")
        );
        assert!(
            world
                .life
                .milestones
                .iter()
                .any(|entry| entry.text == "I finished my first picture book.")
        );
        assert!(world.events.iter().any(|event| matches!(&event.kind, EventKind::FinishedBook(title) if title == LIBRARY[index].title.0)));
        world.step(30.0);
        assert_eq!(world.life.total.books_finished, 1);
        world.life.books.owned.retain(|owned| owned.index == index);
        world.begin(Activity::Read);
        assert_eq!(world.life.books.owned[0].page, 0);
        for _ in 0..LIBRARY[index].pages.len() {
            world.read_page(index, false);
        }
        assert_eq!(world.life.books.owned[0].finished, 2);
        // A reread counts toward its favorite, but it is no new finish.
        assert_eq!(world.life.total.books_finished, 1);
        assert_eq!(
            world
                .life
                .milestones
                .iter()
                .filter(|entry| entry.text == "I finished my first picture book.")
                .count(),
            1
        );
    }

    #[test]
    fn interruptions_put_the_book_away_immediately() {
        for interruption in 0..5 {
            let mut world = settled_reader();
            assert!(world.held_book().is_some());
            match interruption {
                0 => world.begin(Activity::Dance),
                1 => world.poke(),
                2 => {
                    world.hear("no");
                }
                3 => {
                    world.hear("hello");
                }
                _ => {
                    world.hour = 23.0;
                    world.tuck_in();
                }
            }
            assert!(world.reading.is_none());
            assert!(world.held_book().is_none());
            assert_eq!(world.life.total.pages, 0);
        }
    }

    #[test]
    fn night_reading_visits_the_lamp_before_opening_the_book() {
        let mut world = World::new(4, 21.0);
        world.lamp.on = false;
        world.begin(Activity::Read);
        assert!(world.reading.as_ref().unwrap().lighting);
        world.step(12.0);
        assert!(world.lamp.on);
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::SwitchedLamp { on: true })
        );
        assert!(!world.reading.as_ref().unwrap().lighting);
        world.step(30.0);
        assert!(world.life.total.pages > 0);
    }

    #[test]
    fn bedtime_reading_finishes_one_page_then_sleeps() {
        let mut world = World::new(4, 23.0);
        world.begin(Activity::Read);
        world.step(65.0);
        assert_eq!(world.life.total.pages, 1);
        assert_eq!(world.ghost.plan.activity, Activity::Rest);
        assert!(world.ghost.plan.overnight);
        assert_eq!(world.ghost.asleep, 1.0);
        assert!(world.held_book().is_none());
    }

    #[test]
    fn book_questions_and_aloud_requests_work_in_both_languages_without_teaching() {
        for (text, question) in [
            ("what are you reading?", Question::Reading),
            ("무슨 책 읽어?", Question::Reading),
            ("what's your favorite book?", Question::FavoriteBook),
            ("제일 좋아하는 책은?", Question::FavoriteBook),
            ("read to me", Question::ReadAloud),
            ("read me a book", Question::ReadAloud),
            ("책 읽어줘", Question::ReadAloud),
        ] {
            let mut world = settled_reader();
            let heard = world.hear(text);
            assert!(
                matches!(heard.understanding, Understanding::Asked { question: asked, .. } if asked == question),
                "{text}"
            );
            assert!(heard.learned.is_empty());
            assert!(world.life.vocabulary.known().is_empty());
            if question == Question::ReadAloud {
                assert!(world.telling.as_ref().unwrap().book.is_some());
            }
        }
    }

    #[test]
    fn telling_each_book_page_aloud_advances_only_the_spoken_pages() {
        for (request, korean) in [("read to me", false), ("책 읽어줘", true)] {
            let mut world = settled_reader();
            let index = world.reading.as_ref().unwrap().index;
            world.read_page(index, false);
            world.hear(request);
            let telling = world.telling.as_ref().unwrap();
            assert_eq!(telling.book, Some(index));
            assert_eq!(telling.lines.len(), LIBRARY[index].pages.len() - 1);
            assert_eq!(world.ghost.plan.activity, Activity::Visit);
            world.step(0.1);
            assert_eq!(
                world.ghost.speech.as_ref().unwrap().text,
                if korean {
                    LIBRARY[index].pages[1].1
                } else {
                    LIBRARY[index].pages[1].0
                }
            );
            assert_eq!(world.life.total.pages, 2);
            assert_eq!(
                world
                    .life
                    .books
                    .owned
                    .iter()
                    .find(|owned| owned.index == index)
                    .unwrap()
                    .page,
                2
            );
            world.poke();
            assert!(world.telling.is_none());
            assert!(world.held_book().is_none());
            world.step(10.0);
            assert_eq!(world.life.total.pages, 2);
            assert_eq!(world.life.total.books_finished, 0);
            world.hear(request);
            world.step(35.0);
            assert!(world.telling.is_none());
            assert_eq!(world.life.total.pages as usize, LIBRARY[index].pages.len());
            assert_eq!(world.life.total.books_finished, 1);
            assert_eq!(world.life.total.stories, 1);
        }
    }

    #[test]
    fn reading_aloud_lights_a_dark_room_before_speaking() {
        let mut world = World::new(7, 21.0);
        world.lamp.on = false;
        world.hear("read to me");
        assert!(world.telling.as_ref().unwrap().lighting);
        assert!(world.held_book().is_none());
        world.step(0.1);
        assert_eq!(world.life.total.pages, 0);
        assert!(world.ghost.speech.is_none());
        world.step(14.0);
        assert!(world.lamp.on);
        assert!(world.life.total.pages > 0);
        assert!(world.held_book().is_some());
    }

    #[test]
    fn quiet_reading_continues_when_the_viewer_leaves() {
        let mut world = settled_reader();
        world.notice_viewer();
        world.step(0.1);
        world.look_away(true);
        world.step(26.0);
        assert!(!world.viewer_present());
        assert!(world.life.total.pages > 0);
    }

    #[test]
    fn a_saved_birthday_delivery_does_not_arrive_again_on_reload() {
        let mut world = World::new(19, 12.0);
        world.date = jiff::civil::Date::constant(2027, 3, 1);
        world.birthday = Some(jiff::civil::Date::constant(2026, 3, 1));
        world.life.books.deliveries = 9;
        world.step(0.0);
        let saved = serde_json::to_string(&world.saved_life()).unwrap();
        let mut restored = World::new(19, 12.0);
        restored.date = world.date;
        restored.birthday = world.birthday;
        restored.life = serde_json::from_str::<Life>(&saved).unwrap().sanitized();
        restored.step(0.0);
        assert_eq!(restored.life.books, world.life.books);
        assert_eq!(restored.life.total.books_received, 1);
    }

    #[test]
    fn the_books_are_visible_obstacles_and_quiet_focus_options() {
        let mut world = World::new(1, 12.0);
        assert!(
            world
                .obstacles()
                .iter()
                .any(|(position, _)| *position == POSITION)
        );
        world.attend("books", 10.0);
        world.step(1.0);
        assert!(
            world
                .sight()
                .in_view
                .iter()
                .any(|seen| seen.thing == Thing::Books)
        );
        world.start_focus(25);
        assert!(world.options().contains(&Activity::Read));
        assert!(!super::super::focus::seeks_attention(Activity::Read));
    }
}
