"""Three fly agaric mushrooms on a mossy mound: red caps with raised white
warts, cream stems with a drooping skirt, plus grass blades, a small fern,
and a snail passing by."""

import math
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from meshkit import Mesh, extrude, hex_color, icosphere, lathe, merge, mix, save, tube  # noqa: E402

CAP_RED = hex_color("#d8231f")
CAP_ORANGE = hex_color("#f04a1c")
WART = hex_color("#fff8ec")
GILLS = hex_color("#f2e2bf")
STEM = hex_color("#fff6e2")
STEM_SHADOW = hex_color("#e8d3ab")
MOSS_DARK = hex_color("#3e7d2c")
MOSS_LIGHT = hex_color("#8ccc4a")
GRASS = hex_color("#93dc52")
FERN = hex_color("#86d957")
SHELL = hex_color("#f2a23a")
SHELL_BAND = hex_color("#b8622a")
SNAIL_BODY = hex_color("#e9d6a6")

MOUND_WIDTH = 1.25
MOUND_DEPTH = 1.15
MOUND_HEIGHT = 0.42


def revolve(profile_at, segments):
    """Like lathe, but the profile may change with the angle: `profile_at`
    takes an angle and returns a list of (radius, height) points."""
    rows = None
    for s in range(segments):
        angle = 2 * math.pi * s / segments
        points = [(r * math.cos(angle), y, -r * math.sin(angle)) for r, y in profile_at(angle)]
        if rows is None:
            rows = [[] for _ in points]
        for row, point in zip(rows, points):
            row.append(point)
    mesh = Mesh()
    for row in rows:
        mesh.positions += row
    for r in range(len(rows) - 1):
        for s in range(segments):
            a = r * segments + s
            b = r * segments + (s + 1) % segments
            mesh.triangles += [(a, a + segments, b), (b, a + segments, b + segments)]
    mesh.normals = [(0.0, 0.0, 0.0)] * len(mesh.positions)
    mesh.colors = [(1.0, 1.0, 1.0)] * len(mesh.positions)
    return mesh.make_smooth()


def mound_height(x, z):
    inside = 1 - (x / MOUND_WIDTH) ** 2 - (z / MOUND_DEPTH) ** 2
    return MOUND_HEIGHT * math.sqrt(max(inside, 0.0))


def cap(radius, height, generator, warts):
    """A domed cap with its rim at y = 0, a cream underside, and white warts
    standing out from the red skin."""
    # (radius, height) fractions from the crown down to the rim.
    crown = [(0.0, 1.0), (0.28, 0.97), (0.55, 0.85), (0.78, 0.62), (0.93, 0.34), (1.0, 0.1), (0.97, -0.02)]
    top = lathe([(radius * r, height * y) for r, y in crown], 28)
    top.paint(lambda p, n: mix(CAP_RED, CAP_ORANGE, max(0.0, 1 - p[1] / height) ** 1.5))
    under = lathe([(radius * 0.97, -height * 0.02), (radius * 0.6, height * 0.04), (radius * 0.2, height * 0.12)], 28)
    under.paint(GILLS)

    def point_on_crown(t, angle):
        # Walks the crown profile to fraction t and returns the point and normal.
        position = t * (len(crown) - 2)
        i = min(int(position), len(crown) - 3)
        f = position - i
        r = crown[i][0] + (crown[i + 1][0] - crown[i][0]) * f
        y = crown[i][1] + (crown[i + 1][1] - crown[i][1]) * f
        dr = (crown[i + 1][0] - crown[i][0]) * radius
        dy = (crown[i + 1][1] - crown[i][1]) * height
        # The outward normal of the profile, turned to the angle.
        nr, ny = -dy, dr
        length = math.hypot(nr, ny)
        nr, ny = abs(nr) / length, abs(ny) / length
        point = (radius * r * math.cos(angle), height * y, -radius * r * math.sin(angle))
        normal = (nr * math.cos(angle), ny, -nr * math.sin(angle))
        return point, normal

    spots = []
    placed = []
    attempts = 0
    while len(spots) < warts and attempts < 2000:
        attempts += 1
        t = generator.uniform(0.0, 0.8)
        angle = generator.uniform(0, 2 * math.pi)
        point, normal = point_on_crown(t, angle)
        size = radius * generator.uniform(0.1, 0.14)
        if any(math.dist(point, other) < size * 2.6 for other in placed):
            continue
        placed.append(point)
        wart = lathe([(0.0, size * 0.5), (size * 0.75, size * 0.38), (size, -size * 0.1)], 8, WART).align(normal)
        spots.append(wart.translate(*point))
    return merge(top, under, *spots)


def stem(radius, height, skirt_height):
    """A cream stalk, swollen at the foot, with a frilly skirt hanging from
    it just under the cap."""
    profile = [(0.0, -0.15), (radius * 1.25, -0.15), (radius * 1.45, 0.08), (radius * 1.2, 0.25 * height),
               (radius * 1.0, 0.55 * height), (radius * 0.85, height)]
    stalk = lathe(profile, 18)
    stalk.paint(lambda p, n: mix(STEM_SHADOW, STEM, min(1.0, max(0.0, p[1] / height) * 1.4)))

    def skirt_profile(angle):
        # The skirt flares out and droops; its hem ripples around the stalk.
        hem = 1.0 + 0.12 * math.sin(7 * angle)
        top = skirt_height
        r = radius * 0.9
        return [(r, top), (r * 1.55, top - 0.05 * height), (r * 1.95 * hem, top - 0.13 * height),
                (r * 2.05 * hem, top - 0.2 * height)]

    skirt = revolve(skirt_profile, 28).paint(WART)
    return merge(stalk, skirt)


def mushroom(cap_radius, cap_height, stem_radius, stem_height, warts, generator):
    """A whole mushroom standing on y = 0, the skirt just below the cap."""
    return merge(
        stem(stem_radius, stem_height, stem_height * 0.82),
        cap(cap_radius, cap_height, generator, warts).translate(0, stem_height * 0.96, 0),
    )


def mound(generator):
    """A low, lumpy dome of moss. Only the top half of a squashed geodesic
    sphere is kept. Its normals are ruffled so the light breaks into soft
    blotches, like a cushion of moss, instead of one glossy highlight."""
    dome = icosphere(1.0, 3, smooth=True).scale(MOUND_WIDTH, MOUND_HEIGHT, MOUND_DEPTH)
    dome.transform(lambda p: (p[0], max(p[1], 0.0), p[2]))
    dome.triangles = [t for t in dome.triangles if max(dome.positions[i][1] for i in t) > 1e-6]
    dome.jitter(0.03, seed=3)
    dome.make_smooth()
    ruffled = []
    for normal in dome.normals:
        bent = tuple(n + generator.uniform(-0.45, 0.45) for n in normal)
        length = math.sqrt(sum(c * c for c in bent))
        ruffled.append(tuple(c / length for c in bent))
    dome.normals = ruffled

    def color(position, normal):
        x, y, z = position
        patch = 0.5 + 0.35 * math.sin(4.1 * x + 1.7) * math.sin(3.7 * z) + generator.uniform(-0.2, 0.2)
        return mix(MOSS_DARK, MOSS_LIGHT, max(0.0, min(1.0, patch)))

    return dome.paint(color)


def grass_blade(height, width, bend, color):
    """A tapering, curving blade, thick enough to show at low resolution."""
    steps = 3
    left, right = [], []
    for i in range(steps + 1):
        t = i / steps
        half = width / 2 * (1 - t) ** 0.8
        x = bend * t * t
        left.append((x - half, height * t))
        right.append((x + half, height * t))
    polygon = left + right[::-1][1:]
    return extrude(polygon, width * 0.35, color)


def leaflet(base, along, across, length, width, color):
    """A flat diamond leaf from `base`, pointing along `along`, with its
    width across `across` (both unit vectors)."""
    def at(a, b):
        return tuple(p + u * a + v * b for p, u, v in zip(base, along, across))

    mesh = Mesh()
    mesh.positions = [base, at(length * 0.45, width * 0.5), at(length, 0.0), at(length * 0.45, -width * 0.5)]
    mesh.triangles = [(0, 1, 2), (0, 2, 3)]
    mesh.normals = [(0.0, 1.0, 0.0)] * 4
    mesh.colors = [color] * 4
    return mesh.make_flat()


def fern(generator, fronds=5):
    """A little fern: arching stems with paired leaflets that shrink toward
    the tip."""
    parts = []
    for f in range(fronds):
        heading = 2 * math.pi * f / fronds + generator.uniform(-0.3, 0.3)
        length = generator.uniform(0.6, 0.8)
        rise = generator.uniform(0.45, 0.6)
        steps = 8
        path = []
        for i in range(steps + 1):
            t = i / steps
            # Rises, then arches over.
            path.append((length * t * math.cos(heading), rise * (1.6 * t - 1.1 * t * t),
                         -length * t * math.sin(heading)))
        parts.append(tube(path, lambda t: 0.025 * (1 - 0.6 * t), 5, FERN))
        side = (math.sin(heading), 0.0, math.cos(heading))
        for i in range(1, steps + 1):
            t = i / steps
            before, after = path[i - 1], path[min(i + 1, steps)]
            tangent = tuple(b - a for a, b in zip(before, after))
            size = math.sqrt(sum(c * c for c in tangent))
            tangent = tuple(c / size for c in tangent)
            length_here = 0.22 * (1 - t) + 0.07
            color = mix(FERN, MOSS_LIGHT, t * 0.6)
            for flip in (-1, 1):
                # Leaflets reach sideways, a little forward and drooping.
                along = tuple(flip * s + 0.35 * d for s, d in zip(side, tangent))
                along = (along[0], along[1] - 0.25, along[2])
                norm = math.sqrt(sum(c * c for c in along))
                along = tuple(c / norm for c in along)
                parts.append(leaflet(path[i], along, tangent, length_here, length_here * 0.55, color))
    return merge(*parts)


def snail():
    """A small snail facing +Z: a soft body with eye stalks and a coiled,
    banded shell on its back."""
    body_path = [(0.0, 0.02, -0.24), (0.0, 0.035, -0.12), (0.0, 0.045, 0.02), (0.0, 0.06, 0.13),
                 (0.0, 0.1, 0.2), (0.0, 0.13, 0.23)]
    body = tube(body_path, lambda t: 0.025 + 0.045 * math.sin(math.pi * min(1.0, t * 1.15)) ** 0.6, 8, SNAIL_BODY)
    stalks = []
    for side in (-1, 1):
        stalk_path = [(0.0, 0.14, 0.22), (side * 0.04, 0.22, 0.26), (side * 0.07, 0.29, 0.28)]
        stalks.append(tube(stalk_path, 0.016, 5, SNAIL_BODY))
        stalks.append(icosphere(0.032, 1, SHELL_BAND, smooth=True).translate(side * 0.07, 0.3, 0.28))
    # The shell coils inward in the body's plane, drifting sideways a little
    # as it winds so it isn't flat.
    turns = 2.3
    steps = 30
    path, radii = [], []
    for i in range(steps + 1):
        angle = 2 * math.pi * turns * i / steps
        size = 0.15 * math.exp(-0.32 * angle)
        path.append((0.05 * i / steps, 0.2 + size * math.sin(angle), -0.04 + size * math.cos(angle)))
        radii.append(size * 0.55 + 0.012)
    shell = tube(path, lambda t: radii[min(steps, round(t * steps))], 8, SHELL)
    # Band the shell along its coil: each ring of the tube is one path point.
    shell.colors = [SHELL_BAND if (i // 8) % 5 in (0, 1) and i < 8 * (steps + 1) else SHELL
                    for i in range(len(shell.positions))]
    return merge(body, shell, *stalks)


def build():
    generator = random.Random(5)
    parts = [mound(generator)]

    # (x, z, lean about z, lean about x, cap radius, cap height, stem radius, stem height, warts)
    shrooms = [
        (-0.3, -0.15, 0.10, -0.06, 0.8, 0.44, 0.17, 1.3, 16),
        (0.6, 0.3, -0.22, 0.10, 0.54, 0.37, 0.125, 0.92, 11),
        (-0.05, 0.72, 0.05, 0.22, 0.36, 0.32, 0.1, 0.52, 7),
    ]
    for x, z, lean_z, lean_x, cap_radius, cap_height, stem_radius, stem_height, warts in shrooms:
        shroom = mushroom(cap_radius, cap_height, stem_radius, stem_height, warts, generator)
        shroom.rotate_y(generator.uniform(0, 2 * math.pi)).rotate_z(lean_z).rotate_x(lean_x)
        parts.append(shroom.translate(x, mound_height(x, z) - 0.05, z))

    # Grass in tufts around the mound, kept clear of the stems.
    for tuft_angle, tuft_distance in ((0.3, 0.95), (1.9, 0.9), (2.8, 1.0), (4.0, 0.85), (5.3, 1.0)):
        cx, cz = tuft_distance * math.cos(tuft_angle), tuft_distance * math.sin(tuft_angle)
        for _ in range(3):
            x = cx + generator.uniform(-0.12, 0.12)
            z = cz + generator.uniform(-0.12, 0.12)
            blade = grass_blade(generator.uniform(0.3, 0.5), 0.09, generator.uniform(-0.15, 0.15),
                                mix(GRASS, MOSS_LIGHT, generator.uniform(0, 0.6)))
            blade.rotate_y(generator.uniform(0, math.pi)).rotate_x(generator.uniform(-0.25, 0.25))
            parts.append(blade.translate(x, mound_height(x, z) - 0.03, z))

    fern_x, fern_z = 0.75, -0.55
    parts.append(fern(generator).translate(fern_x, mound_height(fern_x, fern_z) - 0.03, fern_z))

    # The snail crawls around the mound, leaning with its slope.
    around, distance = 3.35, 0.8
    x, z = distance * math.cos(around), distance * math.sin(around)
    slope = math.atan(MOUND_HEIGHT * distance / MOUND_WIDTH ** 2 / math.sqrt(1 - (distance / MOUND_WIDTH) ** 2))
    crawler = snail().scale(1.6).rotate_z(-slope).rotate_y(-around)
    parts.append(crawler.translate(x, mound_height(x, z) - 0.02, z))
    return merge(*parts)


if __name__ == "__main__":
    save(build(), "mushrooms")
