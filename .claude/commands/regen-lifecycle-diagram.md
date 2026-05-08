# Regenerate Workspace Lifecycle Diagram

Re-renders `docs/conduit-workspace-lifecycle.excalidraw` into light and dark PNG exports, then commits and pushes the updated images.

Run this whenever the workspace lifecycle flow changes and the `.excalidraw` source has been updated.

## Steps

1. Ensure `cairosvg` is available (`pip3 install cairosvg --break-system-packages` if missing).

2. Run the renderer:

```python
#!/usr/bin/env python3
import json

SRC   = 'docs/conduit-workspace-lifecycle.excalidraw'
SVG_L = '/tmp/lifecycle-light.svg'
SVG_D = '/tmp/lifecycle-dark.svg'
PNG_L = 'docs/conduit-workspace-lifecycle.png'
PNG_D = 'docs/conduit-workspace-lifecycle-dark.png'

# ── Dark-mode colour substitutions ───────────────────────────────────────────
DF = {          # fills / backgroundColors
    '#f8f9fa': '#252535',
    '#e9ecef': '#2d3140',
    '#dee2e6': '#363a50',
    '#d3f9d8': '#1d3529',
    '#d0ebff': '#1c2d44',
    '#fff3bf': '#36300e',
    '#495057': '#3e4460',
    'none':    'none',
}
DS = {          # strokes / borders
    '#adb5bd': '#4d5670',
    '#343a40': '#5c6482',
    '#8ce99a': '#4fa068',
    '#74c0fc': '#4a90d9',
    '#ffd43b': '#c8a020',
    '#dee2e6': '#3d4260',
    '#868e96': '#6b7896',
}
DT = {          # text colours (strokeColor on text elements)
    '#495057': '#c5cfe0',
    '#f8f9fa': '#eef2f8',
    '#2f9e44': '#5cb87c',
    '#1971c2': '#5a9fd4',
    '#e67700': '#d4b020',
    '#6c757d': '#8a9bb2',
    '#868e96': '#8a9bb2',
}
DARK_CANVAS = '#1e1e2e'

def esc(t):
    return t.replace('&','&amp;').replace('<','&lt;').replace('>','&gt;')

def apply(mapping, color):
    return mapping.get(color, color)

def render(el, dark=False):
    t = el['type']
    if   t == 'rectangle': return rect(el, dark)
    elif t == 'diamond':   return diamond(el, dark)
    elif t == 'ellipse':   return ellipse(el, dark)
    elif t == 'text':      return text_el(el, dark)
    elif t == 'arrow':     return arrow(el, dark)
    return ''

def sty(el, dark):
    fill   = el.get('backgroundColor', 'transparent')
    stroke = el.get('strokeColor', '#000')
    sw     = el.get('strokeWidth', 1)
    op     = el.get('opacity', 100) / 100
    if fill in ('transparent',): fill = 'none'
    if dark:
        fill   = apply(DF, fill)
        stroke = apply(DS, stroke)
    return fill, stroke, sw, op

def rect(el, dark):
    x,y,w,h = el['x'],el['y'],el['width'],el['height']
    fill,stroke,sw,op = sty(el, dark)
    rx = 8 if el.get('roundness') else 0
    return (f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" '
            f'fill="{fill}" stroke="{stroke}" stroke-width="{sw}" opacity="{op}"/>')

def diamond(el, dark):
    x,y,w,h = el['x'],el['y'],el['width'],el['height']
    fill,stroke,sw,op = sty(el, dark)
    cx,cy = x+w/2, y+h/2
    pts = f"{cx},{y} {x+w},{cy} {cx},{y+h} {x},{cy}"
    return (f'<polygon points="{pts}" '
            f'fill="{fill}" stroke="{stroke}" stroke-width="{sw}" opacity="{op}"/>')

def ellipse(el, dark):
    x,y,w,h = el['x'],el['y'],el['width'],el['height']
    fill,stroke,sw,op = sty(el, dark)
    cx,cy = x+w/2, y+h/2
    return (f'<ellipse cx="{cx}" cy="{cy}" rx="{w/2}" ry="{h/2}" '
            f'fill="{fill}" stroke="{stroke}" stroke-width="{sw}" opacity="{op}"/>')

def text_el(el, dark):
    x,y,w,h  = el['x'],el['y'],el['width'],el['height']
    txt       = el.get('text','')
    fs        = el.get('fontSize',16)
    color     = el.get('strokeColor','#000')
    align     = el.get('textAlign','left')
    op        = el.get('opacity',100)/100
    lh        = el.get('lineHeight',1.25)
    if not txt.strip(): return ''
    if dark: color = apply(DT, color)
    lines  = txt.split('\n')
    line_h = fs * lh
    anchor = {'center':'middle','right':'end'}.get(align,'start')
    tx     = (x+w/2) if align=='center' else (x+w if align=='right' else x)
    parts  = [f'<g opacity="{op}" font-family="Arial,Helvetica,sans-serif" '
              f'font-size="{fs}" fill="{color}" text-anchor="{anchor}">']
    for i, line in enumerate(lines):
        by = y + fs + i*line_h
        parts.append(f'  <text x="{tx:.2f}" y="{by:.2f}">{esc(line)}</text>')
    parts.append('</g>')
    return '\n'.join(parts)

_arrow_n = 0
def arrow(el, dark):
    global _arrow_n
    _arrow_n += 1
    mid    = f'ah{_arrow_n}'
    x,y    = el['x'],el['y']
    pts    = el.get('points',[[0,0],[0,0]])
    stroke = el.get('strokeColor','#868e96')
    sw     = el.get('strokeWidth',1)
    op     = el.get('opacity',100)/100
    if dark: stroke = apply(DS, stroke)
    absp   = [(x+p[0], y+p[1]) for p in pts]
    d      = 'M ' + ' L '.join(f'{px:.1f},{py:.1f}' for px,py in absp)
    defs   = (f'<defs><marker id="{mid}" markerWidth="9" markerHeight="7" '
              f'refX="8" refY="3.5" orient="auto" markerUnits="strokeWidth">'
              f'<polygon points="0 0,9 3.5,0 7" fill="{stroke}"/>'
              f'</marker></defs>')
    path   = (f'<path d="{d}" fill="none" stroke="{stroke}" stroke-width="{sw}" '
              f'opacity="{op}" marker-end="url(#{mid})"/>')
    return defs + path

def el_order(el):
    eid = el.get('id','')
    if eid.startswith('bg_'): return 0
    if el['type'] == 'arrow': return 2
    if el['type'] == 'text':  return 3
    return 1

def build_svg(els, dark=False):
    global _arrow_n
    _arrow_n = 0
    shapes = [e for e in els if e['type'] not in ('arrow',)]
    content = [e for e in shapes if not e.get('id','').startswith('bg')]
    bbox = content if content else shapes
    pad = 35
    # x bounds + y-bottom: use all shapes so bg elements stay inside the viewport.
    # y-top: use content-only so bg1 covers the top without a self-referential gap.
    all_xs  = [e['x'] for e in shapes]+[e['x']+e.get('width',0) for e in shapes]
    all_yb  = [e['y']+e.get('height',0) for e in shapes]
    cont_yt = [e['y'] for e in bbox]
    mnx = min(all_xs)-pad; mxx = max(all_xs)+pad
    mny = min(cont_yt)-pad; mxy = max(all_yb)+pad
    vw,vh = mxx-mnx, mxy-mny
    canvas = DARK_CANVAS if dark else 'white'
    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{vw:.0f}" height="{vh:.0f}" '
        f'viewBox="{mnx:.0f} {mny:.0f} {vw:.0f} {vh:.0f}">',
        f'<rect width="100%" height="100%" fill="{canvas}"/>',
    ]
    for el in sorted(els, key=el_order):
        r = render(el, dark=dark)
        if r: parts.append(r)
    parts.append('</svg>')
    return '\n'.join(parts)

import cairosvg

data = json.load(open(SRC))
els  = [e for e in data['elements'] if not e.get('isDeleted')]

for dark, svg_path, png_path, label in [
    (False, SVG_L, PNG_L, 'light'),
    (True,  SVG_D, PNG_D, 'dark'),
]:
    svg = build_svg(els, dark=dark)
    open(svg_path, 'w').write(svg)
    cairosvg.svg2png(url=f'file://{svg_path}', write_to=png_path, scale=2)
    print(f'{label}: {png_path}')
```

3. Commit and push:

```bash
git add docs/conduit-workspace-lifecycle.png docs/conduit-workspace-lifecycle-dark.png
git commit -m "docs: regenerate workspace lifecycle diagram PNGs"
git push
```

## Notes

- The `.excalidraw` source is the source of truth. Edit that in Excalidraw first, save it to `docs/conduit-workspace-lifecycle.excalidraw`, then run this command.
- Bound text elements must have `y` and `height` set to the actual text bounding box (not the container dimensions) for correct vertical centering on load. The fix formula is: `text.height = fontSize × lineHeight × numLines`, `text.y = container.y + (container.height − text.height) / 2`. If text appears top-aligned after editing, run the centering fix script below before regenerating the PNG.
- If new colours are introduced in the diagram, add them to the `DF`, `DS`, and `DT` mappings in the script above so they render correctly in dark mode.

## Centering fix (if needed after editing the .excalidraw file)

```python
import json

path = 'docs/conduit-workspace-lifecycle.excalidraw'
data = json.load(open(path))
by_id = {el['id']: el for el in data['elements']}

for el in data['elements']:
    if el['type'] != 'text' or not el.get('containerId'):
        continue
    container = by_id.get(el['containerId'])
    if not container:
        continue
    text_h = el['fontSize'] * el.get('lineHeight', 1.25) * (el['text'].count('\n') + 1)
    el['y']      = container['y'] + (container['height'] - text_h) / 2
    el['height'] = text_h

with open(path, 'w') as f:
    json.dump(data, f, indent=2)
print('done')
```
