use std::io;
use std::path::Path;

use glam::{Mat2, Quat, Vec2, Vec3};

use crate::color::hsv;
use crate::drawing::{self, Drawing};
use crate::frame::{Frame, Grid, Shade};
use crate::mesh::Mesh;
use crate::{aurora, ghost, lava, model, ocean};

/// A drawing or model compiled into the program, so it works without the
/// asset file.
struct Preset {
    name: &'static str,
    label: &'static str,
    data: &'static [u8],
}

// Reads `assets/<name>.<extension>` at compile time.
macro_rules! preset {
    ($name:literal, $label:literal, $extension:literal) => {
        Preset {
            name: $name,
            label: $label,
            data: include_bytes!(concat!("../assets/", $name, ".", $extension)),
        }
    };
}

// Command-line names of the scenes made in code, in `builtins` order.
const GENERATED: [&str; 7] = ["2d", "3d", "ocean", "knot", "lava", "aurora", "ghost"];

const DRAWINGS: &[Preset] = &[
    preset!("wave", "Wave logo", "svg"),
    preset!("koi", "Koi pond", "svg"),
    preset!("fuji", "Mount Fuji", "svg"),
    preset!("synthwave", "Synthwave", "svg"),
    preset!("butterfly", "Butterfly", "svg"),
    preset!("fox", "Fox", "svg"),
    preset!("macaw", "Macaw", "svg"),
    preset!("mandala", "Mandala", "svg"),
    preset!("invaders", "Invaders", "svg"),
];

// Drawings that animate themselves with SVG animation.
const ANIMATIONS: &[Preset] = &[
    preset!("orbits", "Orbits", "svg"),
    preset!("dunes", "Dunes", "svg"),
    preset!("bauhaus", "Bauhaus", "svg"),
    preset!("ribbons", "Ribbons", "svg"),
    preset!("pendulums", "Pendulum wave", "svg"),
    preset!("infinity", "Infinity", "svg"),
    preset!("breathe", "Breathe", "svg"),
];

const MODELS: &[Preset] = &[
    preset!("duck", "Duck", "glb"),
    preset!("lighthouse", "Lighthouse", "glb"),
    preset!("rocket", "Rocket", "glb"),
    preset!("planet", "Ringed planet", "glb"),
    preset!("balloon", "Hot-air balloon", "glb"),
    preset!("ufo", "UFO", "glb"),
    preset!("bonsai", "Bonsai", "glb"),
    preset!("mushrooms", "Mushrooms", "glb"),
    preset!("crystals", "Crystals", "glb"),
    preset!("donut", "Donut", "glb"),
    preset!("chess", "Chess", "glb"),
];

/// The viewer's adjustments: turning with the arrow keys and zooming.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
}

impl Default for View {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            zoom: 1.0,
        }
    }
}

impl View {
    fn orbit(self) -> Quat {
        Quat::from_rotation_x(self.pitch) * Quat::from_rotation_y(self.yaw)
    }
}

pub enum Scene {
    Shapes,
    Cube,
    Ocean,
    Knot(Mesh),
    Lava,
    Aurora,
    Ghost,
    Model {
        label: String,
        mesh: Mesh,
    },
    Drawing {
        label: String,
        drawing: Box<Drawing>,
    },
}

impl Scene {
    /// Command-line names of the built-in scenes under their menu groups, in
    /// `builtins` order.
    pub fn groups() -> [(&'static str, Vec<&'static str>); 4] {
        let names = |presets: &[Preset]| presets.iter().map(|preset| preset.name).collect();
        [
            ("Generated", GENERATED.to_vec()),
            ("Drawings", names(DRAWINGS)),
            ("Animations", names(ANIMATIONS)),
            ("Models", names(MODELS)),
        ]
    }

    /// Command-line names of the built-in scenes, in `builtins` order.
    pub fn names() -> impl Iterator<Item = &'static str> {
        Self::groups().into_iter().flat_map(|(_, names)| names)
    }

    pub fn builtins() -> Vec<Self> {
        let mut scenes = vec![
            Self::Shapes,
            Self::Cube,
            Self::Ocean,
            Self::Knot(Mesh::torus_knot(2, 3)),
            Self::Lava,
            Self::Aurora,
            Self::Ghost,
        ];
        scenes.extend(
            DRAWINGS
                .iter()
                .chain(ANIMATIONS)
                .map(|preset| Self::Drawing {
                    label: preset.label.to_owned(),
                    drawing: Box::new(Drawing::parse(preset.data, None).unwrap_or_else(|error| {
                        panic!("the bundled {}.svg is invalid: {error}", preset.name)
                    })),
                }),
        );
        scenes.extend(MODELS.iter().map(|preset| Self::Model {
            label: preset.label.to_owned(),
            mesh: model::parse_gltf(preset.data).unwrap_or_else(|error| {
                panic!("the bundled {}.glb is invalid: {error}", preset.name)
            }),
        }));
        scenes
    }

    /// Whether `path` names a file `load` understands.
    pub fn is_loadable(path: &Path) -> bool {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                let extension = extension.to_ascii_lowercase();
                drawing::EXTENSIONS
                    .iter()
                    .chain(model::EXTENSIONS)
                    .any(|known| *known == extension)
            })
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let label = path.file_name().map_or_else(
            || path.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        let context =
            |error: io::Error| io::Error::new(error.kind(), format!("{}: {error}", path.display()));
        let is_drawing = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                drawing::EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
            });
        Ok(if is_drawing {
            Self::Drawing {
                label,
                drawing: Box::new(Drawing::load(path).map_err(context)?),
            }
        } else {
            Self::Model {
                label,
                mesh: model::load(path).map_err(context)?,
            }
        })
    }

    /// The heading the scene is listed under in the menu.
    pub fn group(&self) -> &'static str {
        match self {
            Self::Shapes
            | Self::Cube
            | Self::Ocean
            | Self::Knot(_)
            | Self::Lava
            | Self::Aurora
            | Self::Ghost => "Generated",
            Self::Drawing { drawing, .. } if drawing.is_animated() => "Animations",
            Self::Drawing { .. } => "Drawings",
            Self::Model { .. } => "Models",
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Shapes => "2D shapes",
            Self::Cube => "3D cube",
            Self::Ocean => "Ocean",
            Self::Knot(_) => "Torus knot",
            Self::Lava => "Lava lamps",
            Self::Aurora => "Aurora",
            Self::Ghost => "Ghost",
            Self::Model { label, .. } | Self::Drawing { label, .. } => label,
        }
    }

    pub fn render(&self, grid: Grid, time: f32, view: View) -> Frame {
        match self {
            Self::Shapes => {
                let turn = Mat2::from_angle(view.yaw);
                let rotation = Mat2::from_angle(-0.25 - time * 0.7);
                let center = Vec2::new(0.65 + 0.15 * time.sin(), 0.2 * (time * 0.8).sin());
                let radius = 0.32 + 0.08 * (time * 1.5).sin();
                let edge = 1.0 / (grid.height.max(1) as f32 * view.zoom);
                let circle_color = hsv(0.52 + 0.06 * (time * 0.6).sin(), 0.65, 1.0);

                Frame::sample(grid, |point| {
                    let point = turn * point / view.zoom;
                    let local = rotation * (point - Vec2::new(-0.65, 0.0));
                    let delta = local.abs() - Vec2::splat(0.38);
                    let square_distance =
                        delta.max(Vec2::ZERO).length() + delta.max_element().min(0.0);
                    let height = (local.y / 0.76 + 0.5).clamp(0.0, 1.0);
                    let square = Shade::new(
                        coverage(square_distance, edge) * (0.35 + 0.55 * height),
                        Vec3::new(1.0, 0.55, 0.2).lerp(Vec3::new(0.95, 0.3, 0.8), height),
                    );

                    let distance = point.distance(center);
                    let circle = Shade::new(
                        coverage(distance - radius, edge)
                            * (0.25 + 0.75 * (1.0 - (distance / radius).powi(2)).max(0.0).sqrt()),
                        circle_color,
                    );
                    square.brighter(circle)
                })
            }
            Self::Cube => {
                let rotation = view.orbit()
                    * Quat::from_rotation_y(0.6 + time * 0.7)
                    * Quat::from_rotation_x(0.4 + time * 0.45);
                let inverse = rotation.conjugate();
                let origin = inverse * Vec3::new(0.0, 0.0, 3.5);
                let light = Vec3::new(-0.5, 0.8, 1.0).normalize();

                Frame::sample(grid, |point| {
                    let direction =
                        inverse * Vec3::new(point.x, point.y, -2.4 * view.zoom).normalize();
                    cube_hit(origin, direction).map_or(Shade::EMPTY, |(_, normal)| {
                        Shade::new(
                            0.18 + 0.82 * (rotation * normal).dot(light).max(0.0),
                            face_color(normal),
                        )
                    })
                })
            }
            Self::Ocean => ocean::render(grid, time, view.yaw, view.pitch, view.zoom),
            Self::Lava => lava::render(grid, time, view.yaw, view.zoom),
            Self::Aurora => aurora::render(grid, time, view.yaw, view.pitch, view.zoom),
            Self::Ghost => ghost::render(grid, time, view.yaw, view.zoom),
            Self::Knot(mesh) => {
                let rotation = view.orbit()
                    * Quat::from_rotation_y(time * 0.5)
                    * Quat::from_rotation_x(0.5 + time * 0.31);
                mesh.render(grid, rotation, view.zoom)
            }
            // A turntable, tilted a little toward the viewer.
            Self::Model { mesh, .. } => {
                let rotation =
                    view.orbit() * Quat::from_rotation_x(0.3) * Quat::from_rotation_y(time * 0.6);
                mesh.render(grid, rotation, view.zoom)
            }
            Self::Drawing { drawing, .. } => drawing.render(grid, time, -view.yaw, view.zoom),
        }
    }
}

pub fn coverage(distance: f32, edge: f32) -> f32 {
    (0.5 - distance / (2.0 * edge)).clamp(0.0, 1.0)
}

// Opposite faces share a hue family so the cube reads as one object.
fn face_color(normal: Vec3) -> Vec3 {
    const COLORS: [[Vec3; 2]; 3] = [
        [Vec3::new(1.0, 0.35, 0.4), Vec3::new(0.95, 0.5, 0.25)],
        [Vec3::new(1.0, 0.85, 0.3), Vec3::new(0.55, 0.9, 0.35)],
        [Vec3::new(0.3, 0.7, 1.0), Vec3::new(0.65, 0.45, 1.0)],
    ];
    let axis = normal.abs().max_position();
    COLORS[axis][usize::from(normal[axis] > 0.0)]
}

// Intersect the ray with each face and keep the closest visible surface.
fn cube_hit(origin: Vec3, direction: Vec3) -> Option<(f32, Vec3)> {
    const HALF_SIZE: f32 = 0.7;
    let mut closest = f32::INFINITY;
    let mut normal = Vec3::ZERO;

    for (axis, unit) in [Vec3::X, Vec3::Y, Vec3::Z].into_iter().enumerate() {
        if direction[axis].abs() < 1e-6 {
            continue;
        }
        for side in [-1.0, 1.0] {
            let distance = (side * HALF_SIZE - origin[axis]) / direction[axis];
            let point = origin + direction * distance;
            if distance > 0.0 && distance < closest && point.abs().max_element() <= HALF_SIZE + 1e-5
            {
                closest = distance;
                normal = unit * side;
            }
        }
    }

    closest.is_finite().then_some((closest, normal))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Style;
    use crate::color::ColorMode;

    #[test]
    fn a_ray_hits_the_front_face_at_the_expected_distance() {
        let (distance, normal) = cube_hit(Vec3::new(0.0, 0.0, 3.0), -Vec3::Z).unwrap();
        assert!((distance - 2.3).abs() < 1e-5);
        assert_eq!(normal, Vec3::Z);
    }

    #[test]
    fn a_parallel_ray_outside_the_cube_misses() {
        assert!(cube_hit(Vec3::new(3.0, 0.0, 3.0), -Vec3::Z).is_none());
    }

    #[test]
    fn every_face_of_the_cube_has_its_own_color() {
        let normals = [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z];
        for (index, normal) in normals.iter().enumerate() {
            for other in &normals[index + 1..] {
                assert_ne!(face_color(*normal), face_color(*other));
            }
        }
    }

    #[test]
    fn every_builtin_scene_produces_visible_animation() {
        for scene in Scene::builtins() {
            let first = scene.render(Grid::cells(80, 24), 0.0, View::default());
            let later = scene.render(Grid::cells(80, 24), 1.0, View::default());
            assert!(
                Style::Ascii
                    .canvas(&first, ColorMode::Plain)
                    .text()
                    .chars()
                    .any(|character| !character.is_whitespace()),
                "{} is blank",
                scene.label()
            );
            assert_ne!(first, later, "{} is still", scene.label());
        }
    }

    #[test]
    fn scene_names_match_the_builtins() {
        let builtins = Scene::builtins();
        assert_eq!(Scene::names().count(), builtins.len());
        for ((group, names), scenes) in Scene::groups()
            .iter()
            .zip(builtins.chunk_by(|scene, next| scene.group() == next.group()))
        {
            assert_eq!(names.len(), scenes.len(), "{group}");
            assert!(
                scenes.iter().all(|scene| scene.group() == *group),
                "{group}"
            );
        }
    }

    #[test]
    fn only_known_file_types_are_loadable() {
        for name in [
            "logo.svg",
            "LOGO.SVGZ",
            "ship.glb",
            "a.gltf",
            "b.obj",
            "part.STL",
        ] {
            assert!(Scene::is_loadable(Path::new(name)), "{name}");
        }
        for name in ["notes.txt", "2d", "model"] {
            assert!(!Scene::is_loadable(Path::new(name)), "{name}");
        }
    }
}
