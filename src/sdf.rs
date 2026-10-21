use std::f32::consts::PI;

use glam::Vec2;

/// Signed distance to a thin four-pointed star centered on the origin, with
/// its points `radius` away along the axes.
pub fn star(point: Vec2, radius: f32) -> f32 {
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
pub fn chevron(point: Vec2, tip: Vec2, ends: [Vec2; 2], radius: f32) -> f32 {
    capsule(point, tip, ends[0], radius).min(capsule(point, tip, ends[1], radius))
}

pub fn capsule(point: Vec2, start: Vec2, end: Vec2, radius: f32) -> f32 {
    let along = end - start;
    let offset = point - start;
    let fraction = (offset.dot(along) / along.length_squared()).clamp(0.0, 1.0);
    (offset - along * fraction).length() - radius
}

/// Signed distance to a trapezoid centered on the origin and symmetric about
/// the y axis, with half widths `bottom` and `top` at its bottom and top
/// edges, `half_height` from the center.
pub fn trapezoid(point: Vec2, bottom: f32, top: f32, half_height: f32) -> f32 {
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

pub fn rounded_box(point: Vec2, half_size: Vec2, radius: f32) -> f32 {
    let outside = point.abs() - half_size + Vec2::splat(radius);
    outside.max(Vec2::ZERO).length() + outside.max_element().min(0.0) - radius
}

// Exactly `from` at 0 and `to` at 1.
pub fn mix(from: f32, to: f32, amount: f32) -> f32 {
    from * (1.0 - amount) + to * amount
}

/// The union of two shapes with the seam between them rounded over `width`.
pub fn smooth_min(a: f32, b: f32, width: f32) -> f32 {
    let blend = (0.5 + 0.5 * (b - a) / width).clamp(0.0, 1.0);
    b + (a - b) * blend - width * blend * (1.0 - blend)
}

pub fn smoothstep(start: f32, end: f32, value: f32) -> f32 {
    let x = ((value - start) / (end - start)).clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// The share of a pixel that a shape covers, from its signed `distance` and
/// the pixel's `edge`, about its width.
pub fn coverage(distance: f32, edge: f32) -> f32 {
    (0.5 - distance / (2.0 * edge)).clamp(0.0, 1.0)
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
