"""A hot-air balloon with bright vertical gores, a zigzag band, ropes, a
burner, and a wicker basket carrying two passengers, drifting past a couple
of puffy low-poly clouds.

Run from the repository root: python3 -I tools/models/balloon.py
"""

import math
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from meshkit import cone, cylinder, hex_color, icosphere, lathe, merge, mix, save, sphere, tube

RED = hex_color("#d7263d")
YELLOW = hex_color("#ffc83d")
BLUE = hex_color("#2f5fb3")
WHITE = hex_color("#fff6e3")
NAVY = hex_color("#2a3f8f")
SKIRT_COLOR = hex_color("#a8452f")
GORES = [RED, YELLOW, BLUE, YELLOW] * 4
WICKER = hex_color("#b9834a")
WICKER_DARK = hex_color("#8a5a2e")
ROPE = hex_color("#d9c8a5")

# The envelope's profile as (radius, height), from the mouth up to the crown.
ENVELOPE = [(0.24, 0.0), (0.3, 0.14), (0.42, 0.34), (0.56, 0.56), (0.7, 0.8), (0.82, 1.04),
            (0.92, 1.3), (0.98, 1.56), (1.0, 1.8), (0.98, 2.04), (0.92, 2.27), (0.81, 2.48),
            (0.65, 2.65), (0.45, 2.78), (0.22, 2.86), (0.0, 2.89)]
ZIGZAG = (1.12, 0.15)  # the band's middle height and half its height
CROWN = 2.7  # above this the envelope is a solid red cap
SKIRT = 0.16  # below this, the mouth's darker skirt


def dense_profile(step=0.1):
    """Resamples the profile so painted patterns have rows to follow."""
    points = [ENVELOPE[0]]
    for (r0, y0), (r1, y1) in zip(ENVELOPE, ENVELOPE[1:]):
        count = max(1, math.ceil(math.hypot(r1 - r0, y1 - y0) / step))
        points += [(r0 + (r1 - r0) * k / count, y0 + (y1 - y0) * k / count) for k in range(1, count + 1)]
    return points


def weld_normals(mesh, center):
    """Smooth normals for a flat-colored mesh: every corner at the same spot
    gets the average of the faces around it, oriented away from `center`,
    so painted edges stay crisp while the surface shades smoothly."""
    sums = {}
    keys = [tuple(round(v, 5) for v in p) for p in mesh.positions]
    for a, b, c in mesh.triangles:
        pa, pb, pc = mesh.positions[a], mesh.positions[b], mesh.positions[c]
        u = [pb[i] - pa[i] for i in range(3)]
        v = [pc[i] - pa[i] for i in range(3)]
        face = (u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0])
        middle = [(pa[i] + pb[i] + pc[i]) / 3 - center[i] for i in range(3)]
        if sum(face[i] * middle[i] for i in range(3)) < 0:
            face = tuple(-x for x in face)
        for i in (a, b, c):
            total = sums.get(keys[i], (0.0, 0.0, 0.0))
            sums[keys[i]] = tuple(total[k] + face[k] for k in range(3))
    normals = []
    for key in keys:
        n = sums[key]
        length = math.sqrt(sum(x * x for x in n)) or 1.0
        normals.append(tuple(x / length for x in n))
    mesh.normals = normals
    return mesh


def compact(mesh):
    """Merges corners that share a position, normal, and color, which flat
    painting had split apart, to keep the file small."""
    index = {}
    positions, normals, colors, remap = [], [], [], []
    for p, n, c in zip(mesh.positions, mesh.normals, mesh.colors):
        key = (tuple(round(v, 5) for v in p), tuple(round(v, 4) for v in n), tuple(round(v, 4) for v in c))
        if key not in index:
            index[key] = len(positions)
            positions.append(p)
            normals.append(n)
            colors.append(c)
        remap.append(index[key])
    mesh.positions, mesh.normals, mesh.colors = positions, normals, colors
    mesh.triangles = [tuple(remap[i] for i in t) for t in mesh.triangles]
    return mesh


def envelope():
    gores = len(GORES)
    per_gore = 3
    mesh = lathe(dense_profile(), gores * per_gore, smooth=False)

    # Puff each gore out between its seams, as the fabric bulges under pressure.
    def bulge(p):
        angle = math.atan2(-p[2], p[0]) % (2 * math.pi)
        across = (angle * gores / (2 * math.pi)) % 1.0
        swell = 1 + 0.035 * math.sin(math.pi * across)
        return (p[0] * swell, p[1], p[2] * swell)

    mesh.transform(bulge)
    middle, half = ZIGZAG

    def color(center, normal):
        x, y, z = center
        angle = math.atan2(-z, x) % (2 * math.pi)
        gore = int(angle * gores / (2 * math.pi)) % gores
        if y > CROWN:
            return RED
        if y < SKIRT:
            return SKIRT_COLOR
        # A zigzag with one tooth per gore: white below the line, blue above.
        across = (angle * gores / (2 * math.pi)) % 1.0
        line = middle + half * 0.8 * (abs(2 * across - 1) * 2 - 1)
        if abs(y - middle) < half:
            return WHITE if y < line else NAVY
        return GORES[gore]

    mesh.paint_faces(color)
    return compact(weld_normals(mesh, (0.0, 1.5, 0.0)))


def burner():
    frame = cylinder(0.2, 0.04, 16, hex_color("#6f7782")).translate(0, -0.44, 0)
    can = cylinder(0.08, 0.14, 12, hex_color("#a7b0ba")).translate(0, -0.42, 0)
    flame = cone(0.07, 0.3, 12, hex_color("#ff9b2a"), base=False).translate(0, -0.28, 0)
    flame.paint(lambda p, n: mix(hex_color("#fff3b0"), hex_color("#ff6a1f"), (p[1] + 0.28) / 0.3))
    flame.normals = [(0.0, 1.0, 0.0)] * len(flame.positions)  # evenly lit, so it glows
    return merge(frame, can, flame)


def basket():
    """A square wicker basket: alternating rows of light and dark weave,
    with a thick rim and a floor."""
    bottom, top, half = -0.98, -0.62, 0.23
    rows = 6
    # A four-sided lathe, turned so its sides face the axes, flaring slightly.
    profile = [(half * math.sqrt(2) * (0.92 + 0.08 * row / rows), bottom + (top - bottom) * row / rows)
               for row in range(rows + 1)]
    sides = lathe(profile, 4, start=math.pi / 4, smooth=False)
    sides.paint_faces(lambda center, normal: WICKER if int((center[1] - bottom) / (top - bottom) * rows) % 2
                      else WICKER_DARK)
    floor = lathe([(0.0, bottom), (half * math.sqrt(2) * 0.92, bottom)], 4, WICKER_DARK, start=math.pi / 4,
                  smooth=False)
    rim_radius = half * math.sqrt(2) + 0.02
    rim = lathe([(rim_radius - 0.06, top), (rim_radius + 0.02, top - 0.01), (rim_radius + 0.02, top + 0.05),
                 (rim_radius - 0.06, top + 0.05), (rim_radius - 0.06, top)], 4, hex_color("#6e4524"),
                start=math.pi / 4, smooth=False)
    return merge(sides, floor, rim)


def passengers():
    skin = hex_color("#f2c48d")
    people = []
    for x, z, hat in ((-0.08, 0.04, hex_color("#2f5fb3")), (0.09, -0.05, hex_color("#3aa655"))):
        head = sphere(0.075, 10, 6, skin).translate(x, -0.5, z)
        cap = sphere(0.08, 10, 6, hat).scale(1, 0.55, 1).translate(x, -0.465, z)
        jacket = cylinder(0.07, 0.14, 10, hat).translate(x, -0.64, z)
        people += [head, cap, jacket]
    return merge(*people)


def ropes():
    lines = []
    for k in range(4):
        angle = math.pi / 4 + k * math.pi / 2
        start = (0.25 * math.cos(angle), 0.02, 0.25 * math.sin(angle))
        end = (0.21 * math.cos(angle) * math.sqrt(2), -0.6, 0.21 * math.sin(angle) * math.sqrt(2))
        lines.append(tube([start, end], 0.022, 6, ROPE))
    return merge(*lines)


def cloud(seed, size):
    """A puffy cloud of faceted balls with a flattened bottom."""
    generator = random.Random(seed)
    puffs = [(0.0, 0.0, 0.0, 1.0), (0.85, -0.12, 0.1, 0.72), (-0.8, -0.15, -0.05, 0.7),
             (0.35, 0.35, -0.1, 0.75), (-0.3, 0.3, 0.15, 0.65)]
    balls = []
    for x, y, z, radius in puffs:
        ball = icosphere(radius, 1, mix(hex_color("#ffffff"), hex_color("#d6e2f0"), generator.random() * 0.6))
        ball.jitter(radius * 0.08, generator.randrange(1000))
        # Flatten everything below the cloud's base.
        ball.transform(lambda p, y=y: (p[0], max(p[1], -0.35 - y), p[2]))
        balls.append(ball.translate(x, y, z).make_flat())
    return merge(*balls).scale(size)


def main():
    balloon = merge(envelope(), burner(), basket(), passengers(), ropes())
    clouds = [cloud(1, 0.3).rotate_y(0.4).translate(1.3, 1.05, 0.35),
              cloud(2, 0.24).rotate_y(2.0).translate(-1.15, 0.05, -0.6)]
    save(merge(balloon, *clouds), "balloon")


if __name__ == "__main__":
    main()
