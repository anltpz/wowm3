//! Metin2 ikonik eşya kataloğu — TASLAK (D-21 dolduracak): Hilal, Bambu, Gümüş, Zehir, Dolunay, Ejderha
//! Kılıcı... Ad/seviye/istatistik tr-wiki'den, görünüm en benzer WoW modeli (ItemDisplayInfo).
//!
//! `items::ITEM_TABLES` bu tabloyu zaten içerir: `items::def(vnum)`, düşürme havuzu (`gear_for_level`) ve
//! testler (vnum benzersizliği, MPQ ikon denetimi) buradaki satırları da kapsar. Yeni eşyalar yalnızca buraya
//! eklenir; `items.rs`'e dokunulmaz. Kurucular: `weapon`, `armor`, `wear`, `misc` (`items.rs`).

#[allow(unused_imports)]
use super::items::{armor, misc, wear, weapon, EquipSlot, ItemDef, Kind};

pub const ITEMS_METIN2: &[ItemDef] = &[];
