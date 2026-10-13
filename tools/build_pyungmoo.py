"""Pyungmoo (Metin2 Jinno Map1) referans haritasından arazi verisi üretir.

Girdi: Metin2 wiki'deki 1024x1280 kuşbakışı harita (1 piksel = 1 metre, oyun koordinatlarıyla aynı).
Çıktı (assets/pyungmoo/):
  height.bin  — f32 LE, (H/2+1) x (W/2+1) köşe yükseklikleri, 2 m aralık
  splat.png   — RGBA: R=yol, G=kaya, B=kum, A=dere yatağı (çim = kalan)
  water.png   — L: su maskesi (deniz + nehir)
  trees.json  — [[x, y, tür], ...] (tür 0 = yaprak, 1 = çam, 2 = sonbahar)
  preview.png — gölgelendirilmiş kontrol görüntüsü
Kullanım: python -I tools/build_pyungmoo.py <referans.jpg>
"""
import json
import os
import sys

import numpy as np
from PIL import Image, ImageFilter

try:
    from scipy import ndimage as ndi
except ImportError:  # pragma: no cover
    ndi = None

SRC = sys.argv[1]
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "pyungmoo")
os.makedirs(OUT, exist_ok=True)

img = Image.open(SRC).convert("RGB")
W, H = img.size
rgb = np.asarray(img).astype(np.float32)
blur = np.asarray(img.filter(ImageFilter.GaussianBlur(2))).astype(np.float32)

# --- Renk kümeleri (k-means merkezleri, cluster.py ile bulundu) ---
CENTERS = np.array([
    [75, 96, 91],    # 0 su
    [101, 81, 51],   # 1 yol
    [36, 39, 23],    # 2 çok koyu (gölge / hendek / ağaç)
    [91, 87, 70],    # 3 kaya orta
    [52, 65, 38],    # 4 çim
    [74, 69, 46],    # 5 geçiş
    [140, 130, 111], # 6 kaya açık
    [117, 109, 89],  # 7 kaya
    [47, 50, 33],    # 8 koyu (dağ gölgesi / koyu çim)
], np.float32)
d = ((blur[:, :, None, :] - CENTERS[None, None]) ** 2).sum(-1)
lab = d.argmin(-1)

water = lab == 0
road = lab == 1
grass = lab == 4
rockish = np.isin(lab, [3, 6, 7])
darkish = np.isin(lab, [2, 8])

def smooth(mask, r):
    return ndi.uniform_filter(mask.astype(np.float32), size=r)

# Su: küçük gürültüyü temizle.
water = ndi.binary_opening(water, iterations=2)
water = ndi.binary_closing(water, iterations=3)
lbl, n = ndi.label(water)
sizes = ndi.sum(water, lbl, range(1, n + 1))
water = np.isin(lbl, 1 + np.nonzero(sizes > 400)[0])
# Deniz = suyun doğuya bağlı en büyük parçası (x > 780 sütununu içeren).
lbl, n = ndi.label(water)
sea_ids = set(np.unique(lbl[:, W - 5])) - {0}
sea = np.isin(lbl, list(sea_ids))
river = water & ~sea

# Dağ maskesi: geniş pencerede kaya/koyu oranı yüksek bölgeler.
rock_frac = smooth(rockish | (darkish & ~smooth(grass, 9).__gt__(0.45)), 21)
mountain = (rock_frac > 0.55) & ~water
mountain = ndi.binary_opening(mountain, iterations=2)
lbl, n = ndi.label(mountain)
sizes = ndi.sum(mountain, lbl, range(1, n + 1))
# Köy meydanları gibi küçük açık lekeler dağ sayılmasın.
mountain = np.isin(lbl, 1 + np.nonzero(sizes > 3500)[0])

# Köy: meydan çevresi düz (wiki NPC koordinatları x 280-470, y 530-810).
yy, xx = np.mgrid[0:H, 0:W]
village = ((xx - 378) ** 2 / 115.0 ** 2 + (yy - 655) ** 2 / 150.0 ** 2) < 1.0
mountain &= ~village
PLAZAS = [(382, 645, 27), (480, 735, 16), (132, 1114, 18), (136, 126, 18), (448, 930, 16), (296, 268, 14), (624, 290, 14), (186, 560, 14), (685, 912, 14)]
plaza = np.zeros_like(water)
for (cx, cy, r) in PLAZAS:
    plaza |= ((xx - cx) ** 2 + (yy - cy) ** 2) < r * r
mountain &= ~plaza

# --- Yükseklik ---
dist_in = ndi.distance_transform_edt(mountain)
dist_out = ndi.distance_transform_edt(~mountain)
lum = blur.mean(-1)
detail = ndi.gaussian_filter(lum - ndi.gaussian_filter(lum, 25), 2)

h = np.zeros((H, W), np.float32)
# Ovalar: hafif tümsekler + dağa yaklaştıkça etek yükselmesi.
noise = ndi.gaussian_filter(np.random.default_rng(7).standard_normal((H, W)).astype(np.float32), 40) * 60
h += noise
h += np.clip(8.0 - dist_out * 0.18, 0, 8)
# Dağlar: içe doğru dik yükselen, doku ayrıntılı.
mh = 7.0 + np.minimum(dist_in, 60) * 0.9 + np.clip(dist_in - 60, 0, 200) * 0.25 + detail * 0.12
h = np.where(mountain, np.maximum(h, mh), h)
# Hendekler (doğu ovasındaki koyu şeritler): hafif çukur.
gully = (lab == 2) & ~mountain & ~water & ~village
gully = ndi.binary_opening(gully, iterations=1)
h -= ndi.gaussian_filter(gully.astype(np.float32), 3) * 6.0
# Köy ve meydanlar düz.
flat = village | plaza
h = np.where(flat, 1.0, h)
# Su: nehir yatağı ve deniz dibi.
dist_w = ndi.distance_transform_edt(~water)
dist_w_in = ndi.distance_transform_edt(water)
h = np.where(water, -1.5 - np.minimum(dist_w_in, 12) * 0.25, h)
# Kıyı: sudan uzaklaştıkça yükseklik kazanır (yumuşak kıyı).
shore_blend = np.clip(dist_w / 10.0, 0, 1)
h = np.where(~water, h * shore_blend + (0.4) * (1 - shore_blend), h)
# Deniz kıyısında kumsal.
dist_sea = ndi.distance_transform_edt(~sea)
beach = (~sea) & (dist_sea < 22) & ~mountain
h = ndi.gaussian_filter(h, 1.6)
# Köy düzlüğünü bulanıklıktan sonra yeniden uygula (kenarları yumuşat).
vb = ndi.gaussian_filter(flat.astype(np.float32), 6)
h = h * (1 - vb) + 1.0 * vb

# Köşe ızgarası (2 m).
gh, gw = H // 2 + 1, W // 2 + 1
ys = np.clip(np.arange(gh) * 2, 0, H - 1)
xs = np.clip(np.arange(gw) * 2, 0, W - 1)
grid = h[np.ix_(ys, xs)].astype("<f4")
grid.tofile(os.path.join(OUT, "height.bin"))

# --- Splat ---
road_w = ndi.gaussian_filter(road.astype(np.float32), 1.2)
road_w = np.clip(road_w * 1.15, 0, 0.9) * (~mountain)
road_w = np.maximum(road_w, ndi.gaussian_filter(plaza.astype(np.float32), 2))
rock_w = ndi.gaussian_filter(mountain.astype(np.float32), 2.5)
# Dik yamaçlar da kaya.
gy, gx = np.gradient(h)
slope = np.sqrt(gx * gx + gy * gy)
rock_w = np.clip(np.maximum(rock_w, (slope - 0.6) * 1.5), 0, 1)
sand_w = np.clip(ndi.gaussian_filter(beach.astype(np.float32), 3) * 1.3, 0, 1) * (1 - rock_w)
bed_w = np.clip(ndi.gaussian_filter((dist_w < 4).astype(np.float32), 2), 0, 1)
splat = np.stack([road_w, rock_w, sand_w, bed_w], -1)
Image.fromarray((np.clip(splat, 0, 1) * 255).astype(np.uint8), "RGBA").save(os.path.join(OUT, "splat.png"))
Image.fromarray((water * 255).astype(np.uint8), "L").save(os.path.join(OUT, "water.png"))

# --- Ağaçlar: çim üstündeki koyu/kızıl küçük lekeler ---
treeish = (np.isin(lab, [2, 8]) | ((rgb[:, :, 0] > rgb[:, :, 1] + 10) & (rgb[:, :, 0] > 70) & (rgb.mean(-1) < 110) & ~road)) & ~mountain & ~water & ~village & ~plaza
treeish &= smooth(grass | treeish, 15) > 0.55
lbl, n = ndi.label(treeish)
objs = ndi.find_objects(lbl)
trees = []
for i, sl in enumerate(objs):
    area = (lbl[sl] == i + 1).sum()
    if 12 <= area <= 220:
        cy, cx = ndi.center_of_mass(lbl[sl] == i + 1)
        y0, x0 = sl[0].start + cy, sl[1].start + cx
        r, g, b = rgb[int(y0), int(x0)]
        kind = 2 if r > g + 12 else 0
        trees.append([round(float(x0), 1), round(float(y0), 1), kind])
# Dağ yamaçlarına seyrek çamlar.
rng = np.random.default_rng(3)
cand = np.argwhere(mountain & (dist_in > 4) & (dist_in < 30))
for (y0, x0) in cand[rng.choice(len(cand), min(260, len(cand)), replace=False)]:
    trees.append([float(x0), float(y0), 1])
json.dump({"w": W, "h": H, "trees": trees}, open(os.path.join(OUT, "trees.json"), "w"))

# --- Önizleme (tepe gölgesi) ---
ls = np.clip(0.55 + (-gx * 0.7 - gy * 0.7) * 0.6, 0, 1)
base = np.zeros((H, W, 3), np.float32)
base[:] = [70, 110, 50]
base = base * (1 - road_w[..., None]) + np.array([140, 110, 70]) * road_w[..., None]
base = base * (1 - rock_w[..., None]) + np.array([150, 140, 125]) * rock_w[..., None]
base = base * (1 - sand_w[..., None]) + np.array([200, 185, 140]) * sand_w[..., None]
prev = base * ls[..., None]
prev[water] = [40, 90, 120]
for (x0, y0, k) in trees:
    prev[int(y0), int(x0)] = [10, 40, 10] if k != 2 else [120, 50, 20]
Image.fromarray(np.clip(prev, 0, 255).astype(np.uint8)).save(os.path.join(OUT, "preview.png"))
print("ok", W, H, "grid", gw, gh, "trees", len(trees), "h", float(h.min()), float(h.max()))
