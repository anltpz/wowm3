# wowm3 — AzerothMetin

**Metin2 uyarlaması bir WoW oyunu.** Oynanış, harita ve sistemler Metin2'den (Jinno — Mavi Bayrak, Map1
**Pyungmoo**; metin taşları, Metin2 envanteri, +0…+9 artı basma, NPC'ler, görevler, düşürmeler); görsel dil
World of Warcraft 1.12 (WoW modelleri, dokuları, UI çerçeveleri, ikonları).

- **Motor:** Rust + Bevy 0.18.1, dünya/ışık/gökyüzü için [benilla](https://github.com/samwhosung/benilla) crate'leri.
- **Assetler:** depoda WoW ya da Metin2 dosyası **yok**. Oyun, yerel bir WoW 1.12 veri klasöründen (`patch.MPQ`)
  çalışma anında okur: `WOW_DATA=<Data klasörü>`.
- **Harita:** Pyungmoo, Metin2 wiki'sindeki kuşbakışı haritadan yeniden üretildi (`tools/build_pyungmoo.py`,
  `tools/build_zones.py`); istemci dosyası kullanılmadı.

## Çalıştırma

```powershell
$env:WOW_DATA="C:\yol\Data"; cargo run --release
```

Kontroller: WASD / sol tık yürü-saldır, Space saldırı, 1-4 beceriler, F1-F4 iksirler, I envanter, Z topla,
H yardım, F5 kaydet.

## Geliştirme

Proje bir ajan takımıyla sprint sprint ilerler (bkz. `docs/TAKIM.md`, `docs/BACKLOG.md`): Ürün Yöneticisi,
Tasarımcı, Oyun Geliştirici, QA ve Sürüm Yöneticisi. Her sprint `sprint-<n>` dalında toplanır ve `main`'e
PR olarak açılır. Güncel durum: `PROJE_DURUMU.md`.
