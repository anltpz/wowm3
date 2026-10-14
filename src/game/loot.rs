//! Yerdeki ganimet: düşen eşyalar etrafa saçılır, adları yerde görünür. Z tuşu (veya üstünden
//! geçince Yang) ile toplanır; 2 dakika sonra kaybolur.

use bevy::prelude::*;
use rand::Rng;

use super::inventory::Inventory;
use super::items::{Item, Kind};
use super::log::{GameLog, C_BAD, C_ITEM};
use super::metin::DropLoot;
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

#[derive(Resource)]
struct LootAssets {
    mesh: Handle<Mesh>,
    coin: Handle<StandardMaterial>,
    gear: Handle<StandardMaterial>,
    misc: Handle<StandardMaterial>,
    rare: Handle<StandardMaterial>,
}

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(Update, (spawn_drops, bob, pickup));
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let m = |c: Color, e: LinearRgba| StandardMaterial {
        base_color: c,
        emissive: e,
        perceptual_roughness: 0.4,
        metallic: 0.6,
        ..default()
    };
    commands.insert_resource(LootAssets {
        mesh: meshes.add(Cuboid::new(0.28, 0.28, 0.28)),
        coin: mats.add(m(Color::srgb(1.0, 0.8, 0.2), LinearRgba::rgb(0.8, 0.5, 0.0))),
        gear: mats.add(m(Color::srgb(0.85, 0.85, 0.95), LinearRgba::rgb(0.3, 0.3, 0.4))),
        misc: mats.add(m(Color::srgb(0.5, 1.0, 0.5), LinearRgba::rgb(0.1, 0.5, 0.1))),
        rare: mats.add(m(Color::srgb(1.0, 0.5, 0.2), LinearRgba::rgb(1.5, 0.5, 0.0))),
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
            let mat = match item {
                None => a.coin.clone(),
                Some(it) if it.plus >= 3 || matches!(it.def().kind, Kind::DragonScale) => {
                    a.rare.clone()
                }
                Some(it) if it.def().upgradable() => a.gear.clone(),
                Some(_) => a.misc.clone(),
            };
            commands.spawn((
                GroundItem {
                    item,
                    yang,
                    age: 0.0,
                    base_y: p.y + 0.25,
                },
                Mesh3d(a.mesh.clone()),
                MeshMaterial3d(mat),
                Transform::from_translation(p + Vec3::Y * 0.25),
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
        xf.translation.y = g.base_y + (g.age * 2.5).sin() * 0.06;
        xf.rotate_y(time.delta_secs() * 1.5);
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
