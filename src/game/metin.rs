//! Metin taşları: büyük HP'li, saldırmayan dikilitaşlar. Vuruldukça (%80/%60/%40/%20) etrafına
//! mob dalgaları çağırır; kırılınca bol ganimet + EXP verir ve bir süre sonra başka yerde belirir.

use bevy::prelude::*;
use rand::Rng;

use benilla_assets::WorldAssets;
use benilla_world::collision::WorldCollision;
use benilla_world::world_point::WorldPoint;

use super::combat::{Dying, Enemy, Health, Hit, Killed};
use super::items::*;
use super::log::{GameLog, C_BAD, C_ITEM};
use super::mobs::{mob, open_ground, spawn_mob};
use super::model::{load_m2, AnimState, WowModel};
use super::player::Player;

#[derive(Clone, Copy, Debug)]
pub struct MetinDef {
    pub name: &'static str,
    pub level: u32,
    pub hp: i32,
    pub def: i32,
    pub exp: u32,
    pub tint: [f32; 3],
    pub glow: [f32; 3],
    pub scale: f32,
    /// Çağırdığı mob türleri.
    pub summons: &'static [u32],
    pub yang: (u32, u32),
    /// Ganimet tablosu: (vnum, adet, şans).
    pub drops: &'static [(u32, u16, f32)],
}

/// Pyungmoo metin taşları (tr-wiki): Üzüntü (5), Dövüş (10), Savaş (15), Hırs (20), Siyah (25).
pub const METINS: &[MetinDef] = &[
    MetinDef {
        name: "Üzüntü Metini",
        level: 5,
        hp: 1800,
        def: 2,
        exp: 450,
        tint: [0.75, 0.8, 1.0],
        glow: [0.2, 0.35, 1.0],
        scale: 0.42,
        summons: &[103, 104, 105, 106],
        yang: (250, 600),
        drops: &[
            (V_SPIRIT_STONE, 1, 0.5),
            (V_RED_POT_S, 5, 0.8),
            (V_BLESSING, 1, 0.15),
            (V_WOLF_CLAW, 3, 0.5),
            (V_WOLF_PELT, 2, 0.4),
            (20, 1, 0.25),
            (12200, 1, 0.15),
            (15000, 1, 0.15),
            (14000, 1, 0.12),
        ],
    },
    MetinDef {
        name: "Dövüş Metini",
        level: 10,
        hp: 4000,
        def: 5,
        exp: 1100,
        tint: [0.7, 1.0, 0.9],
        glow: [0.1, 0.9, 0.8],
        scale: 0.46,
        summons: &[107, 108, 111, 112, 109],
        yang: (600, 1300),
        drops: &[
            (V_SPIRIT_STONE, 2, 0.6),
            (V_RED_POT_M, 5, 0.7),
            (V_BLESSING, 1, 0.22),
            (V_BOAR_TUSK, 3, 0.5),
            (30, 1, 0.2),
            (35, 1, 0.12),
            (11210, 1, 0.1),
            (16005, 1, 0.12),
            (17005, 1, 0.12),
        ],
    },
    MetinDef {
        name: "Savaş Metini",
        level: 15,
        hp: 7200,
        def: 8,
        exp: 2300,
        tint: [1.0, 0.75, 0.7],
        glow: [1.0, 0.15, 0.05],
        scale: 0.5,
        summons: &[113, 114, 115, 116, 117, 118],
        yang: (1400, 2800),
        drops: &[
            (V_SPIRIT_STONE, 2, 0.7),
            (V_SOUL_CRYSTAL, 1, 0.25),
            (V_HANDBOOK, 1, 0.15),
            (V_BLESSING, 1, 0.3),
            (V_BEAR_PAW, 3, 0.5),
            (V_RED_POT_M, 8, 0.7),
            (40, 1, 0.18),
            (12210, 1, 0.1),
            (13000, 1, 0.15),
        ],
    },
    MetinDef {
        name: "Hırs Metini",
        level: 20,
        hp: 11000,
        def: 12,
        exp: 4200,
        tint: [1.1, 0.95, 0.6],
        glow: [1.0, 0.75, 0.1],
        scale: 0.56,
        summons: &[119, 120, 121, 122, 123, 124, 125, 126],
        yang: (2800, 5200),
        drops: &[
            (V_SOUL_CRYSTAL, 1, 0.5),
            (V_SPIRIT_STONE, 3, 0.7),
            (V_HANDBOOK, 1, 0.22),
            (V_BLESSING, 1, 0.35),
            (V_BEAR_GALL, 3, 0.5),
            (V_RED_POT_L, 4, 0.6),
            (50, 1, 0.12),
            (55, 1, 0.1),
            (11220, 1, 0.06),
            (14010, 1, 0.1),
            (15010, 1, 0.08),
        ],
    },
    MetinDef {
        name: "Siyah Metin",
        level: 25,
        hp: 16000,
        def: 16,
        exp: 7000,
        tint: [0.5, 0.45, 0.6],
        glow: [0.7, 0.1, 1.0],
        scale: 0.62,
        summons: &[127, 128, 129, 130, 131, 132, 133, 134, 136],
        yang: (4500, 9000),
        drops: &[
            (V_SOUL_CRYSTAL, 2, 0.7),
            (V_DRAGON_SCALE, 1, 0.12),
            (V_HANDBOOK, 1, 0.3),
            (V_BLESSING, 2, 0.4),
            (V_TIGER_PELT, 3, 0.5),
            (V_WHITE_RIBBON, 3, 0.4),
            (V_RED_POT_L, 6, 0.6),
            (60, 1, 0.12),
            (70, 1, 0.05),
            (11230, 1, 0.04),
            (13010, 1, 0.08),
            (16010, 1, 0.08),
            (17010, 1, 0.08),
            (14020, 1, 0.06),
        ],
    },
];

#[derive(Component)]
pub struct Metin {
    pub def: &'static MetinDef,
    /// Sonraki dalganın HP eşiği indeksi.
    pub wave: usize,
}

/// Taşın altındaki rün dairesi efekti.
#[derive(Component)]
pub struct MetinRune(pub Entity);

const WAVES: [f32; 4] = [0.8, 0.6, 0.4, 0.2];

/// Pyungmoo metin yuvaları: (tür, -, -). Wiki metin haritasındaki bölgelere düşer.
const PY_SLOTS: &[(usize, f32, f32)] = &[
    (0, 0.0, 0.0), (0, 0.0, 0.0), (0, 0.0, 0.0),
    (1, 0.0, 0.0), (1, 0.0, 0.0), (1, 0.0, 0.0),
    (2, 0.0, 0.0), (2, 0.0, 0.0), (2, 0.0, 0.0), (2, 0.0, 0.0),
    (3, 0.0, 0.0), (3, 0.0, 0.0), (3, 0.0, 0.0),
    (4, 0.0, 0.0), (4, 0.0, 0.0),
];

/// Metin türü için bölgede açık rastgele nokta. Üzüntü (yeşil şeritler) ve güneydeki Dövüş
/// şeritleri haritada soluk olduğundan elle tanımlı.
fn py_metin_point(d: &super::terrain::TerrainData, kind: usize) -> Option<Vec3> {
    let mut rng = rand::rng();
    for _ in 0..200 {
        let m = match kind {
            0 => Vec2::new(rng.random_range(690.0..815.0), rng.random_range(615.0..715.0)),
            1 if rng.random_bool(0.4) => Vec2::new(rng.random_range(300.0..524.0), rng.random_range(1003.0..1138.0)),
            _ => Vec2::new(rng.random_range(0.0..super::terrain::MAP_W), rng.random_range(0.0..super::terrain::MAP_H)),
        };
        let z = d.metin_zone(m);
        let fits = match kind {
            0 => true,
            1 => z == 2 || (m.y > 1000.0 && m.x < 530.0),
            _ => z as usize == kind + 1,
        };
        if fits && d.open(m) && (0..4).all(|k| {
            let a = k as f32 * std::f32::consts::FRAC_PI_2;
            d.open(m + Vec2::new(a.cos(), a.sin()) * 3.0)
        }) {
            return Some(d.to_world(m.x, m.y));
        }
    }
    None
}

/// Yerdeki ganimet (loot modülü işler).
#[derive(Message, Clone)]
pub struct DropLoot {
    pub at: Vec3,
    pub items: Vec<Item>,
    pub yang: u32,
}

#[derive(Resource, Default)]
struct MetinSpawner {
    /// (slot, bekleme sn)
    pending: Vec<(usize, f32)>,
    init: bool,
}

#[derive(Component)]
struct MetinSlot(usize);

pub fn plugin(app: &mut App) {
    app.add_message::<DropLoot>()
        .init_resource::<MetinSpawner>()
        .add_systems(Update, (spawn_metins, metin_waves, metin_killed, mob_drops, spin_runes));
}

fn spawn_metins(
    mut commands: Commands,
    time: Res<Time>,
    mut sp: ResMut<MetinSpawner>,
    server: Res<AssetServer>,
    col: WorldCollision,
    wp: WorldPoint,
    player: Query<(&Transform, &Player)>,
) {
    let Ok((pxf, p)) = player.single() else { return };
    if !p.grounded {
        return;
    }
    if !sp.init {
        sp.init = true;
        let n = if super::terrain::data().is_some() { PY_SLOTS.len() } else { super::elwynn::METIN_SLOTS.len() };
        sp.pending = (0..n).map(|i| (i, 0.0)).collect();
    }
    let mut rng = rand::rng();
    let dt = time.delta_secs();
    let mut placed = Vec::new();
    let td = super::terrain::data();
    for (k, (slot, wait)) in sp.pending.iter_mut().enumerate() {
        *wait -= dt;
        if *wait > 0.0 {
            continue;
        }
        let (kind, at) = if let Some(d) = td {
            let kind = PY_SLOTS[*slot % PY_SLOTS.len()].0;
            match py_metin_point(d, kind) {
                Some(p) => (kind, p),
                None => continue,
            }
        } else {
            // Elwynn: alan içinde rastgele açık nokta (alan henüz akmadıysa sonra tekrar).
            let (kind, x, y, z, r) = super::elwynn::METIN_SLOTS[*slot];
            let ang = rng.random_range(0.0..std::f32::consts::TAU);
            let d = r * rng.random_range(0.0f32..1.0).sqrt();
            let c = benilla_assets::coords::wow_to_bevy([x + ang.cos() * d, y + ang.sin() * d, z]);
            let Some(at) = open_ground(&col, &wp, c, z, 60.0) else {
                continue;
            };
            (kind, at)
        };
        let _ = &pxf;
        let def = &METINS[kind];
        let mut wm = WowModel::new(load_m2(
            &server,
            "World/Azeroth/BurningSteppes/PassiveDoodads/Rocks/LavaRock01.m2",
        ));
        wm.tint = Color::linear_rgb(def.tint[0], def.tint[1], def.tint[2]);
        wm.emissive = LinearRgba::rgb(def.glow[0] * 0.15, def.glow[1] * 0.15, def.glow[2] * 0.15);
        let yaw = rng.random_range(0.0..std::f32::consts::TAU);
        let e = commands
            .spawn((
                wm,
                Transform::from_translation(at - Vec3::Y * 0.3)
                    .with_rotation(Quat::from_rotation_y(yaw))
                    .with_scale(Vec3::splat(def.scale)),
                Visibility::default(),
                Enemy {
                    name: def.name.to_string(),
                    level: def.level,
                    radius: 1.2 / def.scale * 0.5,
                    height: 7.5,
                    exp: def.exp,
                    def: def.def,
                    is_metin: true,
                },
                Health {
                    hp: def.hp,
                    max: def.hp,
                },
                Metin { def, wave: 0 },
                MetinSlot(*slot),
            ))
            .id();
        // Rün dairesi + renkli ışık.
        let rune = commands
            .spawn((
                WowModel::new(load_m2(&server, "World/Goober/G_RuneGroundPurple01.m2")),
                AnimState::default(),
                Transform::from_translation(at + Vec3::Y * 0.05).with_scale(Vec3::splat(0.55)),
                Visibility::default(),
                MetinRune(e),
            ))
            .id();
        commands.spawn((
            PointLight {
                color: Color::linear_rgb(def.glow[0], def.glow[1], def.glow[2]),
                intensity: 60_000.0,
                range: 9.0,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_translation(at + Vec3::Y * 2.0),
            MetinRune(e),
        ));
        let _ = rune;
        info!("metin: {} @ {at}", def.name);
        placed.push(k);
    }
    for k in placed.into_iter().rev() {
        sp.pending.swap_remove(k);
    }
}

fn spin_runes(
    mut commands: Commands,
    time: Res<Time>,
    metins: Query<(), With<Metin>>,
    mut runes: Query<(Entity, &MetinRune, &mut Transform)>,
) {
    for (e, r, mut xf) in &mut runes {
        if metins.get(r.0).is_err() {
            commands.entity(e).despawn();
            continue;
        }
        xf.rotate_y(time.delta_secs() * 0.4);
    }
}

fn metin_waves(
    mut commands: Commands,
    mut hits: MessageReader<Hit>,
    server: Res<AssetServer>,
    mut wa: Option<ResMut<WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    col: WorldCollision,
    wp: WorldPoint,
    mut q: Query<(&mut Metin, &Health, &Transform), Without<Dying>>,
    time: Res<Time>,
    mut log: ResMut<GameLog>,
    db: Option<Res<super::character::CharDb>>,
) {
    let Some(wa) = wa.as_deref_mut() else { return };
    let mut rng = rand::rng();
    for h in hits.read() {
        let Ok((mut m, hp, xf)) = q.get_mut(h.target) else {
            continue;
        };
        let frac = hp.hp as f32 / hp.max as f32;
        while m.wave < WAVES.len() && frac <= WAVES[m.wave] && hp.hp > 0 {
            m.wave += 1;
            let n = rng.random_range(3..=5);
            for _ in 0..n {
                // Elwynn'de bölgenin WoW yaratıkları, Pyungmoo'da Metin2 canavarları.
                let pool = if super::terrain::data().is_some() {
                    m.def.summons
                } else {
                    super::elwynn::metin_summons(METINS.iter().position(|d| std::ptr::eq(d, m.def)).unwrap_or(0))
                };
                let k = pool[rng.random_range(0..pool.len())];
                let ang = rng.random_range(0.0..std::f32::consts::TAU);
                let off = Vec3::new(ang.cos() * 4.5, 0.0, ang.sin() * 4.5);
                let at = open_ground(&col, &wp, xf.translation + off, xf.translation.y, 4.0)
                    .unwrap_or(xf.translation + off);
                spawn_mob(&mut commands, &server, wa, &mut images, db.as_deref(), mob(k), at, None, true);
            }
            log.push(
                time.elapsed_secs(),
                C_BAD,
                format!("{} etrafına canavar çağırdı!", m.def.name),
            );
        }
    }
}

fn roll_drops(def: &MetinDef) -> (Vec<Item>, u32) {
    let mut rng = rand::rng();
    let mut items = Vec::new();
    for &(vnum, count, chance) in def.drops {
        if rng.random_bool(chance as f64) {
            let mut it = Item::stack(vnum, count);
            // Silah/zırh düşerse bazen +1..+3 gelsin (Metin2 hissi).
            if it.def().upgradable() {
                it.count = 1;
                let r: f32 = rng.random();
                it.plus = if r < 0.6 { 0 } else if r < 0.85 { 1 } else if r < 0.96 { 2 } else { 3 };
            }
            items.push(it);
        }
    }
    let yang = rng.random_range(def.yang.0..=def.yang.1);
    (items, yang)
}

fn metin_killed(
    mut killed: MessageReader<Killed>,
    q: Query<(&Metin, &MetinSlot)>,
    mut sp: ResMut<MetinSpawner>,
    mut drops: MessageWriter<DropLoot>,
    mut log: ResMut<GameLog>,
    time: Res<Time>,
) {
    for k in killed.read() {
        let Ok((m, slot)) = q.get(k.entity) else { continue };
        let (items, yang) = roll_drops(m.def);
        log.push(
            time.elapsed_secs(),
            C_ITEM,
            format!("{} parçalandı!", m.def.name),
        );
        drops.write(DropLoot {
            at: k.pos,
            items,
            yang,
        });
        sp.pending.push((slot.0, 40.0));
    }
}

/// Mob ganimeti: Yang, türe özgü malzeme (Kurt Postu, Ayı Safrası...), iksir, nadiren seviyeye uygun
/// ekipman; Beyaz Yeminliler takı düşürür (wiki listesi), mini patronlar bol ganimet.
fn mob_drops(
    mut killed: MessageReader<Killed>,
    q: Query<&super::mobs::Mob>,
    mut drops: MessageWriter<DropLoot>,
) {
    use super::mobs::Species;
    let mut rng = rand::rng();
    for k in killed.read() {
        let Ok(m) = q.get(k.entity) else { continue };
        let d = m.def;
        let mut items = Vec::new();
        let boost = if d.boss { 3.0 } else { 1.0 };
        for &(v, chance) in d.drops() {
            if rng.random_bool((chance * boost).min(1.0) as f64) {
                items.push(Item::stack(v, if d.boss { 3 } else { 1 }));
            }
        }
        if rng.random_bool(0.12) {
            items.push(Item::new(match d.level {
                0..=8 => V_RED_POT_S,
                9..=19 => V_RED_POT_M,
                _ => V_RED_POT_L,
            }));
        }
        if rng.random_bool(if d.boss { 0.8 } else { 0.03 }) {
            items.push(Item::new(V_SPIRIT_STONE));
        }
        if d.boss && rng.random_bool(0.5) {
            items.push(Item::new(V_SOUL_CRYSTAL));
        }
        if d.boss && rng.random_bool(0.3) {
            items.push(Item::new(V_BLESSING));
        }
        // Takılar: Beyaz Yeminli askerler (Bakır Küpe, Gümüş Bilezik, Abanoz Kolye...).
        if d.species == Species::Human && rng.random_bool(0.08) {
            let pool: &[u32] = &[17005, 16005, 14010, 17010, 14020, 16010, 17020, 16020, 17030, 16030];
            items.push(Item::new(pool[rng.random_range(0..pool.len())]));
        }
        let gear_chance = if d.boss { 0.9 } else if d.species == Species::Human { 0.05 } else { 0.025 };
        if rng.random_bool(gear_chance) {
            let pool = gear_for_level(d.level);
            if !pool.is_empty() {
                let mut it = Item::new(pool[rng.random_range(0..pool.len())]);
                if d.boss {
                    it.plus = rng.random_range(1..=4);
                }
                items.push(it);
            }
        }
        let yang = rng.random_range(d.level * 4..=d.level * 12 + 10) * if d.boss { 8 } else { 1 };
        drops.write(DropLoot {
            at: k.pos,
            items,
            yang,
        });
    }
}
