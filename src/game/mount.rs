//! Binek (Seyis): Binicilik Bileti (10 dk) ya da Ölüm Atı Dizginleri (Sv25, süresiz) ile Ölümsüz Savaş Atı.
//! MPQ'da normal at modeli yok; kullanıcı kararıyla `RidingUndeadWarHorse` kullanılıyor.
//! - Ctrl+G ya da bilet/madalyona sağ tık: bin / in. Binekteyken hız ×`SPEED_MUL`, saldırı yok
//!   (saldırı ya da beceri denenince oyuncu attan iner).
//! - Seyis ile konuşunca: Sv25+ ve madalyon yoksa Ölüm Atı Dizginleri, yoksa (bilet ve süre yoksa) bir bilet verir.
//! - Durum `PlayerStats.mount_left` / `riding` ile `save.ron`'a yazılır.
//! - Test: `AZM_RIDE=1` binek üstünde başlatır.
//! - Görsel: at oyuncunun çocuğu; gövde (`Body`) atın binici takma noktasına (0) taşınır, "Mount" animasyonu.

use bevy::prelude::*;
use benilla_assets::WorldAssets;

use super::combat::AutoAttack;
use super::inventory::Inventory;
use super::items::{self, Item, Kind};
use super::log::{GameLog, C_BAD, C_GOOD, C_INFO, C_ITEM};
use super::model::{anim, load_m2, AnimState, ModelReady, WowModel};
use super::npc::{NpcKind, NpcTalked};
use super::player::{Body, Player};
use super::skills::CastSkill;
use super::stats::PlayerStats;

/// Binek hız çarpanı.
pub const SPEED_MUL: f32 = 1.6;
/// Ölüm Atı Dizginleri kullanma seviyesi.
pub const MEDAL_LEVEL: u32 = 25;
const STEED_M2: &str = "Creature/MountedDeathKnight/RidingUndeadWarHorse.m2";
const STEED_SKIN: &str = "Creature/MountedDeathKnight/MountedDeathKnightBlack_01.blp";
/// Binek modelinde binicinin takma noktası.
const RIDER_ATTACH: u16 = 0;

/// Bin / in isteği (envanter sağ tık).
#[derive(Message, Clone, Copy, Debug)]
pub enum MountRequest {
    UseItem(u32),
}

/// Oyuncunun bindiği at (oyuncunun çocuğu).
#[derive(Component)]
pub struct Steed;

pub fn plugin(app: &mut App) {
    app.add_message::<MountRequest>().add_systems(Startup, env_ride).add_systems(
        Update,
        (requests, stable_gift, tick, dismount_on_attack, sync_steed, seat_rider, animate_steed).chain(),
    );
}

/// Test: `AZM_RIDE=1` ile binek üstünde (10 dk) başla.
fn env_ride(mut stats: ResMut<PlayerStats>) {
    if std::env::var("AZM_RIDE").is_ok_and(|v| v == "1") {
        stats.riding = true;
        stats.mount_left = stats.mount_left.max(600.0);
    }
}

/// Madalyon kullanılabilir mi (envanterde ya da kuşanılmış değil; envanterde olması yeter).
pub fn has_medal(stats: &PlayerStats, inv: &Inventory) -> bool {
    stats.level >= MEDAL_LEVEL && inv.count(items::V_HORSE_MEDAL) > 0
}

/// Ctrl+G / madalyon: bin ya da in. Dönen metin sohbet kutusuna yazılır.
pub fn toggle(stats: &mut PlayerStats, inv: &Inventory) -> Result<&'static str, &'static str> {
    if stats.riding {
        stats.riding = false;
        return Ok("Binekten indin.");
    }
    if stats.dead {
        return Err("Ölüyken binemezsin.");
    }
    if has_medal(stats, inv) || stats.mount_left > 0.0 {
        stats.riding = true;
        return Ok("Ölümsüz Savaş Atı'na bindin.");
    }
    if inv.count(items::V_HORSE_MEDAL) > 0 {
        return Err("Ölüm Atı Dizginleri için seviye 25 gerekli.");
    }
    Err("Binek yok. Seyis'ten Binicilik Bileti al.")
}

/// Bileti kullan: süre eklenir ve binilir.
pub fn use_ticket(stats: &mut PlayerStats, inv: &mut Inventory) -> Result<String, &'static str> {
    if stats.dead {
        return Err("Ölüyken binemezsin.");
    }
    let Some(Kind::MountTicket { secs }) = items::def(items::V_RIDE_TICKET).map(|d| d.kind) else {
        return Err("Bilet tanımı yok.");
    };
    if !inv.consume(items::V_RIDE_TICKET, 1) {
        return Err("Binicilik Bileti yok.");
    }
    stats.mount_left += secs as f32;
    stats.riding = true;
    Ok(format!("Ölümsüz Savaş Atı'na bindin. Kalan süre: {}", clock(stats.mount_left)))
}

/// Süreyi düşür (madalyon varken düşmez). Süre bitince iner; `true` = şimdi indi.
pub fn tick_time(stats: &mut PlayerStats, medal: bool, dt: f32) -> bool {
    if !stats.riding || medal {
        return false;
    }
    stats.mount_left = (stats.mount_left - dt).max(0.0);
    if stats.mount_left <= 0.0 {
        stats.riding = false;
        return true;
    }
    false
}

/// dd:ss biçimi.
pub fn clock(secs: f32) -> String {
    let s = secs.max(0.0).ceil() as u32;
    format!("{}:{:02}", s / 60, s % 60)
}

fn requests(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut ev: MessageReader<MountRequest>,
    mut stats: ResMut<PlayerStats>,
    mut inv: ResMut<Inventory>,
    mut log: ResMut<GameLog>,
    mut auto: ResMut<AutoAttack>,
) {
    let now = time.elapsed_secs();
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let mut wants: Vec<Option<u32>> = ev.read().map(|MountRequest::UseItem(v)| Some(*v)).collect();
    if ctrl && keys.just_pressed(KeyCode::KeyG) {
        wants.push(None);
    }
    for w in wants {
        let res = match w {
            // Binekteyken bilete sağ tık: süre ekle, inme.
            Some(items::V_RIDE_TICKET) => use_ticket(&mut stats, &mut inv),
            _ => toggle(&mut stats, &inv).map(str::to_string),
        };
        if res.is_ok() && stats.riding {
            // Binince otomatik saldırı kapanır (açılması attan indirir).
            auto.0 = false;
        }
        match res {
            Ok(m) => log.push(now, C_GOOD, m),
            Err(e) => log.push(now, C_BAD, e),
        }
    }
}

/// Seyis'le konuşunca binek ver.
fn stable_gift(
    time: Res<Time>,
    mut talked: MessageReader<NpcTalked>,
    stats: Res<PlayerStats>,
    mut inv: ResMut<Inventory>,
    mut log: ResMut<GameLog>,
) {
    let now = time.elapsed_secs();
    for t in talked.read() {
        if t.kind != NpcKind::Stable {
            continue;
        }
        let gift = if stats.level >= MEDAL_LEVEL && inv.count(items::V_HORSE_MEDAL) == 0 {
            Some(items::V_HORSE_MEDAL)
        } else if inv.count(items::V_HORSE_MEDAL) == 0
            && inv.count(items::V_RIDE_TICKET) == 0
            && stats.mount_left <= 0.0
        {
            Some(items::V_RIDE_TICKET)
        } else {
            None
        };
        match gift {
            Some(v) => {
                let name = items::def(v).map(|d| d.name).unwrap_or("?");
                if inv.add(Item::new(v)).is_some() {
                    log.push(now, C_BAD, "Envanter dolu, Seyis'in hediyesi verilemedi.");
                } else {
                    log.push(now, C_ITEM, format!("Seyis sana bir {name} verdi. Sağ tıkla bin, Ctrl+G ile in."));
                }
            }
            None => log.push(now, C_INFO, "Seyis: Atın hazır. Ctrl+G ile binip inebilirsin."),
        }
    }
}

fn tick(time: Res<Time>, mut stats: ResMut<PlayerStats>, inv: Res<Inventory>, mut log: ResMut<GameLog>) {
    if stats.dead && stats.riding {
        stats.riding = false;
        return;
    }
    if !stats.riding {
        return;
    }
    let medal = has_medal(&stats, &inv);
    if tick_time(&mut stats, medal, time.delta_secs()) {
        log.push(time.elapsed_secs(), C_INFO, "Binicilik süren doldu, attan indin.");
    }
}

/// Binekteyken saldırı / beceri denenirse attan in (Metin2: at üstünde saldırı yok).
fn dismount_on_attack(
    time: Res<Time>,
    auto: Res<AutoAttack>,
    mut casts: MessageReader<CastSkill>,
    mut stats: ResMut<PlayerStats>,
    mut log: ResMut<GameLog>,
) {
    let cast = casts.read().count() > 0;
    if !stats.riding {
        return;
    }
    // Otomatik saldırı açıldıysa (düşmana sağ tık) ya da beceri basıldıysa in.
    if cast || (auto.is_changed() && auto.0) {
        stats.riding = false;
        log.push(time.elapsed_secs(), C_INFO, "Saldırmak için attan indin.");
    }
}

/// Binince atı oyuncunun altına koy, inince kaldır (gövdeyi önce oyuncuya geri taşı).
fn sync_steed(
    mut commands: Commands,
    stats: Res<PlayerStats>,
    mut wa: Option<ResMut<WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    server: Res<AssetServer>,
    player: Query<Entity, With<Player>>,
    steeds: Query<Entity, With<Steed>>,
    bodies: Query<(Entity, &ChildOf), With<Body>>,
) {
    let Ok(p) = player.single() else { return };
    let have = steeds.iter().next();
    match (stats.riding, have) {
        (true, None) => {
            let Some(wa) = wa.as_deref_mut() else { return };
            let mut wm = WowModel::new(load_m2(&server, STEED_M2));
            wm.skins[0] = wa.texture(STEED_SKIN, (true, true), &mut images);
            commands.spawn((wm, AnimState::default(), Steed, Transform::default(), Visibility::default(), ChildOf(p)));
            info!("mount: bindi");
        }
        (false, Some(s)) => {
            for (b, parent) in &bodies {
                if parent.parent() != p {
                    commands.entity(b).insert((ChildOf(p), Transform::default()));
                }
            }
            commands.entity(s).despawn();
            info!("mount: indi");
        }
        _ => {}
    }
}

/// Gövdeyi atın binici noktasına oturt (gövde yeniden kurulunca da).
fn seat_rider(
    mut commands: Commands,
    stats: Res<PlayerStats>,
    steeds: Query<(Entity, &ModelReady), With<Steed>>,
    bodies: Query<(Entity, &ChildOf), With<Body>>,
    mut warned: Local<bool>,
) {
    if !stats.riding {
        return;
    }
    let Ok((steed, ready)) = steeds.single() else { return };
    let (joint, offset) = match ready.attachment(RIDER_ATTACH) {
        Some(a) => a,
        None => {
            if !*warned {
                *warned = true;
                let ids: Vec<u16> = ready.attachments.iter().map(|a| a.id).collect();
                warn!("mount: binici takma noktası {RIDER_ATTACH} yok (var olanlar: {ids:?})");
            }
            (steed, Vec3::Y * 1.3)
        }
    };
    for (b, parent) in &bodies {
        if parent.parent() != joint {
            commands.entity(b).insert((ChildOf(joint), Transform::from_translation(offset)));
        }
    }
}

fn animate_steed(player: Query<&Player>, mut steeds: Query<&mut AnimState, With<Steed>>) {
    let (Ok(p), Ok(mut st)) = (player.single(), steeds.single_mut()) else { return };
    st.set_loop(if p.moving { anim::RUN } else { anim::STAND });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::stats::derive;

    fn rich(level: u32) -> (PlayerStats, Inventory) {
        let s = PlayerStats { level, ..Default::default() };
        (s, Inventory::default())
    }

    #[test]
    fn ticket_gives_ten_minutes_and_runs_out() {
        let (mut s, mut inv) = rich(1);
        assert!(inv.add(Item::new(items::V_RIDE_TICKET)).is_none());
        use_ticket(&mut s, &mut inv).unwrap();
        assert!(s.riding);
        assert_eq!(s.mount_left, 600.0);
        assert_eq!(inv.count(items::V_RIDE_TICKET), 0);
        assert!(!tick_time(&mut s, false, 599.0));
        assert!(tick_time(&mut s, false, 2.0));
        assert!(!s.riding);
        assert_eq!(s.mount_left, 0.0);
    }

    #[test]
    fn toggle_keeps_remaining_time() {
        let (mut s, mut inv) = rich(1);
        inv.add(Item::new(items::V_RIDE_TICKET));
        use_ticket(&mut s, &mut inv).unwrap();
        tick_time(&mut s, false, 100.0);
        toggle(&mut s, &inv).unwrap();
        assert!(!s.riding);
        // İnikken süre azalmaz.
        tick_time(&mut s, false, 100.0);
        assert_eq!(s.mount_left, 500.0);
        toggle(&mut s, &inv).unwrap();
        assert!(s.riding);
    }

    #[test]
    fn no_mount_without_ticket_or_medal() {
        let (mut s, inv) = rich(30);
        assert!(toggle(&mut s, &inv).is_err());
        assert!(!s.riding);
    }

    #[test]
    fn medal_needs_level_and_is_timeless() {
        let (mut s, mut inv) = rich(20);
        inv.add(Item::new(items::V_HORSE_MEDAL));
        assert!(toggle(&mut s, &inv).is_err());
        s.level = 25;
        toggle(&mut s, &inv).unwrap();
        assert!(s.riding);
        assert!(!tick_time(&mut s, true, 10_000.0));
        assert!(s.riding);
    }

    #[test]
    fn riding_is_faster() {
        let (mut s, inv) = rich(1);
        let walk = derive(&s, &inv).move_speed;
        s.riding = true;
        let ride = derive(&s, &inv).move_speed;
        assert!((ride / walk - SPEED_MUL).abs() < 1e-4, "{walk} -> {ride}");
    }

    #[test]
    fn mount_state_survives_save() {
        let (mut s, mut inv) = rich(1);
        inv.add(Item::new(items::V_RIDE_TICKET));
        use_ticket(&mut s, &mut inv).unwrap();
        tick_time(&mut s, false, 42.0);
        let d = crate::game::save::SaveData::snapshot(&s, &inv, &Default::default(), None);
        let back = crate::game::save::SaveData::parse(&d.to_ron().unwrap()).unwrap();
        assert!(back.stats.riding);
        assert_eq!(back.stats.mount_left, 558.0);
    }

    #[test]
    fn clock_format() {
        assert_eq!(clock(600.0), "10:00");
        assert_eq!(clock(61.2), "1:02");
        assert_eq!(clock(-3.0), "0:00");
    }
}
