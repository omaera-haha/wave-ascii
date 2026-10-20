"""A red-and-white lighthouse on a rocky islet, with a keeper's cottage, a
glowing lantern room, and a little disk of sea ringed with foam.

Run from the repository root: python3 -I tools/models/lighthouse.py
"""

import math
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from meshkit import (box, cone, cylinder, extrude, hex_color, icosphere, lathe, merge, mix, save,
                     sphere, torus)

RED = hex_color("#b81c22")  # darker than the white, so stripes show in plain text
WHITE = hex_color("#f4f1ea")
DARK_RED = hex_color("#8f1d1d")
GLASS = hex_color("#ffd84a")
BEAM = hex_color("#f2e49a")
FRAME = hex_color("#5a4a46")
STONE = hex_color("#a8a29a")
SEA = hex_color("#2a9d9a")
DEEP_SEA = hex_color("#1d5f8a")
FOAM = hex_color("#f2fbff")
ROCKS = [hex_color(c) for c in ("#7d6e60", "#6b5d51", "#8a7b6c", "#5f544b", "#94877a")]
GRASS = [hex_color(c) for c in ("#6aa84f", "#5c9a45", "#7cb85c")]

# Heights along the tower, from the sea at y = 0 up.
TOWER_BOTTOM, TOWER_TOP = 0.62, 2.72
BOTTOM_RADIUS, TOP_RADIUS = 0.46, 0.32
DECK = 2.86
COTTAGE_ANGLE = 0.7  # around the tower, measured from +X toward +Z (the front)


def tower_radius(y):
    """The tower tapers linearly from its base to the gallery."""
    t = (y - TOWER_BOTTOM) / (TOWER_TOP - TOWER_BOTTOM)
    return BOTTOM_RADIUS + (TOP_RADIUS - BOTTOM_RADIUS) * t


def striped_tower():
    """Separate lathe bands so each stripe has a crisp edge."""
    bands = []
    stripes = 5
    for i in range(stripes):
        low = TOWER_BOTTOM + (TOWER_TOP - TOWER_BOTTOM) * i / stripes
        high = TOWER_BOTTOM + (TOWER_TOP - TOWER_BOTTOM) * (i + 1) / stripes
        color = WHITE if i % 2 == 0 else RED
        bands.append(lathe([(tower_radius(low), low), (tower_radius(high), high)], 36, color))
    return merge(*bands)


def on_tower(part, angle, y, inset=0.0):
    """Places a part, built facing +Z, against the tower wall at `angle`."""
    radius = tower_radius(y) - inset
    return part.translate(0, y, radius).rotate_y(angle)


def door():
    arch = [(-0.11, 0.0), (0.11, 0.0), (0.11, 0.2)]
    arch += [(0.11 * math.cos(a * math.pi / 6), 0.2 + 0.11 * math.sin(a * math.pi / 6)) for a in range(1, 6)]
    arch += [(-0.11, 0.2)]
    frame = extrude([(x * 1.35, y * 1.15 - 0.02) for x, y in arch], 0.05, WHITE)
    return merge(frame.translate(0, 0, -0.01), extrude(arch, 0.08, hex_color("#3d6fa8")))


def window():
    pane = box(0.11, 0.15, 0.08, hex_color("#2f4f7a"))
    sill = box(0.16, 0.035, 0.1, WHITE).translate(0, -0.09, 0.01)
    return merge(pane, sill)


def gallery():
    corbel = lathe([(TOP_RADIUS, TOWER_TOP), (TOP_RADIUS + 0.06, TOWER_TOP + 0.06),
                    (0.5, DECK - 0.04)], 36, DARK_RED)
    deck = cylinder(0.53, 0.06, 36, DARK_RED, smooth=False).translate(0, DECK - 0.04, 0)
    posts = []
    count = 14
    for i in range(count):
        angle = 2 * math.pi * i / count
        posts.append(box(0.04, 0.2, 0.04, WHITE).translate(0.5 * math.cos(angle), DECK + 0.12,
                                                          0.5 * math.sin(angle)))
    top_rail = torus(0.5, 0.032, 40, 8, WHITE).translate(0, DECK + 0.22, 0)
    middle_rail = torus(0.5, 0.018, 40, 6, WHITE).translate(0, DECK + 0.11, 0)
    return merge(corbel, deck, top_rail, middle_rail, *posts)


def lantern_room():
    wall = cylinder(0.3, 0.12, 8, WHITE, smooth=False).translate(0, DECK + 0.02, 0)
    glass_bottom = DECK + 0.14
    glass_height = 0.42
    glass = cylinder(0.27, glass_height, 8, GLASS, smooth=False).translate(0, glass_bottom, 0)
    # Normals that all point up light the glass evenly from every side of
    # the turntable, so it reads as glowing rather than shaded.
    glass.normals = [(0.0, 1.0, 0.0)] * len(glass.positions)
    # Mullions at the corners of the octagonal glass.
    bars = []
    for i in range(8):
        angle = 2 * math.pi * i / 8
        bar = box(0.035, glass_height, 0.045, FRAME).translate(0, glass_bottom + glass_height / 2, 0.275)
        bars.append(bar.rotate_y(angle + math.pi / 2))
    eave = cylinder(0.36, 0.05, 32, DARK_RED, smooth=False).translate(0, glass_bottom + glass_height, 0)
    roof_bottom = glass_bottom + glass_height + 0.05
    roof = lathe([(0.34, roof_bottom), (0.3, roof_bottom + 0.08), (0.2, roof_bottom + 0.2),
                  (0.08, roof_bottom + 0.29), (0.0, roof_bottom + 0.31)], 32, DARK_RED)
    vent = cylinder(0.06, 0.1, 12, DARK_RED).translate(0, roof_bottom + 0.27, 0)
    finial = sphere(0.07, 12, 8, hex_color("#e9b949")).translate(0, roof_bottom + 0.42, 0)
    spike = cone(0.025, 0.16, 8, hex_color("#e9b949")).translate(0, roof_bottom + 0.46, 0)
    return merge(wall, glass, eave, roof, vent, finial, spike, *bars)


def cottage():
    """A keeper's cottage with a gabled roof and a chimney, built at the origin."""
    walls = box(0.56, 0.32, 0.4, WHITE).translate(0, 0.16, 0)
    gable = extrude([(-0.2, 0.0), (0.2, 0.0), (0.0, 0.18)], 0.56, WHITE).rotate_y(math.pi / 2)
    gable.translate(0, 0.32, 0)
    # Two roof slabs resting on the gable.
    slope = math.atan2(0.18, 0.2)
    length = math.hypot(0.2, 0.18) + 0.06
    roof = []
    for side in (-1, 1):
        slab = box(0.66, 0.045, length, hex_color("#b3261e")).rotate_x(side * slope)
        roof.append(slab.translate(0, 0.32 + 0.09 + 0.03, side * 0.1))
    chimney = box(0.09, 0.22, 0.09, STONE).translate(0.17, 0.5, 0.06)
    door_panel = box(0.1, 0.17, 0.03, hex_color("#3d6fa8")).translate(-0.12, 0.085, 0.2)
    panes = [box(0.1, 0.1, 0.03, hex_color("#2f4f7a")).translate(0.1, 0.17, 0.2),
             box(0.03, 0.1, 0.1, hex_color("#2f4f7a")).translate(0.28, 0.17, 0.0),
             box(0.1, 0.1, 0.03, hex_color("#2f4f7a")).translate(0.0, 0.17, -0.2)]
    return merge(walls, gable, chimney, door_panel, *roof, *panes)


def boulder(radius, squash, seed, detail=1):
    rock = icosphere(1.0, detail).scale(radius, radius * squash, radius)
    return rock.jitter(radius * 0.18, seed)


def islet():
    """A flattened mound of jittered rocks; upward-facing high faces get grass."""
    generator = random.Random(7)
    rocks = [boulder(0.85, 0.5, 1).translate(0, 0.12, 0),
             boulder(0.5, 0.55, 2).translate(0.75 * math.cos(COTTAGE_ANGLE), 0.08,
                                             0.75 * math.sin(COTTAGE_ANGLE))]
    for i in range(8):
        angle = 2 * math.pi * i / 8 + generator.uniform(-0.25, 0.25)
        distance = generator.uniform(0.72, 0.88)
        size = generator.uniform(0.13, 0.23)
        rocks.append(boulder(size, generator.uniform(0.6, 0.9), 10 + i, 0)
                     .translate(distance * math.cos(angle), 0.03, distance * math.sin(angle)))
    mesh = merge(*rocks)
    colors = random.Random(3)

    def color(center, normal):
        if normal[1] > 0.8 and center[1] > 0.3:
            return colors.choice(GRASS)
        return colors.choice(ROCKS)

    return mesh.paint_faces(color)


def sea():
    """A low-poly disk of rippled water with a deep-blue skirt, like a diorama."""
    rings, segments = 4, 32
    inner, outer = 0.5, 1.45
    radii = [inner + (outer - inner) * r / rings for r in range(rings + 1)]
    mesh = lathe([(radius, 0.0) for radius in radii], segments, SEA, smooth=False)

    # Ripples: lift vertices with a couple of interfering waves.
    def ripple(p):
        angle = math.atan2(p[2], p[0])
        radius = math.hypot(p[0], p[2])
        return (p[0], 0.035 * math.sin(radius * 9 + angle * 3) + 0.025 * math.sin(angle * 7), p[2])

    mesh.transform(ripple)
    shades = random.Random(5)
    mesh.paint_faces(lambda center, normal: mix(SEA, hex_color("#5cc8c0"), shades.random() * 0.6))
    skirt = lathe([(outer, 0.0), (outer, -0.18), (outer - 0.04, -0.2), (0.0, -0.2)], segments, DEEP_SEA,
                  smooth=False)
    return merge(mesh, skirt)


def foam():
    """A lumpy white ring where the islet meets the sea, plus a few loose flecks."""
    ring_mesh = torus(0.93, 0.07, 24, 4, FOAM, smooth=False).scale(1, 0.6, 1).jitter(0.04, 4)
    flecks = []
    generator = random.Random(11)
    for i in range(7):
        angle = generator.uniform(0, 2 * math.pi)
        distance = generator.uniform(1.08, 1.38)
        fleck = icosphere(0.06, 0, FOAM).scale(1.6, 0.4, 1.0).rotate_y(generator.uniform(0, 3))
        flecks.append(fleck.translate(distance * math.cos(angle), 0.03, distance * math.sin(angle)))
    return merge(ring_mesh.make_flat(), *flecks)


def lighthouse():
    plinth = cylinder(0.6, 0.24, 10, STONE, smooth=False).translate(0, TOWER_BOTTOM - 0.24, 0)
    plinth_lip = cylinder(0.64, 0.05, 10, mix(STONE, WHITE, 0.4), smooth=False)
    plinth_lip.translate(0, TOWER_BOTTOM - 0.03, 0)
    details = [
        on_tower(door(), 0.0, TOWER_BOTTOM + 0.02, 0.02),
        on_tower(window(), math.pi, 1.2, 0.02),
        on_tower(window(), math.pi / 2, 1.75, 0.02),
        on_tower(window(), -math.pi / 2, 1.45, 0.02),
        on_tower(window(), 0.0, 2.2, 0.02),
    ]
    return merge(plinth, plinth_lip, striped_tower(), gallery(), lantern_room(), *details)


def light_beams():
    """Two pale cones of light shining out of the lantern on opposite sides,
    so the turntable sweeps them around like a working lighthouse."""
    beams = []
    for direction in (1, -1):
        beam = cylinder(0.08, 0.8, 16, BEAM, caps=False, top_radius=0.24).translate(0, 0.2, 0)
        beam.align((direction, 0.0, 0.0))
        # Fade from pale yellow at the lantern to dim gold at the open end.
        beam.paint(lambda p, n: mix(BEAM, hex_color("#4d4218"), (abs(p[0]) - 0.2) / 0.8))
        # Upward normals give an even, glowing look like the lantern glass.
        beam.normals = [(0.0, 1.0, 0.0)] * len(beam.positions)
        beams.append(beam.translate(0, DECK + 0.35, 0))
    return merge(*beams)


def sailboat():
    """A little dinghy with a white sail, built along X at the origin."""
    hull = lathe([(0.0, -1.0), (0.55, -0.8), (0.9, -0.4), (1.0, 0.0)], 14, hex_color("#f2a541"), smooth=False)
    hull.scale(0.26, 0.11, 0.1).translate(0, 0.08, 0)
    trim = torus(1.0, 0.08, 14, 3, WHITE, smooth=False).scale(0.26, 0.3, 0.1).translate(0, 0.08, 0)
    mast = cylinder(0.018, 0.42, 6, hex_color("#8a5a3c")).translate(0.03, 0.06, 0)
    sail = extrude([(0.0, 0.0), (0.22, 0.0), (0.0, 0.36)], 0.025, WHITE).translate(0.05, 0.12, 0)
    jib = extrude([(0.0, 0.0), (-0.16, 0.0), (0.0, 0.3)], 0.02, hex_color("#e63946")).translate(0.01, 0.12, 0)
    return merge(hull, trim, mast, sail, jib)


def main():
    # Turn the cottage so its long side faces outward from the tower.
    house = cottage().scale(1.3).rotate_y(-(COTTAGE_ANGLE + math.pi / 2))
    house.translate(0.84 * math.cos(COTTAGE_ANGLE), 0.3, 0.84 * math.sin(COTTAGE_ANGLE))
    boat_angle = COTTAGE_ANGLE + 2.6
    # The boat sails around the islet, so it points along the shore.
    boat = sailboat().scale(1.4).rotate_y(-(boat_angle + math.pi / 2))
    boat.translate(1.15 * math.cos(boat_angle), 0.0, 1.15 * math.sin(boat_angle))
    mesh = merge(sea(), foam(), islet(), lighthouse(), house, boat, light_beams())
    save(mesh, "lighthouse")


if __name__ == "__main__":
    main()
