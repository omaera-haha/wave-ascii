use std::f32::consts::TAU;

use glam::{Mat2, Vec2, Vec3};

use crate::frame::{Frame, Grid, Shade};
use crate::scene::coverage;

/// Seconds for the face to run through its expressions.
const CYCLE: f32 = 12.0;
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
/// ripples and trails as it drifts, its eyes look around and blink, and once
/// a cycle they melt into a terminal prompt with a blinking cursor. A ring of
/// light follows its outline. `turn` rotates the view and `zoom` scales it.
pub fn render(grid: Grid, time: f32, turn: f32, zoom: f32) -> Frame {
    let pose = Pose::at(time);
    let pixel = grid.scale() / zoom;
    let gap = GAP.max(1.5 * pixel);
    let ring_width = RING_WIDTH.max(1.2 * pixel);
    let rim_width = 0.035_f32.max(1.2 * pixel);
    let view = Mat2::from_angle(turn);
    let lean = Mat2::from_angle(-pose.lean);

    Frame::sample(grid, |point| {
        let point = lean * (view * point / zoom - Vec2::new(pose.drift, pose.lift));
        // The lower body lags behind the head as the ghost drifts.
        let below = (HEAD_CENTER.y - point.y).max(0.0);
        let point = Vec2::new(point.x + pose.trail * below * below, point.y);

        let silhouette = pose.silhouette(point);
        let shape = silhouette.max(-pose.face(point - HEAD_CENTER));
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

/// Where the ghost is and what its face is doing at one moment.
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
    /// Where the eyes look, from -1 to 1 across and up.
    gaze: Vec2,
    blink: f32,
    /// From eyes at 0 to a prompt at 1.
    prompt: f32,
    cursor: f32,
}

impl Pose {
    fn at(time: f32) -> Self {
        let phase = time.rem_euclid(CYCLE);
        // Bobbing and drifting on periods that divide the cycle, so the loop
        // is seamless.
        let bob = TAU * time / 3.0;
        let sway = TAU * time / 6.0;
        let cursor = if (6.3..9.0).contains(&phase) {
            smoothstep(-0.35, 0.35, (TAU * (phase - 6.3) / 0.9).cos())
        } else {
            1.0
        };
        Self {
            lift: 0.045 * bob.sin(),
            drift: 0.04 * sway.sin(),
            lean: -0.07 * sway.cos(),
            trail: -0.12 * sway.cos(),
            stretch: 0.025 * bob.cos(),
            ripple: 2.2 * time,
            gaze: Vec2::new(
                ease_through(
                    &[
                        (1.0, 0.0),
                        (1.6, -1.0),
                        (2.6, -1.0),
                        (3.3, 0.9),
                        (4.6, 0.9),
                        (5.3, 0.0),
                        (10.3, 0.0),
                        (10.8, -0.5),
                        (11.4, -0.5),
                        (11.9, 0.0),
                    ],
                    phase,
                ),
                ease_through(&[(2.6, 0.0), (3.3, 0.7), (4.6, 0.7), (5.3, 0.0)], phase),
            ),
            blink: ease_through(
                &[
                    (2.78, 0.0),
                    (2.88, 1.0),
                    (3.0, 0.0),
                    (10.88, 0.0),
                    (10.98, 1.0),
                    (11.1, 0.0),
                ],
                phase,
            ),
            prompt: ease_through(&[(5.6, 0.0), (6.3, 1.0), (9.2, 1.0), (9.9, 0.0)], phase),
            cursor,
        }
    }

    /// Signed distance to the ghost's outline: a round head over a skirt
    /// that flares a little and ends in round lobes, which rise and fall in a
    /// wave running along the hem.
    fn silhouette(&self, point: Vec2) -> f32 {
        let head = (point - HEAD_CENTER).length() - HEAD_RADIUS;

        let hem = HEM_HEIGHT - self.stretch;
        let depth = ((HEAD_CENTER.y - point.y) / (HEAD_CENTER.y - hem)).clamp(0.0, 1.0);
        let corner = Vec2::new(
            point.x.abs() - HEAD_RADIUS * (1.0 + FLARE * depth),
            hem - point.y,
        );
        let skirt = (corner.max(Vec2::ZERO).length() + corner.max_element().min(0.0))
            .max(point.y - HEAD_CENTER.y);

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

    /// Signed distance to the face, cut out of the head, with `point`
    /// relative to the head's center. Each feature changes shape by moving
    /// its own points and sizes, so every moment of a change is a clean shape.
    fn face(&self, point: Vec2) -> f32 {
        let look = self.gaze * Vec2::new(0.07, 0.05);
        // An open eye is a tall pill; a shut one is a thin line.
        let eye_size = Vec2::new(0.055, 0.125).lerp(Vec2::new(0.065, 0.016), self.blink);
        let eye_radius = mix(0.055, 0.016, self.blink);
        let eye_offset = look - Vec2::new(0.0, 0.02 * self.blink);

        // The left eye bends at its middle into `>`.
        let left_eye = Vec2::new(-0.17, -0.03) + eye_offset;
        let left = if self.prompt > 0.0 {
            let point = point - left_eye.lerp(Vec2::new(-0.15, -0.04), self.prompt);
            let bend = 0.05 * self.prompt;
            let reach = mix(0.07, 0.08, self.prompt);
            let radius = mix(0.055, 0.03, self.prompt);
            let tip = Vec2::new(bend, 0.0);
            capsule(point, Vec2::new(-bend, reach), tip, radius).min(capsule(
                point,
                tip,
                Vec2::new(-bend, -reach),
                radius,
            ))
        } else {
            rounded_box(point - left_eye, eye_size, eye_radius)
        };

        // The right eye tips over into an underscore, which then blinks like
        // a cursor.
        let right_eye = Vec2::new(0.17, -0.03) + eye_offset;
        let size = eye_size.lerp(Vec2::new(0.075, 0.024 * self.cursor), self.prompt);
        let right = rounded_box(
            point - right_eye.lerp(Vec2::new(0.12, -0.115), self.prompt),
            size,
            mix(eye_radius, 0.01, self.prompt).min(size.min_element()),
        ) + 0.03 * (1.0 - self.cursor);
        left.min(right)
    }
}

fn capsule(point: Vec2, start: Vec2, end: Vec2, radius: f32) -> f32 {
    let along = end - start;
    let offset = point - start;
    let fraction = (offset.dot(along) / along.length_squared()).clamp(0.0, 1.0);
    (offset - along * fraction).length() - radius
}

fn rounded_box(point: Vec2, half_size: Vec2, radius: f32) -> f32 {
    let outside = point.abs() - half_size + Vec2::splat(radius);
    outside.max(Vec2::ZERO).length() + outside.max_element().min(0.0) - radius
}

fn mix(from: f32, to: f32, amount: f32) -> f32 {
    from + (to - from) * amount
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
