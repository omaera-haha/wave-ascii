use glam::Vec2;

use super::Face;
use super::expression::Expression;
use crate::sdf::smoothstep;
use crate::world::{EventKind, Mood, Outcome, Quirk};

/// Seconds a reaction takes to take over the face and to give it back.
const FADE: f32 = 0.15;

const ASLEEP: Face = Face {
    shut: [1.0; 2],
    gaze: Vec2::new(0.0, -0.5),
    ..Face::REST
};

/// Eyes closed in happy arcs, tipped up into the stroke.
const BLISS: Face = Face {
    smile: 1.0,
    gaze: Vec2::new(0.0, 0.3),
    ..Face::REST
};

/// What the face answers to at one moment.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cues {
    pub mood: Mood,
    /// From awake at 0 to asleep at 1.
    pub asleep: f32,
    /// From untouched at 0 to being stroked steadily at 1.
    pub petting: f32,
    /// Where to look, from -1 to 1 across and up on screen, and how firmly,
    /// from 0 (its own glances) to 1 (eyes held there).
    pub gaze: Option<(Vec2, f32)>,
    /// Recent events to react to: when each happened and the expression it
    /// answers with.
    pub reactions: Vec<(f64, Expression)>,
    pub focusing: bool,
}

/// The face at `time`. Each mood holds a base face and makes its own kind of
/// expressions at irregular moments; the gaze, sleep, and blinks are laid
/// over them, and reactions to events take over the face last.
pub fn face(time: f64, cues: &Cues) -> Face {
    let quiet = cues.focusing && cues.mood == Mood::Content;
    let repertoire = if quiet {
        &QUIET
    } else {
        Repertoire::of(cues.mood)
    };
    let mut face = repertoire.base.layer(repertoire.at(time));

    if let Some((gaze, firmness)) = cues.gaze {
        face.gaze = face.gaze * (1.0 - firmness) + gaze.clamp_length_max(1.0) * firmness;
    }
    face = face.lerp(ASLEEP, cues.asleep);
    // Being stroked melts an awake face into bliss.
    face = face.lerp(
        BLISS,
        smoothstep(0.1, 0.5, cues.petting) * (1.0 - cues.asleep),
    );
    // A blink doesn't suit eyes that have become other shapes.
    let blink = BLINKS.at(if quiet { time / 2.0 } else { time }).shut[0]
        * (1.0 - cues.asleep)
        * (1.0 - face.prompt)
        * (1.0 - face.smile);
    face.shut = face.shut.map(|shut| shut.max(blink));

    // A sleeper sleeps through what happens; reactions grow in as it wakes.
    for (start, expression) in &cues.reactions {
        let elapsed = (time - start) as f32;
        let duration = expression.duration();
        if (0.0..duration).contains(&elapsed) {
            let fade =
                smoothstep(0.0, FADE, elapsed) * smoothstep(duration, duration - FADE, elapsed);
            face = face.lerp(expression.at(elapsed), fade * (1.0 - cues.asleep));
        }
    }
    face
}

/// The expression the face answers an event with, if any.
pub fn reaction(event: &EventKind) -> Option<Expression> {
    Some(match event {
        EventKind::MetReflection => Expression::Surprise,
        EventKind::RecognizedSelf => Expression::Sparkle,
        EventKind::Heard(text) if crate::world::quirks::yawn_message(text) => return None,
        EventKind::Quirk(Quirk::Hiccups) => Expression::Hiccup,
        EventKind::Quirk(Quirk::Sneeze) => Expression::Sneeze,
        EventKind::Quirk(Quirk::Yawn) => Expression::Yawn,
        EventKind::Quirk(Quirk::Shiver) => Expression::Worry,
        EventKind::Quirk(Quirk::Sigh) => Expression::Sigh,
        EventKind::ReadViewerLetter => Expression::Smile,
        EventKind::GotLetter => glance(-0.2, -0.1),
        EventKind::ReadLetter => glance(0.25, -0.2),
        EventKind::MadeWish(_) => Expression::Smile,
        EventKind::WishGranted(_) | EventKind::PromiseKept(_) => Expression::Sparkle,
        EventKind::WishFaded(_) => glance(0.0, -0.3),
        EventKind::SawPasserby(kind) if kind.delighted() => Expression::Sparkle,
        EventKind::SawPasserby(_) => Expression::LookAround,
        EventKind::FocusEnded {
            completed: true, ..
        } => Expression::Sparkle,
        EventKind::FocusEnded {
            completed: false, ..
        } => glance(0.0, -0.35),
        EventKind::FriendArrived(_) => Expression::Sparkle,
        EventKind::FriendLeft(_) => Expression::Worry,
        EventKind::Tagged { .. } => Expression::Surprise,
        EventKind::PokedFriend | EventKind::PlayedWithFriend(_) => Expression::Smile,
        EventKind::Poked { count: 1 } => Expression::Surprise,
        EventKind::Poked { count: 2 } => Expression::Smile,
        EventKind::Poked { .. } => Expression::Glare,
        EventKind::AteMote
        | EventKind::Bloomed
        | EventKind::Sprouted(_)
        | EventKind::Flowered(_)
        | EventKind::DroppedMote
        | EventKind::Fetched
        | EventKind::Kept(_)
        | EventKind::Pounced
        | EventKind::Built { .. }
        | EventKind::Found
        | EventKind::ExpectedArrival
        | EventKind::MoonWish
        | EventKind::FoundConstellation(_)
        | EventKind::Celebrated(_)
        | EventKind::Milestone(_) => Expression::Sparkle,
        EventKind::Heard(_) | EventKind::GameOver(Outcome::GhostWon { .. }) => Expression::Sparkle,
        EventKind::WordGame { solved: true, .. } => Expression::Sparkle,
        EventKind::WordGame { solved: false, .. } => Expression::Skeptical,
        EventKind::GameOver(Outcome::ViewerWon { .. }) => Expression::Skeptical,
        EventKind::GameOver(Outcome::Draw) => Expression::LookAround,
        EventKind::PutOn(_) | EventKind::Remembered(_) => Expression::Smile,
        EventKind::NudgedBall
        | EventKind::Tended
        | EventKind::Planted(_)
        | EventKind::PickedUpBall
        | EventKind::Danced
        | EventKind::Doodled(_)
        | EventKind::Wrote { .. } => Expression::Smile,
        EventKind::Composed(_) | EventKind::PlayedMusic { .. } | EventKind::LearnedTune(_) => {
            Expression::Smile
        }
        EventKind::WroteBook(_) | EventKind::SangMySong(_) => Expression::Sparkle,
        EventKind::ViewerFelt(feeling) if feeling.negative() => Expression::Worry,
        EventKind::ViewerFelt(_) | EventKind::FeltBetter => Expression::Sparkle,
        EventKind::Comforted(_) => Expression::Smile,
        EventKind::OpenedWindow | EventKind::ClosedWindow => Expression::Smile,
        EventKind::Puddle => Expression::Worry,
        EventKind::BirdSang
        | EventKind::ToldStory
        | EventKind::FinishedBook(_)
        | EventKind::NewBook(_) => Expression::Smile,
        EventKind::BirdFlewOff { startled: true } => glance(0.0, -0.6),
        EventKind::WokeUp
        | EventKind::KickedBall
        | EventKind::Jumped
        | EventKind::Thunder
        | EventKind::Jostled { rattled: false } => Expression::Surprise,
        EventKind::Toppled { by_viewer: false } => Expression::Smile,
        EventKind::Toppled { by_viewer: true } => Expression::Surprise,
        EventKind::Morning => Expression::Stretch,
        EventKind::ViewerArrived => Expression::Wink,
        EventKind::Praised | EventKind::Petted => Expression::Smile,
        EventKind::Scolded | EventKind::Drooped => Expression::Worry,
        EventKind::MissedVisit => Expression::Worry,
        EventKind::SawDrawing { name: None } => Expression::Skeptical,
        EventKind::SawDrawing { name: Some(name) } if matches!(name.as_str(), "star" | "sun") => {
            Expression::Sparkle
        }
        EventKind::SawDrawing { .. } | EventKind::Copied(_) => Expression::Smile,
        EventKind::LearnedAboutViewer(_) => Expression::Smile,
        EventKind::LearnedPicture(_) => Expression::Sparkle,
        EventKind::Puzzled(_) => Expression::Skeptical,
        EventKind::LearnedWord { .. }
        | EventKind::LearnedName(_)
        | EventKind::Named(_)
        | EventKind::ShootingStar
        | EventKind::ForecastCameTrue { .. } => Expression::Sparkle,
        EventKind::Noticed(thing)
            if thing.starts_with("the rainbow")
                || thing == "the falling snow"
                || thing == "a bird at the window" =>
        {
            Expression::Sparkle
        }
        EventKind::Noticed(thing) if thing.starts_with("a gift on the sill") => {
            Expression::Surprise
        }
        EventKind::WishedLuck(_) | EventKind::PlanWentWell(_) => Expression::Sparkle,
        EventKind::PlanWentBadly(_) => Expression::Worry,
        EventKind::SpottedTreasure => Expression::Surprise,
        EventKind::FoundTreasure { .. } => Expression::Sparkle,
        EventKind::AskedTreasureHint => Expression::Prompt,
        EventKind::SuggestedBreak { long: false } => Expression::Stretch,
        EventKind::SuggestedBreak { long: true }
        | EventKind::TookBreak
        | EventKind::SaidGoodnightTogether => Expression::Smile,
        EventKind::NoticedLateNight => Expression::Doze,
        EventKind::FoldedCrane { .. } => Expression::Smile,
        EventKind::BlewBubbles { .. } => Expression::Smile,
        EventKind::ShowedThing(_) => Expression::Sparkle,
        _ => return None,
    })
}

/// What the face does in one mood: a base it holds, and expressions it makes
/// now and then. In each slot of time it may make one expression, picked at
/// random by weight and started at a random moment.
struct Repertoire {
    base: Face,
    expressions: &'static [(Expression, f32)],
    /// Seconds per slot, at least as long as the longest expression.
    slot: f32,
    /// The chance that a slot passes without an expression.
    rest: f32,
    /// Keeps each repertoire's picks apart from the others'.
    seed: u32,
}

const fn glance(x: f32, y: f32) -> Expression {
    Expression::Glance {
        toward: Vec2::new(x, y),
    }
}

const BLINKS: Repertoire = Repertoire {
    base: Face::REST,
    expressions: &[(Expression::Blink, 4.0), (Expression::DoubleBlink, 1.0)],
    slot: 3.2,
    rest: 0.15,
    seed: 1,
};

const QUIET: Repertoire = Repertoire {
    base: Face {
        lid: [0.08; 2],
        ..Face::REST
    },
    expressions: &[(Expression::Smile, 1.0)],
    slot: 16.0,
    rest: 0.8,
    seed: 10,
};

const CONTENT: Repertoire = Repertoire {
    base: Face::REST,
    expressions: &[
        (glance(-0.9, 0.15), 2.0),
        (glance(0.85, 0.4), 2.0),
        (glance(0.35, -0.75), 1.0),
        (Expression::LookAround, 2.0),
        (Expression::Smile, 1.0),
        (Expression::Wink, 0.3),
    ],
    slot: 6.0,
    rest: 0.35,
    seed: 2,
};

const CHEERFUL: Repertoire = Repertoire {
    base: Face::REST,
    expressions: &[
        (Expression::Smile, 3.0),
        (Expression::Sparkle, 1.5),
        (Expression::Wink, 1.0),
        (glance(0.6, 0.5), 1.0),
        (Expression::LookAround, 1.0),
    ],
    slot: 5.0,
    rest: 0.3,
    seed: 3,
};

const EXCITED: Repertoire = Repertoire {
    base: Face {
        widen: 0.25,
        ..Face::REST
    },
    expressions: &[
        (Expression::Sparkle, 3.0),
        (Expression::Smile, 2.0),
        (Expression::Surprise, 1.0),
    ],
    slot: 3.5,
    rest: 0.2,
    seed: 4,
};

const CURIOUS: Repertoire = Repertoire {
    base: Face {
        widen: 0.1,
        ..Face::REST
    },
    expressions: &[
        (glance(-0.9, 0.3), 2.0),
        (glance(0.9, -0.2), 2.0),
        (Expression::LookAround, 2.0),
        (Expression::Skeptical, 1.0),
        (Expression::Sparkle, 0.5),
    ],
    slot: 5.0,
    rest: 0.3,
    seed: 5,
};

const SLEEPY: Repertoire = Repertoire {
    base: Face {
        lid: [0.35; 2],
        gaze: Vec2::new(0.0, -0.3),
        ..Face::REST
    },
    expressions: &[(Expression::Doze, 2.0), (glance(0.4, -0.6), 1.0)],
    slot: 5.0,
    rest: 0.4,
    seed: 6,
};

const LONELY: Repertoire = Repertoire {
    base: Face {
        slant: -0.35,
        ..Face::REST
    },
    expressions: &[
        (Expression::Worry, 2.0),
        (Expression::LookAround, 1.0),
        (glance(0.0, -0.7), 1.0),
    ],
    slot: 5.0,
    rest: 0.4,
    seed: 7,
};

const BORED: Repertoire = Repertoire {
    base: Face {
        lid: [0.2; 2],
        ..Face::REST
    },
    expressions: &[
        // Waiting at a prompt, as a terminal does.
        (Expression::Prompt, 2.0),
        (Expression::Suspicious, 1.0),
        (Expression::Skeptical, 1.0),
        (Expression::Doze, 0.5),
    ],
    slot: 6.0,
    rest: 0.4,
    seed: 8,
};

const GRUMPY: Repertoire = Repertoire {
    base: Face {
        slant: 0.5,
        lid: [0.25; 2],
        ..Face::REST
    },
    expressions: &[
        (Expression::Glare, 2.0),
        (Expression::Suspicious, 1.0),
        (Expression::Skeptical, 1.0),
    ],
    slot: 4.0,
    rest: 0.3,
    seed: 9,
};

impl Repertoire {
    fn of(mood: Mood) -> &'static Self {
        match mood {
            Mood::Content => &CONTENT,
            Mood::Cheerful => &CHEERFUL,
            Mood::Excited => &EXCITED,
            Mood::Curious => &CURIOUS,
            Mood::Sleepy => &SLEEPY,
            Mood::Lonely => &LONELY,
            Mood::Bored => &BORED,
            Mood::Grumpy => &GRUMPY,
        }
    }

    /// The expression playing at `time` in the slot that `time` falls in,
    /// or the rest face.
    fn at(&self, time: f64) -> Face {
        let slot = (time / f64::from(self.slot)).floor();
        let roll = |salt| random(slot, self.seed, salt);
        if roll(0) < self.rest {
            return Face::REST;
        }
        let total: f32 = self.expressions.iter().map(|(_, weight)| weight).sum();
        let mut pick = roll(1) * total;
        let Some((expression, _)) = self
            .expressions
            .iter()
            .find(|(_, weight)| {
                pick -= weight;
                pick < 0.0
            })
            .or(self.expressions.last())
        else {
            return Face::REST;
        };
        let start = slot * f64::from(self.slot)
            + f64::from(roll(2) * (self.slot - expression.duration()).max(0.0));
        expression.at((time - start) as f32)
    }
}

/// A number from 0 to 1 that looks random but is always the same for the
/// same `slot`, `seed`, and `salt`.
pub(super) fn random(slot: f64, seed: u32, salt: u32) -> f32 {
    let mut bits = (slot as i64 as u32).wrapping_mul(0x9E37_79B9)
        ^ seed.wrapping_mul(0x85EB_CA6B)
        ^ salt.wrapping_mul(0xC2B2_AE35);
    bits ^= bits >> 16;
    bits = bits.wrapping_mul(0x7FEB_352D);
    bits ^= bits >> 15;
    bits = bits.wrapping_mul(0x846C_A68B);
    bits ^= bits >> 16;
    (bits >> 8) as f32 / (1 << 24) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cues(mood: Mood) -> Cues {
        Cues {
            mood,
            ..Cues::default()
        }
    }

    #[test]
    fn every_slot_fits_the_longest_expression_it_can_play() {
        for mood in Mood::ALL {
            let repertoire = Repertoire::of(mood);
            for (expression, weight) in repertoire.expressions {
                assert!(
                    expression.duration() <= repertoire.slot,
                    "{mood:?} {expression:?}"
                );
                assert!(*weight > 0.0);
            }
        }
    }

    #[test]
    fn random_numbers_are_between_zero_and_one_and_vary() {
        let numbers: Vec<f32> = (-50..50).map(|slot| random(slot as f64, 1, 0)).collect();
        assert!(numbers.iter().all(|number| (0.0..1.0).contains(number)));
        let mean = numbers.iter().sum::<f32>() / numbers.len() as f32;
        assert!((0.35..0.65).contains(&mean), "{mean}");
        assert_ne!(random(3.0, 1, 0), random(3.0, 2, 0));
    }

    #[test]
    fn a_bored_ghost_sometimes_waits_at_a_prompt_and_a_cheerful_one_smiles() {
        let over_time = |mood| (0..2400).map(move |step| face(f64::from(step) * 0.5, &cues(mood)));
        assert!(over_time(Mood::Bored).any(|face| face.prompt > 0.9));
        assert!(over_time(Mood::Cheerful).any(|face| face.smile > 0.9));
        assert!(over_time(Mood::Excited).any(|face| face.sparkle > 0.5));
        assert!(over_time(Mood::Grumpy).all(|face| face.slant > 0.4));
    }

    #[test]
    fn a_sleeping_ghost_keeps_its_eyes_shut() {
        let asleep = Cues {
            asleep: 1.0,
            ..cues(Mood::Sleepy)
        };
        for step in 0..200 {
            assert_eq!(face(f64::from(step) * 0.3, &asleep).shut, [1.0; 2]);
        }
    }

    #[test]
    fn reactions_to_what_happens_leave_a_sleeping_face_asleep() {
        for expression in [Expression::Worry, Expression::Sparkle, Expression::Surprise] {
            let cues = Cues {
                asleep: 1.0,
                reactions: vec![(0.0, expression)],
                ..Cues::default()
            };
            for step in 0..10 {
                assert_eq!(face(f64::from(step) * 0.2, &cues).shut, [1.0; 2]);
            }
        }
    }

    #[test]
    fn a_firm_gaze_holds_the_eyes_on_it() {
        let looking = Cues {
            gaze: Some((Vec2::new(2.0, 0.0), 1.0)),
            ..cues(Mood::Content)
        };
        for step in 0..200 {
            assert_eq!(face(f64::from(step) * 0.3, &looking).gaze, Vec2::X);
        }
    }

    #[test]
    fn a_reaction_takes_over_the_face_then_gives_it_back() {
        let reacting = Cues {
            reactions: vec![(10.0, Expression::Surprise)],
            ..cues(Mood::Content)
        };
        let calm = cues(Mood::Content);
        assert!(face(10.4, &reacting).widen > 0.9);
        assert_eq!(face(9.9, &reacting), face(9.9, &calm));
        assert_eq!(face(12.0, &reacting), face(12.0, &calm));
    }

    #[test]
    fn pokes_escalate_from_surprise_to_a_glare() {
        assert_eq!(
            reaction(&EventKind::Poked { count: 1 }),
            Some(Expression::Surprise)
        );
        assert_eq!(
            reaction(&EventKind::Poked { count: 5 }),
            Some(Expression::Glare)
        );
        assert_eq!(reaction(&EventKind::FellAsleep), None);
    }

    #[test]
    fn hiding_and_drawing_events_have_their_own_reactions() {
        assert_eq!(reaction(&EventKind::Found), Some(Expression::Sparkle));
        assert_eq!(reaction(&EventKind::Jumped), Some(Expression::Surprise));
        assert_eq!(
            reaction(&EventKind::Doodled("heart")),
            Some(Expression::Smile)
        );
    }
}
