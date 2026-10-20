"""A classic flying saucer: a paneled metal disc with a glassy cyan dome, a
ring of colored running lights, and a tractor beam of glowing hoops lifting
a very surprised cow.

Run from the repository root: python3 -I tools/models/ufo.py
"""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from meshkit import cone, cylinder, hex_color, lathe, merge, mix, save, sphere, torus, tube

SILVER = hex_color("#cfd6e0")
STEEL = hex_color("#8f9aab")
DARK_STEEL = hex_color("#5d6878")
GLASS = hex_color("#5fe0f2")
BEAM = hex_color("#b8ffcf")
LIGHTS = [hex_color(c) for c in ("#ff4d5e", "#ffd23f", "#6dff7a", "#4dd8ff", "#ff6bd6")]

# The hull's profile as (radius, height), from the belly out to the rim and
# back in over the top to the dome's seat.
HULL = [(0.0, -0.26), (0.32, -0.25), (0.42, -0.22), (0.7, -0.17), (1.0, -0.1), (1.24, -0.03),
        (1.36, 0.02), (1.36, 0.07), (1.28, 0.12), (1.08, 0.2), (0.86, 0.27), (0.62, 0.33),
        (0.5, 0.35)]
RIM = 1.36


def glow(mesh):
    """Points every normal up, so the turntable lights every side alike and
    the part reads as glowing rather than shaded."""
    mesh.normals = [(0.0, 1.0, 0.0)] * len(mesh.positions)
    return mesh


def hull():
    mesh = lathe(HULL, 48, SILVER, smooth=False)

    def panels(center, normal):
        x, y, z = center
        radius = math.hypot(x, z)
        if y < -0.05:
            # The belly: darker, with a ring of panels around the beam emitter.
            return DARK_STEEL if radius < 0.45 or 0.75 < radius < 0.95 else STEEL
        if y < 0.1:
            return hex_color("#e8edf4")  # the bright rim band
        # Concentric bands on the top, alternating light and mid silver.
        return SILVER if int((radius - 0.5) / 0.14) % 2 == 0 else hex_color("#aab4c3")

    mesh.paint_faces(panels)
    return smooth_lathe_normals(mesh)


def smooth_lathe_normals(mesh):
    """Keeps the painted panels' colors but smooths shading around the axis:
    each corner's normal is the average of the faces at the same height
    and radius, turned to its own angle."""
    sums = {}
    keys = [(round(math.hypot(p[0], p[2]), 4), round(p[1], 4)) for p in mesh.positions]
    for a, b, c in mesh.triangles:
        # After make_flat, every corner of a face holds the face's normal.
        face = mesh.normals[a]
        corner = mesh.positions[a]
        angle = math.atan2(corner[2], corner[0])
        # Express the face normal in (outward, up) terms for this angle.
        outward = face[0] * math.cos(angle) + face[2] * math.sin(angle)
        for i in (a, b, c):
            total = sums.get(keys[i], (0.0, 0.0))
            sums[keys[i]] = (total[0] + outward, total[1] + face[1])
    normals = []
    for p, key in zip(mesh.positions, keys):
        outward, up = sums[key]
        length = math.hypot(outward, up) or 1.0
        angle = math.atan2(p[2], p[0])
        normals.append((outward / length * math.cos(angle), up / length, outward / length * math.sin(angle)))
    mesh.normals = normals
    return mesh


def dome():
    # A slightly flattened half sphere, seated on the hull.
    quarter = [(0.5 * math.cos(math.pi / 2 * k / 8), 0.43 * math.sin(math.pi / 2 * k / 8)) for k in range(9)]
    glass = lathe(quarter, 32, GLASS).translate(0, 0.34, 0)
    seat = torus(0.5, 0.05, 32, 6, SILVER).translate(0, 0.35, 0)
    mast = cylinder(0.025, 0.2, 8, SILVER).translate(0, 0.74, 0)
    bulb = glow(sphere(0.06, 12, 6, hex_color("#ff4d5e"))).translate(0, 0.96, 0)
    return merge(glass, seat, mast, bulb)


def running_lights():
    count = 12
    lights = []
    for i in range(count):
        angle = 2 * math.pi * i / count
        bulb = glow(sphere(0.085, 8, 4, LIGHTS[i % len(LIGHTS)]))
        lights.append(bulb.translate(RIM * math.cos(angle), 0.045, RIM * math.sin(angle)))
    return merge(*lights)


def emitter():
    """A glowing lens on the belly, where the beam comes out."""
    return glow(sphere(0.3, 16, 6, BEAM).scale(1, 0.3, 1)).translate(0, -0.26, 0)


def tractor_beam():
    """Glowing hoops that widen and fade as they go down, so the cow stays
    in view between them."""
    hoops = []
    count = 6
    for k in range(count):
        depth = k / (count - 1)
        y = -0.4 - 1.15 * depth
        radius = 0.38 + 0.55 * depth
        color = mix(hex_color("#e9fff0"), hex_color("#3f8f6a"), depth)
        hoops.append(glow(torus(radius, 0.035, 32, 4, color)).translate(0, y, 0))
    return merge(*hoops)


def cow():
    """A small black-and-white cow, built facing +X with its back up."""
    white = hex_color("#f7f5f0")
    patch = hex_color("#3b3533")
    spots = [((0.05, 0.08, 0.1), 0.09), ((-0.12, 0.03, -0.11), 0.08), ((0.14, -0.02, -0.1), 0.06),
             ((-0.15, 0.1, 0.06), 0.07)]

    def spotted(position, normal):
        for center, size in spots:
            if math.dist(position, center) < size:
                return patch
        return white

    body = sphere(1.0, 16, 10).scale(0.24, 0.13, 0.14).paint(spotted)
    head = sphere(1.0, 12, 8, white).scale(0.1, 0.095, 0.085).translate(0.27, 0.07, 0)
    snout = sphere(1.0, 10, 6, hex_color("#f2a0a8")).scale(0.065, 0.055, 0.075).translate(0.35, 0.04, 0)
    parts = [body, head, snout]
    for side in (-1, 1):
        parts.append(cone(0.022, 0.08, 6, hex_color("#efe2b8")).rotate_x(side * -0.5)
                     .translate(0.26, 0.14, side * 0.05))
        parts.append(sphere(1.0, 8, 6, patch).scale(0.03, 0.02, 0.05).translate(0.25, 0.1, side * 0.1))
        parts.append(sphere(0.018, 6, 4, hex_color("#2b2b2b")).translate(0.345, 0.11, side * 0.04))
    # Legs splay a little, as if kicking.
    for x, z, splay in ((0.13, 0.08, 0.3), (0.13, -0.08, -0.3), (-0.13, 0.08, 0.25), (-0.13, -0.08, -0.25)):
        leg = cylinder(0.034, 0.17, 6, white).translate(0, -0.17, 0)
        hoof = cylinder(0.037, 0.04, 6, patch).translate(0, -0.2, 0)
        parts.append(merge(leg, hoof).rotate_x(splay).translate(x, -0.03, z))
    tail = tube([(-0.23, 0.05, 0.0), (-0.3, 0.02, 0.0), (-0.33, -0.08, 0.0)], 0.014, 6, white)
    tuft = sphere(0.03, 8, 4, patch).translate(-0.33, -0.1, 0.0)
    parts += [tail, tuft]
    return merge(*parts)


def main():
    lifted_cow = cow().scale(1.7).rotate_z(0.35).rotate_x(-0.25).rotate_y(0.5).translate(0, -1.12, 0)
    mesh = merge(hull(), dome(), running_lights(), emitter(), tractor_beam(), lifted_cow)
    # A slight tilt makes the saucer wobble as it turns, like it's hovering.
    mesh.rotate_z(0.1)
    save(mesh, "ufo")


if __name__ == "__main__":
    main()
