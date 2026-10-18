use std::env;
use std::fmt::Write;

use glam::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub [u8; 3]);

impl Rgb {
    pub fn from_vec3(color: Vec3) -> Self {
        let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        Self([channel(color.x), channel(color.y), channel(color.z)])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorMode {
    Plain,
    Palette,
    TrueColor,
}

/// A color as the terminal will actually show it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paint {
    TrueColor(Rgb),
    Indexed(u8),
}

#[derive(Clone, Copy)]
pub enum Ground {
    Foreground,
    Background,
}

impl ColorMode {
    /// Picks the richest mode the environment advertises, following the
    /// `NO_COLOR` and `COLORTERM` conventions.
    pub fn detect() -> Self {
        let variable = |name| env::var(name).unwrap_or_default();
        let term = variable("TERM");
        let colorterm = variable("COLORTERM");
        if !variable("NO_COLOR").is_empty() || term == "dumb" {
            Self::Plain
        } else if matches!(colorterm.as_str(), "truecolor" | "24bit")
            || term.ends_with("-direct")
            || ["kitty", "ghostty", "wezterm", "alacritty", "foot"]
                .iter()
                .any(|name| term.contains(name))
        {
            Self::TrueColor
        } else {
            Self::Palette
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "never" | "plain" | "none" => Some(Self::Plain),
            "256" | "palette" => Some(Self::Palette),
            "truecolor" | "24bit" => Some(Self::TrueColor),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Palette => "256 colors",
            Self::TrueColor => "truecolor",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::TrueColor => Self::Palette,
            Self::Palette => Self::Plain,
            Self::Plain => Self::TrueColor,
        }
    }

    pub fn is_colored(self) -> bool {
        self != Self::Plain
    }

    pub fn paint(self, color: Option<Rgb>) -> Option<Paint> {
        match self {
            Self::Plain => None,
            Self::Palette => color.map(|color| Paint::Indexed(palette_index(color))),
            Self::TrueColor => color.map(Paint::TrueColor),
        }
    }
}

impl Paint {
    pub fn write_sgr(paint: Option<Self>, ground: Ground, output: &mut String) {
        let base = match ground {
            Ground::Foreground => 30,
            Ground::Background => 40,
        };
        // Writing to a String cannot fail.
        let _ = match paint {
            None => write!(output, "\x1b[{}m", base + 9),
            Some(Self::Indexed(index)) => write!(output, "\x1b[{};5;{index}m", base + 8),
            Some(Self::TrueColor(Rgb([red, green, blue]))) => {
                write!(output, "\x1b[{};2;{red};{green};{blue}m", base + 8)
            }
        };
    }
}

// The xterm palette: a 6x6x6 color cube at 16..=231 and 24 grays at 232..=255.
const CUBE_LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];

fn palette_index(Rgb(channels): Rgb) -> u8 {
    let level = |value: u8| match value {
        0..48 => 0,
        48..115 => 1,
        _ => (value - 35) / 40,
    };
    let distance = |candidate: [u8; 3]| -> u32 {
        channels
            .iter()
            .zip(candidate)
            .map(|(a, b)| (*a as i32 - b as i32).pow(2) as u32)
            .sum()
    };

    let cube = channels.map(level);
    let cube_color = cube.map(|index| CUBE_LEVELS[index as usize]);
    let average = channels.iter().map(|value| *value as u32).sum::<u32>() / 3;
    let gray = (average.saturating_sub(3) / 10).min(23) as u8;
    let gray_value = 8 + gray * 10;

    if distance([gray_value; 3]) < distance(cube_color) {
        232 + gray
    } else {
        16 + 36 * cube[0] + 6 * cube[1] + cube[2]
    }
}

/// Splits a color into its brightest channel and the hue scaled to full
/// intensity. Black has no hue, so it becomes white with value 0.
pub fn value_and_hue(color: Vec3) -> (f32, Vec3) {
    let value = color.max_element();
    if value > 1e-6 {
        (value, color / value)
    } else {
        (0.0, Vec3::ONE)
    }
}

/// Hue, saturation, and value, each from 0 to 1, to RGB.
pub fn hsv(hue: f32, saturation: f32, value: f32) -> Vec3 {
    let channel = |offset: f32| {
        let k = (offset + hue.rem_euclid(1.0) * 6.0) % 6.0;
        value - value * saturation * k.min(4.0 - k).clamp(0.0, 1.0)
    };
    Vec3::new(channel(5.0), channel(3.0), channel(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_maps_primaries_and_grays_to_their_nearest_entries() {
        assert_eq!(palette_index(Rgb([255, 0, 0])), 196);
        assert_eq!(palette_index(Rgb([0, 0, 0])), 16);
        assert_eq!(palette_index(Rgb([255, 255, 255])), 231);
        assert_eq!(palette_index(Rgb([128, 128, 128])), 244);
    }

    #[test]
    fn hsv_produces_primary_colors() {
        assert!(hsv(0.0, 1.0, 1.0).distance(Vec3::X) < 1e-5);
        assert!(hsv(1.0 / 3.0, 1.0, 1.0).distance(Vec3::Y) < 1e-5);
        assert!(hsv(2.0 / 3.0, 1.0, 1.0).distance(Vec3::Z) < 1e-5);
        assert!(hsv(0.5, 0.0, 0.5).distance(Vec3::splat(0.5)) < 1e-5);
    }

    #[test]
    fn plain_mode_never_paints() {
        assert_eq!(ColorMode::Plain.paint(Some(Rgb([1, 2, 3]))), None);
        assert_eq!(
            ColorMode::TrueColor.paint(Some(Rgb([1, 2, 3]))),
            Some(Paint::TrueColor(Rgb([1, 2, 3])))
        );
    }
}
