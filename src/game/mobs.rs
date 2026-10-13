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
    /// Beyaz Yeminli askerler: zırhlı insan modeli.
    Human,
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
}

use super::items::{
    V_BEAR_GALL, V_BEAR_PAW, V_BOAR_PELT, V_BOAR_TUSK, V_TIGER_PELT, V_WHITE_RIBBON, V_WOLF_CLAW,
    V_WOLF_PELT,
};

impl MobDef {
    pub fn model(&self) -> &'static str {
        match self.species {
            Species::Dog | Species::Wolf => "Creature/Wolf/Wolf.m2",
            Species::Boar => "Creature/Boar/Boar.m2",
            Species::Bear => "Creature/bear/Bear.m2",
            Species::Tiger => "Creature/Tiger/Tiger.m2",
            Species::Human => "Character/Human/Male/HumanMale.m2",
        }
    }
    /// Türe özgü düşürmeler (WoW ikonlu Metin2 malzemeleri): (vnum, şans).
    pub fn drops(&self) -> &'static [(u32, f32)] {
        match self.species {
            Species::Dog => &[(V_WOLF_CLAW, 0.25)],
            Species::Wolf => &[(V_WOLF_CLAW, 0.35), (V_WOLF_PELT, 0.25)],
            Species::Boar => &[(V_BOAR_TUSK, 0.35), (V_BOAR_PELT, 0.2)],
            Species::Bear => &[(V_BEAR_PAW, 0.3), (V_BEAR_GALL, 0.15)],
            Species::Tiger => &[(V_TIGER_PELT, 0.3), (V_WOLF_CLAW, 0.2)],
            Species::Human => &[(V_WHITE_RIBBON, 0.2)],
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

const fn m(
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
            Species::Human => 5.5,
            _ => 6.5,
        },
        aggressive,
        tint,
        boss: false,
        gear: None,
    }
}

const fn boss(mut d: MobDef, mult: i32) -> MobDef {
    d.boss = true;
    d.hp *= mult;
    d.atk = d.atk * 3 / 2;
    d.exp *= mult as u32;
    d.def += 5;
    d
}

const fn human(mut d: MobDef, set: super::character::ArmorSet, weapon: u32) -> MobDef {
    d.gear = Some((set, weapon));
    d
}

const W: [f32; 3] = [1.0, 1.0, 1.0];
const HUNGRY: [f32; 3] = [0.9, 0.85, 0.85];
const BLUE: [f32; 3] = [0.65, 0.8, 1.35];
const GREY: [f32; 3] = [1.55, 1.55, 1.6];
const CURSED: [f32; 3] = [1.15, 0.55, 1.35];
const WOLF_T: &str = "Creature/Wolf/WolfSkinTimber.blp";
const WOLF_TA: &str = "Creature/Wolf/WolfSkinTimberAlpha.blp";
const WOLF_A: &str = "Creature/Wolf/WolfSkinArctic.blp";
const WOLF_AA: &str = "Creature/Wolf/WolfSkinArcticAlpha.blp";
const WOLF_B: &str = "Creature/Wolf/WolfSkinBlack.blp";
const WOLF_BA: &str = "Creature/Wolf/WolfSkinBlackAlpha.blp";
const COYOTE: &str = "Creature/Wolf/WolfSkinCoyote.blp";
const BOAR_B: &str = "Creature/Boar/BoarSkinBrown.blp";
const BOAR_R: &str = "Creature/Boar/BoarSkinCrimson.blp";
const BOAR_ARM: &str = "Creature/Boar/BoarSkinBrownArmored.blp";
const BEAR_BR: &str = "Creature/bear/BearSkinBrown.blp";
const BEAR_DB: &str = "Creature/bear/BearSkinDrkBrown.blp";
const BEAR_BL: &str = "Creature/bear/BearSkinBlack.blp";
const TIGER_Y: &str = "Creature/Tiger/TigerSkinYellow.blp";
const TIGER_W: &str = "Creature/Tiger/TigerSkinWhite.blp";
const TIGER_BS: &str = "Creature/Tiger/TigerSkinBlackStriped.blp";

type Set = super::character::ArmorSet;
/// Beyaz Yeminli zırh setleri (gömlek, göğüs, kemer, pantolon, çizme, bileklik, eldiven, tabard).
const OATH_SOLDIER: Set = [None, Some(16184), Some(3418), Some(3414), Some(25828), Some(4448), Some(25831), None];
const OATH_ARCHER: Set = [None, Some(2183), None, Some(1882), Some(2198), None, None, None];
const OATH_CMD: Set = [None, Some(29958), Some(29959), Some(29963), Some(29960), Some(29961), Some(29962), None];
const OATH_GEN: Set = [None, Some(33628), Some(33632), Some(33630), Some(33631), Some(33627), Some(33629), None];

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
];

pub fn mob(id: u32) -> &'static MobDef {
    MOBS.iter().find(|m| m.id == id).unwrap_or(&MOBS[0])
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

/// Bir sürü yuvası: ölen üyeleri bir süre sonra yeniden doğurur.
#[derive(Component)]
pub struct Pack {
    pub mob: u32,
    pub size: usize,
    pub center: Vec3,
    pub alive: usize,
    pub respawn: f32,
}

/// Bölgeler (Pyungmoo haritası gelene kadar başlangıca uzaklığa göre): seviye arttıkça uzaklaşır.
const ZONES: &[(f32, f32, &[u32], usize)] = &[
    // (iç yarıçap, dış yarıçap, mob türleri, sürü sayısı)
    (22.0, 60.0, &[101, 102, 103, 105], 7),
    (55.0, 105.0, &[104, 106, 107, 108, 109, 111], 7),
    (95.0, 145.0, &[110, 112, 113, 114, 115, 117], 6),
    (135.0, 185.0, &[116, 118, 119, 120, 121, 125], 6),
    (170.0, 230.0, &[122, 123, 124, 126, 127, 129, 131], 6),
    (210.0, 280.0, &[128, 130, 132, 133, 134, 135, 136, 137], 5),
];

/// Mini patronlar: tek tek, dış bölgelerde.
const BOSSES: &[(u32, f32)] = &[(190, 130.0), (191, 175.0), (192, 215.0), (193, 240.0), (194, 255.0), (195, 260.0), (196, 265.0), (197, 270.0)];

#[derive(Resource, Default)]
pub struct Spawner {
    /// Bekleyen sürü yerleştirme denemeleri: (mob, boyut, iç, dış).
    pending: Vec<(u32, usize, f32, f32)>,
    init: bool,
    bosses: Vec<(u32, f32)>,
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
    let (radius, height) = match def.species {
        Species::Dog | Species::Wolf => (0.6, 1.3),
        Species::Boar => (0.6, 1.2),
        Species::Bear => (0.9, 1.9),
        Species::Tiger => (0.7, 1.3),
        Species::Human => (0.45, 1.9),
    };
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
    let start = benilla_assets::coords::wow_to_bevy(super::start_wow());
    if !sp.init {
        sp.init = true;
        sp.bosses = BOSSES.to_vec();
        for (rin, rout, kinds, n) in ZONES {
            for i in 0..*n {
                let k = kinds[i % kinds.len()];
                sp.pending.push((k, rand::rng().random_range(3..=5), *rin, *rout));
            }
        }
    }
    let ref_y = pxf.translation.y;
    let mut rng = rand::rng();
    // Kare başına birkaç deneme (arazi henüz akmadıysa sonra tekrar).
    let mut tries = 0;
    let mut i = 0;
    while i < sp.pending.len() && tries < 24 {
        tries += 1;
        let (k, size, rin, rout) = sp.pending[i];
        let ang = rng.random_range(0.0..std::f32::consts::TAU);
        let r = rng.random_range(rin..rout);
        let c = start + Vec3::new(ang.cos() * r, 0.0, ang.sin() * r);
        let Some(center) = open_ground(&col, &wp, c, ref_y, 25.0) else {
            i += 1;
            continue;
        };
        let pack = commands
            .spawn((
                Pack {
                    mob: k,
                    size,
                    center,
                    alive: 0,
                    respawn: 0.0,
                },
                Transform::from_translation(center),
            ))
            .id();
        sp.pending.swap_remove(i);
        let def = mob(k);
        info!("mobs: {} x{} @ {:.0} yd", def.name, size, center.distance(start));
        for _ in 0..size {
            let off = Vec3::new(rng.random_range(-5.0..5.0), 0.0, rng.random_range(-5.0..5.0));
            let at = open_ground(&col, &wp, center + off, center.y, 4.0).unwrap_or(center);
            spawn_mob(&mut commands, &server, wa, &mut images, db.as_deref(), def, at, Some(pack), false);
        }
    }
    // Mini patronlar.
    if sp.pending.is_empty() {
        let mut placed = Vec::new();
        for (k, (id, r)) in sp.bosses.iter().enumerate() {
            let ang = rng.random_range(0.0..std::f32::consts::TAU);
            let c = start + Vec3::new(ang.cos() * r, 0.0, ang.sin() * r);
            if let Some(at) = open_ground(&col, &wp, c, ref_y, 30.0) {
                spawn_mob(&mut commands, &server, wa, &mut images, db.as_deref(), mob(*id), at, None, false);
                info!("mobs: patron {} @ {:.0} yd", mob(*id).name, at.distance(start));
                placed.push(k);
            }
        }
        for k in placed.into_iter().rev() {
            sp.bosses.swap_remove(k);
        }
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
    mut mobs: Query<(Entity, &mut Mob, &mut Transform, &mut AnimState, &Enemy, &Health), Without<Dying>>,
    mut hurt: MessageWriter<PlayerHurt>,
) {
    let Ok((pe, pxf)) = player.single() else { return };
    let ppos = pxf.translation;
    let dt = time.delta_secs();
    let now = time.elapsed_secs();
    let mut rng = rand::rng();
    for (e, mut m, mut xf, mut st, en, hp) in &mut mobs {
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
