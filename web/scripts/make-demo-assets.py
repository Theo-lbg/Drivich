# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg
"""Generate the demo images and PDFs used by the static GitHub Pages demo
(web/demo-assets). Everything is drawn from scratch, so nothing third-party is
shipped. Needs Pillow:  python3 web/scripts/make-demo-assets.py"""
import math, os, random
from PIL import Image, ImageDraw, ImageFilter, ImageFont

OUT = os.path.join(os.path.dirname(__file__), '..', 'demo-assets')
os.makedirs(os.path.join(OUT, 'photos'), exist_ok=True)
os.makedirs(os.path.join(OUT, 'thumbs'), exist_ok=True)
os.makedirs(os.path.join(OUT, 'docs'), exist_ok=True)
rnd = random.Random(7)
PAL = [((255,183,94),(237,117,87),(70,52,110)), ((120,200,255),(80,140,230),(30,60,110)),
       ((255,214,140),(250,150,120),(60,40,90)), ((170,230,210),(70,170,170),(20,70,90)),
       ((250,170,200),(190,110,200),(50,40,100)), ((200,220,255),(120,150,220),(40,50,90)),
       ((255,235,170),(245,170,90),(90,50,70)), ((190,215,255),(140,120,220),(35,35,85))]

def photo(i, w=1280, h=853):
    top, mid, dark = PAL[i % len(PAL)]
    im = Image.new('RGB', (w, h)); d = ImageDraw.Draw(im)
    for y in range(h):
        t = y / h; d.line([(0, y), (w, y)], fill=tuple(int(top[k]*(1-t)+mid[k]*t) for k in range(3)))
    sx, sy = rnd.randint(w//4, 3*w//4), rnd.randint(h//6, h//2)
    for r in range(180, 0, -5):
        a = r / 180; d.ellipse([sx-r, sy-r, sx+r, sy+r], fill=tuple(int(255*(1-a)+mid[k]*a) for k in range(3)))
    for layer in range(4):
        base = h*(0.55+layer*0.1); col = tuple(int(dark[k]*(0.5+layer*0.17)) for k in range(3))
        ph = rnd.random()*6; amp = h*0.12; pts = [(0, h)]
        for x in range(0, w+20, 20): pts.append((x, base-amp*abs(math.sin(x/(w/(3+layer))+ph))-layer*8))
        pts.append((w, h)); d.polygon(pts, fill=col)
    return im.filter(ImageFilter.GaussianBlur(0.7))

for i in range(12):
    im = photo(i); im.save(f'{OUT}/photos/p{i+1:02d}.jpg', quality=80, optimize=True)
    t = im.copy(); t.thumbnail((480, 480)); t.save(f'{OUT}/thumbs/p{i+1:02d}.jpg', quality=78, optimize=True)

def pdf(name, title):
    im = Image.new('RGB', (1240, 1754), 'white'); d = ImageDraw.Draw(im)
    f = ImageFont.truetype('/System/Library/Fonts/Helvetica.ttc', 64); g = ImageFont.truetype('/System/Library/Fonts/Helvetica.ttc', 32)
    d.rectangle([0, 0, 1240, 40], fill=(80, 60, 200)); d.text((100, 160), title, font=f, fill=(20, 30, 60))
    for i in range(18): d.rectangle([100, 320+i*70, 1140-rnd.randint(0, 400), 340+i*70], fill=(220, 222, 230))
    d.text((100, 1600), 'Document de démonstration', font=g, fill=(120, 120, 130))
    im.save(f'{OUT}/docs/{name}.pdf', resolution=100)

for n, t in [('rapport', 'Rapport annuel 2026'), ('contrat', 'Contrat de location'), ('plan', 'Plan de projet'),
             ('facture-mars', 'Facture mars'), ('facture-avril', 'Facture avril'), ('cahier', 'Cahier des charges')]:
    pdf(n, t)
