//! Light laid over the scene, as from the dream bubble and soap bubbles,
//! and the glow that shows how the creature the viewer is with feels.

use glam::Vec3;

/// The glow of the creature the viewer is with, which shows how it is: its
/// color follows its mood, and its brightness and beat follow its energy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aura {
    pub color: Vec3,
    /// How much of it remains visible while hiding, from 0 to 1.
    pub visibility: f32,
    /// From dim at 0 to bright at 1.
    pub brightness: f32,
    /// Beats per second of the ring's pulse.
    pub beat: f32,
}

/// A soft mint-to-lavender glow, distinct from the resident's mood palette.
pub fn friend_color(hue: f32) -> Vec3 {
    Vec3::new(0.55, 1.0, 0.78).lerp(Vec3::new(0.88, 0.68, 1.0), hue.clamp(0.0, 1.0))
}

/// Light laid over one point: its color, already weighted by how much of
/// the point it covers, and that coverage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glow {
    pub light: Vec3,
    pub cover: f32,
}

impl Glow {
    pub const NONE: Self = Self {
        light: Vec3::ZERO,
        cover: 0.0,
    };

    /// This glow laid over `behind`.
    pub fn over(self, behind: Vec3) -> Vec3 {
        self.light + behind * (1.0 - self.cover)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_glow_covers_what_is_behind_it_by_its_coverage() {
        let glow = Glow {
            light: Vec3::splat(0.5),
            cover: 0.5,
        };
        assert_eq!(glow.over(Vec3::ONE), Vec3::ONE);
        assert_eq!(Glow::NONE.over(Vec3::ONE), Vec3::ONE);
    }
}
