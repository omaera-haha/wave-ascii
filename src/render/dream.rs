use glam::{Vec2, Vec3};

use super::ghost::Glow;
use super::{FOCAL, Scene};
use crate::sdf::{coverage, smooth_min, smoothstep};
use crate::world::dream::{DreamPicture, HOLDS, SLEEPING};

const LOW: Vec2 = Vec2::new(-1.25, -1.65);
const HIGH: Vec2 = Vec2::new(1.2, 1.05);

/// Geometry and screen bounds shared by all rays in one frame.
pub(super) struct Bubble {
    pub position: Vec3,
    pub scale: f32,
    bounds: [Vec2; 2],
    picture: DreamPicture,
    opacity: f32,
    drawn: f32,
    color: Vec3,
}

impl Bubble {
    pub fn new(scene: &Scene, ghost_scale: f32, pixel: f32) -> Option<Self> {
        let dream = scene.world.dream?;
        let opacity = dream.envelope(scene.world.time);
        if scene.world.ghost.asleep <= SLEEPING || opacity <= 0.0 {
            return None;
        }
        let age = ((scene.world.time - dream.started) / HOLDS).clamp(0.0, 1.0) as f32;
        let scale = ghost_scale.max(1e-6) * (0.72 + 0.28 * opacity);
        let position = scene.world.ghost.position
            + scene.view.right * ghost_scale * 0.9
            + scene.view.up * ghost_scale * (2.15 + 0.25 * age);
        let center = scene.view.project(position)?;
        let depth = (position - scene.view.eye).dot(scene.view.forward);
        let projection = scale * FOCAL / depth;
        Some(Self {
            position,
            scale,
            bounds: [
                center + LOW * projection - Vec2::splat(pixel),
                center + HIGH * projection + Vec2::splat(pixel),
            ],
            picture: dream.picture,
            opacity,
            drawn: dream.drawn(scene.world.time),
            color: Vec3::new(0.84, 0.76, 1.0).lerp(scene.aura.color, 0.25),
        })
    }

    pub fn contains(&self, screen: Vec2) -> bool {
        screen.cmpge(self.bounds[0]).all() && screen.cmple(self.bounds[1]).all()
    }

    pub fn shade(&self, point: Vec2, pixel: f32) -> Glow {
        let pixel = pixel.clamp(1e-5, 0.15);
        if point.cmplt(LOW).any() || point.cmpgt(HIGH).any() {
            return Glow::NONE;
        }
        let mut light = 0.0_f32;
        let mut cover = 0.0_f32;
        for (center, radius) in [
            (Vec2::new(-1.0, -1.4), 0.075),
            (Vec2::new(-0.77, -1.08), 0.115),
            (Vec2::new(-0.5, -0.73), 0.16),
        ] {
            let distance = point.distance(center) - radius;
            if distance <= 0.025 + pixel {
                let fill = coverage(distance, pixel * 0.5);
                let ring = coverage(distance.abs() - 0.025, pixel * 0.5);
                light = light.max(fill * 0.08 + ring * 0.5);
                cover = cover.max(fill * 0.08 + ring * 0.45);
            }
        }

        // Puffs are cheap; the cloud and its strokes only touch this circle.
        if point.length_squared() <= (1.08 + pixel).powi(2) {
            let mut cloud = point.distance(Vec2::new(-0.48, 0.0)) - 0.48;
            for (center, radius) in [
                (Vec2::new(0.0, 0.2), 0.58),
                (Vec2::new(0.5, 0.0), 0.48),
                (Vec2::new(0.0, -0.15), 0.6),
            ] {
                cloud = smooth_min(cloud, point.distance(center) - radius, 0.12);
            }
            let fill = coverage(cloud, pixel * 0.5);
            let ring = coverage(cloud.abs() - 0.025, pixel * 0.5);
            light = light.max(fill * 0.065 + ring * 0.45);
            cover = cover.max(fill * 0.08 + ring * 0.4);
            if point.length_squared() <= 0.8_f32.powi(2) && self.drawn > 0.0 {
                let distance = self.picture.distance(point / 0.55, self.drawn) * 0.55;
                let stroke = coverage(distance - 0.025, pixel * 0.5);
                let halo = 1.0 - smoothstep(0.025, 0.1, distance);
                light += stroke * 0.85 + halo * 0.18;
                cover = (cover + stroke * 0.85).min(1.0);
            }
        }
        Glow {
            light: self.color * light * self.opacity,
            cover: cover * self.opacity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::Face;
    use crate::frame::Grid;
    use crate::render::ghost::{Aura, Pose};
    use crate::render::{Camera, GHOST_SCALE, window};
    use crate::world::{Dream, World};

    #[test]
    fn bubble_samples_stay_finite_at_envelope_edges_and_tiny_sizes() {
        let mut world = World::new(4, 23.0);
        world.tuck_in();
        let grid = Grid::cells(100, 32);
        let view = Camera::default().view(grid);
        for picture in DreamPicture::ALL {
            world.dream = Some(Dream {
                picture,
                started: 0.0,
            });
            for time in [0.0, 1e-6, 0.75, 1.5, 2.0, 10.0, HOLDS - 1e-6, HOLDS] {
                world.time = time;
                let scene = Scene {
                    world: &world,
                    outside: window::Outside::clear(23.0, time),
                    view,
                    face: Face::REST,
                    aura: Aura {
                        color: Vec3::new(0.7, 0.8, 1.0),
                        visibility: 1.0,
                        brightness: 1.0,
                        beat: 1.0,
                    },
                    pose: Pose::new(time, 0.0, 0.0, 1.0),
                    friend: None,
                };
                for scale in [GHOST_SCALE, GHOST_SCALE * 1e-6] {
                    let Some(bubble) = Bubble::new(&scene, scale, grid.scale()) else {
                        assert!(time == 0.0 || time == HOLDS);
                        continue;
                    };
                    assert!(bubble.bounds.iter().all(|point| point.is_finite()));
                    for point in [
                        Vec2::ZERO,
                        Vec2::new(0.0, 0.8),
                        Vec2::new(-1.0, -1.4),
                        LOW,
                        HIGH,
                    ] {
                        let screen = view
                            .project(
                                bubble.position
                                    + (view.right * point.x + view.up * point.y) * bubble.scale,
                            )
                            .unwrap();
                        assert!(bubble.contains(screen));
                        let (distance, local) = view
                            .billboard(view.ray(screen), bubble.position, bubble.scale)
                            .unwrap();
                        let sample =
                            bubble.shade(local, distance * grid.scale() / FOCAL / bubble.scale);
                        assert!(sample.light.is_finite());
                        assert!(sample.cover.is_finite() && (0.0..=1.0).contains(&sample.cover));
                    }
                }
            }
        }
    }
}
