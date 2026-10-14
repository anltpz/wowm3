//! Dünyaya bağlı UI: düşman isim plakaları (WoW 1.12 nameplate: Nameplate-Border + UI-StatusBar, 128×32,
//! sabit yerleşim), NPC/oyuncu adları, yükselen hasar sayıları, yerdeki eşya adları ve hedef dairesi.

use std::collections::HashMap;

use bevy::prelude::*;

use super::{hud::name_color, UiAssets};
use crate::game::camera::OrbitCam;
use crate::game::combat::{Dying, Enemy, FloatText, Health, PlayerTarget};
use crate::game::loot::GroundItem;
use crate::game::npc::Npc;
use crate::game::player::Player;
use crate::game::stats::PlayerStats;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Enemy,
    Npc,
    Ground,
    Float,
}

struct Plate {
    root: Entity,
    text: Entity,
    fill: Option<Entity>,
    level: Option<Entity>,
    kind: Kind,
}

/// WoW isim plakası dokuları.
#[derive(Resource)]
struct PlateTex {
    border: Handle<Image>,
    status: Handle<Image>,
}

/// WoW nameplate: 128×32 doku; can çubuğu yuvası x 5-105, y 19-29; seviye kutusu x 107-125.
const NP_W: f32 = 128.0;
const NP_H: f32 = 32.0;

#[derive(Resource, Default)]
struct Plates(HashMap<Entity, Plate>);

#[derive(Component)]
struct TargetRing;

pub fn plugin(app: &mut App) {
    app.init_resource::<Plates>()
        .add_systems(Startup, spawn_ring)
        .add_systems(
            PostUpdate,
            update_plates
                .after(bevy::transform::TransformSystems::Propagate)
                .run_if(resource_exists::<UiAssets>),
        )
        .add_systems(Update, ring_follow);
}

/// WoW hedef dairesi (Interface/SelectionCircle/Selection_SelectionA.blp), düşmanda kırmızı.
const SELECTION_CIRCLE: &str = "Interface/SelectionCircle/Selection_SelectionA.blp";

fn spawn_ring(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let tex: Handle<Image> = server.load(benilla_assets::texture_url(SELECTION_CIRCLE, (false, false)));
    commands.insert_resource(PlateTex {
        border: server.load(benilla_assets::texture_url("Interface/Tooltips/Nameplate-Border.blp", (false, false))),
        status: server.load(benilla_assets::texture_url("Interface/TargetingFrame/UI-StatusBar.blp", (false, false))),
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Z, Vec2::splat(1.0)))),
        MeshMaterial3d(mats.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.2, 0.15),
            base_color_texture: Some(tex),
            unlit: true,
            alpha_mode: AlphaMode::Add,
            double_sided: true,
            cull_mode: None,
            ..default()
        })),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        Visibility::Hidden,
        TargetRing,
    ));
}

fn ring_follow(
    time: Res<Time>,
    target: Res<PlayerTarget>,
    enemies: Query<(&Transform, &Enemy), Without<TargetRing>>,
    mut ring: Query<(&mut Transform, &mut Visibility), With<TargetRing>>,
) {
    let Ok((mut xf, mut vis)) = ring.single_mut() else { return };
    match target.0.and_then(|t| enemies.get(t).ok()) {
        Some((t, en)) => {
            *vis = Visibility::Inherited;
            let r = (en.radius * t.scale.x).max(0.6) + 0.3;
            xf.translation = t.translation + Vec3::Y * 0.08;
            xf.scale = Vec3::splat(r);
            let _ = &time;
            xf.rotation = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
        }
        None => *vis = Visibility::Hidden,
    }
}

fn spawn_plate(commands: &mut Commands, ui: &UiAssets, tex: &PlateTex, kind: Kind) -> Plate {
    let (size, width) = match kind {
        Kind::Float => (22.0, 160.0),
        Kind::Ground => (12.0, 220.0),
        Kind::Enemy => (11.0, NP_W),
        Kind::Npc => (13.0, 220.0),
    };
    let font = if kind == Kind::Float { ui.num_font.clone() } else { ui.font.clone() };
    let abs = |left: f32, top: f32, w: f32, h: f32| Node {
        position_type: PositionType::Absolute,
        left: px(left),
        top: px(top),
        width: px(w),
        height: px(h),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };
    let label = |text: &str, size: f32| {
        (
            Text::new(text.to_string()),
            TextFont { font: font.clone(), font_size: size, ..default() },
            TextColor(Color::WHITE),
            TextShadow { offset: Vec2::new(1.0, 1.0), color: Color::BLACK },
        )
    };
    if kind != Kind::Enemy {
        let root = commands
            .spawn((
                Node { position_type: PositionType::Absolute, width: px(width), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() },
                Visibility::Hidden,
                ZIndex(-1),
            ))
            .id();
        let text = commands.spawn((label("", size), ChildOf(root))).id();
        return Plate { root, text, fill: None, level: None, kind };
    }
    // WoW nameplate: ad üstte, altın çerçeveli can çubuğu ve seviye kutusu.
    let root = commands
        .spawn((Node { position_type: PositionType::Absolute, width: px(NP_W), height: px(NP_H), ..default() }, Visibility::Hidden, ZIndex(-1)))
        .id();
    let name_box = commands.spawn((abs(-36.0, 0.0, NP_W + 72.0, 16.0), ChildOf(root))).id();
    let text = commands.spawn((label("", size), ChildOf(name_box))).id();
    let bar = commands
        .spawn((
            abs(5.0, 19.0, 100.0, 10.0),
            ImageNode { image: tex.status.clone(), color: Color::srgba(0.0, 0.0, 0.0, 0.7), ..default() },
            ChildOf(root),
        ))
        .id();
    let fill = commands
        .spawn((
            Node { position_type: PositionType::Absolute, left: px(0), top: px(0), width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
            ImageNode { image: tex.status.clone(), color: Color::srgb(1.0, 0.0, 0.0), ..default() },
            ChildOf(bar),
        ))
        .id();
    commands.spawn((abs(0.0, 0.0, NP_W, NP_H), ImageNode::new(tex.border.clone()), ChildOf(root)));
    let lvl_box = commands.spawn((abs(107.0, 18.0, 18.0, 12.0), ChildOf(root))).id();
    let level = commands.spawn((label("", 9.0), ChildOf(lvl_box))).id();
    Plate { root, text, fill: Some(fill), level: Some(level), kind }
}

fn update_plates(
    mut commands: Commands,
    ui: Res<UiAssets>,
    tex: Option<Res<PlateTex>>,
    mut plates: ResMut<Plates>,
    cam: Query<(&Camera, &GlobalTransform), With<OrbitCam>>,
    player: Query<(Entity, &GlobalTransform), With<Player>>,
    stats: Res<PlayerStats>,
    target: Res<PlayerTarget>,
    enemies: Query<(Entity, &GlobalTransform, &Enemy, &Health, Option<&Dying>)>,
    npcs: Query<(Entity, &GlobalTransform, &Npc)>,
    ground: Query<(Entity, &GlobalTransform, &GroundItem)>,
    floats: Query<(Entity, &FloatText)>,
    mut nodes: Query<(&mut Node, &mut Visibility)>,
    mut texts: Query<(&mut Text, &mut TextColor, &mut TextFont)>,
) {
    let (Ok((camera, cgt)), Ok((pe, pgt)), Some(tex)) = (cam.single(), player.single(), tex) else {
        return;
    };
    let ppos = pgt.translation();
    // (sahip, tür, dünya konumu, metin, renk, hp oranı?, yazı boyutu?)
    let mut want: Vec<(Entity, Kind, Vec3, String, Color, Option<f32>, Option<f32>)> = Vec::new();
    for (e, gt, en, hp, dying) in &enemies {
        let p = gt.translation();
        if p.distance(ppos) > 45.0 || dying.is_some() {
            continue;
        }
        let s = gt.compute_transform().scale.y;
        let frac = hp.hp as f32 / hp.max.max(1) as f32;
        let _ = &target;
        want.push((
            e,
            Kind::Enemy,
            p + Vec3::Y * (en.height * s + 0.35),
            en.name.to_string(),
            name_color(en, stats.level),
            Some(frac),
            Some(en.level as f32),
        ));
    }
    if !stats.dead {
        want.push((
            pe,
            Kind::Npc,
            // Binekteyken binici daha yüksekte.
            ppos + Vec3::Y * if stats.riding { 3.2 } else { 2.15 },
            format!("{} Lv.{}
<Lonca: Kardeşlik>", stats.name, stats.level),
            Color::srgb(0.35, 1.0, 0.35),
            None,
            None,
        ));
    }
    for (e, gt, npc) in &npcs {
        let p = gt.translation();
        if p.distance(ppos) > 45.0 {
            continue;
        }
        want.push((
            e,
            Kind::Npc,
            p + Vec3::Y * 2.45,
            format!("{}\n<{}>", npc.name, npc.title),
            Color::srgb(0.55, 0.85, 1.0),
            None,
            None,
        ));
    }
    for (e, gt, g) in &ground {
        let p = gt.translation();
        if p.distance(ppos) > 25.0 {
            continue;
        }
        want.push((e, Kind::Ground, p + Vec3::Y * 0.45, g.label(), g.color(), None, None));
    }
    for (e, f) in &floats {
        let a = (1.0 - (f.t - 0.7).max(0.0) / 0.5).clamp(0.0, 1.0);
        let size = if f.big { 30.0 } else { 22.0 } * (1.0 + (0.15 - f.t).max(0.0) * 3.0);
        want.push((e, Kind::Float, f.world, f.text.clone(), f.color.with_alpha(a), None, Some(size)));
    }

    // Artık olmayan sahiplerin plakalarını sil.
    let live: std::collections::HashSet<Entity> = want.iter().map(|w| w.0).collect();
    plates.0.retain(|owner, p| {
        if live.contains(owner) {
            true
        } else {
            commands.entity(p.root).despawn();
            false
        }
    });

    // Yakından uzağa: çakışan NPC plakalarında yakındaki kalır (QA H4).
    let cpos = cgt.translation();
    want.sort_by(|a, b| a.2.distance_squared(cpos).total_cmp(&b.2.distance_squared(cpos)));
    let mut placed: Vec<Vec2> = Vec::new();
    for (owner, kind, world, s, color, frac, size) in want {
        let plate = plates
            .0
            .entry(owner)
            .or_insert_with(|| spawn_plate(&mut commands, &ui, &tex, kind));
        if plate.kind != kind {
            continue;
        }
        let Ok(vp) = camera.world_to_viewport(cgt, world) else {
            if let Ok((_, mut v)) = nodes.get_mut(plate.root) {
                *v = Visibility::Hidden;
            }
            continue;
        };
        // Kameranın arkası.
        let fwd = cgt.forward();
        let behind = (world - cgt.translation()).dot(*fwd) < 0.0;
        let mut hidden = behind;
        // Bina arkasındaki plakalar görünmez.
        if !hidden && matches!(kind, Kind::Npc | Kind::Enemy) && owner != pe {
            hidden = crate::game::terrain::occluded(cpos, world);
        }
        // NPC / oyuncu plakaları üst üste binmesin.
        if !hidden && kind == Kind::Npc {
            if placed.iter().any(|p| (p.x - vp.x).abs() < 120.0 && (p.y - vp.y).abs() < 30.0) {
                hidden = true;
            } else {
                placed.push(vp);
            }
        }
        // Plaka, dünya noktasının üstünde ortalanır (düşman plakası sabit 128×32, kaymaz).
        let (width, h) = match kind {
            Kind::Float => (160.0, 16.0),
            Kind::Enemy => (NP_W, NP_H),
            Kind::Npc => (220.0, 34.0),
            Kind::Ground => (220.0, 16.0),
        };
        if let Ok((mut n, mut v)) = nodes.get_mut(plate.root) {
            *v = if hidden { Visibility::Hidden } else { Visibility::Inherited };
            n.left = px((vp.x - width / 2.0).round());
            n.top = px((vp.y - h).round());
        }
        if let Ok((mut t, mut c, mut f)) = texts.get_mut(plate.text) {
            if t.0 != s {
                t.0 = s;
            }
            c.0 = color;
            if let (Some(sz), Kind::Float) = (size, kind) {
                f.font_size = sz;
            }
        }
        if let (Some(fill), Some(fr)) = (plate.fill, frac) {
            if let Ok((mut n, _)) = nodes.get_mut(fill) {
                n.width = Val::Percent(fr.clamp(0.0, 1.0) * 100.0);
            }
        }
        if let (Some(level), Some(lv)) = (plate.level, size) {
            if let Ok((mut t, _, _)) = texts.get_mut(level) {
                let v = format!("{}", lv as u32);
                if t.0 != v {
                    t.0 = v;
                }
            }
        }
    }
}
