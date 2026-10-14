# Sprint 1 — "WoW görünümlü pencereler, yaşayan Pyungmoo köyü, kalıcı ilerleme"

Hazırlayan: Ürün Yöneticisi · Başlangıç: 2026-10-14 · Durum: **AÇIK**

**Sprint hedefi:** Oyuncu köye girdiğinde Metin2'deki Pyungmoo köyünü tanısın: NPC'ler wiki'deki yerlerinde
duruyor. Açtığı her pencere gerçek WoW 1.12 arayüzü gibi görünsün. Oyunu kapatıp açınca ilerlemesi yerinde olsun.

| Kimlik | Sahip | Öncelik | Amaç (tek cümle) |
|---|---|---|---|
| D-01 | designer | P0 | Envanter, mağaza, demirci ve tooltip pencerelerini gerçek WoW `Interface\` dokularıyla yeniden giydirmek |
| G-01 | gameplay-dev | P0 | Pyungmoo NPC'lerini wiki koordinatlarında veri tabanlı olarak doğurmak ve tıklanınca konuşma olayı üretmek |
| G-02 | gameplay-dev | P0 | Oyuncu ilerlemesini (seviye, EXP, HP/SP, envanter, ekipman, Yang, görev, konum) `save.ron` ile kalıcı yapmak |
| Q-01 | qa-engineer | P0 | Birleştirilmiş kodu derlemek, test etmek, ekran görüntüsüyle doğrulamak, `docs/QA_RAPORU.md` yazmak |

Sıra: D-01 ile G-01 → G-02 **paralel** çalışır (ayrı git worktree'leri). Orkestratör ikisini birleştirir, sonra Q-01 başlar.
Ekran görüntüleri `docs\qa\s1\` klasörüne kaydedilir; klasörü önce oluştur (`New-Item -ItemType Directory -Force docs\qa\s1`).

---

## Dosya sahipliği (paralel çalışma için kesin kurallar)

| Dosya | Sahip | Diğerleri |
|---|---|---|
| `src/game/ui/mod.rs` | **designer** | gameplay-dev dokunmaz |
| `src/game/ui/skin.rs` (yeni) | **designer** | — |
| `src/game/ui/inventory_ui.rs` | **designer** | gameplay-dev dokunmaz (`starter_kit` burada; G-02 bu dosyayı değiştirmeden çözer, aşağıya bakın) |
| `src/game/ui/shop_ui.rs` | **designer** | gameplay-dev dokunmaz (`GOODS` stoğu bu sprintte yerinde kalır) |
| `src/game/ui/upgrade_ui.rs` | **designer** | gameplay-dev dokunmaz |
| `src/game/ui/tooltip.rs` | **designer** | gameplay-dev dokunmaz |
| `src/game/ui/hud.rs`, `src/game/ui/world_ui.rs` | **kimse** (bu sprintte değişmez) | İkisi de `Npc { name, title }` ve `Windows` okur; bu API'ler korunmalı |
| `src/game/npc.rs` | **gameplay-dev** | designer dokunmaz |
| `src/game/save.rs` (yeni) | **gameplay-dev** | — |
| `src/game/mod.rs` | **gameplay-dev** | designer dokunmaz |
| `src/game/stats.rs`, `quest.rs`, `inventory.rs` | **gameplay-dev** (yalnızca serde öznitelikleri / yardımcı fonksiyonlar) | designer dokunmaz |
| `src/game/character.rs` | **gameplay-dev** (G-01'in isteğe bağlı kadın modeli için küçük bir parametre) | designer bu sprintte dokunmaz |
| `Cargo.toml`, `tools/**`, `assets/**` | **kimse** | Gerekirse orkestratöre sorulur |
| `docs/QA_RAPORU.md` | **qa-engineer** | — |

**Korunacak ortak API'ler** (iki taraf da imzalarını değiştirmez):
- `ui::Windows { inventory, upgrade, shop, help }`, `ui::BlocksWorld`, `ui::panel()`, `ui::text()`, `ui::icon()`, `ui::UiAssets`.
  Designer bunlara yeni alan ya da fonksiyon ekleyebilir; var olanları silmez ve yeniden adlandırmaz.
- `npc::Npc { name: &'static str, title: &'static str, kind: NpcKind }`. gameplay-dev yeni alan ve yeni `NpcKind`
  varyantları ekleyebilir. `name` ve `title` alanları aynı kalır.
- `inventory::Inventory`, `stats::PlayerStats`, `quest::Quest`: alan eklenebilir, var olan alan silinmez.

---

## D-01 · WoW pencere derisi — designer

**Kapsam:** `ui/skin.rs` adında ortak bir deri kiti yazılır. Envanter, Tüccar, Demirci (artı basma) ve tooltip
pencereleri bu kitle yeniden giydirilir. HUD (`hud.rs`) ve isim plakaları (`world_ui.rs`) **kapsam dışı** (D-02).

**Teknik notlar:**
- WoW backdrop kenar dokuları ("edgeFile") tek satırda 8 parçalı şeritlerdir: sol, sağ, üst, alt kenar ve dört
  köşe. `UI-DialogBox-Border.blp` ve `UI-Tooltip-Border.blp` yüklenince 8 alt görüntüye bölünmeli ya da UV ile
  kırpılmalı. Arka plan dokusu (`*-Background.blp`) döşenerek çizilir. İlk iş: dokuyu yükleyip ekran görüntüsüyle
  parça sırasını doğrula.
- Bevy 0.18: `ImageNode` + `NodeImageMode::Tiled` / `Sliced(TextureSlicer)` kullanılabilir.
- `UI-Panel-CloseButton` MPQ'da **YOK**. Kapat düğmesi için `Interface\Buttons\CancelButton-Up/Down/Highlight.blp` kullan.
- Türkçe glif sorunu yüzünden UI fontu (Palatino) değişmez.

**Kullanılacak WoW assetleri** (hepsi `tools/mpq_present.txt` içinde ✔):
- Çerçeve: `Interface\DialogFrame\UI-DialogBox-Background.blp`, `UI-DialogBox-Border.blp`, `UI-DialogBox-Header.blp`
- Demirci (altın): `Interface\DialogFrame\UI-DialogBox-Gold-Background.blp`, `UI-DialogBox-Gold-Border.blp`, `UI-DialogBox-Gold-Header.blp`, `UI-DialogBox-Gold-Dragon.blp` (süs)
- Tooltip: `Interface\Tooltips\UI-Tooltip-Background.blp`, `UI-Tooltip-Border.blp`
- Slot: `Interface\Buttons\UI-Slot-Background.blp`, `UI-EmptySlot.blp`, `UI-Quickslot2.blp`, `ButtonHilight-Square.blp`
- Boş ekipman yuvası silüetleri: `Interface\PaperDoll\UI-PaperDoll-Slot-Head.blp`, `-MainHand`, `-Chest`, `-SecondaryHand`, `-Wrists`, `-Feet`, `-Neck`, `-REar`
- Düğme: `Interface\Buttons\UI-Panel-Button-Up.blp`, `-Down`, `-Highlight`, `-Disabled`
- Kapat: `Interface\Buttons\CancelButton-Up.blp`, `-Down`, `-Highlight`
- Sayfa sekmeleri (envanter I/II): `Interface\PaperDollInfoFrame\UI-Character-ActiveTab.blp`, `UI-Character-InactiveTab.blp`
- Yang simgesi: `Interface\MoneyFrame\UI-MoneyIcons.blp` (altın sikke bölgesi)

**Kabul kriterleri:**
1. [ ] Envanter, Tüccar, Demirci ve tooltip pencerelerinin kenarları ve arka planları WoW dokusu. Ekran görüntüsünde düz renk `PANEL_BG` paneli kalmamış.
2. [ ] Demirci penceresinde altın çerçeve (`UI-DialogBox-Gold-*`) var, diğer pencerelerde normal çerçeve.
3. [ ] Envanter slotları WoW slot dokusunda. Üstüne gelince `ButtonHilight-Square` parlıyor. Boş ekipman yuvalarında PaperDoll silüetleri görünüyor.
4. [ ] "Satın Al", "Yükselt" ve kapat düğmeleri WoW dokulu; üstüne gelme ve basılma durumları görünüyor.
5. [ ] Yang miktarının yanında WoW altın sikke simgesi var.
6. [ ] İşlevlerde gerileme yok: sürükle-bırak, Shift ile bölme, sağ tık kuşan/kullan/sat, artı basma, sayfa değiştirme, tooltip.
7. [ ] `AZM_OPEN` ortam değişkeni (`inv`, `shop`, `upgrade` virgülle) pencereleri açılışta açıyor. `ui/mod.rs` içine eklenir.
8. [ ] Önce ve sonra ekran görüntüleri:
   `& .\tools\shot.ps1 -Out docs\qa\s1\d01_inv.png -At "25" -Env "AZM_MAPSTART=394,690;AZM_OPEN=inv;AZM_DIST=8"`
   `... d01_upgrade.png -Env "AZM_MAPSTART=394,690;AZM_OPEN=inv,upgrade;AZM_DIST=8"` (Demirci'nin yanı)
   `... d01_shop.png -Env "AZM_MAPSTART=378,692;AZM_OPEN=inv,shop;AZM_DIST=8"` (Satıcı'nın yanı)
   (Bu noktalar NPC'lerin hem eski konumlarına hem G-01'deki yeni konumlarına 6 m'den yakın; `npc::auto_close` pencereyi kapatmaz.)
9. [ ] `run_shot.log` içinde `Interface\` için `Path not found` yok. `cargo build --release` uyarısız ya da yeni uyarı yok.

**Dokunulmayacak:** `npc.rs`, `game/mod.rs`, `save.rs`, `hud.rs`, `world_ui.rs`, `inventory.rs` (mantık). `GOODS`
dizisi `shop_ui.rs` içinde aynı kalır (G-03'te taşınacak).

---

## G-01 · Pyungmoo NPC'leri — gameplay-dev

**Kapsam:** `npc.rs` veri tabanlı hale gelir. NPC tablosundaki her satırda şunlar bulunur: ad, unvan, `NpcKind`,
wiki harita koordinatı, bakış yönü, zırh seti, elindeki alet ve karşılama satırı. NPC'ler `terrain::data().to_world(x, y)` ile yerleştirilir.

**Doğacak NPC'ler** (wiki koordinatları, harita metresi):

| NPC | x, y | Unvan (örnek) | Tıklanınca |
|---|---|---|---|
| Demirci | 393, 692 | Eşya Yükseltme | Yükseltme penceresi (var) |
| Satıcı | 383, 693 | Genel Mağaza | Mağaza penceresi (var) |
| Silah Satıcısı | 430, 607 | Silahlar | Mağaza penceresi (aynı stok; ayrım G-03'te) |
| Zırh Satıcısı | 403, 586 | Zırhlar | Mağaza penceresi (aynı stok) |
| Depocu | 315, 560 | Depo | Karşılama satırı + `NpcTalked` |
| Seyis | 396, 735 | Ahır | Karşılama + `NpcTalked` |
| Yaşlı Kadın | 417, 671 | Falcı | Karşılama + `NpcTalked` |
| Köy Gardiyanı | 383, 640 | Köy Meydanı | Karşılama + `NpcTalked` |
| Şehir Bekçisi | 468, 714 | Bekçi | Karşılama + `NpcTalked` |
| Işınlayıcı | 432, 685 | Işınlanma | Karşılama + `NpcTalked` |
| Yüzbaşı | 286, 639 | Garnizon | Karşılama + `NpcTalked` |
| Bedensel Savaş Öğretmeni | 444, 623 | Savaşçı Öğretmeni | Karşılama + `NpcTalked` |
| Uriel | 425, 716 | — | Karşılama + `NpcTalked` |
| Octavio | 340, 747 | — | Karşılama + `NpcTalked` |
| Baek-Go | 465, 612 | — | Karşılama + `NpcTalked` |
| Soon | 454, 530 | — | Karşılama + `NpcTalked` |
| Balıkçı | 428, 480 | Balıkçı | Karşılama + `NpcTalked` |
| Balıkçı | 386, 849 | Balıkçı | Karşılama + `NpcTalked` |
| Yaşlı Adam | 125, 1118 | Işınlayıcı | Karşılama + `NpcTalked` |

Kaynak: de-wiki.metin2.gameforge.com/index.php/Pyungmoo (koordinatlar) ve tr-wiki Pyungmoo / NPC sayfaları
(Türkçe adlar ve işlevler). Not: mevcut kodda Demirci 396,688, Tüccar 374,690 noktasında. Tüccar'ın adı **Satıcı** olacak.

**Teknik notlar:**
- Wiki koordinatı bir binanın ya da suyun içine düşerse (`TerrainData::blocked`), en fazla 6 m içinde en yakın açık
  noktaya kaydır ve `info!` ile yaz. Köydeki WMO'lar: Demirci binası 386,706, ahır 428,680, han 406,573 (`terrain::BUILDINGS`).
- `NpcTalked { entity: Entity, kind: NpcKind }` adlı bir Bevy `Message` eklenir (`app.add_message`). Sprint 3'teki diyalog sistemi (G-05) bunu kullanacak.
- Karşılama satırı `GameLog::push` ile sarı renkte yazılır, biçimi `"<Ad>: <metin>"`. Metinler Metin2 havasında ve Türkçe.
- `click_npc`, `arrive` ve `auto_close` mantığı korunur. `auto_close` her mağaza türü NPC'si için çalışmalı (Satıcı, Silah ve Zırh Satıcısı).
- Modeller: varsayılan `HumanMale` + rolüne uygun zırh seti. Satıcı ve Yaşlı Kadın için `HumanFemale` isteğe
  bağlıdır. Bunun için `character::build_body`'ye küçük bir parametre gerekir; yapılamazsa raporda belirt (D-10'a kalır).
- Bevy B0001: NPC sorguları oyuncu ve mob sorgularıyla `&mut Transform` paylaşıyorsa `Without<...>` ile ayır.

**Kabul kriterleri:**
1. [ ] Log'da `npc: 19 NPC doğdu` (ya da tablodaki gerçek sayı, en az 16).
2. [ ] Birim testleri: NPC tablosunda yinelenen (ad, x, y) yok; koordinatlar 0..1024 × 0..1280 içinde; her `NpcKind` için karşılama metni var.
3. [ ] `& .\tools\shot.ps1 -Out docs\qa\s1\g01_meydan.png -At "25" -Env "AZM_MAPSTART=400,660;AZM_DIST=14"` görüntüsünde en az 4 NPC isim plakası (`Ad` + `<Unvan>`) okunuyor.
4. [ ] Demirci yükseltme, Satıcı / Silah Satıcısı / Zırh Satıcısı mağaza penceresi açıyor. Uzaklaşınca (8 m+) pencere kapanıyor.
5. [ ] Diğer NPC'lere tıklanınca sohbet kutusunda karşılama satırı çıkıyor ve `NpcTalked` yayınlanıyor (birim testi ya da log satırı).
6. [ ] `run_shot.log` içinde `panic` ve `B0001` yok.
7. [ ] `ui/**` dosyalarına dokunulmadı (`git diff --stat` raporda).

---

## G-02 · Kayıt / yükleme — gameplay-dev

**Kapsam:** `src/game/save.rs` (yeni modül, `game/mod.rs` içinde kaydedilir).

**Tasarım:**
- `SaveData { version: u32, stats: PlayerStats, inventory: Inventory, quest: Quest, pos_map: Option<[f32; 2]> }`.
  Gelecekteki her alan `#[serde(default)]` ile eklenecek.
- Konum: `%APPDATA%\AzerothMetin\save.ron`. `APPDATA` yoksa exe'nin yanında `save\save.ron`. `AZM_SAVE=<yol>` yolu değiştirir.
- **Kayıt kapatma:** `AZM_NOSAVE=1` **ya da** `AZM_SHOT` tanımlıysa (ekran görüntüsü / test çalıştırmaları) kayıt
  ne okunur ne yazılır. Böylece QA ve tasarımcı çalıştırmaları kullanıcının kaydını bozmaz. İstisna: `AZM_SAVE` açıkça verilmişse kayıt çalışır.
- **Yükleme:** `PreStartup` aşamasında `Inventory`, `PlayerStats` ve `Quest` kaynakları `insert_resource` ile
  yerleştirilir. `inventory_ui::starter_kit` (Startup) dolu ekipmanı görüp yeni eşya vermez. Bu dosyaya dokunulmaz.
- **Konum:** `game::start_wow()` öncelik sırası: `AZM_FOCUS` > `AZM_MAPSTART` > **kayıttaki `pos_map`** > köy meydanı. Kaydedilen konum suda ya da engelliyse köy meydanı kullanılır.
- **Ne zaman yazılır:** 30 sn'de bir otomatik, seviye atlayınca, F5'e basınca (log: "Oyun kaydedildi.") ve
  pencere kapanırken (`WindowCloseRequested` / `AppExit`). Yazma atomik: `save.ron.tmp` + `rename`.
- **Bozuk dosya:** `warn!` ile yaz, dosyayı `save.ron.bak` olarak sakla, yeni oyunla devam et.
- Ölüyken kayıt yapılırsa HP en az 1 olarak yazılır (`dead` zaten `serde(skip)`).
- Log satırları: `save: yüklendi Sv{n} {yang} Yang ({yol})`, `save: yeni oyun`, `save: yazıldı`.

**Kabul kriterleri:**
1. [ ] Birim testleri: (a) `SaveData` RON gidiş-dönüşü (artılı eşya, yığın, ekipman, Yang, görev aşaması aynı kalıyor);
   (b) eksik alanlı eski RON metni `serde(default)` ile yükleniyor; (c) bozuk metin hata veriyor ama panik yok.
2. [ ] Elle test (`AZM_SAVE=%TEMP%\azm_test.ron`): 1. çalıştırma `AZM_AUTO=1` ile 60 sn; seviye, EXP ve Yang artıyor,
   F5 / kapanış ile kayıt yazılıyor. 2. çalıştırmada log'da `save: yüklendi Sv{n}`; seviye, envanter, ekipman (+artı),
   Yang ve görev aşaması aynı; başlangıç eşyaları tekrar verilmemiş; oyuncu kayıttaki konumda doğuyor.
3. [ ] `AZM_SHOT` ile yapılan çalıştırmalar kullanıcı kaydını değiştirmiyor (dosyanın değiştirilme zamanı aynı).
4. [ ] F5 ile sohbet kutusunda "Oyun kaydedildi." yazıyor.
5. [ ] `cargo build --release` ve `cargo test --release` temiz. `ui/**` dosyalarına dokunulmadı.

---

## Q-01 · Sprint 1 doğrulaması — qa-engineer (birleştirmeden sonra)

1. [ ] `cargo build --release` ve `cargo test --release`: uyarı ve hata özeti; yeni testlerin (G-01, G-02) sayısı.
2. [ ] Ekran görüntüleri (`docs\qa\s1\` klasörüne), her biri Read ile incelenir:
   - Köy meydanı NPC'leri: `-Env "AZM_MAPSTART=400,660;AZM_DIST=14"`
   - Envanter / mağaza / demirci (D-01): `-Env "AZM_MAPSTART=378,692;AZM_OPEN=inv,shop;AZM_DIST=8"` ve `-Env "AZM_MAPSTART=394,690;AZM_OPEN=inv,upgrade;AZM_DIST=8"`
   - Metin + mob regresyonu: `-Env "AZM_MAPSTART=725,660;AZM_AUTO=1"` `-At "40"`
   - Uzak NPC: Yaşlı Adam `-Env "AZM_MAPSTART=128,1112;AZM_DIST=10"`
3. [ ] Kayıt senaryosu: G-02'deki 2. kriter `AZM_SAVE=%TEMP%\azm_qa.ron` ile iki çalıştırmada tekrarlanır; log satırları rapora eklenir.
   `AZM_SHOT` ile kayıt dosyasının değişmediği doğrulanır.
4. [ ] `run_shot.log` içinde `panic`, `B0001`, `Path not found` (köy dışı WMO hariç) ve shader hataları aranır.
5. [ ] Yukarıdaki D-01, G-01 ve G-02 kabul kriterleri tek tek işaretlenir.
6. [ ] `docs/QA_RAPORU.md` yazılır: tarih, sprint, geçen/kalan kriterler, ekran görüntüsü yolları, bulunan hatalar ve önerilen sahip.
   Küçük ve açık hatalar düzeltilip ayrı commit'te belirtilebilir.

---

## Riskler
- **Kenar dokusu biçimi** (8 parçalı şerit) yanlış yorumlanabilir. Designer ilk iş olarak bir test ekran görüntüsüyle doğrulamalı.
- **NPC koordinatları** wiki'den geliyor. Bizim harita görselden üretildiği için birkaçı bina ya da su içine düşebilir (kaydırma kuralı var).
- **Kayıt yolu:** `starter_kit` yalnızca ekipmana ve Kırmızı İksir (K)'ya bakıyor. Hiç ekipmanı ve iksiri olmayan
  bir kayıtta başlangıç eşyası tekrar verilebilir. Kabul edilebilir bir risk; Sprint 2'de `save` içinde "yeni oyun" bayrağıyla kapatılacak.
- **Birleştirme:** iki tarafın ortak dokunduğu dosya yok. Tek risk, designer'ın `Windows` yapısına alan eklemesi ile
  G-01'in `Windows`'u kullanması. Alan eklemek uyumlu, silmek yasak.

## Sprint kapanışı (PM dolduracak)
- [ ] D-01 · [ ] G-01 · [ ] G-02 · [ ] Q-01
- `PROJE_DURUMU.md` güncellemesi: NPC listesi, kayıt sistemi, UI derisi, metin taşı adlarının düzeltilmesi.
