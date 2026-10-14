//! Q-06: ItemDisplayInfo denetimi ve bulucu (yalnızca test derlemesinde).
//!
//! Neden: benilla `CharSections::composite_body` eksik bölge dokularını sessizce atlar,
//! `ItemDisplayCatalog::get(id)` `None` dönerse yuva boş kalır, `build_item_model` de model yoksa
//! sessizce hiçbir şey takmaz. Yani yanlış kimlik hata vermez, eşya karakterde görünmez.
//!
//! Testler `#[ignore]`: WoW verisi gerekir (`WOW_DATA` > `C:\Users\ANIL\Desktop\AzerothSurvivors\Data`).
//! - Denetim: `cargo test --release -- --ignored display_audit --nocapture --test-threads=1`
//!   Kodda kullanılan tüm kimlikleri (items.rs, PlayerLook, npc.rs, mobs.rs) kontrol eder, tablo
//!   basar, geçersizleri sonda listeler ve varsa testi düşürür.
//! - Bulucu: `AZM_FIND=Helm_Plate_RaidWarrior cargo test --release -- --ignored display_find --nocapture`
//!   Model / doku / bölge dokusu / ikon adında alt dize (büyük-küçük harf duyarsız) arar.
//!   `AZM_FIND_OK=1` yalnızca dosyaları MPQ'da tam olan satırları gösterir. Rakam/virgül verilirse
//!   kimlik listesi olarak okunur: `AZM_FIND=4908,2416`.
//!
//! Dosya varlığı zincirin karma tablosundan (`Chain::contains`) okunur: listfile'da olup MPQ'da
//! olmayan dosyalar burada doğru biçimde "YOK" çıkar. `tools/mpq_present.txt` ile çapraz kontrol
//! de yapılır (uyuşmazlık sayısı özet satırında).

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use benilla_formats::{equip_region_candidates, Chain, ItemDisplay, ItemDisplayCatalog};

use super::character::ArmorSet;
use super::items::{EquipSlot, Kind, ITEMS};

/// Gövde yuvalarının adları (`ArmorSet` sırası).
const BODY_SLOTS: [&str; 8] = ["Gömlek", "Göğüs", "Kemer", "Pantolon", "Çizme", "Bileklik", "Eldiven", "Tabard"];

/// Bölge kısaltmaları (`region_textures` sırası: ArmUpper … Foot).
const REGION_ABBR: [&str; 8] = ["AU", "AL", "HA", "TU", "TL", "LU", "LL", "FO"];

/// Her gövde yuvasının bileşik dokuda çizildiği katmanlar (benilla `EQUIP_LAYER_COLUMN`, -1 olmayanlar).
const SLOT_LAYERS: [&[usize]; 8] = [
    &[0, 1, 3, 4],       // gömlek
    &[0, 1, 3, 4, 5, 6], // göğüs (cübbe bacağa iner)
    &[5],                // kemer
    &[5, 6],             // pantolon
    &[6, 7],             // çizme
    &[1],                // bileklik
    &[1, 2],             // eldiven
    &[3, 4],             // tabard
];

/// Nesne bileşeni klasörleri (`Item\ObjectComponents\<dir>`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Obj {
    Weapon,
    Shield,
    Head,
    Shoulder,
}

impl Obj {
    const ALL: [Obj; 4] = [Obj::Weapon, Obj::Shield, Obj::Head, Obj::Shoulder];
    fn dir(self) -> &'static str {
        match self {
            Obj::Weapon => "Weapon",
            Obj::Shield => "Shield",
            Obj::Head => "Head",
            Obj::Shoulder => "Shoulder",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Obj::Weapon => "Silah",
            Obj::Shield => "Kalkan",
            Obj::Head => "Kask",
            Obj::Shoulder => "Omuzluk",
        }
    }
    /// Model dosyasının MPQ yolu. Kask: `<stem>_HuM.m2` / `_HuF.m2` (ırk + cinsiyet eki).
    fn model_file(self, name: &str, female: bool) -> String {
        let stem = name.strip_suffix(".m2").unwrap_or(name);
        match self {
            Obj::Head => format!(
                "Item\\ObjectComponents\\Head\\{stem}_Hu{}.m2",
                if female { 'F' } else { 'M' }
            ),
            _ => format!("Item\\ObjectComponents\\{}\\{stem}.m2", self.dir()),
        }
    }
    fn texture_file(self, tex: &str) -> String {
        format!("Item\\ObjectComponents\\{}\\{tex}.blp", self.dir())
    }
}

/// Denetlenen şey: nesne modeli ya da gövde yuvası.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Check {
    Obj(Obj),
    Body(usize),
}

impl Check {
    fn label(self) -> &'static str {
        match self {
            Check::Obj(o) => o.label(),
            Check::Body(s) => BODY_SLOTS[s],
        }
    }
}

/// MPQ zinciri + katalog + `mpq_present.txt` (çapraz kontrol için).
struct Db {
    chain: Chain,
    cat: ItemDisplayCatalog,
    present: HashSet<String>,
    /// Zincir ile present listesinin uyuşmadığı yollar.
    mismatch: std::cell::RefCell<Vec<String>>,
}

impl Db {
    fn open() -> Self {
        let dir = std::env::var_os("WOW_DATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Users\ANIL\Desktop\AzerothSurvivors\Data"));
        assert!(dir.is_dir(), "WoW veri klasörü yok: {} (WOW_DATA ile ver)", dir.display());
        let mut chain = Chain::open(&dir).unwrap_or_else(|e| panic!("MPQ zinciri açılamadı: {e:#}"));
        let cat = benilla_formats::load_item_display_catalog(&mut chain)
            .unwrap_or_else(|e| panic!("ItemDisplayInfo.dbc okunamadı: {e:#}"));
        let present_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tools/mpq_present.txt");
        let present = std::fs::read_to_string(&present_path)
            .map(|s| s.lines().map(norm).collect::<HashSet<String>>())
            .unwrap_or_default();
        println!(
            "ItemDisplayInfo: {} satır; veri: {}; mpq_present.txt: {} yol",
            cat.len(),
            dir.display(),
            present.len()
        );
        Self { chain, cat, present, mismatch: Default::default() }
    }

    /// Dosya MPQ'da mı (zincir karma tablosu esas). Present listesiyle uyuşmazsa kaydeder.
    fn has(&self, path: &str) -> bool {
        let in_chain = self.chain.contains(path);
        if !self.present.is_empty() && in_chain != self.present.contains(&norm(path)) {
            self.mismatch.borrow_mut().push(format!("{path} (zincir: {in_chain})"));
        }
        in_chain
    }

    /// Model adının hangi nesne klasöründe bulunduğu (önce `prefer`).
    fn locate(&self, name: &str, prefer: Option<Obj>) -> Option<(Obj, String)> {
        let order = prefer.into_iter().chain(Obj::ALL.into_iter().filter(|o| Some(*o) != prefer));
        for o in order {
            let p = o.model_file(name, false);
            if self.has(&p) {
                return Some((o, p));
            }
        }
        None
    }

    /// Bölge dokusunun durumu: `Some((erkek_var, kadın_var))`, ad boşsa `None`.
    fn region_ok(&self, layer: usize, name: &str) -> (bool, bool) {
        let m = equip_region_candidates(layer, name, 0).iter().any(|p| self.has(p));
        let f = equip_region_candidates(layer, name, 1).iter().any(|p| self.has(p));
        (m, f)
    }
}

/// Yol karşılaştırması: küçük harf + ters bölü.
fn norm(p: &str) -> String {
    p.trim().replace('/', "\\").to_ascii_lowercase()
}

fn mark(ok: bool) -> &'static str {
    if ok {
        "+"
    } else {
        "YOK"
    }
}

/// Bir kimliği denetler: (tablo özeti, hatalar, uyarılar).
fn check(db: &Db, what: Check, id: u32) -> (String, Vec<String>, Vec<String>) {
    let mut errs = Vec::new();
    let mut warns = Vec::new();
    let Some(d) = db.cat.get(id) else {
        return ("DBC'de satır yok".into(), vec!["ItemDisplayInfo'da satır yok".into()], warns);
    };
    let summary = match what {
        Check::Obj(o) => check_obj(db, o, d, &mut errs, &mut warns),
        Check::Body(slot) => check_body(db, slot, d, &mut errs, &mut warns),
    };
    (summary, errs, warns)
}

fn check_obj(db: &Db, o: Obj, d: &ItemDisplay, errs: &mut Vec<String>, warns: &mut Vec<String>) -> String {
    // Omuzlukta iki model (sol/sağ), diğerlerinde yalnızca sol (`model[0]`) okunur.
    let cols: &[usize] = if o == Obj::Shoulder { &[0, 1] } else { &[0] };
    let mut parts = Vec::new();
    for &c in cols {
        let Some(name) = d.model[c].as_deref() else {
            errs.push(format!("model[{c}] boş"));
            parts.push(format!("model[{c}]=-"));
            continue;
        };
        let file = o.model_file(name, false);
        let ok = db.has(&file);
        if !ok {
            match db.locate(name, None) {
                Some((other, _)) => errs.push(format!("{} yok (model {} klasöründe)", file, other.dir())),
                None => errs.push(format!("{file} MPQ'da yok")),
            }
        }
        if o == Obj::Head && !db.has(&o.model_file(name, true)) {
            warns.push(format!("{} yok (kadın NPC'de kask görünmez)", o.model_file(name, true)));
        }
        parts.push(format!("{} [{}]", file.rsplit('\\').next().unwrap_or(&file), mark(ok)));
        match d.model_texture[c].as_deref() {
            Some(tex) => {
                let tf = o.texture_file(tex);
                let tok = db.has(&tf);
                if !tok {
                    errs.push(format!("{tf} MPQ'da yok"));
                }
                parts.push(format!("doku {tex} [{}]", mark(tok)));
            }
            None => {
                errs.push(format!("model_texture[{c}] boş (model dokusuz çizilir)"));
                parts.push("doku -".into());
            }
        }
    }
    parts.join("  ")
}

fn check_body(db: &Db, slot: usize, d: &ItemDisplay, errs: &mut Vec<String>, warns: &mut Vec<String>) -> String {
    let mut parts = Vec::new();
    let mut drawn = 0;
    for (layer, tex) in d.region_textures.iter().enumerate() {
        let Some(name) = tex.as_deref() else { continue };
        let (m, f) = db.region_ok(layer, name);
        let in_slot = SLOT_LAYERS[slot].contains(&layer);
        if in_slot && m {
            drawn += 1;
        }
        if !m {
            errs.push(format!(
                "Item\\TextureComponents\\{}\\{name}_U/_M.blp yok",
                benilla_formats::equip_tex_dir(layer).unwrap_or("?")
            ));
        } else if !f {
            warns.push(format!("{} {name}: kadın (_F) dokusu yok", REGION_ABBR[layer]));
        }
        if !in_slot {
            warns.push(format!("{} {name}: bu yuvada çizilmeyen katman", REGION_ABBR[layer]));
        }
        parts.push(format!("{}:{name} [{}]", REGION_ABBR[layer], mark(m)));
    }
    if drawn == 0 {
        errs.push(format!(
            "yuvanın katmanlarında ({}) çizilebilir doku yok — karakterde görünmez",
            SLOT_LAYERS[slot].iter().map(|l| REGION_ABBR[*l]).collect::<Vec<_>>().join(",")
        ));
    }
    if parts.is_empty() {
        "bölge dokusu yok".into()
    } else {
        parts.join(" ")
    }
}

/// Kodda kullanılan tüm (denetim türü, kimlik) → kullanan yerler.
fn collect_uses() -> (BTreeMap<(Check, u32), Vec<String>>, Vec<String>) {
    let mut uses: BTreeMap<(Check, u32), Vec<String>> = BTreeMap::new();
    let mut no_display = Vec::new();
    let mut add = |what: Check, id: u32, who: String| uses.entry((what, id)).or_default().push(who);
    fn add_set(add: &mut impl FnMut(Check, u32, String), set: &ArmorSet, who: &str) {
        for (slot, id) in set.iter().enumerate() {
            if let Some(id) = id {
                add(Check::Body(slot), *id, who.to_string());
            }
        }
    }

    for it in ITEMS {
        let who = format!("items:{}", it.name);
        match it.kind {
            Kind::Weapon { display, .. } => add(Check::Obj(Obj::Weapon), display, who),
            Kind::Armor { set, .. } => add_set(&mut add, &set, &who),
            Kind::Wear { slot, display, .. } => {
                let what = match slot {
                    EquipSlot::Helmet => Some(Check::Obj(Obj::Head)),
                    EquipSlot::Shield => Some(Check::Obj(Obj::Shield)),
                    EquipSlot::Shoes => Some(Check::Body(4)),
                    EquipSlot::Bracelet => Some(Check::Body(5)),
                    // Kolye / yüzük WoW'da da görünmez.
                    _ => None,
                };
                match (what, display) {
                    (Some(_), 0) => no_display.push(format!("{} ({}) — display 0", it.name, slot.name())),
                    (Some(w), id) => add(w, id, who),
                    _ => {}
                }
            }
            _ => {}
        }
    }

    let look = super::player::PlayerLook::default();
    add_set(&mut add, &look.armor, "PlayerLook");
    if let Some(w) = look.weapon {
        add(Check::Obj(Obj::Weapon), w, "PlayerLook".into());
    }
    if let Some(s) = look.shield {
        add(Check::Obj(Obj::Shield), s, "PlayerLook".into());
    }

    for n in super::npc::NPCS.iter().chain(super::npc::ELWYNN_NPCS) {
        let who = format!("npc:{}", n.name);
        add_set(&mut add, &n.armor, &who);
        if let Some(t) = n.tool {
            add(Check::Obj(Obj::Weapon), t, who);
        }
    }
    for m in super::mobs::MOBS {
        if let Some((set, weapon)) = &m.gear {
            let who = format!("mob:{}", m.name);
            add_set(&mut add, set, &who);
            add(Check::Obj(Obj::Weapon), *weapon, who);
        }
    }
    (uses, no_display)
}

/// Kullanan listesini kısaltır: kaynak türüne göre grupla, en çok 3 ad.
fn short_users(who: &[String]) -> String {
    let mut uniq: Vec<&String> = Vec::new();
    for w in who {
        if !uniq.contains(&w) {
            uniq.push(w);
        }
    }
    let shown: Vec<&str> = uniq.iter().take(3).map(|s| s.as_str()).collect();
    if uniq.len() > 3 {
        format!("{} (+{})", shown.join(", "), uniq.len() - 3)
    } else {
        shown.join(", ")
    }
}

#[test]
#[ignore = "WoW verisi gerekir: cargo test --release -- --ignored display_audit --nocapture"]
fn display_audit() {
    let db = Db::open();
    let (uses, no_display) = collect_uses();
    let mut bad: Vec<String> = Vec::new();
    let mut notes: Vec<String> = Vec::new();

    println!("\n=== ItemDisplay denetimi: {} benzersiz (tür, kimlik) ===", uses.len());
    println!("{:<9} {:>6}  {:<4} özet  <- kullananlar", "tür", "kimlik", "durum");
    for ((what, id), who) in &uses {
        let (summary, errs, warns) = check(&db, *what, *id);
        let state = if errs.is_empty() { "OK" } else { "HATA" };
        println!("{:<9} {:>6}  {:<4} {summary}  <- {}", what.label(), id, state, short_users(who));
        for e in &errs {
            bad.push(format!("{} {id}: {e}  <- {}", what.label(), short_users(who)));
        }
        for w in &warns {
            notes.push(format!("{} {id}: {w}", what.label()));
        }
    }

    if !no_display.is_empty() {
        println!("\n--- Görünümü olmayan giyilebilirler (display 0, karakterde hiç görünmez) ---");
        for n in &no_display {
            println!("  {n}");
        }
    }
    if !notes.is_empty() {
        println!("\n--- Uyarılar ({}) ---", notes.len());
        for n in &notes {
            println!("  {n}");
        }
    }
    let mism = db.mismatch.borrow();
    println!(
        "\nmpq_present.txt ile zincir uyuşmazlığı: {} yol{}",
        mism.len(),
        if mism.is_empty() { String::new() } else { format!(" (ilk: {})", mism[0]) }
    );
    if bad.is_empty() {
        println!("\nGeçersiz kimlik yok.");
    } else {
        println!("\n=== GEÇERSİZ ({}) ===", bad.len());
        for b in &bad {
            println!("  {b}");
        }
        panic!("{} geçersiz ItemDisplay kullanımı (yukarıdaki listeye bak)", bad.len());
    }
}

#[test]
#[ignore = "WoW verisi gerekir: AZM_FIND=Shield_ cargo test --release -- --ignored display_find --nocapture"]
fn display_find() {
    let Ok(needle) = std::env::var("AZM_FIND") else {
        println!("display_find: AZM_FIND=<alt dize> verilmedi, atlanıyor");
        return;
    };
    let needle = needle.to_ascii_lowercase();
    let only_ok = std::env::var("AZM_FIND_OK").is_ok_and(|v| v == "1");
    let db = Db::open();

    // Yalnızca rakam ve virgülse kimlik listesi (`AZM_FIND=4908,2416`), değilse alt dize araması.
    let id_list: Option<Vec<u32>> = needle
        .contains(|c: char| c.is_ascii_digit())
        .then(|| needle.split(',').map(|p| p.trim().parse().ok()).collect::<Option<Vec<u32>>>())
        .flatten();
    let mut ids: Vec<u32> = match id_list {
        Some(list) => {
            for id in list.iter().filter(|id| db.cat.get(**id).is_none()) {
                println!("{id:>6} DBC'de satır yok");
            }
            list.into_iter().filter(|id| db.cat.get(*id).is_some()).collect()
        }
        None => {
            let mut v: Vec<u32> = db
                .cat
                .iter()
                .filter(|(_, d)| {
                    d.model.iter().chain(&d.model_texture).chain(&d.region_textures).chain(std::iter::once(&d.icon))
                        .flatten()
                        .any(|s| s.to_ascii_lowercase().contains(&needle))
                })
                .map(|(id, _)| id)
                .collect();
            v.sort_unstable();
            v
        }
    };
    ids.dedup();

    println!("\n=== AZM_FIND={needle}: {} eşleşme{} ===", ids.len(), if only_ok { " (yalnız tam olanlar)" } else { "" });
    let mut shown = 0;
    for id in ids {
        let d = db.cat.get(id).expect("kimlik katalogdan geldi");
        let mut ok = true;
        let mut parts = Vec::new();
        for c in 0..2 {
            let Some(name) = d.model[c].as_deref() else { continue };
            match db.locate(name, None) {
                Some((o, file)) => {
                    let mut s = format!("{}/{}", o.dir(), file.rsplit('\\').next().unwrap_or(&file));
                    if o == Obj::Head && !db.has(&o.model_file(name, true)) {
                        s += " (HuF yok)";
                    }
                    if let Some(tex) = d.model_texture[c].as_deref() {
                        let t = db.has(&o.texture_file(tex));
                        ok &= t;
                        s += &format!(" doku {tex} [{}]", mark(t));
                    }
                    parts.push(s);
                }
                None => {
                    ok = false;
                    parts.push(format!("{name} [YOK]"));
                }
            }
        }
        for (layer, tex) in d.region_textures.iter().enumerate() {
            let Some(name) = tex.as_deref() else { continue };
            let (m, f) = db.region_ok(layer, name);
            ok &= m;
            let fx = if m && !f { " (F yok)" } else { "" };
            parts.push(format!("{}:{name} [{}]{fx}", REGION_ABBR[layer], mark(m)));
        }
        if parts.is_empty() {
            // Ne model ne bölge dokusu: görünmez satır.
            ok = false;
            parts.push("model/doku yok".into());
        }
        if only_ok && !ok {
            continue;
        }
        shown += 1;
        if shown > 600 {
            println!("... (600 satırda kesildi, aramayı daralt)");
            break;
        }
        let icon = d.icon.as_deref().map(|i| i.rsplit('\\').next().unwrap_or(i)).unwrap_or("-");
        println!("{id:>6} {} {}  ikon {icon}", if ok { "OK  " } else { "EKSİK" }, parts.join("  "));
    }
}
