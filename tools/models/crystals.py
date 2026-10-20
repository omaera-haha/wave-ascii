"""A gem cluster: faceted hexagonal crystals with pointed tips, shading from
violet through pink to cyan, growing at different angles out of a rough rock."""

import math
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from meshkit import hex_color, icosphere, lathe, merge, mix, save  # noqa: E402

ROCK_DARK = hex_color("#5f564f")
ROCK_LIGHT = hex_color("#a99a88")

# Half extents of the rock, an ellipsoid before it is roughened.
ROCK_WIDTH, ROCK_HEIGHT, ROCK_DEPTH = 1.25, 0.68, 1.1

# (base color, tip color) pairs; each crystal blends from one to the other.
VIOLET = (hex_color("#7c46ff"), hex_color("#e8b4ff"))
PINK = (hex_color("#ff3cae"), hex_color("#ffc8f0"))
CYAN = (hex_color("#14c4ff"), hex_color("#d2fcff"))
LILAC = (hex_color("#a35cff"), hex_color("#ffb4ec"))
TEAL = (hex_color("#3f78ff"), hex_color("#a4f4ff"))

# Each crystal: direction of its root from the rock's center (radians around
# Y), distance of the root from the center, lean outward from vertical,
# length, radius, palette. Roots further out lean further, like a sea urchin.
CRYSTALS = [
    (0.0, 0.0, 0.08, 1.85, 0.32, VIOLET),
    (0.5, 0.3, 0.45, 1.45, 0.24, PINK),
    (2.2, 0.32, 0.5, 1.5, 0.25, CYAN),
    (3.9, 0.3, 0.42, 1.35, 0.23, LILAC),
    (5.3, 0.35, 0.55, 1.3, 0.22, TEAL),
    (1.35, 0.55, 0.78, 1.05, 0.19, VIOLET),
    (3.05, 0.6, 0.85, 1.0, 0.18, PINK),
    (4.6, 0.62, 0.8, 1.05, 0.19, CYAN),
    (6.0, 0.62, 0.9, 0.9, 0.17, LILAC),
    (0.05, 0.85, 1.1, 0.7, 0.14, TEAL),
    (0.95, 0.9, 1.15, 0.6, 0.13, PINK),
    (1.9, 0.85, 1.05, 0.65, 0.14, LILAC),
    (2.65, 0.95, 1.2, 0.55, 0.12, VIOLET),
    (3.6, 0.9, 1.1, 0.6, 0.13, CYAN),
    (4.25, 0.95, 1.2, 0.5, 0.12, PINK),
    (5.0, 0.9, 1.15, 0.6, 0.13, VIOLET),
    (5.7, 0.95, 1.2, 0.5, 0.12, CYAN),
]


def crystal(length, radius, palette, generator):
    """A hexagonal prism standing on y = 0 with a six-sided pointed tip. The
    sides taper slightly and the point sits a little off center, as real
    crystals rarely grow perfectly."""
    shoulder = length - radius * 1.25
    # Extra rings along the sides give bands of color, like zoning in quartz.
    sides = [(radius * (1.04 - 0.04 * f), shoulder * f) for f in (0.0, 0.35, 0.65, 1.0)]
    profile = [(0.0, -0.4), (radius * 1.08, -0.4)] + sides + [(0.0, length)]
    mesh = lathe(profile, segments=6, smooth=False, start=generator.uniform(0, math.pi / 3))
    apex_shift = (generator.uniform(-0.25, 0.25) * radius, generator.uniform(-0.25, 0.25) * radius)
    mesh.transform(lambda p: (p[0] + apex_shift[0], p[1], p[2] + apex_shift[1]) if p[1] > length - 1e-6 else p)
    base, tip = palette

    def color(center, normal):
        # Darker at the root, glowing toward the tip; facets alternate a
        # little in brightness so the prism reads as cut rather than round.
        height = max(0.0, center[1]) / length
        shade = mix(base, tip, height ** 1.3)
        facet = math.atan2(normal[2], normal[0])
        if normal[1] > 0.3:
            return mix(shade, (1.0, 1.0, 1.0), 0.3)
        return tuple(channel * (0.9 + 0.1 * math.cos(3 * facet)) for channel in shade)

    return mesh.paint_faces(color)


def rock(generator):
    """A lumpy, flat-bottomed boulder."""
    mesh = icosphere(1.0, 2).scale(ROCK_WIDTH, ROCK_HEIGHT, ROCK_DEPTH).jitter(0.09, seed=4)

    def flatten(p):
        # Squash the underside so the rock sits on a flat bottom.
        return (p[0], max(p[1], -0.4), p[2])

    mesh.transform(flatten)
    mesh.make_flat()

    def color(center, normal):
        x, y, z = center
        mottle = 0.5 + 0.25 * math.sin(3.1 * x + 1.3 * z) + 0.25 * math.sin(2.3 * z - 4.0 * y + 0.7)
        grain = generator.uniform(-0.08, 0.08)
        return mix(ROCK_DARK, ROCK_LIGHT, max(0.0, min(1.0, mottle + grain)))

    return mesh.paint_faces(color)


def surface_height(x, z):
    """Roughly where the top of the rock is, to plant crystals in it."""
    inside = 1 - (x / ROCK_WIDTH) ** 2 - (z / ROCK_DEPTH) ** 2
    return ROCK_HEIGHT * math.sqrt(max(inside, 0.0))


def druse(generator, count=11):
    """Little crystals poking out of the rock's sides, along its surface."""
    palettes = [VIOLET, PINK, CYAN, LILAC, TEAL]
    nubs = []
    for i in range(count):
        around = 2 * math.pi * (i + generator.uniform(-0.3, 0.3)) / count
        height = generator.uniform(-0.05, 0.35)
        # A point on the rock's ellipsoid and its outward normal.
        spread = math.sqrt(max(0.0, 1 - (height / ROCK_HEIGHT) ** 2))
        x, z = ROCK_WIDTH * spread * math.cos(around), ROCK_DEPTH * spread * math.sin(around)
        normal = (x / ROCK_WIDTH ** 2, height / ROCK_HEIGHT ** 2, z / ROCK_DEPTH ** 2)
        tilt = (generator.uniform(-0.3, 0.3), 0.4, generator.uniform(-0.3, 0.3))
        direction = tuple(n + t for n, t in zip(normal, tilt))
        nub = crystal(generator.uniform(0.2, 0.32), generator.uniform(0.06, 0.08), generator.choice(palettes),
                      generator)
        nubs.append(nub.align(direction).translate(x * 0.97, height, z * 0.97))
    return nubs


def build():
    generator = random.Random(11)
    parts = [rock(generator)]
    for around, distance, lean, length, radius, palette in CRYSTALS:
        x, z = distance * math.cos(around), distance * math.sin(around)
        heading = around + generator.uniform(-0.25, 0.25)
        direction = (math.sin(lean) * math.cos(heading), math.cos(lean), math.sin(lean) * math.sin(heading))
        gem = crystal(length, radius, palette, generator).align(direction)
        parts.append(gem.translate(x, surface_height(x, z) - 0.12, z))
    parts += druse(generator)
    return merge(*parts)


if __name__ == "__main__":
    save(build(), "crystals")
