"""A bonsai tree: an S-curved, tapering trunk with flared roots and a few
branches ending in faceted foliage pads, in a shallow blue glazed pot with
moss on the soil."""

import math
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from meshkit import box, hex_color, icosphere, lathe, merge, mix, save, tube  # noqa: E402

GLAZE = hex_color("#2f66c8")
GLAZE_LIGHT = hex_color("#5d93e8")
SOIL = hex_color("#7a5636")
MOSS = hex_color("#6cc23e")
MOSS_LIGHT = hex_color("#a2e05a")
BARK = hex_color("#8e6a4c")
BARK_LIGHT = hex_color("#b8936c")
LEAF_DARK = hex_color("#3a8f3c")
LEAF = hex_color("#5cbc45")
LEAF_LIGHT = hex_color("#9be060")

POT_LENGTH = 1.5   # stretch of the square pot along X
POT_HEIGHT = 0.5
SOIL_LEVEL = 0.42


def catmull_rom(points, steps):
    """A smooth curve through the control points, `steps` samples per span."""
    padded = [points[0]] + list(points) + [points[-1]]
    curve = []
    for i in range(1, len(padded) - 2):
        p0, p1, p2, p3 = padded[i - 1], padded[i], padded[i + 1], padded[i + 2]
        for s in range(steps):
            t = s / steps
            curve.append(tuple(
                0.5 * (2 * b + (-a + c) * t + (2 * a - 5 * b + 4 * c - d) * t * t + (-a + 3 * b - 3 * c + d) * t ** 3)
                for a, b, c, d in zip(p0, p1, p2, p3)))
    curve.append(points[-1])
    return curve


def point_along(curve, t):
    """The point a fraction t of the way along a sampled curve."""
    position = t * (len(curve) - 1)
    i = min(int(position), len(curve) - 2)
    f = position - i
    return tuple(a + (b - a) * f for a, b in zip(curve[i], curve[i + 1]))


def bark(position, normal):
    """Twisting stripes of light and dark bark."""
    angle = math.atan2(normal[2], normal[0])
    stripe = 0.5 + 0.5 * math.sin(3 * angle + 7 * position[1])
    return mix(BARK, BARK_LIGHT, stripe ** 2 * 0.8)


def pot():
    """A shallow rectangular pot: a square frustum from a four-sided lathe,
    stretched along X, with a rolled lip, an inner wall, and little feet."""
    top = 0.95
    profile = [(0.0, 0.06), (0.78, 0.06), (top, POT_HEIGHT - 0.04), (top + 0.08, POT_HEIGHT - 0.02),
               (top + 0.08, POT_HEIGHT + 0.03), (top - 0.03, POT_HEIGHT + 0.03), (top - 0.05, SOIL_LEVEL - 0.1)]
    body = lathe(profile, 4, smooth=False, start=math.pi / 4).scale(POT_LENGTH, 1, 1)
    body.paint_faces(lambda center, normal: GLAZE_LIGHT if abs(normal[1]) > 0.7 and center[1] > 0.3 else GLAZE)
    # The square's corners sit at radius `top`, so its half side is top / sqrt 2.
    half_x, half_z = 0.78 / math.sqrt(2) * POT_LENGTH, 0.78 / math.sqrt(2)
    feet = [box(0.22, 0.1, 0.18, GLAZE).translate(sx * (half_x - 0.14), 0.04, sz * (half_z - 0.12))
            for sx in (-1, 1) for sz in (-1, 1)]
    soil_half = (top - 0.05) / math.sqrt(2)
    soil = box(2 * soil_half * POT_LENGTH, 0.04, 2 * soil_half, SOIL).translate(0, SOIL_LEVEL - 0.02, 0)
    return merge(body, soil, *feet), soil_half


def moss(generator, soil_half, trunk_base):
    """Low cushions of moss scattered over the soil, avoiding the trunk."""
    parts = []
    while len(parts) < 12:
        x = generator.uniform(-soil_half * POT_LENGTH + 0.3, soil_half * POT_LENGTH - 0.3)
        z = generator.uniform(-soil_half + 0.25, soil_half - 0.25)
        if math.hypot(x - trunk_base[0], z - trunk_base[2]) < 0.3:
            continue
        size = generator.uniform(0.13, 0.22)
        cushion = icosphere(size, 0).scale(1.3, 0.45, 1.1).jitter(size * 0.12, seed=len(parts))
        cushion.paint_faces(lambda center, normal: mix(MOSS, MOSS_LIGHT, generator.uniform(0, 0.7)))
        parts.append(cushion.rotate_y(generator.uniform(0, math.pi)).translate(x, SOIL_LEVEL, z))
    return parts


def foliage_pad(center, size, generator):
    """A flattened cloud of faceted spheres, lighter where it faces the sky."""
    parts = []
    count = 4 if size > 0.4 else 3
    for i in range(count):
        angle = 2 * math.pi * i / count + generator.uniform(-0.4, 0.4)
        reach = size * (0.45 if i else 0.0) * generator.uniform(0.7, 1.1)
        radius = size * generator.uniform(0.58, 0.72) * (1.15 if i == 0 else 1.0)
        blob = icosphere(radius, 1).jitter(radius * 0.12, seed=generator.randrange(1000))
        blob.scale(1.0, 0.62, 1.0)
        blob.translate(center[0] + reach * math.cos(angle), center[1] + generator.uniform(-0.05, 0.08) * size,
                       center[2] + reach * math.sin(angle))
        parts.append(blob)
    pad = merge(*parts)

    def color(middle, normal):
        sky = max(0.0, normal[1])
        base = mix(LEAF_DARK, LEAF, min(1.0, 0.4 + sky))
        return mix(base, LEAF_LIGHT, sky ** 2 * generator.uniform(0.4, 1.0))

    return pad.paint_faces(color)


def build():
    generator = random.Random(8)
    pot_mesh, soil_half = pot()
    parts = [pot_mesh]

    # The trunk rises off center in an S, leaning back over the pot.
    trunk_points = [(-0.42, SOIL_LEVEL - 0.05, 0.0), (-0.36, 0.75, 0.04), (0.08, 1.12, 0.12),
                    (0.26, 1.5, -0.02), (0.0, 1.88, -0.1), (-0.14, 2.22, 0.0)]
    trunk = catmull_rom(trunk_points, 8)

    def trunk_radius(t):
        flare = max(0.0, 1 - t / 0.1) ** 2
        return 0.06 + 0.17 * (1 - t) ** 1.3 + 0.1 * flare

    parts.append(tube(trunk, trunk_radius, 12).paint(bark))

    # Roots grip the soil around the base.
    base = trunk_points[0]
    for angle in (0.3, 1.9, 3.3, 4.6):
        reach = generator.uniform(0.3, 0.42)
        dx, dz = reach * math.cos(angle), reach * math.sin(angle)
        root = catmull_rom([(base[0], SOIL_LEVEL + 0.12, base[2]),
                            (base[0] + dx * 0.5, SOIL_LEVEL + 0.05, base[2] + dz * 0.5),
                            (base[0] + dx, SOIL_LEVEL - 0.02, base[2] + dz)], 3)
        parts.append(tube(root, lambda t: 0.09 * (1 - t) + 0.025, 7).paint(bark))

    # Branches: where they leave the trunk (fraction of its length), the
    # direction they head (radians around Y), length, rise, and pad size.
    branches = [
        (0.38, 0.1, 0.95, 0.12, 0.5),
        (0.52, 3.3, 0.85, 0.18, 0.46),
        (0.64, 1.6, 0.6, 0.2, 0.4),
        (0.72, 4.8, 0.65, 0.22, 0.4),
        (0.84, 0.6, 0.55, 0.2, 0.36),
    ]
    for start, heading, length, rise, pad in branches:
        origin = point_along(trunk, start)
        direction = (math.cos(heading), 0.0, -math.sin(heading))
        middle = tuple(o + d * length * 0.45 for o, d in zip(origin, direction))
        tip = tuple(o + d * length for o, d in zip(origin, direction))
        path = catmull_rom([origin, middle, (tip[0], tip[1] + rise, tip[2])], 5)
        # Bonsai branches dip and then lift toward the pad.
        path = [(x, y - 0.12 * math.sin(math.pi * i / (len(path) - 1)), z) for i, (x, y, z) in enumerate(path)]
        parts.append(tube(path, lambda t: 0.075 * (1 - t) + 0.03, 7).paint(bark))
        end = path[-1]
        parts.append(foliage_pad((end[0], end[1] + 0.08, end[2]), pad, generator))

    apex = trunk[-1]
    parts.append(foliage_pad((apex[0], apex[1] + 0.05, apex[2]), 0.5, generator))
    parts += moss(generator, soil_half, base)
    return merge(*parts)


if __name__ == "__main__":
    save(build(), "bonsai")
