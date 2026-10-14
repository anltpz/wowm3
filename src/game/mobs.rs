//! Moblar: tanımlar, sürü doğurucu, yapay zekâ (dolaş → kovala → saldır → geri dön), yeniden doğma.

use bevy::prelude::*;
use rand::Rng;

use benilla_assets::WorldAssets;
use benilla_world::collision::WorldCollision;
use benilla_world::world_point::WorldPoint;

use super::combat::{Dying, Enemy, Health, Hit, Killed, PlayerHurt};
use super::model::{anim, load_m2, AnimState, WowModel};
use super::player::Player;
use super::stats::PlayerStats;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Species {
    Dog,
    Wolf,
    Boar,
    Bear,
    Tiger,
    /// Beyaz Yeminli askerler / Defias: zırhlı insan modeli.
    Human,
    Kobold,
    Murloc,
    Spider,
    Gnoll,
    /// İri gnoll (Hogger): GnollMelee modeli.
    GnollBrute,
    /// Başka dosyalarda tanımlanan türler (Westfall, zindanlar): model ve gövde `MobDef::body`'den gelir
    /// (`with_body` ile verilmesi şart; test denetler).
    #[allow(dead_code)]
    Other,
}

/// Türden bağımsız model + çarpışma gövdesi (dış tablolar için). `skin` yine `MobDef::skin`'den gelir.
#[derive(Clone, Copy, Debug)]
pub struct MobBody {
    /// M2 yolu, ör. "Creature/HarvestGolem/HarvestGolem.m2".
    pub model: &'static str,
    /// Hedef/çarpışma yarıçapı ve boyu (ölçek uygulanmadan).
    pub radius: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct MobDef {
    pub id: u32,
    /// Metin2'deki Türkçe adı (tr-wiki Pyungmoo listesi).
    pub name: &'static str,
    pub species: Species,
    pub skin: &'static str,
    pub scale: f32,
    pub level: u32,
    pub hp: i32,
    pub atk: i32,
    pub def: i32,
    pub exp: u32,
    pub speed: f32,
    /// "Aç ..." türleri saldırgandır; normal ve "Korkak" türler vurulunca saldırır.
    pub aggressive: bool,
    pub tint: [f32; 3],
    /// Mini patron (büyük, bol ganimet).
    pub boss: bool,
    /// İnsan türleri için zırh seti ve silah görünümü.
    pub gear: Option<(super::character::ArmorSet, u32)>,
    /// Türün model/gövde verisinin yerine geçer (None → `species`'e göre).
    pub body: Option<MobBody>,
    /// Türe özgü düşürmelerin yerine geçer: (vnum, şans). None → `species`'e göre.
    pub loot: Option<&'static [(u32, f32)]>,
}

use super::items::{
    V_BANDANA, V_BEAR_GALL, V_BEAR_PAW, V_BOAR_PELT, V_BOAR_TUSK, V_CANDLE, V_GNOLL_PAW, V_LINEN, V_MURLOC_FIN,
    V_SPIDER_LEG, V_TIGER_PELT, V_WHITE_RIBBON, V_WOLF_CLAW, V_WOLF_PELT,
};

impl MobDef {
    pub fn model(&self) -> &'static str {
        if let Some(b) = self.body {
            return b.model;
        }
        match self.species {
            Species::Dog | Species::Wolf => "Creature/Wolf/Wolf.m2",
            Species::Boar => "Creature/Boar/Boar.m2",
            Species::Bear => "Creature/bear/Bear.m2",
            Species::Tiger => "Creature/Tiger/Tiger.m2",
            Species::Human => "Character/Human/Male/HumanMale.m2",
            Species::Kobold => "Creature/Kobold/Kobold.m2",
            Species::Murloc => "Creature/Murloc/Murloc.m2",
            Species::Spider => "Creature/MineSpider/MineSpider.m2",
            Species::Gnoll => "Creature/Gnoll/gnoll.m2",
            Species::GnollBrute => "Creature/GnollMelee/GnollMelee.m2",
            // `body` verilmemiş dış tür: görünür bir yedek.
            Species::Other => "Creature/Wolf/Wolf.m2",
        }
    }
    /// Hedef/çarpışma gövdesi (yarıçap, boy), ölçeksiz.
    pub fn collision(&self) -> (f32, f32) {
        if let Some(b) = self.body {
            return (b.radius, b.height);
        }
        match self.species {
            Species::Dog | Species::Wolf => (0.6, 1.3),
            Species::Boar => (0.6, 1.2),
            Species::Bear => (0.9, 1.9),
            Species::Tiger => (0.7, 1.3),
            Species::Human => (0.45, 1.9),
            Species::Kobold => (0.45, 1.4),
            Species::Murloc => (0.45, 1.4),
            Species::Spider => (0.8, 1.0),
            Species::Gnoll => (0.5, 1.9),
            Species::GnollBrute => (0.6, 2.3),
            Species::Other => (0.6, 1.5),
        }
    }
    /// Türe özgü düşürmeler (WoW ikonlu Metin2 malzemeleri): (vnum, şans).
    pub fn drops(&self) -> &'static [(u32, f32)] {
        if let Some(l) = self.loot {
            return l;
        }
        match self.species {
            Species::Dog => &[(V_WOLF_CLAW, 0.25)],
            Species::Wolf => &[(V_WOLF_CLAW, 0.35), (V_WOLF_PELT, 0.25)],
            Species::Boar => &[(V_BOAR_TUSK, 0.35), (V_BOAR_PELT, 0.2)],
            Species::Bear => &[(V_BEAR_PAW, 0.3), (V_BEAR_GALL, 0.15)],
            Species::Tiger => &[(V_TIGER_PELT, 0.3), (V_WOLF_CLAW, 0.2)],
            Species::Human => &[(V_WHITE_RIBBON, 0.2), (V_LINEN, 0.35), (V_BANDANA, 0.15)],
            Species::Kobold => &[(V_LINEN, 0.3), (V_CANDLE, 0.35)],
            Species::Murloc => &[(V_MURLOC_FIN, 0.35), (V_LINEN, 0.2)],
            Species::Spider => &[(V_SPIDER_LEG, 0.4)],
            Species::Gnoll | Species::GnollBrute => &[(V_GNOLL_PAW, 0.35), (V_LINEN, 0.3)],
            Species::Other => &[(V_LINEN, 0.3)],
        }
    }
}

/// Wiki'deki EXP'lere yakın formül (Ayı Sv12 → ~174, Asker Sv18 → ~366).
const fn exp_of(l: u32) -> u32 {
    l * l + l * 2 + 6
}
const fn hp_of(l: u32) -> i32 {
    (50 + l * 30 + l * l * 2) as i32
}
const fn atk_of(l: u32) -> i32 {
    (8 + l * 4) as i32
}

/// Mob tanımı kurucusu (seviyeden HP/saldırı/EXP). Dış tablolar (westfall.rs, dungeon/) da kullanır.
pub(crate) const fn m(
    id: u32,
    name: &'static str,
    species: Species,
    skin: &'static str,
    scale: f32,
    level: u32,
    aggressive: bool,
    tint: [f32; 3],
) -> MobDef {
    MobDef {
        id,
        name,
        species,
        skin,
        scale,
        level,
        hp: hp_of(level),
        atk: atk_of(level),
        def: (level / 2) as i32,
        exp: exp_of(level),
        speed: match species {
            Species::Bear => 5.0,
            Species::Boar => 5.8,
            Species::Human | Species::Gnoll | Species::GnollBrute | Species::Murloc => 5.5,
            Species::Kobold => 5.0,
            _ => 6.5,
        },
        aggressive,
        tint,
        boss: false,
        gear: None,
        body: None,
        loot: None,
    }
}

pub(crate) const fn boss(mut d: MobDef, mult: i32) -> MobDef {
    d.boss = true;
    d.hp *= mult;
    d.atk = d.atk * 3 / 2;
    d.exp *= mult as u32;
    d.def += 5;
    d
}

pub(crate) const fn human(mut d: MobDef, set: super::character::ArmorSet, weapon: u32) -> MobDef {
    d.gear = Some((set, weapon));
    d
}

/// Türden bağımsız model/gövde verir (`Species::Other` için şart, diğer türlerde modeli değiştirir).
#[allow(dead_code)]
pub(crate) const fn with_body(mut d: MobDef, model: &'static str, radius: f32, height: f32) -> MobDef {
    d.body = Some(MobBody { model, radius, height });
    d
}

/// Türe özgü düşürmeleri değiştirir.
#[allow(dead_code)]
pub(crate) const fn with_loot(mut d: MobDef, loot: &'static [(u32, f32)]) -> MobDef {
    d.loot = Some(loot);
    d
}

pub(crate) const W: [f32; 3] = [1.0, 1.0, 1.0];
pub(crate) const HUNGRY: [f32; 3] = [0.9, 0.85, 0.85];
pub(crate) const BLUE: [f32; 3] = [0.65, 0.8, 1.35];
pub(crate) const GREY: [f32; 3] = [1.55, 1.55, 1.6];
pub(crate) const CURSED: [f32; 3] = [1.15, 0.55, 1.35];
pub(crate) const WOLF_T: &str = "Creature/Wolf/WolfSkinTimber.blp";
pub(crate) const WOLF_TA: &str = "Creature/Wolf/WolfSkinTimberAlpha.blp";
pub(crate) const WOLF_A: &str = "Creature/Wolf/WolfSkinArctic.blp";
pub(crate) const WOLF_AA: &str = "Creature/Wolf/WolfSkinArcticAlpha.blp";
pub(crate) const WOLF_B: &str = "Creature/Wolf/WolfSkinBlack.blp";
pub(crate) const WOLF_BA: &str = "Creature/Wolf/WolfSkinBlackAlpha.blp";
pub(crate) const COYOTE: &str = "Creature/Wolf/WolfSkinCoyote.blp";
pub(crate) const BOAR_B: &str = "Creature/Boar/BoarSkinBrown.blp";
pub(crate) const BOAR_R: &str = "Creature/Boar/BoarSkinCrimson.blp";
pub(crate) const BOAR_ARM: &str = "Creature/Boar/BoarSkinBrownArmored.blp";
pub(crate) const BEAR_BR: &str = "Creature/bear/BearSkinBrown.blp";
pub(crate) const BEAR_DB: &str = "Creature/bear/BearSkinDrkBrown.blp";
pub(crate) const BEAR_BL: &str = "Creature/bear/BearSkinBlack.blp";
pub(crate) const TIGER_Y: &str = "Creature/Tiger/TigerSkinYellow.blp";
pub(crate) const TIGER_W: &str = "Creature/Tiger/TigerSkinWhite.blp";
pub(crate) const TIGER_BS: &str = "Creature/Tiger/TigerSkinBlackStriped.blp";

pub(crate) const WOLF_D: &str = "Creature/Wolf/WolfSkinDiseased.blp";
pub(crate) const BOAR_I: &str = "Creature/Boar/BoarSkinIvory.blp";
pub(crate) const KOBOLD: &str = "Creature/Kobold/koboldskinNormal.blp";
pub(crate) const KOBOLD_A: &str = "Creature/Kobold/koboldskinAlbino.blp";
pub(crate) const MURLOC: &str = "Creature/Murloc/Sahauginskin.blp";
pub(crate) const MURLOC_B: &str = "Creature/Murloc/SahauginskinBlue.blp";
pub(crate) const SPIDER_G: &str = "Creature/MineSpider/MineSpiderSkinGreen.blp";
pub(crate) const SPIDER_B: &str = "Creature/MineSpider/MineSpiderSkinBlood.blp";
pub(crate) const GNOLL_RP: &str = "Creature/Gnoll/RiverPawGnollWhelpSkin.blp";
pub(crate) const GNOLL_HOGGER: &str = "Creature/GnollMelee/RiverPawGnollFighterSkin.blp";

pub(crate) type Set = super::character::ArmorSet;
/// Defias deri takımı (Leather_A_03DefiasBrown: göğüs, kemer, pantolon, çizme).
pub(crate) const DEFIAS: Set = [None, Some(8550), Some(8551), Some(8552), Some(8553), None, None, None];
/// Beyaz Yeminli zırh setleri (gömlek, göğüs, kemer, pantolon, çizme, bileklik, eldiven, tabard).
pub(crate) const OATH_SOLDIER: Set = [None, Some(16184), Some(3418), Some(3414), Some(25828), Some(4448), Some(25831), None];
pub(crate) const OATH_ARCHER: Set = [None, Some(2183), None, Some(1882), Some(2198), None, None, None];
pub(crate) const OATH_CMD: Set = [None, Some(29958), Some(29959), Some(29963), Some(29960), Some(29961), Some(29962), None];
pub(crate) const OATH_GEN: Set = [None, Some(33628), Some(33632), Some(33630), Some(33631), Some(33627), Some(33629), None];

/// Pyungmoo canavarları (tr-wiki.metin2.gameforge.com/index.php/Pyungmoo), seviyeler wiki'den.
pub const MOBS: &[MobDef] = &[
    m(101, "Yabani Köpek", Species::Dog, COYOTE, 0.75, 1, false, W),
    m(102, "Aç Yabani Köpek", Species::Dog, COYOTE, 0.75, 1, true, HUNGRY),
    m(103, "Kurt", Species::Wolf, WOLF_T, 0.95, 3, false, W),
    m(104, "Aç Kurt", Species::Wolf, WOLF_T, 0.95, 3, true, HUNGRY),
    m(105, "Alfa Kurt", Species::Wolf, WOLF_TA, 1.1, 4, false, W),
    m(106, "Aç Alfa Kurt", Species::Wolf, WOLF_TA, 1.1, 4, true, HUNGRY),
    m(107, "Mavi Kurt", Species::Wolf, WOLF_A, 1.0, 6, false, BLUE),
    m(108, "Aç Mavi Kurt", Species::Wolf, WOLF_A, 1.0, 6, true, BLUE),
    m(109, "Yaban Domuzu", Species::Boar, BOAR_B, 0.9, 7, false, W),
    m(110, "Aç Yaban Domuzu", Species::Boar, BOAR_B, 0.9, 7, true, HUNGRY),
    m(111, "Alfa Mavi Kurt", Species::Wolf, WOLF_AA, 1.15, 9, false, BLUE),
    m(112, "Aç Alfa Mavi Kurt", Species::Wolf, WOLF_AA, 1.15, 9, true, BLUE),
    m(113, "Kırmızı Yaban Domuzu", Species::Boar, BOAR_R, 1.0, 10, false, W),
    m(114, "Aç Kırmızı Yaban Domuzu", Species::Boar, BOAR_R, 1.0, 10, true, W),
    m(115, "Ayı", Species::Bear, BEAR_BR, 0.85, 12, false, W),
    m(116, "Aç Ayı", Species::Bear, BEAR_BR, 0.85, 12, true, HUNGRY),
    m(117, "Gri Kurt", Species::Wolf, WOLF_B, 1.05, 13, false, GREY),
    m(118, "Aç Gri Kurt", Species::Wolf, WOLF_B, 1.05, 13, true, GREY),
    m(119, "Boz Ayı", Species::Bear, BEAR_DB, 0.95, 15, false, [1.15, 1.1, 1.05]),
    m(120, "Aç Boz Ayı", Species::Bear, BEAR_DB, 0.95, 15, true, [1.15, 1.1, 1.05]),
    m(121, "Alfa Gri Kurt", Species::Wolf, WOLF_BA, 1.2, 16, false, GREY),
    m(122, "Aç Alfa Gri Kurt", Species::Wolf, WOLF_BA, 1.2, 16, true, GREY),
    m(123, "Kaplan", Species::Tiger, TIGER_Y, 1.0, 18, false, W),
    m(124, "Aç Kaplan", Species::Tiger, TIGER_Y, 1.0, 18, true, W),
    human(m(125, "Korkak Beyaz Yeminli Asker", Species::Human, "", 1.0, 18, false, W), OATH_SOLDIER, 1550),
    human(m(126, "Beyaz Yeminli Asker", Species::Human, "", 1.0, 18, true, W), OATH_SOLDIER, 1550),
    m(127, "Siyah Ayı", Species::Bear, BEAR_BL, 1.05, 19, false, W),
    m(128, "Aç Siyah Ayı", Species::Bear, BEAR_BL, 1.05, 19, true, W),
    human(m(129, "Korkak Beyaz Yeminli Okçu", Species::Human, "", 1.0, 20, false, W), OATH_ARCHER, 1853),
    human(m(130, "Beyaz Yeminli Okçu", Species::Human, "", 1.0, 20, true, W), OATH_ARCHER, 1853),
    m(131, "Kahverengi Ayı", Species::Bear, BEAR_BR, 1.15, 21, false, [1.1, 0.9, 0.8]),
    m(132, "Aç Kahverengi Ayı", Species::Bear, BEAR_BR, 1.15, 21, true, [1.1, 0.9, 0.8]),
    m(133, "Beyaz Kaplan", Species::Tiger, TIGER_W, 1.1, 24, false, W),
    m(134, "Aç Beyaz Kaplan", Species::Tiger, TIGER_W, 1.1, 24, true, W),
    human(m(135, "Korkak Beyaz Yeminli Komutan", Species::Human, "", 1.05, 25, false, W), OATH_CMD, 2416),
    human(m(136, "Beyaz Yeminli Komutan", Species::Human, "", 1.05, 25, true, W), OATH_CMD, 2416),
    human(m(137, "Beyaz Yeminli General", Species::Human, "", 1.15, 25, true, W), OATH_GEN, 3758),
    // --- Mini patronlar (wiki "Mini Bosslar") ---
    boss(m(190, "Lanetli Kırmızı Yaban Domuzu", Species::Boar, BOAR_R, 1.5, 15, true, CURSED), 6),
    boss(m(191, "Lanetli Alfa Gri Kurt", Species::Wolf, WOLF_BA, 1.6, 21, true, CURSED), 6),
    boss(m(192, "Lanetli Kahverengi Ayı", Species::Bear, BEAR_BR, 1.5, 26, true, CURSED), 6),
    boss(m(193, "Lanetli Beyaz Kaplan", Species::Tiger, TIGER_W, 1.6, 29, true, CURSED), 6),
    boss(m(194, "Lykos", Species::Wolf, WOLF_AA, 2.1, 30, true, [1.2, 1.2, 1.4]), 10),
    boss(m(195, "Scrofa", Species::Boar, BOAR_ARM, 2.0, 31, true, W), 10),
    boss(m(196, "Bera", Species::Bear, BEAR_BL, 1.9, 33, true, [0.8, 0.75, 0.9]), 10),
    boss(m(197, "Tigris", Species::Tiger, TIGER_BS, 2.0, 35, true, W), 10),
    // --- Elwynn Forest (WoW 1.12) ---
    m(201, "Genç Kurt", Species::Wolf, WOLF_T, 0.7, 1, false, W), // Young Wolf
    m(202, "Kobold Haşaratı", Species::Kobold, KOBOLD, 0.85, 1, true, W), // Kobold Vermin
    m(203, "Kobold İşçisi", Species::Kobold, KOBOLD, 0.95, 3, true, W), // Kobold Worker
    m(204, "Kobold Amelesi", Species::Kobold, KOBOLD, 1.0, 4, true, W), // Kobold Laborer
    human(m(205, "Defias Haydudu", Species::Human, "", 1.0, 3, true, W), DEFIAS, 2708), // Defias Thug
    human(boss(m(206, "Garrick Padfoot", Species::Human, "", 1.05, 5, true, W), 4), DEFIAS, 1542),
    m(207, "Taşdiş Yaban Domuzu", Species::Boar, BOAR_B, 0.8, 5, false, W), // Stonetusk Boar
    m(208, "Orman Kurdu", Species::Wolf, WOLF_T, 0.9, 5, false, W), // Timber Wolf
    m(209, "Uyuz Kurt", Species::Wolf, WOLF_D, 0.9, 6, true, W), // Mangy Wolf
    m(210, "Orman Örümceği", Species::Spider, SPIDER_G, 0.75, 5, true, W), // Forest Spider
    m(211, "Kobold Tünelcisi", Species::Kobold, KOBOLD, 1.05, 7, true, W), // Kobold Tunneler
    m(212, "Kobold Madencisi", Species::Kobold, KOBOLD, 1.05, 7, true, W), // Kobold Miner
    m(213, "Kobold Jeomansörü", Species::Kobold, KOBOLD_A, 1.0, 8, true, W), // Kobold Geomancer
    m(214, "Murloc Akıntıkoşucusu", Species::Murloc, MURLOC, 0.9, 6, true, W), // Murloc Streamrunner
    m(215, "Murloc Toplayıcısı", Species::Murloc, MURLOC_B, 1.0, 9, true, W), // Murloc Forager
    human(m(216, "Defias Yankesicisi", Species::Human, "", 1.0, 6, true, W), DEFIAS, 2708), // Defias Cutpurse
    human(m(217, "Defias Eşkıyası", Species::Human, "", 1.0, 9, true, W), DEFIAS, 1542), // Defias Bandit
    boss(m(218, "Princess", Species::Boar, BOAR_ARM, 1.4, 9, false, W), 4), // Brackwell'in domuzu
    m(219, "Kayaderi Yaban Domuzu", Species::Boar, BOAR_I, 0.95, 7, false, W), // Rockhide Boar
    m(220, "Sinsi Kurt", Species::Wolf, WOLF_B, 1.0, 9, true, W), // Prowler
    m(221, "Genç Orman Ayısı", Species::Bear, BEAR_BR, 0.7, 8, false, W), // Young Forest Bear
    m(222, "Gri Orman Kurdu", Species::Wolf, WOLF_A, 1.0, 8, true, GREY), // Gray Forest Wolf
    m(223, "Riverpaw Gnollü", Species::Gnoll, GNOLL_RP, 1.0, 10, true, W), // Riverpaw Gnoll
    boss(m(224, "Hogger", Species::GnollBrute, GNOLL_HOGGER, 1.25, 11, true, W), 8),
    boss(m(225, "Mother Fang", Species::Spider, SPIDER_B, 1.4, 10, true, W), 6),
];

/// Bütün mob tabloları: Pyungmoo + Elwynn (`MOBS`), Westfall, zindanlar. Tablolar kendi dosyalarında
/// (`westfall::MOBS`, `dungeon::spider::MOBS`, `dungeon::tower::MOBS`); yeni tablo = buraya bir satır.
pub const MOB_TABLES: &[&[MobDef]] = &[
    MOBS,
    super::westfall::MOBS,
    super::dungeon::spider::MOBS,
    super::dungeon::tower::MOBS,
];

/// Kayıtlı bütün mob tanımları.
pub fn all_mobs() -> impl Iterator<Item = &'static MobDef> {
    MOB_TABLES.iter().flat_map(|t| t.iter())
}

pub fn find(id: u32) -> Option<&'static MobDef> {
    all_mobs().find(|m| m.id == id)
}

pub fn mob(id: u32) -> &'static MobDef {
    find(id).unwrap_or(&MOBS[0])
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Ai {
    Idle { wait: f32 },
    Wander { to: Vec3 },
    Chase,
    Return,
}

#[derive(Component)]
pub struct Mob {
    pub def: &'static MobDef,
    pub home: Vec3,
    pub ai: Ai,
    pub cd: f32,
    pub hit_at: Option<f32>,
    /// Sürüye bağlı (yeniden doğma için); metin çağırdıysa None.
    pub pack: Option<Entity>,
    pub aggro: bool,
}

/// Edilgen mob (model galerisi `AZM_MOBGAL`): yerinde durur, saldırmaz (QA H6).
#[derive(Component)]
pub struct Passive;

/// Bir sürü yuvası: ölen üyeleri bir süre sonra yeniden doğurur.
#[derive(Component)]
pub struct Pack {
    pub mob: u32,
    pub size: usize,
    pub center: Vec3,
    pub alive: usize,
    pub respawn: f32,
}

#[derive(Resource, Default)]
pub struct Spawner {
    /// Bekleyen sürü yerleştirmeleri (Elwynn): (mob, boyut, WoW merkez, yarıçap). Arazi akınca yerleşir.
    pending: Vec<(u32, usize, [f32; 3], f32)>,
    init: bool,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Spawner>().add_systems(
        Update,
        (place_packs, respawn_packs, mob_ai, mob_hit_feedback, mob_killed).chain(),
    );
}

/// Bir noktanın açık arazi olup olmadığını sınar: ışın zemini terrain yüksekliğiyle örtüşmeli
/// (çatı, ağaç tepesi değil) ve başlangıç yüksekliğinden çok farklı olmamalı.
pub fn open_ground(
    col: &WorldCollision,
    wp: &WorldPoint,
    p: Vec3,
    ref_y: f32,
    max_dy: f32,
) -> Option<Vec3> {
    if let Some(d) = super::terrain::data() {
        let _ = (ref_y, max_dy);
        let m = d.to_map(p);
        return d.open(m).then(|| d.to_world(m.x, m.y));
    }
    let terrain = wp.terrain_height_under(p)?;
    let origin = Vec3::new(p.x, terrain + 40.0, p.z);
    let hit = col.ray_body(origin, Dir3::NEG_Y, 80.0)?;
    let g = origin.y - hit.distance;
    if (g - terrain).abs() > 0.5 || (terrain - ref_y).abs() > max_dy {
        return None;
    }
    // Eğim kontrolü: çevre noktalar da yakın yükseklikte.
    for (dx, dz) in [(2.0, 0.0), (-2.0, 0.0), (0.0, 2.0), (0.0, -2.0)] {
        let t = wp.terrain_height_under(p + Vec3::new(dx, 0.0, dz))?;
        if (t - terrain).abs() > 1.2 {
            return None;
        }
    }
    Some(Vec3::new(p.x, terrain, p.z))
}

pub fn spawn_mob(
    commands: &mut Commands,
    server: &AssetServer,
    wa: &mut WorldAssets,
    images: &mut Assets<Image>,
    db: Option<&super::character::CharDb>,
    def: &'static MobDef,
    at: Vec3,
    pack: Option<Entity>,
    aggro: bool,
) -> Entity {
    let mut tool = None;
    let mut wm = match (def.gear, db) {
        (Some((set, weapon)), Some(db)) => {
            tool = Some(weapon);
            super::character::build_body(db, wa, images, server, &set)
        }
        _ => {
            let mut wm = WowModel::new(load_m2(server, def.model()));
            wm.skins[0] = wa.texture(def.skin, (true, true), images);
            wm
        }
    };
    wm.tint = Color::linear_rgb(def.tint[0], def.tint[1], def.tint[2]);
    let yaw = rand::rng().random_range(0.0..std::f32::consts::TAU);
    let (radius, height) = def.collision();
    let mut e = commands.spawn((
        wm,
        AnimState::default(),
        Transform::from_translation(at)
            .with_rotation(Quat::from_rotation_y(yaw))
            .with_scale(Vec3::splat(def.scale)),
        Visibility::default(),
        Enemy {
            name: def.name.to_string(),
            level: def.level,
            radius,
            height,
            exp: def.exp,
            def: def.def,
            is_metin: false,
        },
        Health {
            hp: def.hp,
            max: def.hp,
        },
        Mob {
            def,
            home: at,
            ai: if aggro { Ai::Chase } else { Ai::Idle { wait: 1.0 } },
            cd: 1.0,
            hit_at: None,
            pack,
            aggro,
        },
    ));
    if tool.is_some() {
        e.insert(super::npc::WantsTool(tool));
    }
    e.id()
}


// ---------- Pyungmoo canavar bölgeleri (wiki "Gebiet Pyungmoo" haritasından) ----------
// Bantlar köyden dışa doğru seviye artar: köpek/kurt → kırmızı → turuncu → bej → yeşil → camgöbeği →
// mavi → Beyaz Yeminliler (kuzey ve güney uçlar).
const B_DOGS: &[u32] = &[101, 102, 103, 104];
const B_RED: &[u32] = &[104, 105, 106, 107, 108, 109];
const B_ORANGE: &[u32] = &[107, 108, 111, 112, 109];
const B_BEIGE: &[u32] = &[113, 109, 117, 118, 110];
const B_GREEN: &[u32] = &[114, 115, 116, 119, 120];
const B_TEAL: &[u32] = &[121, 123, 124, 127, 132];
const B_BLUE: &[u32] = &[131, 132, 133, 134];
const B_NAVY: &[u32] = &[125, 135];
const B_LAV: &[u32] = &[129, 130, 135];
const B_OATH: &[u32] = &[126, 130, 136, 137];

/// (yakın plan harita x, y, yarıçap x, y, bant, sürü sayısı) — "zoom" koordinatları
/// (`m = z·(0.988, 1.259) − (8.3, 15.1)` ile ana haritaya çevrilir).
const PY_ZONES: &[(f32, f32, f32, f32, &[u32], usize)] = &[
    // Kuzey (nehrin üstü)
    (365.0, 335.0, 140.0, 45.0, B_RED, 4),
    (240.0, 300.0, 80.0, 40.0, B_ORANGE, 2),
    (420.0, 290.0, 140.0, 35.0, B_ORANGE, 3),
    (330.0, 245.0, 220.0, 30.0, B_BEIGE, 3),
    (140.0, 330.0, 35.0, 60.0, B_BEIGE, 1),
    (180.0, 230.0, 90.0, 60.0, B_GREEN, 2),
    (420.0, 205.0, 110.0, 25.0, B_GREEN, 2),
    (640.0, 350.0, 90.0, 70.0, B_GREEN, 3),
    (300.0, 140.0, 160.0, 30.0, B_TEAL, 3),
    (675.0, 265.0, 65.0, 40.0, B_TEAL, 2),
    (630.0, 185.0, 120.0, 35.0, B_NAVY, 2),
    (560.0, 130.0, 210.0, 28.0, B_OATH, 3),
    // Güneybatı
    (205.0, 610.0, 50.0, 70.0, B_RED, 2),
    (175.0, 475.0, 35.0, 25.0, B_ORANGE, 1),
    (195.0, 710.0, 55.0, 40.0, B_ORANGE, 2),
    (190.0, 760.0, 60.0, 28.0, B_BEIGE, 1),
    (180.0, 815.0, 60.0, 35.0, B_GREEN, 1),
    (180.0, 865.0, 60.0, 25.0, B_BLUE, 1),
    // Orta güney (iki nehir arası)
    (490.0, 685.0, 90.0, 35.0, B_RED, 2),
    (470.0, 745.0, 85.0, 40.0, B_ORANGE, 2),
    (580.0, 760.0, 45.0, 60.0, B_BEIGE, 1),
    (600.0, 800.0, 100.0, 25.0, B_GREEN, 2),
    (450.0, 825.0, 120.0, 25.0, B_TEAL, 2),
    (470.0, 870.0, 170.0, 30.0, B_BLUE, 3),
    // Doğu
    (600.0, 560.0, 45.0, 55.0, B_RED, 2),
    (640.0, 610.0, 35.0, 80.0, B_ORANGE, 1),
    (690.0, 640.0, 30.0, 150.0, B_BEIGE, 2),
    (735.0, 580.0, 40.0, 80.0, B_GREEN, 2),
    (730.0, 700.0, 50.0, 60.0, B_TEAL, 1),
    (740.0, 795.0, 45.0, 55.0, B_BLUE, 2),
    (720.0, 855.0, 45.0, 25.0, B_LAV, 1),
    (690.0, 880.0, 50.0, 20.0, B_NAVY, 1),
];

/// Köy çevresi yeni başlayan bölgeleri (ana harita metresi): Yabani Köpek, Kurt.
const PY_DOGS: &[(f32, f32, f32, f32, usize)] = &[
    (232.0, 598.0, 18.0, 30.0, 2),
    (470.0, 560.0, 22.0, 22.0, 2),
    (330.0, 790.0, 28.0, 18.0, 2),
    (312.0, 530.0, 18.0, 14.0, 1),
];

/// Mini patronlar: (mob, yakın plan x, y).
const PY_BOSSES: &[(u32, f32, f32)] = &[
    (190, 330.0, 245.0),
    (191, 675.0, 265.0),
    (192, 470.0, 870.0),
    (193, 740.0, 795.0),
    (194, 300.0, 140.0),
    (195, 640.0, 350.0),
    (196, 180.0, 865.0),
    (197, 730.0, 700.0),
];

fn zoom_to_map(zx: f32, zy: f32) -> Vec2 {
    Vec2::new(zx * 0.988 - 8.3, zy * 1.259 - 15.1)
}

/// Elipste açık rastgele nokta (harita metresi).
fn pick_in(d: &super::terrain::TerrainData, c: Vec2, r: Vec2) -> Option<Vec2> {
    let mut rng = rand::rng();
    for _ in 0..60 {
        let a = rng.random_range(0.0..std::f32::consts::TAU);
        let k = rng.random::<f32>().sqrt();
        let m = c + Vec2::new(a.cos() * r.x, a.sin() * r.y) * k;
        if d.open(m) {
            return Some(m);
        }
    }
    None
}

fn place_pyungmoo(
    commands: &mut Commands,
    server: &AssetServer,
    wa: &mut WorldAssets,
    images: &mut Assets<Image>,
    db: Option<&super::character::CharDb>,
    d: &super::terrain::TerrainData,
) {
    let mut rng = rand::rng();
    let mut spawn_pack = |commands: &mut Commands, kinds: &[u32], c: Vec2, r: Vec2| {
        let Some(m) = pick_in(d, c, r) else { return };
        let k = kinds[rng.random_range(0..kinds.len())];
        let size = rng.random_range(3..=5);
        let center = d.to_world(m.x, m.y);
        let pack = commands
            .spawn((Pack { mob: k, size, center, alive: 0, respawn: 0.0 }, Transform::from_translation(center)))
            .id();
        for _ in 0..size {
            let pm = pick_in(d, m, Vec2::splat(5.0)).unwrap_or(m);
            spawn_mob(commands, server, wa, images, db, mob(k), d.to_world(pm.x, pm.y), Some(pack), false);
        }
    };
    for &(zx, zy, rx, ry, kinds, n) in PY_ZONES {
        let c = zoom_to_map(zx, zy);
        let r = Vec2::new(rx * 0.988, ry * 1.259);
        for _ in 0..n {
            spawn_pack(commands, kinds, c, r);
        }
    }
    for &(x, y, rx, ry, n) in PY_DOGS {
        for _ in 0..n {
            spawn_pack(commands, B_DOGS, Vec2::new(x, y), Vec2::new(rx, ry));
        }
    }
    for &(id, zx, zy) in PY_BOSSES {
        if let Some(m) = pick_in(d, zoom_to_map(zx, zy), Vec2::splat(25.0)) {
            spawn_mob(commands, server, wa, images, db, mob(id), d.to_world(m.x, m.y), None, false);
            info!("mobs: patron {} @ harita ({:.0}, {:.0})", mob(id).name, m.x, m.y);
        }
    }
}

fn place_packs(
    mut commands: Commands,
    mut sp: ResMut<Spawner>,
    server: Res<AssetServer>,
    mut wa: Option<ResMut<WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    col: WorldCollision,
    wp: WorldPoint,
    player: Query<(&Transform, &Player)>,
    db: Option<Res<super::character::CharDb>>,
) {
    let Ok((pxf, p)) = player.single() else { return };
    let Some(wa) = wa.as_deref_mut() else { return };
    if db.is_none() {
        return;
    }
    if let Some(d) = super::terrain::data() {
        if !sp.init && p.grounded {
            sp.init = true;
            place_pyungmoo(&mut commands, &server, wa, &mut images, db.as_deref(), d);
        }
        return;
    }
    if !p.grounded {
        return;
    }
    let mut rng = rand::rng();
    if !sp.init {
        sp.init = true;
        for (kinds, x, y, z, r, n, lo, hi) in super::zones::spawns() {
            for i in 0..*n {
                let k = kinds[i % kinds.len()];
                sp.pending.push((k, rng.random_range(*lo..=*hi), [*x, *y, *z], *r));
            }
        }
    }
    let _ = pxf;
    // Kare başına birkaç deneme: alan henüz akmadıysa (uzak) sonra tekrar denenir.
    let mut tries = 0;
    let mut i = 0;
    while i < sp.pending.len() && tries < 24 {
        tries += 1;
        let (k, size, c, r) = sp.pending[i];
        let ang = rng.random_range(0.0..std::f32::consts::TAU);
        let d = r * rng.random_range(0.0f32..1.0).sqrt();
        let wow = [c[0] + ang.cos() * d, c[1] + ang.sin() * d, c[2]];
        let Some(center) = open_ground(&col, &wp, benilla_assets::coords::wow_to_bevy(wow), c[2], 60.0) else {
            i += 1;
            continue;
        };
        let pack = commands
            .spawn((Pack { mob: k, size, center, alive: 0, respawn: 0.0 }, Transform::from_translation(center)))
            .id();
        sp.pending.swap_remove(i);
        let def = mob(k);
        info!("mobs: {} x{} @ WoW ({:.0}, {:.0})", def.name, size, wow[0], wow[1]);
        for _ in 0..size {
            let off = Vec3::new(rng.random_range(-4.0..4.0), 0.0, rng.random_range(-4.0..4.0));
            let at = if size == 1 { center } else { open_ground(&col, &wp, center + off, center.y, 4.0).unwrap_or(center) };
            spawn_mob(&mut commands, &server, wa, &mut images, db.as_deref(), def, at, Some(pack), false);
        }
    }
    // Bu karede bakılıp yerleşemeyen satırları (alanı henüz akmamış, uzak) sona al: bir sonraki karede sıradaki
    // satırlar denenir. Yoksa liste başındaki 24 uzak satır (ör. Westfall'dayken Elwynn'in doğusu) bütün
    // bölgeleri tıkardı.
    let n = sp.pending.len();
    if n > 0 {
        sp.pending.rotate_left(i % n);
    }
}

fn respawn_packs(
    mut commands: Commands,
    time: Res<Time>,
    server: Res<AssetServer>,
    mut wa: Option<ResMut<WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    col: WorldCollision,
    wp: WorldPoint,
    mut packs: Query<(Entity, &mut Pack)>,
    mobs: Query<&Mob, Without<Dying>>,
    db: Option<Res<super::character::CharDb>>,
) {
    let Some(wa) = wa.as_deref_mut() else { return };
    let mut alive = std::collections::HashMap::<Entity, usize>::new();
    for m in &mobs {
        if let Some(p) = m.pack {
            *alive.entry(p).or_default() += 1;
        }
    }
    let mut rng = rand::rng();
    for (e, mut pk) in &mut packs {
        pk.alive = alive.get(&e).copied().unwrap_or(0);
        if pk.alive < pk.size {
            pk.respawn += time.delta_secs();
            if pk.respawn > 25.0 {
                pk.respawn = 0.0;
                let off = Vec3::new(rng.random_range(-5.0..5.0), 0.0, rng.random_range(-5.0..5.0));
                let at = open_ground(&col, &wp, pk.center + off, pk.center.y, 4.0).unwrap_or(pk.center);
                spawn_mob(&mut commands, &server, wa, &mut images, db.as_deref(), mob(pk.mob), at, Some(e), false);
            }
        } else {
            pk.respawn = 0.0;
        }
    }
}

fn mob_ai(
    time: Res<Time>,
    col: WorldCollision,
    stats: Res<PlayerStats>,
    player: Query<(Entity, &Transform), (With<Player>, Without<Mob>)>,
    mut mobs: Query<(Entity, &mut Mob, &mut Transform, &mut AnimState, &Enemy, &Health, Has<Passive>), Without<Dying>>,
    mut hurt: MessageWriter<PlayerHurt>,
) {
    let Ok((pe, pxf)) = player.single() else { return };
    let ppos = pxf.translation;
    let dt = time.delta_secs();
    let now = time.elapsed_secs();
    let mut rng = rand::rng();
    for (e, mut m, mut xf, mut st, en, hp, passive) in &mut mobs {
        if passive {
            continue;
        }
        let pos = xf.translation;
        let to_p = Vec3::new(ppos.x - pos.x, 0.0, ppos.z - pos.z);
        let dist_p = to_p.length();
        let from_home = Vec3::new(pos.x - m.home.x, 0.0, pos.z - m.home.z).length();
        // Uzak mobları dondur (performans).
        if dist_p > 160.0 && !matches!(m.ai, Ai::Return) {
            continue;
        }
        let hurt_recently = hp.hp < hp.max;
        if !stats.dead
            && !matches!(m.ai, Ai::Chase | Ai::Return)
            && ((m.def.aggressive && dist_p < 9.0) || m.aggro)
        {
            m.ai = Ai::Chase;
        }
        if stats.dead && matches!(m.ai, Ai::Chase) {
            m.ai = Ai::Return;
        }
        m.cd -= dt;
        // Vuruş anı.
        if let Some(at) = m.hit_at {
            if now >= at {
                m.hit_at = None;
                if dist_p < 2.2 + en.radius * xf.scale.x && !stats.dead {
                    let amt = (m.def.atk as f32 * rng.random_range(0.8..1.2)) as i32;
                    hurt.write(PlayerHurt { amount: amt, from: e });
                }
            }
        }
        let speed = m.def.speed;
        let mut step = Vec3::ZERO;
        let mut moving_anim = None;
        match m.ai {
            Ai::Idle { wait } => {
                let w = wait - dt;
                if w <= 0.0 {
                    let to = m.home
                        + Vec3::new(rng.random_range(-6.0..6.0), 0.0, rng.random_range(-6.0..6.0));
                    m.ai = Ai::Wander { to };
                } else {
                    m.ai = Ai::Idle { wait: w };
                }
            }
            Ai::Wander { to } => {
                let d = Vec3::new(to.x - pos.x, 0.0, to.z - pos.z);
                if d.length() < 0.5 {
                    m.ai = Ai::Idle {
                        wait: rng.random_range(3.0..8.0),
                    };
                } else {
                    step = d.normalize() * speed * 0.35 * dt;
                    moving_anim = Some(anim::WALK);
                }
            }
            Ai::Chase => {
                if from_home > 40.0 && !hurt_recently || from_home > 60.0 {
                    m.ai = Ai::Return;
                    m.aggro = false;
                } else {
                    let reach = 1.6 + en.radius * xf.scale.x;
                    if dist_p > reach {
                        step = to_p.normalize_or_zero() * speed * dt;
                        moving_anim = Some(anim::RUN);
                    } else if m.cd <= 0.0 && !stats.dead {
                        m.cd = 1.6;
                        m.hit_at = Some(now + 0.45);
                        st.play_once(anim::ATTACK_UNARMED);
                    }
                    if to_p.length_squared() > 0.01 {
                        let want = Quat::from_rotation_arc(Vec3::NEG_Z, to_p.normalize());
                        xf.rotation = xf.rotation.slerp(want, (dt * 10.0).min(1.0));
                    }
                }
            }
            Ai::Return => {
                let d = Vec3::new(m.home.x - pos.x, 0.0, m.home.z - pos.z);
                if d.length() < 1.0 {
                    m.ai = Ai::Idle { wait: 2.0 };
                } else {
                    step = d.normalize() * speed * 1.2 * dt;
                    moving_anim = Some(anim::RUN);
                }
            }
        }
        if step != Vec3::ZERO {
            xf.translation = super::physics::walk(&col, pos, step, 0.3 * xf.scale.x);
            if !matches!(m.ai, Ai::Chase) {
                let want = Quat::from_rotation_arc(Vec3::NEG_Z, step.normalize());
                xf.rotation = xf.rotation.slerp(want, (dt * 8.0).min(1.0));
            }
        } else if let Some(g) = super::physics::ground_at(&col, pos) {
            if (g - pos.y).abs() < 2.0 {
                xf.translation.y = g;
            }
        }
        // Dönüşte iyileş.
        let _ = pe;
        st.set_loop(moving_anim.unwrap_or(anim::STAND));
        st.speed = if moving_anim == Some(anim::WALK) { 0.8 } else { 1.0 };
    }
}

/// Vurulan mob saldırgana döner; vurulma tepkisi.
fn mob_hit_feedback(mut hits: MessageReader<Hit>, mut mobs: Query<(&mut Mob, &mut AnimState)>) {
    for h in hits.read() {
        if let Ok((mut m, mut st)) = mobs.get_mut(h.target) {
            m.aggro = true;
            if !matches!(m.ai, Ai::Return) {
                m.ai = Ai::Chase;
            }
            if !st.busy() && h.crit {
                st.play_once(anim::COMBAT_WOUND);
            }
        }
    }
}

fn mob_killed(mut killed: MessageReader<Killed>, mut q: Query<&mut Mob>) {
    for k in killed.read() {
        if let Ok(mut m) = q.get_mut(k.entity) {
            m.hit_at = None;
        }
    }
}

/// Dönen ve iyileşen moblar.
pub fn heal_returning(mut q: Query<(&Mob, &mut Health)>) {
    for (m, mut hp) in &mut q {
        if matches!(m.ai, Ai::Return | Ai::Idle { .. }) && !m.aggro {
            hp.hp = hp.max;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mob_ids_unique_across_tables() {
        let mut seen = std::collections::HashSet::new();
        for d in all_mobs() {
            assert!(seen.insert(d.id), "yinelenen mob kimliği {}", d.id);
        }
        assert_eq!(seen.len(), MOB_TABLES.iter().map(|t| t.len()).sum::<usize>());
    }

    #[test]
    fn other_species_have_body_and_lookup_works() {
        for d in all_mobs() {
            assert!(d.species != Species::Other || d.body.is_some(), "{}: Species::Other için with_body gerekli", d.name);
            assert_eq!(mob(d.id).name, d.name);
        }
        // Bilinmeyen kimlik eski davranışla ilk moba düşer.
        assert_eq!(mob(9_999_999).id, MOBS[0].id);
        // Dış gövde verisi türün yerine geçer.
        let d = with_loot(with_body(m(1, "x", Species::Other, "", 1.0, 1, false, W), "Creature/A/A.m2", 1.1, 2.2), &[(1, 0.5)]);
        assert_eq!(d.model(), "Creature/A/A.m2");
        assert_eq!(d.collision(), (1.1, 2.2));
        assert_eq!(d.drops(), &[(1, 0.5)]);
        // Eski türler değişmedi.
        assert_eq!(mob(224).collision(), (0.6, 2.3));
        assert_eq!(mob(224).model(), "Creature/GnollMelee/GnollMelee.m2");
    }
}
