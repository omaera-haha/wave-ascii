use glam::{Vec2, Vec3};

use super::{FOCAL, View, glow::Glow, sphere_span};
use crate::sdf::coverage;
use crate::world::bubbles::{Bubble, LIMIT, POP_FOR};

/// Projected circles and shimmer are worked out once, before sampling pixels.
pub(super) struct Shell {
    position: Vec3,
    radius: f32,
    center: Vec2,
    screen_radius_squared: f32,
    popping: Option<f32>,
    phase: f32,
    shimmer: f32,
}

impl Shell {
    pub fn new(bubble: &Bubble, view: View, time: f64, pixel: f32) -> Option<Self> {
        let popping = bubble
            .popping
            .map(|at| ((time - at) / POP_FOR).clamp(0.0, 1.0) as f32);
        let radius = bubble.radius * popping.map_or(1.0, |age| 1.0 + age * 1.2);
        let center = view.project(bubble.position)?;
        let depth = (bubble.position - view.eye).dot(view.forward);
        let projection = radius * (FOCAL + center.length()) / (depth - radius).max(0.01);
        Some(Self {
            position: bubble.position,
            radius,
            center,
            screen_radius_squared: (projection + pixel * 1.5).powi(2),
            popping,
            phase: bubble.seed * 3.0 + (time - bubble.born) as f32 * 0.35,
            shimmer: 0.85 + 0.15 * ((time - bubble.born) as f32 * 2.0 + bubble.seed).sin(),
        })
    }

    fn shade(&self, view: View, direction: Vec3, pixel: f32) -> Option<(f32, Glow)> {
        if let Some(age) = self.popping {
            let distance =
                (self.position - view.eye).dot(view.forward) / direction.dot(view.forward);
            if distance <= 0.0 {
                return None;
            }
            let offset = view.eye + direction * distance - self.position;
            let local = Vec2::new(offset.dot(view.right), offset.dot(view.up));
            let around = local.y.atan2(local.x);
            let droplets = (around * 9.0 + self.phase).cos().powi(12);
            let spray = coverage(
                (local.length() - self.radius).abs() - 0.004,
                (pixel * distance / FOCAL).max(0.001),
            ) * droplets
                * (1.0 - age);
            return Some((
                distance,
                Glow {
                    light: Vec3::new(0.85, 0.92, 1.0) * spray,
                    cover: spray * 0.3,
                },
            ));
        }
        let (distance, _) = sphere_span(view.eye, direction, self.position, self.radius)?;
        if distance <= 0.0 {
            return None;
        }
        let normal = (view.eye + direction * distance - self.position) / self.radius;
        let angle = normal.dot(-direction).abs().clamp(0.0, 1.0);
        let rim = (1.0 - angle).powi(3);
        let film = self.phase + angle * 13.0;
        let rainbow = Vec3::new(film.sin(), (film + 2.1).sin(), (film + 4.2).sin()) * 0.25
            + Vec3::splat(0.72);
        let highlight = normal
            .dot((-view.forward + view.up * 0.7 - view.right * 0.6).normalize())
            .max(0.0)
            .powi(45);
        let opacity = 0.025 + rim * 0.45;
        Some((
            distance,
            Glow {
                light: rainbow * opacity * self.shimmer + Vec3::splat(highlight * 0.7),
                cover: opacity * 0.5,
            },
        ))
    }
}

pub(super) fn hits(
    shells: &[Shell],
    view: View,
    direction: Vec3,
    limit: f32,
    pixel: f32,
) -> [Option<(f32, Glow)>; LIMIT] {
    let point = Vec2::new(direction.dot(view.right), direction.dot(view.up)) * FOCAL
        / direction.dot(view.forward);
    let mut hits = [None; LIMIT];
    let mut count = 0;
    for shell in shells {
        // Empty pixels never reach intersection, normal, or film math.
        if point.distance_squared(shell.center) > shell.screen_radius_squared {
            continue;
        }
        if let Some((distance, glow)) = shell.shade(view, direction, pixel)
            && distance < limit
            && count < LIMIT
        {
            hits[count] = Some((distance, glow));
            count += 1;
        }
    }
    hits[..count].sort_by(|first, second| second.unwrap().0.total_cmp(&first.unwrap().0));
    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::Camera;

    #[test]
    fn projected_shells_and_spray_have_finite_bounded_light_at_every_angle() {
        for yaw in [-2.0, 0.0, 1.5] {
            let view = Camera {
                yaw,
                ..Camera::default()
            }
            .view(crate::frame::Grid::new(100, 64, 0.5));
            for popping in [None, Some(0.0), Some(0.2)] {
                let bubble = Bubble {
                    position: Vec3::new(0.0, 1.2, 0.0),
                    velocity: Vec3::ZERO,
                    radius: 0.11,
                    seed: 2.0,
                    born: 0.0,
                    lifetime: 12.0,
                    popping,
                };
                let shell = Shell::new(&bubble, view, 0.25, 0.01).unwrap();
                assert!(shell.center.is_finite() && shell.screen_radius_squared.is_finite());
                for horizontal in -10..=10 {
                    for vertical in -10..=10 {
                        let point =
                            shell.center + Vec2::new(horizontal as f32, vertical as f32) * 0.005;
                        for (distance, glow) in hits(
                            std::slice::from_ref(&shell),
                            view,
                            view.ray(point),
                            100.0,
                            0.01,
                        )
                        .into_iter()
                        .flatten()
                        {
                            assert!(distance.is_finite() && distance > 0.0);
                            assert!(glow.light.is_finite() && glow.light.min_element() >= 0.0);
                            assert!((0.0..=1.0).contains(&glow.cover));
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn the_wand_has_finite_distances_normals_and_bounded_ray_hits() {
        let piece = crate::render::shapes::Furniture::BubbleWand(Vec3::new(0.0, 1.3, 0.4));
        let (center, radius) = piece.bounds();
        assert!(center.is_finite() && radius.is_finite() && radius > 0.0);
        for horizontal in -5..=5 {
            for vertical in -5..=5 {
                let position =
                    center + Vec3::new(horizontal as f32, vertical as f32, 0.0) * radius / 5.0;
                assert!(piece.distance(position).0.is_finite());
                assert!(piece.normal(position).is_finite());
                if let Some((distance, _)) = piece.trace(position + Vec3::Z, -Vec3::Z, 2.0) {
                    assert!(distance.is_finite() && (0.0..=2.0).contains(&distance));
                }
            }
        }
        for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
            for side in [-1.0, 1.0] {
                assert!(piece.distance(center + axis * side * radius).0 > 0.0);
            }
        }
    }
}
