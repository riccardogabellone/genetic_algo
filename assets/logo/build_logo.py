"""Generate the logo SVG masters from a parametric description of the source art.

Usage (from assets/logo/):  uv run --with pillow python build_logo.py

The source raster (source/logo-src.png) is flat geometry: a navy rounded tile
holding 19 circles and diamonds on a 5x5 grid, in a band around the
anti-diagonal (cells where |row - (4 - col)| <= 2). Tracing it would turn exact
circles into wobbly splines, so instead each shape was measured once (centre,
size, median colour; see AGENTS.md) and is redrawn here as an exact primitive.

Two deliberate changes from the source:
- shapes sit on a uniform square grid (the source drifts: column pitch
  182-210 px, row pitch 169-176 px) and the composition is centred on the tile;
- a separate small-size variant redraws the same band on a 3x3 grid, because at
  16-32 px 19 shapes blur into noise.

Everything is written in a 1024-unit coordinate space.
"""

import math
from pathlib import Path

HERE = Path(__file__).resolve().parent

CANVAS = 1024
NAVY = "#00163D"  # sampled tile median
# Source tile: 1205 px wide with a ~250 px corner radius, i.e. radius = 0.207 * side.
TILE_RADIUS_RATIO = 0.207
# The source shapes are measured in source pixels; the source tile is 1205 px wide.
SOURCE_TILE = 1205
PITCH = 190  # source px between neighbouring cell centres, both axes
DIAMOND_CORNER_RATIO = 0.07  # corner radius / side, the source diamonds are slightly soft

# (col, row, kind, size in source px, colour). Size is the circle diameter or the
# diamond's tip-to-tip width. Row 0 is the top.
CELLS = [
    (2, 0, "circle", 75, "#F6D222"),
    (3, 0, "diamond", 116, "#FECC15"),
    (4, 0, "diamond", 199, "#FE9A0D"),
    (1, 1, "circle", 85, "#AAE271"),
    (2, 1, "circle", 109, "#C7E14F"),
    (3, 1, "diamond", 158, "#F7D526"),
    (4, 1, "diamond", 145, "#FED51B"),
    (0, 2, "circle", 113, "#01D6C3"),
    (1, 2, "circle", 107, "#54DC9F"),
    (2, 2, "circle", 135, "#83E088"),
    (3, 2, "diamond", 154, "#BFE052"),
    (4, 2, "diamond", 122, "#F9D625"),
    (0, 3, "circle", 127, "#00D1D2"),
    (1, 3, "circle", 138, "#1DD6B3"),
    (2, 3, "diamond", 140, "#3EDAA6"),
    (3, 3, "diamond", 128, "#8FE07D"),
    (0, 4, "circle", 146, "#00D1E0"),
    (1, 4, "circle", 133, "#00D0D8"),
    (2, 4, "circle", 103, "#01D3C9"),
]
# The 16-32 px icon: the same band at 3x3 (cells where |row - (2 - col)| <= 1), fewer
# and larger shapes, sizes growing towards the fittest corner. Same colours as above.
SMALL_PITCH = 300
SMALL_CELLS = [
    (1, 0, "diamond", 190, "#FECC15"),
    (2, 0, "diamond", 250, "#FE9A0D"),
    (0, 1, "circle", 170, "#54DC9F"),
    (1, 1, "circle", 190, "#83E088"),
    (2, 1, "diamond", 220, "#F9D625"),
    (0, 2, "circle", 180, "#00D1E0"),
    (1, 2, "circle", 170, "#1DD6B3"),
]


def fmt(value):
    text = f"{value:.2f}".rstrip("0").rstrip(".")
    return "0" if text in ("", "-0") else text


def shape_svg(kind, cx, cy, size, colour):
    if kind == "circle":
        return f'<circle cx="{fmt(cx)}" cy="{fmt(cy)}" r="{fmt(size / 2)}" fill="{colour}"/>'
    # A square rotated 45 degrees. Rounding a corner by rx pulls each tip in by
    # rx * (sqrt(2) - 1), so the side is solved for the requested tip-to-tip width.
    side = size / (math.sqrt(2) + DIAMOND_CORNER_RATIO * (2 - math.sqrt(2)))
    rx = side * DIAMOND_CORNER_RATIO
    return (
        f'<rect x="{fmt(cx - side / 2)}" y="{fmt(cy - side / 2)}" width="{fmt(side)}" '
        f'height="{fmt(side)}" rx="{fmt(rx)}" fill="{colour}" '
        f'transform="rotate(45 {fmt(cx)} {fmt(cy)})"/>'
    )


def layout(cells, scale, centre, pitch=PITCH):
    """Places cells on the uniform grid, centring the drawn extent (not the grid) on `centre`."""
    placed = [(col * pitch, row * pitch, kind, size, colour) for col, row, kind, size, colour in cells]
    left = min(x - size / 2 for x, _, _, size, _ in placed)
    right = max(x + size / 2 for x, _, _, size, _ in placed)
    top = min(y - size / 2 for _, y, _, size, _ in placed)
    bottom = max(y + size / 2 for _, y, _, size, _ in placed)
    dx = centre - (left + right) / 2 * scale
    dy = centre - (top + bottom) / 2 * scale
    return [
        shape_svg(kind, dx + x * scale, dy + y * scale, size * scale, colour)
        for x, y, kind, size, colour in placed
    ]


def tile_svg(inset):
    side = CANVAS - 2 * inset
    radius = side * TILE_RADIUS_RATIO
    return (
        f'<rect x="{fmt(inset)}" y="{fmt(inset)}" width="{fmt(side)}" height="{fmt(side)}" '
        f'rx="{fmt(radius)}" fill="{NAVY}"/>'
    )


def write(name, body):
    lines = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {CANVAS} {CANVAS}">']
    lines += [f"  {element}" for element in body]
    lines.append("</svg>")
    (HERE / name).write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"{name}: {len(body)} elements")


def icon(inset, cells, pitch=PITCH):
    """A tile inset from the canvas edge, with the cells scaled to it like the source."""
    side = CANVAS - 2 * inset
    scale = side / SOURCE_TILE
    return [tile_svg(inset), *layout(cells, scale, CANVAS / 2, pitch)]


# Full-bleed tile: Linux hicolor, Windows .ico, the window icon.
write("logo.svg", icon(0, CELLS))
# macOS draws no mask of its own: Apple's grid puts an 824 px tile in a 1024 canvas.
write("logo-macos.svg", icon(100, CELLS))
# Fewer, larger cells so 16-32 px icons stay legible.
write("logo-small.svg", icon(0, SMALL_CELLS, SMALL_PITCH))
# The mark alone, transparent, for use inside the app on any background.
write("logo-mark.svg", layout(CELLS, CANVAS / (5 * PITCH + 20), CANVAS / 2))
