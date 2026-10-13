//! Oyuncu: WASD kameraya göre hareket, zemine oturtma, dünya akışını oyuncuya bağlama, görünüm
//! (gövde + zırh + silah) yeniden kurma.

use bevy::prelude::*;
use benilla_assets::coords::{bevy_to_wow, wow_to_bevy};
use benilla_assets::WorldAssets;
use benilla_world::terrain_stream::ViewFocus;

use super::camera::OrbitCam;
use super::character::{build_body, build_item_model, ArmorSet, CharDb};
use super::model::{anim, AnimState, ModelReady};

#[derive(Component)]
pub struct Player {
    pub speed: f32,
    pub grounded: bool,
    pub moving: bool,
}

/// Oyuncunun giydiği görünüm; `dirty` olunca gövde yeniden kurulur.
#[derive(Resource)]
pub struct PlayerLook {
    pub armor: ArmorSet,
    pub weapon: Option<u32>,
    pub shield: Option<u32>,
    pub two_hand: bool,
    /// Kuşanılan silahın artısı (parıltı için).
    pub weapon_plus: u8,
    pub dirty: bool,
}

impl Default for PlayerLook {
    fn default() -> Self {
        Self {
            // Gümüş Plaka seti (Plate_A_01Silver).
            armor: [None, Some(16184), Some(3418), Some(3414), Some(25828), Some(4448), Some(25831), None],
            weapon: Some(1546),
            shield: None,
            two_hand: false,
            weapon_plus: 0,
            dirty: true,
        }
    }
}

/// Gövde modeli (oyuncunun çocuğu).
#[derive(Component)]
pub struct Body;

/// Gövde hazır olunca takılacak eşyalar.
#[derive(Component)]
struct WantsItems {
    weapon: Option<u32>,
    shield: Option<u32>,
}

/// Elde tutulan eşya modeli.
#[derive(Component)]
pub struct HeldItem;

pub fn plugin(app: &mut App) {
    app.init_resource::<PlayerLook>()
        .add_systems(Startup, spawn_player.after(benilla_assets::AssetSet::Open))
        .add_systems(
            Update,
            move_player.in_set(benilla_world::schedule::WorldStage::Input),
        )
        .add_systems(Update, (rebuild_look, attach_items, animate_player));
}

fn spawn_player(mut commands: Commands) {
    let start = wow_to_bevy(super::start_wow());
    commands.spawn((
        Player {
            speed: 7.0,
            grounded: false,
            moving: false,
        },
        Transform::from_translation(start),
        Visibility::default(),
    ));
    commands.insert_resource(ViewFocus::body(super::start_wow(), false));
    commands.spawn((
        DirectionalLight {
            illuminance: 4500.0,
            shadows_enabled: false,
            color: Color::srgb(1.0, 0.95, 0.85),
            ..default()
        },
        Transform::from_xyz(1.0, 1.4, 0.6).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // Birimlerimiz (StandardMaterial) benilla'nın ışık tamponunu görmez: yumuşak ortam ışığı.
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.85, 0.9, 1.0),
        brightness: 1400.0,
        ..default()
    });
}

fn rebuild_look(
    mut commands: Commands,
    mut look: ResMut<PlayerLook>,
    db: Option<Res<CharDb>>,
    mut wa: Option<ResMut<WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    server: Res<AssetServer>,
    player: Query<Entity, With<Player>>,
    bodies: Query<(Entity, Option<&AnimState>), With<Body>>,
) {
    if !look.dirty {
        return;
    }
    let (Some(db), Some(wa), Ok(p)) = (db, wa.as_deref_mut(), player.single()) else {
        return;
    };
    let mut anim_state = AnimState::default();
    for (e, st) in &bodies {
        if let Some(st) = st {
            anim_state.idle = st.idle;
            anim_state.want = st.idle;
        }
        commands.entity(e).despawn();
    }
    let wm = build_body(&db, wa, &mut images, &server, &look.armor);
    commands.spawn((
        wm,
        anim_state,
        Body,
        WantsItems {
            weapon: look.weapon,
            shield: look.shield,
        },
        Transform::default(),
        Visibility::default(),
        ChildOf(p),
    ));
    look.dirty = false;
}

fn attach_items(
    mut commands: Commands,
    db: Option<Res<CharDb>>,
    mut wa: Option<ResMut<WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    server: Res<AssetServer>,
    q: Query<(Entity, &ModelReady, &WantsItems), With<Body>>,
) {
    let (Some(db), Some(wa)) = (db, wa.as_deref_mut()) else {
        return;
    };
    for (e, ready, want) in &q {
        // 1 = sağ el, 2 = sol el, 0 = kalkan (kol).
        let slots = [(want.weapon, 1u16, "Weapon"), (want.shield, 0u16, "Shield")];
        for (display, attach, dir) in slots {
            let Some(display) = display else { continue };
            let Some((joint, offset)) = ready.attachment(attach) else {
                warn!("player: takma noktası {attach} yok");
                continue;
            };
            if let Some(wm) = build_item_model(&db, wa, &mut images, &server, display, dir) {
                commands.spawn((
                    wm,
                    HeldItem,
                    Transform::from_translation(offset),
                    Visibility::default(),
                    ChildOf(joint),
                ));
            }
        }
        commands.entity(e).remove::<WantsItems>();
    }
}

pub fn ground_at(collision: &benilla_world::collision::WorldCollision, at: Vec3) -> Option<f32> {
    super::physics::ground_at(collision, at)
}

pub fn move_player(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    cam: Query<&OrbitCam>,
    mut player: Query<(&mut Transform, &mut Player)>,
    collision: benilla_world::collision::WorldCollision,
    mut focus: ResMut<ViewFocus>,
    mut move_to: ResMut<super::combat::MoveTo>,
    target: Res<super::combat::PlayerTarget>,
    enemies: Query<(&Transform, &super::combat::Enemy), Without<Player>>,
    derived: Res<super::stats::Derived>,
    mut stats: ResMut<super::stats::PlayerStats>,
    mut body: Query<&mut AnimState, With<Body>>,
    mut log: ResMut<super::log::GameLog>,
) {
    let Ok((mut xf, mut p)) = player.single_mut() else {
        return;
    };
    p.speed = derived.move_speed;
    let dt = time.delta_secs();
    if stats.dead {
        p.moving = false;
        // R: köyde (başlangıç) yeniden doğ.
        if keys.just_pressed(KeyCode::KeyR) {
            stats.dead = false;
            stats.hp = derived.max_hp / 2;
            stats.last_hurt = time.elapsed_secs();
            xf.translation = wow_to_bevy(super::start_wow());
            p.grounded = false;
            if let Ok(mut st) = body.single_mut() {
                *st = AnimState::default();
            }
            log.push(time.elapsed_secs(), super::log::C_GOOD, "Yeniden doğdun.");
        }
        return;
    }
    let yaw = cam.single().map(|c| c.yaw).unwrap_or(0.0);
    let fwd = Quat::from_rotation_y(yaw) * Vec3::NEG_Z;
    let right = Quat::from_rotation_y(yaw) * Vec3::X;
    let mut dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        dir += fwd;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        dir -= fwd;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        dir -= right;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        dir += right;
    }
    if dir != Vec3::ZERO {
        // Klavye tıkla-yürüyü iptal eder.
        move_to.0 = None;
    } else if let Some(t) = target.0.and_then(|t| enemies.get(t).ok()) {
        // Hedefe yaklaş (menzile girince saldırı sistemi devralır).
        let (txf, en) = t;
        let to = txf.translation - xf.translation;
        let flat = Vec3::new(to.x, 0.0, to.z);
        if flat.length() > super::combat::reach(en, txf.scale.x) {
            dir = flat.normalize_or_zero();
        }
    } else if let Some(goal) = move_to.0 {
        let to = goal - xf.translation;
        let flat = Vec3::new(to.x, 0.0, to.z);
        if flat.length() < 0.4 {
            move_to.0 = None;
        } else {
            dir = flat.normalize_or_zero();
        }
    }
    // Saldırı animasyonu sırasında yerinde dur (Metin2 kombo hissi).
    let attacking = body.single().map(|b| b.busy()).unwrap_or(false);
    p.moving = dir != Vec3::ZERO && !attacking;
    if p.moving {
        let dir = dir.normalize();
        let before = xf.translation;
        xf.translation = super::physics::walk(&collision, before, dir * p.speed * dt, 0.35);
        let target_rot = Quat::from_rotation_arc(Vec3::NEG_Z, dir);
        xf.rotation = xf.rotation.slerp(target_rot, (dt * 14.0).min(1.0));
        // Takıldıysa tıkla-yürüyü bırak.
        if move_to.0.is_some() && xf.translation.distance(before) < p.speed * dt * 0.1 {
            move_to.0 = None;
        }
    }
    if let Some(g) = ground_at(&collision, xf.translation) {
        if !p.grounded || (g - xf.translation.y).abs() < 3.0 {
            xf.translation.y = g;
            p.grounded = true;
        }
    }
    *focus = ViewFocus::body(bevy_to_wow(xf.translation), p.grounded);
}

fn animate_player(
    player: Query<&Player>,
    look: Res<PlayerLook>,
    stats: Res<super::stats::PlayerStats>,
    mut body: Query<&mut AnimState, With<Body>>,
) {
    let (Ok(p), Ok(mut st)) = (player.single(), body.single_mut()) else {
        return;
    };
    if stats.dead {
        return;
    }
    let idle = if p.moving {
        anim::RUN
    } else if look.weapon.is_some() {
        if look.two_hand { anim::READY_2H } else { anim::READY_1H }
    } else {
        anim::STAND
    };
    st.set_loop(idle);
}
