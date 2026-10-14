//! Şeytan Kulesi — TASLAK (G-29): Sv 30-40, 4 kat (öldür / metin / anahtar / boss), ara boss DoomGuard,
//! son boss Şeytan Kral; Dolunay ve Ejderha Kılıcı düşürür.
//!
//! G-26 çekirdeği bu dosyadan yalnızca `DUNGEON` ve `MOBS` okur. Aşağıdaki `DUNGEON` API'yi (dört hedef türünün
//! hepsi) gösteren YER TUTUCUDUR: moblar Elwynn Defias'ları (216, 217) ve Garrick Padfoot (206); G-29 kendi 5xx
//! moblarını `MOBS`'a yazar, kat WMO'larını (`MD_Crypt*`, boss katı `Blackrock_lower_guild`) seçer.

use bevy::prelude::*;

use super::defs::{instance_origin, DungeonDef, FloorDef, Objective};
use crate::game::items_metin2::{V_TICKET_TOWER, V_TOWER_KEY};
use crate::game::mobs::MobDef;

/// Zindan yaratıkları (`mobs::MOB_TABLES` içinde kayıtlı; kimlikler 5xx önerilir).
pub const MOBS: &[MobDef] = &[];

const SLOT: u32 = 2;

pub const DUNGEON: DungeonDef = DungeonDef {
    id: 2,
    key: "tower",
    name: "Şeytan Kulesi",
    level: (30, 99),
    ticket: V_TICKET_TOWER,
    ticket_price: 10_000,
    cooldown_secs: 60 * 60,
    time_limit_secs: 45.0 * 60.0,
    floors: &[
        FloorDef {
            name: "1. Kat: Lanetli Giriş",
            wmo: r"World\wmo\Dungeon\MD_Crypt\MD_Crypt.wmo",
            offset: instance_origin(SLOT, 0),
            yaw: 0.0,
            entry: None,
            mobs: &[(216, 6), (217, 4)],
            objective: Objective::KillAll,
        },
        FloorDef {
            name: "2. Kat: Metin Salonu",
            wmo: r"World\wmo\Dungeon\MD_Crypt\MD_Crypt_C.wmo",
            offset: instance_origin(SLOT, 1),
            yaw: 0.0,
            entry: None,
            mobs: &[(217, 3)],
            objective: Objective::BreakMetin { metin: 3, count: 2, summons: &[216, 217] },
        },
        FloorDef {
            name: "3. Kat: Mühürlü Kapı",
            wmo: r"World\wmo\Dungeon\MD_Crypt\MD_Crypt_B.wmo",
            offset: instance_origin(SLOT, 2),
            yaw: 0.0,
            entry: None,
            mobs: &[(216, 4), (217, 4)],
            objective: Objective::FindKey { carriers: &[217], key_vnum: V_TOWER_KEY, chance: 0.2 },
        },
        FloorDef {
            name: "4. Kat: Şeytan Kral'ın Tahtı",
            wmo: r"World\wmo\Dungeon\MD_Crypt\MD_Crypt_D.wmo",
            offset: instance_origin(SLOT, 3),
            yaw: 0.0,
            entry: None,
            mobs: &[],
            objective: Objective::Boss { id: 206, adds: &[(217, 3)] },
        },
    ],
};

pub fn plugin(_app: &mut App) {}
