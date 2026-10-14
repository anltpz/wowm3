# Sprint 5 — "Ekipman görünümü, Metin2 zindanları, Westfall, Metin2 sistemleri"

Hazırlayan: Ürün Yöneticisi · Başlangıç: 2026-10-14 · Durum: **AÇIK**

**Sprint hedefi:** Oyuncunun giydiği her eşya karakterin üstünde görünsün. Elwynn'in ötesinde Westfall (Sv 10-20)
açılsın. Metin2 tarzı iki katlı zindan (Örümcek Zindanı, Şeytan Kulesi) gerçek instance olarak oynansın ve
bosslardan Metin2'nin ikonik silahları düşsün. Ardından efsun, taş takma ve beceri seviyeleri gelsin.

**Kullanıcı istekleri:**
- Ajan takımı (PM, designer, gameplay-dev, qa-engineer, release-manager) iş dağıtımı yapsın.
- **Her görev ayrı dal ve ayrı PR** olsun.
- WoW kaynakları Metin2 oynanışıyla kullanılsın.
- Metin2 tarzı katlı **zindan** sistemi: Örümcek Zindanı ve Şeytan Kulesi.
- Bosslardan Metin2'nin ikonik silahları düşsün (Dolunay, Zehir, Ejderha…).
- **Giyilen eşyalar karakter üzerinde görünmüyor**; düzeltilsin.
- Kapsama Westfall bölgesi ve Metin2 sistemleri (efsun, taş takma, beceri seviyeleri) eklendi.

**Kullanıcı kararları:**
- Silahlar: Metin2 adı + en benzer WoW modeli.
- Zindan girişi: seviye + bilet + bekleme süresi. Bekleme süresi gerçek zamanla işler (Örümcek 30 dk, Kule 60 dk).
- Zindanlar **gerçek instance**: boş okyanus bölgesinde kendi WMO'muz ve kendi çarpışmamız.
- Şeytan Kulesi **4 kat MVP** olarak başlar.
- Zindanda ölen oyuncu **kaldığı kattan devam eder**.

---

## Görevler

| Dalga | Kimlik | Sahip | Öncelik | Amaç (tek cümle) | Dal | Boyut |
|---|---|---|---|---|---|---|
| 0 | G-24 | gameplay-dev | P0 | Çakışma önleyici refaktör: içerik kayıt noktaları (`zones.rs`, `ITEM_TABLES`, `MOB_TABLES`, NPC zinciri, `metin::spawn_metin`, `physics::INDOOR`), taslak modüller, `mod.rs` satırları. Davranış değişmez | `refactor/icerik-kayitlari` | M |
| 0 | Q-06 | qa-engineer | P0 | `display_audit.rs` (`#[ignore]` testler): ItemDisplay kimliği + MPQ dosya denetimi, `AZM_FIND` arama | `test/display-denetim` | S |
| 1 | G-25 | gameplay-dev | P0 | Giyilen bütün eşyalar (zırh, kask, kalkan, çizme, bileklik, omuzluk, silah) karakterde görünsün | `feat/ekipman-gorunum` | M |
| 1 | G-26 | gameplay-dev | P0 | Zindan çekirdeği: instance WMO + çarpışma, kat durum makinesi, giriş/çıkış, ölümde kat başı | `feat/zindan-cekirdek` | L |
| 1 | G-27 | gameplay-dev | P0 | Westfall Sv 10-20: alt bölgeler, yaratıklar, patronlar, metin alanları (`elwynn.rs` deseni) | `feat/westfall` | M |
| 1 | D-21 | designer | P0 | Metin2 ikonik silah kataloğu `items_metin2.rs` (Hilal, Bambu, Gümüş, Zehir, Dolunay, Ejderha…) | `feat/metin2-silahlar` | S |
| 1→ | Q-08 | qa-engineer | P0 | Westfall + ekipman QA (ekran görüntüleri) | `test/dalga1` | S |
| 2 | G-28 | gameplay-dev | P0 | Örümcek Zindanı (3 kat) | `feat/orumcek-zindani` | M |
| 2 | G-29 | gameplay-dev | P1 | Şeytan Kulesi (4 kat MVP) | `feat/seytan-kulesi` | L |
| 2 | D-22 | designer | P0 | Zindan UI `ui/dungeon_ui.rs`: giriş diyaloğu, kat/hedef izleyici, geri sayım, zafer/başarısızlık bandı | `feat/zindan-ui` | M |
| 2 | D-23 | designer | P1 | Zindan atmosferi `dungeon/atmos.rs`: sis, loş ışık, kat portalı | `feat/zindan-atmosfer` | S |
| 2→ | Q-07 | qa-engineer | P0 | Zindan duman + çarpışma testi (60 sn otopilot, zemin ışını tutarlılığı) | `test/zindan-duman` | S |
| 3 | G-30 | gameplay-dev | P1 | Efsun: `Item.bonus [Bonus; 5]`, bonus havuzu, savaşta etkisi | `feat/efsun` | M |
| 3 | G-31 | gameplay-dev | P1 | Taş takma: `sockets [u32; 3]`, taşlar metinlerden düşer | `feat/tas-takma` | M |
| 3 | D-24 | designer | P1 | Efsun/taş tooltip: Metin2 renkli satırlar | `feat/efsun-ui` | M |
| 3 | G-18/19 + D-04 | gameplay-dev / designer | P1 | Beceri seviyeleri M1→G→P + beceri penceresi (K) | `feat/beceri-seviyeleri`, `feat/beceri-penceresi` | M |
| 3→ | Q-09 | qa-engineer | P0 | Kayıt uyumluluğu: Sprint 4 kaydı açılır, zindanda alınan kayıt girişe döner | `test/kayit-uyum` | S |

**Birleşme sırası:** G-24 + Q-06 → G-25 → G-26 → G-27 → D-21 → (G-28, G-29, D-22, D-23) → G-30 → G-31 → D-24 / G-18 → D-04.

---

## Dosya sahipliği (paralel çalışma için kesin kurallar)

| Dosya | Sahip | Not |
|---|---|---|
| `src/game/mod.rs` | **G-24** | `mod` satırları ve plugin kayıtları G-24'te eklendi; sonraki görevler **dokunmaz** |
| `src/game/zones.rs` | G-24 | Westfall zaten kayıtlı; G-27 bu dosyaya dokunmaz |
| `src/game/items_metin2.rs` | **D-21** | Yeni eşyalar yalnızca buraya eklenir (`items.rs`'e dokunulmaz) |
| `src/game/westfall.rs` | **G-27** | — |
| `src/game/equipment.rs` | **G-25** | — |
| `src/game/player.rs` | **G-25** | Bu sprintte yalnızca G-25 dokunur |
| `src/game/dungeon/mod.rs` | **G-26** | `defs`/`arena`/`entry` alt modüllerini G-26 ekler |
| `src/game/dungeon/spider.rs` | **G-28** | — |
| `src/game/dungeon/tower.rs` | **G-29** | — |
| `src/game/dungeon/atmos.rs` | **D-23** | — |
| `src/game/physics.rs`, `camera.rs`, `save.rs`, `npc.rs` | **G-26** (Dalga 1) | Dalga 1'de başka görev dokunmaz |
| `src/game/ui/**` | designer | gameplay-dev en küçük kancayı ekleyebilir, commit mesajında belirtir |

### İçerik kayıt noktaları (G-24 API'si — sonraki görevler bunlara göre yazar)

- **Bölgeler — `zones.rs`:** `pub struct ZoneDef { name, bounds: [f32; 4], subzones: &[Subzone], spawns: &[SpawnRow],
  metin_slots: &[MetinSlotRow], metin_summons: fn(usize) -> &'static [u32] }`; `pub const ZONES: &[&ZoneDef]`
  (Elwynn + Westfall kayıtlı); `pub fn zone_name(wow: [f32; 3]) -> Option<&'static str>` (HUD mini harita adı);
  `spawns()`, `metin_slots()`, `metin_slot(i)`, `metin_slot_count()`. Westfall yalnızca `westfall.rs`'teki
  `SUBZONES`/`SPAWNS`/`METIN_SLOTS`/`metin_summons`/`ZONE.bounds` değerlerini doldurur.
- **Eşyalar — `items.rs`:** `pub const ITEM_TABLES: &[&[ItemDef]] = &[ITEMS, items_metin2::ITEMS_METIN2]`;
  `all_items()`; `def(vnum)` ve `gear_for_level` bütün tablolara bakar; kurucular `weapon`/`armor`/`wear`/`misc`
  `pub(crate)`. Testler (vnum benzersizliği, MPQ ikon denetimi) bütün tabloları kapsar.
- **Moblar — `mobs.rs`:** `pub const MOB_TABLES: &[&[MobDef]] = &[MOBS, westfall::MOBS, dungeon::spider::MOBS,
  dungeon::tower::MOBS]`; `all_mobs()`, `find(id) -> Option<&MobDef>`, `mob(id)`. Kurucular `m`, `boss`, `human`,
  `with_body(def, model, radius, height)`, `with_loot(def, &[(vnum, şans)])` ve deri/renk sabitleri `pub(crate)`.
  Yeni bir model için `Species::Other` + `with_body` (test denetler). Kimlik önerisi: Westfall 3xx, Örümcek 4xx, Kule 5xx.
- **NPC'ler — `npc.rs`:** `pub const ELWYNN_NPC_TABLES: &[&[NpcDef]] = &[ELWYNN_NPCS, westfall::NPCS, dungeon::NPCS]`;
  `npc_tables()`, `npc_defs()`, `npc_def(row)`. (`NpcKind::DungeonGuide` varyantını G-26 ekler.)
- **Metin taşı — `metin.rs`:** `pub fn spawn_metin(commands: &mut Commands, server: &AssetServer, def: &'static MetinDef,
  at: Vec3, yaw: f32, summons: Option<&'static [u32]>) -> Entity`. Yuvasız taş kırılınca ganimet düşer, yeniden belirmez.
- **İç mekân — `physics.rs`:** `pub static INDOOR: AtomicBool`, `indoor()`, `set_indoor(bool)`. true iken arazi kısa
  yolu ve `ground_at`'taki +60 m yedek ışın kapalı. Şimdilik hiçbir yerde true yapılmaz.

---

## G-24 · Çakışma önleyici refaktör — gameplay-dev

**Kapsam:** Yukarıdaki kayıt noktaları; boş taslaklar `westfall.rs`, `items_metin2.rs`, `equipment.rs`,
`dungeon/{mod,atmos,spider,tower}.rs`; bütün `mod` satırları ve plugin kayıtları `game/mod.rs`'te.

**Kabul kriterleri:**
1. [ ] Davranış değişmez: Elwynn'de aynı moblar, metin taşları, NPC'ler ve mini harita adı (ekran görüntüsü).
2. [ ] `cargo build --release` yeni uyarı vermez; `cargo test --release` temiz, kayıt testleri eklendi.
3. [ ] `hud.rs` yalnızca 1 satır değişti (`zones::zone_name`).

## Q-06 · ItemDisplay denetimi — qa-engineer

**Kapsam:** `display_audit.rs` içinde `#[ignore]` testler: eşya/NPC/mob tablolarındaki her ItemDisplay kimliği
DBC'de var mı, model ve doku dosyaları MPQ'da var mı. `AZM_FIND=<metin>` ile ItemDisplay arama (G-25 ve D-21 model seçer).

**Kabul kriterleri:**
1. [ ] `cargo test --release -- --ignored display_audit` eksik kimlik/dosya listesini yazar.
2. [ ] Rapor `docs/QA_RAPORU.md`'de; bulunan geçersiz kimlikler sahibine atanır.

## G-25 · Ekipman görünümü — gameplay-dev

**Kök nedenler (doğrulandı):**
1. `src/game/ui/inventory_ui.rs:732` `sync_look` yalnızca `Kind::Armor` ve `Kind::Weapon` türlerini işliyor; `Kind::Wear` (kask, kalkan, çizme, bileklik) atlanıyor.
2. `items.rs` içindeki bütün `wear(...)` satırlarında `display: 0`.
3. `PlayerLook.shield` hiç set edilmiyor.
4. Kask ve omuzluk için mekanizma yok: `_HuM` son eki eklenmiyor; `helm_vis: None`, saç kaskın içinden çıkıyor.
5. Geçersiz bir ItemDisplay kimliği hata vermiyor, karakter sessizce çıplak görünüyor (benilla `composite_body`).
6. `sync_look` UI varlıklarının yüklenmesine bağlı.
7. Binekteyken `rebuild_look` gövdeyi bir kare yanlış yere koyuyor.

**Teknik notlar:**
- `src/game/equipment.rs`: `look_from_equip(&Inventory) -> LookSpec` saf fonksiyonu (birim testli). Sıra: zırh seti
  taban → çizme `armor[4]`, bileklik `armor[5]` → kask `helm`, kalkan `shield` (iki elli silahta kalkan yok sayılır)
  → zırhtan omuzluk → silah ve artısı. `sync_look` buraya taşınır ve UI koşulu olmadan çalışır. `attach_items` buraya
  taşınır; takma noktaları: silah 1, kalkan 0, kask 11 (`Head/{stem}_HuM.m2`), sol omuz 6, sağ omuz 5 (`model[1]`).
  Referans: benilla-app `entities/equipment/mod.rs` `ensure_item_model` / `attach_id`.
- `character.rs`: `build_item_model`'e `col` ve `helm_suffix`; `build_body_as`'a `helm` parametresi,
  `helm_vis = d.worn_helm_vis()` ile saç/kulak gizlenir.
- `player.rs`: `PlayerLook`'a `helm` ve `shoulder`; `rebuild_look` yeni gövdeye eski gövdenin `ChildOf` ve
  `Transform` değerlerini devreder (binek titremesi biter).
- `items.rs`: `wear` satırlarına gerçek display kimlikleri (Q-06 ile seçilir); `Kind::Armor`'a `shoulder`
  (Valor = `DungeonWarrior_A`, Might/Wrath/Dreadnaught = `RaidWarrior_A/B/C`, MPQ'da var).
- Test kancası: `AZM_EQUIP=vnum,…` başlangıçta bu eşyaları kuşandırır.

**Kabul kriterleri:**
1. [ ] `look_from_equip` birim testleri (iki el + kalkan, çizme/bileklik üzerine yazma, kask).
2. [ ] `tools/shot.ps1 -Env "AZM_EQUIP=11230,12210,13010,15010,14010;AZM_YAW=3.14;AZM_DIST=4"` önden ve arkadan:
   kask kafada (saç görünmüyor), kalkan kolda, omuzluk görünüyor. Aynı çekim `AZM_RIDE=1` ile de alınır.
3. [ ] Geçersiz display kimliğinde `warn!` satırı.

## G-26 · Zindan çekirdeği (`src/game/dungeon/`) — gameplay-dev

**Teknik notlar:**
- **Çarpışma:** `WmoModel.collision` + `benilla_world::terrain_stream::{placement_collider_data, build_collider_task,
  PendingCollider}` ile Static trimesh. Default katmanı `WorldCollision` gövde ve kamera sorgularında otomatik
  görünür; benilla değişmez.
- **Yerleşim:** ADT'siz okyanus, ör. WoW (-3000, 15500); Pyungmoo odağından (-8000, 14000) uzak; her kat kendi ofsetinde.
- **Modüller:** `mod.rs` (`DungeonRun` + durum makinesi: `Loading` → `Briefing` → `Active(Objective)` → `Cleared` → …
  → `BossFight` → `Victory` / `Failed` → `Exit`), `defs.rs` (`DungeonDef` / `FloorDef` / `Objective { KillAll,
  BreakMetin, FindKey, Boss }`), `arena.rs` (WMO spawn, çarpışma, temizlik, `walkable_points`), `entry.rs`
  (`NpcKind::DungeonGuide` → seviye, bilet, bekleme süresi, ölü/binekli değil kontrolleri → ışınlama; çıkış: portal,
  "Ayrıl", süre aşımı).
- **Ölüm:** süre dolmadıysa kat başında doğulur, aynı kattan devam edilir.
- **Bekleme süresi:** `save.rs` içinde `#[serde(default)] dungeon_cd: Vec<(u32, u64)>` (unix saniye). Zindanda
  kayıt alınırsa dönüş konumu yazılır.
- **Paylaşılan kancalar:** `physics::set_indoor(true/false)`; `camera.rs` içerideyken `cast_camera`; sayaçlar
  `combat::Killed` mesajını dinler; metin hedefi için `metin::spawn_metin`.

**Kabul kriterleri:**
1. [ ] `AZM_DUNGEON=test` ile tek odalı WMO'da yürüme; düşme ve duvardan geçme yok.
2. [ ] Durum makinesi birim testleri (kat geçişi, ölümde kat başı, süre aşımı → çıkış).
3. [ ] Bekleme süresi kayda yazılıyor; eski kayıt `serde(default)` ile açılıyor.

## G-27 · Westfall Sv 10-20 — gameplay-dev

**Kapsam:** `westfall.rs` (`elwynn.rs` deseniyle MCNK areaId / MODF ölçümü). Yaratıklar: Goretusk, Coyote,
Fleshripper, Riverpaw gnoll, murloc, Defias. Patronlar Old Murk-Eye, Vultros. Metin alanları Sv 12-20.
Kayıt noktaları hazır: `ZONE`, `MOBS`, `NPCS` doldurulur; `zones.rs`, `mobs.rs`, `mod.rs`'e dokunulmaz.

**Kabul kriterleri:**
1. [ ] `AZM_FOCUS` ile alt bölge çekimleri; mini haritada Westfall alt bölge adları.
2. [ ] Moblar suda ya da çatıda değil; `zones::registry_tables_are_valid` testi geçiyor.

## D-21 · Metin2 ikonik silah kataloğu — designer

**Kapsam:** `items_metin2.rs`: Hilal, Bambu, Gümüş, Zehir, Dolunay (tek el model), Ejderha… Ad/seviye/istatistik
tr-wiki'den, modeller Q-06 denetiminden geçer. `items.rs`'e dokunulmaz.

**Kabul kriterleri:**
1. [ ] `items::tests` (vnum benzersizliği, MPQ ikon) yeni satırlarla geçiyor.
2. [ ] Q-06 denetiminde yeni display kimlikleri geçerli.

## G-28 · Örümcek Zindanı — gameplay-dev

- Sv 10-25, mekân `MD_Spidermine.wmo`. 3 kat: öldür → metin → boss.
- Moblar MineSpider ve Tarantula (`dungeon/spider.rs` `MOBS`). Boss **Örümcek Kraliçesi** (`MineSpiderBoss.m2`).
- Zehir Kılıcı düşürür.

## G-29 · Şeytan Kulesi — gameplay-dev

- Sv 30-40, 4 kat: öldür / metin / anahtar / boss. Mekânlar `MD_Crypt*` katları, boss katı `Blackrock_lower_guild`.
- Moblar Imp, FelHound, Succubus, VoidWalker, FelGuard, Infernal (`dungeon/tower.rs` `MOBS`). Ara boss DoomGuard,
  son boss **Şeytan Kral** (`DreadLord.m2`).
- Dolunay ve Ejderha Kılıcı düşürür.

**Zindan riskleri ve önlemleri:** mağara açıklıklarından boşluk görünür → sis ve karanlık (D-23); büyük WMO'larda
performans düşer → küçük WMO'lar; yol bulma yok → açık odalar, kısa aggro.

## D-22 · Zindan UI — designer
WoW çerçeveli giriş diyaloğu (şartlar kırmızı/yeşil), kat/hedef izleyici, geri sayım, zafer/başarısızlık bandı (`ui/dungeon_ui.rs`).

## D-23 · Zindan atmosferi — designer
`dungeon/atmos.rs`: sis, loş ışık (çıkışta geri yüklenir), kat portalı (Spells M2).

## Q-07 / Q-08 / Q-09 · Doğrulamalar — qa-engineer
- **Q-07:** `AZM_DUNGEON=spider` / `tower` ile 60 sn `AZM_AUTO=1`. Duvardan çıkış yok, kat geçişi log'da, boss
  düşürmesi envanterde, zemin ışını tutarlı.
- **Q-08:** Westfall alt bölge çekimleri + G-25 ekipman çekimleri (önden/arkadan, binekli).
- **Q-09:** Sprint 4 `save.ron` hatasız yüklenir; zindanda alınan kayıt girişe döner.

## G-30 · Efsun / G-31 · Taş takma / D-24 — Dalga 3
- **G-30:** `Item.bonus [Bonus; 5]` `#[serde(default)]`, bonus havuzu, savaşta kritik/delici, Efsun Nesnesi / Ekleme.
- **G-31:** `sockets [u32; 3]`, taşlar metinlerden düşer.
- **D-24:** efsun/taş tooltip, Metin2 renkli satırlar.
- **G-18/19 + D-04:** bkz. `docs/BACKLOG.md` E7.

---

## Orkestrasyon
- Her dalga için ana oturum elle worktree açar (`../AzerothMetin-<dal>`; `target/` kopyalanır, otomatik isolation
  kullanılmaz). İlgili ajan `.claude/agents/<rol>.md` tanımıyla görevlendirilir.
- Bağımsız işler paralel yürür; aynı dosyaya dokunanlar sırayla.
- Her görev sonunda: derleme temiz, test, ekran görüntüsü. Ardından release-manager dalı iter ve `main`'e ayrı PR
  açar (ne yapıldı, QA, kalan riskler). Birleştirmeyi kullanıcı yapar; release-manager sonraki PR'ı güncel `main`
  üzerine rebase eder.

## Doğrulama
- `cargo build --release`, `cargo test --release`, `cargo test --release -- --ignored display_audit`.
- Ekipman: `tools/shot.ps1 -Env "AZM_EQUIP=11230,12210,13010,15010,14010;AZM_YAW=3.14;AZM_DIST=4"` önden ve arkadan; `AZM_RIDE=1` ile de.
- Zindan: `AZM_DUNGEON=spider` / `tower` ile 60 sn `AZM_AUTO=1`.
- Westfall: `AZM_FOCUS` ile alt bölge çekimleri. Moblar suda veya çatıda değil.
- Kayıt: Sprint 4 `save.ron` dosyası hatasız yüklenir.

## Riskler
- **Kayıt noktaları:** paralel PR'lar aynı tabloya satır eklerse çakışır; her görev yalnızca kendi dosyasındaki tabloyu doldurur.
- **Mob kimlik çakışması:** `mobs::tests::mob_ids_unique_across_tables` yakalar; kimlik aralıkları yukarıda.
- **Zindan çarpışması:** benilla trimesh yolu ilk kez kullanılıyor (G-26); Q-07 ile doğrulanır.

## Sprint kapanışı (PM dolduracak)
- [ ] G-24 · [ ] Q-06 · [ ] G-25 · [ ] G-26 · [ ] G-27 · [ ] D-21 · [ ] Q-08 · [ ] G-28 · [ ] G-29 · [ ] D-22 · [ ] D-23
  · [ ] Q-07 · [ ] G-30 · [ ] G-31 · [ ] D-24 · [ ] G-18/19 · [ ] D-04 · [ ] Q-09
- `PROJE_DURUMU.md` ve hafıza güncellemesi.
