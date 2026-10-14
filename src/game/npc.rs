//! Pyungmoo köy NPC'leri (veri tabanlı). Her satır: ad, unvan, tür, wiki harita koordinatı, bakış noktası,
//! zırh seti, elindeki eşya, görünüm ve karşılama satırı. Tıklanınca oyuncu yaklaşır; menzilde karşılama
//! satırı sohbet kutusuna yazılır, `NpcTalked` yayınlanır ve gerekiyorsa (Demirci / satıcılar) pencere açılır.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use benilla_assets::coords::wow_to_bevy;
use benilla_assets::WorldAssets;

use super::camera::OrbitCam;
use super::character::{build_body_as, build_item_model, Appearance, ArmorSet, CharDb};
use super::combat::{cursor_ray, Clicks};
use super::log::GameLog;
use super::model::{AnimState, ModelReady};
use super::player::Player;
use super::ui::Windows;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum NpcKind {
    /// Demirci: artı basma (yükseltme penceresi).
    Blacksmith,
    /// Satıcı: genel mağaza.
    Merchant,
    /// Silah Satıcısı (bu sprintte genel mağazayla aynı stok).
    WeaponMerchant,
    /// Zırh Satıcısı (aynı stok).
    ArmorMerchant,
    Warehouse,
    Stable,
    FortuneTeller,
    Guard,
    Teleporter,
    Captain,
    Trainer,
    /// Görev veren köylüler (Uriel, Octavio, Baek-Go, Soon).
    Villager,
    Fisherman,
}

impl NpcKind {
    #[allow(dead_code)] // testler ve Sprint 3 diyalog sistemi (G-05)
    pub const ALL: [NpcKind; 13] = [
        NpcKind::Blacksmith,
        NpcKind::Merchant,
        NpcKind::WeaponMerchant,
        NpcKind::ArmorMerchant,
        NpcKind::Warehouse,
        NpcKind::Stable,
        NpcKind::FortuneTeller,
        NpcKind::Guard,
        NpcKind::Teleporter,
        NpcKind::Captain,
        NpcKind::Trainer,
        NpcKind::Villager,
        NpcKind::Fisherman,
    ];

    /// Mağaza penceresi açan türler.
    pub fn is_shop(self) -> bool {
        matches!(self, NpcKind::Merchant | NpcKind::WeaponMerchant | NpcKind::ArmorMerchant)
    }

    /// Türe göre varsayılan karşılama (satırda özel metin yoksa).
    pub fn greeting(self) -> &'static str {
        match self {
            NpcKind::Blacksmith => "Silahını güçlendirmeye mi geldin? Demir sabır ister, şans da cesaret!",
            NpcKind::Merchant => "Hoş geldin yolcu! İksirlerim taze, fiyatlarım köyün en iyisi.",
            NpcKind::WeaponMerchant => "Keskin bir kılıç, canavarlara karşı en sadık dostundur.",
            NpcKind::ArmorMerchant => "Sağlam bir zırh olmadan köyden çıkma derim.",
            NpcKind::Warehouse => "Eşyalarını bana emanet edebilirsin, depon her köyde seni bekler.",
            NpcKind::Stable => "Ahırda bir Ölümsüz Savaş Atı var. Bileti al, Pyungmoo'yu rüzgâr gibi dolaş!",
            NpcKind::FortuneTeller => "Gel evladım, kader taşları bugün sana ne söyleyecek bakalım.",
            NpcKind::Guard => "Köyün dışında Metin taşları düşmeye devam ediyor, dikkatli ol!",
            NpcKind::Teleporter => "Uzak diyarlara gitmek mi istiyorsun? Yolu açarım, yeter ki hazır ol.",
            NpcKind::Captain => "Jinno ordusu her zaman cesur savaşçılara ihtiyaç duyar.",
            NpcKind::Trainer => "Bedenini çelik, kılıcını yıldırım gibi yapmayı öğretirim.",
            NpcKind::Villager => "Selam, yabancı! Köyümüze hoş geldin.",
            NpcKind::Fisherman => "Şşş, balıkları kaçırma! Sabırlı olan en büyüğünü tutar.",
        }
    }
}

#[derive(Component)]
pub struct Npc {
    pub name: &'static str,
    pub title: &'static str,
    pub kind: NpcKind,
    /// `NPCS` tablosundaki satır.
    pub row: usize,
}

/// Bir NPC ile konuşuldu (tıklayıp menzile girince). Sprint 3 diyalog sistemi bunu dinleyecek.
#[derive(Message, Clone, Copy, Debug)]
pub struct NpcTalked {
    pub entity: Entity,
    pub kind: NpcKind,
}

/// NPC tablosunun bir satırı.
pub struct NpcDef {
    pub name: &'static str,
    pub title: &'static str,
    pub kind: NpcKind,
    /// Wiki harita koordinatı (harita metresi; x doğu, y güney).
    pub map: (f32, f32),
    /// Baktığı harita noktası.
    pub face: (f32, f32),
    /// 8 yuva: gömlek, göğüs, kemer, pantolon, çizme, bileklik, eldiven, tabard.
    pub armor: ArmorSet,
    /// Sağ eldeki eşya (ItemDisplayInfo).
    pub tool: Option<u32>,
    pub look: Appearance,
    pub greet: &'static str,
}

// Görünüm kısayolları.
const M_DARK: Appearance = Appearance::male(4, 3, 2, 1, 4);
const M_BROWN: Appearance = Appearance::male(1, 2, 7, 3, 1);
const M_BLOND: Appearance = Appearance::male(0, 5, 3, 2, 0);
const M_GRAY: Appearance = Appearance::male(2, 6, 1, 8, 5);
const F_YOUNG: Appearance = Appearance::female(1, 2, 4, 1, 0);
const F_OLD: Appearance = Appearance::female(2, 7, 2, 8, 0);

// Zırh setleri (ItemDisplayInfo; MPQ'da dokuları var).
const SMITH: ArmorSet = [None, Some(3719), Some(4473), Some(4469), Some(9773), Some(4471), Some(4470), None];
const MERCHANT: ArmorSet = [None, Some(2183), None, Some(1882), Some(2198), None, None, None];
const WEAPONS: ArmorSet = [None, Some(954), None, Some(685), Some(2229), Some(2230), Some(1407), None];
const ARMORS: ArmorSet = [None, Some(4151), Some(3254), Some(16092), Some(16112), None, Some(3773), None];
const KEEPER: ArmorSet = [None, Some(9009), Some(20210), Some(9945), Some(7081), Some(4094), None, None];
const GROOM: ArmorSet = [None, Some(2106), Some(3247), Some(2176), Some(2094), Some(2178), None, None];
const ROBE_BROWN: ArmorSet = [None, Some(9971), None, None, Some(3261), None, None, None];
const GUARD: ArmorSet =
    [None, Some(16183), Some(3418), Some(3414), Some(25828), Some(4448), Some(25831), Some(21337)];
const SENTRY: ArmorSet = [None, Some(3082), Some(34594), Some(3083), Some(16112), Some(3386), Some(3085), Some(3865)];
const MAGE: ArmorSet = [None, Some(30581), Some(30583), Some(30582), Some(30587), Some(30584), Some(30585), None];
const CAPTAIN: ArmorSet =
    [None, Some(29958), Some(29959), Some(29963), Some(29960), Some(29961), Some(29962), None];
const TRAINER: ArmorSet =
    [None, Some(31021), Some(31019), Some(31023), Some(31025), Some(31020), Some(31022), None];
const PRIEST: ArmorSet = [None, Some(30422), Some(30425), Some(30423), Some(30429), Some(30426), Some(30427), None];
const ROGUE: ArmorSet = [None, Some(31113), Some(31110), Some(31108), Some(31111), Some(31112), Some(31114), None];
const HEALER: ArmorSet = [None, Some(9986), None, None, Some(9773), None, None, None];
const PEASANT: ArmorSet = [None, Some(7574), None, Some(9988), Some(9992), None, None, None];
const FISHER: ArmorSet = [None, Some(2470), None, Some(1883), Some(3261), None, None, None];
const ROBE_WHITE: ArmorSet = [None, Some(9943), None, None, Some(16417), None, None, None];

// Eldeki eşyalar.
const HAMMER: u32 = 1022;
const POTION_BLUE: u32 = 1947;
const LONG_SWORD: u32 = 1550;
const MACE: u32 = 1160;
const LANTERN: u32 = 2746;
const PITCHFORK: u32 = 1490;
const POTION_PURPLE: u32 = 1946;
const HALBERD: u32 = 1733;
const SPEAR: u32 = 869;
const STAFF_BLUE: u32 = 1376;
const SILVER_SWORD: u32 = 4443;
const DRAGON_SWORD: u32 = 3758;
const STAFF_LONG: u32 = 1193;
const DAGGER: u32 = 2708;
const POTION_GREEN: u32 = 1948;
const PICK: u32 = 1378;
const FISHING_POLE: u32 = 1487;
const WALKING_STAFF: u32 = 1201;

/// Köy meydanı (NPC'lerin çoğu buraya bakar).
const SQUARE: (f32, f32) = (395.0, 660.0);

/// Pyungmoo NPC'leri (de-wiki Pyungmoo koordinatları, tr-wiki adları).
pub const NPCS: &[NpcDef] = &[
    NpcDef {
        name: "Demirci",
        title: "Eşya Yükseltme",
        kind: NpcKind::Blacksmith,
        map: (393.0, 692.0),
        face: SQUARE,
        armor: SMITH,
        tool: Some(HAMMER),
        look: M_DARK,
        greet: "Silahını güçlendirmeye mi geldin? Demir sabır ister, şans da cesaret!",
    },
    NpcDef {
        name: "Satıcı",
        title: "Genel Mağaza",
        kind: NpcKind::Merchant,
        map: (383.0, 693.0),
        face: SQUARE,
        armor: MERCHANT,
        tool: Some(POTION_BLUE),
        look: F_YOUNG,
        greet: "Hoş geldin yolcu! Kırmızı iksirlerim taze, fiyatlarım köyün en iyisi.",
    },
    NpcDef {
        name: "Silah Satıcısı",
        title: "Silahlar",
        kind: NpcKind::WeaponMerchant,
        map: (430.0, 607.0),
        face: SQUARE,
        armor: WEAPONS,
        tool: Some(LONG_SWORD),
        look: M_BROWN,
        greet: "Keskin bir kılıç, canavarlara karşı en sadık dostundur. Hangisi eline yakışır, bir bak.",
    },
    NpcDef {
        name: "Zırh Satıcısı",
        title: "Zırhlar",
        kind: NpcKind::ArmorMerchant,
        map: (403.0, 586.0),
        face: SQUARE,
        armor: ARMORS,
        tool: Some(MACE),
        look: M_DARK,
        greet: "Sağlam bir zırh olmadan köyden çıkma derim. Kurtların dişi acıtır!",
    },
    NpcDef {
        name: "Depocu",
        title: "Depo",
        kind: NpcKind::Warehouse,
        map: (315.0, 560.0),
        face: (330.0, 575.0),
        armor: KEEPER,
        tool: Some(LANTERN),
        look: M_GRAY,
        greet: "Eşyalarını bana emanet edebilirsin. Depon her köyde seni bekler.",
    },
    NpcDef {
        name: "Seyis",
        title: "Ahır",
        kind: NpcKind::Stable,
        map: (396.0, 735.0),
        face: (396.0, 715.0),
        armor: GROOM,
        tool: Some(PITCHFORK),
        look: M_BLOND,
        greet: "Ahırda bir Ölümsüz Savaş Atı var. Bileti al, Pyungmoo'yu rüzgâr gibi dolaş!",
    },
    NpcDef {
        name: "Yaşlı Kadın",
        title: "Falcı",
        kind: NpcKind::FortuneTeller,
        map: (417.0, 671.0),
        face: SQUARE,
        armor: ROBE_BROWN,
        tool: Some(POTION_PURPLE),
        look: F_OLD,
        greet: "Gel evladım, yüzünde bir yorgunluk var... Kader taşları bugün sana ne söyleyecek bakalım.",
    },
    NpcDef {
        name: "Köy Gardiyanı",
        title: "Köy Meydanı",
        kind: NpcKind::Guard,
        map: (383.0, 640.0),
        face: SQUARE,
        armor: GUARD,
        tool: Some(HALBERD),
        look: M_BROWN,
        greet: "Pyungmoo'da huzur var ama köyün dışında Metin taşları düşmeye devam ediyor. Dikkatli ol!",
    },
    NpcDef {
        name: "Şehir Bekçisi",
        title: "Bekçi",
        kind: NpcKind::Guard,
        map: (468.0, 714.0),
        face: (490.0, 725.0),
        armor: SENTRY,
        tool: Some(SPEAR),
        look: M_DARK,
        greet: "Kapıyı ben bekliyorum. Ötesinde vahşi kurtlar ve Beyaz Yeminliler var.",
    },
    NpcDef {
        name: "Işınlayıcı",
        title: "Işınlanma",
        kind: NpcKind::Teleporter,
        map: (432.0, 685.0),
        face: SQUARE,
        armor: MAGE,
        tool: Some(STAFF_BLUE),
        look: M_GRAY,
        greet: "Uzak diyarlara gitmek mi istiyorsun? Yolu açarım, yeter ki hazır ol.",
    },
    NpcDef {
        name: "Yüzbaşı",
        title: "Garnizon",
        kind: NpcKind::Captain,
        // Wiki koordinatı (286, 639) kışlanın içinde kalıyor (QA H1): kışlanın güneyindeki avluda, doğu kapısına bakar.
        map: (300.0, 663.0),
        face: (312.0, 662.0),
        armor: CAPTAIN,
        tool: Some(SILVER_SWORD),
        look: M_BROWN,
        greet: "Garnizonumuz Jinno'nun kalkanıdır. Asker olacaksan önce kılıcını sına!",
    },
    NpcDef {
        name: "Bedensel Savaş Öğretmeni",
        title: "Savaşçı Öğretmeni",
        kind: NpcKind::Trainer,
        map: (444.0, 623.0),
        face: SQUARE,
        armor: TRAINER,
        tool: Some(DRAGON_SWORD),
        look: M_BLOND,
        greet: "Bedenini çelik, kılıcını yıldırım gibi yap. Seviye 5'e gelince bana uğra, yolunu gösteririm.",
    },
    NpcDef {
        name: "Uriel",
        title: "Bilgin",
        kind: NpcKind::Villager,
        map: (425.0, 716.0),
        face: SQUARE,
        armor: PRIEST,
        tool: Some(STAFF_LONG),
        look: M_GRAY,
        greet: "Ah, genç savaşçı! Metin taşlarının sırrını çözmek için yardımına ihtiyacım var.",
    },
    NpcDef {
        name: "Octavio",
        title: "Gezgin",
        kind: NpcKind::Villager,
        map: (340.0, 747.0),
        face: SQUARE,
        armor: ROGUE,
        tool: Some(DAGGER),
        look: M_DARK,
        greet: "Uzak diyarlardan geldim. Bu köyün hikâyeleri hiç bitmiyor...",
    },
    NpcDef {
        name: "Baek-Go",
        title: "Şifacı",
        kind: NpcKind::Villager,
        map: (465.0, 612.0),
        face: SQUARE,
        armor: HEALER,
        tool: Some(POTION_GREEN),
        look: M_GRAY,
        greet: "Yaralarını sarayım mı? Ormandaki şifalı otları getirirsen ödülünü veririm.",
    },
    NpcDef {
        name: "Soon",
        title: "Köylü",
        kind: NpcKind::Villager,
        map: (454.0, 530.0),
        face: (440.0, 560.0),
        armor: PEASANT,
        tool: Some(PICK),
        look: M_BLOND,
        greet: "Köyün dışında tuhaf şeyler oluyor. Kurtlar her gece daha da cesurlaşıyor.",
    },
    NpcDef {
        name: "Balıkçı",
        title: "Balıkçı",
        kind: NpcKind::Fisherman,
        map: (428.0, 480.0),
        face: (428.0, 470.0),
        armor: FISHER,
        tool: Some(FISHING_POLE),
        look: M_BROWN,
        greet: "Şşş, balıkları kaçırma! Sabırlı olan en büyüğünü tutar.",
    },
    NpcDef {
        name: "Balıkçı",
        title: "Balıkçı",
        kind: NpcKind::Fisherman,
        map: (386.0, 849.0),
        face: (386.0, 860.0),
        armor: FISHER,
        tool: Some(FISHING_POLE),
        look: M_GRAY,
        greet: "Bugün sular sakin. Bir olta alıp sen de şansını denemelisin.",
    },
    NpcDef {
        name: "Yaşlı Adam",
        title: "Işınlayıcı",
        kind: NpcKind::Teleporter,
        map: (125.0, 1118.0),
        face: (140.0, 1110.0),
        armor: ROBE_WHITE,
        tool: Some(WALKING_STAFF),
        look: M_GRAY,
        greet: "1.000 Yang karşılığında seni Seungryong Vadisi'ne ya da Sohan Dağı'na ışınlayabilirim.",
    },
];

/// Elwynn (eski) modunda başlangıca göre (Bevy x, z) ofsetiyle doğan NPC'ler: yalnızca Demirci ve Satıcı.
const ELWYNN_OFFSETS: &[(usize, f32, f32, f32)] = &[(0, 5.0, -3.0, 0.6), (1, -5.0, -3.0, -0.6)];

/// Wiki koordinatı engelliyse en fazla bu kadar metre içinde açık nokta aranır.
const NUDGE_MAX: f32 = 6.0;

/// NPC ile bina ayak izi arasında istenen boşluk (m).
const NPC_CLEARANCE: f32 = 3.0;

/// Karşılama satırının sohbet kutusu rengi (sarı).
const C_NPC: Color = Color::srgb(1.0, 0.9, 0.3);

/// Model hazır olunca sağ ele takılacak eşya görünümü (NPC ve insan moblar).
#[derive(Component)]
pub struct WantsTool(pub Option<u32>);

/// Konuşma menzili (WoW etkileşim mesafesi ≈ 5 yarda).
const TALK_RANGE: f32 = 5.0;

pub fn plugin(app: &mut App) {
    app.add_message::<NpcTalked>().add_systems(
        Update,
        (
            spawn_npcs,
            attach_tools,
            (click_npc.after(super::combat::CombatInput), face_talker).chain(),
        ),
    );
}

/// Karşılama satırı: "<Ad>: <metin>".
pub fn greeting_line(def: &NpcDef) -> String {
    let text = if def.greet.is_empty() { def.kind.greeting() } else { def.greet };
    format!("{}: {}", def.name, text)
}

/// `p` açık değilse `max` metre içinde en yakın açık noktayı arar (halkalar, 0.5 m adım).
pub fn nudge(p: Vec2, max: f32, open: impl Fn(Vec2) -> bool) -> Option<Vec2> {
    if open(p) {
        return Some(p);
    }
    let mut r = 0.5;
    while r <= max + 1e-3 {
        let n = ((std::f32::consts::TAU * r) / 0.5).ceil().max(8.0) as usize;
        let mut best: Option<Vec2> = None;
        for i in 0..n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            let q = p + Vec2::new(a.cos(), a.sin()) * r;
            if open(q) {
                best = Some(q);
                break;
            }
        }
        if best.is_some() {
            return best;
        }
        r += 0.5;
    }
    None
}

/// Harita metresinde `from`'dan `to`'ya bakan dönüş (modeller -Z'ye bakar).
fn facing(from: Vec2, to: Vec2) -> Quat {
    let d = to - from;
    if d.length_squared() < 1e-4 {
        return Quat::IDENTITY;
    }
    Quat::from_rotation_arc(Vec3::NEG_Z, Vec3::new(d.x, 0.0, d.y).normalize())
}

fn spawn_npcs(
    mut commands: Commands,
    db: Option<Res<CharDb>>,
    mut wa: Option<ResMut<WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    server: Res<AssetServer>,
    col: benilla_world::collision::WorldCollision,
    mut done: Local<bool>,
    player: Query<&Player>,
) {
    if *done {
        return;
    }
    let (Some(db), Some(wa), Ok(p)) = (db, wa.as_deref_mut(), player.single()) else { return };
    if !p.grounded {
        return;
    }
    // (satır, konum, dönüş)
    let mut places: Vec<(usize, Vec3, Quat)> = Vec::new();
    match super::terrain::data() {
        Some(d) => {
            for (i, n) in NPCS.iter().enumerate() {
                let want = Vec2::new(n.map.0, n.map.1);
                // Bina görselleri ayak izinden biraz büyük: çevresinde de NPC_CLEARANCE kadar boşluk iste.
                let ok = |m: Vec2| {
                    !d.is_water(m)
                        && !d.blocked(m)
                        && (0..8).all(|k| {
                            let a = k as f32 * std::f32::consts::FRAC_PI_4;
                            !d.blocked(m + Vec2::new(a.cos(), a.sin()) * NPC_CLEARANCE)
                        })
                };
                let m = match nudge(want, NUDGE_MAX, ok) {
                    Some(m) => {
                        if m != want {
                            info!(
                                "npc: {} ({:.0}, {:.0}) engelli, ({:.1}, {:.1}) noktasına kaydırıldı",
                                n.name, want.x, want.y, m.x, m.y
                            );
                        }
                        m
                    }
                    None => {
                        warn!("npc: {} ({:.0}, {:.0}) çevresinde açık nokta yok, yerinde doğuyor", n.name, want.x, want.y);
                        want
                    }
                };
                places.push((i, d.to_world(m.x, m.y), facing(m, Vec2::new(n.face.0, n.face.1))));
            }
        }
        None => {
            let start = wow_to_bevy(super::start_wow());
            for &(i, dx, dz, yaw) in ELWYNN_OFFSETS {
                let mut at = start + Vec3::new(dx, 0.0, dz);
                match super::physics::ground_at(&col, at) {
                    Some(g) => at.y = g,
                    None => return, // arazi henüz yok, sonra dene
                }
                places.push((i, at, Quat::from_rotation_y(yaw)));
            }
        }
    }
    for (i, at, rot) in &places {
        let n = &NPCS[*i];
        let wm = build_body_as(&db, wa, &mut images, &server, &n.armor, &n.look);
        commands.spawn((
            wm,
            AnimState::default(),
            Transform::from_translation(*at).with_rotation(*rot),
            Visibility::default(),
            Npc { name: n.name, title: n.title, kind: n.kind, row: *i },
            WantsTool(n.tool),
        ));
    }
    info!("npc: {} NPC doğdu", places.len());
    *done = true;
}

fn attach_tools(
    mut commands: Commands,
    db: Option<Res<CharDb>>,
    mut wa: Option<ResMut<WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    server: Res<AssetServer>,
    q: Query<(Entity, &ModelReady, &WantsTool)>,
) {
    let (Some(db), Some(wa)) = (db, wa.as_deref_mut()) else { return };
    for (e, ready, tool) in &q {
        if let (Some(display), Some((joint, offset))) = (tool.0, ready.attachment(1)) {
            if let Some(wm) = build_item_model(&db, wa, &mut images, &server, display, "Weapon") {
                commands.spawn((wm, Transform::from_translation(offset), Visibility::default(), ChildOf(joint)));
            }
        }
        commands.entity(e).remove::<WantsTool>();
    }
}

/// NPC türüne göre pencereleri aç.
fn open(w: &mut Windows, kind: NpcKind) {
    match kind {
        NpcKind::Blacksmith => w.open_upgrade(),
        NpcKind::WeaponMerchant => w.open_shop(super::ui::ShopKind::Weapons),
        NpcKind::ArmorMerchant => w.open_shop(super::ui::ShopKind::Armor),
        k if k.is_shop() => w.open_shop(super::ui::ShopKind::General),
        _ => {}
    }
}

/// Konuşma: karşılama satırı + `NpcTalked` + pencere.
fn talk(
    entity: Entity,
    npc: &Npc,
    now: f32,
    w: &mut Windows,
    log: &mut GameLog,
    out: &mut MessageWriter<NpcTalked>,
) {
    let line = match NPCS.get(npc.row) {
        Some(def) => greeting_line(def),
        None => format!("{}: {}", npc.name, npc.kind.greeting()),
    };
    log.say(now, C_NPC, line);
    out.write(NpcTalked { entity, kind: npc.kind });
    open(w, npc.kind);
}

/// WoW: NPC'ye sağ tıklayınca konuşulur (menzildeyse); uzaksa uyarı, oyuncu kendiliğinden yürümez.
fn click_npc(
    time: Res<Time>,
    clicks: Res<Clicks>,
    window: Query<&Window, With<PrimaryWindow>>,
    cam: Query<(&Camera, &GlobalTransform), With<OrbitCam>>,
    npcs: Query<(Entity, &Transform, &Npc), Without<Player>>,
    player: Query<&Transform, With<Player>>,
    (mut w, mut log, mut out): (ResMut<Windows>, ResMut<GameLog>, MessageWriter<NpcTalked>),
) {
    if !clicks.right {
        return;
    }
    let (Ok(window), Ok(cam), Ok(pxf)) = (window.single(), cam.single(), player.single()) else { return };
    let Some(ray) = cursor_ray(window, cam) else { return };
    // Işına en yakın (önde) NPC.
    let mut best: Option<(f32, Entity, &Transform, &Npc)> = None;
    for (e, xf, npc) in &npcs {
        let c = xf.translation + Vec3::Y * 1.0;
        let oc = ray.origin - c;
        let b = oc.dot(*ray.direction);
        let disc = b * b - (oc.length_squared() - 0.9 * 0.9);
        if disc < 0.0 {
            continue;
        }
        let t = -b - disc.sqrt();
        if t < 0.0 {
            continue;
        }
        if best.as_ref().map_or(true, |(bt, ..)| t < *bt) {
            best = Some((t, e, xf, npc));
        }
    }
    let Some((_, e, xf, npc)) = best else { return };
    let now = time.elapsed_secs();
    if xf.translation.distance(pxf.translation) <= TALK_RANGE {
        talk(e, npc, now, &mut w, &mut log, &mut out);
    } else {
        log.push(now, super::log::C_BAD, "Çok uzaktasın.");
    }
}

/// Konuşulan NPC oyuncuya döner.
fn face_talker(
    mut talked: MessageReader<NpcTalked>,
    mut npcs: Query<(&mut Transform, &Npc), Without<Player>>,
    player: Query<&Transform, (With<Player>, Without<Npc>)>,
) {
    let Ok(p) = player.single() else {
        talked.clear();
        return;
    };
    for t in talked.read() {
        if let Ok((mut xf, npc)) = npcs.get_mut(t.entity) {
            let a = Vec2::new(xf.translation.x, xf.translation.z);
            let b = Vec2::new(p.translation.x, p.translation.z);
            xf.rotation = facing(a, b);
            info!("npc: konuşuldu {} ({:?})", npc.name, t.kind);
        }
    }
}

/// Uzaklaşınca NPC pencerelerini kapat (mağaza: herhangi bir satıcının yanında açık kalır).
pub fn auto_close(
    mut w: ResMut<Windows>,
    npcs: Query<(&Transform, &Npc)>,
    player: Query<&Transform, With<Player>>,
) {
    if !(w.upgrade || w.shop) {
        return;
    }
    let Ok(p) = player.single() else { return };
    let near = |f: &dyn Fn(NpcKind) -> bool| {
        npcs.iter().any(|(xf, n)| f(n.kind) && xf.translation.distance(p.translation) < 8.0)
    };
    if w.upgrade && !near(&|k| k == NpcKind::Blacksmith) {
        w.upgrade = false;
    }
    if w.shop && !near(&|k| k.is_shop()) {
        w.shop = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use std::collections::HashSet;

    #[test]
    fn table_has_no_duplicates() {
        let mut seen = HashSet::new();
        for n in NPCS {
            let key = (n.name, n.map.0 as i32, n.map.1 as i32);
            assert!(seen.insert(key), "yinelenen NPC: {key:?}");
        }
        assert!(NPCS.len() >= 16);
    }

    #[test]
    fn coords_inside_map() {
        for n in NPCS {
            for (x, y) in [n.map, n.face] {
                assert!((0.0..1024.0).contains(&x) && (0.0..1280.0).contains(&y), "{} dışarıda", n.name);
            }
        }
    }

    #[test]
    fn every_kind_has_greeting() {
        for k in NpcKind::ALL {
            assert!(!k.greeting().is_empty(), "{k:?}");
        }
        for n in NPCS {
            let line = greeting_line(n);
            assert!(line.starts_with(&format!("{}: ", n.name)));
            assert!(line.len() > n.name.len() + 4);
            assert!(!n.title.is_empty(), "{} unvansız", n.name);
        }
    }

    #[test]
    fn shop_kinds() {
        assert!(NpcKind::Merchant.is_shop());
        assert!(NpcKind::WeaponMerchant.is_shop());
        assert!(NpcKind::ArmorMerchant.is_shop());
        assert!(!NpcKind::Blacksmith.is_shop());
        assert!(!NpcKind::Fisherman.is_shop());
    }

    /// Her NPC'nin (kaydırma payı içinde) bina ve duvardan uzak bir yeri var (QA H1).
    #[test]
    fn every_npc_has_open_spot() {
        use crate::game::terrain::{in_building, in_wall};
        let free = |m: Vec2| {
            !(in_building(m, 0.0) || in_wall(m))
                && (0..8).all(|k| {
                    let a = k as f32 * std::f32::consts::FRAC_PI_4;
                    let q = m + Vec2::new(a.cos(), a.sin()) * NPC_CLEARANCE;
                    !(in_building(q, 0.0) || in_wall(q))
                })
        };
        for n in NPCS {
            assert!(nudge(Vec2::new(n.map.0, n.map.1), NUDGE_MAX, free).is_some(), "{} binanın içinde", n.name);
        }
        let cap = NPCS.iter().find(|n| n.kind == NpcKind::Captain).unwrap();
        assert!(free(Vec2::new(cap.map.0, cap.map.1)), "Yüzbaşı kaydırmasız açık noktada olmalı");
    }

    #[test]
    fn nudge_finds_nearest_open() {
        let p = Vec2::new(100.0, 100.0);
        assert_eq!(nudge(p, 6.0, |_| true), Some(p));
        // 3 m yarıçaplı daire engelli.
        let q = nudge(p, 6.0, |m| m.distance(p) >= 3.0).unwrap();
        let d = q.distance(p);
        assert!((3.0..3.6).contains(&d), "{d}");
        // 6 m'den büyük engel: bulunamaz.
        assert_eq!(nudge(p, 6.0, |m| m.distance(p) >= 10.0), None);
    }

    #[test]
    fn talk_writes_message_and_log() {
        let mut world = World::new();
        world.init_resource::<Windows>();
        world.init_resource::<GameLog>();
        world.init_resource::<Messages<NpcTalked>>();
        let rows: Vec<Entity> = [0usize, 2, 4]
            .iter()
            .map(|&i| {
                let n = &NPCS[i];
                world.spawn(Npc { name: n.name, title: n.title, kind: n.kind, row: i }).id()
            })
            .collect();
        for e in rows.clone() {
            world
                .run_system_once(
                    move |q: Query<&Npc>,
                          mut w: ResMut<Windows>,
                          mut log: ResMut<GameLog>,
                          mut out: MessageWriter<NpcTalked>| {
                        talk(e, q.get(e).unwrap(), 1.0, &mut w, &mut log, &mut out);
                    },
                )
                .unwrap();
        }
        let got: Vec<(Entity, NpcKind)> = world
            .run_system_once(|mut r: MessageReader<NpcTalked>| r.read().map(|m| (m.entity, m.kind)).collect::<Vec<_>>())
            .unwrap();
        assert_eq!(
            got,
            vec![
                (rows[0], NpcKind::Blacksmith),
                (rows[1], NpcKind::WeaponMerchant),
                (rows[2], NpcKind::Warehouse)
            ]
        );
        let log = world.resource::<GameLog>();
        assert_eq!(log.lines.len(), 3);
        assert!(log.lines[2].text.starts_with("Depocu: "));
        let w = world.resource::<Windows>();
        // Son konuşulan Silah Satıcısı mağazayı açtı (Demirci'nin yükseltmesini kapattı).
        assert!(w.shop && w.inventory && !w.upgrade);
    }
}
