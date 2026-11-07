"""Pre-shape Thai for renderers without OpenType shaping (egui / ab_glyph).

For every Thai syllable cluster (consonant + lower vowel + upper vowel + tone mark, incl. SARA AM)
this script asks HarfBuzz how Noto Sans Thai positions it, then:
  * adds composite glyphs (the substituted variant at the shaped offset) on Private Use code points,
  * writes `thai_clusters.txt`: one line per cluster whose rendering differs from the plain text,
    "<input>\t<output>" where <output> uses the new code points.

The same code points are generated for every weight, so one table serves regular and bold text.

usage: python thai_preshape.py <fonts-dir> <out-dir>
requires: fonttools, uharfbuzz
"""
import os
import sys

import uharfbuzz as hb
from fontTools.ttLib import TTFont
from fontTools.ttLib.tables._g_l_y_f import Glyph, GlyphComponent

FONTS = ["NotoSansThai-Regular.ttf", "NotoSansThai-Medium.ttf"]
CONSONANTS = [chr(c) for c in range(0x0E01, 0x0E2F)]
UPPER = [None, "ั", "ิ", "ี", "ึ", "ื", "็", "ํ", "AM"]
TONES = [None, "่", "้", "๊", "๋", "์"]
LOWER = [None, "ุ", "ู", "ฺ"]
PUA_START = 0xE100
TOLERANCE = 4  # font units; smaller offsets are invisible at UI sizes


def clusters():
    for c in CONSONANTS:
        for lo in LOWER:
            for up in UPPER:
                for tone in TONES:
                    if up == "AM":
                        if lo:
                            continue
                        yield c + (tone or "") + "ำ"
                    else:
                        yield c + (lo or "") + (up or "") + (tone or "")


class Shaper:
    def __init__(self, path):
        self.tt = TTFont(path)
        self.order = self.tt.getGlyphOrder()
        self.cmap = self.tt.getBestCmap()
        self.font = hb.Font(hb.Face(hb.Blob.from_file_path(path)))

    def shape(self, text):
        buf = hb.Buffer()
        buf.add_str(text)
        buf.guess_segment_properties()
        hb.shape(self.font, buf, {})
        return [(self.order[i.codepoint], p.x_offset, p.y_offset) for i, p in zip(buf.glyph_infos, buf.glyph_positions)]

    def default_glyph(self, ch):
        return self.cmap.get(ord(ch))


def main():
    fonts_dir, out_dir = sys.argv[1], sys.argv[2]
    shapers = {f: Shaper(os.path.join(fonts_dir, f)) for f in FONTS}
    ref = shapers[FONTS[0]]
    # key: (glyph name, dx, dy) in the reference font -> PUA code point
    pua = {}
    # cluster -> list of (pua code point or original char) ; plus per-font glyph spec for each PUA
    spec = {}  # pua -> {font: (glyph, dx, dy)}
    table = []
    for text in clusters():
        shaped = ref.shape(text)
        # expected unshaped rendering: the chars of `text`, with SARA AM decomposed as Noto does
        plain = list(text.replace("ำ", "ํา")) if "ำ" in text else list(text)
        if len(shaped) != len(plain):
            continue  # unexpected reordering; leave cluster untouched
        out = []
        changed = False
        for k, ((g, dx, dy), ch) in enumerate(zip(shaped, plain)):
            if g == ref.default_glyph(ch) and abs(dx) <= TOLERANCE and abs(dy) <= TOLERANCE:
                out.append(ch)
                continue
            key = (g, round(dx / 2) * 2, round(dy / 2) * 2)
            if key not in pua:
                pua[key] = PUA_START + len(pua)
            cp = pua[key]
            spec.setdefault(cp, {})
            for f, sh in shapers.items():
                if f not in spec[cp]:
                    gg, ddx, ddy = sh.shape(text)[k]
                    spec[cp][f] = (gg, ddx, ddy)
            out.append(chr(cp))
            changed = True
        if changed:
            table.append((text, "".join(out)))

    for f in FONTS:
        tt = TTFont(os.path.join(fonts_dir, f))
        glyf = tt["glyf"]
        hmtx = tt["hmtx"]
        order = tt.getGlyphOrder()
        new_names = []
        for cp, per_font in sorted(spec.items()):
            g, dx, dy = per_font[f]
            name = f"rdthai{cp:04X}"
            comp = GlyphComponent()
            comp.glyphName = g
            comp.x, comp.y = dx, dy
            comp.flags = 0x4  # ROUND_XY_TO_GRID
            glyph = Glyph()
            glyph.numberOfContours = -1
            glyph.components = [comp]
            glyf.glyphs[name] = glyph
            adv = hmtx[g][0]
            hmtx[name] = (adv, hmtx[g][1] + dx)
            new_names.append((cp, name))
        tt.setGlyphOrder(order + [n for _, n in new_names])
        for table_ in tt["cmap"].tables:
            if table_.isUnicode():
                for cp, n in new_names:
                    table_.cmap[cp] = n
        for _, n in new_names:
            glyf.glyphs[n].recalcBounds(glyf)
        tt["maxp"].numGlyphs = len(tt.getGlyphOrder())
        out_name = f.replace(".ttf", "-RD.ttf")
        tt.save(os.path.join(out_dir, out_name))
        print("wrote", out_name, "glyphs added:", len(new_names))

    with open(os.path.join(out_dir, "thai_clusters.txt"), "w", encoding="utf-8") as fh:
        for a, b in table:
            fh.write(f"{a}\t{b}\n")
    print("clusters:", len(table), "pua glyphs:", len(pua))


if __name__ == "__main__":
    main()
