//! The back window: a real opening in the wall with a wooden frame, a
//! middle bar, and a lower sash that slides up when it's open.

use glam::Vec2;

use crate::sdf::{coverage, smoothstep};

pub const WINDOW_LOW: Vec2 = Vec2::new(-0.75, 1.0);
pub const WINDOW_HIGH: Vec2 = Vec2::new(0.75, 2.0);

/// What a point of the window shows: how much frame covers it, and how
/// much faint sheen its glass adds (none where the sash is open).
pub struct Pane {
    pub frame: f32,
    pub sheen: f32,
}

/// The window at `point` on the back wall, as (x, y), with the lower sash
/// lifted by `lift` from 0 shut to 1 open.
pub fn pane(point: Vec2, footprint: f32, lift: f32) -> Option<Pane> {
    pane_in(point, footprint, lift, WINDOW_LOW, WINDOW_HIGH)
}

/// A window like it filling the rectangle from `low` to `high`.
pub fn pane_in(point: Vec2, footprint: f32, lift: f32, low: Vec2, high: Vec2) -> Option<Pane> {
    if point.cmplt(low).any() || point.cmpgt(high).any() {
        return None;
    }
    let size = high - low;
    let coordinates = (point - low) / size;
    // A point sample still needs a nonzero edge at exact shape boundaries.
    let footprint = footprint.max(1e-5);
    let border = (point - low).min(high - point).min_element();
    let middle = low.y + size.y * 0.5;
    let lift = lift.clamp(0.0, 1.0);
    let bottom = low.y + size.y * 0.5 * lift;
    let glazed = point.y >= bottom;
    let distance = (border - 0.05)
        .min(if glazed {
            (point.x - (low.x + size.x * 0.5)).abs() - 0.0175
        } else {
            1.0
        })
        .min((point.y - middle).abs() - 0.02)
        .min((point.y - bottom).abs() - 0.02)
        .min((point.y - middle - size.y * 0.5 * lift).abs() - 0.02);
    let sheen = if glazed {
        let stripe = 1.0
            - smoothstep(
                0.0,
                0.07,
                (coordinates.x + coordinates.y * 0.3 - 0.62).abs(),
            );
        0.03 + stripe * 0.08
    } else {
        0.0
    };
    Some(Pane {
        frame: coverage(distance, footprint * 0.5),
        sheen,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_points_inside_the_window_rectangle_have_a_pane() {
        assert!(pane(Vec2::new(0.3, 1.3), 0.01, 0.0).is_some());
        assert!(pane(Vec2::new(0.8, 1.3), 0.01, 0.0).is_none());
        assert!(pane(Vec2::new(0.0, 0.9), 0.01, 0.0).is_none());
    }

    #[test]
    fn the_border_and_bars_are_frame_and_the_panes_are_not() {
        let shut = |x, y| pane(Vec2::new(x, y), 0.001, 0.0).unwrap();
        assert!(shut(-0.73, 1.3).frame > 0.9);
        assert!(shut(0.0, 1.25).frame > 0.9);
        assert!(shut(0.3, 1.5).frame > 0.9);
        assert!(shut(0.35, 1.25).frame < 0.1);
        let open = pane(Vec2::new(0.0, 1.25), 0.001, 1.0).unwrap();
        assert!(open.frame < 0.1 && open.sheen == 0.0);
    }
}
