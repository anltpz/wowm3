//! Eşya tanımları (Metin2 esinli) ve eşya örnekleri. İkonlar WoW `Interface\Icons`'tan.

use serde::{Deserialize, Serialize};

use super::character::ArmorSet;

/// Ekipman yuvaları (Metin2 envanter üst bölümü).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipSlot {
    Weapon,
    Armor,
    Helmet,
    Shield,
    Shoes,
    Bracelet,
    Necklace,
    Earring,
}

impl EquipSlot {
    pub const ALL: [EquipSlot; 8] = [
        EquipSlot::Weapon,
        EquipSlot::Armor,
        EquipSlot::Helmet,
        EquipSlot::Shield,
        EquipSlot::Shoes,
        EquipSlot::Bracelet,
        EquipSlot::Necklace,
        EquipSlot::Earring,
    ];
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }
    pub fn name(self) -> &'static str {
        match self {
            EquipSlot::Weapon => "Silah",
            EquipSlot::Armor => "Zırh",
            EquipSlot::Helmet => "Kask",
            EquipSlot::Shield => "Kalkan",
            EquipSlot::Shoes => "Ayakkabı",
            EquipSlot::Bracelet => "Bilezik",
            EquipSlot::Necklace => "Kolye",
            EquipSlot::Earring => "Küpe",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// Silah: hasar aralığı, +başına hasar, görünüm, iki el mi.
    Weapon {
        dmg: (i32, i32),
        per_plus: i32,
        display: u32,
        two_hand: bool,
    },
    /// Zırh: savunma, +başına savunma, gövde seti.
    Armor {
        def: i32,
        per_plus: i32,
        set: ArmorSet,
    },
    /// Diğer giyilebilirler: yuva + savunma + ek HP.
    Wear {
        slot: EquipSlot,
        def: i32,
        hp: i32,
        per_plus: i32,
        display: u32,
    },
    /// İksir: anında HP / SP.
    Potion { hp: i32, sp: i32 },
    /// Yükseltme malzemesi.
    Material,
    /// Kutsama Kağıdı: başarısızlıkta eşya yok olmaz, bir seviye düşer.
    Blessing,
    /// Demircinin El Kitabı: +%10 başarı.
    Handbook,
    /// Ejderha Pulu (nadir): +%20 başarı ve koruma.
    DragonScale,
}

#[derive(Clone, Copy, Debug)]
pub struct ItemDef {
    pub vnum: u32,
    pub name: &'static str,
    pub icon: &'static str,
    pub kind: Kind,
    /// Kuşanma seviyesi.
    pub level: u32,
    /// Envanterde kapladığı satır sayısı (Metin2: 1, 2 ya da 3 hücre yüksekliği).
    pub height: u8,
    pub stack: u16,
    /// NPC'ye satış fiyatı (Yang).
    pub price: u32,
}

impl ItemDef {
    pub fn upgradable(&self) -> bool {
        matches!(
            self.kind,
            Kind::Weapon { .. } | Kind::Armor { .. } | Kind::Wear { .. }
        )
    }
    pub fn equip_slot(&self) -> Option<EquipSlot> {
        match self.kind {
            Kind::Weapon { .. } => Some(EquipSlot::Weapon),
            Kind::Armor { .. } => Some(EquipSlot::Armor),
            Kind::Wear { slot, .. } => Some(slot),
            _ => None,
        }
    }
}

const fn weapon(
    vnum: u32,
    name: &'static str,
    icon: &'static str,
    level: u32,
    dmg: (i32, i32),
    per_plus: i32,
    display: u32,
    two_hand: bool,
    price: u32,
) -> ItemDef {
    ItemDef {
        vnum,
        name,
        icon,
        kind: Kind::Weapon {
            dmg,
            per_plus,
            display,
            two_hand,
        },
        level,
        height: if two_hand { 3 } else { 2 },
        stack: 1,
        price,
    }
}

const fn armor(
    vnum: u32,
    name: &'static str,
    icon: &'static str,
    level: u32,
    def: i32,
    per_plus: i32,
    set: ArmorSet,
    price: u32,
) -> ItemDef {
    ItemDef {
        vnum,
        name,
        icon,
        kind: Kind::Armor { def, per_plus, set },
        level,
        height: 2,
        stack: 1,
        price,
    }
}

const fn wear(
    vnum: u32,
    name: &'static str,
    icon: &'static str,
    level: u32,
    slot: EquipSlot,
    def: i32,
    hp: i32,
    per_plus: i32,
    display: u32,
    price: u32,
) -> ItemDef {
    ItemDef {
        vnum,
        name,
        icon,
        kind: Kind::Wear {
            slot,
            def,
            hp,
            per_plus,
            display,
        },
        level,
        height: if matches!(slot, EquipSlot::Shield) { 2 } else { 1 },
        stack: 1,
        price,
    }
}

const fn misc(
    vnum: u32,
    name: &'static str,
    icon: &'static str,
    kind: Kind,
    stack: u16,
    price: u32,
) -> ItemDef {
    ItemDef {
        vnum,
        name,
        icon,
        kind,
        level: 0,
        height: 1,
        stack,
        price,
    }
}

// Malzeme ve özel eşya kimlikleri (yükseltme tarifleri bunlara bakar).
pub const V_RED_POT_S: u32 = 27001;
pub const V_RED_POT_M: u32 = 27002;
pub const V_RED_POT_L: u32 = 27003;
pub const V_BLUE_POT_S: u32 = 27004;
pub const V_WOLF_CLAW: u32 = 30001;
pub const V_BOAR_TUSK: u32 = 30002;
/// Eski adı Kobold Kulağı; Pyungmoo'da Ayı Safrası.
pub const V_KOBOLD_EAR: u32 = 30003;
pub const V_BEAR_GALL: u32 = V_KOBOLD_EAR;
pub const V_WOLF_PELT: u32 = 30004;
pub const V_BEAR_PAW: u32 = 30005;
pub const V_TIGER_PELT: u32 = 30006;
pub const V_WHITE_RIBBON: u32 = 30007;
pub const V_BOAR_PELT: u32 = 30008;
pub const V_SPIRIT_STONE: u32 = 30010;
pub const V_SOUL_CRYSTAL: u32 = 30011;
pub const V_BLESSING: u32 = 25040;
pub const V_HANDBOOK: u32 = 25041;
pub const V_DRAGON_SCALE: u32 = 25042;

pub const ITEMS: &[ItemDef] = &[
    // --- Kılıçlar (Metin2 savaşçı, Pyungmoo düşürmeleri) ---
    weapon(10, "Kılıç", "INV_Sword_04", 1, (14, 18), 3, 1546, false, 50),
    weapon(20, "Uzun Kılıç", "INV_Sword_04", 5, (20, 26), 4, 1550, false, 200),
    weapon(30, "Hilal Kılıç", "INV_Sword_04", 10, (28, 36), 5, 1853, false, 600),
    weapon(35, "Geniş Kılıç", "INV_Sword_06", 12, (40, 52), 7, 1203, true, 900),
    weapon(40, "Bambu Kılıç", "INV_Sword_05", 15, (36, 46), 6, 3234, false, 1500),
    weapon(50, "Gümüş Kılıç", "INV_Weapon_ShortBlade_01", 20, (46, 58), 7, 4443, false, 3000),
    weapon(55, "Dokuz Pala", "INV_Sword_04", 22, (50, 64), 7, 1592, false, 4000),
    weapon(60, "Zehirli Kılıç", "INV_Sword_22", 25, (58, 72), 8, 4908, false, 6000),
    weapon(70, "Dolunay Kılıcı", "INV_Sword_06", 30, (80, 104), 11, 2416, true, 12000),
    weapon(80, "Kara Ejderha Kılıcı", "INV_Sword_07", 35, (100, 130), 14, 3758, true, 25000),
    // --- Zırhlar (Metin2 savaşçı plaka zırhları) ---
    armor(11200, "Demir Plaka Zırh", "INV_Chest_Plate05", 1, 12, 3,
        [None, Some(16184), Some(3418), Some(3414), Some(25828), Some(4448), Some(25831), None], 100),
    armor(11210, "Aslan Plaka Zırh", "INV_Chest_Plate03", 9, 24, 5,
        [None, Some(29958), Some(29959), Some(29963), Some(29960), Some(29961), Some(29962), None], 2000),
    armor(11220, "Kaplan Plaka Zırh", "INV_Chest_Plate16", 18, 40, 7,
        [None, Some(31021), Some(31019), Some(31023), Some(31025), Some(31020), Some(31022), None], 8000),
    armor(11230, "Ejderha Plaka Zırh", "INV_Chest_Plate16", 26, 62, 9,
        [None, Some(33628), Some(33632), Some(33630), Some(33631), Some(33627), Some(33629), None], 20000),
    // --- Kask / kalkan / ayakkabı ---
    wear(12200, "Geleneksel Miğfer", "INV_Helmet_03", 1, EquipSlot::Helmet, 4, 0, 1, 0, 80),
    wear(12210, "Demir Miğfer", "INV_Helmet_05", 15, EquipSlot::Helmet, 10, 0, 2, 0, 1500),
    wear(13000, "Savaş Kalkanı", "INV_Shield_04", 1, EquipSlot::Shield, 5, 0, 2, 0, 90),
    wear(13010, "Pentagon Kalkan", "INV_Shield_06", 21, EquipSlot::Shield, 14, 0, 3, 0, 2500),
    wear(15000, "Deri Ayakkabı", "INV_Boots_02", 1, EquipSlot::Shoes, 2, 0, 1, 0, 60),
    wear(15010, "Kaplan Derisi Ayakkabı", "INV_Boots_05", 20, EquipSlot::Shoes, 6, 0, 2, 0, 1800),
    // --- Takılar (Beyaz Yeminli düşürmeleri) ---
    wear(14000, "Tahta Bilezik", "INV_Jewelry_Ring_03", 1, EquipSlot::Bracelet, 0, 40, 15, 0, 60),
    wear(14010, "Gümüş Bilezik", "INV_Jewelry_Ring_05", 18, EquipSlot::Bracelet, 0, 160, 30, 0, 1500),
    wear(14020, "Altın Bilezik", "INV_Jewelry_Ring_07", 24, EquipSlot::Bracelet, 0, 240, 40, 0, 3000),
    wear(16000, "Tahta Kolye", "INV_Jewelry_Necklace_01", 1, EquipSlot::Necklace, 0, 60, 20, 0, 60),
    wear(16005, "Bakır Kolye", "INV_Jewelry_Necklace_03", 9, EquipSlot::Necklace, 0, 110, 25, 0, 600),
    wear(16010, "Abanoz Kolye", "INV_Jewelry_Necklace_07", 27, EquipSlot::Necklace, 0, 260, 45, 0, 4000),
    wear(16020, "İnci Kolye", "INV_Jewelry_Necklace_05", 30, EquipSlot::Necklace, 0, 320, 50, 0, 6000),
    wear(16030, "Beyaz Altın Kolye", "INV_Jewelry_Necklace_09", 33, EquipSlot::Necklace, 0, 380, 55, 0, 9000),
    wear(17000, "Tahta Küpe", "INV_Jewelry_Talisman_01", 1, EquipSlot::Earring, 1, 20, 10, 0, 60),
    wear(17005, "Bakır Küpe", "INV_Jewelry_Talisman_03", 9, EquipSlot::Earring, 2, 50, 15, 0, 600),
    wear(17010, "Yeşim Taşı Küpe", "INV_Jewelry_Talisman_05", 21, EquipSlot::Earring, 3, 100, 20, 0, 2500),
    wear(17020, "Abanoz Küpe", "INV_Jewelry_Talisman_07", 27, EquipSlot::Earring, 4, 140, 25, 0, 4000),
    wear(17030, "İnci Küpe", "INV_Jewelry_Talisman_09", 30, EquipSlot::Earring, 5, 170, 30, 0, 6000),
    // --- İksirler ---
    misc(V_RED_POT_S, "Kırmızı İksir (K)", "INV_Potion_51", Kind::Potion { hp: 100, sp: 0 }, 200, 10),
    misc(V_RED_POT_M, "Kırmızı İksir (B)", "INV_Potion_52", Kind::Potion { hp: 300, sp: 0 }, 200, 40),
    misc(V_RED_POT_L, "Kırmızı İksir (Y)", "INV_Potion_54", Kind::Potion { hp: 800, sp: 0 }, 200, 100),
    misc(V_BLUE_POT_S, "Mavi İksir (K)", "INV_Potion_72", Kind::Potion { hp: 0, sp: 80 }, 200, 10),
    // --- Canavar düşürmeleri / yükseltme malzemeleri (WoW ikonları) ---
    misc(V_WOLF_CLAW, "Kurt Pençesi", "INV_Misc_MonsterClaw_04", Kind::Material, 200, 15),
    misc(V_WOLF_PELT, "Kurt Postu", "INV_Misc_Pelt_Wolf_01", Kind::Material, 200, 20),
    misc(V_BOAR_TUSK, "Yaban Domuzu Dişi", "INV_Misc_Bone_08", Kind::Material, 200, 15),
    misc(V_BOAR_PELT, "Yaban Domuzu Derisi", "INV_Misc_Pelt_Boar_01", Kind::Material, 200, 20),
    misc(V_BEAR_PAW, "Ayı Ayak Derisi", "INV_Misc_Pelt_Bear_01", Kind::Material, 200, 30),
    misc(V_BEAR_GALL, "Ayı Safrası", "INV_Misc_Organ_03", Kind::Material, 200, 40),
    misc(V_TIGER_PELT, "Kaplan Postu", "INV_Misc_Pelt_04", Kind::Material, 200, 60),
    misc(V_WHITE_RIBBON, "Beyaz Domuz Kuyruğu Kurdele", "INV_Misc_Bag_07", Kind::Material, 200, 50),
    misc(V_SPIRIT_STONE, "Ruh Taşı", "INV_Stone_15", Kind::Material, 200, 100),
    misc(V_SOUL_CRYSTAL, "Ruh Kristali", "INV_Enchant_ShardBrilliantLarge", Kind::Material, 200, 400),
    // --- Özel kağıtlar ---
    misc(V_BLESSING, "Kutsama Kağıdı", "INV_Scroll_03", Kind::Blessing, 200, 500),
    misc(V_HANDBOOK, "Demircinin El Kitabı", "INV_Misc_Book_09", Kind::Handbook, 200, 800),
    misc(V_DRAGON_SCALE, "Ejderha Pulu", "INV_Misc_MonsterScales_13", Kind::DragonScale, 200, 3000),
];

/// Seviyeye uygun rastgele ekipman havuzu (canavar düşürmeleri için).
pub fn gear_for_level(level: u32) -> Vec<u32> {
    ITEMS
        .iter()
        .filter(|d| d.upgradable() && d.level + 10 >= level && d.level <= level + 3)
        .map(|d| d.vnum)
        .collect()
}

pub fn def(vnum: u32) -> Option<&'static ItemDef> {
    ITEMS.iter().find(|d| d.vnum == vnum)
}

/// Bir eşya örneği (envanterde / yerde / ekipmanda).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub vnum: u32,
    pub plus: u8,
    pub count: u16,
}

impl Item {
    pub fn new(vnum: u32) -> Self {
        Self {
            vnum,
            plus: 0,
            count: 1,
        }
    }
    pub fn stack(vnum: u32, count: u16) -> Self {
        Self {
            vnum,
            plus: 0,
            count,
        }
    }
    pub fn def(&self) -> &'static ItemDef {
        def(self.vnum).unwrap_or(&ITEMS[0])
    }
    /// Görünen ad: "Uzun Kılıç+5".
    pub fn display_name(&self) -> String {
        let d = self.def();
        if d.upgradable() {
            format!("{}+{}", d.name, self.plus)
        } else {
            d.name.to_string()
        }
    }
    pub fn icon_path(&self) -> String {
        format!("Interface\\Icons\\{}.blp", self.def().icon)
    }
}

/// Artı basma kuralları (Metin2 esinli): +N → +N+1 başarı yüzdesi.
pub const UPGRADE_CHANCE: [u32; 9] = [95, 90, 80, 70, 60, 50, 40, 30, 20];
pub const MAX_PLUS: u8 = 9;

/// +N → +N+1 için gereken Yang ve malzemeler.
pub fn upgrade_cost(item: &Item) -> (u32, Vec<(u32, u16)>) {
    let d = item.def();
    let lvl = item.plus as u32;
    let base = 300 + d.level * 60;
    let yang = base * (lvl + 1) * (lvl + 1);
    let mut mats = Vec::new();
    match item.plus {
        0..=1 => {}
        2..=3 => mats.push((V_WOLF_CLAW, 2 + item.plus as u16)),
        4 => {
            mats.push((V_WOLF_PELT, 3));
            mats.push((V_BOAR_TUSK, 3));
        }
        5 => {
            mats.push((V_BEAR_PAW, 3));
            mats.push((V_SPIRIT_STONE, 1));
        }
        6 => {
            mats.push((V_BEAR_GALL, 3));
            mats.push((V_SPIRIT_STONE, 2));
        }
        7 => {
            mats.push((V_TIGER_PELT, 3));
            mats.push((V_SPIRIT_STONE, 2));
            mats.push((V_SOUL_CRYSTAL, 1));
        }
        _ => {
            mats.push((V_WHITE_RIBBON, 4));
            mats.push((V_SOUL_CRYSTAL, 2));
        }
    }
    (yang, mats)
}

/// Eşyanın "parlaklık" rengi (Metin2: +7 üstü parlayan silahlar).
pub fn plus_glow(plus: u8) -> Option<[f32; 3]> {
    match plus {
        0..=3 => None,
        4..=5 => Some([0.6, 0.6, 0.7]),
        6 => Some([0.2, 0.5, 1.0]),
        7 => Some([0.3, 0.7, 1.6]),
        8 => Some([1.0, 0.3, 1.6]),
        _ => Some([1.8, 0.4, 0.2]),
    }
}
