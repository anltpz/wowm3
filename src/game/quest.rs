//! Basit zincir görevi: "Metin Taşlarını Yok Et (n/hedef)". Her aşamada ödül, sonraki aşama zorlaşır.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::combat::Killed;
use super::inventory::Inventory;
use super::items::{Item, V_BLESSING, V_HANDBOOK, V_SOUL_CRYSTAL};
use super::log::{GameLog, C_GOOD};
use super::metin::Metin;
use super::stats::{gain_exp, PlayerStats};

#[derive(Resource, Clone, Serialize, Deserialize)]
/// Kayıtta eksik alanlar varsayılandan gelir (eski save.ron uyumu).
#[serde(default)]
pub struct Quest {
    pub stage: u32,
    pub kills: u32,
}

impl Default for Quest {
    fn default() -> Self {
        Self { stage: 1, kills: 0 }
    }
}

impl Quest {
    pub fn goal(&self) -> u32 {
        3 + self.stage * 2
    }
    pub fn text(&self) -> String {
        format!(
            "Görev {}: Metin Taşlarını Yok Et ({}/{})\nÖdül: {} Yang, Kutsama Kağıdı",
            self.stage,
            self.kills,
            self.goal(),
            super::ui::thousands(self.reward_yang())
        )
    }
    fn reward_yang(&self) -> u64 {
        3000 * self.stage as u64 * self.stage as u64
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Quest>().add_systems(Update, count);
}

fn count(
    time: Res<Time>,
    mut killed: MessageReader<Killed>,
    metins: Query<(), With<Metin>>,
    mut q: ResMut<Quest>,
    mut inv: ResMut<Inventory>,
    mut stats: ResMut<PlayerStats>,
    mut log: ResMut<GameLog>,
) {
    let now = time.elapsed_secs();
    for k in killed.read() {
        if metins.get(k.entity).is_err() {
            continue;
        }
        q.kills += 1;
        if q.kills >= q.goal() {
            let yang = q.reward_yang();
            inv.yang += yang;
            inv.add(Item::stack(V_BLESSING, (1 + q.stage / 2) as u16));
            if q.stage >= 2 {
                inv.add(Item::new(V_HANDBOOK));
            }
            if q.stage >= 3 {
                inv.add(Item::stack(V_SOUL_CRYSTAL, 2));
            }
            let exp = 800 * q.stage as u64 * q.stage as u64;
            gain_exp(&mut stats, exp);
            log.banner(now, C_GOOD, format!("Görev {} tamamlandı!", q.stage));
            log.push(now, C_GOOD, format!("Görev ödülü: {} Yang, +{} EXP", super::ui::thousands(yang), exp));
            q.stage += 1;
            q.kills = 0;
        }
    }
}
