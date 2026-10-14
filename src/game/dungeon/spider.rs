//! Örümcek Zindanı — TASLAK (G-28): Sv 10-25, `MD_Spidermine.wmo`, 3 kat (öldür → metin → boss),
//! boss Örümcek Kraliçesi, Zehir Kılıcı düşürür.

use bevy::prelude::*;

use crate::game::mobs::MobDef;

/// Zindan yaratıkları (`mobs::MOB_TABLES` içinde kayıtlı; kimlikler 4xx önerilir).
pub const MOBS: &[MobDef] = &[];

pub fn plugin(_app: &mut App) {}
