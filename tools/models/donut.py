"""A glazed donut with a bite taken out: golden dough, pink icing with drips
down the outside, a scatter of colorful sprinkles, and a crumbly bite mark."""

import math
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from meshkit import Mesh, cylinder, hex_color, merge, mix, save  # noqa: E402

MAJOR = 1.0       # distance from the center of the hole to the middle of the dough
MINOR = 0.46      # radius of the dough tube
ICING = 0.075     # how far the icing stands off the dough
FLATTEN = 0.82    # donuts are a little squat

# The bite is a vertical cylinder standing BITE_DISTANCE from the donut's
# center, at angle 0 around the hole. Its radius dips between teeth, so the
# bitten edge is scalloped like a real bite mark.
BITE_DISTANCE = 1.75
BITE_RADIUS = 1.22
TOOTH_WIDTH = 0.42   # radians around the bite's axis
TOOTH_DEPTH = 0.12

DOUGH_DARK = hex_color("#b9733a")
DOUGH = hex_color("#d9984f")
DOUGH_BAND = hex_color("#f1cc8c")   # the pale ring where the donut floated in the oil
CRUMB = hex_color("#ffc47c")
ICING_PINK = hex_color("#ff7fb8")
SPRINKLE_COLORS = [hex_color(c) for c in ("#ffe347", "#46d8ff", "#ffffff", "#86f25a", "#ff9638", "#b27dff")]


def surface(u, v, radius):
    """A point on a torus of tube radius `radius`, at angle `u` around the
    hole and `v` around the tube (0 is the outer equator, pi/2 the top)."""
    ring = MAJOR + radius * math.cos(v)
    return (ring * math.cos(u), radius * math.sin(v) * FLATTEN, -ring * math.sin(u))


def bite_reach(direction):
    """The bite's radius in a direction (an angle around the bite's axis).
    Each tooth leaves a round scallop; they meet in points."""
    x = ((direction - math.pi) / TOOTH_WIDTH + 0.5) % 1.0 * 2 - 1
    return BITE_RADIUS - TOOTH_DEPTH * (1 - math.sqrt(1 - x * x))


def bitten(x, z):
    """How far inside the bite a point is; positive means eaten."""
    dx = x - BITE_DISTANCE
    return bite_reach(math.atan2(z, dx)) - math.hypot(dx, z)


def bite_angle(distance):
    """The angle around the hole where a circle of the given distance from
    the center leaves the bite. Everything closer to angle 0 is eaten."""
    def eaten(u):
        return bitten(distance * math.cos(u), -distance * math.sin(u)) > 0

    step = 0.01
    u = 0.0
    while eaten(u + step):
        u += step
    low, high = u, u + step
    for _ in range(20):
        middle = (low + high) / 2
        low, high = (middle, high) if eaten(middle) else (low, middle)
    return low


def uneaten(fraction, distance):
    """Maps 0..1 onto the arc of the ring at `distance` that is left."""
    start = bite_angle(distance)
    return start + (2 * math.pi - 2 * start) * fraction


def onto_bite(point):
    """Pushes a point straight out from the bite's axis onto its surface."""
    x, y, z = point
    dx = x - BITE_DISTANCE
    direction = math.atan2(z, dx)
    reach = bite_reach(direction)
    return (BITE_DISTANCE + reach * math.cos(direction), y, reach * math.sin(direction))


def tube_grid(rings):
    """Stitches a list of closed rings (lists of points) into a smooth tube."""
    mesh = Mesh()
    sides = len(rings[0])
    for ring in rings:
        mesh.positions += ring
    for r in range(len(rings) - 1):
        for s in range(sides):
            a = r * sides + s
            b = r * sides + (s + 1) % sides
            mesh.triangles += [(a, a + sides, b), (b, a + sides, b + sides)]
    mesh.normals = [(0.0, 0.0, 0.0)] * len(mesh.positions)
    mesh.colors = [(1.0, 1.0, 1.0)] * len(mesh.positions)
    return mesh.make_smooth()


def sheet_grid(rows):
    """Stitches rows of points (open at both ends) into a smooth sheet."""
    mesh = Mesh()
    columns = len(rows[0])
    for row in rows:
        mesh.positions += row
    for r in range(len(rows) - 1):
        for c in range(columns - 1):
            a = r * columns + c
            mesh.triangles += [(a, a + columns, a + 1), (a + 1, a + columns, a + columns + 1)]
    mesh.normals = [(0.0, 0.0, 0.0)] * len(mesh.positions)
    mesh.colors = [(1.0, 1.0, 1.0)] * len(mesh.positions)
    return mesh.make_smooth()


def make_drips(generator):
    """Random drips around the outside: (angle, half width, length). None
    hang where the bite is."""
    drips = []
    count = 13
    for i in range(count):
        angle = 2 * math.pi * (i + generator.uniform(-0.3, 0.3)) / count
        if 0.75 < angle < 2 * math.pi - 0.75:
            drips.append((angle, generator.uniform(0.11, 0.19), generator.uniform(0.25, 0.75)))
    return drips


def angle_difference(a, b):
    return (a - b + math.pi) % (2 * math.pi) - math.pi


def outer_edge(u, drips):
    """The icing's lower boundary on the outside, as a tube angle. The base
    edge wobbles gently; each drip hangs a rounded tongue below it."""
    edge = 0.32 + 0.07 * math.sin(3 * u + 1.0) + 0.04 * math.sin(7 * u)
    for angle, width, length in drips:
        x = angle_difference(u, angle) / width
        if abs(x) < 1:
            edge -= length * (1 - x * x) ** 0.55
        # A shallow fillet where the drip leaves the main edge.
        if abs(x) < 2:
            edge -= 0.08 * (1 - (x / 2) ** 2) ** 2
    return edge


def inner_edge(u):
    """The icing's boundary on the inner lip, so the hole shows bare dough."""
    return 2.45 + 0.12 * math.sin(5 * u + 0.4)


def dough(columns=48, sides=16):
    """The torus of dough, swept around the hole from one side of the bite
    to the other, with crumb filling both cut ends."""
    rings = []
    for c in range(columns + 1):
        ring = []
        for s in range(sides):
            v = 2 * math.pi * s / sides
            u = uneaten(c / columns, MAJOR + MINOR * math.cos(v))
            ring.append(surface(u, v, MINOR))
        rings.append(ring)
    mesh = tube_grid(rings)

    def color(position, normal):
        # Darker on the top and bottom, a pale band around the outer waist.
        height = abs(position[1]) / (MINOR * FLATTEN)
        outside = math.hypot(position[0], position[2]) > MAJOR
        if outside and height < 0.35:
            return mix(DOUGH_BAND, DOUGH, (height / 0.35) ** 2)
        return mix(DOUGH, DOUGH_DARK, min(1.0, height * 1.2))

    mesh.paint(color)
    return merge(mesh, crumb(rings[0], 1), crumb(rings[-1], 2))


def crumb(boundary, seed, layers=4):
    """Fills a cut end of the dough with a rough, pale crumb surface that
    follows the curve of the bite."""
    generator = random.Random(seed)
    middle = tuple(sum(p[i] for p in boundary) / len(boundary) for i in range(3))
    rings = []
    for layer in range(layers + 1):
        shrink = 1 - layer / layers
        ring = []
        for point in boundary:
            inside = tuple(m + (p - m) * shrink for p, m in zip(point, middle))
            if 0 < layer < layers:
                inside = tuple(value + generator.uniform(-0.022, 0.022) for value in inside)
            ring.append(onto_bite(inside))
        rings.append(ring)
    mesh = tube_grid(rings)
    mesh.paint_faces(lambda center, normal: mix(CRUMB, DOUGH_BAND, generator.uniform(0.0, 0.3)))
    return mesh


def icing(drips, columns=112):
    """A shell of glaze over the top of the dough, between the drippy outer
    edge and the inner lip."""
    # Rows are packed near both edges so the rim of the icing is rounded.
    stations = [0.0, 0.02, 0.06, 0.12, 0.21, 0.33, 0.46, 0.59, 0.71, 0.81, 0.89, 0.95, 0.985, 1.0]

    def point(fraction, s, lift):
        # The angle around the hole depends on how far out the point is,
        # which depends on the angle; a few rounds of iteration settle it.
        u = uneaten(fraction, MAJOR)
        for _ in range(4):
            v = outer_edge(u, drips) + (inner_edge(u) - outer_edge(u, drips)) * s
            radius = MINOR + lift
            u = uneaten(fraction, MAJOR + radius * math.cos(v))
        return surface(u, v, radius)

    def thickness(s):
        # The glaze rises quickly from its edge, like a bead.
        return -0.01 + (ICING + 0.01) * min(1.0, min(s, 1 - s) / 0.08) ** 0.5

    rows = [[point(c / columns, s, thickness(s)) for c in range(columns + 1)] for s in stations]
    top = sheet_grid(rows)
    # Close the glaze where it was bitten, so it shows as a pink layer.
    ends = []
    for fraction in (0.0, 1.0):
        ends.append(sheet_grid([
            [point(fraction, s, thickness(s)) for s in stations],
            [point(fraction, s, -0.01) for s in stations],
        ]))
    return merge(top, *ends).paint(ICING_PINK)


def sprinkles(drips, generator, count=50):
    """Short rods scattered over the icing, spaced apart, none in the bite."""
    meshes = []
    placed = []
    attempts = 0
    while len(meshes) < count and attempts < 5000:
        attempts += 1
        u = generator.uniform(0, 2 * math.pi)
        if not 0.95 < u < 2 * math.pi - 0.95:
            continue
        v = generator.uniform(outer_edge(u, drips) + 0.3, inner_edge(u) - 0.3)
        v = max(v, 0.3)
        point = surface(u, v, MINOR + ICING + 0.012)
        if any(math.dist(point, other) < 0.2 for other in placed):
            continue
        placed.append(point)
        # Lay the sprinkle flat in the icing's tangent plane at a random angle.
        along_u = (-math.sin(u), 0.0, -math.cos(u))
        along_v = (-math.sin(v) * math.cos(u), math.cos(v) * FLATTEN, math.sin(v) * math.sin(u))
        turn = generator.uniform(0, math.pi)
        axis = tuple(math.cos(turn) * a + math.sin(turn) * b for a, b in zip(along_u, along_v))
        length = generator.uniform(0.15, 0.2)
        sprinkle = cylinder(0.037, length, 6, generator.choice(SPRINKLE_COLORS))
        meshes.append(sprinkle.translate(0, -length / 2, 0).align(axis).translate(*point))
    return merge(*meshes)


def build():
    generator = random.Random(7)
    drips = make_drips(generator)
    donut = merge(dough(), icing(drips), sprinkles(drips, generator))
    # Turn the bite to the right, where it notches the first frame's outline.
    return donut.rotate_y(5.6)


if __name__ == "__main__":
    save(build(), "donut")
