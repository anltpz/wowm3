"""Pyungmoo canavar ve metin bölgelerini wiki haritalarından çıkarır.

Canavar haritası (564x730, "BlueDiamond"): renkli bölgeler + gösterge. Metin haritası (1772x1417):
yarı saydam renkli bölgeler. İkisi de ana haritaya (1024x1280, 1 px = 1 m) yer işaretleriyle
(Bakra meydanı, Yaşlı Adam, ada) hizalanır. Çıktı (assets/pyungmoo/, 1/4 çözünürlük 256x320):
  mobzones.png  — 0 = yok, 1..10 = gösterge sırası
  metinzones.png — 0 = yok, 1..5 = Üzüntü, Dövüş, Savaş, Hırs, Siyah
Kullanım: python -I tools/build_zones.py <ref klasörü>
"""
import os
import sys

import numpy as np
from PIL import Image
from scipy import ndimage as ndi

REF = sys.argv[1]
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "pyungmoo")
W, H = 1024, 1280
S = 4  # çıktı ölçeği
gw, gh = W // S, H // S
gy, gx = np.mgrid[0:gh, 0:gw]
mx = gx * S + S / 2
my = gy * S + S / 2
walk = np.asarray(Image.open(os.path.join(OUT, "water.png")).resize((gw, gh))) < 128

# ---------- Canavar bölgeleri ----------
mm = np.asarray(Image.open(os.path.join(REF, "Pyungmoo_2.jpg")).convert("RGB")).astype(np.float32)
rows = [650, 666, 682, 699, 715]
sw = []
for x in (24, 320):
    for y in rows:
        sw.append(mm[y - 2:y + 3, x - 3:x + 4].reshape(-1, 3).mean(0))
sw = np.array(sw)
# ana harita → canavar haritası dönüşümü (yer işaretlerinden)
sx, ox = 0.506, 29.2
sy, oy = 0.397, 96.0
px = np.clip((mx * sx + ox).astype(int), 0, mm.shape[1] - 1)
py = np.clip((my * sy + oy).astype(int), 0, mm.shape[0] - 1)
samp = mm[py, px]
# Taban haritayla karşılaştırmak zor (bölgeler opak değil); en yakın gösterge rengi + eşik.
dist = np.sqrt(((samp[:, :, None, :] - sw[None, None]) ** 2).sum(-1))
best = dist.argmin(-1)
ok = dist.min(-1) < 48
mob = np.where(ok, best + 1, 0).astype(np.uint8)
# Çoğunluk süzgeci (gürültü + yazılar).
def mode_filter(a, k, n):
    out = np.zeros_like(a)
    bestc = np.zeros(a.shape, np.float32)
    for c in range(1, n + 1):
        f = ndi.uniform_filter((a == c).astype(np.float32), size=k)
        sel = f > bestc
        out[sel] = c
        bestc[sel] = f[sel]
    out[bestc < 0.2] = 0
    return out
mob = mode_filter(mob, 9, 10)
mob[~walk] = 0
Image.fromarray(mob, "L").save(os.path.join(OUT, "mobzones.png"))

# ---------- Metin bölgeleri ----------
mk = np.asarray(Image.open(os.path.join(REF, "Pyungmoo-Metinkarte.jpg")).convert("RGB")).astype(np.float32)
base = np.asarray(Image.open(os.path.join(REF, "Pyungmoo.jpg")).convert("RGB")).astype(np.float32)
k = 1.107
px = np.clip((mx * k + 2).astype(int), 0, mk.shape[1] - 1)
py = np.clip((my * k).astype(int), 0, mk.shape[0] - 1)
over = mk[py, px]
under = base[np.clip(my.astype(int), 0, H - 1), np.clip(mx.astype(int), 0, W - 1)]
diff = over - under
# Bindirme renkleri (yaklaşık katkı yönü): yeşil, camgöbeği, sarı, eflatun, kırmızı.
dirs = np.array([[-0.2, 1.0, -0.4], [-0.6, 0.8, 0.8], [0.8, 0.9, -0.6], [0.9, -0.3, 0.9], [1.0, -0.5, -0.5]], np.float32)
dirs /= np.linalg.norm(dirs, axis=1, keepdims=True)
mag = np.linalg.norm(diff, axis=-1) + 1e-3
cos = (diff[:, :, None, :] * dirs[None, None]).sum(-1) / mag[:, :, None]
bestm = cos.argmax(-1)
metin = np.where((cos.max(-1) > 0.75) & (mag > 28), bestm + 1, 0).astype(np.uint8)
metin = mode_filter(metin, 11, 5)
metin[~walk] = 0
Image.fromarray(metin, "L").save(os.path.join(OUT, "metinzones.png"))

# Önizleme
pal = np.array([[0, 0, 0], [120, 130, 200], [180, 180, 255], [60, 60, 100], [30, 90, 160], [90, 170, 170],
                [170, 210, 110], [240, 210, 180], [150, 85, 30], [220, 95, 85], [70, 70, 65]], np.uint8)
Image.fromarray(pal[mob]).resize((512, 640), Image.NEAREST).save(os.path.join(OUT, "mobzones_preview.png"))
palm = np.array([[0, 0, 0], [30, 160, 30], [40, 220, 210], [230, 230, 60], [220, 50, 210], [200, 30, 30]], np.uint8)
Image.fromarray(palm[metin]).resize((512, 640), Image.NEAREST).save(os.path.join(OUT, "metinzones_preview.png"))
for c in range(1, 11):
    print("mob", c, int((mob == c).sum()))
for c in range(1, 6):
    print("metin", c, int((metin == c).sum()))
