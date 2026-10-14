//! Eşya tanımları ve eşya örnekleri. Eşyalar WoW 1.12'den: gerçek WoW eşya adları (Türkçe çeviri),
//! WoW ikonları (`Interface/Icons`) ve ItemDisplayInfo görünümleri. 1.12 istemcisinde eşya → görünüm
//! tablosu yok (sunucudan gelir); kesin eşleşmeler: Aşınmış Kısa Kılıç (1542), Külgetiren/Ashbringer
//! (23875, `Sword_2H_Ashbringer`) ve zırh setleri (doku adlarından: `Plate_DungeonWarrior_A` = Valor,
//! `Plate_RaidWarrior_A/B/C` = Might/Wrath/Dreadnaught, `Plate_A_01Crusader` = Kızıl Haçlı). Diğer silahlarda
//! aynı türden bir WoW modeli seçildi. Oyun seviyesi 40'ta bittiği için WoW seviyeleri sıkıştırıldı.

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
            EquipSlot::Earring => "Yüzük",
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
    /// Koruma Parşömeni: başarısızlıkta eşya yok olmaz, bir seviye düşer.
    Blessing,
    /// Yoğun Bileme Taşı: +%10 başarı.
    Handbook,
    /// Onyxia Pulu (nadir): +%20 başarı ve koruma.
    DragonScale,
    /// Binicilik Bileti: kullanınca `secs` saniyelik binek süresi (Seyis verir).
    MountTicket { secs: u32 },
    /// Ölüm Atı Dizginleri: süresiz binek (Sv25).
    MountMedal,
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
pub const V_RIDE_TICKET: u32 = 50050;
pub const V_HORSE_MEDAL: u32 = 50051;

pub const ITEMS: &[ItemDef] = &[
    // --- Silahlar (WoW 1.12; ad, tür ve sıra WoW'dan, seviyeler 40'a sıkıştırıldı) ---
    weapon(10, "Aşınmış Kısa Kılıç", "INV_Sword_04", 1, (14, 18), 3, 1542, false, 50), // Worn Shortsword
    weapon(15, "Bakır Topuz", "INV_Mace_01", 3, (17, 22), 3, 1161, false, 120), // Copper Mace
    weapon(20, "Gladius", "INV_Sword_04", 5, (20, 26), 4, 1930, false, 200), // Gladius
    weapon(25, "Bakır Balta", "INV_Axe_01", 7, (24, 31), 4, 1386, false, 400), // Copper Axe
    weapon(30, "Pala", "INV_Sword_24", 10, (28, 36), 5, 5040, false, 600), // Cutlass
    weapon(35, "Klaymor", "INV_Sword_08", 12, (40, 52), 7, 563, true, 900), // Claymore
    weapon(40, "Yatağan", "INV_Sword_04", 15, (36, 46), 6, 1592, false, 1500), // Scimitar
    weapon(45, "Savaş Baltası", "INV_Axe_04", 17, (54, 70), 8, 782, true, 2200), // Battle Axe
    weapon(50, "Diken Kılıcı", "INV_Sword_05", 20, (46, 58), 7, 2806, false, 3000), // Thornblade
    weapon(55, "Kasırga Baltası", "INV_Axe_02", 22, (70, 90), 10, 3232, true, 4000), // Whirlwind Axe
    weapon(60, "Dal'Rend'in Kutsal Hücumu", "INV_Sword_40", 25, (58, 72), 8, 20009, false, 6000), // Dal'Rend's Sacred Charge
    weapon(65, "Demirdüşman", "INV_Mace_07", 27, (62, 78), 9, 31861, false, 8000), // Ironfoe
    weapon(70, "Arkanit Biçici", "INV_Axe_12", 30, (80, 104), 11, 31694, true, 12000), // Arcanite Reaper
    weapon(75, "Quel'Serrar", "INV_Sword_57", 32, (74, 92), 10, 33729, false, 16000), // Quel'Serrar
    weapon(80, "Külgetiren", "INV_Sword_48", 35, (100, 130), 14, 23875, true, 25000), // Ashbringer
    // --- Zırh setleri (savaşçı plaka; 8 yuva: gömlek, göğüs, kemer, pantolon, çizme, bileklik, eldiven, tabard) ---
    armor(11200, "Kızıl Haçlı Zırhı", "INV_Chest_Chain_05", 1, 12, 3, // Chain of the Scarlet Crusade
        [None, Some(5317), Some(5319), Some(5320), Some(5321), Some(5322), Some(5323), Some(3865)], 100),
    armor(11210, "Cesaret Savaş Takımı", "INV_Chest_Plate03", 9, 24, 5, // Battlegear of Valor (T0)
        [None, Some(29958), Some(29959), Some(29963), Some(29960), Some(29961), Some(29962), None], 2000),
    armor(11220, "Kudret Savaş Takımı", "INV_Chest_Plate16", 18, 40, 7, // Battlegear of Might (T1)
        [None, Some(31021), Some(31019), Some(31023), Some(31025), Some(31020), Some(31022), None], 8000),
    armor(11230, "Gazap Savaş Takımı", "INV_Chest_Plate16", 26, 62, 9, // Battlegear of Wrath (T2)
        [None, Some(33628), Some(33632), Some(33630), Some(33631), Some(33627), Some(33629), None], 20000),
    armor(11240, "Korkusuz Savaş Takımı", "INV_Chest_Plate02", 34, 80, 11, // Dreadnaught's Battlegear (T3)
        [None, Some(35049), Some(35058), Some(35192), Some(35193), Some(35044), Some(35194), None], 32000),
    // --- Miğfer / kalkan / çizme ---
    wear(12200, "Barbar Demir Miğferi", "INV_Helmet_02", 1, EquipSlot::Helmet, 4, 0, 1, 0, 80), // Barbaric Iron Helm
    wear(12210, "Aslanyürek Miğferi", "INV_Helmet_13", 15, EquipSlot::Helmet, 10, 0, 2, 0, 1500), // Lionheart Helm
    wear(13000, "Aşınmış Tahta Kalkan", "INV_Shield_09", 1, EquipSlot::Shield, 5, 0, 2, 0, 90), // Worn Wooden Shield
    wear(13010, "Kafatası Alevi Kalkanı", "INV_Shield_04", 21, EquipSlot::Shield, 14, 0, 3, 0, 2500), // Skullflame Shield
    wear(15000, "Acemi Çizmeleri", "INV_Boots_01", 1, EquipSlot::Shoes, 2, 0, 1, 0, 60), // Recruit's Boots
    wear(15010, "Barbar Demir Çizmeleri", "INV_Boots_Plate_03", 20, EquipSlot::Shoes, 6, 0, 2, 0, 1800), // Barbaric Iron Boots
    // --- Bileklik / kolye / yüzük ---
    wear(14000, "Bakır Bileklikler", "INV_Bracer_07", 1, EquipSlot::Bracelet, 0, 40, 15, 0, 60), // Copper Bracers
    wear(14010, "Kızıl Bilek Koruyucuları", "INV_Bracer_03", 18, EquipSlot::Bracelet, 0, 160, 30, 0, 1500), // Scarlet Wristguards
    wear(14020, "Cesaret Bileklikleri", "INV_Bracer_18", 24, EquipSlot::Bracelet, 0, 240, 40, 0, 3000), // Bracers of Valor
    wear(16000, "Myzrael'in Kolyesi", "INV_Jewelry_Necklace_04", 1, EquipSlot::Necklace, 0, 60, 20, 0, 60), // Pendant of Myzrael
    wear(16005, "Canavarın Gözü", "INV_Jewelry_Necklace_09", 9, EquipSlot::Necklace, 0, 110, 25, 0, 600), // Eye of the Beast
    wear(16010, "Fordring'in Nişanı", "INV_Jewelry_Amulet_02", 27, EquipSlot::Necklace, 0, 260, 45, 0, 4000), // Mark of Fordring
    wear(16020, "Karanlıkay Muskası", "INV_Jewelry_Necklace_07", 30, EquipSlot::Necklace, 0, 320, 50, 0, 6000), // Amulet of the Darkmoon
    wear(16030, "Onyxia Dişi Kolyesi", "INV_Jewelry_Necklace_05", 33, EquipSlot::Necklace, 0, 380, 55, 0, 9000), // Onyxia Tooth Pendant
    wear(17000, "Karataş Yüzüğü", "INV_Jewelry_Ring_01", 1, EquipSlot::Earring, 1, 20, 10, 0, 60), // Blackstone Ring
    wear(17005, "Hızlı Darbe Yüzüğü", "INV_Jewelry_Ring_15", 9, EquipSlot::Earring, 2, 50, 15, 0, 600), // Quick Strike Ring
    wear(17010, "Don Julio'nun Yüzüğü", "INV_Jewelry_Ring_17", 21, EquipSlot::Earring, 3, 100, 20, 0, 2500), // Don Julio's Band
    wear(17020, "Accuria Yüzüğü", "INV_Jewelry_Ring_24", 27, EquipSlot::Earring, 4, 140, 25, 0, 4000), // Band of Accuria
    wear(17030, "Magni'nin İradesi", "INV_Jewelry_Ring_25", 30, EquipSlot::Earring, 5, 170, 30, 0, 6000), // Magni's Will
    // --- İksirler ---
    misc(V_RED_POT_S, "Hafif Şifa İksiri", "INV_Potion_51", Kind::Potion { hp: 100, sp: 0 }, 200, 10), // Lesser Healing Potion
    misc(V_RED_POT_M, "Şifa İksiri", "INV_Potion_52", Kind::Potion { hp: 300, sp: 0 }, 200, 40), // Healing Potion
    misc(V_RED_POT_L, "Büyük Şifa İksiri", "INV_Potion_53", Kind::Potion { hp: 800, sp: 0 }, 200, 100), // Greater Healing Potion
    misc(V_BLUE_POT_S, "Küçük Mana İksiri", "INV_Potion_70", Kind::Potion { hp: 0, sp: 80 }, 200, 10), // Minor Mana Potion
    // --- Canavar düşürmeleri / yükseltme malzemeleri (WoW ticaret malları) ---
    misc(V_WOLF_CLAW, "Keskin Pençe", "INV_Misc_MonsterClaw_04", Kind::Material, 200, 15), // Sharp Claw
    misc(V_WOLF_PELT, "Hafif Deri", "INV_Misc_LeatherScrap_03", Kind::Material, 200, 20), // Light Leather
    misc(V_BOAR_TUSK, "Kırık Yaban Domuzu Dişi", "INV_Misc_Bone_08", Kind::Material, 200, 15), // Chipped Boar Tusk
    misc(V_BOAR_PELT, "Orta Deri", "INV_Misc_LeatherScrap_05", Kind::Material, 200, 20), // Medium Leather
    misc(V_BEAR_PAW, "Ağır Deri", "INV_Misc_LeatherScrap_07", Kind::Material, 200, 30), // Heavy Leather
    misc(V_BEAR_GALL, "Büyük Ayı Eti", "INV_Misc_Food_14", Kind::Material, 200, 40), // Big Bear Meat
    misc(V_TIGER_PELT, "Kalın Deri", "INV_Misc_LeatherScrap_08", Kind::Material, 200, 60), // Thick Leather
    misc(V_WHITE_RIBBON, "Tuhaf Toz", "INV_Enchant_DustStrange", Kind::Material, 200, 50), // Strange Dust
    misc(V_SPIRIT_STONE, "Ağır Taş", "INV_Stone_09", Kind::Material, 200, 100), // Heavy Stone
    misc(V_SOUL_CRYSTAL, "Arkanit Kristali", "INV_Misc_Gem_Sapphire_01", Kind::Material, 200, 400), // Arcanite Crystal
    // --- Yükseltme yardımcıları ---
    misc(V_BLESSING, "Koruma Parşömeni", "INV_Scroll_07", Kind::Blessing, 200, 500), // Scroll of Protection
    misc(V_HANDBOOK, "Yoğun Bileme Taşı", "INV_Stone_SharpeningStone_05", Kind::Handbook, 200, 800), // Dense Sharpening Stone
    misc(V_DRAGON_SCALE, "Onyxia Pulu", "INV_Misc_MonsterScales_15", Kind::DragonScale, 200, 3000), // Scale of Onyxia
    // --- Binek (Seyis) ---
    misc(V_RIDE_TICKET, "Binicilik Bileti", "INV_Misc_Ticket_Darkmoon_01", Kind::MountTicket { secs: 600 }, 20, 0),
    ItemDef {
        vnum: V_HORSE_MEDAL,
        name: "Ölüm Atı Dizginleri", // Deathcharger's Reins
        icon: "Ability_Mount_Undeadhorse",
        kind: Kind::MountMedal,
        level: 25,
        height: 1,
        stack: 1,
        price: 0,
    },
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

#[cfg(test)]
mod tests {
    use super::*;

    /// MPQ'da gerçekten bulunan dosyalar (listfile'daki ~1850 dosya MPQ'da yok).
    const PRESENT: &str = include_str!("../../tools/mpq_present.txt");

    fn present(path: &str) -> bool {
        let want = path.replace('/', "\\").to_ascii_lowercase();
        PRESENT.lines().any(|l| l.trim().to_ascii_lowercase() == want)
    }

    #[test]
    fn every_icon_is_a_wow_icon() {
        for d in ITEMS {
            let p = format!("Interface/Icons/{}.blp", d.icon);
            assert!(present(&p), "{} ikonu MPQ'da yok: {}", d.name, d.icon);
        }
    }

    #[test]
    fn vnums_unique_and_vendor_stock_defined() {
        let mut seen = std::collections::HashSet::new();
        for d in ITEMS {
            assert!(seen.insert(d.vnum), "yinelenen vnum {}", d.vnum);
        }
        for stock in [crate::game::ui::shop_ui::GENERAL, crate::game::ui::shop_ui::WEAPONS, crate::game::ui::shop_ui::ARMOR] {
            for (v, _, _) in stock {
                assert!(def(*v).is_some(), "satıcıdaki {v} tanımsız");
            }
        }
    }

    #[test]
    fn weapons_progress_by_level() {
        let w: Vec<_> = ITEMS.iter().filter(|d| matches!(d.kind, Kind::Weapon { .. })).collect();
        assert!(w.len() >= 15);
        assert!(w.windows(2).all(|p| p[0].level <= p[1].level), "silahlar seviyeye göre sıralı olmalı");
    }
}
