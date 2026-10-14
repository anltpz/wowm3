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
