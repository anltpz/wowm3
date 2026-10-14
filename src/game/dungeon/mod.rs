//! Metin2 tarzı katlı zindanlar — TASLAK. G-26 çekirdeği (durum makinesi, `defs`/`arena`/`entry` alt modülleri)
//! bu dosyada kurar; `game/mod.rs` bu modülü ve `plugin`'i zaten kaydetti, oraya dokunulmaz.
//!
//! Paralel işler (Dalga 2) kendi dosyalarında çalışsın diye alt modüller önceden açıldı:
//! - `spider` (G-28, Örümcek Zindanı) ve `tower` (G-29, Şeytan Kulesi): `MOBS` tabloları
//!   `mobs::MOB_TABLES` içinde kayıtlı; metin taşı için `metin::spawn_metin`.
//! - `atmos` (D-23): sis / loş ışık / portal efektleri.
//!
//! Paylaşılan kancalar: `physics::INDOOR` (içerideyken arazi kısa yolu ve +60 m yedek ışın kapalı),
//! `npc::ELWYNN_NPC_TABLES` (zindan rehberi NPC'si için `NPCS`).

use bevy::prelude::*;

use super::npc::NpcDef;

pub mod atmos;
pub mod spider;
pub mod tower;

/// Zindan girişindeki NPC'ler (Elwynn haritasında doğar; WoW x/y/z).
pub const NPCS: &[NpcDef] = &[];

pub fn plugin(app: &mut App) {
    app.add_plugins((atmos::plugin, spider::plugin, tower::plugin));
}
