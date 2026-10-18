use std::sync::OnceLock;
use std::thread;

use glam::{Affine2, Vec2, Vec3};

use crate::color::value_and_hue;

// A terminal character is approximately half as wide as it is tall.
pub const CELL_ASPECT: f32 = 0.5;

// Below this many samples, starting threads costs more than it saves.
const PARALLEL_THRESHOLD: usize = 4096;

/// The pixel lattice a scene is sampled on. The y axis runs from -1 to 1
/// across the height, and `aspect` is the width of one pixel divided by its
/// height, so equal distances cover equal physical lengths on screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    pub width: usize,
    pub height: usize,
    pub aspect: f32,
}

impl Grid {
    pub fn new(width: usize, height: usize, aspect: f32) -> Self {
        Self {
            width,
            height,
            aspect,
        }
    }

    /// One pixel per terminal cell.
    #[cfg(test)]
    pub fn cells(columns: usize, rows: usize) -> Self {
        Self::new(columns, rows, CELL_ASPECT)
    }

    /// Scene units covered by one pixel row.
    pub fn scale(&self) -> f32 {
        2.0 / self.height.max(1) as f32
    }

    /// Half the visible width in scene units.
    pub fn half_width(&self) -> f32 {
        self.width as f32 * 0.5 * self.scale() * self.aspect
    }

    /// Maps continuous pixel coordinates, where pixel centers sit at
    /// `index + 0.5`, to scene space.
    pub fn point(&self, pixel: Vec2) -> Vec2 {
        let scale = self.scale();
        Vec2::new(
            (pixel.x - self.width as f32 * 0.5) * scale * self.aspect,
            (self.height as f32 * 0.5 - pixel.y) * scale,
        )
    }

    /// The scene-to-pixel transform, the inverse of `point`.
    pub fn to_pixels(self) -> Affine2 {
        let scale = self.scale();
        Affine2::from_translation(Vec2::new(self.width as f32 * 0.5, self.height as f32 * 0.5))
            * Affine2::from_scale(Vec2::new(1.0 / (scale * self.aspect), -1.0 / scale))
    }

    /// A finer grid with about `samples` square-ish subpixels across each
    /// pixel, for renderers that anti-alias by averaging. Returns the grid and
    /// the subpixel counts per pixel horizontally and vertically.
    pub fn refine(self, samples: usize) -> (Self, usize, usize) {
        let across = samples.max(1);
        let down = ((across as f32 / self.aspect).round() as usize).max(1);
        let grid = Self::new(
            self.width * across,
            self.height * down,
            self.aspect * down as f32 / across as f32,
        );
        (grid, across, down)
    }
}

/// What a scene reports for one point: how much light reaches the eye, and
/// the hue of that light at full intensity. Glyph density shows brightness,
/// and the color shows the hue.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shade {
    pub brightness: f32,
    pub color: Vec3,
}

impl Shade {
    pub const EMPTY: Self = Self {
        brightness: 0.0,
        color: Vec3::ONE,
    };

    pub fn new(brightness: f32, color: Vec3) -> Self {
        Self { brightness, color }
    }

    #[cfg(test)]
    pub fn gray(brightness: f32) -> Self {
        Self::new(brightness, Vec3::ONE)
    }

    /// Splits a light value into its brightest channel and its hue.
    pub fn from_rgb(rgb: Vec3) -> Self {
        let (brightness, hue) = value_and_hue(rgb);
        Self::new(brightness, hue)
    }

    pub fn rgb(self) -> Vec3 {
        self.color * self.brightness
    }

    pub fn brighter(self, other: Self) -> Self {
        if other.brightness > self.brightness {
            other
        } else {
            self
        }
    }

    fn clamped(self) -> Self {
        Self::new(
            self.brightness.clamp(0.0, 1.0),
            self.color.clamp(Vec3::ZERO, Vec3::ONE),
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    width: usize,
    height: usize,
    shades: Vec<Shade>,
}

impl Frame {
    /// Builds a frame from a function of each pixel's column and row, spread
    /// across all cores for large frames.
    pub fn build(
        width: usize,
        height: usize,
        shade: impl Fn(usize, usize) -> Shade + Sync,
    ) -> Self {
        let mut shades = vec![Shade::EMPTY; width * height];
        if shades.is_empty() {
            return Self {
                width,
                height,
                shades,
            };
        }

        let threads = if shades.len() < PARALLEL_THRESHOLD {
            1
        } else {
            parallelism().min(height)
        };
        let rows_per_thread = height.div_ceil(threads);
        let shade = &shade;
        let fill = move |first_row: usize, chunk: &mut [Shade]| {
            for (index, value) in chunk.iter_mut().enumerate() {
                *value = shade(index % width, first_row + index / width).clamped();
            }
        };

        if threads == 1 {
            fill(0, &mut shades);
        } else {
            thread::scope(|scope| {
                for (chunk_index, chunk) in shades.chunks_mut(rows_per_thread * width).enumerate() {
                    scope.spawn(move || fill(chunk_index * rows_per_thread, chunk));
                }
            });
        }

        Self {
            width,
            height,
            shades,
        }
    }

    /// Samples a scene function at the center of every pixel of `grid`.
    pub fn sample(grid: Grid, sample: impl Fn(Vec2) -> Shade + Sync) -> Self {
        Self::build(grid.width, grid.height, |column, row| {
            sample(grid.point(Vec2::new(column as f32 + 0.5, row as f32 + 0.5)))
        })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn shade(&self, column: usize, row: usize) -> Shade {
        self.shades[row * self.width + column]
    }

    /// Averages each `across` x `down` block of pixels into one pixel. Colors
    /// are weighted by brightness so dark subpixels do not wash out hues.
    pub fn downsample(&self, across: usize, down: usize) -> Self {
        let samples = (across * down) as f32;
        Self::build(self.width / across, self.height / down, |column, row| {
            let mut brightness = 0.0;
            let mut light = Vec3::ZERO;
            for y in row * down..(row + 1) * down {
                for x in column * across..(column + 1) * across {
                    let shade = self.shade(x, y);
                    brightness += shade.brightness;
                    light += shade.rgb();
                }
            }
            if brightness <= 1e-6 {
                Shade::EMPTY
            } else {
                Shade::new(brightness / samples, light / brightness)
            }
        })
    }
}

fn parallelism() -> usize {
    static THREADS: OnceLock<usize> = OnceLock::new();
    *THREADS.get_or_init(|| thread::available_parallelism().map_or(1, |count| count.get()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disk(radius: f32) -> impl Fn(Vec2) -> Shade + Sync {
        move |point| Shade::gray(if point.length() <= radius { 1.0 } else { 0.0 })
    }

    #[test]
    fn brightness_and_color_are_clamped() {
        let dark = Frame::sample(Grid::cells(1, 1), |_| Shade::new(-3.0, Vec3::splat(-1.0)));
        assert_eq!(dark.shade(0, 0), Shade::new(0.0, Vec3::ZERO));
        let bright = Frame::sample(Grid::cells(1, 1), |_| Shade::new(4.0, Vec3::splat(2.0)));
        assert_eq!(bright.shade(0, 0), Shade::gray(1.0));
    }

    #[test]
    fn a_circle_uses_twice_as_many_columns_as_rows() {
        let frame = Frame::sample(Grid::cells(65, 33), disk(0.5));
        let horizontal = (0..65)
            .filter(|column| frame.shade(*column, 16).brightness > 0.0)
            .count();
        let vertical = (0..33)
            .filter(|row| frame.shade(32, *row).brightness > 0.0)
            .count();
        assert!(horizontal.abs_diff(vertical * 2) <= 1);
    }

    #[test]
    fn an_empty_view_has_no_pixels() {
        let frame = Frame::sample(Grid::cells(0, 0), |_| Shade::gray(1.0));
        assert_eq!((frame.width(), frame.height()), (0, 0));
    }

    #[test]
    fn to_pixels_inverts_point() {
        let grid = Grid::cells(37, 11);
        let pixel = Vec2::new(12.25, 3.75);
        let round_trip = grid.to_pixels().transform_point2(grid.point(pixel));
        assert!(round_trip.distance(pixel) < 1e-4);
    }

    #[test]
    fn refining_a_cell_grid_makes_square_subpixels() {
        let (fine, across, down) = Grid::cells(10, 5).refine(2);
        assert_eq!((across, down), (2, 4));
        assert_eq!((fine.width, fine.height), (20, 20));
        assert!((fine.aspect - 1.0).abs() < 1e-6);
    }

    #[test]
    fn parallel_sampling_matches_a_serial_loop() {
        let grid = Grid::cells(160, 60);
        let shape = disk(0.7);
        let frame = Frame::sample(grid, &shape);
        for row in 0..grid.height {
            for column in 0..grid.width {
                let point = grid.point(Vec2::new(column as f32 + 0.5, row as f32 + 0.5));
                assert_eq!(frame.shade(column, row), shape(point));
            }
        }
    }

    #[test]
    fn downsampling_weights_color_by_brightness() {
        let frame = Frame::build(2, 1, |column, _| {
            if column == 0 {
                Shade::new(1.0, Vec3::X)
            } else {
                Shade::new(0.0, Vec3::Y)
            }
        });
        let average = frame.downsample(2, 1).shade(0, 0);
        assert_eq!(average, Shade::new(0.5, Vec3::X));
    }
}
