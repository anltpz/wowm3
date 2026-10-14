//! Westfall (Sv 10-20) — TASLAK (G-27 dolduracak). Konumlar WoW dünya koordinatı (x kuzey, y batı, z yukarı),
//! `elwynn.rs` deseniyle MCNK areaId / MODF ölçümünden.
//!
//! Kayıt noktaları (bu dosya dışında değişiklik gerekmez):
//! - `ZONE` → `zones::ZONES` listesinde zaten kayıtlı (alt bölgeler, yerleşimler, metin alanları, dalga çağrıları).
//! - `MOBS` → `mobs::MOB_TABLES` içinde kayıtlı (yeni mob kimlikleri: 3xx önerilir).
//! - `NPCS` → `npc::ELWYNN_NPC_TABLES` içinde kayıtlı (WoW x/y/z, `npc::ELWYNN_NPCS` biçimi).
//! - `plugin` → `game::GamePlugin` içinde kayıtlı (bölgeye özgü sistem gerekirse).

use bevy::prelude::*;

use super::mobs::MobDef;
use super::npc::NpcDef;
use super::zones::{MetinSlotRow, SpawnRow, Subzone, ZoneDef};

/// Bölge kaydı (`zones::ZONES`). Kutu yaklaşık; G-27 ölçümle düzeltir. Tablolar boşken hiçbir şey değişmez.
pub const ZONE: ZoneDef = ZoneDef {
    name: "Westfall",
    bounds: [-11800.0, -10150.0, 300.0, 2400.0],
    subzones: SUBZONES,
    spawns: SPAWNS,
    metin_slots: METIN_SLOTS,
    metin_summons,
};

/// Alt bölgeler (mini harita adı): (ad, x, y, yarıçap). Küçükler önce.
pub const SUBZONES: &[Subzone] = &[];

/// Yaratık yerleşimi (`zones::SpawnRow`).
pub const SPAWNS: &[SpawnRow] = &[];

/// Metin taşı alanları (`zones::MetinSlotRow`).
pub const METIN_SLOTS: &[MetinSlotRow] = &[];

/// Westfall yaratıkları (`mobs::m`, `mobs::boss`, `mobs::human`, `mobs::with_body` ile).
pub const MOBS: &[MobDef] = &[];

/// Westfall NPC'leri.
pub const NPCS: &[NpcDef] = &[];

/// Metin taşı dalgalarında çağrılan Westfall yaratıkları (metin türüne göre).
pub fn metin_summons(_kind: usize) -> &'static [u32] {
    &[]
}

pub fn plugin(_app: &mut App) {}
