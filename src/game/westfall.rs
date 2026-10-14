//! Orijinal WoW haritası: Westfall (Sv 10-20). Konumlar WoW dünya koordinatı (x kuzey, y batı, z yukarı).
//!
//! Ölçüm `tools/adt_zones.py` ile MPQ'daki ADT dosyalarından yapıldı:
//! - Alt bölgeler: MCNK areaId (AreaTable.dbc, Westfall = 40) parçalarından k-means çemberleri (`circles Westfall`,
//!   parçaların ~%88'i doğru adı alır). Bölge kutusu Westfall parçalarının kapsamı.
//! - Bina konumları MODF'tan: Sentinel Hill kulesi -10501,1025 (z 60.5); Saldean evi -10111,1056; Furlbrow çiftliği
//!   -9846,1287; Jangolode madeni (md_goldmine) -9949,1453; Molsen çiftliği -10311,1400; Moonbrook belediyesi
//!   -10959,1514; Deadmines girişi -11099,1528; Dead Acre çiftliği -10760,893.
//! - Zemin yüksekliği MCVT'den, su MCLQ'dan (`check src/game/westfall.rs` bütün satırları sınar).
//!
//! **MPQ kısıtı:** bu veri paketinde Azeroth_27/28_* ADT'leri yok; dünya y ≈ 1600'de biter. Gold Coast Quarry,
//! Westfall Lighthouse, Demont's Place ve güney sahil (Longshore'un batısı) yüklenmez. Murloc'lar bu yüzden
//! kuzey sahilde (Longshore, Jansen Stead'in kuzeyi) — WoW'da da oradalar; Old Murk-Eye de kuzey sahilde.
//!
//! Yaratıklar WoW 1.12 Westfall'daki türler ve yerleri (kuzey Sv 10-13: Genç Kandiş, çakal, Genç Etyırtan,
//! kuzey sahil murloc'ları, Jangolode kaçakçıları, Molsen tuzakçıları; orta Sv 13-17: Kandiş, Etyırtan, Riverpaw
//! izcileri; güney Sv 16-20: Büyük Kandiş/Etyırtan, Dust Plains gnolları, Moonbrook Defias'ları). Patronlar:
//! Old Murk-Eye (murloc), Vultros (etyırtan elit), Sergeant Brashclaw (Riverpaw elit).

use bevy::prelude::*;

use benilla_assets::WorldAssets;

use super::character::{Appearance, ArmorSet};
use super::mobs::{boss, human, m, with_body, MobDef, Species, BOAR_ARM, BOAR_B, COYOTE, DEFIAS, GNOLL_HOGGER, GNOLL_RP, MURLOC, MURLOC_B, W};
use super::npc::{NpcDef, NpcKind};
use super::zones::{MetinSlotRow, SpawnRow, Subzone, ZoneDef};

/// Sentinel Hill kulesinin önü (WoW). `AZM_FOCUS` çekimleri ve testler için.
#[allow(dead_code)]
pub const SENTINEL_HILL: [f32; 3] = [-10520.0, 1090.0, 51.0];
/// Moonbrook meydanı (WoW).
#[allow(dead_code)]
pub const MOONBROOK: [f32; 3] = [-11010.0, 1440.0, 42.0];

/// Bölge kaydı (`zones::ZONES`). Kutu ADT ölçümü: Westfall parçaları x[-11783, -9083], y[350, 1583]
/// (batı ucu MPQ'daki son ADT sütunu). Kuzeydoğuda Elwynn kutusu önce gelir (zones::zone_name sırası).
pub const ZONE: ZoneDef = ZoneDef {
    name: "Westfall",
    bounds: [-11800.0, -9050.0, 350.0, 1600.0],
    subzones: SUBZONES,
    spawns: SPAWNS,
    metin_slots: METIN_SLOTS,
    metin_summons,
};

/// Alt bölgeler (mini harita adı): (ad, x, y, yarıçap). Küçükler önce; uzun alanlar birkaç çemberle.
/// `python -I tools/adt_zones.py circles Westfall` çıktısı (isabet %88).
pub const SUBZONES: &[Subzone] = &[
    ("Alexston Farmstead", -10717.0, 1583.0, 50.0),
    ("Alexston Farmstead", -10617.0, 1583.0, 50.0),
    ("Longshore", -11630.0, 1512.0, 130.0),
    ("The Jansen Stead", -9797.0, 993.0, 140.0),
    ("Saldean's Farm", -10193.0, 1130.0, 145.0),
    ("Furlbrow's Pumpkin Farm", -9879.0, 1237.0, 155.0),
    ("Jangolode Mine", -9884.0, 1474.0, 155.0),
    ("Stendel's Pond", -10758.0, 1330.0, 155.0),
    ("The Dagger Hills", -11192.0, 1220.0, 155.0),
    ("Longshore", -9614.0, 1458.0, 160.0),
    ("The Dagger Hills", -11341.0, 1482.0, 160.0),
    ("The Molsen Farm", -10240.0, 1422.0, 160.0),
    ("Longshore", -9593.0, 1153.0, 165.0),
    ("Longshore", -9387.0, 1404.0, 165.0),
    ("Sentinel Hill", -10583.0, 1091.0, 165.0),
    ("The Dead Acre", -10774.0, 824.0, 170.0),
    ("Longshore", -9256.0, 1148.0, 175.0),
    ("Moonbrook", -11032.0, 1430.0, 190.0),
    ("The Dust Plains", -11126.0, 856.0, 250.0),
];

/// Yaratık yerleşimi: (mob türleri, x, y, z, yarıçap, sürü sayısı, sürü boyu en az, en çok).
/// Sentinel Hill kulesinin 120 m çevresinde mob yok (köy güvenli).
pub const SPAWNS: &[SpawnRow] = &[
    // --- Kuzey Westfall (Sv 10-13): Jansen Stead, Furlbrow, Saldean, Jangolode, Molsen ---
    (&[301], -9850.0, 1000.0, 33.0, 100.0, 5, 1, 2),
    (&[301, 302], -10044.0, 939.0, 35.0, 110.0, 5, 1, 2),
    (&[302, 303], -9920.0, 1230.0, 41.0, 100.0, 5, 2, 3),
    (&[223], -10100.0, 930.0, 35.0, 90.0, 4, 1, 2),
    (&[304], -10210.0, 1150.0, 38.0, 110.0, 5, 1, 2),
    (&[304, 302], -10107.0, 1477.0, 42.0, 70.0, 4, 1, 2),
    (&[331], -10035.0, 1440.0, 41.0, 35.0, 4, 2, 3),
    (&[331], -9790.0, 1430.0, 42.0, 35.0, 3, 1, 2),
    (&[332], -10250.0, 1430.0, 41.0, 90.0, 5, 1, 3),
    // Kuzey sahil (Longshore): murloc köyleri + Old Murk-Eye.
    (&[323, 324], -9690.0, 1280.0, 12.0, 45.0, 5, 2, 3),
    (&[324, 325], -9690.0, 1110.0, 12.0, 45.0, 4, 2, 3),
    (&[326], -9675.0, 1250.0, 9.0, 40.0, 2, 1, 2),
    (&[327], -9690.0, 1300.0, 10.0, 15.0, 1, 1, 1),
    // --- Orta Westfall (Sv 13-17): ovalar, Stendel's Pond, Dead Acre, doğu tepeleri ---
    (&[305], -10438.0, 1295.0, 41.0, 140.0, 6, 1, 2),
    (&[305, 306], -10800.0, 1280.0, 34.0, 90.0, 5, 1, 2),
    (&[306], -10877.0, 1049.0, 36.0, 130.0, 5, 1, 2),
    (&[311, 312], -10330.0, 860.0, 40.0, 90.0, 5, 2, 3),
    (&[312, 313], -10774.0, 824.0, 36.0, 120.0, 5, 2, 3),
    (&[332, 305], -10659.0, 1480.0, 48.0, 75.0, 4, 1, 2),
    (&[309], -10900.0, 1180.0, 36.0, 60.0, 1, 1, 1),
    // --- Güney Westfall (Sv 16-20): Moonbrook, Dagger Hills, Dust Plains ---
    (&[333, 334], -11032.0, 1430.0, 43.0, 110.0, 7, 2, 3),
    (&[335], -10930.0, 1300.0, 39.0, 90.0, 4, 1, 2),
    (&[335, 334], -11330.0, 1470.0, 119.0, 85.0, 4, 1, 2),
    (&[307], -11192.0, 1220.0, 90.0, 130.0, 5, 1, 2),
    (&[308], -11384.0, 1111.0, 130.0, 120.0, 4, 1, 2),
    (&[307, 308], -11252.0, 588.0, 88.0, 120.0, 4, 1, 2),
    (&[314, 315], -11126.0, 856.0, 41.0, 150.0, 6, 2, 3),
    (&[315, 316], -11050.0, 700.0, 36.0, 100.0, 4, 2, 3),
    (&[317], -11140.0, 860.0, 40.0, 20.0, 1, 1, 1),
];

/// Metin taşı alanları: (metin türü, x, y, z, yarıçap). Sv 10 (Dövüş) kuzeyde, Sv 15 (Savaş) ortada,
/// Sv 20 (Hırs) güneyde; dalgalarda o kuşağın Westfall yaratıkları.
pub const METIN_SLOTS: &[MetinSlotRow] = &[
    (1, -9900.0, 1100.0, 35.0, 110.0),
    (1, -10150.0, 980.0, 35.0, 110.0),
    (1, -10250.0, 1300.0, 40.0, 110.0),
    (2, -10438.0, 1295.0, 41.0, 130.0),
    (2, -10877.0, 1049.0, 36.0, 130.0),
    (2, -10330.0, 860.0, 40.0, 90.0),
    (2, -10800.0, 1280.0, 34.0, 90.0),
    (3, -11126.0, 856.0, 41.0, 150.0),
    (3, -11252.0, 588.0, 88.0, 100.0),
    (3, -11192.0, 1220.0, 90.0, 120.0),
    (3, -10960.0, 1320.0, 45.0, 80.0),
];

/// Metin taşı dalgalarında çağrılan Westfall yaratıkları (metin türüne göre).
pub fn metin_summons(kind: usize) -> &'static [u32] {
    match kind {
        0 | 1 => &[301, 302, 303, 304, 331],
        2 => &[305, 306, 311, 312, 313, 332],
        3 => &[307, 308, 314, 315, 333, 334],
        _ => &[316, 335, 308, 315],
    }
}

// --- Yaratık görünümleri (MPQ'da var: tools/mpq_present.txt) ---
const BOAR_Y: &str = "Creature/Boar/BoarSkinYellow.blp";
const COYOTE_A: &str = "Creature/Wolf/WolfSkinCoyoteAlpha.blp";
const BIRD: &str = "Creature/CarrionBird/CarrionBird.m2";
const BIRD_SKIN: &str = "Creature/CarrionBird/CarrionBirdSkin.blp";
const BIRD_BROWN: &str = "Creature/CarrionBird/CarrionBirdSkinBrown.blp";
const BIRD_RED: &str = "Creature/CarrionBird/CarrionBirdSkinRed.blp";
const BIRD_WHITE: &str = "Creature/CarrionBird/CarrionBirdSkinWhite.blp";
const GNOLL_CASTER: &str = "Creature/GnollCaster/GnollCaster.m2";
const GNOLL_CASTER_RP: &str = "Creature/GnollCaster/RiverPawGnollCasterSkin.blp";
const GNOLL_MELEE: &str = "Creature/GnollMelee/GnollMelee.m2";
const MURLOC_G: &str = "Creature/Murloc/SahauginskinGreen.blp";
const MURLOC_W: &str = "Creature/Murloc/SahauginskinWhite.blp";
const MURLOC_O: &str = "Creature/Murloc/SahauginskinOrange.blp";

/// Akbaba (CarrionBird) gövdesi: kanat dokusu `plugin`'deki gözlemciyle ikinci deri yuvasına konur.
const fn bird(d: MobDef) -> MobDef {
    with_body(d, BIRD, 0.8, 1.6)
}
const fn caster(d: MobDef) -> MobDef {
    with_body(d, GNOLL_CASTER, 0.5, 1.9)
}
const fn brute(d: MobDef) -> MobDef {
    with_body(d, GNOLL_MELEE, 0.6, 2.3)
}

// Defias silahları (ItemDisplayInfo; Elwynn Defias'larıyla aynı aile).
const DAGGER: u32 = 2708;
const SWORD: u32 = 1542;
const LONG_SWORD: u32 = 1550;
const MACE: u32 = 1160;
const STAFF: u32 = 1193;

/// Westfall yaratıkları (kimlik 3xx; Riverpaw Gnollü 223 Elwynn tablosundan paylaşılır).
pub const MOBS: &[MobDef] = &[
    // --- Domuzlar (Goretusk) ---
    m(301, "Genç Kandiş Yaban Domuzu", Species::Boar, BOAR_B, 0.8, 10, false, W), // Young Goretusk
    m(305, "Kandiş Yaban Domuzu", Species::Boar, BOAR_Y, 0.95, 14, true, W),     // Goretusk
    m(307, "Büyük Kandiş Yaban Domuzu", Species::Boar, BOAR_ARM, 1.1, 17, true, W), // Great Goretusk
    // --- Çakallar ---
    m(302, "Çakal", Species::Wolf, COYOTE, 0.85, 10, false, W),                       // Coyote
    m(303, "Çakal Sürü Lideri", Species::Wolf, COYOTE_A, 0.95, 12, true, W),          // Coyote Packleader
    // --- Akbabalar (Fleshripper) ---
    bird(m(304, "Genç Etyırtan", Species::Other, BIRD_SKIN, 0.6, 11, false, W)),     // Young Fleshripper
    bird(m(306, "Etyırtan", Species::Other, BIRD_BROWN, 0.7, 15, true, W)),           // Fleshripper
    bird(m(308, "Büyük Etyırtan", Species::Other, BIRD_WHITE, 0.85, 18, true, W)),    // Greater Fleshripper
    boss(bird(m(309, "Vultros", Species::Other, BIRD_RED, 1.2, 21, true, W)), 8),    // nadir elit
    // --- Riverpaw gnolları ---
    m(311, "Riverpaw Melezi", Species::Gnoll, GNOLL_RP, 1.0, 14, true, W),            // Riverpaw Mongrel
    m(312, "Riverpaw İzcisi", Species::Gnoll, GNOLL_RP, 1.05, 15, true, W),           // Riverpaw Scout
    caster(m(313, "Riverpaw Otacısı", Species::Gnoll, GNOLL_CASTER_RP, 1.0, 16, true, W)), // Riverpaw Herbalist
    brute(m(314, "Riverpaw Zorbası", Species::Gnoll, GNOLL_HOGGER, 1.0, 17, true, W)),   // Riverpaw Brute
    caster(m(315, "Riverpaw Mistiği", Species::Gnoll, GNOLL_CASTER_RP, 1.05, 18, true, W)), // Riverpaw Mystic
    brute(m(316, "Riverpaw Kâhyası", Species::Gnoll, GNOLL_HOGGER, 1.1, 19, true, W)),   // Riverpaw Taskmaster
    boss(brute(m(317, "Sergeant Brashclaw", Species::Gnoll, GNOLL_HOGGER, 1.3, 19, true, W)), 6),
    // --- Murloc'lar (kuzey sahil) ---
    m(323, "Murloc Akıncısı", Species::Murloc, MURLOC, 0.95, 11, true, W),            // Murloc Raider
    m(324, "Murloc Kıyıkoşucusu", Species::Murloc, MURLOC_G, 1.0, 12, true, W),       // Murloc Coastrunner
    m(325, "Murloc Gelgit Avcısı", Species::Murloc, MURLOC_B, 1.05, 14, true, W),     // Murloc Tidehunter
    m(326, "Murloc Kâhini", Species::Murloc, MURLOC_W, 1.0, 15, true, W),             // Murloc Oracle
    boss(m(327, "Old Murk-Eye", Species::Murloc, MURLOC_O, 1.5, 20, true, W), 8),
    // --- Defias ---
    human(m(331, "Defias Kaçakçısı", Species::Human, "", 1.0, 12, true, W), DEFIAS, DAGGER), // Defias Smuggler
    human(m(332, "Defias Tuzakçısı", Species::Human, "", 1.0, 13, true, W), DEFIAS, SWORD),  // Defias Trapper
    human(m(333, "Defias Yağmacısı", Species::Human, "", 1.0, 16, true, W), DEFIAS, STAFF),  // Defias Pillager
    human(m(334, "Defias Çapulcusu", Species::Human, "", 1.0, 16, true, W), DEFIAS, MACE),   // Defias Looter
    human(m(335, "Defias Yol Kesicisi", Species::Human, "", 1.05, 18, true, W), DEFIAS, LONG_SWORD), // Defias Highwayman
];

// --- NPC görünümleri (npc.rs'teki setlerin kopyası; ItemDisplayInfo, MPQ'da dokuları var) ---
const GUARD: ArmorSet = [None, Some(16183), Some(3418), Some(3414), Some(25828), Some(4448), Some(25831), Some(21337)];
const CAPTAIN: ArmorSet = [None, Some(29958), Some(29959), Some(29963), Some(29960), Some(29961), Some(29962), None];
const SMITH: ArmorSet = [None, Some(3719), Some(4473), Some(4469), Some(9773), Some(4471), Some(4470), None];
const MERCHANT: ArmorSet = [None, Some(2183), None, Some(1882), Some(2198), None, None, None];
const WEAPONS: ArmorSet = [None, Some(954), None, Some(685), Some(2229), Some(2230), Some(1407), None];
const ARMORS: ArmorSet = [None, Some(4151), Some(3254), Some(16092), Some(16112), None, Some(3773), None];
const GROOM: ArmorSet = [None, Some(2106), Some(3247), Some(2176), Some(2094), Some(2178), None, None];
const PEASANT: ArmorSet = [None, Some(7574), None, Some(9988), Some(9992), None, None, None];

const M_DARK: Appearance = Appearance::male(4, 3, 2, 1, 4);
const M_BROWN: Appearance = Appearance::male(1, 2, 7, 3, 1);
const M_BLOND: Appearance = Appearance::male(0, 5, 3, 2, 0);
const M_GRAY: Appearance = Appearance::male(2, 6, 1, 8, 5);
const F_YOUNG: Appearance = Appearance::female(1, 2, 4, 1, 0);
const F_OLD: Appearance = Appearance::female(2, 7, 2, 8, 0);

const HAMMER: u32 = 1022;
const SILVER_SWORD: u32 = 4443;
const HALBERD: u32 = 1733;
const PITCHFORK: u32 = 1490;
const BOW_STAFF: u32 = 1201;

/// Sentinel Hill kulesinin içi (zemin kat, WMO tabanı z 60.5). WoW konumları: Gryan Stoutmantle (56.2, 47.6) ve
/// Quartermaster Lewis (56.9, 47.2) harita yüzdesi → WorldMapArea.dbc (Westfall) ile dünya koordinatı.
const TOWER: (f32, f32) = (-10501.0, 1025.0);

/// Westfall NPC'leri (WoW x/y/z). Sentinel Hill (Halk Milisi) + Saldean ve Furlbrow çiftlikleri.
pub const NPCS: &[NpcDef] = &[
    NpcDef {
        name: "Gryan Stoutmantle",
        title: "Halk Milisi",
        kind: NpcKind::Captain,
        map: (-10510.7, 1049.7),
        face: TOWER,
        z: 60.5,
        armor: CAPTAIN,
        tool: Some(SILVER_SWORD),
        look: M_GRAY,
        greet: "Westfall'u Defias'lara bırakmayacağız! Moonbrook'a giden yolda metin taşları belirdi, kır onları.",
    },
    NpcDef {
        name: "Yüzbaşı Danuvin",
        title: "Halk Milisi",
        kind: NpcKind::Guard,
        map: (-10496.0, 1040.0),
        face: TOWER,
        z: 60.5,
        armor: GUARD,
        tool: Some(HALBERD),
        look: M_BROWN,
        greet: "Riverpaw gnolları Dust Plains'ten kuzeye iniyor. Kılıcını hazır tut.",
    },
    NpcDef {
        name: "Levazımcı Lewis",
        title: "Genel Malzeme",
        kind: NpcKind::Merchant,
        map: (-10501.3, 1025.2),
        face: (-10510.0, 1045.0),
        z: 60.5,
        armor: MERCHANT,
        tool: None,
        look: M_BLOND,
        greet: "Milisin erzakı bende: iksir, parşömen, bileme taşı. Ne lazım?",
    },
    NpcDef {
        name: "William MacGregor",
        title: "Silah Satıcısı",
        kind: NpcKind::WeaponMerchant,
        map: (-10512.0, 1032.0),
        face: TOWER,
        z: 60.5,
        armor: WEAPONS,
        tool: Some(BOW_STAFF),
        look: M_DARK,
        greet: "Yayım da kılıcım da sağlamdır. Moonbrook'a çıplak elle gidilmez.",
    },
    NpcDef {
        name: "Christopher Hewen",
        title: "Zırh Satıcısı",
        kind: NpcKind::ArmorMerchant,
        map: (-10490.0, 1030.0),
        face: TOWER,
        z: 60.5,
        armor: ARMORS,
        tool: None,
        look: M_BROWN,
        greet: "Etyırtan pençesi deriyi deler; iyi bir zırh hayat kurtarır.",
    },
    NpcDef {
        name: "Milis Demircisi",
        title: "Demirci",
        kind: NpcKind::Blacksmith,
        map: (-10492.0, 1016.0),
        face: TOWER,
        z: 60.5,
        armor: SMITH,
        tool: Some(HAMMER),
        look: M_DARK,
        greet: "Kulenin örsü sıcak. Silahını güçlendirelim mi? Şans cesurdan yanadır!",
    },
    NpcDef {
        name: "Kirk Maxwell",
        title: "Ahır Ustası",
        kind: NpcKind::Stable,
        map: (-10518.0, 1082.0),
        face: (-10505.0, 1050.0),
        z: 52.5,
        armor: GROOM,
        tool: Some(PITCHFORK),
        look: M_BLOND,
        greet: "Westfall'un ovaları binekle bir günde geçilir. Bileti al, dörtnala sür!",
    },
    NpcDef {
        name: "Han Sahibi Heather",
        title: "Han Sahibi",
        kind: NpcKind::Villager,
        map: (-10505.0, 1012.0),
        face: TOWER,
        z: 60.5,
        armor: PEASANT,
        tool: None,
        look: F_YOUNG,
        greet: "Tepeye hoş geldin! Sıcak bir çorba ve kuru bir yatak, Westfall'da bundan iyisi yok.",
    },
    NpcDef {
        name: "Salma Saldean",
        title: "Breadbasket Loncası",
        kind: NpcKind::Villager,
        map: (-10113.8, 1043.4),
        face: (-10125.0, 1070.0),
        z: 38.0,
        armor: PEASANT,
        tool: None,
        look: F_OLD,
        greet: "Çiftliği Defias'lar yağmaladı, Etyırtanlar tavukları kaptı... Yardım edersen çorbam hazır.",
    },
    NpcDef {
        name: "Çiftçi Furlbrow",
        title: "Jansen Stead",
        kind: NpcKind::Villager,
        map: (-9848.0, 916.7),
        face: (-9850.0, 1000.0),
        z: 30.6,
        armor: PEASANT,
        tool: Some(PITCHFORK),
        look: M_GRAY,
        greet: "Defias haydutları evimizi elimizden aldı. Şimdi yol kenarında sefil bekliyoruz.",
    },
];

/// Bölgeye özgü sistemler: akbaba (CarrionBird) kanat dokusu.
pub fn plugin(app: &mut App) {
    app.add_observer(bird_wings);
}

/// CarrionBird M2'sinin ikinci deri yuvası (tip 12 = kanat) `MobDef::skin`'le gelmez; mob doğduğu anda
/// (model kurulmadan önce) aynı renkteki kanat dokusu konur. Yoksa kanatlar çizilmez.
fn bird_wings(
    add: On<Add, super::mobs::Mob>,
    mut q: Query<(&super::mobs::Mob, &mut super::model::WowModel)>,
    mut wa: Option<ResMut<WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Ok((mob, mut wm)) = q.get_mut(add.entity) else { return };
    let Some(wing) = wing_of(mob.def) else { return };
    let Some(wa) = wa.as_deref_mut() else { return };
    if wm.skins[1].is_none() {
        wm.skins[1] = wa.texture(&wing, (true, true), &mut images);
    }
}

/// Akbaba derisinin kanat karşılığı ("...SkinBrown.blp" → "...WingBrown.blp").
fn wing_of(def: &MobDef) -> Option<String> {
    (def.model() == BIRD).then(|| def.skin.replacen("CarrionBirdSkin", "CarrionBirdWing", 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{mobs, zones};

    fn inside(p: [f32; 3]) -> bool {
        ZONE.contains(p)
    }

    #[test]
    fn subzones_resolve() {
        assert_eq!(zones::zone_name(SENTINEL_HILL), Some("Sentinel Hill"));
        assert_eq!(zones::zone_name(MOONBROOK), Some("Moonbrook"));
        assert_eq!(zones::zone_name([-10113.8, 1043.4, 38.0]), Some("Saldean's Farm"));
        assert_eq!(zones::zone_name([-9884.0, 1474.0, 49.0]), Some("Jangolode Mine"));
        assert_eq!(zones::zone_name([-11126.0, 856.0, 56.0]), Some("The Dust Plains"));
        // Alt bölge dışı Westfall ovası → bölge adı; Elwynn değişmedi.
        assert_eq!(zones::zone_name([-10440.0, 1220.0, 40.0]), Some("Westfall"));
        assert_eq!(zones::zone_name([-9438.0, 64.0, 57.0]), Some("Goldshire"));
        assert_eq!(zones::zone_name([-9700.0, -300.0, 50.0]), Some("Elwynn Forest"));
        // Alt bölge çemberleri kutunun içinde; küçükler önce.
        for w in SUBZONES.windows(2) {
            assert!(w[0].3 <= w[1].3, "{} sırası", w[1].0);
        }
        for (n, x, y, _) in SUBZONES {
            assert!(inside([*x, *y, 0.0]), "{n} kutunun dışında");
        }
    }

    #[test]
    fn mob_ids_in_westfall_range() {
        let mut seen = std::collections::HashSet::new();
        for d in MOBS {
            assert!((300..400).contains(&d.id), "{}: kimlik {} 3xx değil", d.name, d.id);
            assert!(seen.insert(d.id), "yinelenen {}", d.id);
            assert_eq!(mobs::mob(d.id).name, d.name, "kayıt zinciri");
            assert!((10..=21).contains(&d.level), "{} seviye {}", d.name, d.level);
            // Akbabalar kanat dokusu alır; MPQ'da olmayan modeller (HarvestGolem, DustDevil) kullanılmaz.
            assert!(!d.model().contains("HarvestGolem") && !d.model().contains("DustDevil"));
            if d.model() == BIRD {
                assert!(wing_of(d).is_some_and(|w| w.contains("CarrionBirdWing")), "{}", d.name);
            }
        }
        assert_eq!(MOBS.iter().filter(|d| d.boss).count(), 3);
    }

    #[test]
    fn spawns_and_metins_inside_westfall() {
        for (kinds, x, y, z, r, n, lo, hi) in SPAWNS {
            assert!(inside([*x, *y, *z]), "yerleşim ({x}, {y}) Westfall dışında");
            assert!(*r > 0.0 && *n > 0 && lo <= hi);
            // Çember MPQ'daki son ADT sütunundan (y 1600, dünyanın batı kenarı) en az 40 m içeride kalsın.
            assert!(y + r <= 1560.0, "yerleşim ({x}, {y}) r={r} dünya kenarına çok yakın");
            for k in *kinds {
                let d = mobs::find(*k).unwrap_or_else(|| panic!("tanımsız mob {k}"));
                assert!((10..=21).contains(&d.level), "{} Westfall seviyesinde değil", d.name);
            }
            // Sentinel Hill köyü güvenli.
            assert!((x - TOWER.0).hypot(y - TOWER.1) - r > 100.0, "({x}, {y}) Sentinel Hill'e çok yakın");
        }
        for (kind, x, y, z, r) in METIN_SLOTS {
            assert!(inside([*x, *y, *z]) && y + r <= 1560.0, "metin ({x}, {y})");
            let lvl = crate::game::metin::METINS[*kind].level;
            assert!((10..=20).contains(&lvl), "metin türü {kind} Sv {lvl}");
            assert!(!metin_summons(*kind).is_empty());
            for k in metin_summons(*kind) {
                assert!(mobs::find(*k).is_some(), "tanımsız çağrı {k}");
            }
        }
    }

    #[test]
    fn npcs_in_westfall() {
        for n in NPCS {
            assert!(inside([n.map.0, n.map.1, n.z]), "{} Westfall dışında", n.name);
            assert!(zones::zone_name([n.map.0, n.map.1, n.z]).is_some());
        }
        for k in [NpcKind::Blacksmith, NpcKind::Merchant, NpcKind::WeaponMerchant, NpcKind::ArmorMerchant, NpcKind::Stable] {
            assert!(NPCS.iter().any(|n| n.kind == k && zones::zone_name([n.map.0, n.map.1, n.z]) == Some("Sentinel Hill")), "{k:?} Sentinel Hill'de yok");
        }
    }
}
