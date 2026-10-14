//! Metin2 ikonik silah kataloğu (D-21). Kullanıcı kararı: **Metin2 adı + en benzer WoW 1.12 modeli**.
//!
//! - Ad, tür (Kılıçlar = tek el, Çifteller = iki el) ve seviye tr-wiki'nin "Savaşçı Silahlar" sayfasından
//!   (tr-wiki.metin2.gameforge.com). Dolunay Kılıcı tek eldir (Kılıçlar bölümü, Sv30).
//! - Seviye sıkıştırma: Metin2 Sv120 → oyun Sv40, yani **oyun seviyesi = ⌈Metin2 seviyesi / 3⌉**
//!   (Dolunay 30 → 10, Zehir 75 → 25, Kara Ejderha 96 → 32, Yılan 120 → 40).
//! - Hasar `items::ITEMS` silah eğrisine göre: sıradan kılıçlar aynı seviyedeki WoW silahıyla başa baş,
//!   boss ödülleri (Dolunay ve üstü, iki elliler) aynı seviyedeki WoW silahından ~%10-20 güçlü. Metin2'deki
//!   karakter korunur: Geniş Kılıç ve Yarımay geniş aralıklı, Dolunay kendi seviyesinde belirgin güçlü.
//! - Görünüm: wiki'deki oyun içi görsellerle WoW modelleri yan yana (geçici silah galerisi) karşılaştırılarak
//!   seçildi; hepsinin modeli + dokusu MPQ'da var. `Sword_1H_Fire_A_01` (eski 4908 "Zehir") KULLANILMAZ:
//!   alev ağzı parçacık olduğu için bizim çizicide yalnızca kabza görünüyor.
//! - vnum aralığı 19000-19999 (tek el 190x0, iki el 195x0); `items::ITEMS` ile çakışmaz.
//!
//! `items::ITEM_TABLES` bu tabloyu zaten içerir: `items::def(vnum)`, düşürme havuzu (`gear_for_level`) ve
//! testler (vnum benzersizliği, MPQ ikon denetimi) buradaki satırları da kapsar. Yeni eşyalar yalnızca buraya
//! eklenir; `items.rs`'e dokunulmaz. Kurucular: `weapon`, `armor`, `wear`, `misc` (`items.rs`).
//!
//! Boss düşürme önerisi (tabloları G-28/G-29 yazar):
//! - Örümcek Zindanı (Sv 10-25): kat ara bossu → [`V_DOLUNAY`] / [`V_KIRMIZI_DEMIR_PALA`];
//!   Örümcek Kraliçesi → [`V_ZEHIR`] (Metin2'de de Örümcek Barones düşürür) / [`V_KIN`]; nadir [`V_YARIMAY`].
//! - Şeytan Kulesi (Sv 30-40): ara boss DoomGuard → [`V_KARA_EJDERHA`] / [`V_KARA_EJDERHA_BICAGI`];
//!   son boss Şeytan Kral → [`V_YILAN`] / [`V_YILAN_BUYUK`] (en üst kademe, Sv40).
//! - Sıradan Metin2 kılıçları (Kılıç…Gümüş) normal düşürme havuzunda kalır ([`BOSS_ONLY`] dışında).

#[allow(unused_imports)]
use super::items::{armor, misc, wear, weapon, EquipSlot, ItemDef, Kind};

// --- Sıradan Metin2 kılıçları (tek el; metin taşı / canavar düşürmesi) ---
/// Kılıç (Metin2 Sv1).
pub const V_KILIC: u32 = 19010;
/// Uzun Kılıç (Metin2 Sv5).
pub const V_UZUN_KILIC: u32 = 19020;
/// Hilal Kılıç (Metin2 Sv10).
pub const V_HILAL: u32 = 19030;
/// Bambu Kılıcı (Metin2 Sv15).
pub const V_BAMBU: u32 = 19040;
/// Geniş Kılıç (Metin2 Sv20).
pub const V_GENIS: u32 = 19050;
/// Gümüş Kılıç (Metin2 Sv25).
pub const V_GUMUS: u32 = 19060;
// --- Boss ödülleri: tek el ---
/// Dolunay Kılıcı (Metin2 Sv30, tek el).
pub const V_DOLUNAY: u32 = 19070;
/// Yarımay Kılıcı (Metin2 Sv60).
pub const V_YARIMAY: u32 = 19080;
/// Zehir Kılıcı (Metin2 Sv75).
pub const V_ZEHIR: u32 = 19090;
/// Kara Ejderha Kılıcı (Metin2 Sv96).
pub const V_KARA_EJDERHA: u32 = 19100;
/// Yılan Kılıcı (Metin2 Sv120).
pub const V_YILAN: u32 = 19110;
// --- Boss ödülleri: iki el (Metin2 "Çifteller") ---
/// Kırmızı Demir Pala (Metin2 Sv30).
pub const V_KIRMIZI_DEMIR_PALA: u32 = 19510;
/// Kin Kılıcı (Metin2 Sv75).
pub const V_KIN: u32 = 19520;
/// Kara Ejderha Bıçağı (Metin2 Sv96).
pub const V_KARA_EJDERHA_BICAGI: u32 = 19530;
/// Yılan Büyük Kılıcı (Metin2 Sv120).
pub const V_YILAN_BUYUK: u32 = 19540;

/// Yalnızca bosslardan düşmesi önerilen silahlar. `items::gear_for_level` şu an bütün tabloları
/// süzmeden kullanıyor (metin taşları da bunları düşürebilir); süzgeç gerekirse bu liste kullanılır.
#[allow(dead_code)] // G-28/G-29 düşürme tabloları ve olası `gear_for_level` süzgeci için
pub const BOSS_ONLY: &[u32] = &[
    V_DOLUNAY,
    V_YARIMAY,
    V_ZEHIR,
    V_KARA_EJDERHA,
    V_YILAN,
    V_KIRMIZI_DEMIR_PALA,
    V_KIN,
    V_KARA_EJDERHA_BICAGI,
    V_YILAN_BUYUK,
];

pub const ITEMS_METIN2: &[ItemDef] = &[
    // weapon(vnum, ad, ikon, oyun Sv, hasar, +başına, ItemDisplay, iki el, satış)
    // Görsel gerekçe her satırın sonunda: WoW modeli — Metin2 görünümü.
    // --- Tek el: Kılıçlar ---
    weapon(V_KILIC, "Kılıç", "INV_Sword_10", 1, (14, 17), 3, 16128, false, 50), // Sword_1H_Katana_B_02Black — ince, hafif kavisli, siyah kabzalı
    weapon(V_UZUN_KILIC, "Uzun Kılıç", "INV_Sword_20", 2, (16, 20), 3, 1566, false, 120), // Sword_1H_Long_B_03Black — uzun düz koyu ağız
    weapon(V_HILAL, "Hilal Kılıç", "INV_Sword_16", 4, (19, 24), 4, 1853, false, 300), // Sword_1H_Scimitar_A_02 — kavisli pala
    weapon(V_BAMBU, "Bambu Kılıcı", "INV_Sword_42", 5, (21, 26), 4, 9370, false, 400), // Sword_1H_Katana_A_01 — düz katana, bambu/ahşap tonu
    weapon(V_GENIS, "Geniş Kılıç", "INV_Sword_18", 7, (21, 33), 4, 4090, false, 700), // Sword_1H_Machete_C_01 — geniş, ucu kıvrık pala
    weapon(V_GUMUS, "Gümüş Kılıç", "INV_Sword_39", 9, (30, 38), 5, 30994, false, 1000), // Sword_1H_Long_D_03 — gümüş, rünlü, süslü balçak
    weapon(V_DOLUNAY, "Dolunay Kılıcı", "INV_Sword_21", 10, (34, 44), 6, 859, false, 2500), // Sword_1H_Machete_A_01 — geniş bronz ağız, sırtında dişler
    weapon(V_YARIMAY, "Yarımay Kılıcı", "INV_Weapon_ShortBlade_23", 20, (46, 68), 8, 22803, false, 5000), // Sword_1H_Troll_B_01 — kemik rengi hilal ağız
    weapon(V_ZEHIR, "Zehir Kılıcı", "INV_Sword_54", 25, (66, 84), 9, 20035, false, 9000), // Sword_1H_Horde_C_01Green — dalgalı, ucu kancalı, yeşil zehir oluklu
    weapon(V_KARA_EJDERHA, "Kara Ejderha Kılıcı", "INV_Sword_51", 32, (82, 110), 11, 31867, false, 18000), // Sword_1H_Blackwing_A_02 — siyah ağız, gümüş süslü balçak (Kara Kanat = kara ejderha)
    weapon(V_YILAN, "Yılan Kılıcı", "INV_Sword_52", 40, (98, 128), 13, 33097, false, 40000), // Sword_1H_Blackwing_A_01 + Dragon01 — siyah tırtıklı ağız, yeşil balçak
    // --- İki el: Çifteller ---
    weapon(V_KIRMIZI_DEMIR_PALA, "Kırmızı Demir Pala", "INV_Sword_47", 10, (42, 56), 7, 29170, true, 3000), // Sword_2H_Horde_C_03Bone — uzun sivri, paslı bronz ağız
    weapon(V_KIN, "Kin Kılıcı", "INV_Sword_53", 25, (84, 112), 12, 34304, true, 11000), // Sword_2H_AhnQiraj_D_02 — kızıl-kahve süslü ağız
    weapon(V_KARA_EJDERHA_BICAGI, "Kara Ejderha Bıçağı", "INV_Sword_50", 32, (96, 126), 13, 32000, true, 22000), // Sword_2H_Blackwing_A_02 — koyu ağız, ejder başlı balçak
    weapon(V_YILAN_BUYUK, "Yılan Büyük Kılıcı", "INV_Sword_56", 40, (136, 174), 17, 33882, true, 45000), // Sword_2H_AhnQiraj_D_01Green — siyah-yeşil, dikenli dev ağız
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_sane() {
        let mut last = [0u32; 2];
        for d in ITEMS_METIN2 {
            let Kind::Weapon { dmg, two_hand, per_plus, .. } = d.kind else {
                panic!("{}: yalnızca silah beklenir", d.name);
            };
            assert!((19000..20000).contains(&d.vnum), "{} vnum aralık dışı", d.name);
            assert!((1..=40).contains(&d.level), "{} seviyesi 1-40 dışında", d.name);
            assert!(dmg.0 > 0 && dmg.0 < dmg.1 && per_plus > 0, "{} hasarı tutarsız", d.name);
            let g = two_hand as usize;
            assert!(d.level >= last[g], "{} seviyeye göre sıralı değil", d.name);
            last[g] = d.level;
        }
        for v in BOSS_ONLY {
            assert!(ITEMS_METIN2.iter().any(|d| d.vnum == *v), "BOSS_ONLY {v} tanımsız");
        }
    }

    /// Boss ödülleri, aynı ya da daha düşük seviyedeki her WoW silahından (aynı el türü) ortalamada güçlü.
    #[test]
    fn boss_weapons_beat_wow_peers() {
        let avg = |k: Kind| match k {
            Kind::Weapon { dmg, .. } => (dmg.0 + dmg.1) as f32 / 2.0,
            _ => 0.0,
        };
        let two = |k: Kind| matches!(k, Kind::Weapon { two_hand: true, .. });
        for d in ITEMS_METIN2.iter().filter(|d| BOSS_ONLY.contains(&d.vnum)) {
            for w in super::super::items::ITEMS.iter().filter(|w| matches!(w.kind, Kind::Weapon { .. })) {
                if w.level <= d.level && two(w.kind) == two(d.kind) {
                    assert!(avg(d.kind) > avg(w.kind), "{} ({}) {} ({})'dan zayıf", d.name, d.level, w.name, w.level);
                }
            }
        }
    }
}
