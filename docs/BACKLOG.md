# AzerothMetin — Ürün Backlog'u

Hazırlayan: Ürün Yöneticisi (product-manager) · Tarih: 2026-10-14 · Sürüm: 1

Vizyon: **Metin2 uyarlaması bir WoW oyunu.** Oynanış, harita ve sistemler Metin2'den gelir (Pyungmoo / Mavi Bayrak
köyü, metin taşları, 5×9 envanter, artı basma, NPC'ler, görevler, düşürmeler). Görünüm ise WoW 1.12'den gelir
(WoW modelleri, dokuları, `Interface\` UI dokuları, ikonlar, büyü efektleri, sesler). Hepsi yerel MPQ'dan okunur.

Kaynaklar: `PROJE_DURUMU.md`, `docs/TAKIM.md`, `src/game/**` kod taraması, `tools/mpq_present.txt` (grep ile
doğrulandı), tr-wiki.metin2.gameforge.com ve de-wiki.metin2.gameforge.com (NPC koordinatları de-wiki
Pyungmoo sayfasında listeleniyor; tr-wiki'de yalnızca harita görselinde var).

---

## 0. Kodda doğrulanan mevcut durum (2026-10-14)

| Alan | Durum | Not |
|---|---|---|
| Harita | Pyungmoo 1024×1280 m, 12 WMO bina, köprüler, su (`terrain.rs`) | `TerrainData::open/blocked` var |
| Moblar | 37 tür + 8 mini patron, wiki adları/seviyeleri (`mobs.rs`) | Wiki'deki adlı "Sn.4" moblar (Cung-Mok, Jug-Hyul, Mu-Rang, Young-Ji, Mi-Jung, Li-An, Eun-Jung, Se-Rang, Jin-Hee) ve "Moralsiz" türler **yok** |
| Metin taşları | Üzüntü / Dövüş / Savaş / Hırs Metini + Siyah Metin (`metin.rs`) | Adlar wiki ile aynı. Can değerleri oyun dengesi için küçültülmüş (Üzüntü: wiki 14.550, kod 1.800) |
| Düşürmeler | `metin.rs::mob_drops` içinde **kod olarak** yazılı: tür malzemesi, iksir, Ruh Taşı, takı havuzu, `gear_for_level` | Mob başına veri tablosu yok. Wiki'deki Dönüşüm Küresi, Beceri Kitabı, Zihin Taşı gibi eşyalar yok |
| Eşyalar | 10 kılıç, 4 zırh, 6 kask/kalkan/ayakkabı, 13 takı, 4 iksir, 10 malzeme, 3 kağıt (`items.rs`) | Bütün ikonlar MPQ'da var (grep ile doğrulandı) |
| NPC | Sadece **Demirci** ve **Tüccar** (`npc.rs`), ikisi de HumanMale modeli | `character::build_body` HumanMale'e sabitlenmiş |
| Görev | Tek bir döngü görevi: "Metin Taşlarını Yok Et (n/hedef)" (`quest.rs`) | Görev motoru, NPC görevi ve diyalog yok |
| Beceriler | 4 savaşçı becerisi, SP + bekleme süresi (`skills.rs`) | Beceri seviyesi, beceri puanı, öğretmen ve kitap yok. Efektler prosedürel halkalar, `Spells\*.m2` kullanılmıyor |
| İstatistik | Seviye/EXP/HP/SP, en yüksek seviye 40 (`stats.rs`) | Statü puanları (GÜÇ/CAN/ZEK/ÇEV) yok |
| Kayıt | **Yok.** `PlayerStats`, `Inventory`, `Quest`, `Item` zaten `Serialize`/`Deserialize`. `ron` bağımlılığı var | `inventory_ui::starter_kit` "kayıt yoksa" kontrolü yapıyor |
| UI | Prosedürel kahverengi/altın paneller (`ui/mod.rs::panel`). HUD bazı WoW dokularını kullanıyor (EndCap, portre, ikonlar) | Pencerelerde WoW `Interface\` çerçeve dokusu yok. Karakter penceresi (C), harita (M) ve beceri penceresi yok |
| Radar | Renkli noktalar (`hud.rs::radar`) | Gerçek minimap dokusu yok |
| +artı parıltısı | `items::plus_glow` tanımlı ama **hiçbir yerde kullanılmıyor**; `player::Look.weapon_plus` izleniyor | Görsel yok |
| Ses | **Yok.** `Cargo.toml` Bevy özelliklerinde `bevy_audio` / `wav` / `mp3` yok | MPQ'da 817 `Sound\` dosyası var |
| Binek | Yok | MPQ'da **at modeli yok** (aşağıdaki karara bakın) |
| Tuşlar | I/Tab, H, Esc, Z/`, R, Space, 1-4, F1-F4, WASD/oklar | C, M, K, L, F5, Ctrl+G boş |

### Kod ile Metin2 arasındaki çelişkiler (karar gerekir)
1. **"Ruh Taşı" adı:** Kodda artı basma malzemesi olarak kullanılıyor (`V_SPIRIT_STONE`). Metin2'de Ruh Taşı, becerileri
   Büyük Usta'ya yükseltmek için okunur. Öneri: malzemenin adı değişsin (G-13), "Ruh Taşı" adı beceri sisteminde
   kullanılsın (G-19).
2. **At modeli:** MPQ'da `Creature\Horse\*.m2` yok, yalnızca at ayak sesleri var. Var olan binekler:
   `Creature\MountedDeathKnight\RidingUndeadWarHorse.m2` (ölümsüz savaş atı) ve `Creature\DireWolf\RidingDireWolf.m2`
   (8 deri). Öneri: Metin2'deki "At" yerine `RidingDireWolf` kullanılsın (WoW hissine de uyar), ölümsüz at ise
   ileri seviye binek olsun. Bu kararı kullanıcı onaylamalı (bkz. G-22 / D-18).
3. **`PROJE_DURUMU.md` metin taşı adları** eski ("Hüzün/Açgözlülük/Kara"). Kodda adlar doğru. Sprint kapanışında düzeltilecek.

---

## 1. Kimlik, sahip ve öncelik kuralları
- **D-xx** tasarımcı (`designer`), **G-xx** oyun geliştirici (`gameplay-dev`), **Q-xx** QA (`qa-engineer`).
- **P0**: Metin2 hissini ya da görselliği en çok etkileyen iş, sıradaki 1-2 sprintte yapılır. **P1**: önemli. **P2**: cila / sonra.
- Her görev tek bir ajan oturumunda bitecek büyüklükte olmalı. Daha büyük işler parçalara bölünmüştür.
- Kabul kriterlerinin hepsi gözlemlenebilir olmalı: ekran görüntüsü, log satırı ya da `cargo test`.
- WoW asset yollarının hepsi `tools/mpq_present.txt` içinde grep ile kontrol edildi. ✔ = doğrulandı, ✘ = MPQ'da YOK.
- Telifli Metin2 istemci dosyaları (pack, model, doku, ses) kullanılmaz. Wiki'den yalnızca bilgi (ad, seviye, koordinat, kural) alınır.

## 2. Epikler

| Epik | Ad | Görevler |
|---|---|---|
| E1 | WoW arayüz dili (pencere / HUD derisi) | D-01, D-02, D-16, D-19, D-20 |
| E2 | Karakter ve beceri pencereleri | D-03, D-04, G-20 |
| E3 | Harita ve minimap | D-05, D-06, G-23 |
| E4 | Pyungmoo NPC'leri ve hizmetleri | G-01, G-03, G-04, G-05, G-21b, D-08, D-09, D-10 |
| E5 | Eşyalar ve Metin2 düşürme tabloları | G-10, G-11, G-12, G-13, G-14, D-14 |
| E6 | Görevler (Metin2 tarzı başlangıç zinciri) | G-06, G-07, G-08, G-09, D-07 |
| E7 | Beceri seviyeleri ve beceri kitapları | G-18, G-19, D-04, D-12 |
| E8 | Kayıt / yükleme | G-02 |
| E9 | Görsel cila: parıltı, efektler, köy dekoru | D-11, D-12, D-13, D-15 |
| E10 | Ses | G-21, D-17 |
| E11 | Binek (at) | G-22, D-18 |
| E12 | Kalite ve araçlar | Q-01 … Q-05 |

---

## E1 — WoW arayüz dili

### D-01 · WoW pencere derisi: envanter, mağaza, demirci, tooltip — designer · P0 · **Sprint 1**
- **Açıklama:** Prosedürel kahverengi panellerin yerine gerçek WoW 1.12 `Interface\` dokularını koy. Bunun için
  ortak bir "deri kiti" yazılacak (`ui/skin.rs`): backdrop (döşenen arka plan), 8 parçalı kenar ("edgeFile"),
  başlık bandı, WoW düğmesi, slot, kapat düğmesi, Yang simgesi. Yerleşim Metin2'deki gibi kalır: 5×9 ızgara,
  2 sayfa, ekipman yuvaları.
- **Kabul kriterleri:**
  - Envanter, Tüccar, Demirci ve tooltip pencerelerinin kenarı ve arka planı WoW dokusu. Ekran görüntüsünde düz renk panel kalmamış.
  - Demirci (artı basma) penceresinde altın çerçeve var (`UI-DialogBox-Gold-*`).
  - Slotlarda WoW slot dokusu, üstüne gelince `ButtonHilight-Square` parlaması. Boş ekipman yuvasında PaperDoll yuva silüeti.
  - Düğmeler (Satın Al, Yükselt, Kapat) `UI-Panel-Button-*`. Basılı ve üstüne gelinmiş durumları görünüyor.
  - Bütün işlevler aynen çalışıyor: sürükle-bırak, Shift ile bölme, sağ tık, satış, yükseltme.
  - `AZM_OPEN=inv,shop,upgrade` ortam değişkeni pencereleri açılışta açıyor (QA ekran görüntüsü için).
  - `run_shot.log` içinde `Interface\` için "Path not found" yok.
- **Dosyalar:** `src/game/ui/mod.rs`, `src/game/ui/skin.rs` (yeni), `ui/inventory_ui.rs`, `ui/shop_ui.rs`, `ui/upgrade_ui.rs`, `ui/tooltip.rs`
- **Metin2 referansı:** Envanter ve demirci penceresi düzeni (tr-wiki "Envanter", "Demirci")
- **WoW assetleri:** ✔ `Interface\DialogFrame\UI-DialogBox-Background.blp`, ✔ `UI-DialogBox-Border.blp`,
  ✔ `UI-DialogBox-Gold-Border.blp`, ✔ `UI-DialogBox-Gold-Background.blp`, ✔ `UI-DialogBox-Header.blp`,
  ✔ `UI-DialogBox-Gold-Header.blp`, ✔ `UI-DialogBox-Gold-Dragon.blp`, ✔ `Interface\Tooltips\UI-Tooltip-Background.blp`,
  ✔ `UI-Tooltip-Border.blp`, ✔ `Interface\Buttons\UI-Slot-Background.blp`, ✔ `UI-EmptySlot.blp`, ✔ `UI-Quickslot2.blp`,
  ✔ `ButtonHilight-Square.blp`, ✔ `UI-Panel-Button-Up/Down/Highlight/Disabled.blp`, ✔ `CancelButton-Up/Down/Highlight.blp`,
  ✔ `Interface\MoneyFrame\UI-MoneyIcons.blp`, ✔ `Interface\PaperDoll\UI-PaperDoll-Slot-{Head,MainHand,Chest,SecondaryHand,Wrists,Feet,Neck,REar}.blp`,
  ✔ `Interface\PaperDollInfoFrame\UI-Character-ActiveTab.blp` / `InactiveTab.blp` (sayfa sekmeleri).
  ✘ `UI-Panel-CloseButton` MPQ'da yok, yerine `CancelButton-*` kullanılacak.

### D-02 · HUD'u WoW dokularıyla yeniden giydirme — designer · P0
- **Açıklama:** Alt çubuk, beceri/iksir slotları, oyuncu portresi ve hedef çerçevesi WoW dokularıyla çizilsin.
  Metin2 düzeni korunur: HP/SP küreleri, 1-4 beceri ve F1-F4 iksir slotları. Metin taşı ve mini patron hedef
  alındığında "Elite/Rare" ejderha çerçevesi görünsün.
- **Kabul kriterleri:** Beceri ve iksir slotlarında `UI-Quickslot2` + `UI-ActionButton-Border`. Bekleme süresinde
  `Interface\Cooldown\cooldown.blp` karartması. Hedef çerçevesi `UI-TargetingFrame` dokusu. Patron ve metin taşında
  `UI-TargetingFrame-Elite` görünüyor. EXP çubuğunda `UI-ExhaustionTickNormal`. Ekran görüntüsü öncesi ve sonrası karşılaştırılır.
- **Dosyalar:** `ui/hud.rs`, `ui/world_ui.rs`
- **WoW assetleri:** ✔ `Interface\MainMenuBar\UI-MainMenuBar-Human.blp`, ✔ `UI-MainMenuBar-EndCap-Human.blp`,
  ✔ `Interface\TargetingFrame\UI-TargetingFrame.blp`, ✔ `UI-TargetingFrame-Elite.blp`, ✔ `UI-TargetingFrame-Rare.blp`,
  ✔ `UI-TargetingFrame-BarFill.blp`, ✔ `UI-StatusBar.blp`, ✔ `Interface\CharacterFrame\UI-Player-Portrait.blp`,
  ✔ `Interface\Buttons\UI-ActionButton-Border.blp`, ✔ `Interface\Cooldown\cooldown.blp`, ✔ `Interface\Tooltips\Nameplate-Border.blp`

### D-16 · WoW imleçleri — designer · P2
- **Açıklama:** İmleç bağlama göre değişsin: düşman üzerinde saldırı, NPC üzerinde konuşma, satıcıda satın alma, yerdeki ganimette toplama.
- **Kabul kriterleri:** Düşmanın üstünde kılıç, NPC'de konuşma balonu, ganimette el imleci. Ekran görüntüsünde görünüyor.
- **Dosyalar:** `ui/mod.rs` (yeni sistem), `ui/world_ui.rs`
- **WoW assetleri:** ✔ `Interface\Cursor\Attack.blp`, ✔ `Speak.blp`, ✔ `Buy.blp`, ✔ `Pickup.blp`, ✔ `Point.blp`, ✔ `Interact.blp`, ✔ `Repair.blp`

### D-19 · Yükleme ekranı ve açılış — designer · P2
- **Açıklama:** Dünya akışı hazır olana kadar WoW yükleme çubuğu gösterilsin.
- **Kabul kriterleri:** Açılışta yükleme çubuğu dolar, sonra HUD görünür.
- **WoW assetleri:** ✔ `Interface\Glues\LoadingBar\Loading-BarBorder.blp`, ✔ `Loading-BarFill.blp`, ✔ `Loading-BarGlow.blp`

### D-20 · Sistem menüsü (Esc) ve mikro düğmeler — designer · P1
- **Açıklama:** HUD'da Metin2'deki menü düğmeleri olsun: Karakter, Envanter, Beceri, Görev, Harita, Sistem.
  Görünüm WoW mikro düğmeleri. Esc menüsünde "Kaydet ve Çık" (G-02 API'sini kullanır).
- **Kabul kriterleri:** 6 mikro düğme tıklanınca ilgili pencereyi açıyor. Esc menüsü açılıyor ve "Kaydet ve Çık" oyunu kaydedip kapatıyor.
- **WoW assetleri:** ✔ `Interface\Buttons\UI-MicroButtonCharacter-Up.blp`, ✔ `UI-MicroButton-Spellbook-Up.blp`,
  ✔ `UI-MicroButton-Quest-Up.blp`, ✔ `UI-MicroButton-World-Up.blp`, ✔ `UI-MicroButton-MainMenu-Up.blp`, ✔ `Button-Backpack-Up.blp`

---

## E2 — Karakter ve beceri pencereleri

### D-03 · Karakter penceresi (C) — designer · P0
- **Açıklama:** C tuşu WoW PaperDoll düzeninde bir karakter penceresi açsın. Solda ekipman yuvaları, ortada
  karakter adı ve seviyesi, altta Metin2 değerleri: HP, SP, Saldırı min-max, Savunma, Saldırı Hızı, Hareket Hızı.
  G-20 bittikten sonra GÜÇ/CAN/ZEK/ÇEV ve "+" düğmeleri de eklenir.
- **Kabul kriterleri:** C tuşu pencereyi açıp kapatıyor. Değerler `stats::Derived` ile birebir aynı. Ekipman
  değişince değerler güncelleniyor. Ekran görüntüsü alınır.
- **Dosyalar:** `ui/character_ui.rs` (yeni), `ui/mod.rs`
- **Metin2 referansı:** tr-wiki "Karakter Penceresi", "Statü"
- **WoW assetleri:** ✔ `Interface\PaperDollInfoFrame\UI-Character-General-TopLeft/TopRight/BottomLeft/BottomRight.blp`,
  ✔ `UI-Character-StatBackground.blp`, ✔ `UI-Character-CharacterTab-L1.blp`, ✔ `Interface\PaperDoll\UI-PaperDoll-Slot-*.blp`,
  ✔ `Interface\PaperDoll\UI-PaperDollBackground-L1/L2/R1/R2.blp`

### D-04 · Beceri penceresi (K) — designer · P1 (G-18'e bağlı)
- **Açıklama:** WoW Spellbook görünümünde Metin2 beceri penceresi. Her becerinin yanında ikon, ad, seviye
  (1-16 / M1-M10 / G1-G10 / P), "+" düğmesi ve kalan beceri puanı. Beceriler sürüklenerek 1-4 slotlarına konabilir.
- **Kabul kriterleri:** K tuşu pencereyi açıyor. "+" düğmesi G-18 API'sine bağlı. Seviye etiketleri doğru (ör. "M3").
- **WoW assetleri:** ✔ `Interface\Spellbook\UI-SpellbookPanel-TopLeft/TopRight/BotLeft/BotRight.blp`,
  ✔ `UI-Spellbook-SpellBackground.blp`, ✔ `UI-SpellBook-Tab1-Selected.blp`, ✔ `Interface\Buttons\UI-PlusButton-Up.blp`

### G-20 · Statü puanları (GÜÇ / CAN / ZEK / ÇEV) — gameplay-dev · P1
- **Açıklama:** Metin2'deki gibi seviye atlayınca statü puanı kazanılır. GÜÇ saldırıyı, CAN HP ve savunmayı,
  ZEK SP'yi, ÇEV isabet ve kaçınmayı artırır. Savaşçının başlangıç değerleri wiki'den: CAN 4, GÜÇ 6 (ZEK ve ÇEV
  için tr-wiki "Savaşçı" sayfası).
- **Kabul kriterleri:** `PlayerStats` içinde `str/vit/int/dex/points` alanları var, kayıtla uyumlu (`#[serde(default)]`).
  `derive()` bu değerleri kullanıyor. Birim testi: GÜÇ +1 → saldırı artıyor. `spend_point(stat)` API'si D-03 için hazır.
- **Dosyalar:** `stats.rs`, `combat.rs`
- **Metin2 referansı:** tr-wiki "Savaşçı" (başlangıç: CAN 4, HP 760, GÜÇ 6), "Statü"

---

## E3 — Harita ve minimap

### D-05 · Gerçek minimap — designer · P0
- **Açıklama:** Radar noktalarının altında gerçek bir harita dokusu olsun. Doku, oyunun kendi Pyungmoo verisinden
  (`assets/pyungmoo/splat.png` + `water.png` + yükseklik) çalışırken ya da araçla üretilir; wiki görseli doğrudan
  kopyalanmaz. Harita döner ve oyuncunun etrafını gösterir. WoW minimap çerçevesi, oyuncu oku, NPC/metin simgeleri
  ve yakınlaştırma (+/−) düğmeleri olur.
- **Kabul kriterleri:** Ekran görüntüsünde köy yolları, dere ve köprüler minimapte seçiliyor. Ok kamera yönünü
  gösteriyor. Metin taşı ve NPC'ler `ObjectIcons`/`POIIcons` simgeleriyle görünüyor. Yakınlaştırma 3 kademe.
- **Dosyalar:** `ui/hud.rs` (radar), `ui/minimap.rs` (yeni)
- **WoW assetleri:** ✔ `Interface\Minimap\UI-Minimap-Border.blp`, ✔ `MinimapArrow.blp`, ✔ `ROTATING-MINIMAPARROW.blp`,
  ✔ `ObjectIcons.blp`, ✔ `POIIcons.blp`, ✔ `UI-Minimap-ZoomInButton-Up.blp`, ✔ `UI-Minimap-ZoomOutButton-Up.blp`, ✔ `MiniMap-TrackingBorder.blp`

### D-06 · Büyük harita (M) — designer · P1
- **Açıklama:** M tuşu tam ekran Pyungmoo haritasını açar (aynı üretilmiş doku). Üzerinde oyuncu, NPC adları,
  metin bölgeleri ve canavar bölge etiketleri (seviye aralığıyla) görünür.
- **Kabul kriterleri:** M tuşu haritayı açıp kapatıyor. Oyuncu simgesinin konumu `TerrainData::to_map` ile tutarlı
  (köy meydanı yaklaşık 382, 655). NPC adları okunuyor.
- **WoW assetleri:** ✔ `Interface\WorldMap\WorldMapPlayerIcon.blp`, ✔ `WorldMapPartyIcon.blp`, ✔ `Interface\QuestFrame\UI-QuestLog-BookIcon.blp`

### G-23 · Harita POI veri API'si — gameplay-dev · P1
- **Açıklama:** Minimap ve büyük harita için tek bir kaynak: NPC'ler (ad + kind), aktif metin taşları, canavar
  bölgeleri (ad + seviye aralığı), görev hedefleri.
- **Kabul kriterleri:** `pub fn map_pois(world) -> Vec<Poi>` ya da `Resource<MapPois>` var, her karede ya da
  0,5 sn'de bir güncelleniyor. Birim testinde canavar bölgesi etiketlerinin seviye aralıkları doğru.
- **Dosyalar:** `mobs.rs`, `metin.rs`, `npc.rs`, yeni `map_data.rs`

---

## E4 — Pyungmoo NPC'leri ve hizmetleri

**Wiki koordinatları** (Pyungmoo, harita metresi; kaynak de-wiki Pyungmoo, Türkçe adlar tr-wiki Pyungmoo NPC listesinden):

| NPC (tr) | x, y | İşlev (Metin2) | Öncelik |
|---|---|---|---|
| Demirci | 393, 692 | Artı basma (var) | P0 |
| Satıcı (kadın) | 383, 693 | Genel mağaza: iksir, başlangıç eşyası | P0 |
| Silah Satıcısı | 430, 607 | Silah satışı | P0 |
| Zırh Satıcısı | 403, 586 | Zırh / kask / kalkan / ayakkabı satışı | P0 |
| Depocu | 315, 560 | 500 Yang'a 45 bölmeli depo, depo şifresi. Görev: Demirciye İyilik (Sv6) | P0 |
| Seyis | 396, 735 | Binicilik Bileti → 10 dk Midilli. At Madalyonu (Sv25) → At. At görevleri | P1 |
| Yaşlı Kadın | 417, 671 | Sv30'a kadar beceri sıfırlama, günlük fal, evlilik (Sv25+). Görevler: Kız Çocuğun Düğünü (Sv8), Topraktan Toprağa (Sv13) | P0 |
| Köy Gardiyanı | 383, 640 | Oyun bilgisi, lonca (Sv40, 200.000 Yang). Görevler: Metin2'ye Hoşgeldin! (Sv1), Mi-Jung'u Öldür (Sv20), Hasta Kardeş (Sv22) | P0 |
| Şehir Bekçisi | 468, 714 | Metin2'ye Hoşgeldin! ilk adımı | P0 |
| Işınlayıcı | 432, 685 | Yang karşılığı ışınlanma (Sv10+, her 5 seviyede +1.000 Yang) | P1 |
| Yüzbaşı | 286, 639 | Garnizon komutanı (görevler) | P1 |
| Bedensel Savaş Öğretmeni | 444, 623 | Savaşçı becerileri (Sv5 "Eğitim"), görevler: Deneyimli Asker, Devriyeye Çık (Sv1-4) | P0 |
| Zihinsel Savaş Öğretmeni | (wiki görselinden doğrulanacak; 8 öğretmen var) | Zihinsel savaşçı doktrini | P1 |
| Ticaret Yöneticisi | 401, 675 | (Pazar, bizde dekor) | P2 |
| Savaş Bölgesi Ustası | 371, 703 | (PvP, dekor) | P2 |
| Savaş Sorumlusu | 367, 617 | (dekor) | P2 |
| Ejderha Bekçisi Jaharr | 322, 734 | (dekor) | P2 |
| Simyacı | 292, 812 | (dekor) | P2 |
| Gaya Tüccarı / Gaya Mağazası | 308, 812 / 313, 812 | (dekor) | P2 |
| Posta Kutusu | 316, 554 | (nesne, dekor) | P2 |
| Uriel, Octavio, Baek-Go, Soon, Ah-Yu, Aranyo, Mirine, Yonah, Seon-Hae, Teovahdan | 425,716 · 340,747 · 465,612 · 454,530 · 377,577 · 343,560 · 336,770 · 292,717 · 471,576 · 413,539 | Görev NPC'leri | P1 |
| Handu-Up, Wonda-Rim | 447, 928 · 456, 936 | Görev NPC'leri (köy dışı) | P2 |
| Yaşlı Adam | 125, 1118 | Işınlayıcı (her zaman 1.000 Yang) | P1 |
| Biyolog Chaegirab | 285, 285 | Biyolog görevleri | P2 |
| Balıkçı (8 adet) | 819,315 · 773,905 · 772,472 · 763,809 · 732,390 · 711,234 · 428,480 · 386,849 | Olta, Solucan, Hamur satar. Görev: Başka Bir İyilik (Sv7) | P1 |
| Yanılsama Heykeli, Weol Anıtı, İsimsiz Çiçekler | 342,800 · 114,960 · 771,78 | Nesne / dekor | P2 |
| Sarhoş | (dolaşır) | Dolaşan NPC | P2 |

### G-01 · Pyungmoo NPC'leri: veri tablosu, yerleştirme, konuşma olayı — gameplay-dev · P0 · **Sprint 1**
- **Açıklama:** `npc.rs` veri tabanlı hale gelsin. Tablodaki P0 ve P1 NPC'ler (Balıkçılardan en az köye yakın
  ikisi: 428,480 ve 386,849) wiki koordinatlarında doğsun. Her NPC'nin adı, unvanı, `NpcKind`'ı, zırh seti ve
  elindeki aleti olsun. Tıklanınca `NpcTalked { entity, kind }` mesajı yayınlanır ve sohbet kutusuna Metin2 tarzı
  bir karşılama satırı düşer ("Yaşlı Kadın: Gel evladım, sana falını söyleyeyim..."). Zaten penceresi olan
  işlevler açılır: Demirci → yükseltme; Satıcı, Silah Satıcısı ve Zırh Satıcısı → mağaza (stok ayrımı G-03'te).
- **Kabul kriterleri:**
  - Log'da `npc: N NPC doğdu` (N ≥ 16). Her NPC'nin harita koordinatı wiki değerinden en fazla 6 m uzakta.
    Konum `blocked()` ise en yakın açık noktaya kaydırılır ve log'a yazılır.
  - `AZM_MAPSTART=400,650` ile alınan ekran görüntüsünde en az 4 NPC'nin isim plakası okunuyor (`Ad <Unvan>`).
  - Demirci ve Satıcı eskisi gibi pencere açıyor. Diğer NPC'ler log satırı ve `NpcTalked` mesajı üretiyor.
  - Birim testi: NPC tablosunda yinelenen ad yok ve koordinatların hepsi harita sınırları (1024×1280) içinde.
  - `Npc` bileşeninin `name` ve `title` alanları korunuyor (`ui/world_ui.rs` ve `ui/hud.rs` okuyor).
  - (İsteğe bağlı) Satıcı ve Yaşlı Kadın için HumanFemale modeli. Yapılmazsa raporda belirtilir ve D-10'a kalır.
- **Dosyalar:** `src/game/npc.rs`. İsteğe bağlı: `src/game/character.rs` (kadın gövdesi için küçük bir parametre)
- **Metin2 referansı:** tr-wiki Pyungmoo (NPC listesi), de-wiki Pyungmoo (koordinatlar), tr-wiki Yaşlı Kadın, Köy Gardiyanı, Seyis, Depocu, Balıkçı, Işınlayıcı
- **WoW assetleri:** ✔ `Character\Human\Male\HumanMale.m2`, ✔ `Character\Human\Female\HumanFemale.m2`,
  ✔ `Character\Dwarf\Male\DwarfMale.m2` (Demirci için de kullanılabilir), ✔ `Item\ObjectComponents\Weapon\Misc_2H_FishingPole_A_01.m2` (Balıkçı'nın elinde)

### G-03 · NPC'ye göre mağaza stoğu — gameplay-dev · P0
- **Açıklama:** `GOODS` sabiti `shop_ui.rs` dosyasından gameplay tarafına (`shops.rs`) taşınsın. Satıcı (iksir,
  Tahta takılar, Kutsama Kağıdı), Silah Satıcısı (Sv1-15 kılıçlar), Zırh Satıcısı (zırh, kask, kalkan, ayakkabı)
  ve Balıkçı (Olta, Solucan, Hamur) için ayrı stok olsun. `ActiveShop(NpcKind)` kaynağı eklenir. UI tarafında
  `shop_ui.rs` bu kaynağı okuyacak şekilde değişir (küçük kanca; designer ile sıralı yapılır).
- **Kabul kriterleri:** Silah Satıcısı'nın penceresinde yalnızca silah var. Satıcı'da iksir var. Pencere başlığında NPC adı yazıyor. Birim testi: her stok eşyası `items::def()` ile bulunuyor.
- **Dosyalar:** `shops.rs` (yeni), `npc.rs`, `ui/shop_ui.rs` (yalnızca kanca)
- **Metin2 referansı:** tr-wiki Silah Satıcısı, Zırh Satıcısı, Satıcı

### G-04 · Depocu ve depo — gameplay-dev · P0 (G-02'ye bağlı)
- **Açıklama:** 500 Yang'a 45 bölmeli (5×9) depo açılır. Depo kayda dahil edilir. İsteğe bağlı: depo şifresi.
- **Kabul kriterleri:** İlk konuşmada "500 Yang karşılığında depo kirala?" sorusu çıkıyor. Kiralanınca `Storage` kaynağı
  açılıyor. Envanter ile depo arasında taşıma API'si var. Kaydedip yeniden açınca depo içeriği duruyor (birim testi + elle test).
- **Dosyalar:** `storage.rs` (yeni), `save.rs`, `npc.rs`
- **Metin2 referansı:** tr-wiki Depocu ("500 Yang karşılığında 45 bölmeli bir depo kiralamanızı sağlar")

### D-09 · Depo penceresi — designer · P0 (G-04'e bağlı)
- **Kabul kriterleri:** WoW banka çerçevesinde 5×9 ızgara. Envanter ile sürükle-bırak çalışıyor. Ekran görüntüsü alınır.
- **Dosyalar:** `ui/storage_ui.rs` (yeni)
- **WoW assetleri:** ✔ `Interface\BankFrame\UI-BankFrame-TopLeft/TopRight/BotLeft/BotRight.blp`, ✔ `Interface\ContainerFrame\UI-Bag-Components-Bank.blp`

### G-05 · Diyalog (gossip) veri modeli — gameplay-dev · P0
- **Açıklama:** Her NPC için Metin2 tarzı diyalog: karşılama metni ve seçenekler ("Mağazayı aç", "Depoyu aç",
  "Görev: ...", "Becerilerimi sıfırla", "Günlük fal", "Işınlan: ...", "Hoşça kal"). Seçim bir `NpcAction` mesajı üretir.
- **Kabul kriterleri:** `Dialog { npc, text, options: Vec<DialogOption> }` kaynağı var. Seçeneğin çalıştırılması
  birim testiyle doğrulanıyor. G-01'deki doğrudan pencere açma bu modele taşınıyor.
- **Dosyalar:** `dialog.rs` (yeni), `npc.rs`

### D-08 · NPC diyalog penceresi — designer · P0 (G-05'e bağlı)
- **Kabul kriterleri:** NPC'ye tıklanınca WoW Gossip/Quest çerçevesinde metin ve seçenekler çıkıyor. Seçenek
  türüne göre ikon gösteriliyor (satıcı, banka, eğitmen, görev, ışınlanma).
- **Dosyalar:** `ui/dialog_ui.rs` (yeni)
- **WoW assetleri:** ✔ `Interface\QuestFrame\UI-QuestGreeting-TopLeft/TopRight/BotLeft/BotRight.blp`,
  ✔ `Interface\GossipFrame\VendorGossipIcon.blp`, ✔ `BankerGossipIcon.blp`, ✔ `TrainerGossipIcon.blp`,
  ✔ `TaxiGossipIcon.blp`, ✔ `AvailableQuestIcon.blp`, ✔ `ActiveQuestIcon.blp`, ✔ `GossipGossipIcon.blp`, ✔ `BinderGossipIcon.blp`

### D-10 · NPC görünüm cilası ve baş üstü işaretleri — designer · P1
- **Açıklama:** Her NPC rolüne uygun görünsün: Demirci'de önlük ve çekiç, Satıcı ve Yaşlı Kadın kadın modeli,
  gardiyanlarda plaka zırh ve kalkan, Balıkçı'nın elinde olta. Görev veren NPC'nin başında "!", tamamlanacak
  görevi olanın başında "?" işareti. Yaşlı Kadın için yaşlı görünümlü yüz/saç seçimi.
- **Kabul kriterleri:** Köy meydanı ekran görüntüsünde NPC'ler birbirinden ayırt edilebiliyor. Görev işaretleri görünüyor ve dönüyor.
- **Dosyalar:** `character.rs` (kadın gövdesi), `ui/world_ui.rs`, NPC görünüm tablosu
- **WoW assetleri:** ✔ `Character\Human\Female\HumanFemale.m2`, ✔ `Interface\Buttons\TalkToMe.m2`,
  ✔ `TalkToMeQuestionMark.m2`, ✔ `TalkToMeQuestion_Grey.m2`, ✔ `Item\ObjectComponents\Weapon\Misc_2H_FishingPole_A_01.m2`

### G-21b · Yaşlı Kadın hizmetleri ve Işınlayıcı — gameplay-dev · P1
- **Açıklama:** Yaşlı Kadın Sv30'a kadar becerileri sıfırlar ve günlük fal verir (günde 1 kez, rastgele küçük
  bir bonus). Işınlayıcı ve Yaşlı Adam harita içi noktalara ışınlar: köy, Yaşlı Adam (125, 1118), metin bölgeleri.
  Fiyat wiki formülüyle hesaplanır: `floor(sv/5) × 1.000` Yang (Sv10'dan itibaren), Yaşlı Adam her zaman 1.000.
- **Kabul kriterleri:** Birim testi: Sv22 → 4.000 Yang, Sv9 → 0. Işınlanma sonrası oyuncu hedefin 5 m yakınında
  ve zeminde. `Spells\Teleport.m2` efekti için designer'a kanca bırakılıyor.
- **Dosyalar:** `npc.rs`, `dialog.rs`, `skills.rs`
- **Metin2 referansı:** tr-wiki Işınlayıcı, Yaşlı Kadın

---

## E5 — Eşyalar ve Metin2 düşürme tabloları

### G-10 · Mob başına veri tabanlı düşürme tablosu — gameplay-dev · P0
- **Açıklama:** `metin.rs::mob_drops` içindeki kodla yazılmış kurallar `drops.rs` içinde veri tablosuna dönüşsün:
  `DropTable { mob_id | species | level_range → [(vnum, şans, adet aralığı, artı aralığı)] }`. Tablo içeriği wiki
  mob sayfalarından alınır, bizdeki eşdeğer eşyalarla eşlenir. Örnek: Aç Kurt (Sv3, 27 EXP) → Kurt Pençesi,
  Dönüşüm Küresi (+ yay/yelpaze/çan; savaşçı olmayan silahlar bizde yok, onların yerine Yang). Siyah Ayı (Sv19,
  385 EXP) → Geniş Kılıç, Altın Kolye, Altın Bilezik, Abanoz Küpe, Ayı Ayak Derisi, Dönüşüm Küresi, Kakiçiçeği.
  Metin2'deki seviye farkı kuralı da gelsin: oyuncu, mobdan 15+ seviye yüksekse düşürme azalır.
- **Kabul kriterleri:** Birim testi: 10.000 simülasyonda her mobun düşürme oranı tablodaki değerin ±%2'si içinde.
  Pyungmoo'daki 37 türün hepsinin tablosu var. `mob_drops` sistemi yalnızca tabloyu okuyor. Mobların EXP'leri
  wiki ile karşılaştırılıyor (Aç Kurt 27, Siyah Ayı 385); sapma raporlanıyor.
- **Dosyalar:** `drops.rs` (yeni), `metin.rs`, `mobs.rs` (`drops()` kaldırılır)
- **Metin2 referansı:** tr-wiki her mob sayfası ("Düşebilecek Silahlar / Zırhlar / Diğer Eşyalar"), ör. Aç Kurt, Siyah Ayı

### G-11 · Metin taşı düşürmeleri wiki'ye göre — gameplay-dev · P0
- **Açıklama:** Her metin taşının düşürmeleri wiki listesine uysun. Örnek, Üzüntü Metini (Sv5): Uzun Kılıç,
  Keşiş Plaka Zırh, Beceri Kitabı, Zihin Taşı (+0…+4), Kırmızı İksir (B) ×30, Mavi İksir (B) ×20. Takı ve
  yükseltme malzemesi düşürmez. Çağırdığı moblar: Kurt, Alfa Kurt, Mavi Kurt, Alfa Mavi Kurt, Yaban Domuzu, Kırmızı Yaban Domuzu.
- **Kabul kriterleri:** Beş metin taşının düşürme ve çağırma listeleri wiki ile eşleşiyor (eşlenemeyen eşyalar
  tabloda yorumla belirtiliyor). Birim testi. Oyunda bir Üzüntü Metini kırılınca log'da en az bir iksir yığını görünüyor.
- **Dosyalar:** `metin.rs`, `drops.rs`, `items.rs`
- **Metin2 referansı:** tr-wiki Üzüntü Metini, Dövüş Metini, Savaş Metini, Hırs Metini, Siyah Metin

### G-12 · Eşya kataloğunu genişletme ve Metin2 adlarını doğrulama — gameplay-dev · P1
- **Açıklama:** Eksik Metin2 eşyaları eklensin: Keşiş Plaka Zırh, Kırmızı İksir (O), Mavi İksir (B/O), Dönüşüm
  Küresi, Beceri Kitabı (G-19 ile), Zihin Taşı, Kakiçiçeği, Balık / Solucan / Olta, Binicilik Bileti, At Madalyonu.
  Var olan zırh adları wiki ile karşılaştırılsın ("Aslan Plaka Zırh", "Kaplan Plaka Zırh" doğru mu?). Her eşyaya MPQ'da bulunan bir WoW ikonu.
- **Kabul kriterleri:** Birim testi: `ITEMS` içindeki her ikon `tools/mpq_present.txt` içinde var (test listeyi okur).
  Wiki adlarıyla eşleşmeyen eşyalar raporda listeleniyor.
- **Dosyalar:** `items.rs`
- **WoW assetleri (örnek ikonlar):** ✔ `Interface\Icons\INV_Misc_Book_11.blp` (Beceri Kitabı),
  ✔ `INV_Misc_Orb_01.blp` (Dönüşüm Küresi), ✔ `INV_Misc_Gem_Diamond_01.blp` / `INV_Misc_Gem_Ruby_01.blp` /
  `INV_Misc_Gem_Sapphire_01.blp` (Zihin Taşları), ✔ `INV_Misc_Flower_02.blp` (Kakiçiçeği),
  ✔ `INV_Misc_Fish_09.blp`, ✔ `INV_Fishingpole_01.blp`, ✔ `INV_Misc_Ticket_Darkmoon_01.blp` (Binicilik Bileti),
  ✔ `Ability_Mount_RidingHorse.blp`, ✔ `INV_Potion_93.blp`. ✘ `Spell_Holy_SummonChampion.blp` yok.

### G-13 · "Ruh Taşı" ad çakışmasını giderme — gameplay-dev · P1
- **Açıklama:** Artı basma malzemesi `V_SPIRIT_STONE`'un adı değişsin. Öneri: Metin2'deki yükseltme malzemelerine
  yakın bir ad. "Ruh Taşı" adı G-19'da beceri yükseltme eşyası olarak kullanılsın. Eski kayıtlar bozulmamalı (vnum aynı kalır).
- **Kabul kriterleri:** Oyunda "Ruh Taşı" yalnızca beceri bağlamında geçiyor. Kayıt uyumluluk testi geçiyor.
- **Dosyalar:** `items.rs`, `metin.rs`, `drops.rs`

### G-14 · Adlı mobları ve "Moralsiz" türleri ekleme — gameplay-dev · P1
- **Açıklama:** Wiki'deki Sn.4 mobları eklensin: Cung-Mok (9), Jug-Hyul (10), Mu-Rang (16), Young-Ji (21),
  Mi-Jung / Moralsiz Mi-Jung (23), Li-An (24), Eun-Jung / Moralsiz Eun-Jung (26), Se-Rang / Moralsiz Se-Rang (28),
  Jin-Hee / Moralsiz Jin-Hee (31). İnsan modeli ve farklı zırh setleriyle. Görev hedefi olarak da kullanılır ("Mi-Jung'u Öldür", Sv20).
- **Kabul kriterleri:** Moblar haritada tek tek doğuyor. Log'da konumları yazıyor. `AZM_MOBGAL=1` galerisinde görünüyorlar.
- **Dosyalar:** `mobs.rs`
- **Metin2 referansı:** tr-wiki Pyungmoo canavar listesi

### D-14 · Yerdeki ganimetin görünümü — designer · P2
- **Açıklama:** Ganimet küpleri yerine WoW nesneleri: Yang için para kesesi, eşya için çuval ya da eşyanın kendi
  modeli (kılıç yerde yatar), nadir eşyada parıltı.
- **Kabul kriterleri:** Ekran görüntüsünde küp kalmamış, isim etiketleri yerinde.
- **Dosyalar:** `loot.rs` (yalnızca görsel kısım; gameplay-dev ile sıralı yapılır)
- **WoW assetleri:** ✔ `World\Generic\PassiveDoodads\Furniture\Containers\Sack01.m2`, ✔ `Sack02.m2`, ✔ `Item\ObjectComponents\Weapon\*.m2`, ✔ `Spells\Enchantments\WhiteGlow_Low.m2`

---

## E6 — Görevler (Metin2 tarzı başlangıç zinciri)

### G-06 · Görev motoru — gameplay-dev · P0
- **Açıklama:** Veri tabanlı görevler: adımlar (`Talk(npc)`, `Kill(mob_id, n)`, `Collect(vnum, n)`, `Reach(x, y, r)`,
  `Level(n)`), ödüller (EXP, Yang, eşya, seçmeli eşya), ön koşullar (seviye, önceki görev). Aynı anda birden fazla
  görev, görev günlüğü. Var olan "Metin Taşlarını Yok Et" görevi bu motora taşınır. Kayıtla uyumlu olmalı.
- **Kabul kriterleri:** Birim testleri: adım ilerlemesi, ödül verme, ön koşul. `QuestLog` kaynağı HUD görev takibine besleniyor (var olan metin API'si korunuyor).
- **Dosyalar:** `quest.rs`, `npc.rs`, `save.rs`

### G-07 · "Metin2'ye Hoşgeldin!" görev zinciri — gameplay-dev · P0 (G-06'ya bağlı)
- **Açıklama:** Wiki'deki Genel Görev 1: Şehir Bekçisi ile konuş → Köy Gardiyanı'ndan 200 Yang al → Satıcı'dan
  Kırmızı İksir (K) ×5 al → Köy Gardiyanı'na dön. Ödül: 300 EXP, Tahta Küpe / Tahta Kolye / Tahta Bilezik arasında seçim.
  Ardından Bedensel Savaş Öğretmeni'nin "Deneyimli Asker" ve "Devriyeye Çık" (Sv1-4) görevleri, Sv5'te "Eğitim" (beceri açılışı, G-18).
- **Kabul kriterleri:** Yeni oyunda zincir otomatik başlıyor. Her adım log'da ve görev takibinde görünüyor. Sona
  ulaşınca 300 EXP ve seçilen takı veriliyor. Uçtan uca birim testi (simüle edilmiş mesajlarla).
- **Metin2 referansı:** tr-wiki "Metin2'ye Hoşgeldin!", Bedensel Savaş Öğretmeni, Köy Gardiyanı

### G-08 · Av görevleri (Sv2-20) — gameplay-dev · P1
- **Açıklama:** Wiki Av Görevleri tablosu birebir. Sv2: Aç Yabani Köpek ×10 + Aç Kurt ×5 → zırh +0. Sv3: Aç Yabani
  Köpek ×20 + Aç Kurt ×10 → kask +0. Sv4: Aç Kurt ×15 + Aç Alfa Kurt ×5 → Savaş Kalkanı +0. … Sv20: Aç Kahverengi
  Ayı ×20 + Korkak Beyaz Yeminli Okçu ×15. EXP ödülü: seviyenin gereken EXP'sinin %15'i (Sv11'de %30).
  Seviye atlayınca görev kendiliğinden gelir (Metin2'deki kağıt / mektup hissi).
- **Kabul kriterleri:** 19 görevin hepsi veride var. Birim testi: Sv11 ödülü %30. Seviye atlayınca görev log'da "Yeni görev" olarak görünüyor.
- **Metin2 referansı:** tr-wiki "Av Görevleri"

### G-09 · Köy yan görevleri — gameplay-dev · P2
- **Açıklama:** Yaşlı Kadın: Kız Çocuğun Düğünü (Sv8), Topraktan Toprağa (Sv13). Depocu: Demirciye İyilik (Sv6).
  Balıkçı: Başka Bir İyilik (Sv7). Köy Gardiyanı: Mi-Jung'u Öldür (Sv20). Wiki'de ayrıntısı olmayan adımlar
  Metin2 ruhuna uygun biçimde uyarlanır ve görev metninde "uyarlama" diye işaretlenir.
- **Kabul kriterleri:** 5 görev oynanabiliyor ve birim testleri var.

### D-07 · Görev penceresi (L) ve görev takibi — designer · P1
- **Kabul kriterleri:** L tuşu WoW QuestLog çerçevesini açıyor: solda görev listesi, sağda ayrıntı ve ödül ikonları.
  HUD'daki görev takibi WoW stilinde (başlık sarı, adımlar beyaz). Görev alınınca ve tamamlanınca banner çıkıyor.
- **Dosyalar:** `ui/quest_ui.rs` (yeni), `ui/hud.rs`
- **WoW assetleri:** ✔ `Interface\QuestFrame\UI-QuestLog-TopLeft/TopRight/BotLeft/BotRight.blp`,
  ✔ `UI-QuestLogTitleHighlight.blp`, ✔ `UI-Quest-BulletPoint.blp`, ✔ `UI-HorizontalBreak.blp`, ✔ `UI-QuestItemNameFrame.blp`

---

## E7 — Beceri seviyeleri ve beceri kitapları

### G-18 · Beceri puanı ve beceri seviyeleri — gameplay-dev · P0
- **Açıklama:** Metin2 kuralı: Sv5'te öğretmenden doktrin seçilir (Bedensel / Zihinsel Savaş). Her seviye
  atlamada 1 beceri puanı verilir. Beceri 1'den 16'ya puanla çıkar, 17-20 puanla Usta (M1) olur. Bedensel doktrin
  becerileri: Üç Yönlü Kesme, Kılıç Çevirme, Öfke, Hamle (bizde var) ve yeni olarak Hava Kılıcı ile Yaşama İsteği.
  Hasar ve süre beceri seviyesine göre artar.
- **Kabul kriterleri:** `SkillState { doctrine, points, levels[6] }` kayda dahil. Birim testi: seviye 1 → 16 → M1
  geçişi, hasar katsayısı artıyor. Doktrin seçilmeden beceriler kilitli (Sv1-4'te temel saldırı).
  `AZM_AUTO=1` otopilotu bozulmuyor (otopilot doktrini kendisi seçiyor).
- **Dosyalar:** `skills.rs`, `stats.rs`, `save.rs`
- **Metin2 referansı:** tr-wiki Savaşçı (doktrin beceri listeleri), Beceri Kitabı
- **WoW assetleri (yeni beceri ikonları):** ✔ `Interface\Icons\Ability_Warrior_BattleShout.blp`, ✔ `Spell_Nature_Swiftness.blp`

### G-19 · Beceri Kitabı (M1-M10) ve Ruh Taşı (G) — gameplay-dev · P1 (G-18'e bağlı)
- **Açıklama:** Usta seviyesindeki beceri Beceri Kitabı okunarak yükselir. Her okuma 20.000 EXP harcar, 24 saatte
  bir okunabilir (bizde oyun saati olarak ayarlanabilir, kayıtla birlikte). Başarı şansı var. M10'dan sonra Ruh
  Taşı ile Büyük Usta (G1-G10), sonra Mükemmel (P). Kitap metin taşlarından ve patronlardan düşer (G-11).
- **Kabul kriterleri:** Birim testleri: bekleme süresi, EXP düşümü, başarısızlıkta kitap gidiyor ama seviye
  kalıyor. Log'da "Beceri Kitabı okundu: Kılıç Çevirme M3" satırı.
- **Metin2 referansı:** tr-wiki Beceri Kitabı ("Master beceri seviyesini arttırır", her kullanım 20.000 EXP, 24 saatte bir)
- **WoW assetleri:** ✔ `Interface\Icons\INV_Misc_Book_11.blp`, ✔ `INV_Misc_Book_07.blp`, ✔ `INV_Misc_Rune_05.blp` (Ruh Taşı adayı)

### D-12 · WoW büyü efektleri (beceriler) — designer · P0
- **Açıklama:** Prosedürel halkaların yerine WoW spell M2'leri: Üç Yönlü Kesme → Cleave, Kılıç Çevirme → Whirlwind,
  Hamle → ChargeTrail, Öfke → Bloodlust (elde) ve kırmızı aura, seviye atlama → LevelUp.
- **Kabul kriterleri:** Her becerinin ekran görüntüsünde WoW efekti görünüyor ve efekt bitince temizleniyor
  (sızıntı yok: 60 sn `AZM_AUTO=1` sonrası varlık sayısı sabit, log'a yazılır).
- **Dosyalar:** `skills.rs` (yalnızca görsel fonksiyonlar: `spawn_ring`, `rings`, `rage_aura`), `model.rs` (M2 efekt yükleyici gerekiyorsa)
- **WoW assetleri:** ✔ `Spells\Cleave_Cast_Base.m2`, ✔ `Spells\Cleave_Impact_Chest.m2`, ✔ `Spells\Whirlwind_State_Base.m2`,
  ✔ `Spells\WhirlwindGeoWhite.m2`, ✔ `Spells\ChargeTrail.m2`, ✔ `Spells\Bloodlust_State_Hand.m2`, ✔ `Spells\BloodLust_Cast_Hand.m2`,
  ✔ `Spells\LevelUp\LevelUp.m2`, ✔ `Spells\StunSwirl_State_Head.m2`, ✔ `Spells\ThunderClap_Cast_Base.m2` (Deprem için)

---

## E8 — Kayıt / yükleme

### G-02 · Kayıt / yükleme (save.ron) — gameplay-dev · P0 · **Sprint 1**
- **Açıklama:** Oyuncunun ilerlemesi kalıcı olsun. `SaveData { version, stats, inventory, quest, pos_map }`
  RON olarak yazılır. Yükleme `PreStartup` aşamasında kaynakları yerleştirir; böylece `starter_kit` (Startup) yeni
  eşya vermez. Otomatik kayıt 30 sn'de bir, ayrıca seviye atlayınca, F5'e basınca ve pencere kapanırken
  (`AppExit` / `WindowCloseRequested`). Yazma atomik yapılır (`.tmp` + yeniden adlandırma). Bozuk dosya
  `.bak` olarak saklanır ve yeni oyun başlar. Gelecekteki alanlar (depo, beceriler, statü) `#[serde(default)]` ile eklenecek.
- **Kabul kriterleri:** Bkz. `docs/SPRINT_1.md`.
- **Dosyalar:** `src/game/save.rs` (yeni), `src/game/mod.rs`, gerekirse `stats.rs`, `quest.rs`, `inventory.rs` (serde öznitelikleri). `AZM_SHOT` ya da `AZM_NOSAVE` tanımlıysa kayıt kendiliğinden kapanır, bu yüzden `tools/shot.ps1`'e dokunulmaz.
- **Metin2 referansı:** Metin2'de ilerleme sunucuda tutulur; bizde yerel kayıt.

---

## E9 — Görsel cila

### D-11 · +4…+9 silah parıltısı — designer · P0
- **Açıklama:** Metin2'de artısı yüksek silah parlar. `items::plus_glow` ve `Look.weapon_plus` hazır ama
  kullanılmıyor. Silah modeline WoW büyü (enchant) M2'si takılsın: +4/+5 beyaz düşük, +6 mavi düşük, +7 mavi
  yüksek, +8 mor yüksek, +9 kırmızı yüksek. Yerdeki ve envanterdeki ikon adı rengi zaten var (`loot::item_color`).
- **Kabul kriterleri:** `+0`, `+7`, `+9` kılıçlarla üç ayrı ekran görüntüsü; parıltı silah boyunca görünüyor. Artı
  değişince parıltı anında değişiyor. 2H kılıçta da çalışıyor.
- **Dosyalar:** `character.rs`, `player.rs` (`attach_items` / `rebuild_look` görsel kısmı)
- **WoW assetleri:** ✔ `Spells\Enchantments\WhiteGlow_Low.m2`, ✔ `BlueGlow_Low.m2`, ✔ `BlueGlow_Med.m2`, ✔ `BlueGlow_High.m2`,
  ✔ `PurpleGlow_High.m2`, ✔ `RedGlow_High.m2`, ✔ `RedFlame_Low.m2` (+9 için alternatif), ✔ `Spells\Enchantments\Flare_Wide.blp`

### D-13 · Olay efektleri: metin kırılması, ışınlanma, iksir — designer · P1
- **Kabul kriterleri:** Metin taşı kırılınca patlama ve toz efekti, ışınlanmada Teleport efekti, iksir içince iyileşme efekti. Her biri ekran görüntüsüyle doğrulanır.
- **Dosyalar:** `metin.rs` (yalnızca görsel), `skills.rs` (görsel)
- **WoW assetleri:** ✔ `SPELLS\DustNova_Cast_Base.m2`, ✔ `SPELLS\Meteor_Impact_Base.m2`, ✔ `Spells\Teleport.m2`,
  ✔ `Spells\Heal_Low_Base.m2`, ✔ `Spells\Food_HealEffect_Base.m2`, ✔ `SPELLS\Creature_SpellPortal_Purple.m2` (metin çağırma)

### D-15 · Köy dekoru — designer · P1
- **Açıklama:** Pyungmoo köyü daha "yaşayan" bir yer olsun: fenerler, çitler, fıçılar, sandıklar, çadırlar,
  NPC tezgâhları, kuyu. Yerleşim wiki köy görüntüsüne yakın. Nesneler yürüme engeli olur.
- **Kabul kriterleri:** Köy meydanında en az 25 dekor nesnesi var. NPC'lerin önü açık. FPS düşüşü %10'dan az (log'da FPS).
- **Dosyalar:** `terrain.rs` (yalnızca `BUILDINGS`/dekor tablosu; gameplay-dev'e bildirilir)
- **WoW assetleri:** ✔ `World\Azeroth\Elwynn\PassiveDoodads\LampPost\LampPost.m2`, ✔ `World\Azeroth\Elwynn\PassiveDoodads\ElwynnFences\ElwynnFenceSimple.m2`,
  ✔ `World\Generic\PassiveDoodads\Barrel\Barrel01.m2`, ✔ `World\Generic\PassiveDoodads\Crate01\Crate01.m2`,
  ✔ `World\Generic\Buildings\HumanTentLarge\HumanTentLarge.m2`, ✔ `World\Azeroth\Westfall\PassiveDoodads\BrokenCart\BrokenCart.m2`,
  ✔ `World\Azeroth\Stranglethorn\PassiveDoodads\HolySpringWell\HolySpringWell.m2`, ✔ `World\wmo\Azeroth\Buildings\Duskwood_Stable\Duskwood_Stable.wmo`

---

## E10 — Ses

### G-21 · Ses altyapısı ve oyun olayı sesleri — gameplay-dev · P1
- **Açıklama:** `Cargo.toml` içinde Bevy'ye `bevy_audio` + `wav` (+ müzik için `mp3`) özellikleri eklenir. MPQ'dan
  bayt okuyan bir `AudioSource` yükleyici yazılır. Olay → ses eşlemesi yapılır: vuruş, kritik, ıska, mob saldırısı
  ve ölümü (türe göre), seviye atlama, ganimet alma, Yang alma. Ses düzeyi ayarı ve `AZM_MUTE=1`.
- **Kabul kriterleri:** Log'da ses yükleme hatası yok. Kurt saldırınca kurt sesi çalıyor (elle test + log satırı).
  `AZM_MUTE=1` sesi tamamen kapatıyor. Derleme süresi ve exe boyutu raporlanıyor.
- **Dosyalar:** `Cargo.toml`, `audio.rs` (yeni), `combat.rs`, `loot.rs`, `stats.rs`
- **WoW assetleri:** ✔ `Sound\Item\Weapons\Sword1H\m1hSwordHitFlesh1a.wav`, ✔ `m1hSwordHitFleshCrit.wav`,
  ✔ `Sound\Item\Weapons\Sword2H\m2hSwordHitFlesh1a.wav`, ✔ `Sound\Item\Weapons\WeaponSwings\mWooshMedium1.wav`,
  ✔ `Sound\Creature\Wolf\mWolfAttack1.wav`, ✔ `mWolfDeath1.wav`, ✔ `Sound\Creature\Boar\mWildBoarAttack1.wav`,
  ✔ `Sound\Creature\Bear\mBearAttackB.wav`, ✔ `Sound\Creature\Tiger\mTigerAttackC.wav`, ✔ `Sound\interface\LevelUp.wav`,
  ✔ `Sound\Interface\LootCoinSmall.wav`, ✔ `Sound\interface\LootCoinLarge.wav`, ✔ `Sound\Spells\CleaveTarget.wav`, ✔ `Sound\Spells\WhirlwindShort.wav`

### D-17 · Arayüz sesleri ve ortam sesi / müzik — designer · P2 (G-21'e bağlı)
- **Kabul kriterleri:** Pencere açılıp kapanırken, eşya alınıp bırakılırken (eşya türüne göre) ve düğmeye tıklanırken ses çalıyor. Köyde orman ortam sesi, dışarıda hafif müzik.
- **WoW assetleri:** ✔ `Sound\interface\iEquipmentContainerOpenA.wav`, ✔ `uCharacterSheetOpen.wav`, ✔ `iQuestActivate.wav`,
  ✔ `iQuestComplete.wav`, ✔ `Sound\interface\PickUp\PickUpMetalSmall.wav`, ✔ `PutDownRing.wav`, ✔ `iMoneyDialogOpen.wav`,
  ✔ `Sound\Ambience\ZoneAmbience\ForestNormalDay.wav`, ✔ `Sound\Music\ZoneMusic\Forest\DayForest03.mp3`,
  ✔ `Sound\Interface\NPC\Human\Male\HumanMaleMerchantCStartShopping01.wav` (satıcı selamı)

---

## E11 — Binek (at)

### G-22 · Binek sistemi (Seyis) — gameplay-dev · P1
- **Açıklama:** Seyis'ten Binicilik Bileti karşılığı 10 dakikalık "Midilli". Sv25'ten itibaren At Madalyonu ile
  kalıcı binek (Metin2 At seviyeleri sadeleştirilmiş: Genç / Asker / Usta). Binekteyken hareket hızı artar,
  saldırı yapılamaz (Metin2'de at üstü saldırı ileri seviyede). Tuş: Ctrl+G (Metin2) ya da H yerine yeni tuş.
- **Kabul kriterleri:** Biletle binince 10 dk geri sayım HUD'da görünüyor, süre bitince iniliyor. Hız artışı birim testiyle ölçülüyor. Binek durumu kayda dahil.
- **Dosyalar:** `mount.rs` (yeni), `player.rs` (hız), `npc.rs`
- **Metin2 referansı:** tr-wiki Seyis (Binicilik Bileti → 10 dk Midilli, At Madalyonu Sv25)
- **Karar:** MPQ'da at modeli yok (bkz. Bölüm 0). Önerilen model `RidingDireWolf`.

### D-18 · Binek görseli ve binme animasyonu — designer · P1 (G-22'ye bağlı)
- **Kabul kriterleri:** Oyuncu binek üstünde "Mount" animasyonuyla oturuyor, binek yürüme/koşma animasyonu
  hızla senkron. Ekran görüntüsü alınır. Binek simgesi HUD'da.
- **WoW assetleri:** ✔ `Creature\DireWolf\RidingDireWolf.m2` + ✔ `RidingDireWolfSkinBrown.blp` (ve 7 deri daha),
  ✔ `Creature\DireWolf\RIDINGDIREWOLFSADDLE.blp`, ✔ `Creature\MountedDeathKnight\RidingUndeadWarHorse.m2` (alternatif),
  ✔ `Interface\Icons\Ability_Mount_WhiteDireWolf.blp`, ✔ `Ability_Mount_RidingHorse.blp`,
  ✔ `Sound\Creature\Horse\mFootStepsHorseGrass01.wav` (ayak sesi). ✘ `Creature\Horse\*.m2` yok.

---

## E12 — Kalite ve araçlar

### Q-01 · Sprint 1 doğrulaması — qa-engineer · P0 · **Sprint 1**
- Bkz. `docs/SPRINT_1.md`.

### Q-02 · Asset varlık denetimi aracı — qa-engineer · P1
- **Açıklama:** `tools/check_assets.py`: `src/**` içindeki bütün `mpq://...`, `Interface\\...`, `Spells/...`,
  `.blp/.m2/.wmo/.wav` yollarını çıkarıp `tools/mpq_present.txt` ile büyük/küçük harf duyarsız karşılaştırır.
  `items.rs` ikonlarını `Interface\Icons\<ad>.blp` biçimine çevirir.
- **Kabul kriterleri:** Çıktıda YOK listesi var. Bugünkü kodda 0 eksik bekleniyor (PM grep ile ikonları kontrol etti).

### Q-03 · Kayıt dosyası regresyon testleri — qa-engineer · P1
- **Kabul kriterleri:** Eski sürüm kayıt örnekleri (`tests/fixtures/save_v1.ron`) her sprintte yükleniyor. Yeni alanlar eklenince test kırılmıyor.

### Q-04 · Otomatik duman (smoke) senaryoları — qa-engineer · P1
- **Açıklama:** `tools/smoke.ps1`: köy, metin bölgesi (`AZM_MAPSTART=725,660;AZM_AUTO=1`), mob galerisi, pencereler
  (`AZM_OPEN=...`), NPC'ler (`AZM_MAPSTART=400,650`) senaryolarını sırayla çalıştırır. PNG'leri `docs/qa/<tarih>/`
  klasörüne yazar, log'larda `panic|B0001|Path not found` arar.
- **Kabul kriterleri:** Tek komutla bütün senaryolar çalışıyor ve özet tablo üretiyor.

### Q-05 · Performans ölçümü — qa-engineer · P2
- **Kabul kriterleri:** Köyde ve metin bölgesinde 60 sn ortalama FPS ve varlık sayısı `docs/QA_RAPORU.md`'ye yazılıyor; sprintler arasında karşılaştırılıyor.

---

## 3. Önerilen sprint sırası (taslak)

| Sprint | Tasarımcı | Oyun geliştirici | QA |
|---|---|---|---|
| 1 | D-01 pencere derisi | G-01 NPC'ler + G-02 kayıt/yükleme | Q-01 |
| 2 | D-11 silah parıltısı + D-03 karakter penceresi | G-10 + G-11 düşürme tabloları (+ G-13) | Q-02 + Q-03 |
| 3 | D-08 diyalog penceresi + D-10 NPC cilası | G-05 diyalog + G-03 mağaza stokları + G-04 depo | Q-04 |
| 4 | D-09 depo penceresi + D-07 görev penceresi | G-06 görev motoru + G-07 Hoşgeldin zinciri | Q-01 benzeri |
| 5 | D-12 büyü efektleri + D-05 minimap | G-18 beceri seviyeleri + G-14 adlı moblar | — |
| 6 | D-04 beceri penceresi + D-02 HUD | G-19 beceri kitabı + G-08 av görevleri | — |
| 7+ | D-06, D-13, D-15, D-17, D-18 | G-20, G-21, G-21b, G-22, G-23, G-09 | Q-05 |
