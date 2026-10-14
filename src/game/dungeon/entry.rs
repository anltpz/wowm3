//! Zindan girişi ve çıkışı.
//!
//! - Rehber NPC (`NpcKind::DungeonGuide(id)`) ile konuşunca `DungeonOffer` açılır: şartlar (`check_entry`)
//!   sohbet kutusuna yazılır. **Y** = gir (bilet yoksa ve rehber satıyorsa önce bileti satın alır),
//!   **N**/Esc = kapat. Uzaklaşınca teklif kapanır.
//! - Zindanda **L** iki kez (3 sn içinde) = "Ayrıl". Zaferden sonra portal ya da 60 sn; süre dolunca otomatik.
//! - UI (D-22) aynı işleri mesajlarla yapar: `DungeonEnter`, `DungeonBuyTicket`, `DungeonLeave`; teklif ve
//!   şart sonucu `DungeonOffer`'dan okunur.
//! - Test: `AZM_DUNGEON=<test|spider|tower>` oyun açılınca şartsız doğrudan o zindana girer.

use bevy::prelude::*;
use benilla_assets::coords::bevy_to_wow;

use super::defs::{self, DungeonDef};
use super::{unix_now, Action, DungeonCooldowns, DungeonEvent, DungeonRun, RunState};
use crate::game::inventory::Inventory;
use crate::game::items::Item;
use crate::game::log::{GameLog, C_BAD, C_GOOD, C_INFO};
use crate::game::npc::{Npc, NpcKind, NpcTalked};
use crate::game::player::Player;
use crate::game::stats::PlayerStats;

/// Zindana gir. `force` şartları ve bileti atlar (test, `AZM_DUNGEON`).
#[derive(Message, Clone, Copy, Debug)]
pub struct DungeonEnter {
    pub id: u32,
    pub force: bool,
}

/// Rehberden bilet satın al (`ticket_price` Yang).
#[derive(Message, Clone, Copy, Debug)]
pub struct DungeonBuyTicket {
    pub id: u32,
}

/// Zindandan ayrıl ("Ayrıl" düğmesi / L L).
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct DungeonLeave;

/// Giriş şartı hatası.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryError {
    AlreadyInside,
    Dead,
    Riding,
    Level { min: u32, max: u32 },
    /// Bekleme süresi (kalan sn).
    Cooldown { secs: u64 },
    /// Bilet yok (vnum).
    NoTicket { vnum: u32 },
}

impl EntryError {
    pub fn text(&self) -> String {
        match *self {
            EntryError::AlreadyInside => "Zaten bir zindandasın.".into(),
            EntryError::Dead => "Ölüyken giremezsin.".into(),
            EntryError::Riding => "Binekten inmelisin.".into(),
            EntryError::Level { min, max } => format!("Seviyen uygun değil (Sv {min}-{max})."),
            EntryError::Cooldown { secs } => format!("Tekrar girebilmek için {} beklemelisin.", super::fmt_secs(secs)),
            EntryError::NoTicket { vnum } => {
                let name = crate::game::items::def(vnum).map_or("Giriş bileti", |d| d.name);
                format!("{name} gerekli.")
            }
        }
    }
}

fn ticket_name(vnum: u32) -> &'static str {
    crate::game::items::def(vnum).map_or("Giriş bileti", |d| d.name)
}

/// Giriş şartlarının girdisi (saf kontrol için).
#[derive(Clone, Copy, Debug)]
pub struct EntryCtx {
    pub level: u32,
    /// Envanterdeki bilet sayısı.
    pub tickets: u32,
    pub now: u64,
    pub dead: bool,
    pub riding: bool,
    pub inside: bool,
}

/// Giriş şartları (saf): içeride değil → ölü değil → binekli değil → seviye → bekleme → bilet.
pub fn check_entry(def: &DungeonDef, ctx: &EntryCtx, cds: &DungeonCooldowns) -> Result<(), EntryError> {
    if ctx.inside {
        return Err(EntryError::AlreadyInside);
    }
    if ctx.dead {
        return Err(EntryError::Dead);
    }
    if ctx.riding {
        return Err(EntryError::Riding);
    }
    if ctx.level < def.level.0 || ctx.level > def.level.1 {
        return Err(EntryError::Level { min: def.level.0, max: def.level.1 });
    }
    let secs = cds.remaining(def.id, ctx.now, def.cooldown_secs);
    if secs > 0 {
        return Err(EntryError::Cooldown { secs });
    }
    if def.ticket != 0 && ctx.tickets == 0 {
        return Err(EntryError::NoTicket { vnum: def.ticket });
    }
    Ok(())
}

/// Rehberle konuşunca açılan giriş teklifi (D-22 giriş diyaloğu bunu çizer).
#[derive(Clone, Copy, Debug)]
pub struct Offer {
    pub dungeon: &'static DungeonDef,
    pub npc: Entity,
    /// Son şart kontrolü (her karede güncellenir).
    pub check: Result<(), EntryError>,
}

#[derive(Resource, Default)]
pub struct DungeonOffer(pub Option<Offer>);

/// Giriş sistemlerinin kümesi (arena sistemlerinden önce).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntrySet;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<DungeonOffer>()
        .add_message::<DungeonEnter>()
        .add_message::<DungeonBuyTicket>()
        .add_message::<DungeonLeave>()
        .configure_sets(Update, EntrySet.in_set(super::DungeonSet).before(super::arena::ArenaSet))
        .add_systems(
            Update,
            (env_start, open_offer, offer_input, buy_ticket, enter, leave).chain().in_set(EntrySet),
        );
}

fn ctx(stats: &PlayerStats, inv: &Inventory, def: &DungeonDef, inside: bool) -> EntryCtx {
    EntryCtx {
        level: stats.level,
        tickets: if def.ticket == 0 { 0 } else { inv.count(def.ticket) },
        now: unix_now(),
        dead: stats.dead,
        riding: stats.riding,
        inside,
    }
}

/// `AZM_DUNGEON=<ad>`: ilk karede şartsız giriş.
fn env_start(mut done: Local<bool>, player: Query<(), With<Player>>, mut out: MessageWriter<DungeonEnter>) {
    if *done || player.single().is_err() {
        return;
    }
    *done = true;
    let Ok(key) = std::env::var("AZM_DUNGEON") else { return };
    match defs::by_key(&key) {
        Some(d) => {
            info!("dungeon: AZM_DUNGEON={key} → {}", d.name);
            out.write(DungeonEnter { id: d.id, force: true });
        }
        None => warn!("dungeon: AZM_DUNGEON={key} bilinmiyor ({:?})", defs::DUNGEONS.iter().map(|d| d.key).collect::<Vec<_>>()),
    }
}

/// Rehberle konuşma → teklif + şartlar sohbet kutusuna.
fn open_offer(
    time: Res<Time>,
    mut talked: MessageReader<NpcTalked>,
    mut offer: ResMut<DungeonOffer>,
    run: Res<DungeonRun>,
    (stats, inv, cds): (Res<PlayerStats>, Res<Inventory>, Res<DungeonCooldowns>),
    mut log: ResMut<GameLog>,
) {
    for t in talked.read() {
        let NpcKind::DungeonGuide(id) = t.kind else { continue };
        let Some(def) = defs::find(id) else {
            warn!("dungeon: rehberin zindanı {id} kayıtlı değil");
            continue;
        };
        let check = check_entry(def, &ctx(&stats, &inv, def, run.active()), &cds);
        let now = time.elapsed_secs();
        log.push(
            now,
            C_INFO,
            format!("{} — Sv {}-{}, {} kat, süre {}.", def.name, def.level.0, def.level.1, def.floors.len(), super::fmt_secs(def.time_limit_secs as u64)),
        );
        match check {
            Ok(()) => log.push(now, C_GOOD, "Girmek için Y, vazgeçmek için N."),
            Err(EntryError::NoTicket { vnum }) if def.ticket_price > 0 => log.push(
                now,
                C_INFO,
                format!("{} yok. {} Yang'a satın almak için Y.", ticket_name(vnum), def.ticket_price),
            ),
            Err(e) => log.push(now, C_BAD, e.text()),
        }
        offer.0 = Some(Offer { dungeon: def, npc: t.entity, check });
    }
}

/// Teklif klavyesi (Y/N/Esc), uzaklaşınca kapanma, zindanda L L = ayrıl.
fn offer_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut offer: ResMut<DungeonOffer>,
    run: Res<DungeonRun>,
    (stats, inv, cds): (Res<PlayerStats>, Res<Inventory>, Res<DungeonCooldowns>),
    npcs: Query<&Transform, With<Npc>>,
    player: Query<&Transform, With<Player>>,
    (mut enter, mut buy, mut leave): (MessageWriter<DungeonEnter>, MessageWriter<DungeonBuyTicket>, MessageWriter<DungeonLeave>),
    mut log: ResMut<GameLog>,
    mut leave_armed: Local<Option<f32>>,
) {
    let now = time.elapsed_secs();
    if run.active() {
        offer.0 = None;
        if keys.just_pressed(KeyCode::KeyL) {
            match *leave_armed {
                Some(t) if now - t < 3.0 => {
                    *leave_armed = None;
                    leave.write(DungeonLeave);
                }
                _ => {
                    *leave_armed = Some(now);
                    log.push(now, C_INFO, "Zindandan ayrılmak için 3 sn içinde tekrar L'ye bas.");
                }
            }
        }
        return;
    }
    let Some(o) = offer.0.as_mut() else { return };
    o.check = check_entry(o.dungeon, &ctx(&stats, &inv, o.dungeon, false), &cds);
    let far = match (npcs.get(o.npc), player.single()) {
        (Ok(n), Ok(p)) => n.translation.distance(p.translation) > 8.0,
        _ => true,
    };
    if far || keys.just_pressed(KeyCode::KeyN) || keys.just_pressed(KeyCode::Escape) {
        offer.0 = None;
        return;
    }
    if !keys.just_pressed(KeyCode::KeyY) {
        return;
    }
    match o.check {
        Ok(()) => {
            enter.write(DungeonEnter { id: o.dungeon.id, force: false });
        }
        Err(EntryError::NoTicket { .. }) if o.dungeon.ticket_price > 0 => {
            buy.write(DungeonBuyTicket { id: o.dungeon.id });
        }
        Err(e) => log.push(now, C_BAD, e.text()),
    }
}

/// Bilet satın alma (rehber).
fn buy_ticket(
    time: Res<Time>,
    mut msgs: MessageReader<DungeonBuyTicket>,
    mut inv: ResMut<Inventory>,
    mut log: ResMut<GameLog>,
) {
    for m in msgs.read() {
        let Some(def) = defs::find(m.id) else { continue };
        let now = time.elapsed_secs();
        if def.ticket == 0 || def.ticket_price == 0 {
            continue;
        }
        if inv.yang < def.ticket_price as u64 {
            log.push(now, C_BAD, format!("Yeterli Yang yok ({} gerekli).", def.ticket_price));
            continue;
        }
        if inv.add(Item::new(def.ticket)).is_some() {
            log.push(now, C_BAD, "Envanterin dolu.");
            continue;
        }
        inv.yang -= def.ticket_price as u64;
        inv.touch();
        log.push(
            now,
            C_GOOD,
            format!("{} satın aldın (-{} Yang). Girmek için Y.", ticket_name(def.ticket), def.ticket_price),
        );
    }
}

/// Giriş: şartlar → bilet tüketimi + bekleme süresi → koşu başlar, oyuncu instance'a ışınlanır (arena).
fn enter(
    time: Res<Time>,
    mut msgs: MessageReader<DungeonEnter>,
    mut run: ResMut<DungeonRun>,
    (mut stats, mut inv, mut cds): (ResMut<PlayerStats>, ResMut<Inventory>, ResMut<DungeonCooldowns>),
    mut offer: ResMut<DungeonOffer>,
    player: Query<&Transform, With<Player>>,
    (mut target, mut move_to, mut auto): (
        ResMut<crate::game::combat::PlayerTarget>,
        ResMut<crate::game::combat::MoveTo>,
        ResMut<crate::game::combat::AutoAttack>,
    ),
    mut log: ResMut<GameLog>,
    mut events: MessageWriter<DungeonEvent>,
) {
    for m in msgs.read() {
        let now = time.elapsed_secs();
        let Some(def) = defs::find(m.id) else {
            warn!("dungeon: bilinmeyen zindan {}", m.id);
            continue;
        };
        let Ok(pxf) = player.single() else { continue };
        if !m.force {
            if let Err(e) = check_entry(def, &ctx(&stats, &inv, def, run.active()), &cds) {
                log.push(now, C_BAD, e.text());
                continue;
            }
            if def.ticket != 0 && !inv.consume(def.ticket, 1) {
                log.push(now, C_BAD, EntryError::NoTicket { vnum: def.ticket }.text());
                continue;
            }
            cds.start(def.id, unix_now(), def.cooldown_secs);
        } else if run.active() {
            continue;
        }
        if stats.riding {
            stats.riding = false;
        }
        let ret = bevy_to_wow(pxf.translation);
        run.run = Some(RunState::new(def, ret));
        run.pending.clear();
        run.pending.push(Action::LoadFloor(0));
        run.exit_hold = None;
        crate::game::physics::set_indoor(true);
        offer.0 = None;
        target.0 = None;
        move_to.0 = None;
        auto.0 = false;
        info!("dungeon: giriş {} (dönüş WoW {:.1}, {:.1}, {:.1})", def.name, ret[0], ret[1], ret[2]);
        log.banner(now, C_GOOD, def.name);
        log.push(now, C_GOOD, format!("{} kapıları açıldı. Süre: {}.", def.name, super::fmt_secs(def.time_limit_secs as u64)));
        events.write(DungeonEvent::Entered { id: def.id });
    }
}

/// "Ayrıl": koşuyu bitirir (arena çıkışı işler).
fn leave(mut msgs: MessageReader<DungeonLeave>, mut run: ResMut<DungeonRun>) {
    if msgs.read().count() == 0 {
        return;
    }
    let a = run.run.as_mut().and_then(|r| r.abandon());
    if let Some(a) = a {
        run.pending.push(a);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::dungeon::defs::{FloorDef, Objective};

    const D: DungeonDef = DungeonDef {
        id: 5,
        key: "d",
        name: "D",
        level: (10, 25),
        ticket: 71000,
        ticket_price: 100,
        cooldown_secs: 1800,
        time_limit_secs: 600.0,
        floors: &[FloorDef {
            name: "1",
            wmo: "x.wmo",
            offset: [0.0; 3],
            yaw: 0.0,
            entry: None,
            mobs: &[],
            objective: Objective::KillAll,
        }],
    };

    fn ok_ctx() -> EntryCtx {
        EntryCtx { level: 12, tickets: 1, now: 10_000, dead: false, riding: false, inside: false }
    }

    #[test]
    fn entry_checks_in_order() {
        let cds = DungeonCooldowns::default();
        assert_eq!(check_entry(&D, &ok_ctx(), &cds), Ok(()));
        assert_eq!(check_entry(&D, &EntryCtx { inside: true, dead: true, ..ok_ctx() }, &cds), Err(EntryError::AlreadyInside));
        assert_eq!(check_entry(&D, &EntryCtx { dead: true, riding: true, ..ok_ctx() }, &cds), Err(EntryError::Dead));
        assert_eq!(check_entry(&D, &EntryCtx { riding: true, ..ok_ctx() }, &cds), Err(EntryError::Riding));
        assert_eq!(check_entry(&D, &EntryCtx { level: 9, ..ok_ctx() }, &cds), Err(EntryError::Level { min: 10, max: 25 }));
        assert_eq!(check_entry(&D, &EntryCtx { level: 26, ..ok_ctx() }, &cds), Err(EntryError::Level { min: 10, max: 25 }));
        assert_eq!(check_entry(&D, &EntryCtx { tickets: 0, ..ok_ctx() }, &cds), Err(EntryError::NoTicket { vnum: 71000 }));
        // Bekleme bilet kontrolünden önce.
        let mut cds = DungeonCooldowns::default();
        cds.start(D.id, 10_000, D.cooldown_secs);
        assert_eq!(
            check_entry(&D, &EntryCtx { tickets: 0, now: 10_600, ..ok_ctx() }, &cds),
            Err(EntryError::Cooldown { secs: 1200 })
        );
        assert_eq!(check_entry(&D, &EntryCtx { now: 11_800, ..ok_ctx() }, &cds), Ok(()));
        // Biletsiz zindan bilet istemez.
        let free = DungeonDef { ticket: 0, ..D };
        assert_eq!(check_entry(&free, &EntryCtx { tickets: 0, ..ok_ctx() }, &DungeonCooldowns::default()), Ok(()));
    }

    #[test]
    fn error_texts() {
        assert!(EntryError::Cooldown { secs: 125 }.text().contains("2 dk 5 sn"));
        assert!(EntryError::NoTicket { vnum: 71000 }.text().contains("Örümcek"));
        assert!(EntryError::Level { min: 10, max: 25 }.text().contains("10-25"));
    }
}
