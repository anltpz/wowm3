"""Orijinal WoW bölge ölçümü (ADT MCNK areaId + MCVT yükseklik + MCLQ su + MODF/MDDF yerleşimleri).

`elwynn.rs` / `westfall.rs` tablolarındaki alt bölge merkezleri, bölge kutusu, zemin yükseklikleri ve
bina konumları bu betikle ölçülür. MPQ okuyucu: `tools/mpq.py` (zlib, şifresiz).

Kullanım (çalışma dizini = depo kökü):
  python -I tools/adt_zones.py areas  <Bölge adı>             # alt bölgeler: merkez, yarıçap, kutu
  python -I tools/adt_zones.py circles <Bölge adı>            # Rust `Subzone` çemberleri + isabet oranı
  python -I tools/adt_zones.py wmo    <x0> <x1> <y0> <y1>     # MODF bina konumları (WoW x/y/z)
  python -I tools/adt_zones.py doodad <x0> <x1> <y0> <y1> [filtre]   # MDDF doodad konumları
  python -I tools/adt_zones.py probe  <x> <y> [<x> <y> ...]   # zemin z, su, alan adı, bina üstü mü
  python -I tools/adt_zones.py check  <rs dosyası>            # SpawnRow / MetinSlotRow / NpcDef satırlarını sına
Veri: WOW_DATA ortam değişkeni ya da C:/Users/ANIL/Desktop/AzerothSurvivors/Data (patch-2.MPQ önce).

Koordinatlar WoW dünya koordinatı: x kuzey, y batı, z yukarı. ADT dosyası Azeroth_<tx>_<ty>:
tx = floor(32 - y/TILE), ty = floor(32 - x/TILE). MCNK konumu (position) = (x, y, z) köşe; parça x ve y
yönünde azalarak 33.33 yd uzanır.
"""
import math
import os
import re
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from mpq import MPQ  # noqa: E402

TILE = 1600.0 / 3.0
CHUNK = TILE / 16.0
UNIT = CHUNK / 8.0
ORIGIN = 32.0 * TILE

DATA = os.environ.get("WOW_DATA", r"C:/Users/ANIL/Desktop/AzerothSurvivors/Data")
ARCHIVES = [MPQ(os.path.join(DATA, n)) for n in ("patch-2.MPQ", "patch.MPQ") if os.path.exists(os.path.join(DATA, n))]


def read(name):
    for a in ARCHIVES:
        try:
            d = a.read(name)
        except Exception:
            d = None
        if d is not None:
            return d
    return None


# ---------------- DBC ----------------
def dbc(name):
    d = read("DBFilesClient\\" + name)
    magic, n, fields, size, ssize = struct.unpack_from("<4sIIII", d, 0)
    assert magic == b"WDBC"
    recs = [d[20 + i * size: 20 + (i + 1) * size] for i in range(n)]
    strs = d[20 + n * size:]
    return recs, strs, fields


def cstr(strs, off):
    if off <= 0 or off >= len(strs):
        return ""
    return strs[off: strs.index(b"\0", off)].decode("utf-8", "replace")


def area_table():
    """id -> (ad, üst bölge id). 1.12: 0 ID, 1 harita, 2 üst alan, ..., 11 ad (enUS)."""
    recs, strs, _ = dbc("AreaTable.dbc")
    out = {}
    for r in recs:
        v = struct.unpack_from("<%dI" % (len(r) // 4), r)
        out[v[0]] = (cstr(strs, v[11]), v[2], v[1])
    return out


# ---------------- ADT ----------------
class Chunk:
    __slots__ = ("x", "y", "z", "area", "heights", "liquid", "flags", "holes")


class Adt:
    def __init__(self, tx, ty):
        self.tx, self.ty = tx, ty
        d = read("World\\Maps\\Azeroth\\Azeroth_%d_%d.adt" % (tx, ty))
        self.ok = d is not None
        self.chunks = []
        self.wmos = []
        self.doodads = []
        if not self.ok:
            return
        mmdx = mwmo = b""
        mmid = mwid = []
        pos = 0
        while pos + 8 <= len(d):
            tag = d[pos:pos + 4][::-1].decode("latin1")
            size = struct.unpack_from("<I", d, pos + 4)[0]
            body = d[pos + 8: pos + 8 + size]
            if tag == "MMDX":
                mmdx = body
            elif tag == "MMID":
                mmid = struct.unpack("<%dI" % (size // 4), body)
            elif tag == "MWMO":
                mwmo = body
            elif tag == "MWID":
                mwid = struct.unpack("<%dI" % (size // 4), body)
            elif tag == "MDDF":
                for i in range(size // 36):
                    nid, uid, px, py, pz, rx, ry, rz, sc, fl = struct.unpack_from("<II3f3fHH", body, i * 36)
                    nm = mmdx[mmid[nid]: mmdx.index(b"\0", mmid[nid])].decode("latin1") if nid < len(mmid) else "?"
                    self.doodads.append((uid, nm, ORIGIN - pz, ORIGIN - px, py, sc / 1024.0))
            elif tag == "MODF":
                for i in range(size // 64):
                    v = struct.unpack_from("<II3f3f3f3fHHHH", body, i * 64)
                    nid, uid = v[0], v[1]
                    px, py, pz = v[2:5]
                    e0, e1 = v[8:11], v[11:14]
                    nm = mwmo[mwid[nid]: mwmo.index(b"\0", mwid[nid])].decode("latin1") if nid < len(mwid) else "?"
                    # Kapsam kutusu da yerleşim koordinatında (x/z yer düzlemi).
                    xs = sorted([ORIGIN - e0[2], ORIGIN - e1[2]])
                    ys = sorted([ORIGIN - e0[0], ORIGIN - e1[0]])
                    zs = sorted([e0[1], e1[1]])
                    self.wmos.append((uid, nm, ORIGIN - pz, ORIGIN - px, py, xs, ys, zs))
            elif tag == "MCNK":
                self.chunks.append(self.mcnk(d, pos))
            pos += 8 + size

    @staticmethod
    def mcnk(d, pos):
        h = pos + 8
        flags, ix, iy = struct.unpack_from("<III", d, h)
        ofs_h = struct.unpack_from("<I", d, h + 0x14)[0]
        area = struct.unpack_from("<I", d, h + 0x34)[0]
        holes = struct.unpack_from("<I", d, h + 0x3C)[0] & 0xFFFF
        ofs_lq, size_lq = struct.unpack_from("<II", d, h + 0x60)
        z0, x0, zz = struct.unpack_from("<3f", d, h + 0x68)
        c = Chunk()
        # position = (x kuzey, y batı, z): wowdev "C3Vector position" (köşe).
        c.x, c.y, c.z = z0, x0, zz
        c.flags, c.area, c.holes = flags, area, holes
        hv = struct.unpack_from("<145f", d, pos + ofs_h + 8)
        c.heights = [zz + v for v in hv]
        c.liquid = None
        if flags & 0x3C and size_lq > 8:
            q = pos + ofs_lq
            if d[q:q + 4] in (b"QLCM", b"MCLQ"):
                q += 8
            lo, hi = struct.unpack_from("<2f", d, q)
            verts = [struct.unpack_from("<f", d, q + 8 + i * 8 + 4)[0] for i in range(81)]
            tiles = d[q + 8 + 81 * 8: q + 8 + 81 * 8 + 64]
            c.liquid = (lo, hi, verts, tiles)
        return c


ADTS = {}


def adt(tx, ty):
    k = (tx, ty)
    if k not in ADTS:
        ADTS[k] = Adt(tx, ty)
    return ADTS[k]


def tile_of(x, y):
    return int(math.floor(32 - y / TILE)), int(math.floor(32 - x / TILE))


def chunk_at(x, y):
    tx, ty = tile_of(x, y)
    a = adt(tx, ty)
    if not a.ok:
        return None
    for c in a.chunks:
        if c.x - CHUNK <= x <= c.x and c.y - CHUNK <= y <= c.y:
            return c
    return None


def height_in(c, x, y):
    """MCVT çift doğrusal yükseklik (dış 9x9 ızgara)."""
    fx = (c.y - y) / UNIT  # sütun: batı→doğu (y azalır)
    fy = (c.x - x) / UNIT  # satır: kuzey→güney (x azalır)
    col = min(max(int(fx), 0), 7)
    row = min(max(int(fy), 0), 7)
    u, v = fx - col, fy - row

    def outer(r, cl):
        return c.heights[r * 17 + cl]
    h00, h01 = outer(row, col), outer(row, col + 1)
    h10, h11 = outer(row + 1, col), outer(row + 1, col + 1)
    return (h00 * (1 - u) + h01 * u) * (1 - v) + (h10 * (1 - u) + h11 * u) * v, row, col


def probe(x, y):
    c = chunk_at(x, y)
    if c is None:
        return None
    z, row, col = height_in(c, x, y)
    water = None
    if c.liquid:
        lo, hi, verts, tiles = c.liquid
        t = tiles[row * 8 + col] if len(tiles) == 64 else 0
        lz = verts[row * 9 + col]
        if (t & 0x0F) != 0x0F and lz > z:
            water = lz - z
    return {"z": z, "area": c.area, "water": water}


def wmo_over(x, y, z, margin=1.0):
    """(x, y) noktasını örten bina kapsamları (çatı/iç mekân riski)."""
    tx, ty = tile_of(x, y)
    out, seen = [], set()
    for dx in (-1, 0, 1):
        for dy in (-1, 0, 1):
            a = adt(tx + dx, ty + dy)
            for w in a.wmos:
                xs, ys, zs = w[5], w[6], w[7]
                if w[0] in seen:
                    continue
                if xs[0] - margin <= x <= xs[1] + margin and ys[0] - margin <= y <= ys[1] + margin and zs[1] > z + 2:
                    seen.add(w[0])
                    out.append(w)
    return out


def zone_tiles(zone_id, areas):
    """Bölgenin (ve alt alanlarının) bulunduğu bütün ADT'leri Azeroth haritasında tara."""
    ids = {zone_id} | {i for i, (_, p, _) in areas.items() if p == zone_id}
    res = []
    for tx in range(20, 45):
        for ty in range(25, 60):
            if not read("World\\Maps\\Azeroth\\Azeroth_%d_%d.adt" % (tx, ty)) and (tx, ty) not in ADTS:
                continue
            a = adt(tx, ty)
            for c in a.chunks:
                if c.area in ids:
                    res.append(c)
    return ids, res


def cmd_areas(name):
    areas = area_table()
    zid = [i for i, (n, p, m) in areas.items() if n == name and p == 0 and m == 0]
    if not zid:
        sys.exit("bölge yok: " + name)
    zid = zid[0]
    ids, chunks = zone_tiles(zid, areas)
    xs = [c.x - CHUNK / 2 for c in chunks]
    ys = [c.y - CHUNK / 2 for c in chunks]
    print("bölge %s id=%d parça=%d kutu x[%.0f, %.0f] y[%.0f, %.0f]" % (name, zid, len(chunks), min(xs), max(xs), min(ys), max(ys)))
    by = {}
    for c in chunks:
        by.setdefault(c.area, []).append(c)
    rows = []
    for aid, cs in by.items():
        cx = sum(c.x - CHUNK / 2 for c in cs) / len(cs)
        cy = sum(c.y - CHUNK / 2 for c in cs) / len(cs)
        r = math.sqrt(len(cs) * CHUNK * CHUNK / math.pi)
        far = max(math.hypot(c.x - CHUNK / 2 - cx, c.y - CHUNK / 2 - cy) for c in cs)
        zs = [probe(c.x - CHUNK / 2, c.y - CHUNK / 2)["z"] for c in cs]
        wet = sum(1 for c in cs if c.liquid) / len(cs)
        rows.append((r, areas.get(aid, ("?",))[0], aid, cx, cy, far, sum(zs) / len(zs), len(cs), wet))
    for r, n, aid, cx, cy, far, z, k, wet in sorted(rows):
        print("  %-28s id=%-4d merkez (%.0f, %.0f) z~%.0f  eşdeğer r=%.0f  en uzak=%.0f  parça=%d  su=%.0f%%" % (n, aid, cx, cy, z, r, far, k, wet * 100))


def kmeans(pts, k, it=30):
    cs = [pts[int(i * len(pts) / k)] for i in range(k)]
    for _ in range(it):
        groups = [[] for _ in cs]
        for p in pts:
            j = min(range(len(cs)), key=lambda j: (p[0] - cs[j][0]) ** 2 + (p[1] - cs[j][1]) ** 2)
            groups[j].append(p)
        cs = [(sum(p[0] for p in g) / len(g), sum(p[1] for p in g) / len(g)) if g else cs[j] for j, g in enumerate(groups)]
    return cs, groups


def cmd_circles(name, skip=("The Great Sea", "Not Used Deadmines")):
    """Alt bölgeleri (ad, x, y, r) çemberlerine dönüştürür (uzun alanlar k-means ile 2-4 çember);
    küçükler önce sıralanır ve bütün parçalarda isabet ölçülür. Çıktı Rust `Subzone` satırları."""
    areas = area_table()
    zid = [i for i, (n, p, m) in areas.items() if n == name and p == 0 and m == 0][0]
    _, chunks = zone_tiles(zid, areas)
    by = {}
    for c in chunks:
        by.setdefault(c.area, []).append((c.x - CHUNK / 2, c.y - CHUNK / 2))
    circles = []
    for aid, pts in by.items():
        n = areas.get(aid, ("?",))[0]
        if aid == zid or n in skip:
            continue
        cx = sum(p[0] for p in pts) / len(pts)
        cy = sum(p[1] for p in pts) / len(pts)
        req = math.sqrt(len(pts) * CHUNK * CHUNK / math.pi)
        far = max(math.hypot(p[0] - cx, p[1] - cy) for p in pts)
        k = max(1, min(6, int(far / (1.5 * req)) + 1))
        cs, groups = kmeans(pts, k)
        for (gx, gy), g in zip(cs, groups):
            if len(g) < 3:
                continue
            ds = sorted(math.hypot(p[0] - gx, p[1] - gy) for p in g)
            r = ds[int(len(ds) * 0.8)] + CHUNK * 0.5
            circles.append((round(r / 5) * 5, n, round(gx), round(gy)))
    circles.sort()
    ok = 0
    miss = {}
    for c in chunks:
        x, y = c.x - CHUNK / 2, c.y - CHUNK / 2
        want = areas.get(c.area, ("?",))[0]
        if c.area == zid or want in skip:
            want = name
        got = next((n for r, n, cx, cy in circles if math.hypot(x - cx, y - cy) <= r), name)
        if got == want:
            ok += 1
        else:
            miss[(want, got)] = miss.get((want, got), 0) + 1
    for r, n, cx, cy in circles:
        print('    ("%s", %.1f, %.1f, %.1f),' % (n.replace('"', '\\"'), cx, cy, r))
    print("isabet %.1f%% (%d/%d)" % (100.0 * ok / len(chunks), ok, len(chunks)))
    for (w, g), k in sorted(miss.items(), key=lambda t: -t[1])[:12]:
        print("  %s -> %s: %d" % (w, g, k))


def in_box(x, y, b):
    return b[0] <= x <= b[1] and b[2] <= y <= b[3]


def all_tiles_box(b):
    t = set()
    for x in (b[0], b[1], (b[0] + b[1]) / 2):
        for y in (b[2], b[3], (b[2] + b[3]) / 2):
            t.add(tile_of(x, y))
    tx0, tx1 = min(a for a, _ in t), max(a for a, _ in t)
    ty0, ty1 = min(b_ for _, b_ in t), max(b_ for _, b_ in t)
    return [(i, j) for i in range(tx0, tx1 + 1) for j in range(ty0, ty1 + 1)]


def cmd_wmo(b):
    seen = set()
    for t in all_tiles_box(b):
        for w in adt(*t).wmos:
            if w[0] in seen or not in_box(w[2], w[3], b):
                continue
            seen.add(w[0])
            print("  %-60s (%.1f, %.1f, %.1f)  kutu x[%.0f,%.0f] y[%.0f,%.0f] z[%.0f,%.0f]" % (w[1], w[2], w[3], w[4], *w[5], *w[6], *w[7]))


def cmd_doodad(b, flt):
    seen = set()
    for t in all_tiles_box(b):
        for dd in adt(*t).doodads:
            if dd[0] in seen or not in_box(dd[2], dd[3], b):
                continue
            if flt and flt.lower() not in dd[1].lower():
                continue
            seen.add(dd[0])
            print("  %-60s (%.1f, %.1f, %.1f) ölçek %.2f" % (dd[1], dd[2], dd[3], dd[4], dd[5]))


def describe(x, y, z=None, areas=None):
    p = probe(x, y)
    if p is None:
        return "ADT yok"
    n = areas.get(p["area"], ("?",))[0] if areas else p["area"]
    s = "z=%.1f alan=%s" % (p["z"], n)
    if p["water"]:
        s += " SU(derinlik %.1f)" % p["water"]
    w = wmo_over(x, y, p["z"])
    if w:
        s += " BİNA:" + ",".join(os.path.basename(i[1]) for i in w[:2])
    if z is not None and abs(z - p["z"]) > 6:
        s += " Z FARKI %.0f" % (z - p["z"])
    return s


def cmd_check(path):
    """Rust tablosundaki satırları sına: merkez + yarıçap çemberinde 12 örnek nokta (su/bina oranı)."""
    areas = area_table()
    src = open(path, encoding="utf-8").read()
    num = r"(-?\d+(?:\.\d+)?)"
    spawn = re.compile(r"\(&\[([\d, ]+)\],\s*" + r",\s*".join([num] * 4) + r",\s*(\d+),\s*(\d+),\s*(\d+)\)")
    slot = re.compile(r"^\s*\((\d+),\s*" + r",\s*".join([num] * 4) + r"\),", re.M)
    npc = re.compile(r'NpcDef \{\s*name:\s*"([^"]+)".*?map:\s*\(' + num + r",\s*" + num + r"\).*?z:\s*" + num, re.S)
    bad = 0
    for kind, rows in (("spawn", spawn.findall(src)), ("metin", slot.findall(src))):
        for r in rows:
            if kind == "spawn":
                label, x, y, z, rad = r[0], *map(float, r[1:5])
            else:
                label, x, y, z, rad = "metin " + r[0], *map(float, r[1:5])
            pts = [(x, y)] + [(x + math.cos(a) * rad * k, y + math.sin(a) * rad * k)
                              for a in [i * math.pi / 4 for i in range(8)] for k in (0.5, 1.0)]
            wet = roof = 0
            zs = []
            for px, py in pts:
                p = probe(px, py)
                if p is None:
                    continue
                zs.append(p["z"])
                wet += bool(p["water"])
                roof += bool(wmo_over(px, py, p["z"], 0.0))
            pc = probe(x, y)
            name = areas.get(pc["area"], ("?",))[0] if pc else "?"
            warn = ""
            if wet > len(pts) // 3:
                warn += " SU%d/%d" % (wet, len(pts))
            if pc and abs(pc["z"] - z) > 15:
                warn += " zFARK(%.0f)" % pc["z"]
            if warn:
                bad += 1
            print("%-6s %-14s (%.0f, %.0f) r=%.0f z=%.0f zemin~%.0f [%.0f..%.0f] su=%d bina=%d  %s%s" % (
                kind, label, x, y, rad, z, pc["z"] if pc else 0, min(zs or [0]), max(zs or [0]), wet, roof, name, warn))
    for n, x, y, z in npc.findall(src):
        x, y, z = float(x), float(y), float(z)
        print("npc    %-24s (%.0f, %.0f, %.0f) %s" % (n, x, y, z, describe(x, y, z, areas)))
    print("uyarılı satır: %d" % bad)


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    a = sys.argv[1:]
    if not a:
        sys.exit(__doc__)
    if a[0] == "areas":
        cmd_areas(" ".join(a[1:]))
    elif a[0] == "circles":
        cmd_circles(" ".join(a[1:]))
    elif a[0] == "wmo":
        cmd_wmo(list(map(float, a[1:5])))
    elif a[0] == "doodad":
        cmd_doodad(list(map(float, a[1:5])), a[5] if len(a) > 5 else "")
    elif a[0] == "probe":
        areas = area_table()
        v = list(map(float, a[1:]))
        for i in range(0, len(v) - 1, 2):
            print("(%.0f, %.0f): %s" % (v[i], v[i + 1], describe(v[i], v[i + 1], None, areas)))
    elif a[0] == "check":
        cmd_check(a[1])
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
