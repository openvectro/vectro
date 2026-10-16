"""Builds every Vectro brand asset from one description of the mark.

python3 brand/build.py        (Pillow and numpy; brand/wordmark.json for the SVG wordmark)

The mark: a V drawn the way a pen tool draws it. The left arm is a straight segment, the right
arm a curve that ends in an anchor node with its handle out, on a violet-to-cyan tile.
"""
import json
import math
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont

HERE = Path(__file__).parent
FONT = HERE / "fonts/InterTight.ttf"

VIOLET, BLUE, CYAN = (124, 108, 255), (79, 140, 255), (34, 211, 238)
INK = (15, 15, 20)
WHITE = (255, 255, 255)

# Geometry in a 512 x 512 box.
A, B = (128, 170), (236, 366)            # left arm: A down to the vertex B
C1, C2, C = (272, 300), (296, 234), (334, 176)  # right arm: a curve from B up to the anchor C
H = (412, 118)                           # the anchor's handle
STROKE, HANDLE_W, TILE_R = 60, 9, 112
ANCHOR, ANCHOR_HOLE, HANDLE_R = 44, 18, 22


def bezier(p0, p1, p2, p3, n=160):
    for i in range(n + 1):
        t = i / n
        u = 1 - t
        yield (u**3 * p0[0] + 3 * u * u * t * p1[0] + 3 * u * t * t * p2[0] + t**3 * p3[0],
               u**3 * p0[1] + 3 * u * u * t * p1[1] + 3 * u * t * t * p2[1] + t**3 * p3[1])


def line(p0, p1, n=160):
    for i in range(n + 1):
        t = i / n
        yield (p0[0] + (p1[0] - p0[0]) * t, p0[1] + (p1[1] - p0[1]) * t)


def gradient_tile(size, radius):
    """A violet to cyan diagonal gradient inside a rounded square, as RGBA."""
    y, x = np.mgrid[0:size, 0:size].astype(np.float32)
    t = np.clip((x + y) / (2 * size), 0, 1)[..., None]
    stops = np.array([VIOLET, BLUE, CYAN], dtype=np.float32)
    seg = np.where(t < 0.5, 0, 1)
    tt = np.where(t < 0.5, t * 2, (t - 0.5) * 2)
    rgb = stops[seg[..., 0]] * (1 - tt) + stops[seg[..., 0] + 1] * tt
    img = Image.fromarray(np.dstack([rgb.astype(np.uint8), np.full((size, size, 1), 255, np.uint8)]), "RGBA")
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, size - 1, size - 1), radius=radius, fill=255)
    img.putalpha(mask)
    return img


def draw_mark(size, tile=True, colour=WHITE, ss=4):
    """The mark at `size` px. With tile=False it is the white V on transparent (or a flat colour)."""
    k = size * ss / 512
    S = size * ss
    img = gradient_tile(S, int(TILE_R * k)) if tile else Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    sc = lambda p: (p[0] * k, p[1] * k)
    r = STROKE * k / 2
    for p in list(line(A, B)) + list(bezier(B, C1, C2, C)):
        x, y = sc(p)
        d.ellipse((x - r, y - r, x + r, y + r), fill=colour)
    # Handle, then the anchor square with a hole, then the handle's end circle.
    hw = HANDLE_W * k / 2
    for p in line(C, H, 120):
        x, y = sc(p)
        d.ellipse((x - hw, y - hw, x + hw, y + hw), fill=colour)
    cx, cy = sc(C)
    a = ANCHOR * k / 2
    d.rounded_rectangle((cx - a, cy - a, cx + a, cy + a), radius=6 * k, fill=colour)
    hole = ANCHOR_HOLE * k / 2
    hole_colour = (0, 0, 0, 0) if not tile else None
    if tile:
        # Cut the hole back to the tile gradient by sampling it.
        tile_img = gradient_tile(S, int(TILE_R * k))
        patch = tile_img.crop((int(cx - hole), int(cy - hole), int(cx + hole), int(cy + hole)))
        img.paste(patch, (int(cx - hole), int(cy - hole)))
    else:
        d.rounded_rectangle((cx - hole, cy - hole, cx + hole, cy + hole), radius=3 * k, fill=hole_colour)
    hx, hy = sc(H)
    rr = HANDLE_R * k
    ring = HANDLE_W * k
    if tile:
        tile_img = gradient_tile(S, int(TILE_R * k))
        disc = Image.new("L", (S, S), 0)
        ImageDraw.Draw(disc).ellipse((hx - rr + ring, hy - rr + ring, hx + rr - ring, hy + rr - ring), fill=255)
        d.ellipse((hx - rr, hy - rr, hx + rr, hy + rr), fill=colour)
        img.paste(tile_img, (0, 0), disc)
    else:
        d.ellipse((hx - rr, hy - rr, hx + rr, hy + rr), outline=colour, width=int(ring))
    return img.resize((size, size), Image.LANCZOS)


def wordmark_font(px):
    f = ImageFont.truetype(str(FONT), px)
    f.set_variation_by_axes([640])
    return f


def lockup(height, colour, scale=2):
    """Mark plus wordmark on transparent, `height` px tall."""
    Hs = height * scale
    mark = draw_mark(Hs, tile=True)
    f = wordmark_font(int(Hs * 0.78))
    text = "Vectro"
    tw = int(f.getlength(text) * 0.97)
    gap = int(Hs * 0.22)
    img = Image.new("RGBA", (Hs + gap + tw + int(Hs * 0.05), Hs), (0, 0, 0, 0))
    img.paste(mark, (0, 0), mark)
    d = ImageDraw.Draw(img)
    # Tight tracking: draw glyph by glyph.
    x = Hs + gap
    for i, ch in enumerate(text):
        d.text((x, Hs * 0.5), ch, font=f, fill=colour, anchor="lm")
        x += f.getlength(ch) - Hs * 0.012
    return img.resize((img.width // scale, img.height // scale), Image.LANCZOS)


def social_preview():
    W, Hh, s = 1280, 640, 2
    img = Image.new("RGBA", (W * s, Hh * s), INK + (255,))
    # Two soft glows.
    y, x = np.mgrid[0:Hh * s, 0:W * s].astype(np.float32)
    glow = np.zeros((Hh * s, W * s, 3), np.float32)
    for (gx, gy, rad, col, amt) in [(260 * s, 180 * s, 620 * s, VIOLET, 0.55), (1120 * s, 560 * s, 640 * s, CYAN, 0.35)]:
        dist = np.sqrt((x - gx) ** 2 + (y - gy) ** 2) / rad
        w = np.clip(1 - dist, 0, 1) ** 2 * amt
        glow += w[..., None] * np.array(col, np.float32)
    base = np.array(INK, np.float32) + glow
    img = Image.fromarray(np.dstack([np.clip(base, 0, 255).astype(np.uint8), np.full((Hh * s, W * s, 1), 255, np.uint8)]), "RGBA")
    mark = draw_mark(200 * s, tile=True)
    img.paste(mark, (96 * s, 160 * s), mark)
    d = ImageDraw.Draw(img)
    f = wordmark_font(150 * s)
    x, ty = 340 * s, 258 * s
    for ch in "Vectro":
        d.text((x, ty), ch, font=f, fill=WHITE, anchor="lm")
        x += f.getlength(ch) - 150 * s * 0.012
    def fitted(text, px, weight, max_w):
        """The largest size at or under px whose text fits in max_w."""
        while px > 10:
            f = ImageFont.truetype(str(FONT), px * s)
            f.set_variation_by_axes([weight])
            if f.getlength(text) <= max_w * s:
                return f
            px -= 1
        return f
    right_margin = 72
    l1 = "Free, open-source interactive vector animation."
    l2 = "Design it, animate it, wire it to inputs. Exports .riv for every platform."
    f1 = fitted(l1, 34, 450, W - 342 - right_margin)
    f2 = fitted(l2, 34, 450, W - 342 - right_margin)
    d.text((342 * s, 352 * s), l1, font=f1, fill=(205, 205, 215), anchor="lm")
    d.text((342 * s, 398 * s), l2, font=f2, fill=(150, 150, 165), anchor="lm")
    small = ImageFont.truetype(str(FONT), 26 * s)
    small.set_variation_by_axes([550])
    label = "github.com/openvectro/vectro"
    tw = small.getlength(label)
    px, py = 342 * s, 470 * s
    # A translucent pill has to be composited; drawing RGBA straight onto the image replaces alpha.
    layer = Image.new("RGBA", img.size, (0, 0, 0, 0))
    ld = ImageDraw.Draw(layer)
    ld.rounded_rectangle((px, py, px + tw + 44 * s, py + 50 * s), radius=25 * s, fill=(255, 255, 255, 22), outline=(255, 255, 255, 60), width=2 * s)
    img = Image.alpha_composite(img, layer)
    d = ImageDraw.Draw(img)
    d.text((px + 22 * s, py + 25 * s), label, font=small, fill=(230, 230, 240), anchor="lm")
    return img.resize((W, Hh), Image.LANCZOS).convert("RGB")


def write_svgs():
    def path_d():
        return f"M{A[0]} {A[1]}L{B[0]} {B[1]}C{C1[0]} {C1[1]} {C2[0]} {C2[1]} {C[0]} {C[1]}"
    a = ANCHOR / 2
    hole = ANCHOR_HOLE / 2
    mark_body = f"""  <path d="{path_d()}" fill="none" stroke="#fff" stroke-width="{STROKE}" stroke-linecap="round" stroke-linejoin="round"/>
  <path d="M{C[0]} {C[1]}L{H[0]} {H[1]}" fill="none" stroke="#fff" stroke-width="{HANDLE_W}" stroke-linecap="round"/>
  <rect x="{C[0]-a}" y="{C[1]-a}" width="{ANCHOR}" height="{ANCHOR}" rx="6" fill="#fff"/>
  <rect x="{C[0]-hole}" y="{C[1]-hole}" width="{ANCHOR_HOLE}" height="{ANCHOR_HOLE}" rx="3" fill="url(#g)"/>
  <circle cx="{H[0]}" cy="{H[1]}" r="{HANDLE_R - HANDLE_W/2}" fill="url(#g)" stroke="#fff" stroke-width="{HANDLE_W}"/>"""
    grad = f"""  <defs><linearGradient id="g" x1="0" y1="0" x2="512" y2="512" gradientUnits="userSpaceOnUse">
    <stop offset="0" stop-color="#7c6cff"/><stop offset="0.5" stop-color="#4f8cff"/><stop offset="1" stop-color="#22d3ee"/>
  </linearGradient></defs>"""
    mark = f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
{grad}
  <rect width="512" height="512" rx="{TILE_R}" fill="url(#g)"/>
{mark_body}
</svg>
"""
    (HERE / "vectro-mark.svg").write_text(mark)
    # Monochrome mark: the V alone, for favicons on coloured backgrounds and for print.
    mono = f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <path d="{path_d()}" fill="none" stroke="currentColor" stroke-width="{STROKE}" stroke-linecap="round" stroke-linejoin="round"/>
  <path d="M{C[0]} {C[1]}L{H[0]} {H[1]}" fill="none" stroke="currentColor" stroke-width="{HANDLE_W}" stroke-linecap="round"/>
  <path fill-rule="evenodd" d="M{C[0]-a} {C[1]-a}h{ANCHOR}v{ANCHOR}h-{ANCHOR}z M{C[0]-hole} {C[1]-hole}h{ANCHOR_HOLE}v{ANCHOR_HOLE}h-{ANCHOR_HOLE}z" fill="currentColor"/>
  <circle cx="{H[0]}" cy="{H[1]}" r="{HANDLE_R - HANDLE_W/2}" fill="none" stroke="currentColor" stroke-width="{HANDLE_W}"/>
</svg>
"""
    (HERE / "vectro-mark-mono.svg").write_text(mono)
    wm = HERE / "wordmark.json"
    if not wm.exists():
        print("no brand/wordmark.json: run wordmark_paths.py for the SVG lockups")
        return
    w = json.loads(wm.read_text())
    upm, cap = w["unitsPerEm"], w["capHeight"]
    # Lockup: mark 512 tall, wordmark cap height 0.62 of it, baseline so caps centre on the mark.
    k = 512 * 0.62 / cap
    gap = 512 * 0.22
    text_w = w["width"] * k
    base_y = 256 + cap * k / 2
    glyphs = "\n".join(f'  <path transform="translate({512 + gap + g["x"] * k:.1f} {base_y:.1f}) scale({k:.4f} {-k:.4f})" d="{g["d"]}"/>' for g in w["glyphs"])
    total = 512 + gap + text_w
    for name, fill in (("vectro-logo-dark", "#ffffff"), ("vectro-logo-light", "#15151b")):
        svg = f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {total:.0f} 512" width="{total:.0f}" height="512">
{grad}
  <rect width="512" height="512" rx="{TILE_R}" fill="url(#g)"/>
{mark_body}
  <g fill="{fill}">
{glyphs}
  </g>
</svg>
"""
        (HERE / f"{name}.svg").write_text(svg)


def main():
    write_svgs()
    for size in (1024, 512, 256, 128, 64, 32, 16):
        draw_mark(size).save(HERE / f"icon-{size}.png")
    icons = [draw_mark(s) for s in (16, 32, 48, 64, 128, 256)]
    icons[-1].save(HERE / "favicon.ico", sizes=[(i.width, i.height) for i in icons], append_images=icons[:-1])
    lockup(128, WHITE).save(HERE / "vectro-logo-dark.png")
    lockup(128, INK).save(HERE / "vectro-logo-light.png")
    social_preview().save(HERE / "social-preview.png", optimize=True)
    # macOS-style: the tile already has the rounded corners; a padded square for stores.
    pad = Image.new("RGBA", (1024, 1024), (0, 0, 0, 0))
    m = draw_mark(820)
    pad.paste(m, (102, 102), m)
    pad.save(HERE / "icon-1024-padded.png")
    print("brand assets written to", HERE)


if __name__ == "__main__":
    main()
