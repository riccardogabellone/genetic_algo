"""Measure every shape in the source raster: the numbers behind CELLS in build_logo.py.

Usage (from assets/logo/):  uv run --with pillow python measure_source.py

Prints the tile's extent and corner profile, then one line per shape: kind,
centre, bounding box, fill ratio and core colour. Run it only when the source
changes; the results are copied into build_logo.py by hand, so the masters never
depend on a measurement run.

- A shape pixel is any pixel clearly away from the navy tile colour, at least
  MARGIN px inside the image (this skips the white paper around the tile).
- Shapes are 4-connected components larger than MIN_AREA px.
- Kind comes from how much of its bounding box a shape fills: a circle fills
  pi/4 = 0.785 and a diamond 0.5 (a little more for soft corners).
- Colour is the median over the central 30% of the shape, away from antialiased edges.
"""

import statistics
from collections import deque
from pathlib import Path

from PIL import Image

HERE = Path(__file__).resolve().parent
SOURCE = HERE / "source/logo-src.png"
NAVY = (1, 22, 61)
PAPER = (254, 254, 254)
MARGIN = 60
MIN_AREA = 200
SHAPE_DISTANCE = 60  # RGB distance from navy above which a pixel belongs to a shape


def distance_squared(a, b):
    return sum((x - y) ** 2 for x, y in zip(a, b))


image = Image.open(SOURCE).convert("RGB")
width, height = image.size
pixels = image.load()

# Tile extent along the middle row and column, then where each row near the top starts.
in_tile = lambda x, y: distance_squared(pixels[x, y], PAPER) > distance_squared(pixels[x, y], NAVY)
columns = [x for x in range(width) if in_tile(x, height // 2)]
rows = [y for y in range(height) if in_tile(width // 2, y)]
print(f"tile x {columns[0]}..{columns[-1]}  y {rows[0]}..{rows[-1]}")
for offset in (0, 10, 40, 80, 120, 160, 200, 240):
    starts = [x for x in range(width) if in_tile(x, rows[0] + offset)]
    print(f"  row top+{offset}: starts at x {starts[0]}")

is_shape = [
    [
        MARGIN < x < width - MARGIN
        and MARGIN < y < height - MARGIN
        and distance_squared(pixels[x, y], NAVY) > SHAPE_DISTANCE**2
        for x in range(width)
    ]
    for y in range(height)
]
seen = [[False] * width for _ in range(height)]
shapes = []
for y in range(height):
    for x in range(width):
        if not is_shape[y][x] or seen[y][x]:
            continue
        queue, points = deque([(x, y)]), []
        seen[y][x] = True
        while queue:
            px, py = queue.popleft()
            points.append((px, py))
            for nx, ny in ((px + 1, py), (px - 1, py), (px, py + 1), (px, py - 1)):
                if 0 <= nx < width and 0 <= ny < height and is_shape[ny][nx] and not seen[ny][nx]:
                    seen[ny][nx] = True
                    queue.append((nx, ny))
        if len(points) > MIN_AREA:
            shapes.append(points)

lines = []
for points in shapes:
    xs, ys = [p[0] for p in points], [p[1] for p in points]
    box_w, box_h = max(xs) - min(xs) + 1, max(ys) - min(ys) + 1
    cx, cy = sum(xs) / len(xs), sum(ys) / len(ys)
    fill = len(points) / (box_w * box_h)
    core_radius = min(box_w, box_h) * 0.3
    core = [pixels[p] for p in points if (p[0] - cx) ** 2 + (p[1] - cy) ** 2 < core_radius**2]
    colour = tuple(int(statistics.median(c[i] for c in core)) for i in range(3))
    if distance_squared(colour, PAPER) < SHAPE_DISTANCE**2:
        continue  # paper showing through the tile's rounded corners, not a shape
    kind = "circle" if fill > 0.65 else "diamond"
    lines.append((cy, cx, kind, box_w, box_h, fill, colour))
for cy, cx, kind, box_w, box_h, fill, (r, g, b) in sorted(lines):
    print(
        f"{kind:7s} centre ({cx:6.1f}, {cy:6.1f})  {box_w}x{box_h}  fill {fill:.3f}  #{r:02X}{g:02X}{b:02X}"
    )
print(f"{len(lines)} shapes")
