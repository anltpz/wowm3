//! Yerdeki ganimet: düşen eşyalar etrafa saçılır, adları yerde görünür. Z tuşu (veya üstünden
//! geçince Yang) ile toplanır; 2 dakika sonra kaybolur.

use bevy::prelude::*;
use rand::Rng;

use super::inventory::Inventory;
use super::items::{Item, Kind};
use super::log::{GameLog, C_BAD, C_ITEM};
use super::metin::DropLoot;
use super::model::{load_m2, WowModel};
use super::player::Player;
use super::stats::PlayerStats;

#[derive(Component)]
pub struct GroundItem {
    pub item: Option<Item>,
    pub yang: u32,
    pub age: f32,
    pub base_y: f32,
}

impl GroundItem {
    pub fn label(&self) -> String {
        match self.item {
            Some(it) if it.count > 1 => format!("{} ({})", it.display_name(), it.count),
            Some(it) => it.display_name(),
            None => format!("{} Yang", self.yang),
        }
    }
    pub fn color(&self) -> Color {
        match self.item {
            None => Color::srgb(1.0, 0.85, 0.3),
            Some(it) => item_color(&it),
        }
    }
}

/// Eşya adı rengi (Metin2: artılı/nadir eşyalar renkli).
pub fn item_color(it: &Item) -> Color {
    match it.def().kind {
        Kind::Weapon { .. } | Kind::Armor { .. } | Kind::Wear { .. } => match it.plus {
            0 => Color::srgb(0.95, 0.95, 0.95),
            1..=3 => Color::srgb(0.55, 0.85, 1.0),
            4..=6 => Color::srgb(0.75, 0.55, 1.0),
            _ => Color::srgb(1.0, 0.6, 0.2),
        },
        Kind::Blessing | Kind::Handbook | Kind::DragonScale => Color::srgb(1.0, 0.8, 0.3),
        Kind::Material => Color::srgb(0.7, 1.0, 0.7),
        Kind::Potion { .. } => Color::srgb(1.0, 0.6, 0.6),
        Kind::MountTicket { .. } | Kind::MountMedal => Color::srgb(0.6, 0.9, 1.0),
    }
}

/// Yerdeki ganimet modelleri (WoW doodad'ları).
#[derive(Resource)]
struct LootAssets {
    gold: Handle<benilla_assets::M2Model>,
    sack: Handle<benilla_assets::M2Model>,
    chest: Handle<benilla_assets::M2Model>,
}

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(Update, (spawn_drops, bob, pickup));
}

fn setup(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(LootAssets {
        gold: load_m2(&server, "World/Generic/PassiveDoodads/TreasurePiles/GoldPilesmall01.m2"),
        sack: load_m2(&server, "World/Generic/PassiveDoodads/Furniture/Containers/Sack01.m2"),
        chest: load_m2(&server, "World/SkillActivated/Containers/TreasureChest01.m2"),
    });
}

fn spawn_drops(
    mut commands: Commands,
    mut ev: MessageReader<DropLoot>,
    assets: Option<Res<LootAssets>>,
    col: benilla_world::collision::WorldCollision,
) {
    let Some(a) = assets else { return };
    let mut rng = rand::rng();
    for d in ev.read() {
        let mut spawn = |item: Option<Item>, yang: u32| {
            let ang = rng.random_range(0.0..std::f32::consts::TAU);
            let r = rng.random_range(0.8..2.6);
            let mut p = d.at + Vec3::new(ang.cos() * r, 0.0, ang.sin() * r);
            if let Some(g) = super::physics::ground_at(&col, p) {
                p.y = g;
            }
            // Yang: altın yığını, nadir: hazine sandığı, diğerleri: çuval.
            let (model, scale) = match item {
                None => (a.gold.clone(), 0.6),
                Some(it) if it.plus >= 3 || matches!(it.def().kind, Kind::DragonScale) => (a.chest.clone(), 0.35),
                Some(_) => (a.sack.clone(), 0.45),
            };
            let mut wm = WowModel::new(model);
            wm.shared = true;
            commands.spawn((
                GroundItem {
                    item,
                    yang,
                    age: 0.0,
                    base_y: p.y,
                },
                wm,
                Transform::from_translation(p).with_scale(Vec3::splat(scale)),
                Visibility::default(),
            ));
        };
        if d.yang > 0 {
            spawn(None, d.yang);
        }
        for it in &d.items {
            spawn(Some(*it), 0);
        }
    }
}

fn bob(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut GroundItem, &mut Transform)>) {
    for (e, mut g, mut xf) in &mut q {
        g.age += time.delta_secs();
        xf.translation.y = g.base_y;
        if g.age > 120.0 {
            commands.entity(e).despawn();
        }
    }
}

fn pickup(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    player: Query<&Transform, With<Player>>,
    q: Query<(Entity, &GroundItem, &Transform)>,
    mut inv: ResMut<Inventory>,
    stats: Res<PlayerStats>,
    mut log: ResMut<GameLog>,
) {
    let Ok(pxf) = player.single() else { return };
    if stats.dead {
        return;
    }
    let now = time.elapsed_secs();
    let z = keys.just_pressed(KeyCode::KeyZ) || keys.just_pressed(KeyCode::Backquote);
    let mut full_warned = false;
    for (e, g, xf) in &q {
        let d = xf.translation.distance(pxf.translation);
        let take_yang = g.item.is_none() && (d < 1.5 || (z && d < 4.0));
        let take_item = g.item.is_some() && z && d < 4.0;
        if take_yang {
            inv.yang += g.yang as u64;
            inv.touch();
            log.push(now, C_ITEM, format!("{} Yang aldın.", g.yang));
            commands.entity(e).despawn();
        } else if take_item {
            let it = g.item.unwrap();
            match inv.add(it) {
                None => {
                    log.push(now, C_ITEM, format!("{} aldın.", GroundItem { item: Some(it), yang: 0, age: 0.0, base_y: 0.0 }.label()));
                    commands.entity(e).despawn();
                }
                Some(_) if !full_warned => {
                    full_warned = true;
                    log.push(now, C_BAD, "Envanterde yeterli yer yok.");
                }
                Some(_) => {}
            }
        }
    }
}
