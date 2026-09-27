# BachelorPad+ application icon

**Status: approved by the owner on 2026-09-26 (release plan M2-01)**, as drawn and as shown in `artifacts/ui/icon-preview-2026-09-25.png`.

## The mark

A squat, burnt-orange **text-processing appliance** on two stubby feet, with a
sheet of paper feeding into the intake slot on its top. The front carries a dark
readout window with a lit green cursor and two round status lights, one green
("ready") and one white. The housing is a rounded trapezoid, wider at the base,
in the chunky, deadpan, 1970s consumer-product manner of
[the brand guide](../../docs/brand/BRAND_GUIDE.md).

It is made only of flat shapes: no lettering (the paper shows bars, not letters), no gradients, no
filters, no embedded images. It is original work and copies no logo, character,
prop, typography or colour/layout combination from any show or product, and it
is deliberately unlike Notepad's icon (no spiral-bound pad, no pencil, no blue).

The 16, 24 and 32 px sizes are hand-tuned variants drawn on their own pixel
grid (one-pixel outlines, square lights, fewer paper lines, a rectangular
housing) so they stay crisp; 48 px and up are the master drawing.

## Palette

All four colours come from the product's own themes (`crates/bp-theme`).

| Role | Hex | Theme source |
|---|---|---|
| Housing, feet | `#b8541f` | Light accent |
| Outlines, slot, window, paper text | `#1b1b1b` | Light ink |
| Paper, the white light | `#f7f6f3` | Light panel |
| Cursor, the green light | `#3ddc6a` | Terminal accent |

## Files

| File | Use |
|---|---|
| `io.github.dboles99.BachelorPad.svg` | the master (viewBox 0 0 256 256) |
| `hicolor/scalable/apps/io.github.dboles99.BachelorPad.svg` | a copy of the master for the hicolor theme |
| `hicolor/<N>x<N>/apps/io.github.dboles99.BachelorPad.png` | N = 16, 24, 32, 48, 64, 128, 256, 512 |
| `bachelorpad.ico` | Windows icon: 16, 24, 32, 48 as 32-bit BMP entries; 64, 128, 256 PNG-compressed |
| `render.py` | the generator: the single source of the geometry |

Preview sheet (every size, on a light and a dark ground, with 16/24/32 enlarged
8x): [`artifacts/ui/icon-preview-2026-09-25.png`](../../artifacts/ui/icon-preview-2026-09-25.png).

## Regenerating

```sh
python3 assets/icon/render.py
```

Needs only Python 3 and Pillow. The geometry is defined once in `render.py` as
a list of primitives (rounded rectangles, circles, a rounded polygon) in the
256-unit space; the script writes the SVG from that list and rasterises the
same list with Pillow for every PNG, the `.ico` and the preview sheet, so the
SVG and the PNGs cannot drift. Edit the geometry in `render.py`, never the SVG
by hand.

## Licence

Original work for BachelorPad+, under the project's licence:
MIT OR Apache-2.0.
