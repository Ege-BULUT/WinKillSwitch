"""Builds docs/assets/title.svg: "WinKillSwitch" where each letter first flickers in as an Elder
Futhark rune, then resolves into Cinzel with an aurora gradient that keeps drifting across it.

Glyphs are converted to SVG paths, so the file needs no fonts and renders the same on GitHub.

    py -3 docs/tools/make_title.py <Cinzel[wght].ttf> <NotoSansRunic-Regular.ttf> [out.svg]

Both fonts are SIL Open Font License (Google Fonts).
"""
import sys
from fontTools.ttLib import TTFont
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.boundsPen import BoundsPen
from fontTools.varLib.instancer import instantiateVariableFont

TITLE = "WinKillSwitch"
RUNES = {"W": "ᚹ", "I": "ᛁ", "N": "ᚾ", "K": "ᚲ", "L": "ᛚ", "S": "ᛊ", "T": "ᛏ", "C": "ᚲ", "H": "ᚺ"}
# A couple of extra runes each slot flickers through before its own.
FLICKER = "ᚠᚢᚦᚨᚱᚷᛃᛇᛈᛉᛒᛖᛗᛜᛞᛟ"

SIZE = 128.0  # px per em for the Latin letters
TRACK = 30.0  # extra advance per letter, font units
PAD_X, TOP, BASE = 40.0, 22.0, 150.0


def glyph(font, ch):
    cmap = font.getBestCmap()
    name = cmap[ord(ch)]
    gs = font.getGlyphSet()
    pen = SVGPathPen(gs)
    gs[name].draw(pen)
    bp = BoundsPen(gs)
    gs[name].draw(bp)
    return pen.getCommands(), gs[name].width, bp.bounds


def main():
    cinzel = TTFont(sys.argv[1])
    if "fvar" in cinzel:
        cinzel = instantiateVariableFont(cinzel, {"wght": 700})
    runic = TTFont(sys.argv[2])
    out = sys.argv[3] if len(sys.argv) > 3 else "docs/assets/title.svg"

    s = SIZE / cinzel["head"].unitsPerEm
    letters, x = [], 0.0
    for ch in TITLE:
        d, adv, _ = glyph(cinzel, ch)
        letters.append((ch, d, x, adv))
        x += adv + TRACK
    width_units = x - TRACK
    W = width_units * s + PAD_X * 2
    H = 210.0

    rs = SIZE * 0.9 / runic["head"].unitsPerEm
    parts = []
    for i, (ch, d, lx, adv) in enumerate(letters):
        cx = PAD_X + (lx + adv / 2) * s  # slot centre in px
        delay = 0.25 + i * 0.085
        # Runes: two flicker glyphs and the letter's own, stacked, each blinking in turn.
        seq = [FLICKER[(i * 5) % len(FLICKER)], FLICKER[(i * 7 + 3) % len(FLICKER)], RUNES[ch.upper()]]
        for k, rch in enumerate(seq):
            rd, radv, rb = glyph(runic, rch)
            rw = (rb[2] - rb[0]) if rb else radv
            tx = cx - (rb[0] + rw / 2) * rs if rb else cx - radv * rs / 2
            parts.append(
                f'<path class="r" style="animation-delay:{delay + k * 0.16:.3f}s" '
                f'transform="translate({tx:.2f} {BASE:.2f}) scale({rs:.5f} {-rs:.5f})" d="{rd}"/>'
            )
        # CSS transforms replace an element's transform attribute, so the rise animation lives on
        # an outer group and the glyph's own placement on an inner one.
        parts.append(
            f'<g class="l" style="animation-delay:{delay + 0.5:.3f}s">'
            f'<g transform="translate({PAD_X + lx * s:.2f} {BASE:.2f}) scale({s:.5f} {-s:.5f})"><path d="{d}"/></g></g>'
        )

    line_y = BASE + 26
    total = 0.25 + len(letters) * 0.085 + 0.5
    svg = f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W:.0f} {H:.0f}" width="{W:.0f}" height="{H:.0f}" role="img" aria-label="WinKillSwitch">
<title>WinKillSwitch</title>
<defs>
  <linearGradient id="aurora" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="{W:.0f}" y2="0" spreadMethod="repeat">
    <stop offset="0" stop-color="#7c5cff"/><stop offset=".3" stop-color="#ff5ca8"/>
    <stop offset=".55" stop-color="#22d3ee"/><stop offset=".8" stop-color="#4f8dff"/><stop offset="1" stop-color="#7c5cff"/>
    <animateTransform attributeName="gradientTransform" type="translate" from="0 0" to="{W:.0f} 0" dur="7s" begin="{total:.2f}s" repeatCount="indefinite"/>
  </linearGradient>
  <linearGradient id="rule" gradientUnits="userSpaceOnUse" x1="{PAD_X + 40:.0f}" y1="0" x2="{W - PAD_X - 40:.0f}" y2="0"><stop offset="0" stop-color="#7c5cff" stop-opacity="0"/><stop offset=".5" stop-color="#ff5ca8"/><stop offset="1" stop-color="#22d3ee" stop-opacity="0"/></linearGradient>
  <filter id="glow" x="-10%" y="-40%" width="120%" height="180%"><feGaussianBlur stdDeviation="9"/></filter>
</defs>
<style>
  .r {{ fill: #9b84ff; opacity: 0; animation: rune .42s ease-out forwards; }}
  .l {{ opacity: 0; animation: rise .9s cubic-bezier(.2,.9,.2,1) forwards; transform-box: fill-box; }}
  .m {{ fill: #fff; }}
  .halo {{ opacity: 0; animation: halo 1.2s ease-out {total:.2f}s forwards; }}
  .rule {{ stroke: url(#rule); stroke-width: 2; stroke-linecap: round; stroke-dasharray: {W:.0f}; stroke-dashoffset: {W:.0f}; animation: draw 1.4s cubic-bezier(.6,0,.2,1) {total - 0.3:.2f}s forwards; }}
  @keyframes rune {{ 0% {{ opacity: 0 }} 30% {{ opacity: .95 }} 60% {{ opacity: .4 }} 100% {{ opacity: 0 }} }}
  @keyframes rise {{ 0% {{ opacity: 0; transform: translateY(14px) }} 100% {{ opacity: 1; transform: translateY(0) }} }}
  @keyframes halo {{ to {{ opacity: .38 }} }}
  @keyframes draw {{ to {{ stroke-dashoffset: 0 }} }}
  @media (prefers-reduced-motion: reduce) {{ .r {{ display: none }} .l, .halo {{ animation: none; opacity: 1 }} .halo {{ opacity: .3 }} .rule {{ animation: none; stroke-dashoffset: 0 }} }}
</style>
<mask id="word" maskUnits="userSpaceOnUse" x="0" y="0" width="{W:.0f}" height="{H:.0f}"><g class="m">{''.join(p for p in parts if 'class="l"' in p)}</g></mask>
<g>{''.join(p for p in parts if 'class="r"' in p)}</g>
<g class="halo" filter="url(#glow)"><rect width="{W:.0f}" height="{H:.0f}" fill="url(#aurora)" mask="url(#word)"/></g>
<rect width="{W:.0f}" height="{H:.0f}" fill="url(#aurora)" mask="url(#word)"/>
<line class="rule" x1="{PAD_X + 40:.0f}" y1="{line_y:.0f}" x2="{W - PAD_X - 40:.0f}" y2="{line_y:.0f}"/>
</svg>
'''
    with open(out, "w", encoding="utf-8") as f:
        f.write(svg)
    print(f"wrote {out}: {W:.0f}x{H:.0f}, {len(svg) // 1024} KB")


if __name__ == "__main__":
    main()
