# AzerothMetin Geliştirme Takımı

Vizyon: **Metin2 uyarlaması bir WoW oyunu.** Oynanış, harita ve sistemler Metin2 (Pyungmoo, metin taşı,
envanter, artı basma, NPC'ler, düşürmeler); görsel dil ve oyun içi tasarım World of Warcraft 1.12 gibi
(WoW modelleri, dokuları, UI çerçeveleri, ikonları, efektleri).

## Roller

| Rol | Ajan | Sahip olduğu alanlar |
|---|---|---|
| Ürün Yöneticisi (PM) | `product-manager` | `docs/BACKLOG.md`, `docs/SPRINT_*.md`, önceliklendirme, kabul kriterleri, görev dağıtımı |
| Tasarımcı | `designer` | Görseller: `src/game/ui/**`, `terrain.wgsl`, efektler (`skills.rs` görsel kısmı, parıltılar), model/doku seçimi, köy yerleşimi görünümü |
| Oyun Geliştirici | `gameplay-dev` | Sistemler: `items.rs`, `mobs.rs`, `metin.rs`, `loot.rs`, `npc.rs`, `quest.rs`, `stats.rs`, `combat.rs`, `skills.rs` mantığı, kayıt/yükleme |
| Test / QA | `qa-engineer` | Derleme, birim testleri, `tools/shot.ps1` ile ekran görüntüsü doğrulaması, hata raporu `docs/QA_RAPORU.md` |
| Sürüm Yöneticisi | `release-manager` | GitHub (anltpz/wowm3): sprint dalını iter, PR açar/günceller (ne yapıldı, düzeltmeler, QA) |

Akış (her sprint): PM backlog'dan sprint planı yazar → orkestratör (ana Claude oturumu) görevleri ilgili
ajanlara verir → ajanlar ayrı git worktree'lerde çalışır ve commit eder → orkestratör birleştirir →
QA derler, test eder, ekran görüntüsü alır → Sürüm Yöneticisi `sprint-<n>` dalını GitHub'a iter ve `main`'e PR açar →
PM sprinti kapatır, `PROJE_DURUMU.md`'yi günceller. PR'ı birleştirme kararı kullanıcının.

## Ortak kurallar
- Önce `PROJE_DURUMU.md`'yi oku. Belgeler ve kod yorumları **Türkçe**.
- Rust + Bevy 0.18.1 (eğitim verisi eski olabilir: `Message`/`MessageWriter`, `ChildOf`, `children!`,
  `px()`, `BorderColor::all`, sistem parametre sınırı 16 → tuple ile grupla, edition 2021 → let-chain yok).
- Derleme: `cargo build --release` (ilk derleme ~4 dk). Test: `cargo test --release`.
- Görsel doğrulama: PowerShell'de `& .\tools\shot.ps1 -Out <png> -At "25" -Env "AZM_DIST=14;AZM_AUTO=1"`.
  Kullanışlı env: `AZM_MAPSTART=x,y` (Pyungmoo harita metresi), `AZM_MOBGAL=1`, `AZM_AUTO=1`, `AZM_YAW`, `AZM_PITCH`,
  `AZM_RIDE=1` (10 dk binekte başlar), `AZM_LEVEL=<n>` (test seviyesi), `AZM_MAPDIR=<klasör>` (Blender'dan dışa aktarılan harita, `docs/BLENDER_HARITA.md`).
- Metin + mob senaryosu (QA H7): `-At "15" -Env "AZM_MAPSTART=725,660;AZM_AUTO=1;AZM_YAW=-1.57;AZM_DIST=30"`.
  Otopilot Sv6, oradaki moblar Sv10–15: 20 sn'den uzun çekimde oyuncu ölür. Köy meydanının 60 m çevresinde mob yok,
  otopilot orada hedef bulamaz.
- WoW asset yolları MPQ'da gerçekten var mı `tools/mpq_present.txt` ile kontrol et (listfile'daki ~1850 dosya yok). Yükleme: `mpq://Yol/Dosya.m2`.
- Metin2 isimleri/seviyeleri için kaynak: tr-wiki.metin2.gameforge.com (Türkçe adlar birebir).
- Telifli istemci dosyaları (Metin2 pack) indirilmez/kopyalanmaz; wiki görsel ve bilgileri referanstır.
- Kendi alanının dışındaki dosyaya dokunman gerekirse küçük tut ve commit mesajında belirt.
- Her görev sonunda: derleme temiz + kısa rapor (ne yapıldı, nasıl test edildi, kalan riskler).
