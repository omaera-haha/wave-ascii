use std::f32::consts::{PI, TAU};

use glam::{Mat2, Vec2, Vec3};

use crate::frame::{Frame, Grid, Shade};
use crate::scene::{Viewer, coverage};

mod expression;
mod situation;

const HEAD_CENTER: Vec2 = Vec2::new(0.0, 0.2);
const HEAD_RADIUS: f32 = 0.46;
const HEM_HEIGHT: f32 = -0.56;
// Round lobes hang from the hem, rising and falling by `LOBE_SWING`.
const LOBES: usize = 4;
const LOBE_SWING: f32 = 0.03;
// How much wider the ghost is at the hem than at the head.
const FLARE: f32 = 0.07;
// The outline ring's distance from the ghost and its width.
const GAP: f32 = 0.1;
const RING_WIDTH: f32 = 0.065;
const BODY_TOP: Vec3 = Vec3::ONE;
const BODY_BOTTOM: Vec3 = Vec3::new(0.8, 0.87, 1.0);
const BLUE: Vec3 = Vec3::new(0.27, 0.45, 1.0);
const CYAN: Vec3 = Vec3::new(0.35, 0.88, 1.0);

/// A ghost floating in place, after the one on Ghostty's home page. Its hem
/// ripples and trails as it drifts, and its face follows what the viewer
/// does: left alone it makes faces of its own, it watches the mouse pointer,
/// reacts to clicks, and falls asleep when nobody is around. A ring of light
/// follows its outline. `turn` rotates the view and `zoom` scales it.
pub fn render(grid: Grid, time: f32, turn: f32, zoom: f32, viewer: &Viewer) -> Frame {
    let pose = Pose::at(time);
    let face = situation::face(time, viewer, |time, point| {
        let pose = Pose::at(time);
        let point = pose.placement(turn, zoom)(point);
        (point, pose.silhouette(point))
    });
    let pixel = grid.scale() / zoom;
    let gap = GAP.max(1.5 * pixel);
    let ring_width = RING_WIDTH.max(1.2 * pixel);
    let rim_width = 0.035_f32.max(1.2 * pixel);
    let place = pose.placement(turn, zoom);

    Frame::sample(grid, |point| {
        let point = place(point);
        let silhouette = pose.silhouette(point);
        let shape = silhouette.max(-face.distance(point - HEAD_CENTER));
        let inside = coverage(shape, pixel * 0.5);
        if inside > 0.0 {
            let height = ((point.y - HEM_HEIGHT) / (HEAD_CENTER.y + HEAD_RADIUS - HEM_HEIGHT))
                .clamp(0.0, 1.0);
            let rim = 1.0 - smoothstep(0.0, rim_width, -shape);
            return Shade::new(
                inside * (0.84 + 0.06 * height + 0.16 * rim),
                BODY_BOTTOM.lerp(BODY_TOP, height.sqrt()),
            );
        }

        let ring = coverage(
            (silhouette - gap - 0.5 * ring_width).abs() - 0.5 * ring_width,
            pixel * 0.5,
        );
        if ring <= 0.0 {
            return Shade::EMPTY;
        }
        // Light runs around the ring in two waves that pass through each
        // other.
        let around = (point.y + 0.15).atan2(point.x);
        let wave =
            0.6 * (3.0 * around - 1.7 * time).sin() + 0.4 * (5.0 * around + 1.1 * time).sin();
        Shade::new(
            ring * (0.62 + 0.38 * wave),
            BLUE.lerp(CYAN, 0.5 + 0.5 * (2.0 * around - 0.6 * time).sin()),
        )
    })
}

/// Where the ghost's body is at one moment.
struct Pose {
    lift: f32,
    drift: f32,
    lean: f32,
    /// How far the hem trails behind the head.
    trail: f32,
    /// How far the hem stretches down while the ghost rises.
    stretch: f32,
    /// The phase of the waves running along the hem.
    ripple: f32,
}

impl Pose {
    fn at(time: f32) -> Self {
        let bob = TAU * time / 3.0;
        let sway = TAU * time / 6.0;
        Self {
            lift: 0.045 * bob.sin(),
            drift: 0.04 * sway.sin(),
            lean: -0.07 * sway.cos(),
            trail: -0.12 * sway.cos(),
            stretch: 0.025 * bob.cos(),
            ripple: 2.2 * time,
        }
    }

    /// Maps points of a view turned by `turn` and scaled by `zoom` into the
    /// ghost's own space, where it stands upright with its head at
    /// `HEAD_CENTER`.
    fn placement(&self, turn: f32, zoom: f32) -> impl Fn(Vec2) -> Vec2 + Sync + use<> {
        let view = Mat2::from_angle(turn);
        let lean = Mat2::from_angle(-self.lean);
        let offset = Vec2::new(self.drift, self.lift);
        let trail = self.trail;
        move |point| {
            let point = lean * (view * point / zoom - offset);
            // The lower body lags behind the head as the ghost drifts.
            let below = (HEAD_CENTER.y - point.y).max(0.0);
            Vec2::new(point.x + trail * below * below, point.y)
        }
    }

    /// Signed distance to the ghost's outline: a round head over a skirt
    /// that flares a little and ends in round lobes, which rise and fall in a
    /// wave running along the hem.
    fn silhouette(&self, point: Vec2) -> f32 {
        let head = (point - HEAD_CENTER).length() - HEAD_RADIUS;

        // The skirt's distance is exact outside it too, so the outline ring,
        // which is an offset of this distance, rounds its top corners
        // instead of squaring them off where they meet the head.
        let hem = HEM_HEIGHT - self.stretch;
        let skirt = trapezoid(
            point - Vec2::new(0.0, 0.5 * (HEAD_CENTER.y + hem)),
            HEAD_RADIUS * (1.0 + FLARE),
            HEAD_RADIUS,
            0.5 * (HEAD_CENTER.y - hem),
        );

        let hem_width = HEAD_RADIUS * (1.0 + FLARE);
        let radius = hem_width / LOBES as f32;
        let lobes = (0..LOBES)
            .map(|index| {
                let center = Vec2::new(
                    radius * (2 * index + 1) as f32 - hem_width,
                    hem + LOBE_SWING * (self.ripple - 1.3 * index as f32).sin(),
                );
                point.distance(center) - radius
            })
            .fold(f32::INFINITY, f32::min);
        smooth_min(head.min(skirt), lobes, 0.04)
    }
}

/// What the face shows at one moment: the controls that expressions move.
/// At rest every control is zero except `cursor`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Face {
    /// Where the eyes look, from -1 to 1 across and up.
    gaze: Vec2,
    /// How shut each eye is, left then right, from open at 0 to a line at 1.
    shut: [f32; 2],
    /// How far each upper lid has come down over its eye, left then right,
    /// from 0 to covering it at 1.
    lid: [f32; 2],
    /// How the lids slant, from -1 to 1: positive lowers their inner ends
    /// for a glare and negative lowers their outer ends for worry.
    slant: f32,
    /// How wide the eyes open, from narrow at -1 to wide at 1.
    widen: f32,
    /// From eyes at 0 to happy `^` arcs at 1.
    smile: f32,
    /// The size of the twinkles of light in the eyes, from none at 0 to 1.
    sparkle: f32,
    /// From eyes at 0 to a prompt at 1.
    prompt: f32,
    /// How visible the prompt's cursor is, from 0 to 1.
    cursor: f32,
}

impl Face {
    const REST: Self = Self {
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

    /// The face `amount` of the way from this one to `other`.
    fn lerp(self, other: Self, amount: f32) -> Self {
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
    fn distance(&self, point: Vec2) -> f32 {
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

/// Signed distance to a thin four-pointed star centered on the origin, with
/// its points `radius` away along the axes.
fn star(point: Vec2, radius: f32) -> f32 {
    // Fold the point into the eighth of the turn between a point of the star,
    // which lands at `tip`, and the notch beside it on the x axis, then
    // measure to the edge running in from the tip.
    let half_sector = PI / 4.0;
    let tip = Vec2::from_angle(half_sector);
    let edge = Vec2::from_angle(PI / 3.0);
    let angle = point.x.atan2(point.y).rem_euclid(2.0 * half_sector) - half_sector;
    let point = point.length() * Vec2::new(angle.cos(), angle.sin().abs()) - radius * tip;
    let point = point + edge * (-point.dot(edge)).clamp(0.0, radius * tip.y / edge.y);
    point.length() * point.x.signum()
}

/// Signed distance to two strokes of `radius` from `tip` to each of `ends`.
fn chevron(point: Vec2, tip: Vec2, ends: [Vec2; 2], radius: f32) -> f32 {
    capsule(point, tip, ends[0], radius).min(capsule(point, tip, ends[1], radius))
}

fn capsule(point: Vec2, start: Vec2, end: Vec2, radius: f32) -> f32 {
    let along = end - start;
    let offset = point - start;
    let fraction = (offset.dot(along) / along.length_squared()).clamp(0.0, 1.0);
    (offset - along * fraction).length() - radius
}

/// Signed distance to a trapezoid centered on the origin and symmetric about
/// the y axis, with half widths `bottom` and `top` at its bottom and top
/// edges, `half_height` from the center.
fn trapezoid(point: Vec2, bottom: f32, top: f32, half_height: f32) -> f32 {
    let point = Vec2::new(point.x.abs(), point.y);
    let width = if point.y < 0.0 { bottom } else { top };
    let end = Vec2::new(point.x - point.x.min(width), point.y.abs() - half_height);
    let corner = Vec2::new(top, half_height);
    let side = Vec2::new(top - bottom, 2.0 * half_height);
    let fraction = ((corner - point).dot(side) / side.length_squared()).clamp(0.0, 1.0);
    let slant = point - corner + side * fraction;
    let sign = if slant.x < 0.0 && end.y < 0.0 {
        -1.0
    } else {
        1.0
    };
    sign * end.length_squared().min(slant.length_squared()).sqrt()
}

fn rounded_box(point: Vec2, half_size: Vec2, radius: f32) -> f32 {
    let outside = point.abs() - half_size + Vec2::splat(radius);
    outside.max(Vec2::ZERO).length() + outside.max_element().min(0.0) - radius
}

// Exactly `from` at 0 and `to` at 1.
fn mix(from: f32, to: f32, amount: f32) -> f32 {
    from * (1.0 - amount) + to * amount
}

/// The union of two shapes with the seam between them rounded over `width`.
fn smooth_min(a: f32, b: f32, width: f32) -> f32 {
    let blend = (0.5 + 0.5 * (b - a) / width).clamp(0.0, 1.0);
    b + (a - b) * blend - width * blend * (1.0 - blend)
}

fn smoothstep(start: f32, end: f32, value: f32) -> f32 {
    let x = ((value - start) / (end - start)).clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// The value at `time` along `keys` of (time, value), easing in and out
/// between keys and holding before the first and after the last.
fn ease_through(keys: &[(f32, f32)], time: f32) -> f32 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star_points_reach_the_radius_along_the_axes_and_the_notches_are_narrow() {
        let radius = 0.04;
        assert!(star(Vec2::ZERO, radius) < 0.0);
        for direction in [Vec2::X, Vec2::Y, -Vec2::X, -Vec2::Y] {
            assert!(star(direction * radius, radius).abs() < 1e-5, "{direction}");
            assert!(star(direction * radius * 0.9, radius) < 0.0, "{direction}");
        }
        let diagonal = Vec2::new(1.0, 1.0).normalize();
        assert!(star(diagonal * radius * 0.5, radius) > 0.0);
    }

    #[test]
    fn trapezoid_distance_is_the_distance_to_the_nearest_edge() {
        let (bottom, top, half_height) = (0.5, 0.3, 0.4);
        let corners = [
            Vec2::new(-bottom, -half_height),
            Vec2::new(bottom, -half_height),
            Vec2::new(top, half_height),
            Vec2::new(-top, half_height),
        ];
        for column in -20..=20 {
            for row in -20..=20 {
                let point = Vec2::new(column as f32, row as f32) * 0.05;
                let nearest = (0..4)
                    .map(|index| capsule(point, corners[index], corners[(index + 1) % 4], 0.0))
                    .fold(f32::INFINITY, f32::min);
                let width = mix(bottom, top, (point.y + half_height) / (2.0 * half_height));
                let inside = point.y.abs() < half_height && point.x.abs() < width;
                let expected = if inside { -nearest } else { nearest };
                let distance = trapezoid(point, bottom, top, half_height);
                assert!(
                    (distance - expected).abs() < 1e-5,
                    "{point}: {distance} vs {expected}"
                );
            }
        }
    }
}
