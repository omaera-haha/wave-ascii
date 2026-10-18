use std::collections::HashMap;
use std::f32::consts::TAU;

use glam::{Quat, Vec2, Vec3};

use crate::color::{hsv, value_and_hue};
use crate::frame::{Frame, Grid, Shade};

// The camera sits on the z axis looking toward the origin. From this distance
// a unit sphere spans 85% of the view height, so a spinning model never clips.
const CAMERA_DISTANCE: f32 = 3.0;
const FOCAL_LENGTH: f32 = 2.4;
const NEAR: f32 = 0.05;
// Corners whose faces meet at a sharper angle than this keep separate normals.
const CREASE_COSINE: f32 = 0.766;
// Terminal pixels are coarse, so large textures are averaged down at load.
const TEXTURE_LIMIT: usize = 512;
const NO_TRIANGLE: u32 = u32::MAX;
const DARKEST_ALBEDO: f32 = 0.3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub position: Vec3,
    /// Zero when the source file has no normal; see `fill_missing_normals`.
    pub normal: Vec3,
    pub uv: Vec2,
    pub color: Vec3,
}

impl Vertex {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            normal: Vec3::ZERO,
            uv: Vec2::ZERO,
            color: Vec3::ONE,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Triangle {
    pub corners: [u32; 3],
    /// Index into `Mesh::textures`.
    pub texture: Option<u32>,
}

/// An sRGB image sampled with wrapping texture coordinates. The v axis runs
/// down the image, as in glTF.
#[derive(Clone, Debug, PartialEq)]
pub struct Texture {
    width: usize,
    height: usize,
    texels: Vec<Vec3>,
}

impl Texture {
    pub fn new(width: usize, height: usize, texels: Vec<Vec3>) -> Self {
        assert_eq!(texels.len(), width * height);
        let mut texture = Self {
            width,
            height,
            texels,
        };
        while texture.width.max(texture.height) > TEXTURE_LIMIT
            && texture.width >= 2
            && texture.height >= 2
        {
            texture = texture.halved();
        }
        texture
    }

    fn halved(&self) -> Self {
        let (width, height) = (self.width / 2, self.height / 2);
        let texel = |x: usize, y: usize| self.texels[y * self.width + x];
        let texels = (0..width * height)
            .map(|index| {
                let (x, y) = (index % width * 2, index / width * 2);
                (texel(x, y) + texel(x + 1, y) + texel(x, y + 1) + texel(x + 1, y + 1)) * 0.25
            })
            .collect();
        Self {
            width,
            height,
            texels,
        }
    }

    fn sample(&self, uv: Vec2) -> Vec3 {
        let x = (uv.x.rem_euclid(1.0) * self.width as f32) as usize;
        let y = (uv.y.rem_euclid(1.0) * self.height as f32) as usize;
        self.texels[y.min(self.height - 1) * self.width + x.min(self.width - 1)]
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub triangles: Vec<Triangle>,
    pub textures: Vec<Texture>,
}

// A vertex after the model and camera transforms.
struct Projected {
    position: Vec3,
    normal: Vec3,
    pixel: Vec2,
    inverse_depth: f32,
}

// The closest triangle at a pixel and the perspective-correct weights of its
// first two corners.
#[derive(Clone, Copy)]
struct Hit {
    triangle: u32,
    weights: Vec2,
}

impl Mesh {
    pub fn face_normal(&self, triangle: &Triangle) -> Vec3 {
        let [a, b, c] = triangle
            .corners
            .map(|corner| self.vertices[corner as usize].position);
        (b - a).cross(c - a)
    }

    /// Gives every corner without a normal the area-weighted average of the
    /// faces around its position, skipping faces across a sharp crease. This
    /// smooths curved surfaces while keeping hard edges, such as a box's, flat.
    pub fn fill_missing_normals(&mut self) {
        let key = |position: Vec3| position.to_array().map(f32::to_bits);
        let faces: Vec<Vec3> = self
            .triangles
            .iter()
            .map(|triangle| self.face_normal(triangle))
            .collect();
        let mut around: HashMap<[u32; 3], Vec<usize>> = HashMap::new();
        for (index, triangle) in self.triangles.iter().enumerate() {
            for corner in triangle.corners {
                let vertex = &self.vertices[corner as usize];
                if vertex.normal == Vec3::ZERO {
                    around.entry(key(vertex.position)).or_default().push(index);
                }
            }
        }

        for index in 0..self.triangles.len() {
            let own = faces[index].normalize_or_zero();
            for slot in 0..3 {
                let corner = self.triangles[index].corners[slot] as usize;
                let vertex = self.vertices[corner];
                if vertex.normal != Vec3::ZERO {
                    continue;
                }
                let normal: Vec3 = around[&key(vertex.position)]
                    .iter()
                    .map(|neighbor| faces[*neighbor])
                    .filter(|face| face.normalize_or_zero().dot(own) >= CREASE_COSINE)
                    .sum();
                // Corners may differ across a crease, so each gets its own vertex.
                self.triangles[index].corners[slot] = self.vertices.len() as u32;
                self.vertices.push(Vertex {
                    normal: normal.try_normalize().unwrap_or(own),
                    ..vertex
                });
            }
        }
        self.drop_unused_vertices();
    }

    fn drop_unused_vertices(&mut self) {
        let mut remap = vec![NO_TRIANGLE; self.vertices.len()];
        let mut vertices = Vec::new();
        for triangle in &mut self.triangles {
            for corner in &mut triangle.corners {
                if remap[*corner as usize] == NO_TRIANGLE {
                    remap[*corner as usize] = vertices.len() as u32;
                    vertices.push(self.vertices[*corner as usize]);
                }
                *corner = remap[*corner as usize];
            }
        }
        self.vertices = vertices;
    }

    /// Centers the mesh on the origin and scales it to fit a unit sphere.
    pub fn normalize(&mut self) {
        let Some(first) = self.vertices.first() else {
            return;
        };
        let (min, max) = self
            .vertices
            .iter()
            .fold((first.position, first.position), |(min, max), vertex| {
                (min.min(vertex.position), max.max(vertex.position))
            });
        let center = (min + max) * 0.5;
        let radius = self
            .vertices
            .iter()
            .map(|vertex| vertex.position.distance(center))
            .fold(0.0, f32::max);
        let scale = if radius > 0.0 { 1.0 / radius } else { 1.0 };
        for vertex in &mut self.vertices {
            vertex.position = (vertex.position - center) * scale;
        }
    }

    /// A tube wound `windings` times around a torus while circling it `turns`
    /// times, colored with a rainbow along its length.
    pub fn torus_knot(windings: u32, turns: u32) -> Self {
        const SEGMENTS: u32 = 360;
        const SIDES: u32 = 18;
        const TUBE_RADIUS: f32 = 0.55;
        let curve = |t: f32| {
            let ring = 2.0 + (turns as f32 * t).cos();
            Vec3::new(
                ring * (windings as f32 * t).cos(),
                ring * (windings as f32 * t).sin(),
                (turns as f32 * t).sin(),
            )
        };

        let mut mesh = Self::default();
        for segment in 0..SEGMENTS {
            let t = segment as f32 / SEGMENTS as f32 * TAU;
            let step = 0.01;
            let (before, center, after) = (curve(t - step), curve(t), curve(t + step));
            let tangent = (after - before).normalize();
            let bend = after + before - 2.0 * center;
            let inward = (bend - tangent * bend.dot(tangent)).normalize();
            let binormal = tangent.cross(inward);
            for side in 0..SIDES {
                let angle = side as f32 / SIDES as f32 * TAU;
                let normal = inward * angle.cos() + binormal * angle.sin();
                mesh.vertices.push(Vertex {
                    position: center + normal * TUBE_RADIUS,
                    normal,
                    uv: Vec2::ZERO,
                    color: hsv(segment as f32 / SEGMENTS as f32, 0.7, 1.0),
                });
            }
        }
        let index = |segment: u32, side: u32| (segment % SEGMENTS) * SIDES + side % SIDES;
        for segment in 0..SEGMENTS {
            for side in 0..SIDES {
                let quad = [
                    index(segment, side),
                    index(segment + 1, side),
                    index(segment + 1, side + 1),
                    index(segment, side + 1),
                ];
                for corners in [[quad[0], quad[1], quad[2]], [quad[0], quad[2], quad[3]]] {
                    mesh.triangles.push(Triangle {
                        corners,
                        texture: None,
                    });
                }
            }
        }
        mesh.normalize();
        mesh
    }

    /// Rasterizes the mesh with a depth buffer at twice the grid's resolution
    /// in each direction, shades each visible pixel once, and averages the
    /// result down to the grid.
    pub fn render(&self, grid: Grid, rotation: Quat, zoom: f32) -> Frame {
        let (fine, across, down) = grid.refine(2);
        let to_pixels = fine.to_pixels();
        let focal_length = FOCAL_LENGTH * zoom;
        let projected: Vec<Projected> = self
            .vertices
            .iter()
            .map(|vertex| {
                let position = rotation * vertex.position;
                let depth = CAMERA_DISTANCE - position.z;
                Projected {
                    position,
                    normal: rotation * vertex.normal,
                    pixel: to_pixels.transform_point2(position.truncate() * (focal_length / depth)),
                    inverse_depth: if depth > NEAR { 1.0 / depth } else { 0.0 },
                }
            })
            .collect();

        let (width, height) = (fine.width, fine.height);
        let mut nearest = vec![0.0_f32; width * height];
        let mut hits = vec![
            Hit {
                triangle: NO_TRIANGLE,
                weights: Vec2::ZERO,
            };
            width * height
        ];
        for (index, triangle) in self.triangles.iter().enumerate() {
            let [a, b, c] = triangle.corners.map(|corner| &projected[corner as usize]);
            let area = edge(a.pixel, b.pixel, c.pixel);
            if a.inverse_depth == 0.0
                || b.inverse_depth == 0.0
                || c.inverse_depth == 0.0
                || area.abs() < 1e-9
            {
                continue;
            }
            let low = (a.pixel.min(b.pixel).min(c.pixel) - 0.5)
                .ceil()
                .max(Vec2::ZERO);
            let high = (a.pixel.max(b.pixel).max(c.pixel) - 0.5)
                .floor()
                .min(Vec2::new(width as f32 - 1.0, height as f32 - 1.0));
            if low.x > high.x || low.y > high.y {
                continue;
            }
            for y in low.y as usize..=high.y as usize {
                for x in low.x as usize..=high.x as usize {
                    let pixel = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                    let weight_a = edge(b.pixel, c.pixel, pixel) / area;
                    let weight_b = edge(c.pixel, a.pixel, pixel) / area;
                    let weight_c = 1.0 - weight_a - weight_b;
                    if weight_a < 0.0 || weight_b < 0.0 || weight_c < 0.0 {
                        continue;
                    }
                    let inverse_depth = weight_a * a.inverse_depth
                        + weight_b * b.inverse_depth
                        + weight_c * c.inverse_depth;
                    let slot = y * width + x;
                    if inverse_depth > nearest[slot] {
                        nearest[slot] = inverse_depth;
                        hits[slot] = Hit {
                            triangle: index as u32,
                            weights: Vec2::new(
                                weight_a * a.inverse_depth,
                                weight_b * b.inverse_depth,
                            ) / inverse_depth,
                        };
                    }
                }
            }
        }

        Frame::build(width, height, |column, row| {
            let hit = hits[row * width + column];
            if hit.triangle == NO_TRIANGLE {
                Shade::EMPTY
            } else {
                self.shade(hit, &projected)
            }
        })
        .downsample(across, down)
    }

    fn shade(&self, hit: Hit, projected: &[Projected]) -> Shade {
        const KEY: Vec3 = Vec3::new(-0.42, 0.67, 0.61);
        const FILL: Vec3 = Vec3::new(0.85, -0.3, 0.43);

        let triangle = self.triangles[hit.triangle as usize];
        let [a, b, c] = triangle.corners.map(|corner| corner as usize);
        let weights = Vec3::new(
            hit.weights.x,
            hit.weights.y,
            1.0 - hit.weights.x - hit.weights.y,
        );
        let mix = |value: fn(&Vertex) -> Vec3| {
            value(&self.vertices[a]) * weights.x
                + value(&self.vertices[b]) * weights.y
                + value(&self.vertices[c]) * weights.z
        };

        let position = projected[a].position * weights.x
            + projected[b].position * weights.y
            + projected[c].position * weights.z;
        let view = (Vec3::new(0.0, 0.0, CAMERA_DISTANCE) - position).normalize();
        let mut normal = (projected[a].normal * weights.x
            + projected[b].normal * weights.y
            + projected[c].normal * weights.z)
            .try_normalize()
            .unwrap_or(view);
        // Surfaces are double-sided: light whichever side faces the camera.
        if normal.dot(view) < 0.0 {
            normal = -normal;
        }

        let mut albedo = mix(|vertex| vertex.color);
        if let Some(texture) = triangle.texture {
            let uv = mix(|vertex| vertex.uv.extend(0.0)).truncate();
            albedo *= self.textures[texture as usize].sample(uv);
        }

        let diffuse = normal.dot(KEY).max(0.0);
        let fill = normal.dot(FILL).max(0.0);
        let specular = normal.dot((KEY + view).normalize()).max(0.0).powi(40);
        let rim = (1.0 - normal.dot(view)).powi(3);
        // The albedo's brightest channel scales the light and the rest is hue,
        // so hue times brightness is the lit color. Very dark materials keep a
        // floor so their shape still shows.
        let (value, hue) = value_and_hue(albedo);
        let brightness = (0.1 + 0.72 * diffuse + 0.22 * fill) * value.max(DARKEST_ALBEDO)
            + 0.25 * rim
            + 0.55 * specular;
        Shade::new(
            brightness,
            hue.lerp(Vec3::ONE, (specular + 0.3 * rim).min(1.0)),
        )
    }
}

fn edge(from: Vec2, to: Vec2, point: Vec2) -> f32 {
    (to - from).perp_dot(point - from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad() -> Mesh {
        let mut mesh = Mesh::default();
        for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            mesh.vertices.push(Vertex::new(Vec3::new(x, y, 0.0)));
        }
        for corners in [[0, 1, 2], [0, 2, 3]] {
            mesh.triangles.push(Triangle {
                corners,
                texture: None,
            });
        }
        mesh
    }

    #[test]
    fn a_facing_quad_covers_the_center_but_not_the_corners() {
        let mut mesh = quad();
        mesh.fill_missing_normals();
        let frame = mesh.render(Grid::cells(80, 24), Quat::IDENTITY, 0.5);
        assert!(frame.shade(40, 12).brightness > 0.3);
        assert_eq!(frame.shade(0, 0).brightness, 0.0);
    }

    #[test]
    fn missing_normals_are_smooth_on_flat_faces_and_sharp_across_creases() {
        let mut flat = quad();
        flat.fill_missing_normals();
        assert!(
            flat.vertices
                .iter()
                .all(|vertex| vertex.normal.z.abs() > 0.999)
        );

        // Two faces folded at a right angle along the x axis.
        let mut folded = Mesh::default();
        for position in [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z] {
            folded.vertices.push(Vertex::new(position));
        }
        for corners in [[0, 1, 2], [0, 3, 1]] {
            folded.triangles.push(Triangle {
                corners,
                texture: None,
            });
        }
        folded.fill_missing_normals();
        let normals: Vec<Vec3> = folded.vertices.iter().map(|vertex| vertex.normal).collect();
        assert!(
            normals[..3]
                .iter()
                .all(|normal| normal.distance(Vec3::Z) < 1e-5)
        );
        assert!(
            normals[3..]
                .iter()
                .all(|normal| normal.distance(Vec3::Y) < 1e-5)
        );
    }

    #[test]
    fn normalizing_fits_the_mesh_in_a_unit_sphere_around_the_origin() {
        let mut mesh = quad();
        for vertex in &mut mesh.vertices {
            vertex.position = vertex.position * 5.0 + Vec3::new(10.0, -3.0, 2.0);
        }
        mesh.normalize();
        let farthest = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position.length())
            .fold(0.0, f32::max);
        assert!((farthest - 1.0).abs() < 1e-5);
    }

    #[test]
    fn large_textures_are_averaged_down() {
        let texture = Texture::new(2048, 1024, vec![Vec3::splat(0.5); 2048 * 1024]);
        assert_eq!((texture.width, texture.height), (512, 256));
        assert_eq!(texture.sample(Vec2::new(-0.25, 3.5)), Vec3::splat(0.5));
    }

    #[test]
    fn the_torus_knot_is_closed_and_unit_sized() {
        let knot = Mesh::torus_knot(2, 3);
        assert_eq!(knot.triangles.len(), 360 * 18 * 2);
        assert!(
            knot.vertices
                .iter()
                .all(|vertex| vertex.position.length() <= 1.0 + 1e-5)
        );
    }
}
