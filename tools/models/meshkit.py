"""A small mesh-building kit for the bundled models, using only the standard
library. Shapes are built from primitives, moved into place, colored, merged,
and written as binary glTF (.glb) with positions, normals, and vertex colors.

Colors are sRGB triples from 0 to 1; `write_glb` converts them to the linear
values glTF expects. Every scale is arbitrary: wave-ascii centers each model
and fits it in a unit sphere. Y points up.
"""

import json
import math
import random
import struct


def _sub(a, b):
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def _add(a, b):
    return (a[0] + b[0], a[1] + b[1], a[2] + b[2])


def _scale(a, s):
    return (a[0] * s, a[1] * s, a[2] * s)


def _cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def _dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def _normalize(a):
    length = math.sqrt(_dot(a, a))
    return (0.0, 0.0, 0.0) if length < 1e-12 else _scale(a, 1.0 / length)


def mix(a, b, t):
    """Blends two colors (or any triples)."""
    return tuple(x + (y - x) * t for x, y in zip(a, b))


def hex_color(text):
    """'#ff8800' -> (1.0, 0.53, 0.0)"""
    text = text.lstrip("#")
    return tuple(int(text[i:i + 2], 16) / 255 for i in (0, 2, 4))


class Mesh:
    """Triangles with per-vertex positions, normals, and colors."""

    def __init__(self):
        self.positions = []
        self.normals = []
        self.colors = []
        self.triangles = []

    def copy(self):
        other = Mesh()
        other.positions = list(self.positions)
        other.normals = list(self.normals)
        other.colors = list(self.colors)
        other.triangles = list(self.triangles)
        return other

    def extend(self, *others):
        for other in others:
            offset = len(self.positions)
            self.positions += other.positions
            self.normals += other.normals
            self.colors += other.colors
            self.triangles += [(a + offset, b + offset, c + offset) for a, b, c in other.triangles]
        return self

    # Transforms. Each changes the mesh in place and returns it, so calls chain.

    def transform(self, point_function, normal_function=None):
        self.positions = [tuple(point_function(p)) for p in self.positions]
        if normal_function is not None:
            self.normals = [_normalize(normal_function(n)) for n in self.normals]
        return self

    def translate(self, x=0.0, y=0.0, z=0.0):
        return self.transform(lambda p: (p[0] + x, p[1] + y, p[2] + z))

    def scale(self, x, y=None, z=None):
        y = x if y is None else y
        z = x if z is None else z
        return self.transform(
            lambda p: (p[0] * x, p[1] * y, p[2] * z),
            lambda n: (n[0] / x, n[1] / y, n[2] / z),
        )

    def rotate_x(self, angle):
        c, s = math.cos(angle), math.sin(angle)
        turn = lambda p: (p[0], p[1] * c - p[2] * s, p[1] * s + p[2] * c)
        return self.transform(turn, turn)

    def rotate_y(self, angle):
        c, s = math.cos(angle), math.sin(angle)
        turn = lambda p: (p[0] * c + p[2] * s, p[1], -p[0] * s + p[2] * c)
        return self.transform(turn, turn)

    def rotate_z(self, angle):
        c, s = math.cos(angle), math.sin(angle)
        turn = lambda p: (p[0] * c - p[1] * s, p[0] * s + p[1] * c, p[2])
        return self.transform(turn, turn)

    def align(self, direction):
        """Turns the mesh so its +Y axis points along `direction`."""
        direction = _normalize(direction)
        axis = _cross((0.0, 1.0, 0.0), direction)
        sine = math.sqrt(_dot(axis, axis))
        cosine = _dot((0.0, 1.0, 0.0), direction)
        if sine < 1e-9:
            return self if cosine > 0 else self.rotate_x(math.pi)
        axis = _scale(axis, 1.0 / sine)
        angle = math.atan2(sine, cosine)

        def turn(p):
            # Rodrigues' rotation formula.
            c, s = math.cos(angle), math.sin(angle)
            return _add(_add(_scale(p, c), _scale(_cross(axis, p), s)), _scale(axis, _dot(axis, p) * (1 - c)))

        return self.transform(turn, turn)

    # Color.

    def paint(self, color):
        """Sets every vertex color. `color` is a triple or a function of
        (position, normal) returning one."""
        if callable(color):
            self.colors = [tuple(color(p, n)) for p, n in zip(self.positions, self.normals)]
        else:
            self.colors = [tuple(color)] * len(self.positions)
        return self

    def paint_faces(self, color):
        """Colors each triangle by a function of its center and normal. The
        mesh is made flat first, so faces don't share vertices."""
        self.make_flat()
        for triangle in self.triangles:
            a, b, c = (self.positions[i] for i in triangle)
            center = _scale(_add(_add(a, b), c), 1 / 3)
            normal = _normalize(_cross(_sub(b, a), _sub(c, a)))
            value = tuple(color(center, normal))
            for i in triangle:
                self.colors[i] = value
        return self

    # Shading.

    def make_flat(self):
        """Gives each triangle its own corners with the face normal, for a
        faceted, low-poly look."""
        positions, normals, colors, triangles = [], [], [], []
        for triangle in self.triangles:
            a, b, c = (self.positions[i] for i in triangle)
            normal = _normalize(_cross(_sub(b, a), _sub(c, a)))
            start = len(positions)
            for i in triangle:
                positions.append(self.positions[i])
                normals.append(normal)
                colors.append(self.colors[i])
            triangles.append((start, start + 1, start + 2))
        self.positions, self.normals, self.colors, self.triangles = positions, normals, colors, triangles
        return self

    def make_smooth(self):
        """Averages normals over triangles that share a vertex index."""
        sums = [(0.0, 0.0, 0.0)] * len(self.positions)
        for triangle in self.triangles:
            a, b, c = (self.positions[i] for i in triangle)
            face = _cross(_sub(b, a), _sub(c, a))
            for i in triangle:
                sums[i] = _add(sums[i], face)
        self.normals = [_normalize(n) for n in sums]
        return self

    def jitter(self, amount, seed=1):
        """Moves each distinct position randomly by up to `amount`, keeping
        shared corners together, for rocks and organic shapes."""
        generator = random.Random(seed)
        moved = {}
        result = []
        for p in self.positions:
            key = tuple(round(v, 5) for v in p)
            if key not in moved:
                moved[key] = tuple(v + generator.uniform(-amount, amount) for v in p)
            result.append(moved[key])
        self.positions = result
        return self

    def bounds(self):
        low = tuple(min(p[i] for p in self.positions) for i in range(3))
        high = tuple(max(p[i] for p in self.positions) for i in range(3))
        return low, high


def _from_grid(rows, wrap, color, smooth):
    """Builds a mesh from a grid of points: `rows` is a list of rings, each a
    list of points. Adjacent rings are stitched with quads."""
    mesh = Mesh()
    columns = len(rows[0])
    for ring in rows:
        mesh.positions += [tuple(p) for p in ring]
    span = columns if wrap else columns - 1
    for r in range(len(rows) - 1):
        for c in range(span):
            a = r * columns + c
            b = r * columns + (c + 1) % columns
            d = (r + 1) * columns + c
            e = (r + 1) * columns + (c + 1) % columns
            mesh.triangles += [(a, d, b), (b, d, e)]
    mesh.normals = [(0.0, 0.0, 0.0)] * len(mesh.positions)
    mesh.colors = [(1.0, 1.0, 1.0)] * len(mesh.positions)
    if smooth:
        mesh.make_smooth()
    else:
        mesh.make_flat()
    return mesh.paint(color)


def lathe(profile, segments=32, color=(1, 1, 1), smooth=True, start=0.0, sweep=2 * math.pi):
    """Revolves a profile of (radius, height) points around the Y axis. A
    point with radius 0 closes the shape there. Points that should make a
    sharp edge can be repeated, or use `smooth=False` for facets."""
    full = abs(sweep - 2 * math.pi) < 1e-9
    count = segments if full else segments + 1
    rows = []
    for radius, height in profile:
        ring = []
        for s in range(count):
            angle = start + sweep * s / segments
            ring.append((radius * math.cos(angle), height, -radius * math.sin(angle)))
        rows.append(ring)
    return _from_grid(rows, full, color, smooth)


def cylinder(radius=1.0, height=1.0, segments=24, color=(1, 1, 1), caps=True, smooth=True, top_radius=None):
    """A cylinder or truncated cone standing on y = 0."""
    top = radius if top_radius is None else top_radius
    side = lathe([(radius, 0.0), (top, height)], segments, color, smooth)
    if not caps:
        return side
    return side.extend(
        disk(radius, segments, color).rotate_x(math.pi),
        disk(top, segments, color).translate(0, height, 0),
    )


def cone(radius=1.0, height=1.0, segments=24, color=(1, 1, 1), smooth=True, base=True):
    """A cone standing on y = 0 with its tip at `height`."""
    mesh = lathe([(radius, 0.0), (0.0, height)], segments, color, smooth)
    if base:
        mesh.extend(disk(radius, segments, color).rotate_x(math.pi))
    return mesh


def disk(radius=1.0, segments=24, color=(1, 1, 1)):
    """A flat disk at y = 0 facing +Y."""
    mesh = Mesh()
    mesh.positions.append((0.0, 0.0, 0.0))
    for s in range(segments):
        angle = 2 * math.pi * s / segments
        mesh.positions.append((radius * math.cos(angle), 0.0, -radius * math.sin(angle)))
    for s in range(segments):
        mesh.triangles.append((0, 1 + s, 1 + (s + 1) % segments))
    mesh.normals = [(0.0, 1.0, 0.0)] * len(mesh.positions)
    return mesh.paint(color)


def ring(inner, outer, segments=48, color=(1, 1, 1)):
    """A flat annulus at y = 0, like a planet's rings."""
    rows = []
    for radius in (inner, outer):
        rows.append([(radius * math.cos(2 * math.pi * s / segments), 0.0,
                      -radius * math.sin(2 * math.pi * s / segments)) for s in range(segments)])
    mesh = _from_grid(rows, True, color, True)
    mesh.normals = [(0.0, 1.0, 0.0)] * len(mesh.positions)
    return mesh


def sphere(radius=1.0, segments=32, rings=16, color=(1, 1, 1), smooth=True):
    """A UV sphere centered on the origin."""
    profile = [(radius * math.sin(math.pi * r / rings), -radius * math.cos(math.pi * r / rings))
               for r in range(rings + 1)]
    return lathe(profile, segments, color, smooth)


def icosphere(radius=1.0, subdivisions=1, color=(1, 1, 1), smooth=False):
    """A geodesic sphere; faceted by default for a low-poly look."""
    t = (1 + math.sqrt(5)) / 2
    points = [(-1, t, 0), (1, t, 0), (-1, -t, 0), (1, -t, 0), (0, -1, t), (0, 1, t),
              (0, -1, -t), (0, 1, -t), (t, 0, -1), (t, 0, 1), (-t, 0, -1), (-t, 0, 1)]
    points = [_normalize(p) for p in points]
    faces = [(0, 11, 5), (0, 5, 1), (0, 1, 7), (0, 7, 10), (0, 10, 11), (1, 5, 9), (5, 11, 4),
             (11, 10, 2), (10, 7, 6), (7, 1, 8), (3, 9, 4), (3, 4, 2), (3, 2, 6), (3, 6, 8),
             (3, 8, 9), (4, 9, 5), (2, 4, 11), (6, 2, 10), (8, 6, 7), (9, 8, 1)]
    for _ in range(subdivisions):
        middles = {}

        def middle(a, b):
            key = (min(a, b), max(a, b))
            if key not in middles:
                middles[key] = len(points)
                points.append(_normalize(_scale(_add(points[a], points[b]), 0.5)))
            return middles[key]

        refined = []
        for a, b, c in faces:
            ab, bc, ca = middle(a, b), middle(b, c), middle(c, a)
            refined += [(a, ab, ca), (b, bc, ab), (c, ca, bc), (ab, bc, ca)]
        faces = refined
    mesh = Mesh()
    mesh.positions = [_scale(p, radius) for p in points]
    mesh.triangles = faces
    mesh.normals = [(0.0, 0.0, 0.0)] * len(points)
    mesh.colors = [(1.0, 1.0, 1.0)] * len(points)
    if smooth:
        mesh.make_smooth()
    else:
        mesh.make_flat()
    return mesh.paint(color)


def box(width=1.0, height=1.0, depth=1.0, color=(1, 1, 1)):
    """A box centered on the origin, with flat faces."""
    x, y, z = width / 2, height / 2, depth / 2
    corners = [(-x, -y, -z), (x, -y, -z), (x, y, -z), (-x, y, -z),
               (-x, -y, z), (x, -y, z), (x, y, z), (-x, y, z)]
    quads = [(0, 3, 2, 1), (4, 5, 6, 7), (0, 4, 7, 3), (1, 2, 6, 5), (3, 7, 6, 2), (0, 1, 5, 4)]
    mesh = Mesh()
    mesh.positions = corners
    for a, b, c, d in quads:
        mesh.triangles += [(a, b, c), (a, c, d)]
    mesh.normals = [(0.0, 0.0, 0.0)] * 8
    mesh.colors = [(1.0, 1.0, 1.0)] * 8
    mesh.make_flat()
    return mesh.paint(color)


def torus(major=1.0, minor=0.3, segments=48, sides=16, color=(1, 1, 1), smooth=True):
    """A torus around the Y axis."""
    profile = []
    for s in range(sides + 1):
        angle = 2 * math.pi * s / sides
        profile.append((major + minor * math.cos(angle), minor * math.sin(angle)))
    return lathe(profile, segments, color, smooth)


def _triangulate(polygon):
    """Ear clipping for a simple polygon of (x, y) points, either winding."""
    area = sum(polygon[i][0] * polygon[(i + 1) % len(polygon)][1]
               - polygon[(i + 1) % len(polygon)][0] * polygon[i][1] for i in range(len(polygon)))
    order = list(range(len(polygon)))
    if area < 0:
        order.reverse()

    def inside(p, a, b, c):
        def side(u, v, w):
            return (v[0] - u[0]) * (w[1] - u[1]) - (v[1] - u[1]) * (w[0] - u[0])
        return side(a, b, p) >= 0 and side(b, c, p) >= 0 and side(c, a, p) >= 0

    triangles = []
    guard = 0
    while len(order) > 3 and guard < 10000:
        guard += 1
        for k in range(len(order)):
            i, j, l = order[k - 1], order[k], order[(k + 1) % len(order)]
            a, b, c = polygon[i], polygon[j], polygon[l]
            if (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]) <= 0:
                continue
            if any(inside(polygon[m], a, b, c) for m in order if m not in (i, j, l)):
                continue
            triangles.append((i, j, l))
            order.pop(k)
            break
        else:
            break
    if len(order) == 3:
        triangles.append(tuple(order))
    return triangles


def extrude(polygon, depth=0.1, color=(1, 1, 1)):
    """Extrudes a simple polygon of (x, y) points along Z, centered on z = 0,
    with flat faces. Good for fins, sails, flags, and leaves."""
    mesh = Mesh()
    count = len(polygon)
    front = [(x, y, depth / 2) for x, y in polygon]
    back = [(x, y, -depth / 2) for x, y in polygon]
    mesh.positions = front + back
    for a, b, c in _triangulate(polygon):
        mesh.triangles.append((a, b, c))
        mesh.triangles.append((count + a, count + c, count + b))
    for i in range(count):
        j = (i + 1) % count
        mesh.triangles += [(i, count + i, count + j), (i, count + j, j)]
    mesh.normals = [(0.0, 0.0, 0.0)] * len(mesh.positions)
    mesh.colors = [(1.0, 1.0, 1.0)] * len(mesh.positions)
    mesh.make_flat()
    return mesh.paint(color)


def tube(path, radius=0.1, sides=12, color=(1, 1, 1), smooth=True, caps=True):
    """Sweeps a circle along a path of 3D points. `radius` is a number or a
    function of the position along the path, from 0 to 1."""
    radius_at = radius if callable(radius) else (lambda t: radius)
    count = len(path)
    tangents = []
    for i in range(count):
        before = path[max(i - 1, 0)]
        after = path[min(i + 1, count - 1)]
        tangents.append(_normalize(_sub(after, before)))
    # Parallel transport keeps the rings from twisting.
    reference = (0.0, 1.0, 0.0) if abs(tangents[0][1]) < 0.9 else (1.0, 0.0, 0.0)
    normal = _normalize(_cross(tangents[0], reference))
    rows = []
    for i in range(count):
        tangent = tangents[i]
        normal = _normalize(_sub(normal, _scale(tangent, _dot(normal, tangent))))
        binormal = _cross(tangent, normal)
        r = radius_at(i / max(count - 1, 1))
        ring_points = []
        for s in range(sides):
            angle = 2 * math.pi * s / sides
            offset = _add(_scale(normal, math.cos(angle) * r), _scale(binormal, math.sin(angle) * r))
            ring_points.append(_add(path[i], offset))
        rows.append(ring_points)
    mesh = _from_grid(rows, True, color, smooth)
    if caps:
        for i, end in ((0, path[0]), (count - 1, path[-1])):
            cap = Mesh()
            cap.positions = [end] + rows[i]
            cap.triangles = [(0, 1 + s, 1 + (s + 1) % sides) for s in range(sides)]
            facing = tangents[i] if i else _scale(tangents[0], -1)
            cap.normals = [facing] * len(cap.positions)
            cap.paint(color)
            mesh.extend(cap)
    return mesh


def merge(*meshes):
    result = Mesh()
    return result.extend(*meshes)


def _to_linear(value):
    return max(0.0, min(1.0, value)) ** 2.2


def write_glb(path, mesh, name="model"):
    """Writes the mesh as a single-primitive binary glTF."""
    if not mesh.triangles:
        raise ValueError("the mesh has no triangles")
    count = len(mesh.positions)
    positions = b"".join(struct.pack("<3f", *p) for p in mesh.positions)
    normals = b"".join(struct.pack("<3f", *n) for n in mesh.normals)
    colors = b"".join(
        struct.pack("<4B", *(round(_to_linear(c) * 255) for c in color), 255) for color in mesh.colors
    )
    wide = count > 65535
    index_format = "<3I" if wide else "<3H"
    indices = b"".join(struct.pack(index_format, *t) for t in mesh.triangles)

    blob = bytearray()
    views = []
    for data, target in ((positions, 34962), (normals, 34962), (colors, 34962), (indices, 34963)):
        while len(blob) % 4:
            blob.append(0)
        views.append({"buffer": 0, "byteOffset": len(blob), "byteLength": len(data), "target": target})
        blob += data
    while len(blob) % 4:
        blob.append(0)

    low, high = mesh.bounds()
    document = {
        "asset": {"version": "2.0", "generator": "wave-ascii tools/models/meshkit.py"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0, "name": name}],
        "meshes": [{"name": name, "primitives": [{
            "attributes": {"POSITION": 0, "NORMAL": 1, "COLOR_0": 2},
            "indices": 3,
        }]}],
        "buffers": [{"byteLength": len(blob)}],
        "bufferViews": views,
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": count, "type": "VEC3",
             "min": list(low), "max": list(high)},
            {"bufferView": 1, "componentType": 5126, "count": count, "type": "VEC3"},
            {"bufferView": 2, "componentType": 5121, "normalized": True, "count": count, "type": "VEC4"},
            {"bufferView": 3, "componentType": 5125 if wide else 5123,
             "count": len(mesh.triangles) * 3, "type": "SCALAR"},
        ],
    }
    text = json.dumps(document, separators=(",", ":")).encode()
    while len(text) % 4:
        text += b" "
    total = 12 + 8 + len(text) + 8 + len(blob)
    with open(path, "wb") as output:
        output.write(struct.pack("<4sII", b"glTF", 2, total))
        output.write(struct.pack("<I4s", len(text), b"JSON") + text)
        output.write(struct.pack("<I4s", len(blob), b"BIN\x00") + bytes(blob))
    print(f"{path}: {len(mesh.triangles)} triangles, {count} vertices, {total // 1024} KiB")


def save(mesh, name):
    """Writes `assets/<name>.glb` in the repository, wherever the script runs from."""
    import os
    assets = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "assets")
    write_glb(os.path.normpath(os.path.join(assets, name + ".glb")), mesh, name)
