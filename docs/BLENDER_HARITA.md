# Blender ile Harita Düzenleme

Pyungmoo biçimindeki açık arazi haritaları Blender'da şekillendirilip boyanır, tek tuşla oyunun okuduğu
dosyalara aktarılır. Araç: `tools/blender_harita.py` (Blender 4.2+ / 5.x; ek kurulum gerekmez).

## Hızlı başlangıç

1. **Mevcut haritayı Blender'a aktar** (bir kez, PowerShell'de depo kökünden):
   ```powershell
   & "C:\Program Files\Blender Foundation\Blender 5.2\blender.exe" -b --python tools\blender_harita.py -- ice-aktar assets\pyungmoo C:\Haritalar\pyungmoo.blend
   ```
2. **`pyungmoo.blend`'i Blender'da aç.** "Python betiklerini çalıştır" sorusuna izin ver: 3B görünümde **N** tuşuyla
   açılan kenar çubuğunda **AzerothMetin** sekmesi belirir. İzin vermediysen Scripting sekmesinde `blender_harita.py`
   metnini açıp **Run Script**'e bas.
3. **Düzenle** (aşağıdaki tablo), ardından panelde **Dışa aktar**. Varsayılan klasör `.blend`'in yanında `harita_cikti\`.
4. **Oyunda dene (derlemeden):**
   ```powershell
   & .\tools\shot.ps1 -Out C:\Haritalar\deneme.png -At "25" -Env "AZM_MAPDIR=C:\Haritalar\harita_cikti;AZM_MAPSTART=470,700;AZM_DIST=24"
   ```
   ya da oyunu `AZM_MAPDIR` ortam değişkeniyle başlat. `AZM_MAPDIR`, Pyungmoo modunu da açar (`AZM_PYUNGMOO=1` gerekmez).
5. **Kalıcı yap:** `harita_cikti\` içindeki 5 dosyayı `assets\pyungmoo\`'ya kopyala ve `cargo build --release`.
   Oyun bu dosyaları exe'ye gömer.

## Blender sahnesi

Üstten bakış için **Numpad 7** kullan; kuzey yukarıdadır. Blender konumu `(x, -y, yükseklik)`, birim metredir.
Harita 1024 × 1280 m.

| Ne | Blender'da | Nasıl düzenlenir |
|---|---|---|
| Yükseklik | `Arazi` nesnesi (513 × 641 köşe, 2 m aralık) | **Sculpt Mode** (Grab, Draw, Smooth, Flatten). Dyntopo, remesh ya da modifier da olur: dışa aktarma yukarıdan ışınla örnekler. |
| Yol / Kaya / Kum / Dere yatağı | `Yol`, `Kaya`, `Kum`, `Dere` görüntüleri (1024 × 1280, gri) | Arazi seçiliyken **Texture Paint**. Sağdaki **Texture Slots** listesinden katmanı seç; **beyaz = var, siyah = yok**. Boyanmayan yer çimdir. |
| Su | `Su` görüntüsü (1024 × 1280) | Aynı şekilde boya (beyaz = su). Su yüzeyi sabit −0,35 m'de; su alanının arazisini Sculpt ile bunun altına indir. `Su_Yuzeyi` düzlemi yalnızca seviyeyi gösterir. |
| Metin taşı bölgeleri | `MetinBolge` görüntüsü (256 × 320, 4 m/piksel) | Paneldeki renklerle boya: **Üzüntü** mavi, **Dövüş** turuncu, **Savaş** kırmızı, **Hırs** mor, **Siyah** beyaz; siyah = bölge yok. Görmek için panelde **Metin bölgelerini göster/gizle**. |
| Ağaçlar | `Agaclar` > `Agac_Yaprak`, `Agac_Cam`, `Agac_Sonbahar` koleksiyonları | Her nesne bir ağaçtır (yalnız x/y kullanılır). Eklemek için o koleksiyondaki bir ağacı **Alt+D** ile çoğalt (G ile taşı); silmek için **X**. Oyunda Elwynn ağaç modelleri kullanılır. |

Görüntüler `.blend`'in içinde paketlidir. Boyadıktan sonra **Dışa aktar** onları yeniden paketler; yine de
`.blend`'i **Ctrl+S** ile kaydetmeyi unutma.

Önizleme rengi için görünümü **Material Preview** yap (Z menüsü). Boyarken **Solid > Texture** görünümü etkin katmanı gri gösterir.

## Komut satırı

```powershell
# .blend → oyun dosyaları (pencere açmadan)
& "C:\Program Files\Blender Foundation\Blender 5.2\blender.exe" -b C:\Haritalar\pyungmoo.blend --python tools\blender_harita.py -- disa-aktar C:\Haritalar\harita_cikti
```

## Dosya biçimi

| Dosya | İçerik |
|---|---|
| `height.bin` | f32 LE, 641 satır × 513 sütun köşe yüksekliği (satır = güneye doğru), 2 m aralık |
| `splat.png` | RGBA 1024 × 1280: R = yol, G = kaya, B = kum, A = dere yatağı |
| `water.png` | L 1024 × 1280: 255 = su |
| `metinzones.png` | L 256 × 320: 0 = yok, 1..5 = Üzüntü, Dövüş, Savaş, Hırs, Siyah |
| `trees.json` | `{"w": 1024, "h": 1280, "trees": [[x, y, tür], ...]}`, tür 0 = yaprak, 1 = çam, 2 = sonbahar |

Oyun her ağacın model çeşidini, ölçeğini ve dönüşünü listedeki sırasından türetir. Bu yüzden içe aktarılan
ağaçların özgün sırası nesnelerde (`sira` özelliği) saklanır; dokunulmayan ağaçlar oyunda aynı görünür, yeni ağaçlar
listenin sonuna eklenir. Değişiklik yapmadan içe ve dışa aktarma, 5 dosyayı **birebir aynı** üretir (doğrulandı).

`AZM_MAPDIR` klasöründe eksik ya da bozuk (yanlış boyutlu) bir dosya varsa oyun onun yerine gömülü olanı kullanır
ve log'a uyarı yazar (`terrain: ... geçersiz` / `okunamadı`).

## Kısıtlar

- Harita boyutu sabit 1024 × 1280 m (`terrain.rs` `GW`/`GH`).
- Köprüler, köy binaları, garnizon duvarı (`terrain.rs` `BRIDGES`/`BUILDINGS`/`COMPOUND`), NPC'ler (`npc.rs`) ve canavar
  bölgeleri (`mobs.rs` `PY_ZONES`) hâlâ kodda. Bir binanın altını Sculpt ile değiştirirsen binayı da kodda taşıman gerekir.
- Bu araç Pyungmoo biçimindeki özel haritalar içindir. Elwynn ve Westfall gerçek WoW verisinden (MPQ'daki ADT/WMO)
  gelir ve bu araçla düzenlenmez.
