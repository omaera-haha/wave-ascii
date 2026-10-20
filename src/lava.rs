use std::f32::consts::TAU;

use glam::{Mat2, Vec2, Vec3};

use crate::frame::{Frame, Grid, Shade};
use crate::scene::coverage;

// A lamp spans y from BASE_BOTTOM to CAP_TOP in its own units, centered on x = 0.
const BASE_BOTTOM: f32 = -0.95;
const GLASS_BOTTOM: f32 = -0.52;
const GLASS_TOP: f32 = 0.6;
const CAP_TOP: f32 = 0.92;
const SHELF_FRONT: f32 = BASE_BOTTOM - 0.04;
const SHELF_BOTTOM: f32 = BASE_BOTTOM - 0.11;
const SPACING: f32 = 0.95;
// The wax surface is where the metaball field reaches this level.
const THRESHOLD: f32 = 1.0;

/// One lamp's colors and the wax blobs inside it.
struct Lamp {
    liquid: Vec3,
    hot_wax: Vec3,
    cool_wax: Vec3,
    // Phase offsets keep the lamps from moving in step.
    phase: f32,
}

const LAMPS: [Lamp; 3] = [
    Lamp {
        liquid: Vec3::new(0.38, 0.22, 0.85),
        hot_wax: Vec3::new(1.0, 0.85, 0.3),
        cool_wax: Vec3::new(1.0, 0.25, 0.3),
        phase: 0.0,
    },
    Lamp {
        liquid: Vec3::new(0.1, 0.45, 0.8),
        hot_wax: Vec3::new(0.85, 1.0, 0.35),
        cool_wax: Vec3::new(0.2, 0.95, 0.55),
        phase: 2.1,
    },
    Lamp {
        liquid: Vec3::new(0.15, 0.55, 0.6),
        hot_wax: Vec3::new(1.0, 0.6, 0.9),
        cool_wax: Vec3::new(0.85, 0.2, 0.75),
        phase: 4.3,
    },
];

/// A blob's size and the period and sway of its trip up and down.
struct Blob {
    radius: f32,
    period: f32,
    offset: f32,
    sway: f32,
}

const BLOBS: [Blob; 6] = [
    Blob::new(0.075, 23.0, 0.0, 0.05),
    Blob::new(0.06, 17.0, 1.7, -0.06),
    Blob::new(0.09, 29.0, 3.1, 0.03),
    Blob::new(0.05, 13.0, 4.4, 0.07),
    Blob::new(0.065, 19.0, 5.6, -0.04),
    Blob::new(0.045, 11.0, 2.4, -0.07),
];

impl Blob {
    const fn new(radius: f32, period: f32, offset: f32, sway: f32) -> Self {
        Self {
            radius,
            period,
            offset,
            sway,
        }
    }
}

/// Three lava lamps on a shelf, their wax rising, merging, and sinking. `yaw`
/// tilts the shelf and `zoom` scales it.
pub fn render(grid: Grid, time: f32, yaw: f32, zoom: f32) -> Frame {
    // Fit the shelf to narrow views, leaving a margin above and below.
    let fit = (grid.half_width() / (SPACING + 0.45)).min(1.0) * 0.94 * zoom;
    let turn = Mat2::from_angle(yaw);
    let edge = 1.2 / (grid.height.max(1) as f32 * fit);
    let wax: Vec<[(Vec2, f32); 6]> = LAMPS
        .iter()
        .map(|lamp| blob_positions(lamp, time))
        .collect();

    Frame::sample(grid, |point| {
        let point = turn * point / fit - Vec2::new(0.0, 0.07);
        let index = ((point.x / SPACING).round() + 1.0).clamp(0.0, 2.0) as usize;
        let local = point - Vec2::new((index as f32 - 1.0) * SPACING, 0.0);
        let back = shelf(point, edge);
        shade(&LAMPS[index], &wax[index], local, edge, back)
    })
}

// Where each blob is: a slow heat cycle carries it between the bottom and top
// of the glass, with a sideways drift.
fn blob_positions(lamp: &Lamp, time: f32) -> [(Vec2, f32); 6] {
    BLOBS.map(|blob| {
        let cycle = (time / blob.period * TAU + blob.offset + lamp.phase).cos();
        // Blobs linger at the ends of their trip, where they melt into the pools.
        let height = GLASS_BOTTOM + 0.06 + (GLASS_TOP - GLASS_BOTTOM - 0.16) * (0.5 - 0.5 * cycle);
        let room = (glass_half_width(height) - blob.radius - 0.02).max(0.0);
        let drift = blob.sway * (time * 0.4 + blob.offset * 2.0 + lamp.phase).sin() / 0.07;
        (
            Vec2::new(drift.clamp(-1.0, 1.0) * room, height),
            blob.radius,
        )
    })
}

// The glass bulges a third of the way up and narrows toward the cap.
fn glass_half_width(y: f32) -> f32 {
    let along = ((y - GLASS_BOTTOM) / (GLASS_TOP - GLASS_BOTTOM)).clamp(0.0, 1.0);
    if along < 0.3 {
        0.15 + 0.1 * (along / 0.3 * std::f32::consts::FRAC_PI_2).sin()
    } else {
        0.25 - 0.14 * ((along - 0.3) / 0.7)
    }
}

// The wax field: blobs plus a pool of melted wax at the bottom.
fn field(blobs: &[(Vec2, f32); 6], point: Vec2) -> f32 {
    let pool = 0.0035 / ((point.y - GLASS_BOTTOM + 0.02).powi(2) + 1e-4);
    blobs.iter().fold(pool, |total, (center, radius)| {
        total + radius * radius / (point - *center).length_squared().max(1e-5)
    })
}

fn shade(lamp: &Lamp, blobs: &[(Vec2, f32); 6], point: Vec2, edge: f32, back: Shade) -> Shade {
    let (x, y) = (point.x, point.y);
    if y < BASE_BOTTOM - edge || y > CAP_TOP + edge {
        return lit_shelf(lamp, point, back);
    }

    if y < GLASS_BOTTOM {
        // The base: a cone, widening toward the shelf.
        let along = (y - BASE_BOTTOM) / (GLASS_BOTTOM - BASE_BOTTOM);
        let half_width = 0.33 - 0.18 * along;
        let inside = coverage(x.abs() - half_width, edge) * coverage(BASE_BOTTOM - y, edge);
        let metal = metal(x / half_width);
        // Light from the bulb spills onto the top of the base.
        let spill = (1.0 - (GLASS_BOTTOM - y) / 0.1).max(0.0) * 0.3;
        let base = Shade::new(metal.brightness + spill, metal.color);
        return blend(lit_shelf(lamp, point, back), base, inside);
    }
    if y > GLASS_TOP {
        let along = (y - GLASS_TOP) / (CAP_TOP - GLASS_TOP);
        let half_width = 0.11 - 0.06 * along;
        let inside = coverage(x.abs() - half_width, edge) * coverage(y - CAP_TOP, edge);
        return blend(back, metal(x / half_width), inside);
    }

    let half_width = glass_half_width(y);
    let inside = coverage(x.abs() - half_width, edge);
    if inside <= 0.0 {
        return back;
    }
    let across = (x / half_width).clamp(-1.0, 1.0);
    // The liquid is lit from the bulb below and darkens toward the curved sides.
    let height = (y - GLASS_BOTTOM) / (GLASS_TOP - GLASS_BOTTOM);
    let rounding = (1.0 - across * across).max(0.0).powf(0.35);
    let mut color = lamp.liquid * (0.42 - 0.22 * height) * (0.4 + 0.6 * rounding);

    // For a lone blob, field^(-1/2) grows linearly with distance from its
    // center, so dividing by its slope gives a distance to the wax surface
    // that stays accurate deep inside blobs too.
    let value = field(blobs, point).max(1e-6);
    let slope = 0.5 * value.powf(-1.5) * gradient(blobs, point).length();
    let distance = (value.powf(-0.5) - THRESHOLD.powf(-0.5)) / slope.max(1e-4);
    let wax = coverage(distance, edge);
    if wax > 0.0 {
        // A fake bulge: brighter toward the middle of each blob.
        let depth = (1.0 - THRESHOLD / value.max(THRESHOLD)).sqrt();
        let hue = lamp
            .cool_wax
            .lerp(lamp.hot_wax, (1.0 - height * 1.2).clamp(0.0, 1.0));
        let lit = hue * (0.6 + 0.4 * depth) * (0.75 + 0.25 * rounding);
        color = color.lerp(lit, wax);
    }

    // A streak of reflected light down the left of the glass.
    let streak = (-((across + 0.55) / 0.12).powi(2)).exp() * 0.35;
    color += Vec3::splat(streak);
    blend(back, Shade::from_rgb(color), inside)
}

// A wooden shelf across the whole view.
fn shelf(point: Vec2, edge: f32) -> Shade {
    if point.y > BASE_BOTTOM + edge || point.y < SHELF_BOTTOM - edge {
        return Shade::EMPTY;
    }
    let grain = 0.85 + 0.15 * (point.x * 23.0 + 2.0 * (point.x * 3.7).sin()).sin();
    let wood = Vec3::new(0.62, 0.38, 0.2) * grain;
    // The top is lit from above; the front edge is in shadow.
    let color = if point.y > SHELF_FRONT {
        wood * 0.9
    } else {
        wood * 0.55
    };
    let inside = coverage(point.y - BASE_BOTTOM, edge) * coverage(SHELF_BOTTOM - point.y, edge);
    Shade::new(color.max_element() * inside, color / color.max_element())
}

// The shelf top catches each lamp's colored light.
fn lit_shelf(lamp: &Lamp, point: Vec2, back: Shade) -> Shade {
    if back.brightness <= 0.0 || point.y < SHELF_FRONT {
        return back;
    }
    let spill = 0.5 * (-(point.x / 0.4).powi(2)).exp();
    Shade::from_rgb(back.rgb() + lamp.liquid.lerp(lamp.hot_wax, 0.5) * spill)
}

fn gradient(blobs: &[(Vec2, f32); 6], point: Vec2) -> Vec2 {
    let step = 0.004;
    Vec2::new(
        field(blobs, point + Vec2::X * step) - field(blobs, point - Vec2::X * step),
        field(blobs, point + Vec2::Y * step) - field(blobs, point - Vec2::Y * step),
    ) / (2.0 * step)
}

// Polished metal reflects a bright band on the left and a dim one on the right.
fn metal(across: f32) -> Shade {
    let across = across.clamp(-1.0, 1.0);
    let band = 0.25
        + 0.6 * (-((across + 0.45) / 0.25).powi(2)).exp()
        + 0.2 * (-((across - 0.55) / 0.2).powi(2)).exp();
    Shade::new(
        band * (0.6 + 0.4 * (1.0 - across * across).sqrt()),
        Vec3::new(0.75, 0.72, 0.8),
    )
}

fn blend(back: Shade, front: Shade, amount: f32) -> Shade {
    if amount >= 1.0 {
        return front;
    }
    Shade::from_rgb(back.rgb().lerp(front.rgb(), amount))
}
