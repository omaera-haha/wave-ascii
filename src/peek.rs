//! A one-line glance at the saved ghost while the app is closed, for a shell
//! prompt or a terminal multiplexer's status bar. It only reads the save.

use jiff::civil::DateTime;

use crate::store::Save;

/// Two days away and the ghost has started to miss the viewer.
const MISSES_AFTER: i64 = 2 * 24 * 3600;

/// What the ghost is probably up to at local time `now`, `away` seconds
/// after the app last closed, in the language it last heard.
pub fn glance(save: &Save, now: DateTime, away: i64) -> String {
    let name: String = save
        .name
        .trim()
        .chars()
        .filter(|character| !character.is_control())
        .take(24)
        .collect();
    let name = if name.is_empty() {
        "Boo".to_owned()
    } else {
        name
    };
    let life = &save.life;
    let korean = life.korean > 0.5;
    let hour = f32::from(now.hour()) + f32::from(now.minute()) / 60.0;
    let birthday = jiff::Timestamp::from_second(save.born)
        .ok()
        .map(|born| born.to_zoned(jiff::tz::TimeZone::system()).date())
        .is_some_and(|born| {
            born.year() < now.year()
                && born.month() == now.month()
                && (born.day() == now.day()
                    || (born.month() == 2
                        && born.day() == 29
                        && now.day() == 28
                        && now.date().last_of_month().day() == 28))
        });
    let (mark, english, korean_words) = if !(6.5..22.0).contains(&hour) {
        ("☾", "is asleep", "쿨쿨 자는 중")
    } else if life
        .cold
        .as_ref()
        .is_some_and(|cold| cold.active_after(now, away))
    {
        ("~", "has a little cold", "감기 기운이 있어")
    } else if birthday {
        ("✦", "is having a birthday", "오늘 생일이야")
    } else if away >= MISSES_AFTER {
        ("♡", "misses you", "네가 보고 싶대")
    } else if life.expectation(hour) >= 0.5 {
        (
            "♥",
            "is waiting for you at the glass",
            "유리 앞에서 기다리는 중",
        )
    } else if hour < 10.0 {
        (
            "☀",
            "is watching the morning from the window",
            "창가에서 아침 구경 중",
        )
    } else if hour < 17.0 {
        ("~", "is floating about", "둥둥 떠다니는 중")
    } else {
        ("☆", "is reading by the lamp", "램프 옆에서 책 읽는 중")
    };
    let mut line = if korean {
        format!("{mark} {name}, {korean_words}")
    } else {
        format!("{mark} {name} {english}")
    };
    let unread = life.letters.iter().filter(|letter| !letter.read).count();
    if unread > 0 {
        line.push_str(" · ✉");
    }
    line
}

/// The glance for a missing save: nobody lives here yet.
pub fn nobody() -> &'static str {
    "No ghost lives here yet. Run wave-ascii to meet one."
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::World;

    fn save(world: &World) -> Save {
        Save {
            version: Save::VERSION,
            name: "Pip".into(),
            born: 1_780_000_000,
            last_seen: 1_780_000_000,
            needs: world.ghost.needs,
            plant: world.plant,
            lamp_on: false,
            goal: None,
            memories: Vec::new(),
            conversation: Vec::new(),
            life: world.saved_life(),
            seedlings: Vec::new(),
            room: None,
        }
    }

    fn at(hour: i8) -> DateTime {
        jiff::civil::date(2026, 10, 21).at(hour, 0, 0, 0)
    }

    #[test]
    fn the_glance_follows_the_hour_absence_and_language() {
        let world = World::new(1, 12.0);
        let mut save = save(&world);
        assert_eq!(glance(&save, at(23), 60), "☾ Pip is asleep");
        assert_eq!(glance(&save, at(3), 60), "☾ Pip is asleep");
        assert_eq!(glance(&save, at(13), 60), "~ Pip is floating about");
        assert_eq!(glance(&save, at(13), MISSES_AFTER), "♡ Pip misses you");
        assert_eq!(glance(&save, at(23), MISSES_AFTER), "☾ Pip is asleep");
        save.life.korean = 1.0;
        assert_eq!(glance(&save, at(8), 60), "☀ Pip, 창가에서 아침 구경 중");
    }

    #[test]
    fn a_waiting_letter_and_an_expected_hour_show() {
        let mut world = World::new(1, 12.0);
        world.date = jiff::civil::date(2026, 10, 18);
        for day in 18..21 {
            world.date = jiff::civil::date(2026, 10, day);
            world.life.begin_day(&world.date.to_string());
            world.life.arrive(19.0);
        }
        let mut save = save(&world);
        assert_eq!(
            glance(&save, at(19), 60),
            "♥ Pip is waiting for you at the glass"
        );
        save.life.letters.push(crate::world::mail::Letter {
            day: "2026-10-20".to_owned(),
            lines: vec!["Hello.".to_owned()],
            korean: false,
            read: false,
            claude: false,
            reply_to: None,
        });
        assert_eq!(
            glance(&save, at(19), 60),
            "♥ Pip is waiting for you at the glass · ✉"
        );
    }

    #[test]
    fn names_stay_on_one_line() {
        let world = World::new(1, 12.0);
        let mut save = save(&world);
        save.name = "Pi\np\u{1b}[31m".to_owned();
        assert!(!glance(&save, at(13), 60).contains(['\n', '\u{1b}']));
        save.name = "   ".to_owned();
        assert!(glance(&save, at(13), 60).contains("Boo"));
    }
    #[test]
    fn the_saved_cold_shows_below_bedtime_and_only_until_it_has_run_its_course() {
        let mut save = save(&World::new(1, 12.0));
        save.life.cold = Some(crate::world::cold::Cold {
            day: "2026-10-21".to_owned(),
            hour: 9.0,
            severity: 0.5,
            care: Vec::new(),
            remaining: 20.0 * 3600.0,
        });
        assert_eq!(glance(&save, at(13), 60), "~ Pip has a little cold");
        assert_eq!(glance(&save, at(23), 60), "☾ Pip is asleep");
        assert_eq!(glance(&save, at(13), 80_000), "~ Pip is floating about");
        save.life.cold.as_mut().unwrap().hour = 13.0 + 20.0 / 3600.0;
        assert_eq!(
            glance(&save, at(13).with().second(30).build().unwrap(), 0),
            "~ Pip has a little cold"
        );
        save.life.cold.as_mut().unwrap().hour = 9.0;
        save.life.korean = 1.0;
        assert_eq!(glance(&save, at(13), 60), "~ Pip, 감기 기운이 있어");
        save.life.cold.as_mut().unwrap().hour = 25.0;
        assert!(!glance(&save, at(13), 60).contains("감기"));
    }
}
