"""Turn the wordmark into outlines so the SVG logo needs no font.

Needs fontTools:  python3 -m venv .venv && .venv/bin/pip install fonttools
Usage:            .venv/bin/python brand/wordmark_paths.py > brand/wordmark.json
"""
import json
import sys
from pathlib import Path

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

HERE = Path(__file__).parent
TEXT = "Vectro"
WEIGHT = 640
TRACKING = -0.025  # of an em, tightened like the UI type

font = instantiateVariableFont(TTFont(HERE / "fonts/InterTight.ttf"), {"wght": WEIGHT})
cmap = font.getBestCmap()
glyphs = font.getGlyphSet()
hmtx = font["hmtx"]
upm = font["head"].unitsPerEm

out = {"unitsPerEm": upm, "capHeight": font["OS/2"].sCapHeight, "glyphs": []}
x = 0
for ch in TEXT:
    name = cmap[ord(ch)]
    pen = SVGPathPen(glyphs)
    glyphs[name].draw(pen)
    advance = hmtx[name][0]
    out["glyphs"].append({"char": ch, "x": x, "d": pen.getCommands()})
    x += advance + TRACKING * upm
out["width"] = x - TRACKING * upm
json.dump(out, sys.stdout)
