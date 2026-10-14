//! Örümcek Zindanı — TASLAK (G-28): Sv 10-25, `MD_Spidermine.wmo`, 3 kat (öldür → metin → boss),
//! boss Örümcek Kraliçesi, Zehir Kılıcı düşürür.
//!
//! G-26 çekirdeği bu dosyadan yalnızca `DUNGEON` ve `MOBS` okur (`defs::DUNGEONS`, `mobs::MOB_TABLES`).
//! Aşağıdaki `DUNGEON` API'yi gösteren YER TUTUCUDUR: moblar Elwynn örümcekleri (210, 225); G-28 kendi 4xx
//! moblarını `MOBS`'a yazıp kat tablosunu değiştirir. Giriş noktası `entry: None` ise otomatik seçilir
//! (log'daki `arena: giriş` satırı); sabitlemek için WMO yerel WoW koordinatı yazılır.

use bevy::prelude::*;

use super::defs::{instance_origin, DungeonDef, FloorDef, Objective};
use crate::game::items_metin2::V_TICKET_SPIDER;
use crate::game::mobs::MobDef;

/// Zindan yaratıkları (`mobs::MOB_TABLES` içinde kayıtlı; kimlikler 4xx önerilir).
pub const MOBS: &[MobDef] = &[];

/// Instance slotu (`instance_origin(SLOT, kat)`); her zindanın kendi slotu var.
const SLOT: u32 = 1;
const WMO: &str = r"World\wmo\Dungeon\MD_Spidermine\MD_Spidermine.wmo";

pub const DUNGEON: DungeonDef = DungeonDef {
    id: 1,
    key: "spider",
    name: "Örümcek Zindanı",
    level: (10, 99),
    ticket: V_TICKET_SPIDER,
    ticket_price: 2_000,
    cooldown_secs: 30 * 60,
    time_limit_secs: 30.0 * 60.0,
    floors: &[
        FloorDef {
            name: "1. Kat: Örümcek Yuvası",
            wmo: WMO,
            offset: instance_origin(SLOT, 0),
            yaw: 0.0,
            entry: None,
            mobs: &[(210, 10)],
            objective: Objective::KillAll,
        },
        FloorDef {
            name: "2. Kat: Ağlarla Örülü Tüneller",
            wmo: WMO,
            offset: instance_origin(SLOT, 1),
            yaw: 0.0,
            entry: None,
            mobs: &[(210, 4)],
            objective: Objective::BreakMetin { metin: 1, count: 2, summons: &[210] },
        },
        FloorDef {
            name: "3. Kat: Kraliçenin İni",
            wmo: WMO,
            offset: instance_origin(SLOT, 2),
            yaw: 0.0,
            entry: None,
            mobs: &[],
            objective: Objective::Boss { id: 225, adds: &[(210, 4)] },
        },
    ],
};

pub fn plugin(_app: &mut App) {}
