#!/usr/bin/env python3
"""Render the BachelorPad+ icon set from one geometry definition.

The mark is defined once, as a list of flat primitives in a 256 x 256 unit
space. The same list is written out as the master SVG and rasterised with
Pillow (4x supersampling, LANCZOS downscale; a box filter for the
pixel-grid variants) for every PNG size, so the SVG
and the PNGs cannot drift apart. Small sizes use a simplified variant whose
geometry sits on the target pixel grid (the usual practice for 16/24 px).

Usage:  python3 assets/icon/render.py
Needs:  Python 3 and Pillow (no SVG rasteriser required).

Original work for BachelorPad+, licensed MIT OR Apache-2.0.
"""

from __future__ import annotations

import io
import os
import shutil
import struct
import sys

from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
APP_ID = "io.github.dboles99.BachelorPad"
PNG_SIZES = [16, 24, 32, 48, 64, 128, 256, 512]
ICO_SIZES = [16, 24, 32, 48, 64, 128, 256]
PREVIEW = os.path.join(ROOT, "artifacts", "ui", "icon-preview-2026-09-25.png")
SS = 4  # supersampling factor

# Palette: taken from the product's own themes (crates/bp-theme).
INK = "#1b1b1b"     # Light theme ink: outlines, slot, feet
BODY = "#b8541f"    # Light theme accent: the appliance housing
PAPER = "#f7f6f3"   # Light theme panel: the sheet being processed
LAMP = "#3ddc6a"    # Terminal theme accent: the "ready" status light


# --------------------------------------------------------------------------
# Primitives. Coordinates are in the 256-unit space. A stroke is centred on
# the outline, as in SVG, and is painted over the fill.

def rrect(x, y, w, h, r, fill, stroke=None, sw=0):
    return ("rrect", dict(x=x, y=y, w=w, h=h, r=r, fill=fill, stroke=stroke, sw=sw))


def circle(cx, cy, r, fill, stroke=None, sw=0):
    return ("circle", dict(cx=cx, cy=cy, r=r, fill=fill, stroke=stroke, sw=sw))


def poly(points, r, fill, stroke=None, sw=0):
    """A closed polygon whose corners are rounded by radius r (quadratic)."""
    return ("poly", dict(points=points, r=r, fill=fill, stroke=stroke, sw=sw))


# --------------------------------------------------------------------------
# The mark: a squat, splay-footed text-processing appliance, a sheet of
# paper feeding into its top slot, a readout window and two status lights.

def master():
    s = 10  # outline weight
    return [
        # feet
        rrect(44, 196, 40, 38, 8, BODY, INK, s),
        rrect(172, 196, 40, 38, 8, BODY, INK, s),
        # the sheet, entering the slot
        rrect(76, 22, 104, 110, 6, PAPER, INK, s),
        rrect(96, 48, 64, 10, 5, INK),
        rrect(96, 70, 48, 10, 5, INK),
        rrect(96, 92, 56, 10, 5, INK),
        # the housing: a trapezoid, wider at the base
        poly([(36, 116), (220, 116), (240, 212), (16, 212)], 22, BODY, INK, s),
        # the intake slot
        rrect(60, 112, 136, 20, 10, INK),
        # the readout window with a lit cursor
        rrect(42, 148, 92, 44, 10, INK),
        rrect(56, 160, 14, 20, 3, LAMP),
        # status lights
        circle(166, 170, 14, LAMP, INK, 8),
        circle(206, 170, 14, PAPER, INK, 8),
    ]


# The small variants are drawn on the target pixel grid: coordinates below
# are in pixels and scaled by u = 256 / size. A one-pixel stroke is centred
# on a half pixel so that it lands on exactly one pixel row or column.

def small16():
    u = 256 / 16
    return [
        rrect(3 * u, 13 * u, 3 * u, 3 * u, 0, BODY),          # feet
        rrect(10 * u, 13 * u, 3 * u, 3 * u, 0, BODY),
        rrect(4.5 * u, 1.5 * u, 7 * u, 7 * u, 0, PAPER, INK, u),
        rrect(6 * u, 3 * u, 4 * u, u, 0, INK),                # text lines
        rrect(6 * u, 5 * u, 3 * u, u, 0, INK),
        rrect(0.5 * u, 7.5 * u, 15 * u, 6 * u, 1.5 * u, BODY, INK, u),
        rrect(2 * u, 9 * u, 5 * u, 3 * u, 0, INK),            # window
        rrect(3 * u, 10 * u, u, u, 0, LAMP),                  # cursor
        rrect(9 * u, 9 * u, 2 * u, 2 * u, 0, LAMP),           # lights
        rrect(12 * u, 9 * u, 2 * u, 2 * u, 0, PAPER),
    ]


def small24():
    u = 256 / 24
    return [
        rrect(3 * u, 19 * u, 4 * u, 3 * u, 0.5 * u, BODY),
        rrect(17 * u, 19 * u, 4 * u, 3 * u, 0.5 * u, BODY),
        rrect(6.5 * u, 1.5 * u, 11 * u, 11 * u, 0, PAPER, INK, u),
        rrect(8 * u, 4 * u, 7 * u, u, 0, INK),
        rrect(8 * u, 6 * u, 5 * u, u, 0, INK),
        rrect(8 * u, 8 * u, 6 * u, u, 0, INK),
        rrect(0.5 * u, 10.5 * u, 23 * u, 9 * u, 2 * u, BODY, INK, u),
        rrect(5 * u, 10 * u, 14 * u, 2 * u, 0.5 * u, INK),    # slot
        rrect(3 * u, 13 * u, 8 * u, 4 * u, 0.5 * u, INK),     # window
        rrect(4 * u, 14 * u, 2 * u, 2 * u, 0, LAMP),
        rrect(14 * u, 13 * u, 3 * u, 3 * u, 1 * u, LAMP),     # lights
        rrect(19 * u, 13 * u, 3 * u, 3 * u, 1 * u, PAPER),
    ]


def small32():
    u = 256 / 32
    return [
        rrect(5.5 * u, 24.5 * u, 5 * u, 5 * u, 1 * u, BODY, INK, u),
        rrect(21.5 * u, 24.5 * u, 5 * u, 5 * u, 1 * u, BODY, INK, u),
        rrect(9.5 * u, 2.5 * u, 13 * u, 14 * u, 0.5 * u, PAPER, INK, u),
        rrect(12 * u, 5 * u, 8 * u, 2 * u, 0.5 * u, INK),
        rrect(12 * u, 8 * u, 6 * u, 2 * u, 0.5 * u, INK),
        rrect(12 * u, 11 * u, 7 * u, 2 * u, 0.5 * u, INK),
        rrect(1.5 * u, 14.5 * u, 29 * u, 12 * u, 3 * u, BODY, INK, u),
        rrect(7 * u, 14 * u, 18 * u, 3 * u, 1.5 * u, INK),    # slot
        rrect(4 * u, 18 * u, 12 * u, 6 * u, 1.5 * u, INK),    # window
        rrect(6 * u, 19 * u, 2 * u, 4 * u, 0.5 * u, LAMP),
        rrect(18 * u, 19 * u, 4 * u, 4 * u, 1 * u, LAMP),   # lights
        rrect(24 * u, 19 * u, 4 * u, 4 * u, 1 * u, PAPER),
    ]


def variant(size):
    if size <= 16:
        return small16()
    if size <= 24:
        return small24()
    if size <= 32:
        return small32()
    return master()


# --------------------------------------------------------------------------
# Geometry helpers shared by the SVG writer and the rasteriser.

def _rounded_poly_segments(points, r):
    """Yield (start, control, end) for each rounded corner, in order."""
    n = len(points)
    segs = []
    for i in range(n):
        p0, p1, p2 = points[i - 1], points[i], points[(i + 1) % n]

        def toward(a, b, d):
            dx, dy = b[0] - a[0], b[1] - a[1]
            L = (dx * dx + dy * dy) ** 0.5
            d = min(d, L / 2)
            return (a[0] + dx * d / L, a[1] + dy * d / L)

        segs.append((toward(p1, p0, r), p1, toward(p1, p2, r)))
    return segs


def _sample_rounded_poly(points, r, steps=16):
    pts = []
    for a, c, b in _rounded_poly_segments(points, r):
        for k in range(steps + 1):
            t = k / steps
            x = (1 - t) ** 2 * a[0] + 2 * (1 - t) * t * c[0] + t * t * b[0]
            y = (1 - t) ** 2 * a[1] + 2 * (1 - t) * t * c[1] + t * t * b[1]
            pts.append((x, y))
    return pts


def _num(v):
    v = round(v, 2)
    return str(int(v)) if v == int(v) else repr(v)


# --------------------------------------------------------------------------
# SVG output.

def to_svg(prims):
    out = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        "<!-- BachelorPad+ application icon: a text-processing appliance. -->",
        "<!-- Original work for the BachelorPad+ project. -->",
        "<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->",
        "<!-- Generated by assets/icon/render.py; edit the geometry there. -->",
        '<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 256 256">',
    ]
    for kind, p in prims:
        paint = f'fill="{p["fill"]}"'
        if p["stroke"]:
            paint += f' stroke="{p["stroke"]}" stroke-width="{_num(p["sw"])}"'
        if kind == "rrect":
            rx = f' rx="{_num(p["r"])}"' if p["r"] else ""
            out.append(
                f'  <rect x="{_num(p["x"])}" y="{_num(p["y"])}" width="{_num(p["w"])}"'
                f' height="{_num(p["h"])}"{rx} {paint}/>'
            )
        elif kind == "circle":
            out.append(
                f'  <circle cx="{_num(p["cx"])}" cy="{_num(p["cy"])}" r="{_num(p["r"])}" {paint}/>'
            )
        elif kind == "poly":
            segs = _rounded_poly_segments(p["points"], p["r"])
            d = f"M{_num(segs[0][2][0])} {_num(segs[0][2][1])}"
            for a, c, b in segs[1:] + segs[:1]:
                d += f"L{_num(a[0])} {_num(a[1])}Q{_num(c[0])} {_num(c[1])} {_num(b[0])} {_num(b[1])}"
            out.append(f'  <path d="{d}Z" stroke-linejoin="round" {paint}/>')
    out.append("</svg>")
    return "\n".join(out) + "\n"


# --------------------------------------------------------------------------
# Raster output.

def _draw(draw, kind, p, k):
    """Draw one primitive at scale k (pixels per unit)."""
    sw = p["sw"] * k
    R = round  # snap to the supersampled grid; 256/24 is not exact in floats
    if kind == "rrect":
        x0, y0 = p["x"] * k, p["y"] * k
        x1, y1 = (p["x"] + p["w"]) * k, (p["y"] + p["h"]) * k
        r = p["r"] * k
        if p["stroke"]:
            h = sw / 2
            draw.rounded_rectangle([R(x0 - h), R(y0 - h), R(x1 + h) - 1, R(y1 + h) - 1],
                                   R(r + h), fill=p["stroke"])
            draw.rounded_rectangle([R(x0 + h), R(y0 + h), R(x1 - h) - 1, R(y1 - h) - 1],
                                   R(max(r - h, 0)), fill=p["fill"])
        else:
            draw.rounded_rectangle([R(x0), R(y0), R(x1) - 1, R(y1) - 1], R(r), fill=p["fill"])
    elif kind == "circle":
        cx, cy, r = p["cx"] * k, p["cy"] * k, p["r"] * k
        if p["stroke"]:
            h = sw / 2
            draw.ellipse([cx - r - h, cy - r - h, cx + r + h - 1, cy + r + h - 1], fill=p["stroke"])
            draw.ellipse([cx - r + h, cy - r + h, cx + r - h - 1, cy + r - h - 1], fill=p["fill"])
        else:
            draw.ellipse([cx - r, cy - r, cx + r - 1, cy + r - 1], fill=p["fill"])
    elif kind == "poly":
        pts = [(x * k, y * k) for x, y in _sample_rounded_poly(p["points"], p["r"])]
        draw.polygon(pts, fill=p["fill"])
        if p["stroke"]:
            h = sw / 2
            ring = pts + [pts[0]]
            draw.line(ring, fill=p["stroke"], width=max(1, round(sw)))
            for x, y in pts:
                draw.ellipse([x - h, y - h, x + h - 1, y + h - 1], fill=p["stroke"])


def render(size):
    big = size * SS
    img = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    k = big / 256
    for kind, p in variant(size):
        _draw(draw, kind, p, k)
    # The grid-aligned small variants are box-filtered, which keeps their
    # pixel edges exact; LANCZOS would add ringing halos at 16-32 px.
    resample = Image.BOX if size <= 32 else Image.LANCZOS
    return img.resize((size, size), resample)


# --------------------------------------------------------------------------
# ICO: BMP (32-bit BGRA) entries up to 48 px for the widest compatibility,
# PNG-compressed entries from 64 px up (the 256 px entry must be PNG to stay
# small).

def _bmp_entry(img):
    w, h = img.size
    header = struct.pack("<IiiHHIIiiII", 40, w, h * 2, 1, 32, 0, 0, 0, 0, 0, 0)
    px = img.convert("RGBA").tobytes()
    rows = []
    for y in range(h - 1, -1, -1):
        row = px[y * w * 4:(y + 1) * w * 4]
        rows.append(b"".join(bytes((row[i + 2], row[i + 1], row[i], row[i + 3]))
                             for i in range(0, len(row), 4)))
    mask_stride = ((w + 31) // 32) * 4
    return header + b"".join(rows) + b"\0" * (mask_stride * h)


def _png_entry(img):
    buf = io.BytesIO()
    img.save(buf, "PNG", optimize=True)
    return buf.getvalue()


def write_ico(path, images):
    entries = [(im.size[0], _bmp_entry(im) if im.size[0] <= 48 else _png_entry(im))
               for im in images]
    offset = 6 + 16 * len(entries)
    head = struct.pack("<HHH", 0, 1, len(entries))
    dirs, blobs = b"", b""
    for size, data in entries:
        d = size if size < 256 else 0
        dirs += struct.pack("<BBBBHHII", d, d, 0, 0, 1, 32, len(data), offset)
        offset += len(data)
        blobs += data
    with open(path, "wb") as f:
        f.write(head + dirs + blobs)


# --------------------------------------------------------------------------
# Preview sheet: every size, native and on a light and a dark ground, plus
# the three smallest sizes enlarged 8x (nearest neighbour) to show pixels.

def write_preview(images):
    grounds = [("#f3f2ee", "#1b1b1b"), ("#1a1d21", "#e6e6e6")]
    pad = 24
    row_h = 512 + 2 * pad
    zoom = [16, 24, 32]
    zoom_w = sum(s * 8 + pad for s in zoom)
    native_w = sum(s + pad for s in PNG_SIZES) + pad
    width = native_w + zoom_w
    sheet = Image.new("RGB", (width, row_h * len(grounds)), "#000000")
    for gi, (bg, _fg) in enumerate(grounds):
        y0 = gi * row_h
        ImageDraw.Draw(sheet).rectangle([0, y0, width, y0 + row_h], fill=bg)
        x = pad
        for s in PNG_SIZES:
            im = images[s]
            sheet.paste(im, (x, y0 + pad + 512 - s), im)
            x += s + pad
        for s in zoom:
            im = images[s].resize((s * 8, s * 8), Image.NEAREST)
            sheet.paste(im, (x, y0 + pad), im)
            x += s * 8 + pad
    os.makedirs(os.path.dirname(PREVIEW), exist_ok=True)
    sheet.save(PREVIEW, optimize=True)


def main():
    svg = to_svg(master())
    with open(os.path.join(HERE, f"{APP_ID}.svg"), "w", encoding="utf-8") as f:
        f.write(svg)
    scalable = os.path.join(HERE, "hicolor", "scalable", "apps")
    os.makedirs(scalable, exist_ok=True)
    shutil.copyfile(os.path.join(HERE, f"{APP_ID}.svg"), os.path.join(scalable, f"{APP_ID}.svg"))

    images = {}
    for s in PNG_SIZES:
        im = render(s)
        images[s] = im
        d = os.path.join(HERE, "hicolor", f"{s}x{s}", "apps")
        os.makedirs(d, exist_ok=True)
        im.save(os.path.join(d, f"{APP_ID}.png"), optimize=True)

    write_ico(os.path.join(HERE, "bachelorpad.ico"), [images[s] for s in ICO_SIZES])
    write_preview(images)
    print("wrote", HERE, "and", PREVIEW, file=sys.stderr)


if __name__ == "__main__":
    main()
