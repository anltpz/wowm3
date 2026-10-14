//! Metin2 tarzı katlı zindanlar — G-26 çekirdeği. `game/mod.rs` bu modülü ve `plugin`'i kaydetti (dokunulmaz).
//!
//! Akış: rehber NPC (`entry.rs`) → `DungeonEnter` → kontroller (seviye, bilet, bekleme süresi, ölü/binekli
//! değil) → oyuncu ADT'siz okyanustaki instance'a ışınlanır → her kat için durum makinesi:
//! `Loading` (WMO + çarpışma kurulur, oyuncu sabit) → `Briefing` (3 sn) → `Active` / `BossFight` (hedef)
//! → `Cleared` (5 sn) → sonraki katın `Loading`'i … → `Victory` (60 sn sonra ya da portaldan çıkış) /
//! `Failed` (süre doldu) → `Exit` (dönüş noktasına ışınlanma, dış dünya akana kadar sabitleme).
//! Zindanda ölen oyuncu R ile **kat başında** doğar ve aynı kattan devam eder (süre dolmadıysa).
//!
//! Alt modüller: `defs` (veri tipleri + `DUNGEONS` kaydı), `arena` (WMO, çarpışma, yürünebilir noktalar,
//! canavar/metin/boss doğurma, temizlik), `entry` (rehber, giriş/çıkış), `spider` (G-28), `tower` (G-29),
//! `atmos` (D-23).
//!
//! UI (D-22) ve atmosfer (D-23) için API: `DungeonRun` (durum, kat, hedef, ilerleme, kalan süre),
//! `DungeonOffer` (rehberle konuşunca açılan giriş teklifi ve şart sonucu), `DungeonEvent` mesajı (giriş,
//! kat, hedef, zafer, başarısızlık, çıkış), komut mesajları `DungeonEnter` / `DungeonBuyTicket` / `DungeonLeave`.

use bevy::prelude::*;

use super::character::ArmorSet;
use super::npc::{NpcDef, NpcKind};

pub mod arena;
pub mod atmos;
pub mod defs;
pub mod entry;
pub mod spider;
pub mod tower;

// UI (D-22) ve içerik (G-28/G-29) için dışa açılan API.
#[allow(unused_imports)]
pub use defs::{DungeonDef, FloorDef, Objective, DUNGEONS};
#[allow(unused_imports)]
pub use entry::{DungeonBuyTicket, DungeonEnter, DungeonLeave, DungeonOffer, EntryError, Offer};

// ---------------------------------------------------------------------------------------------------
// Rehber NPC'leri
// ---------------------------------------------------------------------------------------------------

const GUIDE_SET: ArmorSet = [None, Some(9971), None, None, Some(3261), None, None, None];
const GUIDE_SET2: ArmorSet = [None, Some(30581), Some(30583), Some(30582), Some(30587), Some(30584), Some(30585), None];

/// Zindan girişindeki NPC'ler (Elwynn haritasında doğar; WoW x/y/z): Goldshire yol kavşağı, muhafızın yanı.
/// `NpcKind::DungeonGuide(zindan kimliği)` konuşunca giriş teklifi açar.
pub const NPCS: &[NpcDef] = &[
    NpcDef {
        name: "Örümcek Avcısı Hwang",
        title: "Örümcek Zindanı",
        kind: NpcKind::DungeonGuide(spider::DUNGEON.id),
        map: (-9428.0, 50.0),
        face: (-9440.0, 60.0),
        z: 57.8,
        armor: GUIDE_SET,
        tool: Some(1490),
        look: super::character::Appearance::male(2, 6, 1, 8, 5),
        greet: "Madenin dibinde örümcekler yuva kurdu. Biletin varsa seni oraya götürürüm.",
    },
    NpcDef {
        name: "Bilge Seon-Ja",
        title: "Şeytan Kulesi",
        kind: NpcKind::DungeonGuide(tower::DUNGEON.id),
        map: (-9428.0, 70.0),
        face: (-9440.0, 60.0),
        z: 57.8,
        armor: GUIDE_SET2,
        tool: Some(1376),
        look: super::character::Appearance::female(2, 7, 2, 8, 0),
        greet: "Kulenin mühürleri zayıflıyor. Yalnızca güçlü savaşçılar oraya adım atabilir.",
    },
];

// ---------------------------------------------------------------------------------------------------
// Durum makinesi (saf; birim testli)
// ---------------------------------------------------------------------------------------------------

/// Brifing süresi (sn).
pub const BRIEFING_SECS: f32 = 3.0;
/// Kat temizlendikten sonra sonraki kata geçişe kadar (sn).
pub const CLEARED_SECS: f32 = 5.0;
/// Zaferden sonra otomatik çıkış (sn).
pub const VICTORY_EXIT_SECS: f32 = 60.0;
/// Başarısızlıktan sonra çıkış (sn).
pub const FAILED_EXIT_SECS: f32 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// Katın WMO'su ve çarpışması kuruluyor; oyuncu sabit.
    Loading,
    /// Kat adı + hedef gösteriliyor (`left` sn kaldı).
    Briefing { left: f32 },
    /// Hedef sürüyor (boss dışı).
    Active,
    /// Boss katı.
    BossFight,
    /// Kat bitti; `left` sn sonra sonraki kat.
    Cleared { left: f32 },
    /// Zindan bitti; `left` sn sonra çıkış (portal hemen çıkarır).
    Victory { left: f32 },
    /// Süre doldu; `left` sn sonra çıkış.
    Failed { left: f32 },
    /// Çıkış işlendi/işleniyor.
    Exit,
}

/// Bir katın hedef ilerlemesi.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Progress {
    /// Sayılan canavar (hedefe dahil) sayısı ve öldürülenler.
    pub spawned: u32,
    pub killed: u32,
    /// Kırılan metin taşları.
    pub metins_broken: u32,
    pub key_found: bool,
    pub boss_dead: bool,
}

/// Hedef tamam mı (saf).
pub fn objective_met(obj: &Objective, p: &Progress) -> bool {
    match *obj {
        Objective::KillAll => p.killed >= p.spawned,
        Objective::BreakMetin { count, .. } => p.metins_broken >= count,
        Objective::FindKey { .. } => p.key_found,
        Objective::Boss { .. } => p.boss_dead,
    }
}

/// Durum makinesinin ECS'e bıraktığı iş.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    /// `floor` katının WMO'sunu/çarpışmasını kur (öncekini temizle).
    LoadFloor(usize),
    /// Brifing bitti: katın canavarlarını/metinlerini/boss'unu doğur.
    StartObjective(usize),
    FloorCleared(usize),
    Victory,
    Failed,
    /// Zindandan çık (dönüş noktasına ışınla).
    Leave,
}

/// Etkin bir zindan koşusu.
#[derive(Clone, Debug)]
pub struct RunState {
    pub def: &'static DungeonDef,
    pub floor: usize,
    pub phase: Phase,
    /// Geçen süre (sn; `Loading` sırasında işlemez).
    pub elapsed: f32,
    /// Girişteki WoW konumu: çıkışta buraya dönülür, zindanda alınan kayıt bunu yazar.
    pub return_wow: [f32; 3],
    pub progress: Progress,
    pub deaths: u32,
}

#[allow(dead_code)] // `time_left` / `progress_text`: D-22 izleyicisi
impl RunState {
    pub fn new(def: &'static DungeonDef, return_wow: [f32; 3]) -> Self {
        Self { def, floor: 0, phase: Phase::Loading, elapsed: 0.0, return_wow, progress: Progress::default(), deaths: 0 }
    }

    pub fn floor_def(&self) -> &'static FloorDef {
        &self.def.floors[self.floor.min(self.def.floors.len() - 1)]
    }

    pub fn is_last_floor(&self) -> bool {
        self.floor + 1 >= self.def.floors.len()
    }

    /// Kalan süre (sn).
    pub fn time_left(&self) -> f32 {
        (self.def.time_limit_secs - self.elapsed).max(0.0)
    }

    /// Hedef sürüyor mu (sayaçlar işler).
    pub fn in_objective(&self) -> bool {
        matches!(self.phase, Phase::Active | Phase::BossFight)
    }

    /// Kat yüklendi (WMO + çarpışma hazır, oyuncu giriş noktasında): brifing başlar.
    pub fn on_floor_ready(&mut self) {
        if self.phase == Phase::Loading {
            self.phase = Phase::Briefing { left: BRIEFING_SECS };
        }
    }

    /// Zaman ilerletir; geçiş olursa ECS'in yapacağı işi döndürür.
    pub fn tick(&mut self, dt: f32) -> Option<Action> {
        match &mut self.phase {
            Phase::Victory { left } | Phase::Failed { left } => {
                *left -= dt;
                if *left <= 0.0 {
                    self.phase = Phase::Exit;
                    return Some(Action::Leave);
                }
                return None;
            }
            Phase::Exit => return None,
            Phase::Loading => return None,
            _ => {}
        }
        self.elapsed += dt;
        if self.elapsed >= self.def.time_limit_secs {
            self.phase = Phase::Failed { left: FAILED_EXIT_SECS };
            return Some(Action::Failed);
        }
        match &mut self.phase {
            Phase::Briefing { left } => {
                *left -= dt;
                if *left <= 0.0 {
                    let f = self.floor_def();
                    self.phase = if f.objective.is_boss() { Phase::BossFight } else { Phase::Active };
                    // Sayılacak canavar sayısı tanımdan: doğurma bir kare sonra olsa da hedef erken bitmez.
                    self.progress = Progress { spawned: f.counted_mobs(), ..Default::default() };
                    return Some(Action::StartObjective(self.floor));
                }
                None
            }
            Phase::Cleared { left } => {
                *left -= dt;
                if *left <= 0.0 {
                    if self.is_last_floor() {
                        self.phase = Phase::Victory { left: VICTORY_EXIT_SECS };
                        return Some(Action::Victory);
                    }
                    self.floor += 1;
                    self.phase = Phase::Loading;
                    return Some(Action::LoadFloor(self.floor));
                }
                None
            }
            _ => None,
        }
    }

    /// Hedef tamamlandıysa katı bitirir (son katta zafer).
    pub fn check_objective(&mut self) -> Option<Action> {
        if !self.in_objective() || !objective_met(&self.floor_def().objective, &self.progress) {
            return None;
        }
        if self.is_last_floor() {
            self.phase = Phase::Victory { left: VICTORY_EXIT_SECS };
            Some(Action::Victory)
        } else {
            self.phase = Phase::Cleared { left: CLEARED_SECS };
            Some(Action::FloorCleared(self.floor))
        }
    }

    /// Oyuncu öldü (sayılır; diriliş `respawn_floor` ile kat başında).
    pub fn on_death(&mut self) {
        self.deaths += 1;
    }

    /// Dirilince doğulacak kat: süre dolmadıysa (başarısız/çıkış değil) şu anki kat.
    pub fn respawn_floor(&self) -> Option<usize> {
        (!matches!(self.phase, Phase::Failed { .. } | Phase::Exit)).then_some(self.floor)
    }

    /// "Ayrıl" / portal: hemen çık.
    pub fn abandon(&mut self) -> Option<Action> {
        if self.phase == Phase::Exit {
            return None;
        }
        self.phase = Phase::Exit;
        Some(Action::Leave)
    }

    /// İzleyici satırı ("Canavarlar 3/8", "Metin 1/2", "Anahtar aranıyor", "Boss").
    pub fn progress_text(&self) -> String {
        let p = &self.progress;
        match self.floor_def().objective {
            Objective::KillAll => format!("Canavarlar {}/{}", p.killed.min(p.spawned), p.spawned),
            Objective::BreakMetin { count, .. } => format!("Metin taşı {}/{}", p.metins_broken.min(count), count),
            Objective::FindKey { .. } => if p.key_found { "Anahtar bulundu".into() } else { "Anahtar aranıyor".into() },
            Objective::Boss { .. } => if p.boss_dead { "Boss yenildi".into() } else { "Boss hayatta".into() },
        }
    }
}

/// Zindan durumu (UI ve atmosfer okur). Zindanda değilken `run` boştur.
#[derive(Resource, Default)]
pub struct DungeonRun {
    pub run: Option<RunState>,
    /// Arena sisteminin işleyeceği geçişler.
    pending: Vec<Action>,
    /// Çıkıştan sonra dış dünya akarken oyuncunun sabitlendiği nokta.
    exit_hold: Option<ExitHold>,
}

#[allow(dead_code)] // `state` / `exiting`: D-22 / D-23
impl DungeonRun {
    /// Oyuncu bir zindanda mı.
    pub fn active(&self) -> bool {
        self.run.is_some()
    }
    pub fn state(&self) -> Option<&RunState> {
        self.run.as_ref()
    }
    /// Zindandaysa dönüş konumu (WoW): kayıt bunu yazar.
    pub fn return_wow(&self) -> Option<[f32; 3]> {
        self.run.as_ref().map(|r| r.return_wow)
    }
    /// Çıkış sonrası sabitleme sürüyor mu.
    pub fn exiting(&self) -> bool {
        self.exit_hold.is_some()
    }
}

#[derive(Clone, Copy, Debug)]
struct ExitHold {
    pos: Vec3,
    t: f32,
}

/// UI/atmosfer olayları (D-22 bant/izleyici, D-23 sis + loş ışık: `Entered` / `Left`).
#[derive(Message, Clone, Debug, PartialEq)]
pub enum DungeonEvent {
    /// Zindana girildi (instance'a ışınlanıldı). Atmosfer burada içeri moduna geçer.
    Entered { id: u32 },
    FloorLoading { floor: usize },
    /// Kat hazır, brifing başladı.
    FloorReady { floor: usize },
    ObjectiveStarted { floor: usize },
    KeyFound,
    FloorCleared { floor: usize },
    Victory,
    Failed,
    /// Oyuncu kat başında dirildi.
    Respawned { floor: usize },
    /// Zindandan çıkıldı (dönüş noktasına ışınlanıldı). Atmosfer burada dış moda döner.
    Left { id: u32 },
}

// ---------------------------------------------------------------------------------------------------
// Gerçek zamanlı bekleme süreleri
// ---------------------------------------------------------------------------------------------------

/// Zindan bekleme süreleri: (zindan kimliği, hazır olacağı unix saniye). Kayda `dungeon_cd` olarak yazılır;
/// gerçek saatle işlediği için oyun kapalıyken de akar.
#[derive(Resource, Default, Clone, Debug, PartialEq)]
pub struct DungeonCooldowns(pub Vec<(u32, u64)>);

impl DungeonCooldowns {
    /// Kalan bekleme (sn); `max` saat geri alındıysa sınırı aşmasın diye (zindanın kendi süresi).
    pub fn remaining(&self, id: u32, now: u64, max: u64) -> u64 {
        self.0
            .iter()
            .find(|(d, _)| *d == id)
            .map_or(0, |(_, ready)| ready.saturating_sub(now).min(max))
    }
    /// Beklemeyi başlatır (girişte).
    pub fn start(&mut self, id: u32, now: u64, secs: u64) {
        self.0.retain(|(d, _)| *d != id);
        if secs > 0 {
            self.0.push((id, now + secs));
        }
    }
    /// Süresi dolanları atar (kayıt küçük kalsın).
    pub fn pruned(&self, now: u64) -> Vec<(u32, u64)> {
        self.0.iter().copied().filter(|(_, ready)| *ready > now).collect()
    }
}

/// Şimdiki unix zamanı (sn).
pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// "12 dk 5 sn" biçimi.
pub fn fmt_secs(s: u64) -> String {
    let (h, m, sec) = (s / 3600, (s % 3600) / 60, s % 60);
    if h > 0 {
        format!("{h} sa {m} dk")
    } else if m > 0 {
        format!("{m} dk {sec} sn")
    } else {
        format!("{sec} sn")
    }
}

// ---------------------------------------------------------------------------------------------------
// ECS
// ---------------------------------------------------------------------------------------------------

/// Zindan koşusuna ait her varlık (WMO, çarpışma, canavar, metin, portal, ganimet): kat değişince ve
/// çıkışta silinir.
#[derive(Component)]
pub struct DungeonEntity;

/// Zindan canavarının rolü.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MobRole {
    /// Hedefe sayılan kat canavarı.
    Normal,
    /// Anahtar taşıyıcı (`FindKey`; sayılır da).
    Carrier,
    Boss,
    /// Boss yardımcısı (sayılmaz).
    Add,
}

/// Zindan canavarı etiketi.
#[derive(Component, Clone, Copy, Debug)]
pub struct DungeonMob {
    pub floor: usize,
    pub role: MobRole,
}

/// Zindan metin taşı etiketi.
#[derive(Component, Clone, Copy, Debug)]
pub struct DungeonMetin {
    pub floor: usize,
}

/// Zindan sistemlerinin sırası (oyuncu hareketinden sonra, kameradan önce).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct DungeonSet;

pub fn plugin(app: &mut App) {
    app.add_plugins((atmos::plugin, spider::plugin, tower::plugin))
        .init_resource::<DungeonRun>()
        .init_resource::<DungeonCooldowns>()
        .add_message::<DungeonEvent>()
        .configure_sets(
            Update,
            DungeonSet
                .after(super::player::move_player)
                .before(super::camera::orbit)
                .in_set(benilla_world::schedule::WorldStage::Input),
        )
        .add_systems(
            Update,
            (run_tick, count_kills, check_objective, hold_player).chain().in_set(DungeonSet).after(arena::ArenaSet),
        );
    entry::plugin(app);
    arena::plugin(app);
}

fn push(run: &mut DungeonRun, a: Option<Action>, events: &mut MessageWriter<DungeonEvent>) {
    let Some(a) = a else { return };
    match a {
        Action::LoadFloor(f) => {
            events.write(DungeonEvent::FloorLoading { floor: f });
        }
        Action::StartObjective(f) => {
            events.write(DungeonEvent::ObjectiveStarted { floor: f });
        }
        Action::FloorCleared(f) => {
            events.write(DungeonEvent::FloorCleared { floor: f });
        }
        Action::Victory => {
            events.write(DungeonEvent::Victory);
        }
        Action::Failed => {
            events.write(DungeonEvent::Failed);
        }
        Action::Leave => {}
    }
    run.pending.push(a);
}

fn run_tick(time: Res<Time>, mut run: ResMut<DungeonRun>, mut events: MessageWriter<DungeonEvent>) {
    let dt = time.delta_secs();
    let a = run.run.as_mut().and_then(|r| r.tick(dt));
    push(&mut run, a, &mut events);
}

/// Sayaçlar: kat canavarları, metin taşları, boss, anahtar taşıyıcıları (`combat::Killed`).
fn count_kills(
    time: Res<Time>,
    mut killed: MessageReader<super::combat::Killed>,
    mobs: Query<(&DungeonMob, &super::mobs::Mob)>,
    carriers: Query<(Entity, &DungeonMob), Without<super::combat::Dying>>,
    metins: Query<&DungeonMetin>,
    mut run: ResMut<DungeonRun>,
    mut inv: ResMut<super::inventory::Inventory>,
    mut log: ResMut<super::log::GameLog>,
    mut events: MessageWriter<DungeonEvent>,
) {
    let Some(st) = run.run.as_mut() else {
        killed.clear();
        return;
    };
    let dead: Vec<Entity> = killed.read().map(|k| k.entity).collect();
    if !st.in_objective() {
        return;
    }
    let floor = st.floor;
    let obj = st.floor_def().objective;
    let mut rng = rand::rng();
    use rand::Rng;
    for &e in &dead {
        if let Ok(m) = metins.get(e) {
            if m.floor == floor {
                st.progress.metins_broken += 1;
            }
            continue;
        }
        let Ok((dm, mob)) = mobs.get(e) else { continue };
        if dm.floor != floor {
            continue;
        }
        match dm.role {
            MobRole::Normal | MobRole::Carrier => st.progress.killed += 1,
            MobRole::Boss => st.progress.boss_dead = true,
            MobRole::Add => {}
        }
        if let Objective::FindKey { carriers: ids, key_vnum, chance } = obj {
            if st.progress.key_found || dm.role != MobRole::Carrier || !ids.contains(&mob.def.id) {
                continue;
            }
            // Son taşıyıcı öldüyse anahtar kesin düşer.
            let left = carriers
                .iter()
                .filter(|(ce, cm)| cm.floor == floor && cm.role == MobRole::Carrier && !dead.contains(ce))
                .count();
            if left == 0 || rng.random_bool(chance.clamp(0.0, 1.0) as f64) {
                st.progress.key_found = true;
                let now = time.elapsed_secs();
                if key_vnum != 0 && inv.add(super::items::Item::new(key_vnum)).is_some() {
                    log.push(now, super::log::C_BAD, "Envanterin dolu, anahtar elinde kaldı.");
                }
                log.banner(now, super::log::C_ITEM, "Mühür anahtarını buldun!");
                events.write(DungeonEvent::KeyFound);
            }
        }
    }
}

fn check_objective(mut run: ResMut<DungeonRun>, mut events: MessageWriter<DungeonEvent>) {
    let a = run.run.as_mut().and_then(|r| r.check_objective());
    push(&mut run, a, &mut events);
}

/// Oyuncu sabitleme ve düşme koruması (`move_player`'dan sonra çalışır; player.rs'e dokunmaz):
/// - Kat yüklenirken oyuncu bekleme noktasında asılı durur.
/// - Zindanda dirilen oyuncu (R) köy yerine kat başına ışınlanır.
/// - Zemini olmayan bir noktaya (WMO açıklığı, boşluk) adım atılamaz; çok aşağı düşen kat başına döner.
/// - Çıkıştan sonra dış dünya akıp dönüş noktasının altında zemin bulunana kadar oyuncu sabit kalır.
fn hold_player(
    time: Res<Time>,
    mut run: ResMut<DungeonRun>,
    mut arena: ResMut<arena::Arena>,
    stats: Res<super::stats::PlayerStats>,
    mut player: Query<(&mut Transform, &mut super::player::Player)>,
    col: benilla_world::collision::WorldCollision,
    mut log: ResMut<super::log::GameLog>,
    mut events: MessageWriter<DungeonEvent>,
    mut prev_dead: Local<bool>,
) {
    let Ok((mut xf, mut p)) = player.single_mut() else { return };
    let run = &mut *run;
    let xf = &mut *xf;
    let p = &mut *p;
    let revived = *prev_dead && !stats.dead;
    let died = !*prev_dead && stats.dead;
    *prev_dead = stats.dead;
    let pin = |xf: &mut Transform, p: &mut super::player::Player, at: Vec3| {
        xf.translation = at;
        p.airborne = false;
        p.vy = 0.0;
        p.grounded = true;
    };
    // Çıkış sonrası: dış dünya akana kadar dönüş noktasında bekle.
    if let Some(h) = run.exit_hold.as_mut() {
        if revived || stats.dead {
            run.exit_hold = None;
            return;
        }
        h.t += time.delta_secs();
        let found = super::physics::ground_at(&col, h.pos).filter(|g| (g - h.pos.y).abs() < 2.5);
        let (pos, t) = (h.pos, h.t);
        if let Some(g) = found {
            pin(xf, p, Vec3::new(pos.x, g, pos.z));
            info!("dungeon: dış dünya hazır ({t:.1} sn), zemin {g:.2}");
            run.exit_hold = None;
        } else if t > 20.0 {
            warn!("dungeon: dönüş noktasında zemin bulunamadı (20 sn), oyuncu serbest");
            p.grounded = false;
            run.exit_hold = None;
        } else {
            pin(xf, p, pos);
        }
        return;
    }
    let Some(st) = run.run.as_mut() else { return };
    if died {
        st.on_death();
        return;
    }
    if !arena.ready {
        // Kat kuruluyor: bekleme noktasında asılı.
        if let Some(h) = arena.hold {
            pin(xf, p, h);
        }
        return;
    }
    let Some(entry) = arena.entry else { return };
    if revived {
        if let Some(floor) = st.respawn_floor() {
            pin(xf, p, entry);
            arena.last_good = Some(entry);
            log.push(time.elapsed_secs(), super::log::C_GOOD, "Kat başında yeniden doğdun.");
            events.write(DungeonEvent::Respawned { floor });
        }
        return;
    }
    if stats.dead {
        return;
    }
    // Düşme koruması.
    let pos = xf.translation;
    let ground = super::physics::ground_at(&col, pos);
    // Test/QA (`AZM_DUNGEON`): 2 sn'de bir konum ve zemin ışını (Q-07 zemin tutarlılığı).
    if std::env::var_os("AZM_DUNGEON").is_some() {
        let t = time.elapsed_secs();
        if (t / 2.0).floor() != ((t - time.delta_secs()) / 2.0).floor() {
            info!(
                "dungeon-izle: kat {} {:?} oyuncu ({:.2}, {:.2}, {:.2}) zemin {:?} hava {} kalan {:.0} sn",
                st.floor + 1,
                st.phase,
                pos.x,
                pos.y,
                pos.z,
                ground.map(|g| (g * 100.0).round() / 100.0),
                p.airborne,
                st.time_left()
            );
        }
    }
    if pos.y < arena.min_y - 12.0 {
        warn!("dungeon: oyuncu katın altına düştü ({:.1}), kat başına dönüyor", pos.y);
        pin(xf, p, entry);
    } else if ground.is_none() && !p.airborne {
        let back = arena.last_good.unwrap_or(entry);
        xf.translation = back;
    } else if ground.is_some() && !p.airborne {
        arena.last_good = Some(pos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TWO_FLOORS: DungeonDef = DungeonDef {
        id: 77,
        key: "iki",
        name: "İki Katlı",
        level: (5, 30),
        ticket: 0,
        ticket_price: 0,
        cooldown_secs: 600,
        time_limit_secs: 100.0,
        floors: &[
            FloorDef {
                name: "1",
                wmo: "a.wmo",
                offset: [0.0; 3],
                yaw: 0.0,
                entry: None,
                mobs: &[(202, 2)],
                objective: Objective::KillAll,
            },
            FloorDef {
                name: "2",
                wmo: "b.wmo",
                offset: [0.0; 3],
                yaw: 0.0,
                entry: None,
                mobs: &[],
                objective: Objective::Boss { id: 225, adds: &[] },
            },
        ],
    };

    fn ready_and_brief(r: &mut RunState) -> Option<Action> {
        r.on_floor_ready();
        assert!(matches!(r.phase, Phase::Briefing { .. }));
        let mut out = None;
        for _ in 0..40 {
            if let Some(a) = r.tick(0.1) {
                out = Some(a);
                break;
            }
        }
        out
    }

    #[test]
    fn floor_transition_and_victory() {
        let mut r = RunState::new(&TWO_FLOORS, [1.0, 2.0, 3.0]);
        assert_eq!(r.phase, Phase::Loading);
        // Yüklenirken süre işlemez.
        assert_eq!(r.tick(10.0), None);
        assert_eq!(r.elapsed, 0.0);
        assert_eq!(ready_and_brief(&mut r), Some(Action::StartObjective(0)));
        assert_eq!(r.phase, Phase::Active);
        assert_eq!(r.progress.spawned, 2, "sayaç tanımdan");
        assert_eq!(r.check_objective(), None, "doğurmadan önce bitmez");
        r.progress.killed = 1;
        assert_eq!(r.check_objective(), None);
        r.progress.killed = 2;
        assert_eq!(r.check_objective(), Some(Action::FloorCleared(0)));
        assert!(matches!(r.phase, Phase::Cleared { .. }));
        // Sayaç tekrar tetiklemez.
        assert_eq!(r.check_objective(), None);
        let mut a = None;
        for _ in 0..60 {
            a = a.or(r.tick(0.1));
        }
        assert_eq!(a, Some(Action::LoadFloor(1)));
        assert_eq!((r.floor, r.phase), (1, Phase::Loading));
        // Boss katı.
        assert_eq!(ready_and_brief(&mut r), Some(Action::StartObjective(1)));
        assert_eq!(r.phase, Phase::BossFight);
        assert_eq!(r.progress, Progress::default());
        r.progress.boss_dead = true;
        assert_eq!(r.check_objective(), Some(Action::Victory));
        // Zafer 60 sn sonra çıkar; süre sınırı zaferden sonra başarısızlığa çevirmez.
        let mut leave = None;
        for _ in 0..700 {
            if let Some(a) = r.tick(0.1) {
                leave = Some(a);
                break;
            }
        }
        assert_eq!(leave, Some(Action::Leave));
        assert_eq!(r.phase, Phase::Exit);
        assert_eq!(r.tick(1.0), None);
    }

    #[test]
    fn death_respawns_on_same_floor_until_timeout() {
        let mut r = RunState::new(&TWO_FLOORS, [0.0; 3]);
        ready_and_brief(&mut r);
        r.progress.spawned = 2;
        r.progress.killed = 2;
        r.check_objective();
        for _ in 0..60 {
            r.tick(0.1);
        }
        assert_eq!(r.floor, 1);
        r.on_death();
        assert_eq!(r.deaths, 1);
        assert_eq!(r.respawn_floor(), Some(1));
        // Ölüm katı/ilerlemeyi sıfırlamaz.
        assert_eq!(r.floor, 1);
    }

    #[test]
    fn timeout_fails_then_leaves() {
        let mut r = RunState::new(&TWO_FLOORS, [0.0; 3]);
        ready_and_brief(&mut r);
        let mut got = Vec::new();
        for _ in 0..2000 {
            if let Some(a) = r.tick(0.1) {
                got.push(a);
            }
        }
        assert_eq!(got, vec![Action::Failed, Action::Leave]);
        assert_eq!(r.respawn_floor(), None);
        assert_eq!(r.check_objective(), None);
        assert_eq!(r.time_left(), 0.0);
    }

    #[test]
    fn abandon_leaves_once() {
        let mut r = RunState::new(&TWO_FLOORS, [0.0; 3]);
        assert_eq!(r.abandon(), Some(Action::Leave));
        assert_eq!(r.abandon(), None);
    }

    #[test]
    fn objectives() {
        let p = Progress { spawned: 3, killed: 3, ..Default::default() };
        assert!(objective_met(&Objective::KillAll, &p));
        assert!(objective_met(&Objective::KillAll, &Progress::default()), "mobsuz kat hemen biter");
        assert!(!objective_met(&Objective::BreakMetin { metin: 0, count: 2, summons: &[] }, &Progress { metins_broken: 1, ..p }));
        assert!(objective_met(&Objective::BreakMetin { metin: 0, count: 2, summons: &[] }, &Progress { metins_broken: 2, ..p }));
        assert!(!objective_met(&Objective::FindKey { carriers: &[1], key_vnum: 0, chance: 0.1 }, &p));
        assert!(objective_met(&Objective::FindKey { carriers: &[1], key_vnum: 0, chance: 0.1 }, &Progress { key_found: true, ..p }));
        assert!(!objective_met(&Objective::Boss { id: 1, adds: &[] }, &p));
    }

    #[test]
    fn cooldowns_real_time() {
        let mut cd = DungeonCooldowns::default();
        assert_eq!(cd.remaining(1, 1000, 1800), 0);
        cd.start(1, 1000, 1800);
        assert_eq!(cd.remaining(1, 1000, 1800), 1800);
        // Oyun kapalıyken geçen süre de sayılır (unix saat).
        assert_eq!(cd.remaining(1, 2000, 1800), 800);
        assert_eq!(cd.remaining(1, 2800, 1800), 0);
        assert_eq!(cd.remaining(2, 1000, 1800), 0);
        // Saat geri alınırsa bekleme zindanın süresini aşmaz.
        assert_eq!(cd.remaining(1, 10, 1800), 1800);
        // Yeniden başlatma eskisinin yerine geçer.
        cd.start(1, 5000, 60);
        assert_eq!(cd.0.len(), 1);
        assert_eq!(cd.pruned(5059), vec![(1, 5060)]);
        assert!(cd.pruned(5060).is_empty());
        assert_eq!(fmt_secs(3725), "1 sa 2 dk");
        assert_eq!(fmt_secs(125), "2 dk 5 sn");
    }

    #[test]
    fn guides_point_to_registered_dungeons() {
        for n in NPCS {
            let NpcKind::DungeonGuide(id) = n.kind else { panic!("{} rehber değil", n.name) };
            let d = defs::find(id).expect("rehberin zindanı kayıtlı");
            assert!(d.ticket == 0 || d.ticket_price > 0, "{}: bilet satılmıyor", d.name);
            assert_eq!(crate::game::elwynn::subzone([n.map.0, n.map.1, n.z]), "Goldshire");
        }
    }
}
