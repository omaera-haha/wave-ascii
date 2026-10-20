use std::fs;
use std::io;
use std::path::Path;

use glam::{Affine2, Vec2, Vec3, Vec4};
use resvg::{tiny_skia, usvg};

use crate::animation::Animation;
use crate::color::value_and_hue;
use crate::frame::{Frame, Grid, Shade};

pub const EXTENSIONS: &[&str] = &["svg", "svgz"];

// Ink darker than this is lifted so black artwork stays visible on dark
// terminal backgrounds.
const MINIMUM_VALUE: f32 = 0.45;
// A highlight sweeps across a still drawing once per period.
const SHEEN_PERIOD: f32 = 5.0;
const SHEEN_DURATION: f32 = 1.4;

/// Vector artwork from any SVG editor, rendered with resvg.
pub struct Drawing {
    /// The artwork, or its first moment if it is animated.
    tree: usvg::Tree,
    center: Vec2,
    size: Vec2,
    animation: Option<Animated>,
}

/// SVG animation and the options to read each moment of it with.
struct Animated {
    animation: Animation,
    options: usvg::Options<'static>,
}

impl Drawing {
    pub fn load(path: &Path) -> io::Result<Self> {
        Self::parse(&fs::read(path)?, path.parent())
    }

    /// Parses SVG or SVGZ data. Images it links to are found relative to
    /// `resources`.
    pub fn parse(data: &[u8], resources: Option<&Path>) -> io::Result<Self> {
        let invalid =
            |error: &dyn ToString| io::Error::new(io::ErrorKind::InvalidData, error.to_string());
        let decompressed;
        let data = if data.starts_with(&[0x1f, 0x8b]) {
            decompressed = usvg::decompress_svgz(data).map_err(|error| invalid(&error))?;
            &decompressed
        } else {
            data
        };
        let source = std::str::from_utf8(data).map_err(|error| invalid(&error))?;
        let mut options = usvg::Options {
            resources_dir: resources.map(Path::to_path_buf),
            ..usvg::Options::default()
        };
        // Scanning system fonts is slow, so do it only when text may appear.
        if source.contains("<text") {
            options.fontdb_mut().load_system_fonts();
        }
        let animation = Animation::parse(source).map_err(|error| invalid(&error))?;
        let tree = match &animation {
            Some(animation) => usvg::Tree::from_str(&animation.at(0.0), &options),
            None => usvg::Tree::from_str(source, &options),
        }
        .map_err(|error| invalid(&error))?;
        Ok(Self::new(
            tree,
            animation.map(|animation| Animated { animation, options }),
        ))
    }

    fn new(tree: usvg::Tree, animation: Option<Animated>) -> Self {
        // Frame the visible artwork rather than the declared canvas, since
        // terminal pixels are too scarce to spend on empty margins. Animated
        // artwork moves within its canvas, so frame the canvas.
        let bounds = tree.root().abs_layer_bounding_box();
        let (center, size) = if tree.root().has_children() && animation.is_none() {
            (
                Vec2::new(
                    bounds.x() + bounds.width() * 0.5,
                    bounds.y() + bounds.height() * 0.5,
                ),
                Vec2::new(bounds.width(), bounds.height()),
            )
        } else {
            let size = tree.size();
            (
                Vec2::new(size.width(), size.height()) * 0.5,
                Vec2::new(size.width(), size.height()),
            )
        };
        Self {
            tree,
            center,
            size,
            animation,
        }
    }

    /// Whether the SVG animates itself.
    pub fn is_animated(&self) -> bool {
        self.animation.is_some()
    }

    /// Draws the artwork centered and fitted to the view, turned by `angle`
    /// and scaled by `zoom`. A still drawing sways gently and catches a
    /// passing highlight; an animated one plays at `time` within its canvas.
    pub fn render(&self, grid: Grid, time: f32, angle: f32, zoom: f32) -> Frame {
        let (fine, across, down) = grid.refine(3);
        let Some(mut pixmap) = tiny_skia::Pixmap::new(fine.width as u32, fine.height as u32) else {
            return Frame::build(grid.width, grid.height, |_, _| Shade::EMPTY);
        };

        let moment = self.animation.as_ref().and_then(|animated| {
            usvg::Tree::from_str(&animated.animation.at(time), &animated.options).ok()
        });
        let still = self.animation.is_none();
        let (sway, breath) = if still {
            (0.08 * (time * 0.8).sin(), 0.03 * (time * 1.1).sin())
        } else {
            (0.0, 0.0)
        };
        let fit = 0.9 * (2.0 / self.size.y).min(2.0 * grid.half_width() / self.size.x);
        let scale = fit * zoom * (1.0 + breath);
        // SVG's y axis points down; scene space's points up.
        let placement = Affine2::from_angle(angle + sway)
            * Affine2::from_scale(Vec2::new(scale, -scale))
            * Affine2::from_translation(-self.center);
        let [sx, ky, kx, sy, tx, ty] = (fine.to_pixels() * placement).to_cols_array();
        let transform = tiny_skia::Transform::from_row(sx, ky, kx, sy, tx, ty);
        resvg::render(
            moment.as_ref().unwrap_or(&self.tree),
            transform,
            &mut pixmap.as_mut(),
        );
        if !still {
            // As in a browser, nothing shows outside the canvas.
            let canvas = self.tree.size();
            let mut mask = tiny_skia::Mask::new(pixmap.width(), pixmap.height())
                .expect("the mask is the pixmap's size");
            if let Some(rect) =
                tiny_skia::Rect::from_xywh(0.0, 0.0, canvas.width(), canvas.height())
            {
                mask.fill_path(
                    &tiny_skia::PathBuilder::from_rect(rect),
                    tiny_skia::FillRule::Winding,
                    true,
                    transform,
                );
            }
            pixmap.apply_mask(&mask);
        }

        let pixels = pixmap.data();
        let samples = (across * down) as f32;
        let sheen = if still {
            Sheen::at(time, grid.half_width())
        } else {
            Sheen { position: None }
        };
        Frame::build(grid.width, grid.height, |column, row| {
            let mut sum = Vec4::ZERO;
            for y in row * down..(row + 1) * down {
                for x in column * across..(column + 1) * across {
                    let index = (y * fine.width + x) * 4;
                    sum += Vec4::from_array(
                        [0, 1, 2, 3].map(|channel| pixels[index + channel] as f32),
                    );
                }
            }
            if sum.w <= 0.0 {
                return Shade::EMPTY;
            }
            // Pixels are premultiplied, so this is the coverage-weighted color.
            let color = sum.truncate() / sum.w;
            let (value, hue) = value_and_hue(color);
            let coverage = sum.w / (255.0 * samples);
            let point = grid.point(Vec2::new(column as f32 + 0.5, row as f32 + 0.5));
            let highlight = sheen.strength(point);
            Shade::new(
                coverage * (value.max(MINIMUM_VALUE) + 0.45 * highlight),
                hue.lerp(Vec3::ONE, 0.6 * highlight),
            )
        })
    }
}

/// A soft diagonal band of light that crosses the view.
struct Sheen {
    position: Option<f32>,
}

impl Sheen {
    const DIRECTION: Vec2 = Vec2::new(0.6, 0.8);
    const WIDTH: f32 = 0.18;

    fn at(time: f32, half_width: f32) -> Self {
        let progress = time.rem_euclid(SHEEN_PERIOD) / SHEEN_DURATION;
        let reach = half_width * Self::DIRECTION.x + Self::DIRECTION.y + 3.0 * Self::WIDTH;
        Self {
            position: (time > 0.0 && progress < 1.0).then_some(reach * (2.0 * progress - 1.0)),
        }
    }

    fn strength(&self, point: Vec2) -> f32 {
        self.position.map_or(0.0, |position| {
            (-((point.dot(Self::DIRECTION) - position) / Self::WIDTH).powi(2)).exp()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQUARE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
        <rect x="20" y="20" width="60" height="60" fill="#ff0000"/>
        <rect x="0" y="0" width="10" height="10" fill="#000000"/>
    </svg>"##;

    fn square() -> Drawing {
        Drawing::new(
            usvg::Tree::from_str(SQUARE, &usvg::Options::default()).unwrap(),
            None,
        )
    }

    #[test]
    fn artwork_is_framed_by_its_visible_bounds() {
        let drawing = square();
        assert_eq!(drawing.center, Vec2::new(40.0, 40.0));
        assert_eq!(drawing.size, Vec2::new(80.0, 80.0));
    }

    #[test]
    fn animated_artwork_plays_within_its_canvas() {
        // The square slides right and stops just past the canvas's edge,
        // where the view still reaches.
        let drawing = Drawing::parse(
            br##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100">
                <rect x="0" y="40" width="6" height="20" fill="#ff0000">
                    <animate attributeName="x" values="0;203" dur="2s" fill="freeze"/>
                </rect>
            </svg>"##,
            None,
        )
        .unwrap();
        assert!(drawing.is_animated());
        assert_eq!(drawing.size, Vec2::new(200.0, 100.0));
        let visible = |time| {
            let frame = drawing.render(Grid::cells(80, 24), time, 0.0, 1.0);
            (0..24).any(|row| (0..80).any(|column| frame.shade(column, row).brightness > 0.0))
        };
        assert!(visible(1.0));
        assert!(!visible(3.0));
    }

    #[test]
    fn the_drawing_fills_the_center_with_its_color() {
        let frame = square().render(Grid::cells(80, 24), 0.0, 0.0, 1.0);
        let center = frame.shade(48, 13);
        assert!(center.brightness > 0.5);
        assert!(center.color.x > 0.9 && center.color.y < 0.1);
        assert_eq!(frame.shade(0, 23).brightness, 0.0);
    }

    #[test]
    fn black_ink_is_lifted_to_a_visible_gray() {
        let frame = square().render(Grid::cells(80, 24), 0.0, 0.0, 1.0);
        let corner = frame.shade(21, 2);
        assert!((corner.brightness - MINIMUM_VALUE).abs() < 1e-3);
        assert_eq!(corner.color, Vec3::ONE);
    }

    #[test]
    fn the_sheen_is_absent_between_sweeps() {
        assert_eq!(Sheen::at(0.0, 1.5).strength(Vec2::ZERO), 0.0);
        assert_eq!(
            Sheen::at(SHEEN_DURATION + 0.5, 1.5).strength(Vec2::ZERO),
            0.0
        );
        assert!(Sheen::at(SHEEN_DURATION * 0.5, 1.5).strength(Vec2::ZERO) > 0.9);
    }
}
