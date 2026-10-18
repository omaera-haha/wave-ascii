use std::collections::HashMap;
use std::f32::consts::FRAC_PI_2;
use std::fs;
use std::io;
use std::path::Path;

use glam::{Mat3, Mat4, Quat, Vec2, Vec3, Vec4};

use crate::mesh::{Mesh, Texture, Triangle, Vertex};

pub const EXTENSIONS: &[&str] = &["glb", "gltf", "obj", "stl"];

/// Loads a 3D model, fills in missing normals, and fits it in a unit sphere.
pub fn load(path: &Path) -> io::Result<Mesh> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    let mesh = match extension.as_deref() {
        Some("glb" | "gltf") => read_gltf(&fs::read(path)?, path.parent())?,
        Some("obj") => parse_obj(&fs::read_to_string(path)?, |library| {
            fs::read_to_string(path.with_file_name(library)).ok()
        })?,
        Some("stl") => parse_stl(&fs::read(path)?)?,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("unsupported model format; use {}", EXTENSIONS.join(", ")),
            ));
        }
    };
    prepare(mesh)
}

/// Parses a self-contained glTF or GLB, such as one compiled into the program.
pub fn parse_gltf(bytes: &[u8]) -> io::Result<Mesh> {
    prepare(read_gltf(bytes, None)?)
}

fn prepare(mut mesh: Mesh) -> io::Result<Mesh> {
    if mesh.triangles.is_empty() {
        return Err(invalid("the model has no triangles"));
    }
    mesh.fill_missing_normals();
    mesh.normalize();
    Ok(mesh)
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn push_triangle(mesh: &mut Mesh, corners: [Vertex; 3]) {
    let first = mesh.vertices.len() as u32;
    mesh.vertices.extend(corners);
    mesh.triangles.push(Triangle {
        corners: [first, first + 1, first + 2],
        texture: None,
    });
}

// glTF colors are linear; everything here works in sRGB.
fn to_srgb(linear: Vec3) -> Vec3 {
    linear.max(Vec3::ZERO).powf(1.0 / 2.2)
}

// `base` is the directory that external buffers and images are relative to.
fn read_gltf(bytes: &[u8], base: Option<&Path>) -> io::Result<Mesh> {
    let error = |error: gltf::Error| invalid(error.to_string());
    let gltf::Gltf { document, blob } = gltf::Gltf::from_slice(bytes).map_err(error)?;
    let buffers = gltf::import_buffers(&document, base, blob).map_err(error)?;
    // Textures are optional: an image format the decoder lacks only costs color.
    let images = gltf::import_images(&document, base, &buffers).unwrap_or_default();

    let mut mesh = Mesh::default();
    let mut textures = HashMap::new();
    let mut add = |gltf_mesh: gltf::Mesh, transform: Mat4| {
        for primitive in gltf_mesh.primitives() {
            add_primitive(
                &mut mesh,
                &primitive,
                transform,
                &buffers,
                &images,
                &mut textures,
            );
        }
    };

    match document
        .default_scene()
        .or_else(|| document.scenes().next())
    {
        Some(scene) => {
            let mut pending: Vec<_> = scene.nodes().map(|node| (node, Mat4::IDENTITY)).collect();
            while let Some((node, parent)) = pending.pop() {
                let transform = parent * Mat4::from_cols_array_2d(&node.transform().matrix());
                if let Some(gltf_mesh) = node.mesh() {
                    add(gltf_mesh, transform);
                }
                pending.extend(node.children().map(|child| (child, transform)));
            }
        }
        None => document
            .meshes()
            .for_each(|gltf_mesh| add(gltf_mesh, Mat4::IDENTITY)),
    }
    Ok(mesh)
}

fn add_primitive(
    mesh: &mut Mesh,
    primitive: &gltf::Primitive,
    transform: Mat4,
    buffers: &[gltf::buffer::Data],
    images: &[gltf::image::Data],
    textures: &mut HashMap<usize, Option<u32>>,
) {
    if primitive.mode() != gltf::mesh::Mode::Triangles {
        return;
    }
    let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(|data| &data[..]));
    let Some(positions) = reader.read_positions() else {
        return;
    };

    let material = primitive.material().pbr_metallic_roughness();
    let factor = to_srgb(Vec4::from(material.base_color_factor()).truncate());
    let texture_info = material.base_color_texture();
    let texture = texture_info.as_ref().and_then(|info| {
        let image = info.texture().source().index();
        *textures.entry(image).or_insert_with(|| {
            let texture = texture_from_image(images.get(image)?)?;
            mesh.textures.push(texture);
            Some(mesh.textures.len() as u32 - 1)
        })
    });
    let uv_set = texture_info.as_ref().map_or(0, |info| info.tex_coord());

    let normals: Vec<[f32; 3]> = reader.read_normals().map_or(Vec::new(), Iterator::collect);
    let uvs: Vec<[f32; 2]> = reader
        .read_tex_coords(uv_set)
        .map_or(Vec::new(), |uvs| uvs.into_f32().collect());
    let colors: Vec<[f32; 4]> = reader
        .read_colors(0)
        .map_or(Vec::new(), |colors| colors.into_rgba_f32().collect());
    let normal_transform = Mat3::from_mat4(transform).inverse().transpose();

    let first = mesh.vertices.len() as u32;
    for (index, position) in positions.enumerate() {
        mesh.vertices.push(Vertex {
            position: transform.transform_point3(position.into()),
            normal: normals.get(index).map_or(Vec3::ZERO, |normal| {
                (normal_transform * Vec3::from(*normal)).normalize_or_zero()
            }),
            uv: uvs.get(index).map_or(Vec2::ZERO, |uv| Vec2::from(*uv)),
            color: factor
                * colors
                    .get(index)
                    .map_or(Vec3::ONE, |color| to_srgb(Vec4::from(*color).truncate())),
        });
    }
    let count = mesh.vertices.len() as u32 - first;
    let indices: Vec<u32> = reader.read_indices().map_or_else(
        || (0..count).collect(),
        |indices| indices.into_u32().collect(),
    );
    for corners in indices.as_chunks::<3>().0 {
        if corners.iter().all(|corner| *corner < count) {
            mesh.triangles.push(Triangle {
                corners: corners.map(|corner| first + corner),
                texture,
            });
        }
    }
}

fn texture_from_image(image: &gltf::image::Data) -> Option<Texture> {
    use gltf::image::Format;
    let (channels, depth) = match image.format {
        Format::R8 => (1, 1),
        Format::R8G8 => (2, 1),
        Format::R8G8B8 => (3, 1),
        Format::R8G8B8A8 => (4, 1),
        Format::R16 => (1, 2),
        Format::R16G16 => (2, 2),
        Format::R16G16B16 => (3, 2),
        Format::R16G16B16A16 => (4, 2),
        Format::R32G32B32FLOAT => (3, 4),
        Format::R32G32B32A32FLOAT => (4, 4),
    };
    let (width, height) = (image.width as usize, image.height as usize);
    let stride = channels * depth;
    if width == 0 || height == 0 || image.pixels.len() < width * height * stride {
        return None;
    }
    let channel = |texel: &[u8], index: usize| {
        let bytes = &texel[index * depth..(index + 1) * depth];
        match *bytes {
            [byte] => byte as f32 / 255.0,
            [low, high] => u16::from_ne_bytes([low, high]) as f32 / 65535.0,
            [a, b, c, d] => f32::from_ne_bytes([a, b, c, d]),
            _ => 0.0,
        }
    };
    let texels = image
        .pixels
        .chunks_exact(stride)
        .take(width * height)
        .map(|texel| {
            if channels < 3 {
                Vec3::splat(channel(texel, 0))
            } else {
                Vec3::new(channel(texel, 0), channel(texel, 1), channel(texel, 2))
            }
        })
        .collect();
    Some(Texture::new(width, height, texels))
}

/// Parses Wavefront OBJ: positions with optional vertex colors, texture
/// coordinates, normals, and polygon faces. Materials from `mtllib` files,
/// read through `read_library`, contribute their diffuse color.
fn parse_obj(source: &str, read_library: impl Fn(&str) -> Option<String>) -> io::Result<Mesh> {
    let mut positions = Vec::new();
    let mut vertex_colors = Vec::new();
    let mut uvs = Vec::new();
    let mut normals = Vec::new();
    let mut materials = HashMap::new();
    let mut material = Vec3::ONE;
    let mut mesh = Mesh::default();

    for (number, line) in source.lines().enumerate() {
        let context = |message: &str| invalid(format!("line {}: {message}", number + 1));
        let line = line.split('#').next().unwrap_or_default().trim();
        let Some((keyword, rest)) = line.split_once(char::is_whitespace).or(Some((line, "")))
        else {
            continue;
        };
        let rest = rest.trim();
        let numbers = || -> io::Result<Vec<f32>> {
            rest.split_whitespace()
                .map(|word| word.parse().map_err(|_| context("expected a number")))
                .collect()
        };
        match keyword {
            "v" => {
                let values = numbers()?;
                if values.len() < 3 {
                    return Err(context("a vertex needs three coordinates"));
                }
                positions.push(Vec3::from_slice(&values));
                vertex_colors.push(if values.len() >= 6 {
                    Vec3::from_slice(&values[3..])
                } else {
                    Vec3::ONE
                });
            }
            // OBJ's v axis points up the image; textures here index downward.
            "vt" => {
                let values = numbers()?;
                let u = values.first().copied().unwrap_or(0.0);
                let v = values.get(1).copied().unwrap_or(0.0);
                uvs.push(Vec2::new(u, 1.0 - v));
            }
            "vn" => {
                let values = numbers()?;
                if values.len() < 3 {
                    return Err(context("a normal needs three components"));
                }
                normals.push(Vec3::from_slice(&values));
            }
            "f" => {
                let corners = rest
                    .split_whitespace()
                    .map(|corner| {
                        let mut parts = corner.split('/');
                        let position = resolve(parts.next(), positions.len())
                            .ok_or_else(|| context("a face refers to a missing vertex"))?;
                        let uv = resolve(parts.next(), uvs.len());
                        let normal = resolve(parts.next(), normals.len());
                        Ok(Vertex {
                            position: positions[position],
                            normal: normal
                                .map_or(Vec3::ZERO, |index| normals[index].normalize_or_zero()),
                            uv: uv.map_or(Vec2::ZERO, |index| uvs[index]),
                            color: vertex_colors[position] * material,
                        })
                    })
                    .collect::<io::Result<Vec<_>>>()?;
                // Fan out polygons into triangles.
                for index in 2..corners.len() {
                    push_triangle(&mut mesh, [corners[0], corners[index - 1], corners[index]]);
                }
            }
            "mtllib" => {
                if let Some(library) = read_library(rest) {
                    materials.extend(parse_mtl(&library));
                }
            }
            "usemtl" => material = materials.get(rest).copied().unwrap_or(Vec3::ONE),
            _ => {}
        }
    }
    Ok(mesh)
}

// OBJ indices start at 1, and negative indices count back from the end.
fn resolve(index: Option<&str>, count: usize) -> Option<usize> {
    let index: i64 = index.filter(|index| !index.is_empty())?.parse().ok()?;
    let resolved = if index < 0 {
        count as i64 + index
    } else {
        index - 1
    };
    (0..count as i64)
        .contains(&resolved)
        .then_some(resolved as usize)
}

fn parse_mtl(source: &str) -> HashMap<String, Vec3> {
    let mut materials = HashMap::new();
    let mut name = None;
    for line in source.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("newmtl") {
            name = Some(rest.trim().to_owned());
        } else if let (Some(rest), Some(name)) = (line.strip_prefix("Kd"), &name) {
            let values: Vec<f32> = rest
                .split_whitespace()
                .filter_map(|word| word.parse().ok())
                .collect();
            if values.len() >= 3 {
                materials.insert(name.clone(), Vec3::from_slice(&values));
            }
        }
    }
    materials
}

/// Parses binary or ASCII STL. STL files are usually Z-up, so they are turned
/// to the Y-up convention used here.
fn parse_stl(bytes: &[u8]) -> io::Result<Mesh> {
    let up = Quat::from_rotation_x(-FRAC_PI_2);
    let mut mesh = Mesh::default();
    let mut add = |corners: [Vec3; 3]| {
        push_triangle(&mut mesh, corners.map(|corner| Vertex::new(up * corner)))
    };

    let binary_size = bytes.get(80..84).map(|count| {
        84 + 50 * u32::from_le_bytes([count[0], count[1], count[2], count[3]]) as usize
    });
    if binary_size == Some(bytes.len()) {
        for record in bytes[84..].as_chunks::<50>().0 {
            let point = |offset: usize| {
                Vec3::from_array([0, 4, 8].map(|component| {
                    let start = offset + component;
                    f32::from_le_bytes([
                        record[start],
                        record[start + 1],
                        record[start + 2],
                        record[start + 3],
                    ])
                }))
            };
            add([point(12), point(24), point(36)]);
        }
    } else {
        let text = std::str::from_utf8(bytes).map_err(|_| invalid("not a valid STL file"))?;
        let points = text
            .lines()
            .filter_map(|line| line.trim().strip_prefix("vertex"))
            .map(|rest| {
                let values: Vec<f32> = rest
                    .split_whitespace()
                    .map(str::parse)
                    .collect::<Result<_, _>>()
                    .map_err(|_| invalid("an STL vertex has a malformed number"))?;
                if values.len() == 3 {
                    Ok(Vec3::from_slice(&values))
                } else {
                    Err(invalid("an STL vertex needs three coordinates"))
                }
            })
            .collect::<io::Result<Vec<_>>>()?;
        for corners in points.as_chunks::<3>().0 {
            add(*corners);
        }
    }
    Ok(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQUARE_OBJ: &str = "\
mtllib square.mtl
v -1 -1 0
v 1 -1 0
v 1 1 0
v -1 1 0 0.5 0.5 0.5
vt 0 0
vt 1 1
usemtl red
f 1/1 2/1 3/2 -1/2  # one quad, the last corner indexed from the end
";

    #[test]
    fn obj_quads_become_two_triangles_with_material_colors() {
        let mesh = parse_obj(SQUARE_OBJ, |name| {
            (name == "square.mtl").then(|| "newmtl red\nKd 1 0 0\n".to_owned())
        })
        .unwrap();
        assert_eq!(mesh.triangles.len(), 2);
        assert_eq!(mesh.vertices[0].color, Vec3::X);
        assert_eq!(mesh.vertices[0].uv, Vec2::new(0.0, 1.0));
        assert_eq!(mesh.vertices[5].color, Vec3::new(0.5, 0.0, 0.0));
        assert_eq!(mesh.vertices[5].position, Vec3::new(-1.0, 1.0, 0.0));
    }

    #[test]
    fn obj_faces_must_refer_to_existing_vertices() {
        let error = parse_obj("v 0 0 0\nf 1 2 3\n", |_| None).unwrap_err();
        assert!(error.to_string().contains("line 2"));
    }

    #[test]
    fn ascii_and_binary_stl_describe_the_same_triangle() {
        let ascii = "solid t\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid t\n";
        let mut binary = vec![0; 80];
        binary.extend(1_u32.to_le_bytes());
        for value in [
            0.0_f32, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
        ] {
            binary.extend(value.to_le_bytes());
        }
        binary.extend([0, 0]);

        let from_ascii = parse_stl(ascii.as_bytes()).unwrap();
        let from_binary = parse_stl(&binary).unwrap();
        assert_eq!(from_ascii, from_binary);
        // Z-up becomes Y-up: the triangle's y axis now points away from the viewer.
        assert!(
            from_ascii.vertices[2]
                .position
                .distance(Vec3::new(0.0, 0.0, -1.0))
                < 1e-6
        );
    }
}
