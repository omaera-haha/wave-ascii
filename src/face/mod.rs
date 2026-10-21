use glam::Vec2;

use crate::sdf::{chevron, mix, rounded_box, star};

pub mod expression;
pub mod mood;

/// What the face shows at one moment: the controls that expressions move.
/// At rest every control is zero except `cursor`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Face {
    /// Where the eyes look, from -1 to 1 across and up.
    pub gaze: Vec2,
    /// How shut each eye is, left then right, from open at 0 to a line at 1.
    pub shut: [f32; 2],
    /// How far each upper lid has come down over its eye, left then right,
    /// from 0 to covering it at 1.
    pub lid: [f32; 2],
    /// How the lids slant, from -1 to 1: positive lowers their inner ends
    /// for a glare and negative lowers their outer ends for worry.
    pub slant: f32,
    /// How wide the eyes open, from narrow at -1 to wide at 1.
    pub widen: f32,
    /// From eyes at 0 to happy `^` arcs at 1.
    pub smile: f32,
    /// The size of the twinkles of light in the eyes, from none at 0 to 1.
    pub sparkle: f32,
    /// From eyes at 0 to a prompt at 1.
    pub prompt: f32,
    /// How visible the prompt's cursor is, from 0 to 1.
    pub cursor: f32,
}

impl Face {
    pub const REST: Self = Self {
        gaze: Vec2::ZERO,
        shut: [0.0; 2],
        lid: [0.0; 2],
        slant: 0.0,
        widen: 0.0,
        smile: 0.0,
        sparkle: 0.0,
        prompt: 0.0,
        cursor: 1.0,
    };

    /// This face with `other`'s departures from rest added on: shut eyes,
    /// lids, smiles, and the like as far as either face has them, and gaze,
    /// slant, and widening summed.
    pub fn layer(self, other: Self) -> Self {
        let pair =
            |first: [f32; 2], second: [f32; 2]| [first[0].max(second[0]), first[1].max(second[1])];
        Self {
            gaze: (self.gaze + other.gaze).clamp(Vec2::NEG_ONE, Vec2::ONE),
            shut: pair(self.shut, other.shut),
            lid: pair(self.lid, other.lid),
            slant: (self.slant + other.slant).clamp(-1.0, 1.0),
            widen: (self.widen + other.widen).clamp(-1.0, 1.0),
            smile: self.smile.max(other.smile),
            sparkle: self.sparkle.max(other.sparkle),
            prompt: self.prompt.max(other.prompt),
            cursor: self.cursor.min(other.cursor),
        }
    }

    /// The face `amount` of the way from this one to `other`.
    pub fn lerp(self, other: Self, amount: f32) -> Self {
        let pair = |from: [f32; 2], to: [f32; 2]| {
            [mix(from[0], to[0], amount), mix(from[1], to[1], amount)]
        };
        Self {
            gaze: self.gaze * (1.0 - amount) + other.gaze * amount,
            shut: pair(self.shut, other.shut),
            lid: pair(self.lid, other.lid),
            slant: mix(self.slant, other.slant, amount),
            widen: mix(self.widen, other.widen, amount),
            smile: mix(self.smile, other.smile, amount),
            sparkle: mix(self.sparkle, other.sparkle, amount),
            prompt: mix(self.prompt, other.prompt, amount),
            cursor: mix(self.cursor, other.cursor, amount),
        }
    }

    /// Signed distance to the face, cut out of the head, with `point`
    /// relative to the head's center. Each feature changes shape by moving
    /// its own points and sizes, so every moment of a change is a clean shape.
    pub fn distance(&self, point: Vec2) -> f32 {
        self.eye(point, 0).min(self.eye(point, 1))
    }

    /// Signed distance to the left eye for `index` 0 or the right for 1.
    fn eye(&self, point: Vec2, index: usize) -> f32 {
        let side = if index == 0 { -1.0 } else { 1.0 };
        let shut = self.shut[index];
        // An open eye is a tall pill, which widening stretches; a shut one is
        // a thin line.
        let open =
            Vec2::new(0.055, 0.125) * Vec2::new(1.0 + 0.15 * self.widen, 1.0 + 0.3 * self.widen);
        let size = open.lerp(Vec2::new(0.065, 0.016), shut);
        let radius = mix(open.x, 0.016, shut);
        let center = Vec2::new(0.17 * side, -0.03) + self.gaze * Vec2::new(0.07, 0.05)
            - Vec2::new(0.0, 0.02 * shut);

        // The prompt takes over from the other changes of shape: the left eye
        // bends at its middle into `>`, and the right tips over into an
        // underscore, which then blinks like a cursor.
        let shape = if self.prompt > 0.0 && index == 0 {
            let bend = 0.05 * self.prompt;
            let reach = mix(size.y - radius, 0.08, self.prompt);
            chevron(
                point - center.lerp(Vec2::new(-0.15, -0.04), self.prompt),
                Vec2::new(bend, 0.0),
                [Vec2::new(-bend, reach), Vec2::new(-bend, -reach)],
                mix(radius, 0.03, self.prompt),
            )
        } else if self.prompt > 0.0 {
            let size = size.lerp(Vec2::new(0.075, 0.024 * self.cursor), self.prompt);
            rounded_box(
                point - center.lerp(Vec2::new(0.12, -0.115), self.prompt),
                size,
                mix(radius, 0.01, self.prompt).min(size.min_element()),
            ) + 0.03 * (1.0 - self.cursor)
        } else if self.smile > 0.0 {
            // The pill folds down from its top into `^`.
            let reach = (size.y - radius).max(0.0);
            let foot = Vec2::new(0.07 * self.smile, mix(-reach, -0.025, self.smile));
            chevron(
                point - center,
                Vec2::new(0.0, mix(reach, 0.03, self.smile)),
                [foot * Vec2::new(-1.0, 1.0), foot],
                mix(radius, 0.025, self.smile),
            )
        } else if self.sparkle > 0.0 {
            // A star of light near the top of the open eye and a dot below
            // it are left uncut.
            let point = point - center;
            let star = star(point - Vec2::new(-0.33, 0.4) * size, 0.034 * self.sparkle);
            let dot = (point - Vec2::new(0.35, -0.36) * size).length() - 0.013 * self.sparkle;
            rounded_box(point, size, radius).max(-star.min(dot))
        } else {
            rounded_box(point - center, size, radius)
        };

        // The upper lid is a straight edge coming down over the eye, which
        // can slant toward or away from the middle of the face.
        let angle = 0.5 * self.slant;
        let normal = Vec2::new(-side * angle.sin(), angle.cos());
        let height = mix(open.y + 0.04, -open.y, self.lid[index]);
        shape.max(normal.dot(point - center) - height)
    }
}

/// The value at `time` along `keys` of (time, value), easing in and out
/// between keys and holding before the first and after the last.
pub fn ease_through(keys: &[(f32, f32)], time: f32) -> f32 {
    let next = keys.partition_point(|(key, _)| *key <= time);
    match (next.checked_sub(1).map(|index| keys[index]), keys.get(next)) {
        (Some((start, from)), Some(&(end, to))) => {
            let x = ((time - start) / (end - start)).clamp(0.0, 1.0);
            from + (to - from) * x * x * x * (x * (6.0 * x - 15.0) + 10.0)
        }
        (Some((_, value)), None) | (None, Some(&(_, value))) => value,
        (None, None) => 0.0,
    }
}
