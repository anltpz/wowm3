//! Metin2 ikonik eşya kataloğu — TASLAK (D-21 dolduracak): Hilal, Bambu, Gümüş, Zehir, Dolunay, Ejderha
//! Kılıcı... Ad/seviye/istatistik tr-wiki'den, görünüm en benzer WoW modeli (ItemDisplayInfo).
//!
//! `items::ITEM_TABLES` bu tabloyu zaten içerir: `items::def(vnum)`, düşürme havuzu (`gear_for_level`) ve
//! testler (vnum benzersizliği, MPQ ikon denetimi) buradaki satırları da kapsar. Yeni eşyalar yalnızca buraya
//! eklenir; `items.rs`'e dokunulmaz. Kurucular: `weapon`, `armor`, `wear`, `misc` (`items.rs`).

#[allow(unused_imports)]
use super::items::{armor, misc, wear, weapon, EquipSlot, ItemDef, Kind};

// --- Zindan giriş biletleri ve anahtarları (G-26; 71000+ aralığı zindanlara ayrıldı) ---
/// Örümcek Zindanı giriş bileti (girişte 1 adet tüketilir; rehber satar).
pub const V_TICKET_SPIDER: u32 = 71000;
/// Şeytan Kulesi giriş bileti.
pub const V_TICKET_TOWER: u32 = 71001;
/// Şeytan Kulesi mühür anahtarı (anahtar katında taşıyıcılardan düşer, kat bitince tüketilir).
pub const V_TOWER_KEY: u32 = 71002;

pub const ITEMS_METIN2: &[ItemDef] = &[
    // Zindan biletleri / anahtarları (G-26).
    misc(V_TICKET_SPIDER, "Örümcek Zindanı Bileti", "INV_Misc_Note_02", Kind::Material, 20, 500),
    misc(V_TICKET_TOWER, "Şeytan Kulesi Bileti", "INV_Misc_Note_04", Kind::Material, 20, 2_500),
    misc(V_TOWER_KEY, "Kule Mühür Anahtarı", "INV_Misc_Key_03", Kind::Material, 1, 0),
];
