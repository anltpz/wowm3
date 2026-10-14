//! Şeytan Kulesi — TASLAK (G-29): Sv 30-40, 4 kat (öldür / metin / anahtar / boss), ara boss DoomGuard,
//! son boss Şeytan Kral; Dolunay ve Ejderha Kılıcı düşürür.

use bevy::prelude::*;

use crate::game::mobs::MobDef;

/// Zindan yaratıkları (`mobs::MOB_TABLES` içinde kayıtlı; kimlikler 5xx önerilir).
pub const MOBS: &[MobDef] = &[];

pub fn plugin(_app: &mut App) {}
