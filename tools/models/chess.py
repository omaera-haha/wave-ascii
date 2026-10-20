"""A few chess pieces on a small checkered board: a turned king, queen, rook,
bishop, and pawn from lathe profiles and a knight from a horse-head
silhouette, cream against mid-brown wood."""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from meshkit import Mesh, box, extrude, hex_color, lathe, merge, save  # noqa: E402

CREAM = hex_color("#ffe2b0")
WOOD = hex_color("#9a5b2e")
LIGHT_SQUARE = hex_color("#e9e4c4")
DARK_SQUARE = hex_color("#5f9a48")
FRAME = hex_color("#7a4a28")

SQUARE = 0.54         # width of one square; the pieces nearly fill it
SQUARES = 3           # the board is SQUARES x SQUARES
PIECE_SCALE = 1.15    # chunkier than real pieces, so they read at low resolution
BOARD_TOP = 0.0
SEGMENTS = 16


def arc(center_r, center_y, radius, start, end, steps):
    """Points on a circle in the profile plane, from angle `start` to `end`
    (degrees, 0 pointing outward, 90 up)."""
    points = []
    for i in range(steps + 1):
        angle = math.radians(start + (end - start) * i / steps)
        points.append((max(0.0, center_r + radius * math.cos(angle)), center_y + radius * math.sin(angle)))
    return points


def turned(profile, color):
    """Revolves a profile. A point given as (r, y, True) is a sharp corner:
    it is doubled so the shading breaks there."""
    points = []
    for entry in profile:
        points.append(entry[:2])
        if len(entry) > 2 and entry[2]:
            points.append(entry[:2])
    return lathe(points, SEGMENTS, color)


def base(radius=0.2):
    """The common foot: a disk with a bead and a cove above it."""
    return [(0.0, 0.0), (radius, 0.0, True), (radius, 0.05, True), (radius * 0.92, 0.065),
            (radius * 0.88, 0.09), (radius * 0.8, 0.105, True), (radius * 0.66, 0.12)]


def pawn(color):
    """A ball on a short waisted stem with a collar."""
    profile = base(0.18) + [(0.085, 0.2), (0.07, 0.28), (0.115, 0.3, True), (0.115, 0.32, True), (0.065, 0.34)]
    profile += arc(0.0, 0.43, 0.1, -55, 90, 7)
    return turned(profile, color)


def rook(color):
    """A tower with a crenellated top: a hollow cup with four merlons."""
    profile = base(0.21) + [(0.135, 0.2), (0.125, 0.44), (0.16, 0.47), (0.185, 0.5, True), (0.185, 0.64, True),
                            (0.115, 0.64, True), (0.115, 0.58, True), (0.0, 0.58)]
    body = turned(profile, color)
    merlons = []
    for i in range(4):
        middle = math.pi / 2 * i + math.pi / 4
        merlons.append(annular_block(0.115, 0.185, 0.635, 0.75, middle - 0.42, middle + 0.42, color))
    return merge(body, *merlons)


def annular_block(inner, outer, low, high, start, end, color, steps=4):
    """A closed slice of a thick ring, for the rook's merlons."""
    loop = [(inner, low), (outer, low), (outer, low), (outer, high), (outer, high),
            (inner, high), (inner, high), (inner, low)]
    ring = lathe(loop, steps, color, start=start, sweep=end - start)
    caps = []
    for angle in (start, end):
        # A flat end wall in the XY plane, turned to stand at `angle`.
        cap = Mesh()
        cap.positions = [(inner, low, 0.0), (outer, low, 0.0), (outer, high, 0.0), (inner, high, 0.0)]
        cap.triangles = [(0, 1, 2), (0, 2, 3)]
        cap.normals = [(0.0, 0.0, 1.0)] * 4
        cap.colors = [color] * 4
        caps.append(cap.rotate_y(angle))
    return merge(ring, *caps)


def bishop(color):
    """A slim stem and collar under a tall, pointed mitre with a little ball."""
    profile = base(0.2) + [(0.1, 0.22), (0.075, 0.42), (0.125, 0.45, True), (0.125, 0.47, True),
                           (0.085, 0.49), (0.11, 0.52), (0.135, 0.6), (0.13, 0.68), (0.1, 0.76), (0.05, 0.82),
                           (0.03, 0.84, True)]
    profile += arc(0.0, 0.88, 0.045, -50, 90, 4)
    return turned(profile, color)


def queen(color):
    """A tall stem flaring into a crown ringed with beads, and a ball on top."""
    profile = base(0.21) + [(0.11, 0.24), (0.08, 0.52), (0.135, 0.56, True), (0.135, 0.58, True),
                            (0.095, 0.6), (0.11, 0.66), (0.17, 0.78, True), (0.15, 0.79), (0.08, 0.8),
                            (0.06, 0.84)]
    profile += arc(0.0, 0.9, 0.065, -60, 90, 5)
    body = turned(profile, color)
    beads = []
    for i in range(8):
        angle = 2 * math.pi * i / 8
        bead = lathe(arc(0.0, 0.0, 0.04, -90, 90, 3), 6, color)
        beads.append(bead.translate(0.15 * math.cos(angle), 0.81, -0.15 * math.sin(angle)))
    return merge(body, *beads)


def king(color):
    """The tallest piece: a flared crown topped with a cross."""
    profile = base(0.22) + [(0.12, 0.25), (0.09, 0.58), (0.145, 0.62, True), (0.145, 0.645, True),
                            (0.1, 0.665), (0.12, 0.72), (0.155, 0.84, True), (0.13, 0.86), (0.07, 0.88, True),
                            (0.07, 0.9, True), (0.0, 0.9)]
    body = turned(profile, color)
    cross = merge(box(0.075, 0.24, 0.075, color).translate(0, 1.0, 0),
                  box(0.2, 0.07, 0.075, color).translate(0, 1.02, 0))
    return merge(body, cross)


def inset(polygon, distance):
    """Moves each corner of a counterclockwise polygon inward by `distance`,
    mitered so the edges stay parallel."""
    count = len(polygon)
    result = []
    for i in range(count):
        before, here, after = polygon[i - 1], polygon[i], polygon[(i + 1) % count]
        normals = []
        for a, b in ((before, here), (here, after)):
            dx, dy = b[0] - a[0], b[1] - a[1]
            length = math.hypot(dx, dy)
            normals.append((-dy / length, dx / length))
        nx, ny = normals[0][0] + normals[1][0], normals[0][1] + normals[1][1]
        length = math.hypot(nx, ny)
        nx, ny = nx / length, ny / length
        miter = min(2.0, 1 / max(0.1, nx * normals[0][0] + ny * normals[0][1]))
        result.append((here[0] + nx * distance * miter, here[1] + ny * distance * miter))
    return result


def bevelled_slab(polygon, half_thickness, bevel, color):
    """A thick silhouette with chamfered edges, so its faces catch the light
    at different angles. `half_thickness(x, y)` lets it thin out in places."""
    inner = inset(polygon, bevel)
    # The flat faces: an extrusion of the inset outline, pushed out to the
    # full thickness at each corner.
    core = extrude(inner, 2.0, color)
    core.transform(lambda p: (p[0], p[1], p[2] * half_thickness(p[0], p[1])))
    # The chamfers and the narrow outer wall.
    mesh = Mesh()
    count = len(polygon)
    for side in (1, -1):
        for i in range(count):
            j = (i + 1) % count
            a, b = polygon[i], polygon[j]
            c, d = inner[j], inner[i]
            outer_a = (a[0], a[1], side * 0.5 * half_thickness(*a))
            outer_b = (b[0], b[1], side * 0.5 * half_thickness(*b))
            inner_c = (c[0], c[1], side * half_thickness(*c))
            inner_d = (d[0], d[1], side * half_thickness(*d))
            start = len(mesh.positions)
            mesh.positions += [outer_a, outer_b, inner_c, inner_d]
            mesh.triangles += [(start, start + 1, start + 2), (start, start + 2, start + 3)]
    for i in range(count):
        j = (i + 1) % count
        a, b = polygon[i], polygon[j]
        start = len(mesh.positions)
        mesh.positions += [(a[0], a[1], 0.5 * half_thickness(*a)), (b[0], b[1], 0.5 * half_thickness(*b)),
                           (b[0], b[1], -0.5 * half_thickness(*b)), (a[0], a[1], -0.5 * half_thickness(*a))]
        mesh.triangles += [(start, start + 1, start + 2), (start, start + 2, start + 3)]
    mesh.normals = [(0.0, 0.0, 0.0)] * len(mesh.positions)
    mesh.colors = [color] * len(mesh.positions)
    return merge(core, mesh).make_flat()


def knight(color):
    """A horse's head cut from a silhouette, chamfered and thickest at the
    neck, on a turned foot. It faces +X."""
    # Counterclockwise from the front of the chest: throat, jaw, muzzle,
    # forehead, ear, then down the mane.
    head = [(-0.15, 0.1), (0.15, 0.1), (0.16, 0.2), (0.12, 0.3), (0.06, 0.38), (0.1, 0.44), (0.2, 0.46),
            (0.3, 0.48), (0.33, 0.53), (0.31, 0.58), (0.2, 0.66), (0.12, 0.74), (0.1, 0.84), (0.06, 0.9),
            (0.01, 0.8), (-0.06, 0.78), (-0.14, 0.7), (-0.19, 0.58), (-0.21, 0.44), (-0.2, 0.28)]

    def half_thickness(x, y):
        return max(0.035, 0.1 - 0.12 * max(0.0, x - 0.08) - 0.3 * max(0.0, y - 0.76))

    neck = bevelled_slab(head, half_thickness, 0.03, color)
    foot = turned(base(0.2) + [(0.0, 0.12)], color)
    return merge(foot, neck.translate(-0.02, 0, 0))


def board():
    """The squares, set on a slab of darker wood that frames them."""
    parts = []
    for row in range(SQUARES):
        for column in range(SQUARES):
            color = LIGHT_SQUARE if (row + column) % 2 else DARK_SQUARE
            tile = box(SQUARE, 0.06, SQUARE, color)
            parts.append(tile.translate((column - (SQUARES - 1) / 2) * SQUARE, BOARD_TOP - 0.03,
                                        (row - (SQUARES - 1) / 2) * SQUARE))
    edge = SQUARE * SQUARES + 0.2
    parts.append(box(edge, 0.16, edge, FRAME).translate(0, BOARD_TOP - 0.09, 0))
    return merge(*parts)


def place(piece, column, row, turn=0.0):
    """Stands a piece on a square, turned `turn` radians about its axis."""
    x = (column - (SQUARES - 1) / 2) * SQUARE
    z = (row - (SQUARES - 1) / 2) * SQUARE
    return piece.scale(PIECE_SCALE).rotate_y(turn).translate(x, BOARD_TOP, z)


def build():
    return merge(
        board(),
        place(king(CREAM), 1, 1),
        place(rook(CREAM), 0, 0, 0.2),
        place(pawn(CREAM), 2, 0),
        place(queen(WOOD), 2, 2),
        place(knight(WOOD), 0, 2, -0.2),
        place(bishop(WOOD), 1, 0),
    )


if __name__ == "__main__":
    save(build(), "chess")
