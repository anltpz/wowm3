# QA Raporu — Sprint 1

- **Tarih:** 2026-10-14
- **Sprint:** 1 ("WoW görünümlü pencereler, yaşayan Pyungmoo köyü, kalıcı ilerleme")
- **Test eden:** qa-engineer (Q-01)
- **Dal / commit:** `sprint-1` @ `2306224` (G-01 + G-02 `e749a8a`, `95097b4`; D-01 `9f09297`)
- **Ortam:** Windows 11 Pro 26200, AMD Ryzen 7 7700, NVIDIA RTX 4070 SUPER (DX12), pencere 1600×900, Rust + Bevy 0.18.1, `--release`

## Genel sonuç: **GEÇTİ** (engelleyici hata yok)

Sprint kabul kriterlerinin hepsi geçti. Tek kısmi madde D-01/6'daki "sağ tık kullan" ile kapat düğmesinin vurgu
durumu; bunlar elle kontrol edilmeli. 2 orta ve 6 düşük önemde hata bulundu (aşağıda). Hiçbir çalıştırmada
`panic`, `B0001`, `Path not found` ya da shader hatası yok.

---

## 1. Derleme ve testler

| Komut | Sonuç |
|---|---|
| `cargo build --release` | Temiz, 0 hata, **7 uyarı**. Uyarıların hepsi ilk commit'ten (`87d25e9`) kalma, bu sprintte yeni uyarı yok (`skills.rs:336, 95`, `hud.rs:1006`, `character.rs:205`, `combat.rs:52`, `items.rs:393`, `model.rs:105`). |
| `cargo test --release` | **16/16 geçti**, 0 başarısız. Yeni testler: G-01 `npc::tests` 6 adet, G-02 `save::tests` 7 adet. Önceden olan: `inventory::tests` 3 adet. |

## 2. Ekran görüntüleri

Dosyalar: `C:\Users\ANIL\AppData\Local\Temp\claude\C--Users-ANIL\a4d6a5e4-3872-4844-8f32-0ca23ba43f6d\scratchpad\qa\s1\`
(`.png` ve aynı adlı `.log`). Bu klasör oturuma özel; görüntüler kalıcı olsun isteniyorsa `docs\qa\s1\` altına kopyalanmalı.
Betimler görüntülere bakılarak yazıldı.

| Dosya | Ortam | Görülen |
|---|---|---|
| `a_meydan.png` | `AZM_MAPSTART=400,660;AZM_DIST=14;AZM_YAW=3.14` | Köy meydanı (toprak alan). Okunan isim plakaları: **Demirci `<Eşya Yükseltme>`**, **Satıcı `<Genel Mağaza>`** (demirci binasının önünde), **Yaşlı Kadın `<Falcı>`**, **Işınlayıcı `<Işınlanma>`** (sol kenarda, mavi çatılı ahırın üstünde; NPC ahırın arkasında kalıyor). Toplam 4 plaka. |
| `b1_pencereler.png` | `AZM_OPEN=inv,shop,upgrade;AZM_DIST=10` (görevdeki gibi, MAPSTART yok) | Yalnızca Envanter açık. Varsayılan başlangıçta (382,662) yakında satıcı yok, bu yüzden `auto_close` mağaza ve demirci pencerelerini kapatıyor (tasarım gereği). Bu çalıştırmada makinedeki gerçek fare/klavye girdisi oyuna karıştı: Köy Gardiyanı ile konuşuldu ve beceri tuşlarına basıldı (otomasyon kodu bunları üretmiyor). |
| `b2_pencereler_ortada.png` | `AZM_MAPSTART=388,690;AZM_OPEN=inv,shop,upgrade;AZM_DIST=10` | **Risk 6 doğrulandı:** Satıcı ve Demirci pencereleri aynı anda açık ve aynı yerde üst üste duruyor. Demirci penceresinin yarı saydam arka planından alttaki mağaza satırları ("Kırmızı İksir (K) x10 … Satın Al") görünüyor. Solda Satıcı, sağda Demirci modeli var. |
| `b3_shop.png` | `AZM_MAPSTART=378,692;AZM_OPEN=inv,shop;AZM_DIST=8` | "Satıcı — Genel Mağaza" penceresi: WoW gri taş çerçeve, koyu dokulu arka plan, başlık plakası, sağ üstte kırmızı X (CancelButton). 12 satırın her birinde ikon, ad, fiyat, altın sikke ve kırmızı "Satın Al" düğmesi var. Altta "Satmak için: eşyayı buraya sürükle ya da sağ tıkla" yazısı ve "Kapat" düğmesi. Sağda Envanter. |
| `b4_upgrade.png` | `AZM_MAPSTART=394,690;AZM_OPEN=inv,upgrade;AZM_DIST=8` | "Demirci — Eşya Yükseltme": sol üstte altın ejderha süsü, altın tonlu çerçeve ve kahverengi (Gold) arka plan. "Eşyayı bırak" yuvası, açıklama kutusu, Kutsama Kağıdı / El Kitabı / Ejderha Pulu onay kutuları, "Yükselt" ve "Kapat" düğmeleri. Envanter normal (gri) çerçevede. |
| `z_upgrade_frame.png`, `z_inv_frame.png`, `z_shop_frame.png` | (büyütülmüş kırpımlar) | Envanter: 8 ekipman yuvası (boşlarda PaperDoll silüetleri: kask, kolye, küpe, kalkan, bileklik, ayakkabı), I/II sekmeleri, slot dokuları, altta "2.000" ve altın sikke simgesi. |
| `b5_tooltip.png` | `AZM_OPEN=inv;AZM_HOVER=equip:weapon` | Silah yuvası mavi `ButtonHilight` ile parlıyor. Tooltip WoW çerçevesinde (gümüş yuvarlak kenar, koyu mor arka plan): "Kılıç+0 / Tek el kılıç / Saldırı değeri 14 - 18 / Sonraki artı: +3 saldırı / Satış: 50 Yang". |
| `b6_hover_cell.png` | `AZM_OPEN=inv;AZM_HOVER=cell:2` | 3. hücredeki miğfer mavi vurgu çerçevesinde, "Miğfer+0" tooltip'i açık. |
| `c_metin_mob.png` | `AZM_MAPSTART=725,660;AZM_AUTO=1;AZM_DIST=16;AZM_PITCH=-0.35`, `-At 40` | 3 adet "Lv.15 Boz Ayı" modeli düzgün ve animasyonlu. Oyuncu (Lv 6) ölmüş: "Öldün / Köyde yeniden doğmak için R tuşuna bas". Hedef çubuğu "Boz Ayı (Lv. 15) 185/950". Metin taşı kadrajda değil. |
| `c2_metin_0.png`, `c2_metin_1.png` | aynı ortam, `-At "14,24"` | 14. sn: Lv.10 Aç Kırmızı Yaban Domuzu ve Lv.13 Gri Kurt sürüleri, oyuncu koşuyor. 24. sn: oyuncu ölü. |
| `c3_metin_yawn.png` | `AZM_MAPSTART=725,660;AZM_DIST=30;AZM_PITCH=-0.5;AZM_YAW=-1.57` | Ufukta iki metin taşı (koyu dikilitaş) görünüyor. Sağda 3 "Lv.15 Aç Boz Ayı" var. |
| `d_mobgal.png` | `AZM_MOBGAL=1;AZM_YAW=3.14159;AZM_DIST=14` | Galeride Lv.30 Lykos, Lv.18 Beyaz Yeminli Asker, Lv.24 Beyaz Kaplan, Lv.18 Kaplan, Lv.3 Kurt, Lv.7 Yaban Domuzu ve Lv.12 Ayı var. Modellerin hepsi doğru. Ancak oyuncu (Lv 1) ölmüş (bkz. H6). Arkada Demirci ve Satıcı plakaları görünüyor. |
| `e_kusbakisi.png` | `AZM_DIST=260;AZM_PITCH=-1.5` (ayrı oturumda) | Kuşbakışı köy: dairesel meydan, garnizon duvarı, beyaz kule, dere ve köprüler. Plakalar: Köy Gardiyanı, Yaşlı Kadın, Satıcı ve Demirci (bu iki plaka üst üste biniyor), oyuncu. |
| `f_yasli_adam.png` | `AZM_MAPSTART=128,1112;AZM_DIST=10` | Uzak NPC: sol altta "Yaşlı Adam `<Işınlayıcı>`" plakası ve beyaz saçlı model var. |
| `g_save_run2.png` | Kayıt testi 2. çalıştırma (aşağıda) | Lv 6, HP 505/680, 2.027 Yang, EXP %2.1. Envanterde 5 eşya var (Kırmızı İksir 30, Mavi İksir 10, miğfer, ayakkabı, kağıt), çoğalma yok. Oyuncu metin bölgesindeki çayırda. |
| `h_f5.png` / `z_f5_chat.png` | F5 testi | Sohbet kutusunda iki kez yeşil "[Sistem] Oyun kaydedildi." yazıyor (F5'e iki kez basıldı). |
| `i_yuzbasi.png` | `AZM_MAPSTART=292,646;AZM_DIST=30;AZM_PITCH=-1.0` | Yüzbaşı `<Garnizon>` plakası mavi çatının üstünde. NPC görünmüyor, garnizon binasının içinde (bkz. H1). |
| `m0.png` | Fare: Satıcı'ya tıklama | Satıcı penceresi ve Envanter açık. Sohbette "[Sistem] Satıcı: Hoş geldin yolcu! Kırmızı iksirlerim taze, fiyatlarım köyün en iyisi." yazıyor. **Arka plan bulanık kahverengi doku**, çünkü kamera Demirci binasının içinde (bkz. H2). |
| `m1_*.png`, `z_m1_buttons.png`, `z_m1_inv.png` | Fare: satın alma, sağ tık, uzaklaşma | "Satın Al" üstüne gelince parlak, basılınca koyu. Satın alma Kırmızı İksir'i 30'dan 40'a çıkardı. Mağaza açıkken sağ tık miğferi sattı ("Geleneksel Miğfer+0 satıldı: +80 Yang", Yang 1.880). W ile uzaklaşınca mağaza penceresi kapandı, envanter açık kaldı. |
| `m2_*.png`, `z_m2_inv.png` | Fare: sürükle-bırak, sağ tık kuşan, Shift ile bölme | Kırmızı İksir 30, 3. satıra taşındı. Miğfer kask yuvasına takıldı ("Geleneksel Miğfer+0 kuşanıldı.", silüet gizlendi). Mavi İksir 10, iki adet 5'e bölündü. |
| `m3_0.png` | Fare: Demirci'ye tıklama, II sekmesi, Esc | Sohbette Demirci karşılama satırı var. Esc ile yükseltme penceresi kapandı. Envanter II. sayfada (boş). Sol altta kamera bir bina duvarının içine girmiş (beyaz düzlem). |
| `m4.png`, `z_m4_up.png`, `z_m4_after.png` | Fare: sağ tık → yükseltme yuvası → Yükselt | Sağ tık miğferi yuvaya koydu (%95 şans, 360 Yang, "Başarısızlıkta: EŞYA YOK OLUR!"). Yükselt'e basınca: "Yükseltme başarılı! Geleneksel Miğfer+1". Yang 2.000'den 1.640'a düştü, sonraki adım %90 / 1.440 Yang. |

Not: Fare/klavye etkileşimleri user32 (`mouse_event`/`keybd_event`, scan code'lu) ile otomatikleştirildi
(betik: scratchpad `qa\mouse.ps1`). Oyun saati pencere açıldıktan yaklaşık 8 sn sonra başlıyor; `AZM_SHOT_AT` oyun saatine göredir.

## 3. Kayıt testi (G-02/2, Q-01/3)

`AZM_SAVE=%TEMP%\azm_qa.ron`

1. **1. çalıştırma** (`Start-Process`, AZM_SHOT yok, `AZM_AUTO=1;AZM_MAPSTART=725,660`, 47 sn, sonra `Stop-Process`):
   ```
   00:55:50 save: yeni oyun (C:\Users\ANIL\AppData\Local\Temp\azm_qa.ron)
   00:56:02 save: yazıldı Sv6 2000 Yang      ← seviye atlama (otopilot Sv6'ya çekiyor)
   00:56:32 save: yazıldı Sv6 2027 Yang      ← 30 sn otomatik kayıt
   ```
   Dosya 1596 bayt: `level: 6, exp: 63, hp: 1` (oyuncu ölüydü, en az 1 kuralı çalıştı), `sp: 14`, envanter 5 eşya
   (27001×30, 27004×10, 12200, 15000, 25040), ekipman silah 10 + zırh 11200, `yang: 2027`, `quest: (stage: 1, kills: 0)`,
   `pos_map: Some((741.87305, 681.89014))`. Kopyası scratchpad'de: `qa\s1\azm_qa_run1.ron`.
2. **2. çalıştırma** (`shot.ps1`, `AZM_SAVE` + `AZM_OPEN=inv`, MAPSTART yok):
   ```
   00:56:56 save: yüklendi Sv6 2027 Yang (C:\Users\ANIL\AppData\Local\Temp\azm_qa.ron)
   00:57:31 save: yazıldı Sv6 2027 Yang      ← AppExit ile kapanış kaydı
   ```
   Seviye, Yang ve envanter aynı kaldı, başlangıç eşyaları çoğalmadı (iksirler 30/10). Oyuncu kayıttaki konumda doğdu:
   görüş odağı, 725,660 başlangıcına göre tam (+16.9, +21.9) m kaydı. Bu fark kayıttaki konumla aynı.
3. **F5** (`AZM_SAVE=%TEMP%\azm_qa_f5.ron`, SendKeys ile 2× F5): iki kez `save: yazıldı` ve sohbette iki kez "Oyun kaydedildi.".
   Çıkışta üçüncü yazma yapıldı (AppExit).
4. **AZM_SHOT çalıştırmaları kullanıcı kaydını bozmuyor:** `%APPDATA%\AzerothMetin\save.ron` testlerden önce de sonra da
   yok. ~25 ekran görüntüsü çalıştırmasında oluşmadı, `target\release\save\` de oluşmadı. Log'da `save: kayıt kapalı (AZM_SHOT / AZM_NOSAVE)` yazıyor.

## 4. Log taraması

Bütün çalıştırmalarda (23 log): **panic 0, B0001 0, `Path not found` / `not found` 0, shader/wgsl/naga 0, ERROR 0.**
Tekrarlayan uyarılar yalnızca şunlar:
- `npc: Yüzbaşı (286, 639) çevresinde açık nokta yok, yerinde doğuyor` (her çalıştırmada, bkz. H1).
- `terrain: view focus … is OFF map Azeroth's tile grid`. Pyungmoo boş okyanusa odaklandığı için beklenen bir uyarı (PROJE_DURUMU).
- Bilgi satırları: `npc: Işınlayıcı (432, 685) engelli, (433.9, 685.5) noktasına kaydırıldı`, `npc: Octavio (340, 747) engelli, (339.0, 748.1) noktasına kaydırıldı`, `npc: 19 NPC doğdu`.

## 5. Kabul kriterleri

### D-01 · WoW pencere derisi
| # | Kriter | Sonuç | Kanıt |
|---|---|---|---|
| 1 | Envanter/Tüccar/Demirci/tooltip WoW dokulu, düz `PANEL_BG` yok | **GEÇTİ** | b3, b4, b5. `PANEL_BG`/`panel()` yalnızca `hud.rs:634`'te kullanılıyor (kapsam dışı) |
| 2 | Demirci altın çerçeve, diğerleri normal | **GEÇTİ** | b4/z_upgrade_frame: altın ejderha süsü + altın çerçeve. Envanter ve mağaza gri |
| 3 | Slot dokusu, `ButtonHilight` vurgu, PaperDoll silüetleri | **GEÇTİ** | z_inv_frame, b5, b6, m2 (dolu yuvada silüet gizleniyor) |
| 4 | Satın Al / Yükselt / kapat düğmeleri WoW dokulu, vurgu + basılma | **GEÇTİ** (kısmen elle) | z_m1_buttons: Satın Al vurgu ve basılma durumları görülüyor. Yükselt/Kapat/X dokulu. X ve Kapat'ın vurgu durumları ayrıca elle bakılmalı |
| 5 | Yang yanında altın sikke | **GEÇTİ** | z_inv_frame "2.000" + sikke, mağaza fiyatları |
| 6 | Gerileme yok: sürükle-bırak, Shift böl, sağ tık kuşan/kullan/sat, artı basma, sayfa, tooltip | **GEÇTİ** ("kullan" elle test gerekli) | m2: sürükle-bırak, Shift böl, kuşan. m1: sat. m4: artı basma. m3: sayfa II. b5: tooltip. Sağ tıkla iksir "kullan" denenmedi |
| 7 | `AZM_OPEN` | **GEÇTİ** | Log: `ui: AZM_OPEN uygulandı: [...]`; b2–b6 |
| 8 | Önce/sonra ekran görüntüleri | **GEÇTİ** | b3/b4 (görüntüler scratchpad'de, `docs\qa\s1\` değil) |
| 9 | `Interface\` için `Path not found` yok, yeni uyarı yok | **GEÇTİ** | §1, §4 |

### G-01 · Pyungmoo NPC'leri
| # | Kriter | Sonuç | Kanıt |
|---|---|---|---|
| 1 | `npc: 19 NPC doğdu` (≥16) | **GEÇTİ** | Her log |
| 2 | Birim testleri: yinelenen yok, koordinat sınırı, her tür için karşılama | **GEÇTİ** | `table_has_no_duplicates`, `coords_inside_map`, `every_kind_has_greeting` |
| 3 | Meydanda ≥4 isim plakası | **GEÇTİ** | a_meydan (4 plaka). Not: sprintteki komutta `AZM_YAW` yok, onsuz kamera kuzeye bakıyor. `AZM_YAW=3.14` eklendi |
| 4 | Demirci yükseltme, satıcılar mağaza açıyor; 8 m+ uzakta kapanıyor | **GEÇTİ** | Oyunda: Satıcı (m0/m1), Demirci (m3). Silah Satıcısı `talk_writes_message_and_log` birim testiyle, Zırh Satıcısı `shop_kinds` testiyle (`is_shop`). Kapanma: m1_3 (mağaza kapandı) |
| 5 | Diğer NPC'lerde karşılama satırı + `NpcTalked` | **GEÇTİ** | b1 (Köy Gardiyanı satırı + `npc: konuşuldu Köy Gardiyanı (Guard)`), birim testi (Depocu) |
| 6 | `panic` ve `B0001` yok | **GEÇTİ** | §4 |
| 7 | `ui/**` dosyalarına dokunulmadı | **GEÇTİ** | `git show --stat e749a8a 95097b4`: yalnızca character, inventory, mod, npc, quest, save, stats, player |

### G-02 · Kayıt / yükleme
| # | Kriter | Sonuç | Kanıt |
|---|---|---|---|
| 1 | Birim testleri (a) gidiş-dönüş (b) eksik alan (c) bozuk metin | **GEÇTİ** | `ron_roundtrip` (+7 silah, yığın, Yang, görev), `old_file_with_missing_fields_loads`, `corrupt_text_is_error_not_panic` (+ `file_write_load_and_corrupt_backup`) |
| 2 | İki çalıştırmalı elle test | **GEÇTİ** | §3. Seviye, EXP, Yang ve konum korundu, eşya çoğalmadı. Oyunda artılı ekipman olmadığı için "+artı korunuyor" yalnızca birim testiyle doğrulandı. Pencerenin X'i ile kapatma (`WindowCloseRequested`) denenmedi, AppExit yolu çalışıyor |
| 3 | `AZM_SHOT` kullanıcı kaydını değiştirmiyor | **GEÇTİ** | §3/4 |
| 4 | F5 → "Oyun kaydedildi." | **GEÇTİ** | h_f5, z_f5_chat |
| 5 | build/test temiz, `ui/**` dokunulmadı | **GEÇTİ** | §1, G-01/7 |

### Q-01
| # | Madde | Sonuç |
|---|---|---|
| 1 | Derleme + test özeti | **GEÇTİ** (§1) |
| 2 | Ekran görüntüleri | **GEÇTİ** (§2; scratchpad'de) |
| 3 | Kayıt senaryosu | **GEÇTİ** (§3) |
| 4 | Log taraması | **GEÇTİ** (§4) |
| 5 | Kriterlerin işaretlenmesi | **GEÇTİ** (§5) |
| 6 | Rapor | Bu dosya |

---

## 6. Bulunan hatalar

| # | Önem | Sahip | Hata | Kanıt / öneri |
|---|---|---|---|---|
| H1 | **Orta** | gameplay-dev | **Yüzbaşı garnizon binasının içinde doğuyor**, oyuncu onu görmüyor ve tıklayamıyor. Wiki koordinatı (286,639) çevresinde 6 m içinde açık nokta yok. | `i_yuzbasi.png` (plaka çatının üstünde), her log'da WARN. Öneri: `NPCS` tablosunda Yüzbaşı'yı garnizon kapısının önündeki açık bir noktaya taşımak ya da satıra özel daha büyük bir kaydırma yarıçapı vermek. |
| H2 | **Orta** | gameplay-dev (designer onayıyla) | **Kamera köy binalarının içine giriyor.** Pyungmoo modunda kamera kolu çarpışması kapalı (`camera.rs:89`: `cast_camera` yalnızca `terrain::data().is_none()` iken çalışıyor). Satıcı ve Demirci binanın dibinde durduğu için onlarla konuşurken ekran bina dokusuyla kaplanıyor. Hata önceden vardı, G-01'in yerleşimi yüzünden artık sık görülüyor. | `m0.png` (kahverengi bulanık arka plan), `m3_0.png` (sol altta beyaz duvar). Öneri: kamera kolunu `TerrainData::blocked` ayak izleri / WMO sınırları boyunca yürütüp kısaltmak. |
| H3 | Düşük | designer | **Risk 6:** Satıcı ve Demirci arası 10 m. Orta noktada (388,690) ikisi de 8 m içinde olduğu için `auto_close` iki pencereyi de açık bırakıyor. Pencereler aynı yerde üst üste biniyor ve üstteki yarı saydam arka plan alttaki metni gösteriyor. **Normal oyunda oluşmuyor:** `npc::open` Demirci açılınca `shop`'u, satıcı açılınca `upgrade`'i kapatıyor. `auto_close` ise yalnızca kapatıyor, dışlamayı zorlamıyor. Klavyeyle (I/H/Esc) iki NPC penceresini birden açmanın yolu yok. Durum yalnızca `AZM_OPEN=…shop,upgrade` ile oluşuyor. | `b2_pencereler_ortada.png`. Öneri: `env_open` içinde shop ile upgrade'i dışlayıcı yapmak ya da iki pencereye ayrı konum vermek. Sprint 2'de bir `Windows` yardımcı fonksiyonuyla (`open_npc_window`) dışlamayı tek yere toplamak. |
| H4 | Düşük | designer (D-02) | İsim plakaları binaların arkasından da görünüyor: Işınlayıcı'nın plakası ahırın üstünde, Yüzbaşı'nınki çatının üstünde. Aşırı uzaklıkta Satıcı ve Demirci plakaları üst üste biniyor. | `a_meydan.png`, `i_yuzbasi.png`, `e_kusbakisi.png` |
| H5 | Düşük | designer (D-02, `hud.rs` sohbet kutusu) | NPC karşılama satırları "[Sistem] Satıcı: …" önekiyle çıkıyor. G-01'in biçimi "<Ad>: <metin>". Önek sohbet kutusu çiziminden geliyor (`GameLog` metni doğru). | `m0.png`, `m3_0.png` |
| H6 | Düşük (test aracı) | gameplay-dev | `AZM_MOBGAL` galerisi "edilgen" değil. Saldırgan türler (`def.aggressive`, 9 m menzil; galeri 8 m önde diziyor) oyuncuya saldırıyor ve Lv 1 oyuncu ölüyor. Hata bu sprintten önce de vardı. | `d_mobgal.png`. Öneri: galeri moblarına "edilgen" işareti, `mobs.rs:665`'te kontrol |
| H7 | Düşük (test senaryosu) | product-manager / qa | Sprintteki "metin + mob" senaryosu (725,660 + `AZM_AUTO=1`, `-At 40`): Lv 6 otopilot, Lv10–15 moblara ~20 sn'de ölüyor ve metin taşı varsayılan kadraja girmiyor. | `c_metin_mob.png`, `c2_metin_*.png`. Öneri: `-At 15` ve/veya `AZM_YAW=-1.57;AZM_DIST=30` (`c3_metin_yawn.png`'de metinler görünüyor) |
| H8 | Düşük (araç) | orkestratör (`tools/**` kimsenin değil) | `tools/shot.ps1` ortam değişkenlerini çağıran PowerShell sürecine yazıyor ve geri almıyor. Aynı oturumda ikinci çağrıya sızıyor: ilk kuşbakışı denemesi `AZM_MOBGAL=1`'i miras aldı. | Öneri: betiğin sonunda `-Env` ile verilen anahtarları ve `AZM_SHOT*`'ı `Remove-Item env:` ile temizlemek |

Bilgi notları (hata değil):
- `AZM_OPEN=shop`/`upgrade` yakında NPC yoksa NPC'ler doğunca hemen kapanıyor; bu tasarım gereği (`env_open` yorumu). Bu pencereler için `AZM_MAPSTART` ile NPC'nin yanına başlanmalı.
- `b1` çalıştırmasında, oyun penceresi odağı alınca makinedeki gerçek kullanıcı girdisi oyuna geçti. Ekran görüntüsü testleri sırasında klavye/fareye dokunulmamalı.

## 7. Yapılan düzeltmeler

Kod düzeltmesi yapılmadı. Bulunan hataların hiçbiri tek satırlık ve risksiz değil: H1 için güvenli bir koordinat
oyun içinde doğrulanmalı, H2 tasarım gerektiriyor, H8 `tools/**` altında. Bu commit yalnızca bu raporu ekliyor.

## 8. Sprint 2 düzeltmeleri (2026-10-14, `sprint-2`)

| # | Durum | Düzeltme | Doğrulama |
|---|---|---|---|
| H1 | **DÜZELDİ** | Asıl sebep: `BUILDINGS` ayak izleri elle tahmin edilmiş kutulardı ve binalardan çok küçüktü. Kışla gerçekte x 283–315, y 623–657 aralığını kaplıyor ve WMO kökü binanın dışında. Ayak izleri artık MPQ'daki WMO iç mekân grup sınırlarından alınıyor (`[x0, x1, z0, z1]`, binanın kendi eksenlerinde). Yüzbaşı kışlanın güneyindeki avluya (300, 663) taşındı. Kışla doğu kapısını kapattığı için kapı y 653'ten y 662'ye alındı. Kışlanın içinden geçen çit parçaları artık çizilmiyor. | Testler: `every_npc_has_open_spot`, `barracks_footprint_matches_visual`, `east_gate_is_south_of_barracks`. Ekran görüntüsü: Yüzbaşı doğu kapısından görünüyor, log'da WARN yok. |
| H2 | **DÜZELDİ** | Pyungmoo'da kamera kolu bina ayak izleri (+1 m pay, çatı yüksekliği 11 m) boyunca yürütülüp kısaltılıyor (`terrain::camera_hit`). | Demirci binasına bakan kamera (386,696, yaw 0) binanın dışında kalıyor. |
| H3 | **DÜZELDİ** | `Windows::open_shop` / `open_upgrade` tek yerde ve birbirini dışlıyor. `npc::open` ve `AZM_OPEN` bunları kullanıyor. | Derleme, kod incelemesi |
| H4 | **DÜZELDİ** | NPC ve düşman plakaları bina arkasındaysa gizleniyor (`terrain::occluded`). Çakışan NPC plakalarından kameraya yakın olan kalıyor. Binekteyken oyuncu plakası binicinin üstüne çıkıyor. | Meydan çekiminde Işınlayıcı'nın ahırın üstündeki plakası artık yok, Demirci ve Satıcı plakaları ayrı duruyor. |
| H5 | **DÜZELDİ** | `GameLog::say` (önek yok) NPC konuşmalarında kullanılıyor. `[Sistem]` öneki yalnızca sistem satırlarında. | Kod incelemesi (oyun içinde NPC'ye tıklanmadı) |
| H6 | **DÜZELDİ** | `mobs::Passive` bileşeni eklendi. Galeri mobları hareket etmiyor ve saldırmıyor. | `AZM_MOBGAL=1`, 25. saniyede oyuncu canı 280/280 |
| H7 | **BELGELENDİ** | Önerilen komut `docs/TAKIM.md`'ye yazıldı. | Yok |
| H8 | **DÜZELDİ** (Sprint 1 son commit'i) | Yok | Yok |

---

## Q-06 — ItemDisplay denetimi ve bulucu (2026-10-14, dal `test/display-denetim`)

Araç: `src/game/display_audit.rs` (yalnızca `#[cfg(test)]`, iki `#[ignore]` test). Dosya varlığı zincirin karma
tablosundan (`Chain::contains`) okunur; `tools/mpq_present.txt` ile çapraz kontrol: **0 uyuşmazlık**.

```
cargo test --release -- --ignored display_audit --nocapture --test-threads=1     # denetim (geçersiz varsa FAIL)
AZM_FIND=Helm_Plate_RaidWarrior cargo test --release -- --ignored display_find --nocapture
AZM_FIND=Shield_ AZM_FIND_OK=1 ...      # yalnız dosyaları tam olanlar
AZM_FIND=4908,2416 ...                  # kimlik listesi
```
Kask: `Item\ObjectComponents\Head\<stem>_HuM.m2` (+ `_HuF` uyarısı), omuzluk: `model[0]`=L, `model[1]`=R,
gövde: `Item\TextureComponents\<Bölge>\<ad>_U.blp` / `_M.blp`; yuvanın kendi katmanlarında çizilebilir doku yoksa
"karakterde görünmez" hatası.

### Denetim sonucu: 141 benzersiz (tür, kimlik) → **25 hata, 7 kimlik** (test şu an FAIL, beklenen)

| Kimlik | Kullanan | Sorun | Öneri |
|---|---|---|---|
| 33729 (silah) | items: Quel'Serrar | `sword_1h_ahnqiraj_d_01button.m2` MPQ'da yok | **33839** (aynı model `sword_1h_ahnqiraj_d_01`, ikon INV_Sword_57) |
| 5317 (göğüs) | items: Kızıl Haçlı Zırhı | AL `Cloth_A_01Black_Sleeve_AL` yok (ön kol çıplak) | **6147** veya **34915** (Crusader AU+AL+TU+TL tam) |
| 5319 (kemer) | Kızıl Haçlı | `Plate_A_01Crusader_Belt_LU` yok → görünmez | 3418 (Silver) |
| 5321 (çizme) | Kızıl Haçlı | Crusader Boot LL/FO yok → görünmez | 25828 (Silver) |
| 5322 (bileklik) | Kızıl Haçlı | Crusader Bracer AL yok → görünmez | 4448 (Silver) |
| 5323 (eldiven) | Kızıl Haçlı | Crusader Glove HA yok → görünmez | 25831 (Silver) |
| 8550–8553 | mob: Defias Haydudu/Yankesicisi/Eşkıyası, Garrick Padfoot | `Leather_A_03DefiasBrown_*` hiçbiri MPQ'da yok → 4 yuva görünmez | Gövde: 3719/4473/4469/9773 (Leather_A_03Brown) ya da Red 9983/8129/8130; Defias maskesi (kask): 8266/13690; omuz 8549 |

Not: Crusader dokularından MPQ'da yalnız Sleeve AU/AL, Chest TU/TL, Pant LU/LL var; Belt/Boot/Bracer/Glove yok.
Valor/Might/Wrath/Dreadnaught setlerinin 6 yuvası da tam. Dreadnaught 35058 (kemer) ve 35044 (bileklik) satırları
başka katmanlarda da Plate_A_01Black taşıyor; composite yalnız yuvanın katmanlarını çizdiği için sorun değil (uyarı).
PlayerLook, tüm NPC setleri ve PROJE_DURUMU'ndaki 8 Metin2 kılıcı (1546, 1550, 1853, 3234, 4443, 4908, 2416, 3758) geçerli.

**Görünmeyen eşyaların asıl nedeni:** `wear(...)` eşyalarının 9'unda (miğfer ×2, kalkan ×2, çizme ×2, bileklik ×3)
`display = 0` — hiç görünüm kimliği yok; ayrıca kask/kalkan/çizme/bileklik `PlayerLook`'a bağlanmıyor (G-25).

### Öneri: `wear(...)` eşyaları (tümü MPQ'da tam, kasklarda `_HuM` ve `_HuF` var)

| Eşya | Öneri | Model / doku | Alternatif |
|---|---|---|---|
| Barbar Demir Miğferi | **1132** | helm_mail_a_02 / Helm_Mail_A_02Silver (INV_Helmet_02) | 29965 helm_plate_d_03 Brown |
| Aslanyürek Miğferi | **22901** | helm_plate_d_03 / Helm_Plate_D_03WhiteRed (INV_Helmet_13) | 27306 (Silver) |
| Aşınmış Tahta Kalkan | **18653** | shield_round_b_01 / WoodSteelCap | 2208 buckler_damaged_a_01 Rusty (INV_Shield_09) |
| Kafatası Alevi Kalkanı | **23825** | shield_horde_c_03 (dikenli koyu) | 32559 shield_zulgurub_d_01, 32558 zulgurub_d_02 (MPQ'da "skull" adlı kalkan yok) |
| Acemi Çizmeleri | **2229** | LL+FO Mail_A_02 Boot (INV_Boots_01) | 3261 (yalnız FO, kumaş) |
| Barbar Demir Çizmeleri | **25828** | LL+FO Plate_A_01Silver Boot | 29960 DungeonWarrior Gold (INV_Boots_Plate_03) |
| Bakır Bileklikler | **3622** | AL Mail_A_02_Bracer (INV_Bracer_07) | 4448 Plate Silver |
| Kızıl Bilek Koruyucuları | **4448** | AL Plate_A_01Silver_Bracer | (Crusader bileklik dokusu MPQ'da yok) |
| Cesaret Bileklikleri | **29961** | AL Plate_DungeonWarrior_A_01Gold_Bracer (INV_Bracer_18) | — (Valor setiyle aynı) |

### Öneri: set başına kask + omuzluk

| Set | Kask | Omuzluk |
|---|---|---|
| Kızıl Haçlı | **4415** / 6146 helm_plate_a_01crusader | **6141** L/RShoulder_Plate_A_01 + Shoulder_Plate_A_01Crusader; kalkan 4404 / 15818 shield_crest_a_02 Crusader |
| Valor (T0, Gold) | **31284** helm_plate_dungeonwarrior_a_01 Gold | **29964** L/RShoulder_Plate_DungeonWarrior_A_01 |
| Might (T1) | **31259** (31260) helm_plate_raidwarrior_a_01 Blue | **31024** L/RShoulder_Plate_RaidWarrior_A_01 Blue |
| Wrath (T2) | **33741** helm_plate_raidwarrior_b_01 (33981 Black) | **33679** L/RShoulder_Plate_RaidWarrior_B_01 (33988 Black) |
| Dreadnaught (T3) | **35784** helm_plate_raidwarrior_c_01 (gövde dokusu taşımayan satır) | **35189** L/RShoulder_Plate_RaidWarrior_C_01 |

Kaçınılacak: 33677 (`helm_plate_raidwarrior_b.m2` yok), 12872 (`helm_plate_a_01.m2` yok).

### Öneri: Metin2 ikonik kılıçları (hepsi MPQ'da tam)

| Metin2 | Şimdiki (geçerli) | Alternatifler |
|---|---|---|
| Kılıç | 1546 short_a_01 Rusty | 1593 short_a_02 Silver |
| Uzun Kılıç | 1550 long_a_02 Brown | 2806 long_d_01, 25053 long_d_02 Blue |
| Hilal Kılıç | 1853 scimitar_a_02 | 1592 sabre_b_02 Orange |
| Bambu Kılıç | 3234 katana_b_01 Green | 9370 katana_a_01, 16128 katana_b_02 Black |
| Geniş Kılıç | — | 18156 horde_b_01 Silver, 30994 long_d_03 |
| Gümüş Kılıç | 4443 sabre_a_01 Silver | 1593 short_a_02 Silver |
| Zehir Kılıcı | 4908 fire_a_01 + Long_A_02Green | 22378 horde_b_05 Green, 32529 zulgurub_d_02 |
| Dolunay Kılıcı (2H) | 2416 claymore_b_02 White | 21194 2h_horde_c_03 Blue, 5057 2h_katana_b_01 Blue |
| Ejderha Kılıcı (2H) | 3758 claymore_b_02 Gold | 23259 2h_horde_d_01, 34112 2h_epic_a_03, 29098 2h_ashbringer_a_01 Blue, 35255/35256 2h_stratholme_d |
| Diğer 2H (Metin2 hissi) | — | 729 2h_korean_c_01, 9037 2h_korean_a_01, 9121 2h_katana_b_02 Black |

---

## Q-08 — Dalga 1 QA: ekipman görünümü + Westfall (2026-10-14, dal `test/dalga1`)

- **Test eden:** qa-engineer (Q-08)
- **Commit:** `main` @ `eda1876` (#5 Q-06, #6 G-24, #7 G-25, #8 G-26, #9 D-21, #10 G-27 birleşik)
- **Ortam:** Sprint 1 raporundakiyle aynı (DX12, 1600×900, `--release`)
- **Ekran görüntüleri:** `C:\Users\ANIL\AppData\Local\Temp\claude\C--Users-ANIL\aab144fc-ff3e-4967-983a-b5dfac098313\scratchpad\qa8\`
  (`.png` ve aynı adlı `.log`). Klasör oturuma özel; görüntülerde WoW assetleri olduğu için depoya konmadı.

### Genel sonuç: **GEÇTİ** (engelleyici hata yok)

### 1. Derleme ve testler
| Komut | Sonuç |
|---|---|
| `cargo build --release` | Temiz, 6 uyarı. Hepsi önceki sprintlerden; Dalga 1 yeni uyarı eklemedi. |
| `cargo test --release` | **74/74** geçti (3 `#[ignore]`). |
| `cargo test --release -- --ignored display_audit` | **2/2** geçti. D-21 silahları ve G-25 kask/kalkan/omuzluk dahil bütün ItemDisplay kimlikleri MPQ'da var. |

### 2. Ekipman (G-25)
Ortam: `AZM_EQUIP=11220,12210,13010,15010,14010`.
| Dosya | Ortam | Görülen |
|---|---|---|
| `e5_yakin.png` | `AZM_YAW=3.14;AZM_DIST=2.2;AZM_PITCH=-0.15` | Önden yakın çekim. Kırmızı siperlikli kask kafada, saç ve kulak görünmüyor. Might omuzlukları iki omuzda, dikenli kalkan sol kolda, kılıç sağ elde. Set dokuları (göğüs, kemer, bileklik, çizme) doğru. |
| `e1_onden.png` | `AZM_YAW=3.14;AZM_DIST=4` | Aynı görünüm, uzaktan. Envanterde kuşanılan yuvalar dolu. |
| `e2_arkadan.png` | `AZM_DIST=4` | Arkadan: omuzluk dikenleri, kaskın arkası, kalkanın sırtı. Kopuk ya da yüzen parça yok. |
| `e3_binek_onden.png`, `e4_binek_yan.png` | `AZM_RIDE=1` | Ölüm atının üstünde kask, omuzluk ve kalkan doğru yerde, gövde eyerde. Yan çekimde isim plakası kask boynuzlarına biniyor (bkz. Q8-3). |

Binekteyken eşya değiştirince 1 karelik titreme düzeltmesi (K8) yalnızca kod okunarak doğrulandı (`player.rs` `rebuild_look`, eski `ChildOf` ve `Transform` devrediliyor). Otomatik testte binekteyken eşya değiştirme girdisi yok.

### 3. Westfall (G-27)
| Dosya | Ortam | Görülen |
|---|---|---|
| `w1_jansen_auto.png` | `AZM_FOCUS=-9850,1000,33;AZM_AUTO=1` + ekipman, 45 sn | Otopilot Jansen Stead sürüleriyle savaştı, oyuncu Sv1'den **Sv6**'ya çıktı. Mini harita "The Jansen Stead". Çiftlik tarlası ve korkuluk doodad'ları var. |
| `w3_saldean.png` | `AZM_FOCUS=-10150,1070,37` | "Saldean's Farm". İki katlı ev (WMO) yerinde, **Salma Saldean `<Breadbasket Loncası>`** plakası var, mini haritada sürüler görünüyor. |
| `w4_dustplains.png` | `AZM_FOCUS=-11100,860,42` | "The Dust Plains". **Hırs Metni (Sv20)** ve can çubuğu görünüyor. Mavi gövdeli Westfall ağaçları doğru. |
| `w5_jangolode.png` | `AZM_FOCUS=-9940,1430,42` | "Jangolode Mine". Oyuncu tepe yamacında, mini haritada sürüler var. |
| `w2_longshore.png` | `AZM_FOCUS=-9720,1270,14` | Kuzey sahil, kum ve su geçişi. Mini haritada murloc sürüleri var. Alt bölge adı "Westfall" görünüyor (bkz. Q8-2). |

- Hiçbir çalıştırmada `panic` ya da `B0001` yok.
- Satır denetimi (`python -I tools/adt_zones.py check src/game/westfall.rs`) 0 uyarı verdi: sürü, metin ve NPC satırlarının hiçbiri suda ya da yanlış bina üstünde değil.

### 4. Bulunan sorunlar
| # | Önem | Sorun | Sahip önerisi |
|---|---|---|---|
| Q8-1 | Orta | **Moonbrook ve Westfall çiftlik binaları MPQ'da yok.** `moonbrook_inn/blacksmith/townhall_nowall/humantwostory`, `westfall_human_farm`, `westfall_barn`, `westfall_lumbermill`, `westfall_stable`, `prisonlonghouse`, `az_deadmines_a/c` → benilla `failed to load: nothing spawned`. `tools/mpq_present.txt` doğruluyor (yalnızca `Moonbrook_human_farm` ve `Westfall_human_farm_Burnt` var). Moonbrook'un Defias sürüleri boş bir meydanda duruyor. Kod hatası değil, veri paketi kısıtı. | designer + gameplay-dev: eksik yerleşimleri MPQ'da olan benzer WMO'larla değiştiren bir eşleme (ör. `Westfall_human_farm_Burnt`, `Moonbrook_human_farm`, Elwynn binaları), ya da Moonbrook sürülerini binası olan bir yere taşımak. Sprint 6 adayı. |
| Q8-2 | Düşük | Kuzey sahildeki murloc köyü (-9690, 1280) "Longshore" çemberinin dışında kalıyor, mini harita "Westfall" yazıyor. Çemberler %88 isabetli; sahil şeridi tek çemberle iyi örtülmüyor. | **DÜZELTİLDİ** (bu dal): ADT probe'una göre iki küçük çember eklendi (-9670, 1250 / 1120, r 70; daha büyüğü x -9750'deki Furlbrow'a taşıyordu) + test. `w6_longshore_duzeltme.png`: mini harita "Longshore", murloc'lar ve Old Murk-Eye (Sv20) sahilde. |
| Q8-3 | Düşük | Binekteyken isim plakası boynuzlu kaskın üstüne biniyor (Sprint 2'den kalan "binekteyken plaka yüksekliği" maddesi). | **DÜZELTİLDİ** (bu dal): `world_ui.rs` binekli plaka yüksekliği 3.2 → 3.6 m. `e6_binek_yan_duzeltme.png`: plaka boynuzların üstünde. |
| Q8-4 | Düşük | Westfall'un bazı doodad'ları (çeşme, cenaze arabası, tabelalar, Darkshore kütükleri) MPQ'da yok, log'da `Path not found` görünüyor. Zararsız. | — |

### 5. Kabul kriterleri
| Kriter | Sonuç |
|---|---|
| G-25: Kask kafada ve saç görünmüyor; kalkan kolda; omuzluk görünüyor (önden, arkadan, binekte) | **GEÇTİ** |
| G-25: Geçersiz display kimliği yok | **GEÇTİ** (display_audit) |
| G-27: Alt bölge adları doğru | **GEÇTİ** (Q8-2 düzeltildi) |
| G-27: Yaratıklar suda ya da çatıda değil | **GEÇTİ** (adt_zones check, 0 uyarı) |
| G-27: Metin alanları ve Westfall dalgaları | **GEÇTİ** (Hırs Metni, Dust Plains) |
| Log: `panic` / `B0001` yok | **GEÇTİ** |
