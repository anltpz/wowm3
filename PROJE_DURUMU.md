# AzerothMetin — Proje Durumu

Metin2 tarzı aksiyon RPG; AzerothSurvivors'ın MPQ verisi (WoW 1.12 assetleri) ve motoru (benilla)
üzerinde. **Yeni oturumda önce bu dosyayı oku.**

## Kimlik / kararlar (kullanıcı onaylı, 2026-10-14)
- **Teknoloji:** Rust + Bevy 0.18.1 masaüstü exe (kullanıcı seçti; web önerisi reddedildi).
- **Harita:** gerçek Elwynn/Northshire (MPQ'daki ADT + WMO + doodad), benilla-world motoru akıtıyor.
- **Hedef özellikler:** metin taşı kesme, Metin2 benzeri envanter, silahlara artı basma (+0…+9).

## Mimari
- `benilla-world` (git rev `2af917044d25bc929ee7edab25ab0a0628c59fab`, `~/.cargo/git/checkouts/benilla-*`):
  arazi, WMO, doodad, gökyüzü, ışık, çarpışma (`WorldCollision::ray_body`), dünya akışı (`ViewFocus::body`).
  Açılış `src/boot.rs` = benilla-worldview `run()`'ının oyunlu kopyası.
- **Birim modelleri (`src/game/model.rs`):** benilla'nın kendi GPU palet/rig yolu istemciye bağlı olduğu için
  KULLANILMIYOR; `M2Model` (benilla-assets yükler) → Bevy standart `SkinnedMesh` + `AnimationPlayer`
  (`bone_target_id` + `AnimatedBy`), `StandardMaterial`. Mesh'ler model başına önbellekli.
- **Karakter (`src/game/character.rs`):** `CharSections::composite_body` (deri+yüz+saç+iç çamaşırı+zırh
  dokuları) + `CharacterGeosets::visible_geosets` + ItemDisplayInfo'dan silah modeli (sağ el = takma 1).
- Veri klasörü: `WOW_DATA` env > exe yanında `Data` > `C:\Users\ANIL\Desktop\AzerothSurvivors\Data`.
- Grafik arka ucu varsayılan **DX12** (Vulkan, takılan bir süreçten sonra kilitlendi; `WGPU_BACKEND` ile değişir).

## Test araçları
- `tools/shot.ps1 -Out a.png -At "20,30" -Env "AZM_YAW=3.14;AZM_DIST=4"` → ekran görüntüsü alıp çıkar.
- Env: `AZM_SHOT`, `AZM_SHOT_AT`, `AZM_YAW`, `AZM_DIST`, `AZM_GALLERY=1` (model galerisi).
- Python MPQ okuyucu (scratchpad'de `mpq.py`/`dbc.py`): listfile'daki ~1850 dosya MPQ'da YOK
  (ör. `Doodads\StoneRunes\*`), varlık için `present.txt` kontrolü gerekir.

## Seçilen assetler
- Metin taşı: `World/Azeroth/BurningSteppes/PassiveDoodads/Rocks/LavaRock01.m2` (lav çatlaklı obsidyen dikilitaş)
  + zemin efekti `World/Goober/G_RuneGroundPurple01.m2`.
- Moblar: Wolf (WolfSkinTimber/Black/Arctic), Boar (Brown/Crimson), Kobold (Normal/Albino).
- Kılıçlar (ItemDisplay): 1546 Kılıç, 1550 Uzun Kılıç, 1853 Hilal Kılıç, 3234 Bambu Kılıç, 4443 Gümüş Kılıç,
  4908 Zehir Kılıcı, 2416 Dolunay Kılıcı (2H), 3758 Ejderha Kılıcı (2H).
- Zırh setleri (8 yuva: gömlek, göğüs, kemer, pantolon, çizme, bileklik, eldiven, tabard):
  - Gümüş Plaka: göğüs 16184, kemer 3418, pantolon 3414, çizme 25828, bileklik 4448, eldiven 25831
  - Altın Savaşçı: 29958, 29959, 29963, 29960, 29961, 29962
  - Mavi Ejder: 31021, 31019, 31023, 31025, 31020, 31022
  - Kızıl Kartal: 33628, 33632, 33630, 33631, 33627, 33629
- Silah parıltıları (artı basma görseli için): `Spells/Enchantments/{White,Blue,Purple,Red}Glow_{Low,High}.m2`.
- Fontlar: `Fonts/FRIZQT__.TTF` (Türkçe karakterler), `Fonts/SKURRI.TTF` (hasar sayıları).

## Kilometre taşları
- [x] M0 — Dünya açılışı (Northshire), 3. şahıs kamera, zemine oturan oyuncu, ekran görüntüsü aracı
- [x] M0.5 — Animasyonlu WoW modelleri (kurt/domuz/kobold), giydirilmiş İnsan savaşçı + elde kılıç
- [x] M1 — Savaş (`combat.rs`): sol tık hedef + otomatik saldırı, Space, tıkla-yürü, %6 ıska ("Miss"),
  %12 kritik ("CRITICAL!"), EXP/seviye (`stats.rs`, max 40), ölüm + R ile doğma. Moblar (`mobs.rs`):
  8 tür, uzaklığa göre zorlaşan 4 bölge, sürü yeniden doğma, AI dolaş/kovala/saldır/dön, patron "Ak Kurt Lideri".
- [x] M2 — Metin taşları (`metin.rs`): Hüzün/Savaş/Açgözlülük/Kara Metin, %80/60/40/20'de mob dalgası,
  rün dairesi + renkli ışık, kırılınca ganimet, 40 sn sonra başka yerde. Yerdeki ganimet (`loot.rs`), Z ile topla.
- [x] M3 — Envanter (`inventory.rs` + `ui/inventory_ui.rs`): 5×9×2 sayfa, 1×1/1×2/1×3, sürükle-bırak,
  Shift ile böl, sağ tık kuşan/kullan, ekipman yuvaları, tooltip (`ui/tooltip.rs`), görünüm otomatik değişir.
- [x] M4 — Demirci + artı basma (`ui/upgrade_ui.rs`): +0…+9, şans %95→%20, Yang + malzeme, başarısızlıkta
  eşya yok olur; Kutsama Kağıdı/Ejderha Pulu → -1 artı; El Kitabı +%10. Tüccar (`ui/shop_ui.rs`), NPC'ler (`npc.rs`).
- [x] M5 — Yeni HUD (`ui/hud.rs`, kullanıcının tasarım görseline göre): prosedürel HP/SP küreleri + grifon
  süsleri, beceri (1-4) + iksir (F1-F4) çubuğu, bekleme karartması, EXP çubuğu, portreli oyuncu çerçevesi,
  hedef çubuğu, radar + bölge adı, görev takibi (`quest.rs`), sohbet kutusu, oyuncu isim plakası (<Lonca>).
- [x] Beceriler (`skills.rs`): 1 Üç Yönlü Kesme, 2 Kılıç Çevirme, 3 Öfke (buff), 4 Hamle; SP + bekleme.
- [ ] Sonraki: kayıt/yükleme (save.ron), +7…+9 silah parıltısı (enchant M2 / emissive), gerçek minimap
  dokuları, ses efektleri, beceri seviyeleri, parti/lonca görselleri (tasarım görselindeki sol liste).


## Pyungmoo haritası (2026-10-14, kullanıcı isteği: "Metin2 mavi bayrak haritasının aynısı")
- Kaynak: resmi Metin2 wiki görselleri (de/en/tr-wiki.metin2.gameforge.com/index.php/Pyungmoo): 1024x1280
  kuşbakışı harita (1 px = 1 m = oyun koordinatı), canavar bölge haritası, metin bölge haritası, NPC koordinatları.
  Oyun istemci dosyası KOPYALANMADI (makinede Metin2 istemcisi yok; sızdırılmış paketler telifli → kullanılmadı);
  harita görselden yeniden üretildi.
- `tools/build_pyungmoo.py <Pyungmoo.jpg>`: k-means renk sınıfları → su/yol/çim/kaya → yükseklik (dağ = içe mesafe),
  splat (yol/kaya/kum/dere), su maskesi, 682 ağaç + 260 çam → `assets/pyungmoo/` (exe'ye gömülür).
- `tools/build_zones.py <ref klasörü>`: metin bölgeleri rasterı (Üzüntü/Dövüş elle eklendi), canavar bölgeleri
  `mobs.rs` PY_ZONES'ta elle (yakın plan → harita: m = z·(0.988, 1.259) − (8.3, 15.1)).
- `terrain.rs` + `terrain.wgsl`: 64 m parçalar, splat shader (WoW Elwynn çim/çiçek, Westfall toprak/kum, Alterac kaya),
  su düzlemi, 8 köprü (yürünebilir), 12 WMO köy binası (`model.rs` WowWmo), garnizon duvarı, zemin/su/yamaç/bina engeli.
- Dünya akışı boş okyanusa (WoW −8000, 14000) odaklanır; benilla gökyüzü + ışık kalır, ADT yüklenmez.
- `AZM_ELWYNN=1` eski Northshire modu; `AZM_MAPSTART=x,y` harita koordinatında başlangıç (test).
- Canavarlar tr-wiki Pyungmoo listesinden (adlar/seviyeler birebir), düşürmeler WoW ikonlu Metin2 malzemeleri.

## Test
- `AZM_AUTO=1` otopilot: en yakın düşmanı hedefler, becerileri sırayla kullanır (seviyeyi 6'ya çeker).
- Bevy `debug` özelliği açık: B0001 sorgu çakışmalarında sistem adı görünür.
- Türkçe glifler WoW fontlarında yok → UI fontu `C:\Windows\Fonts\pala.ttf` (Palatino), yoksa georgia/segoeui.

## Bilinen sorunlar / notlar
- Takılan (öldürülemeyen) bir oyun süreci exe'yi kilitlerse: `target/release/azeroth-metin.exe`'yi yeniden adlandır.
- Birim aydınlatması benilla'nın ışık tamponunu kullanmıyor (kendi DirectionalLight + GlobalAmbientLight).
