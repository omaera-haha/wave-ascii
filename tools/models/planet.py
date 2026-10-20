"""A ringed gas giant with wavy cloud bands and a storm spot, a tilted ring
system split by dark gaps, and a small cratered moon.

Run from the repository root: python3 -I tools/models/planet.py
"""

import math
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from meshkit import hex_color, icosphere, lathe, merge, mix, save, sphere

# Cloud bands from the north pole (latitude 1) to the south pole (-1).
BANDS = [
    (0.86, hex_color("#b98d5a")),
    (0.66, hex_color("#f1dcab")),
    (0.5, hex_color("#dd8f45")),
    (0.34, hex_color("#efcf94")),
    (0.18, hex_color("#c06a35")),
    (-0.14, hex_color("#fbeccb")),
    (-0.3, hex_color("#cf7a3c")),
    (-0.46, hex_color("#ebc68a")),
    (-0.62, hex_color("#d9904e")),
    (-0.82, hex_color("#f0d9a6")),
    (-1.01, hex_color("#b98d5a")),
]
STORM = hex_color("#c2412a")
STORM_CENTER = (0.27, 1.2)  # (latitude as height, longitude in radians)

# Ring bands as (inner radius, outer radius, color); the gaps between them
# are the dark divisions.
RINGS = [
    (1.32, 1.5, hex_color("#9c8466")),
    (1.54, 1.72, hex_color("#e9d6ae")),
    (1.72, 1.92, hex_color("#f8ecd0")),
    (2.0, 2.18, hex_color("#cdb38a")),
    (2.18, 2.26, hex_color("#e2cba0")),
    (2.33, 2.39, hex_color("#bfa47c")),
]
# The rings tip toward the viewer at the start. Tilted less than the
# turntable's 0.3 radians, they never turn fully edge-on.
RING_TILT = 0.2


def band_color(height):
    """Blends between neighboring bands so their edges are soft but clear."""
    for (top, upper), (bottom, lower) in zip(BANDS, BANDS[1:]):
        if height >= bottom:
            t = (top - height) / (top - bottom)
            # Hold each band's color, then blend over its lower fifth.
            return mix(upper, lower, max(0.0, (t - 0.8) / 0.2))
    return BANDS[-1][1]


def gas_giant():
    mesh = sphere(1.0, 48, 30)

    def color(position, normal):
        x, y, z = position
        longitude = math.atan2(z, x)
        # Ripple the band edges so they look like flowing clouds.
        wavy = y + 0.035 * math.sin(4 * longitude + 6 * y) + 0.02 * math.sin(9 * longitude - 3 * y)
        base = band_color(wavy)
        # An oval storm, wider than tall, with a pale halo around it.
        storm_y, storm_longitude = STORM_CENTER
        across = math.atan2(math.sin(longitude - storm_longitude), math.cos(longitude - storm_longitude))
        distance = math.hypot(across / 0.4, (y - storm_y) / 0.13)
        if distance < 1.0:
            return mix(STORM, hex_color("#e0703f"), distance ** 2)
        if distance < 1.5:
            return mix(hex_color("#fbe6c4"), base, (distance - 1.0) / 0.5)
        return base

    return mesh.paint(color)


def rings():
    """Each band is a flat annulus; its shade varies a little from one narrow
    concentric strip to the next, like the fine ringlets of real rings."""
    generator = random.Random(2)
    pieces = []
    for inner, outer, color in RINGS:
        strips = max(1, round((outer - inner) / 0.06))
        radii = [inner + (outer - inner) * strip / strips for strip in range(strips + 1)]
        shades = [mix(color, hex_color("#ffffff") if generator.random() < 0.5 else hex_color("#5e4e3a"),
                      generator.uniform(0.0, 0.18)) for _ in radii]

        def shade(position, normal, radii=radii, shades=shades):
            radius = math.hypot(position[0], position[2])
            nearest = min(range(len(radii)), key=lambda i: abs(radii[i] - radius))
            return shades[nearest]

        band = lathe([(radius, 0.0) for radius in radii], 64).paint(shade)
        band.normals = [(0.0, 1.0, 0.0)] * len(band.positions)
        pieces.append(band)
    return merge(*pieces).rotate_x(RING_TILT)


def cratered_moon(radius, seed):
    """A faceted moon with bowl-shaped craters pressed into it, each with a
    raised rim and a darker floor."""
    generator = random.Random(seed)
    mesh = icosphere(1.0, 2)
    craters = []
    for _ in range(7):
        # A random direction, and an angular radius for the crater.
        z = generator.uniform(-1, 1)
        angle = generator.uniform(0, 2 * math.pi)
        around = math.sqrt(1 - z * z)
        craters.append(((around * math.cos(angle), around * math.sin(angle), z), generator.uniform(0.3, 0.55)))

    def angle_between(a, b):
        return math.acos(max(-1.0, min(1.0, sum(x * y for x, y in zip(a, b)))))

    def dent(p):
        length = math.sqrt(sum(v * v for v in p))
        unit = tuple(v / length for v in p)
        height = 1.0
        for center, size in craters:
            reach = angle_between(unit, center) / size
            if reach < 1.0:
                height -= 0.14 * (1 - reach * reach)  # the bowl
            elif reach < 1.35:
                height += 0.05 * math.sin((reach - 1.0) / 0.35 * math.pi)  # the rim
        return tuple(v * height * radius for v in unit)

    mesh.transform(dent)
    mesh = mesh.jitter(radius * 0.03, seed)

    def color(center, normal):
        length = math.sqrt(sum(v * v for v in center))
        unit = tuple(v / length for v in center)
        inside = any(angle_between(unit, c) < size * 0.8 for c, size in craters)
        base = hex_color("#76808f") if inside else hex_color("#cdd5df")
        return mix(base, hex_color("#ffffff"), generator.uniform(0.0, 0.12))

    return mesh.paint_faces(color)


def main():
    moon = cratered_moon(0.33, 5).translate(-1.6, 1.0, 0.45)
    mesh = merge(gas_giant(), rings(), moon)
    save(mesh, "planet")


if __name__ == "__main__":
    main()
