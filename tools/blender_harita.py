"""AzerothMetin harita aracı (Blender 4.2+ / 5.x): Pyungmoo biçimindeki haritayı Blender'da düzenle, oyuna geri aktar.

Oyunun okuduğu dosyalar (`assets/pyungmoo/`, ya da `AZM_MAPDIR=<klasör>` ile çalışma anında):
  height.bin      f32 LE, 641 satır x 513 sütun köşe yüksekliği, 2 m aralık (harita 1024 x 1280 m)
  splat.png       RGBA 1024x1280: R = yol, G = kaya, B = kum, A = dere yatağı (kalan = çim)
  water.png       L 1024x1280: su maskesi (su yüzeyi sabit -0.35 m)
  metinzones.png  L 256x320: 0 = yok, 1..5 = Üzüntü, Dövüş, Savaş, Hırs, Siyah metin bölgesi
  trees.json      {"w": 1024, "h": 1280, "trees": [[x, y, tür], ...]}  tür 0 = yaprak, 1 = çam, 2 = sonbahar
Harita koordinatı: x sağa (doğu), y aşağı (güney), metre. Blender'da konum = (x, -y, yükseklik), yani üstten
bakınca (Numpad 7) kuzey yukarıda.

Blender sahnesi (içe aktarma bunu kurar):
  "Arazi"            513 x 641 köşeli ızgara. Sculpt ile şekillendir (dyntopo/remesh de olur: dışa aktarma ışınla örnekler).
  Görüntüler         "Yol", "Kaya", "Kum", "Dere", "Su" (1024x1280 gri: beyaz = var) ve "MetinBolge" (256x320, renk paleti).
                     Texture Paint modunda Arazi seçiliyken sağdaki "Texture Slots" listesinden katmanı seçip boya.
  Koleksiyonlar      "Agac_Yaprak", "Agac_Cam", "Agac_Sonbahar": içindeki her nesne bir ağaç (yalnızca x/y kullanılır).
                     Çoğaltmak için Alt+D (bağlı kopya), silmek için X.
  "Su_Yuzeyi"        yalnızca görsel (su seviyesi); dışa aktarılmaz, su "Su" katmanından gelir.

Kullanım
  1) İçe aktar (Pyungmoo'yu .blend'e çevir, komut satırı):
       blender -b --python tools/blender_harita.py -- ice-aktar assets/pyungmoo harita.blend
  2) harita.blend'i aç. Panel: 3B görünüm > N kenar çubuğu > "AzerothMetin" sekmesi (betik dosyaya gömülüdür;
     panel görünmezse Scripting sekmesinde "blender_harita.py" metnini açıp Run Script).
  3) Düzenle, panelden "Dışa aktar" (varsayılan klasör: .blend'in yanında harita_cikti/).
     Komut satırından:  blender -b harita.blend --python tools/blender_harita.py -- disa-aktar <klasör>
  4) Oyunda dene (derlemeden):  AZM_MAPDIR=<klasör>  (ör. tools/shot.ps1 -Env "AZM_MAPDIR=C:\\yol\\harita_cikti")
     Kalıcı yapmak için dosyaları assets/pyungmoo/'ya kopyala ve yeniden derle.
Kısıt: harita boyutu sabit 1024 x 1280 m (terrain.rs GW/GH). Köprüler, binalar, garnizon ve NPC'ler kodda
(terrain.rs BUILDINGS/BRIDGES, npc.rs); canavar bölgeleri mobs.rs PY_ZONES'ta.
"""
import json
import os
import struct
import sys
import warnings
import zlib

import bpy
import numpy as np

MAP_W, MAP_H = 1024, 1280
CELL = 2.0
GW, GH = MAP_W // 2 + 1, MAP_H // 2 + 1
ZW, ZH = MAP_W // 4, MAP_H // 4
WATER_LEVEL = -0.35

LAYERS = ["Yol", "Kaya", "Kum", "Dere"]  # splat RGBA sırası
LAYER_COLORS = {
    "Yol": (0.45, 0.33, 0.20, 1.0),
    "Kaya": (0.42, 0.42, 0.44, 1.0),
    "Kum": (0.76, 0.68, 0.48, 1.0),
    "Dere": (0.30, 0.26, 0.20, 1.0),
}
GRASS = (0.22, 0.38, 0.12, 1.0)
WATER_COLOR = (0.10, 0.28, 0.42, 1.0)
# Metin bölgesi paleti (indeks 1..5); boyarken bu renkleri kullan, siyah = bölge yok.
ZONE_NAMES = ["Üzüntü", "Dövüş", "Savaş", "Hırs", "Siyah"]
ZONE_COLORS = [(0.20, 0.45, 1.00), (1.00, 0.55, 0.10), (1.00, 0.15, 0.15), (0.75, 0.20, 1.00), (0.95, 0.95, 0.95)]
TREE_COLLS = ["Agac_Yaprak", "Agac_Cam", "Agac_Sonbahar"]
TREE_COLORS = [(0.15, 0.45, 0.12, 1.0), (0.05, 0.28, 0.12, 1.0), (0.85, 0.45, 0.10, 1.0)]
SCRIPT_TEXT = "blender_harita.py"


# --------------------------------------------------------------------------- PNG okuma/yazma (bağımlılıksız)

def png_write(path, arr):
    """uint8 dizisini PNG olarak yazar: (H, W) → gri (L), (H, W, 4) → RGBA. Satır 0 = üst."""
    arr = np.ascontiguousarray(arr, dtype=np.uint8)
    h, w = arr.shape[:2]
    color = 0 if arr.ndim == 2 else 6
    raw = b"".join(b"\x00" + arr[y].tobytes() for y in range(h))

    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n")
        f.write(chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, color, 0, 0, 0)))
        f.write(chunk(b"IDAT", zlib.compress(raw, 6)))
        f.write(chunk(b"IEND", b""))


def image_load_raw(path):
    """PNG'yi Blender ile yükler, (H, W, 4) float32 0..1 döndürür (satır 0 = üst, renk dönüşümü yok)."""
    img = bpy.data.images.load(path, check_existing=False)
    img.colorspace_settings.name = "Non-Color"
    w, h = img.size
    px = np.empty(w * h * 4, dtype=np.float32)
    img.pixels.foreach_get(px)
    bpy.data.images.remove(img)
    return px.reshape(h, w, 4)[::-1]


# --------------------------------------------------------------------------- Blender görüntüleri

def new_image(name, w, h, rgba_top_down):
    """(H, W, 4) float 0..1 diziden paketlenmiş, Non-Color bir Blender görüntüsü kurar."""
    old = bpy.data.images.get(name)
    if old:
        bpy.data.images.remove(old)
    img = bpy.data.images.new(name, w, h, alpha=True, float_buffer=False)
    img.colorspace_settings.name = "Non-Color"
    img.pixels.foreach_set(np.ascontiguousarray(rgba_top_down[::-1], dtype=np.float32).ravel())
    img.pack()
    return img


def gray_to_rgba(g):
    g = np.clip(g, 0.0, 1.0).astype(np.float32)
    return np.stack([g, g, g, np.ones_like(g)], -1)


def image_rgba(name):
    """Blender görüntüsünden (H, W, 4) float, satır 0 = üst."""
    img = bpy.data.images.get(name)
    if img is None:
        raise RuntimeError(f"'{name}' görüntüsü yok (içe aktarmayla kurulan sahneyi kullan)")
    w, h = img.size
    px = np.empty(w * h * 4, dtype=np.float32)
    img.pixels.foreach_get(px)
    return px.reshape(h, w, 4)[::-1], (w, h)


def image_gray(name, size):
    rgba, (w, h) = image_rgba(name)
    if (w, h) != size:
        raise RuntimeError(f"'{name}' boyutu {w}x{h}, beklenen {size[0]}x{size[1]}")
    return rgba[:, :, :3].mean(-1)


# --------------------------------------------------------------------------- malzeme

def use_nodes(mat):
    """Düğüm ağacını açar (Blender 5'te varsayılan; `use_nodes` 6.0'da kalkacak, uyarısı susturulur)."""
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", DeprecationWarning)
        mat.use_nodes = True


def _sock(sockets, name, kind):
    for s in sockets:
        if s.name == name and s.type == kind:
            return s
    raise RuntimeError(f"soket yok: {name} ({kind})")


def mix_node(nodes, links, a_out, b_color, fac_out, loc):
    """A ile sabit B rengini `fac` oranında karıştırır; çıktı soketi döner."""
    n = nodes.new("ShaderNodeMix")
    n.data_type = "RGBA"
    n.location = loc
    links.new(fac_out, _sock(n.inputs, "Factor", "VALUE"))
    a = _sock(n.inputs, "A", "RGBA")
    if isinstance(a_out, tuple):
        a.default_value = a_out
    else:
        links.new(a_out, a)
    _sock(n.inputs, "B", "RGBA").default_value = b_color
    return _sock(n.outputs, "Result", "RGBA")


def build_material():
    """Önizleme malzemesi: çim + katmanlar (Yol/Kaya/Kum/Dere) + su + isteğe bağlı metin bölgesi kaplaması.
    Görüntü düğümleri Texture Paint'in "Texture Slots" listesinde boyama hedefi olarak görünür."""
    mat = bpy.data.materials.get("Arazi") or bpy.data.materials.new("Arazi")
    use_nodes(mat)
    nodes, links = mat.node_tree.nodes, mat.node_tree.links
    nodes.clear()
    out = nodes.new("ShaderNodeOutputMaterial")
    out.location = (1400, 0)
    bsdf = nodes.new("ShaderNodeBsdfPrincipled")
    bsdf.location = (1150, 0)
    _sock(bsdf.inputs, "Roughness", "VALUE").default_value = 1.0
    links.new(bsdf.outputs[0], out.inputs["Surface"])
    uv = nodes.new("ShaderNodeUVMap")
    uv.location = (-700, 0)

    def tex(name, y):
        t = nodes.new("ShaderNodeTexImage")
        t.name = t.label = name
        t.image = bpy.data.images[name]
        t.interpolation = "Closest" if name == "MetinBolge" else "Linear"
        t.location = (-450, y)
        links.new(uv.outputs["UV"], t.inputs["Vector"])
        return t

    col = GRASS
    x = 0
    for i, name in enumerate(LAYERS + ["Su"]):
        t = tex(name, 300 - i * 280)
        col = mix_node(nodes, links, col, LAYER_COLORS.get(name, WATER_COLOR), t.outputs["Color"], (x, 300 - i * 150))
        x += 180
    # Metin bölgesi kaplaması: "Bölge görünürlüğü" değerini 0 → 1 yaparak aç.
    zt = tex("MetinBolge", -1200)
    show = nodes.new("ShaderNodeValue")
    show.name = show.label = "Bölge görünürlüğü"
    show.outputs[0].default_value = 0.0
    show.location = (-450, -1500)
    sep = nodes.new("ShaderNodeSeparateColor")
    sep.location = (-200, -1200)
    links.new(zt.outputs["Color"], sep.inputs["Color"])
    mx = nodes.new("ShaderNodeMath")
    mx.operation = "MAXIMUM"
    mx.location = (0, -1200)
    links.new(sep.outputs[0], mx.inputs[0])
    links.new(sep.outputs[2], mx.inputs[1])
    gate = nodes.new("ShaderNodeMath")
    gate.operation = "MULTIPLY"
    gate.use_clamp = True
    gate.location = (200, -1300)
    links.new(mx.outputs[0], gate.inputs[0])
    links.new(show.outputs[0], gate.inputs[1])
    zmix = nodes.new("ShaderNodeMix")
    zmix.data_type = "RGBA"
    zmix.location = (950, -300)
    links.new(gate.outputs[0], _sock(zmix.inputs, "Factor", "VALUE"))
    links.new(col, _sock(zmix.inputs, "A", "RGBA"))
    links.new(zt.outputs["Color"], _sock(zmix.inputs, "B", "RGBA"))
    links.new(_sock(zmix.outputs, "Result", "RGBA"), _sock(bsdf.inputs, "Base Color", "RGBA"))
    # Boyamaya hazır: ilk katman etkin.
    nodes.active = nodes["Yol"]
    return mat


# --------------------------------------------------------------------------- içe aktarma

def grid_mesh(name, heights):
    """GW x GH köşeli ızgara; köşe (i, j) indeksi j * GW + i (satır = güney yönünde)."""
    xs = np.arange(GW, dtype=np.float32) * CELL
    ys = np.arange(GH, dtype=np.float32) * CELL
    gx, gy = np.meshgrid(xs, ys)
    co = np.stack([gx, -gy, heights.astype(np.float32)], -1).reshape(-1, 3)
    qi, qj = np.meshgrid(np.arange(GW - 1), np.arange(GH - 1))
    a = (qj * GW + qi).ravel()
    quads = np.stack([a, a + GW, a + GW + 1, a + 1], -1)  # alttan bakınca saat yönü → normal yukarı
    me = bpy.data.meshes.new(name)
    me.vertices.add(len(co))
    me.vertices.foreach_set("co", co.ravel())
    nq = len(quads)
    me.loops.add(nq * 4)
    me.loops.foreach_set("vertex_index", quads.astype(np.int32).ravel())
    me.polygons.add(nq)
    me.polygons.foreach_set("loop_start", np.arange(0, nq * 4, 4, dtype=np.int32))
    me.update(calc_edges=True)
    uvl = me.uv_layers.new(name="UVMap")
    vi = quads.ravel()
    u = (vi % GW) / (GW - 1)
    v = 1.0 - (vi // GW) / (GH - 1)
    uvl.data.foreach_set("uv", np.stack([u, v], -1).astype(np.float32).ravel())
    me.validate()
    if hasattr(me, "shade_smooth"):
        me.shade_smooth()
    else:
        me.polygons.foreach_set("use_smooth", np.ones(len(me.polygons), dtype=bool))
    return me


def coll(name, parent=None):
    c = bpy.data.collections.get(name) or bpy.data.collections.new(name)
    parent = parent or bpy.context.scene.collection
    if c.name not in parent.children:
        parent.children.link(c)
    return c


def marker_mesh(name, kind):
    """Ağaç işaretçisi: koni (çam) ya da sekizgen gövde + tepe (yaprak/sonbahar); taban z = 0."""
    import bmesh

    me = bpy.data.meshes.get(name)
    if me:
        return me
    me = bpy.data.meshes.new(name)
    bm = bmesh.new()
    if kind == 1:
        bmesh.ops.create_cone(bm, cap_ends=True, segments=8, radius1=2.2, radius2=0.0, depth=9.0)
        bmesh.ops.translate(bm, verts=bm.verts, vec=(0, 0, 4.5))
    else:
        bmesh.ops.create_cone(bm, cap_ends=True, segments=6, radius1=0.4, radius2=0.4, depth=3.0)
        bmesh.ops.translate(bm, verts=bm.verts, vec=(0, 0, 1.5))
        ico = bmesh.ops.create_icosphere(bm, subdivisions=1, radius=3.0)
        bmesh.ops.translate(bm, verts=ico["verts"], vec=(0, 0, 5.0))
    bm.to_mesh(me)
    bm.free()
    m = bpy.data.materials.new(name)
    m.diffuse_color = TREE_COLORS[kind]
    use_nodes(m)
    _sock(m.node_tree.nodes["Principled BSDF"].inputs, "Base Color", "RGBA").default_value = TREE_COLORS[kind]
    me.materials.append(m)
    return me


def height_at(heights, x, y):
    i = min(max(int(round(x / CELL)), 0), GW - 1)
    j = min(max(int(round(y / CELL)), 0), GH - 1)
    return float(heights[j, i])


def import_map(src, blend_out):
    src = os.path.abspath(src)
    heights = np.fromfile(os.path.join(src, "height.bin"), dtype="<f4")
    if heights.size != GW * GH:
        raise RuntimeError(f"height.bin {heights.size} değer, beklenen {GW * GH}")
    heights = heights.reshape(GH, GW)
    splat = image_load_raw(os.path.join(src, "splat.png"))
    water = image_load_raw(os.path.join(src, "water.png"))[:, :, 0]
    zones = np.round(image_load_raw(os.path.join(src, "metinzones.png"))[:, :, 0] * 255).astype(np.int32)
    trees = json.load(open(os.path.join(src, "trees.json"), encoding="utf-8"))["trees"]

    # Boş sahne.
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.unit_settings.system = "METRIC"

    for i, name in enumerate(LAYERS):
        new_image(name, MAP_W, MAP_H, gray_to_rgba(splat[:, :, i]))
    new_image("Su", MAP_W, MAP_H, gray_to_rgba(water))
    pal = np.zeros((6, 4), dtype=np.float32)
    pal[:, 3] = 1.0
    pal[1:, :3] = ZONE_COLORS
    new_image("MetinBolge", ZW, ZH, pal[np.clip(zones, 0, 5)])

    me = grid_mesh("Arazi", heights)
    me.materials.append(build_material())
    terrain = bpy.data.objects.new("Arazi", me)
    coll("Harita").objects.link(terrain)

    wme = bpy.data.meshes.new("Su_Yuzeyi")
    wme.from_pydata([(0, 0, WATER_LEVEL), (MAP_W, 0, WATER_LEVEL), (MAP_W, -MAP_H, WATER_LEVEL), (0, -MAP_H, WATER_LEVEL)], [], [(0, 1, 2, 3)])
    wmat = bpy.data.materials.new("Su_Yuzeyi")
    wmat.diffuse_color = WATER_COLOR
    use_nodes(wmat)
    _sock(wmat.node_tree.nodes["Principled BSDF"].inputs, "Base Color", "RGBA").default_value = WATER_COLOR
    wme.materials.append(wmat)
    wobj = bpy.data.objects.new("Su_Yuzeyi", wme)
    wobj.hide_select = True
    wobj.hide_render = True
    wobj.display_type = "WIRE"
    coll("Harita").objects.link(wobj)

    root = coll("Agaclar")
    tcolls = [coll(n, root) for n in TREE_COLLS]
    meshes = [marker_mesh(f"{n}_isaret", k) for k, n in enumerate(TREE_COLLS)]
    for n, (x, y, kind) in enumerate(trees):
        k = int(kind) if int(kind) in (0, 1, 2) else 0
        ob = bpy.data.objects.new(f"{TREE_COLLS[k]}.{n:04d}", meshes[k])
        ob.location = (float(x), -float(y), height_at(heights, x, y))
        ob["sira"] = n  # oyun model çeşidini/ölçeği/dönüşü listedeki sıradan türetir: dışa aktarmada korunur
        tcolls[k].objects.link(ob)

    embed_script()
    # Üstten görünüm için kamera yok; görünüm ayarı .blend açılınca kullanıcıya kalır.
    scene["azeroth_harita"] = {"w": MAP_W, "h": MAP_H, "cell": CELL}
    bpy.ops.wm.save_as_mainfile(filepath=os.path.abspath(blend_out), compress=True)
    print(f"[harita] içe aktarıldı: {blend_out} ({len(trees)} ağaç)")


def embed_script():
    """Bu betiği .blend'e "kayıtlı modül" olarak gömer: dosya açılınca panel kendiliğinden gelir
    (Blender'da Python betiklerinin otomatik çalışmasına izin verildiyse)."""
    txt = bpy.data.texts.get(SCRIPT_TEXT) or bpy.data.texts.new(SCRIPT_TEXT)
    txt.from_string(open(os.path.abspath(__file__), encoding="utf-8").read())
    txt.use_module = True


# --------------------------------------------------------------------------- dışa aktarma

def sample_heights(obj, depsgraph):
    """Arazi yüksekliklerini 2 m ızgarada okur. Izgara bozulmadıysa köşelerden, yoksa yukarıdan ışınla."""
    ev = obj.evaluated_get(depsgraph)
    me = ev.to_mesh()
    try:
        mw = np.array(obj.matrix_world, dtype=np.float64)
        n = len(me.vertices)
        co = np.empty(n * 3, dtype=np.float32)
        me.vertices.foreach_get("co", co)
        co = co.reshape(-1, 3).astype(np.float64)
        co = co @ mw[:3, :3].T + mw[:3, 3]
        if n == GW * GH:
            xs = np.tile(np.arange(GW) * CELL, GH)
            ys = np.repeat(np.arange(GH) * CELL, GW)
            if np.abs(co[:, 0] - xs).max() < 0.05 and np.abs(-co[:, 1] - ys).max() < 0.05:
                return co[:, 2].reshape(GH, GW).astype(np.float32), "köşeler"
        from mathutils import Vector
        from mathutils.bvhtree import BVHTree

        tris = []
        me.calc_loop_triangles()
        for t in me.loop_triangles:
            tris.append(tuple(t.vertices))
        tree = BVHTree.FromPolygons([Vector(c) for c in co], tris)
        top = float(co[:, 2].max()) + 100.0
        down = Vector((0, 0, -1))
        out = np.empty((GH, GW), dtype=np.float32)
        missed = 0
        for j in range(GH):
            for i in range(GW):
                hit = tree.ray_cast(Vector((i * CELL, -j * CELL, top)), down)
                if hit[0] is None:
                    missed += 1
                    out[j, i] = out[j, i - 1] if i else (out[j - 1, i] if j else 0.0)
                else:
                    out[j, i] = hit[0].z
        if missed:
            print(f"[harita] uyarı: {missed} nokta arazide değil (önceki komşu yüksekliği kullanıldı)")
        return out, "ışın"
    finally:
        ev.to_mesh_clear()


def repack_painted():
    """Boyanmış (kirli) paketli görüntüleri yeniden paketler: .blend kaydedilince boyalar kaybolmasın."""
    for img in bpy.data.images:
        if img.is_dirty and img.packed_file is not None:
            img.pack()


def export_map(out_dir):
    repack_painted()
    out_dir = bpy.path.abspath(out_dir)
    os.makedirs(out_dir, exist_ok=True)
    terrain = bpy.data.objects.get("Arazi")
    if terrain is None:
        raise RuntimeError("'Arazi' nesnesi yok")
    heights, how = sample_heights(terrain, bpy.context.evaluated_depsgraph_get())
    heights.astype("<f4").tofile(os.path.join(out_dir, "height.bin"))

    splat = np.stack([image_gray(n, (MAP_W, MAP_H)) for n in LAYERS], -1)
    png_write(os.path.join(out_dir, "splat.png"), np.round(np.clip(splat, 0, 1) * 255))
    water = image_gray("Su", (MAP_W, MAP_H)) > 0.5
    png_write(os.path.join(out_dir, "water.png"), water.astype(np.uint8) * 255)

    zr, (zw, zh) = image_rgba("MetinBolge")
    if (zw, zh) != (ZW, ZH):
        raise RuntimeError(f"'MetinBolge' boyutu {zw}x{zh}, beklenen {ZW}x{ZH}")
    rgb = zr[:, :, :3]
    pal = np.array(ZONE_COLORS, dtype=np.float32)
    d = ((rgb[:, :, None, :] - pal[None, None]) ** 2).sum(-1)
    zones = d.argmin(-1).astype(np.uint8) + 1
    zones[rgb.max(-1) < 0.15] = 0  # siyah/koyu = bölge yok
    png_write(os.path.join(out_dir, "metinzones.png"), zones)

    # Sıra: içe aktarılan ağaçlar özgün sıralarıyla (`sira`), yeniler sonda (ada göre). Böylece dokunulmayan
    # ağaçların oyundaki görünümü (sıraya bağlı model çeşidi, ölçek, dönüş) değişmez.
    rows = []
    for k, n in enumerate(TREE_COLLS):
        c = bpy.data.collections.get(n)
        if not c:
            continue
        for ob in c.all_objects:
            x, y = ob.matrix_world.translation.x, -ob.matrix_world.translation.y
            if 0.0 <= x <= MAP_W and 0.0 <= y <= MAP_H:
                rows.append(((0, int(ob["sira"])) if "sira" in ob else (1, 0), ob.name, [round(x, 1), round(y, 1), k]))
    rows.sort(key=lambda r: (r[0], r[1]))
    trees = [r[2] for r in rows]
    with open(os.path.join(out_dir, "trees.json"), "w", encoding="utf-8") as f:
        json.dump({"w": MAP_W, "h": MAP_H, "trees": trees}, f)
    counts = [int((zones == i).sum()) for i in range(1, 6)]
    msg = (f"dışa aktarıldı: {out_dir} — yükseklik ({how}) {heights.min():.1f}..{heights.max():.1f} m, "
           f"su %{water.mean() * 100:.1f}, {len(trees)} ağaç, metin bölgeleri {dict(zip(ZONE_NAMES, counts))}")
    print("[harita] " + msg)
    return msg


# --------------------------------------------------------------------------- arayüz (N kenar çubuğu)

class AZM_OT_disa_aktar(bpy.types.Operator):
    bl_idname = "azm.harita_disa_aktar"
    bl_label = "Dışa aktar"
    bl_description = "Haritayı oyunun dosya biçiminde (height.bin, splat.png, water.png, metinzones.png, trees.json) yazar"

    def execute(self, context):
        try:
            msg = export_map(context.scene.azm_cikti)
        except Exception as e:  # noqa: BLE001 — kullanıcıya göster
            self.report({"ERROR"}, str(e))
            return {"CANCELLED"}
        self.report({"INFO"}, msg)
        return {"FINISHED"}


class AZM_OT_bolge_goster(bpy.types.Operator):
    bl_idname = "azm.bolge_goster"
    bl_label = "Metin bölgelerini göster/gizle"

    def execute(self, context):
        mat = bpy.data.materials.get("Arazi")
        node = mat and mat.node_tree.nodes.get("Bölge görünürlüğü")
        if node:
            v = node.outputs[0]
            v.default_value = 0.0 if v.default_value > 0.5 else 1.0
        return {"FINISHED"}


class AZM_PT_harita(bpy.types.Panel):
    bl_label = "AzerothMetin Harita"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "AzerothMetin"

    def draw(self, context):
        col = self.layout.column(align=True)
        col.label(text="Boyama: Arazi > Texture Paint,")
        col.label(text="Texture Slots'tan katman seç.")
        col.label(text="Beyaz = var, siyah = yok.")
        col.separator()
        col.operator("azm.bolge_goster", icon="HIDE_OFF")
        box = self.layout.box()
        box.label(text="Metin bölgesi renkleri:")
        for name, c in zip(ZONE_NAMES, ZONE_COLORS):
            box.label(text=f"{name}  ({c[0]:.2f}, {c[1]:.2f}, {c[2]:.2f})")
        self.layout.separator()
        self.layout.prop(context.scene, "azm_cikti")
        self.layout.operator("azm.harita_disa_aktar", icon="EXPORT")


CLASSES = (AZM_OT_disa_aktar, AZM_OT_bolge_goster, AZM_PT_harita)


def register():
    bpy.types.Scene.azm_cikti = bpy.props.StringProperty(
        name="Klasör", subtype="DIR_PATH", default="//harita_cikti/",
        description="Dışa aktarma klasörü (oyunda AZM_MAPDIR olarak ver)")
    for c in CLASSES:
        try:
            bpy.utils.unregister_class(c)
        except RuntimeError:
            pass
        bpy.utils.register_class(c)


# --------------------------------------------------------------------------- giriş noktası

def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    if not argv:
        register()  # Blender arayüzünden çalıştırıldı (Run Script / gömülü modül)
        return
    cmd = argv[0]
    if cmd == "ice-aktar" and len(argv) == 3:
        import_map(argv[1], argv[2])
    elif cmd == "disa-aktar" and len(argv) == 2:
        export_map(argv[1])
    else:
        print(__doc__)
        sys.exit(2)


if __name__ == "__main__":
    main()
else:
    register()
