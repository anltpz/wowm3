//! Kayıt / yükleme: oyuncu ilerlemesi (seviye, EXP, HP/SP, envanter, ekipman, Yang, görev, konum) `save.ron`.
//!
//! - Konum: `AZM_SAVE=<yol>` > `%APPDATA%\AzerothMetin\save.ron` > exe yanında `save\save.ron`.
//! - `AZM_NOSAVE=1` ya da `AZM_SHOT` tanımlıysa kayıt ne okunur ne yazılır (`AZM_SAVE` verilmişse yine çalışır).
//! - Yükleme `PreStartup`'ta kaynakları yerleştirir; `inventory_ui::starter_kit` dolu ekipmanı görüp eşya vermez.
//! - Yazma: 30 sn'de bir, seviye atlayınca, F5 ve kapanışta; atomik (`save.ron.tmp` + `rename`).
//! - Bozuk dosya `save.ron.bak` olarak saklanır, yeni oyunla devam edilir.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use bevy::prelude::*;
use bevy::window::WindowCloseRequested;
use serde::{Deserialize, Serialize};

use super::inventory::{Cell, Inventory, CELLS};
use super::log::{GameLog, C_GOOD};
use super::player::Player;
use super::quest::Quest;
use super::stats::PlayerStats;

/// Kayıt biçimi sürümü. Yeni alanlar `#[serde(default)]` ile eklenir.
pub const VERSION: u32 = 1;

/// Otomatik kayıt aralığı (sn).
const AUTOSAVE_SECS: f32 = 30.0;

#[derive(Clone, Serialize, Deserialize)]
pub struct SaveData {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub stats: PlayerStats,
    #[serde(default)]
    pub inventory: Inventory,
    #[serde(default)]
    pub quest: Quest,
    /// Oyuncunun harita metresi konumu (Pyungmoo; Elwynn modunda `None`).
    #[serde(default)]
    pub pos_map: Option<[f32; 2]>,
    /// Oyuncunun WoW dünya konumu (Elwynn / orijinal harita).
    #[serde(default)]
    pub pos_wow: Option<[f32; 3]>,
}

impl SaveData {
    /// Oyun durumundan kayıt (ölüyse HP en az 1).
    pub fn snapshot(stats: &PlayerStats, inv: &Inventory, quest: &Quest, pos_map: Option<[f32; 2]>) -> Self {
        let mut stats = stats.clone();
        stats.hp = stats.hp.max(1);
        stats.sp = stats.sp.max(0);
        Self {
            version: VERSION,
            stats,
            inventory: inv.clone(),
            quest: quest.clone(),
            pos_map,
            pos_wow: None,
        }
    }

    pub fn to_ron(&self) -> Result<String, String> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::new().depth_limit(4).compact_arrays(true)).map_err(|e| e.to_string())
    }

    /// RON metnini çözer ve doğrular (envanter ızgarası tutarsızsa yeniden kurulur).
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut d: SaveData = ron::from_str(text).map_err(|e| e.to_string())?;
        d.inventory = sanitize_inventory(d.inventory);
        d.stats.hp = d.stats.hp.max(1);
        d.stats.dead = false;
        d.stats.level = d.stats.level.clamp(1, super::stats::MAX_LEVEL);
        if d.quest.stage == 0 {
            d.quest.stage = 1;
        }
        if let Some([x, y]) = d.pos_map {
            if !(x.is_finite() && y.is_finite()) {
                d.pos_map = None;
            }
        }
        Ok(d)
    }
}

/// Izgara tutarlı mı: uzunluk doğru, her `Tail` bir `Head`'in kapladığı hücreyi gösteriyor, her `Head`'in
/// alt hücreleri kendisini gösteriyor.
fn inventory_consistent(inv: &Inventory) -> bool {
    if inv.cells.len() != CELLS {
        return false;
    }
    for (i, c) in inv.cells.iter().enumerate() {
        match *c {
            Cell::Empty => {}
            Cell::Head(it) => {
                let (page, col, row) = super::inventory::coords(i);
                let h = it.def().height as usize;
                if row + h > super::inventory::ROWS {
                    return false;
                }
                for k in 1..h {
                    if inv.cells[super::inventory::index(page, col, row + k)] != Cell::Tail(i as u16) {
                        return false;
                    }
                }
            }
            Cell::Tail(h) => {
                if !matches!(inv.cells.get(h as usize), Some(Cell::Head(_))) {
                    return false;
                }
            }
        }
    }
    true
}

/// Bozuk ızgarayı eşyaları kaybetmeden yeniden kurar (yerine sığmayanı ilk boş yere koyar).
pub fn sanitize_inventory(mut inv: Inventory) -> Inventory {
    if !inventory_consistent(&inv) {
        warn!("save: envanter ızgarası tutarsız, yeniden kuruluyor");
        let heads: Vec<(usize, super::items::Item)> = inv
            .cells
            .iter()
            .enumerate()
            .filter_map(|(i, c)| match c {
                Cell::Head(it) => Some((i, *it)),
                _ => None,
            })
            .collect();
        let mut fresh = Inventory {
            equip: inv.equip,
            yang: inv.yang,
            ..Default::default()
        };
        let mut rest = Vec::new();
        for (i, it) in heads {
            if !fresh.place_at(i, it) {
                rest.push(it);
            }
        }
        for it in rest {
            if fresh.add(it).is_some() {
                warn!("save: envanterde yer yok, eşya düştü: {}", it.vnum);
            }
        }
        inv = fresh;
    }
    inv.touch();
    inv
}

/// Kayıt yolunu ortamdan çözer (saf fonksiyon, test edilebilir). `None` = kayıt kapalı.
pub fn resolve_path(
    save_env: Option<&str>,
    nosave: bool,
    shot: bool,
    appdata: Option<&str>,
    exe_dir: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(p) = save_env.map(str::trim).filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(p));
    }
    if nosave || shot {
        return None;
    }
    if let Some(a) = appdata.map(str::trim).filter(|a| !a.is_empty()) {
        return Some(PathBuf::from(a).join("AzerothMetin").join("save.ron"));
    }
    exe_dir.map(|d| d.join("save").join("save.ron"))
}

/// Ortam değişkenlerinden kayıt yolu (bir kez hesaplanır).
pub fn save_path() -> Option<&'static Path> {
    static P: OnceLock<Option<PathBuf>> = OnceLock::new();
    P.get_or_init(|| {
        let env = |k: &str| std::env::var(k).ok();
        let nosave = env("AZM_NOSAVE").is_some_and(|v| !matches!(v.trim(), "" | "0"));
        let exe_dir = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf));
        resolve_path(
            env("AZM_SAVE").as_deref(),
            nosave,
            std::env::var_os("AZM_SHOT").is_some(),
            env("APPDATA").as_deref(),
            exe_dir.as_deref(),
        )
    })
    .as_deref()
}

/// `foo.ron` → `foo.ron.<ext>`.
fn sibling(path: &Path, ext: &str) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".");
    s.push(ext);
    PathBuf::from(s)
}

/// Dosyadan okur. Yoksa `None`; bozuksa `.bak` olarak saklar ve `None`.
pub fn load_file(path: &Path) -> Option<SaveData> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            warn!("save: {} okunamadı: {e}", path.display());
            return None;
        }
    };
    match SaveData::parse(&text) {
        Ok(d) => Some(d),
        Err(e) => {
            let bak = sibling(path, "bak");
            warn!("save: {} bozuk ({e}); {} olarak saklandı, yeni oyun", path.display(), bak.display());
            let _ = std::fs::remove_file(&bak);
            if let Err(e) = std::fs::rename(path, &bak) {
                warn!("save: .bak yapılamadı: {e}");
            }
            None
        }
    }
}

/// Atomik yazma: önce `.tmp`, sonra yerine taşı.
pub fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = sibling(path, "tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

/// Açılışta okunan kayıt (bir kez, tembel). `start_wow` konumu buradan alır.
pub fn loaded() -> Option<&'static SaveData> {
    static L: OnceLock<Option<SaveData>> = OnceLock::new();
    L.get_or_init(|| save_path().and_then(load_file)).as_ref()
}

/// Kayıttaki harita konumu (varsa).
pub fn saved_pos() -> Option<[f32; 2]> {
    loaded().and_then(|d| d.pos_map)
}

/// Kayıttaki WoW konumu (Elwynn; varsa ve sayıysa).
pub fn saved_wow() -> Option<[f32; 3]> {
    loaded().and_then(|d| d.pos_wow).filter(|p| p.iter().all(|v| v.is_finite()))
}

/// Yazma durumu.
#[derive(Resource, Default)]
struct SaveState {
    since: f32,
    last_level: Option<u32>,
}

pub fn plugin(app: &mut App) {
    if save_path().is_none() {
        info!("save: kayıt kapalı (AZM_SHOT / AZM_NOSAVE)");
        return;
    }
    app.init_resource::<SaveState>()
        .add_systems(PreStartup, load_into_world)
        .add_systems(Update, autosave)
        .add_systems(Last, save_on_exit);
}

fn load_into_world(mut commands: Commands) {
    let path = save_path().map(|p| p.display().to_string()).unwrap_or_default();
    match loaded() {
        Some(d) => {
            info!("save: yüklendi Sv{} {} Yang ({path})", d.stats.level, d.inventory.yang);
            commands.insert_resource(d.stats.clone());
            let mut inv = d.inventory.clone();
            inv.touch();
            commands.insert_resource(inv);
            commands.insert_resource(d.quest.clone());
        }
        None => info!("save: yeni oyun ({path})"),
    }
}

/// Oyuncunun konumu: (Pyungmoo harita metresi, WoW dünya koordinatı) — hangi harita açıksa.
fn player_pos(player: &Query<&Transform, With<Player>>) -> (Option<[f32; 2]>, Option<[f32; 3]>) {
    let Ok(xf) = player.single() else { return (None, None) };
    match super::terrain::data() {
        Some(d) => {
            let m = d.to_map(xf.translation);
            (Some([m.x, m.y]), None)
        }
        None => (None, Some(benilla_assets::coords::bevy_to_wow(xf.translation))),
    }
}

fn write_now(stats: &PlayerStats, inv: &Inventory, quest: &Quest, pos: (Option<[f32; 2]>, Option<[f32; 3]>)) -> bool {
    let Some(path) = save_path() else { return false };
    let mut data = SaveData::snapshot(stats, inv, quest, pos.0);
    data.pos_wow = pos.1;
    let res = data.to_ron().map_err(|e| e.to_string()).and_then(|t| write_atomic(path, &t).map_err(|e| e.to_string()));
    match res {
        Ok(()) => {
            info!("save: yazıldı Sv{} {} Yang", stats.level, inv.yang);
            true
        }
        Err(e) => {
            warn!("save: yazılamadı ({}): {e}", path.display());
            false
        }
    }
}

fn autosave(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut st: ResMut<SaveState>,
    (stats, inv, quest): (Res<PlayerStats>, Res<Inventory>, Res<Quest>),
    player: Query<&Transform, With<Player>>,
    mut log: ResMut<GameLog>,
) {
    st.since += time.delta_secs();
    let level_up = st.last_level.is_some_and(|l| stats.level > l);
    st.last_level = Some(stats.level);
    let f5 = keys.just_pressed(KeyCode::F5);
    // Oyuncu henüz doğmadıysa (ilk kareler) konum yok: otomatik kaydı ertele.
    let pos = player_pos(&player);
    if player.single().is_err() && !f5 {
        return;
    }
    if f5 || level_up || st.since >= AUTOSAVE_SECS {
        st.since = 0.0;
        let ok = write_now(&stats, &inv, &quest, pos);
        if f5 {
            if ok {
                log.push(time.elapsed_secs(), C_GOOD, "Oyun kaydedildi.");
            } else {
                log.push(time.elapsed_secs(), super::log::C_BAD, "Oyun kaydedilemedi!");
            }
        }
    }
}

fn save_on_exit(
    mut exit: MessageReader<AppExit>,
    mut close: MessageReader<WindowCloseRequested>,
    (stats, inv, quest): (Res<PlayerStats>, Res<Inventory>, Res<Quest>),
    player: Query<&Transform, With<Player>>,
    mut done: Local<bool>,
) {
    let any = exit.read().count() + close.read().count() > 0;
    if !any || *done {
        return;
    }
    *done = true;
    write_now(&stats, &inv, &quest, player_pos(&player));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::items::{EquipSlot, Item};

    fn sample() -> SaveData {
        let mut inv = Inventory::default();
        let mut sword = Item::new(10);
        sword.plus = 7;
        inv.equip[EquipSlot::Weapon.index()] = Some(sword);
        inv.equip[EquipSlot::Armor.index()] = Some(Item::new(11200));
        inv.add(Item::stack(27001, 137));
        let mut bow = Item::new(20);
        bow.plus = 3;
        assert!(inv.place_at(super::super::inventory::index(1, 2, 4), bow));
        inv.yang = 1_234_567;
        let stats = PlayerStats { level: 17, exp: 4321, hp: 900, sp: 250, ..Default::default() };
        let quest = Quest { stage: 4, kills: 2 };
        SaveData::snapshot(&stats, &inv, &quest, Some([401.5, 662.25]))
    }

    #[test]
    fn ron_roundtrip() {
        let a = sample();
        let text = a.to_ron().unwrap();
        let b = SaveData::parse(&text).unwrap();
        assert_eq!(b.version, VERSION);
        assert_eq!(b.stats.level, 17);
        assert_eq!(b.stats.exp, 4321);
        assert_eq!((b.stats.hp, b.stats.sp), (900, 250));
        assert_eq!(b.inventory.cells, a.inventory.cells);
        assert_eq!(b.inventory.equip, a.inventory.equip);
        assert_eq!(b.inventory.equip[EquipSlot::Weapon.index()].unwrap().plus, 7);
        assert_eq!(b.inventory.count(27001), 137);
        assert_eq!(b.inventory.yang, 1_234_567);
        assert_eq!((b.quest.stage, b.quest.kills), (4, 2));
        assert_eq!(b.pos_map, Some([401.5, 662.25]));
        // Saf olmayan alanlar kaydedilmez.
        assert!(!text.contains("last_hurt") && !text.contains("rev"));
    }

    #[test]
    fn old_file_with_missing_fields_loads() {
        let text = "(version: 1, stats: (level: 7, exp: 100), inventory: (yang: 500))";
        let d = SaveData::parse(text).unwrap();
        assert_eq!(d.stats.level, 7);
        assert_eq!(d.stats.exp, 100);
        assert_eq!(d.stats.name, PlayerStats::default().name);
        assert_eq!(d.inventory.yang, 500);
        assert_eq!(d.inventory.cells.len(), CELLS);
        assert_eq!(d.quest.stage, 1);
        assert_eq!(d.pos_map, None);
        // Tamamen boş yapı da yüklenir.
        assert!(SaveData::parse("()").is_ok());
    }

    #[test]
    fn corrupt_text_is_error_not_panic() {
        for t in ["", "(version: ", "{\"json\": 1}", "(stats: (level: \"x\"))", "\u{0}\u{1}garbage"] {
            assert!(SaveData::parse(t).is_err(), "{t:?}");
        }
    }

    #[test]
    fn dead_player_saved_with_hp() {
        let stats = PlayerStats { hp: -40, dead: true, ..Default::default() };
        let d = SaveData::snapshot(&stats, &Inventory::default(), &Quest::default(), None);
        assert_eq!(d.stats.hp, 1);
        let back = SaveData::parse(&d.to_ron().unwrap()).unwrap();
        assert!(!back.stats.dead && back.stats.hp == 1);
    }

    #[test]
    fn broken_grid_is_rebuilt() {
        let mut inv = Inventory::default();
        inv.add(Item::new(10));
        inv.add(Item::stack(27001, 5));
        inv.cells.truncate(10); // kısa ızgara
        inv.cells.push(Cell::Tail(3)); // boşu gösteren kuyruk
        let fixed = sanitize_inventory(inv);
        assert_eq!(fixed.cells.len(), CELLS);
        assert!(inventory_consistent(&fixed));
        assert_eq!(fixed.count(10), 1);
        assert_eq!(fixed.count(27001), 5);
    }

    #[test]
    fn path_resolution() {
        let exe = Path::new("C:/oyun");
        let p = |s, n, sh, a| resolve_path(s, n, sh, a, Some(exe));
        assert_eq!(p(None, false, false, Some("C:/AD")), Some(PathBuf::from("C:/AD/AzerothMetin/save.ron")));
        assert_eq!(p(None, false, false, None), Some(PathBuf::from("C:/oyun/save/save.ron")));
        assert_eq!(p(None, true, false, Some("C:/AD")), None);
        assert_eq!(p(None, false, true, Some("C:/AD")), None);
        // AZM_SAVE açıkça verilirse AZM_SHOT / AZM_NOSAVE'e rağmen çalışır.
        assert_eq!(p(Some("D:/t.ron"), true, true, Some("C:/AD")), Some(PathBuf::from("D:/t.ron")));
    }

    #[test]
    fn file_write_load_and_corrupt_backup() {
        let dir = std::env::temp_dir().join(format!("azm_save_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("alt").join("save.ron");
        assert!(load_file(&path).is_none()); // dosya yok
        let a = sample();
        write_atomic(&path, &a.to_ron().unwrap()).unwrap();
        assert!(!sibling(&path, "tmp").exists());
        let b = load_file(&path).unwrap();
        assert_eq!(b.inventory.yang, a.inventory.yang);
        // Üzerine yazma da çalışır.
        write_atomic(&path, &a.to_ron().unwrap()).unwrap();
        // Bozuk dosya → None + .bak
        std::fs::write(&path, "(bozuk").unwrap();
        assert!(load_file(&path).is_none());
        assert!(!path.exists());
        assert_eq!(std::fs::read_to_string(sibling(&path, "bak")).unwrap(), "(bozuk");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
