//! Oyuncu istatistikleri: seviye, EXP, HP/SP ve ekipmandan türeyen saldırı/savunma.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::inventory::Inventory;
use super::items::Kind;

pub const MAX_LEVEL: u32 = 40;

#[derive(Resource, Clone, Serialize, Deserialize)]
/// Kayıtta eksik alanlar varsayılandan gelir (eski save.ron uyumu).
#[serde(default)]
pub struct PlayerStats {
    pub name: String,
    pub level: u32,
    pub exp: u64,
    pub hp: i32,
    pub sp: i32,
    /// Son hasar alma zamanı (yenilenme gecikmesi için), saniye.
    #[serde(skip)]
    pub last_hurt: f32,
    #[serde(skip)]
    pub dead: bool,
    /// Öfke (berserk) bitiş zamanı, saniye.
    #[serde(skip)]
    pub rage_until: f32,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            name: "Savaşçı".into(),
            level: 1,
            exp: 0,
            hp: 280,
            sp: 100,
            last_hurt: -100.0,
            dead: false,
            rage_until: -1.0,
        }
    }
}

/// Seviye için gereken EXP.
pub fn exp_needed(level: u32) -> u64 {
    let l = level as u64;
    60 * l * l + 140 * l
}

/// Ekipman + seviyeden hesaplanan değerler.
#[derive(Resource, Clone, Copy, Default, Debug)]
pub struct Derived {
    pub max_hp: i32,
    pub max_sp: i32,
    pub atk_min: i32,
    pub atk_max: i32,
    pub def: i32,
    pub two_hand: bool,
    /// Saldırı aralığı (sn).
    pub swing: f32,
    pub move_speed: f32,
}

pub fn derive(stats: &PlayerStats, inv: &Inventory) -> Derived {
    let l = stats.level as i32;
    let mut d = Derived {
        max_hp: 200 + 80 * l,
        max_sp: 80 + 20 * l,
        atk_min: 4 + 2 * l,
        atk_max: 6 + 2 * l,
        def: l,
        two_hand: false,
        swing: 0.95,
        move_speed: 7.0,
    };
    for it in inv.equip.iter().flatten() {
        let p = it.plus as i32;
        match it.def().kind {
            Kind::Weapon {
                dmg,
                per_plus,
                two_hand,
                ..
            } => {
                d.atk_min += dmg.0 + per_plus * p;
                d.atk_max += dmg.1 + per_plus * p;
                d.two_hand = two_hand;
                if two_hand {
                    d.swing = 1.2;
                }
            }
            Kind::Armor { def, per_plus, .. } => d.def += def + per_plus * p,
            Kind::Wear {
                def,
                hp,
                per_plus,
                slot,
                ..
            } => {
                d.def += def;
                if hp > 0 {
                    d.max_hp += hp + per_plus * p;
                } else {
                    d.def += per_plus * p;
                }
                if matches!(slot, super::items::EquipSlot::Shoes) {
                    d.move_speed += 0.15 * p as f32;
                }
            }
            _ => {}
        }
    }
    d
}

pub fn plugin(app: &mut App) {
    app.init_resource::<PlayerStats>()
        .init_resource::<Derived>()
        .add_systems(Update, (update_derived, regen).chain());
}

fn update_derived(time: Res<Time>, stats: Res<PlayerStats>, inv: Res<Inventory>, mut d: ResMut<Derived>) {
    let mut n = derive(&stats, &inv);
    if time.elapsed_secs() < stats.rage_until {
        n.atk_min = n.atk_min * 5 / 4;
        n.atk_max = n.atk_max * 5 / 4;
        n.swing /= 1.3;
        n.move_speed *= 1.1;
    }
    if n.max_hp != d.max_hp || n.atk_min != d.atk_min || n.swing != d.swing || n.def != d.def
        || n.max_sp != d.max_sp || n.move_speed != d.move_speed || n.atk_max != d.atk_max
        || n.two_hand != d.two_hand
    {
        *d = n;
    }
}

/// Savaş dışı (5 sn hasar almadan) HP/SP yenilenmesi.
fn regen(
    time: Res<Time>,
    d: Res<Derived>,
    mut stats: ResMut<PlayerStats>,
    mut acc: Local<f32>,
) {
    if stats.dead {
        return;
    }
    if stats.hp > d.max_hp && d.max_hp > 0 {
        stats.hp = d.max_hp;
    }
    if stats.sp > d.max_sp && d.max_sp > 0 {
        stats.sp = d.max_sp;
    }
    *acc += time.delta_secs();
    if *acc < 1.0 {
        return;
    }
    *acc = 0.0;
    let out_of_combat = time.elapsed_secs() - stats.last_hurt > 5.0;
    let hp_rate = if out_of_combat { 0.03 } else { 0.005 };
    if stats.hp < d.max_hp {
        stats.hp = (stats.hp + (d.max_hp as f32 * hp_rate).ceil() as i32).min(d.max_hp);
    }
    if stats.sp < d.max_sp {
        stats.sp = (stats.sp + (d.max_sp as f32 * 0.02).ceil() as i32).min(d.max_sp);
    }
}

/// EXP ekler, seviye atlatır; atlanan seviye sayısını döner.
pub fn gain_exp(stats: &mut PlayerStats, amount: u64) -> u32 {
    if stats.level >= MAX_LEVEL {
        return 0;
    }
    stats.exp += amount;
    let mut ups = 0;
    while stats.level < MAX_LEVEL && stats.exp >= exp_needed(stats.level) {
        stats.exp -= exp_needed(stats.level);
        stats.level += 1;
        ups += 1;
    }
    ups
}
