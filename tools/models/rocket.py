"""A retro 1950s rocket: a cream, lathe-turned body with a red nose, three
swept fins, round portholes, and a lobed flame out of the nozzle, among a
few twinkling stars.

Run from the repository root: python3 -I tools/models/rocket.py
"""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from meshkit import cylinder, extrude, hex_color, lathe, merge, mix, save, sphere, torus

CREAM = hex_color("#fbf3df")
RED = hex_color("#c8203a")
SILVER = hex_color("#b9c2cc")
STEEL = hex_color("#7d8794")
GLASS = hex_color("#4fd6f0")

# The body's profile as (radius, height) from the tail up to the nose tip.
BODY = [(0.0, 0.0), (0.34, 0.0), (0.45, 0.18), (0.54, 0.45), (0.6, 0.8), (0.62, 1.1),
        (0.6, 1.4), (0.54, 1.7), (0.44, 1.98), (0.31, 2.24), (0.17, 2.44), (0.07, 2.56),
        (0.0, 2.6)]
NOSE_START = 1.72   # the red nose cone begins here
BAND = (0.72, 0.84)  # a red belt around the middle
TAIL_BAND = (0.12, 0.24)
FIN_ANGLES = [math.pi / 2 + 2 * math.pi * i / 3 for i in range(3)]


def body_radius(y):
    for (r0, y0), (r1, y1) in zip(BODY[1:], BODY[2:]):
        if y0 <= y <= y1:
            return r0 + (r1 - r0) * (y - y0) / (y1 - y0)
    return 0.0


def with_cuts(profile, heights, gap=0.004):
    """Adds profile points just below and above each height, so a color
    change painted at that height stays crisp while normals stay smooth."""
    # The first point closes the tail at the axis; the rest climb in height.
    points = list(profile[1:])
    for h in heights:
        points += [(body_radius(y), y) for y in (h - gap, h + gap)]
    return [profile[0]] + sorted(points, key=lambda point: point[1])


def body():
    cuts = [NOSE_START, *BAND, *TAIL_BAND]
    mesh = lathe(with_cuts(BODY, cuts), 40, CREAM)

    def color(position, normal):
        y = position[1]
        if y > NOSE_START or BAND[0] < y < BAND[1] or TAIL_BAND[0] < y < TAIL_BAND[1]:
            return RED
        return CREAM

    return mesh.paint(color)


def fin(angle):
    """A swept fin that curves out and down past the tail like a landing leg."""
    outline = [(0.4, 1.0), (0.64, 0.86), (0.88, 0.56), (1.05, 0.14), (1.15, -0.3), (1.09, -0.4),
               (0.99, -0.36), (0.84, -0.06), (0.6, 0.1), (0.4, 0.12)]
    shape = extrude(outline, 0.1, RED)
    # Turn the fin's outward (+X) edge toward `angle` around the body.
    return shape.rotate_y(-angle)


def porthole(angle, y):
    radius = body_radius(y)
    rim = torus(0.15, 0.045, 20, 8, SILVER)
    glass = sphere(0.135, 16, 8, GLASS).scale(1, 0.35, 1)
    window = merge(rim, glass)
    # Lean with the body's slope, then face outward.
    slope = math.atan2(body_radius(y - 0.05) - body_radius(y + 0.05), 0.1)
    window.align((1.0, 0.0, 0.0)).rotate_z(slope).translate(radius - 0.01, y, 0)
    return window.rotate_y(-angle)


def nozzle():
    bell = lathe([(0.28, 0.02), (0.22, -0.05), (0.23, -0.13), (0.3, -0.24), (0.36, -0.32),
                  (0.33, -0.32)], 32, STEEL)
    collar = torus(0.3, 0.04, 32, 8, SILVER)
    return merge(bell, collar)


def flame():
    """A lobed, twisting flame. It fades from a white-hot core at the nozzle
    to orange and red at the tip, and its normals point up so every side is
    lit alike and it reads as glowing."""
    top, bottom = -0.22, -1.4
    profile = [(0.31, top), (0.36, -0.4), (0.33, -0.62), (0.26, -0.86), (0.17, -1.08), (0.08, -1.27),
               (0.0, bottom)]
    mesh = lathe(profile, 36)

    def lobe(p):
        angle = math.atan2(p[2], p[0])
        depth = (top - p[1]) / (top - bottom)
        wobble = 1 + 0.18 * depth * math.sin(6 * angle + 5 * depth)
        return (p[0] * wobble, p[1], p[2] * wobble)

    mesh.transform(lobe)
    stops = [(0.0, hex_color("#fffbe0")), (0.25, hex_color("#ffd84a")), (0.6, hex_color("#ff8c1a")),
             (1.0, hex_color("#e8401c"))]

    def gradient(position, normal):
        depth = (top - position[1]) / (top - bottom)
        for (t0, c0), (t1, c1) in zip(stops, stops[1:]):
            if depth <= t1:
                return mix(c0, c1, (depth - t0) / (t1 - t0))
        return stops[-1][1]

    mesh.paint(gradient)
    mesh.normals = [(0.0, 1.0, 0.0)] * len(mesh.positions)
    return mesh


def twinkle(size, color):
    """A thin four-pointed star. Its normals point up so it glows, and as
    the turntable turns it edge-on it thins away, like a twinkle."""
    outline = []
    for k in range(8):
        angle = math.pi / 2 + k * math.pi / 4
        radius = size if k % 2 == 0 else size * 0.32
        outline.append((radius * math.cos(angle), radius * math.sin(angle)))
    star = extrude(outline, size * 0.2, color)
    star.normals = [(0.0, 1.0, 0.0)] * len(star.positions)
    return star


def stars():
    # (position, size, turn about Y) for a few stars scattered around the rocket.
    scattered = [((1.25, 1.9, 0.2), 0.26, 0.3), ((-1.2, 0.85, -0.4), 0.2, 1.2),
                 ((0.95, -0.3, -0.8), 0.17, 2.1), ((-0.9, 2.2, 0.7), 0.15, 2.8)]
    return merge(*(twinkle(size, hex_color("#fff3a6")).rotate_y(turn).translate(*position)
                   for position, size, turn in scattered))


def antenna():
    mast = cylinder(0.03, 0.22, 8, SILVER).translate(0, 2.55, 0)
    tip = sphere(0.065, 12, 6, RED).translate(0, 2.8, 0)
    return merge(mast, tip)


def main():
    # Portholes sit between the fins.
    windows = [porthole(angle + math.pi / 3, 1.22) for angle in FIN_ANGLES]
    fins = [fin(angle) for angle in FIN_ANGLES]
    mesh = merge(body(), nozzle(), flame(), antenna(), stars(), *fins, *windows)
    save(mesh, "rocket")


if __name__ == "__main__":
    main()
