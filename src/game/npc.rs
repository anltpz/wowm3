//! Köy NPC'leri: Demirci (artı basma) ve Tüccar (alışveriş). Tıklanınca yaklaşır, menzilde pencere açar.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use benilla_assets::coords::wow_to_bevy;
use benilla_assets::WorldAssets;

use super::camera::OrbitCam;
use super::character::{build_body, build_item_model, ArmorSet, CharDb};
use super::combat::{cursor_ray, MoveTo, PlayerTarget, UiHover};
use super::model::{AnimState, ModelReady};
use super::player::Player;
use super::ui::Windows;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NpcKind {
    Blacksmith,
    Merchant,
}

#[derive(Component)]
pub struct Npc {
    pub name: &'static str,
    pub title: &'static str,
    pub kind: NpcKind,
}

/// Tıklanan, yürüyerek ulaşılmayı bekleyen NPC.
#[derive(Resource, Default)]
struct PendingTalk(Option<Entity>);

/// Model hazır olunca sağ ele takılacak eşya görünümü (NPC ve insan moblar).
#[derive(Component)]
pub struct WantsTool(pub Option<u32>);

/// Başlangıca göre (Bevy x, z) ofset, bakış yönü (yaw).
const NPCS: &[(NpcKind, &str, &str, f32, f32, f32, ArmorSet, Option<u32>)] = &[
    (
        NpcKind::Blacksmith,
        "Demirci",
        "Eşya Yükseltme",
        5.0,
        -3.0,
        0.6,
        [None, Some(2163), None, Some(1883), Some(2198), None, Some(2164), None],
        Some(1022),
    ),
    (
        NpcKind::Merchant,
        "Tüccar",
        "Genel Mağaza",
        -5.0,
        -3.0,
        -0.6,
        [None, Some(2183), None, Some(1882), Some(2198), None, None, None],
        None,
    ),
];

const TALK_RANGE: f32 = 4.0;

pub fn plugin(app: &mut App) {
    app.init_resource::<PendingTalk>()
        .add_systems(Update, (spawn_npcs, attach_tools, click_npc.after(super::combat::CombatInput), arrive));
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
    let start = wow_to_bevy(super::start_wow());
    for (kind, name, title, dx, dz, yaw, armor, tool) in NPCS {
        let mut at = match (super::terrain::data(), kind) {
            // Wiki: Demirci (393, 692), Satıcı (383, 693).
            (Some(d), NpcKind::Blacksmith) => d.to_world(396.0, 688.0),
            (Some(d), NpcKind::Merchant) => d.to_world(374.0, 690.0),
            _ => start + Vec3::new(*dx, 0.0, *dz),
        };
        match super::physics::ground_at(&col, at) {
            Some(g) => at.y = g,
            None => return, // arazi henüz yok, sonra dene
        }
        let wm = build_body(&db, wa, &mut images, &server, armor);
        commands.spawn((
            wm,
            AnimState::default(),
            Transform::from_translation(at).with_rotation(Quat::from_rotation_y(*yaw)),
            Visibility::default(),
            Npc { name, title, kind: *kind },
            WantsTool(*tool),
        ));
    }
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

fn open(w: &mut Windows, kind: NpcKind) {
    w.inventory = true;
    match kind {
        NpcKind::Blacksmith => {
            w.upgrade = true;
            w.shop = false;
        }
        NpcKind::Merchant => {
            w.shop = true;
            w.upgrade = false;
        }
    }
}

fn click_npc(
    buttons: Res<ButtonInput<MouseButton>>,
    ui: Res<UiHover>,
    window: Query<&Window, With<PrimaryWindow>>,
    cam: Query<(&Camera, &GlobalTransform), With<OrbitCam>>,
    npcs: Query<(Entity, &Transform, &Npc)>,
    player: Query<&Transform, With<Player>>,
    mut move_to: ResMut<MoveTo>,
    mut target: ResMut<PlayerTarget>,
    mut pending: ResMut<PendingTalk>,
    mut w: ResMut<Windows>,
) {
    if ui.0 || !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let (Ok(window), Ok(cam), Ok(pxf)) = (window.single(), cam.single(), player.single()) else { return };
    let Some(ray) = cursor_ray(window, cam) else { return };
    for (e, xf, npc) in &npcs {
        let c = xf.translation + Vec3::Y * 1.0;
        let oc = ray.origin - c;
        let b = oc.dot(*ray.direction);
        let disc = b * b - (oc.length_squared() - 0.9 * 0.9);
        if disc < 0.0 || -b - disc.sqrt() < 0.0 {
            continue;
        }
        target.0 = None;
        if xf.translation.distance(pxf.translation) <= TALK_RANGE {
            open(&mut w, npc.kind);
            move_to.0 = None;
            pending.0 = None;
        } else {
            let dir = (pxf.translation - xf.translation).normalize_or_zero();
            move_to.0 = Some(xf.translation + dir * 2.0);
            pending.0 = Some(e);
        }
        return;
    }
}

fn arrive(
    mut pending: ResMut<PendingTalk>,
    npcs: Query<(&Transform, &Npc)>,
    player: Query<&Transform, With<Player>>,
    move_to: Res<MoveTo>,
    mut w: ResMut<Windows>,
) {
    let Some(e) = pending.0 else { return };
    let (Ok((xf, npc)), Ok(pxf)) = (npcs.get(e), player.single()) else {
        pending.0 = None;
        return;
    };
    if xf.translation.distance(pxf.translation) <= TALK_RANGE {
        open(&mut w, npc.kind);
        pending.0 = None;
    } else if move_to.0.is_none() {
        // Yürüme iptal edildi.
        pending.0 = None;
    }
}

/// Uzaklaşınca NPC pencerelerini kapat.
pub fn auto_close(
    mut w: ResMut<Windows>,
    npcs: Query<(&Transform, &Npc)>,
    player: Query<&Transform, With<Player>>,
) {
    if !(w.upgrade || w.shop) {
        return;
    }
    let Ok(p) = player.single() else { return };
    let near = |k: NpcKind| npcs.iter().any(|(xf, n)| n.kind == k && xf.translation.distance(p.translation) < 8.0);
    if w.upgrade && !near(NpcKind::Blacksmith) {
        w.upgrade = false;
    }
    if w.shop && !near(NpcKind::Merchant) {
        w.shop = false;
    }
}
