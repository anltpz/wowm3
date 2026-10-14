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
    /// Geri geri yürüyor (WoW: yavaş, geri yürüme animasyonu).
    pub backward: bool,
    /// Zıplamada (dikey hız `vy`).
    pub airborne: bool,
    pub vy: f32,
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
            backward: false,
            airborne: false,
            vy: 0.0,
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
    (keys, buttons, ui): (Res<ButtonInput<KeyCode>>, Res<ButtonInput<MouseButton>>, Res<super::combat::UiHover>),
    mut cam: Query<&mut OrbitCam>,
    mut player: Query<(&mut Transform, &mut Player)>,
    collision: benilla_world::collision::WorldCollision,
    mut focus: ResMut<ViewFocus>,
    mut move_to: ResMut<super::combat::MoveTo>,
    mut autorun: Local<bool>,
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
            xf.translation = wow_to_bevy(super::village_wow());
            p.grounded = false;
            p.airborne = false;
            p.vy = 0.0;
            if let Ok(mut st) = body.single_mut() {
                *st = AnimState::default();
            }
            log.push(time.elapsed_secs(), super::log::C_GOOD, "Yeniden doğdun.");
        }
        return;
    }
    // --- WoW kontrolleri ---
    // W/S ileri-geri, A/D dön (sağ tık basılıyken yan adım), Q/E yan adım, iki fare tuşu = ileri,
    // Num Lock otomatik koşu, Space zıpla. Karakter her zaman kendi yönüne bakar; geri yavaş yürür.
    let (lmb, rmb) = (buttons.pressed(MouseButton::Left) && !ui.0, buttons.pressed(MouseButton::Right) && !ui.0);
    let key = |a: KeyCode, b: KeyCode| keys.pressed(a) || keys.pressed(b);
    if keys.just_pressed(KeyCode::NumLock) {
        *autorun = !*autorun;
    }
    let mut fwd_in = 0.0f32;
    if key(KeyCode::KeyW, KeyCode::ArrowUp) || (lmb && rmb) || *autorun {
        fwd_in += 1.0;
    }
    if key(KeyCode::KeyS, KeyCode::ArrowDown) {
        fwd_in -= 1.0;
        *autorun = false;
    }
    let mut strafe = 0.0f32;
    let mut turn = 0.0f32;
    if keys.pressed(KeyCode::KeyQ) {
        strafe -= 1.0;
    }
    if keys.pressed(KeyCode::KeyE) {
        strafe += 1.0;
    }
    let (left, right) = (key(KeyCode::KeyA, KeyCode::ArrowLeft), key(KeyCode::KeyD, KeyCode::ArrowRight));
    if rmb {
        strafe += right as i32 as f32 - left as i32 as f32;
    } else {
        turn += left as i32 as f32 - right as i32 as f32;
    }
    let mut yaw = yaw_of(xf.rotation);
    if turn != 0.0 {
        yaw += turn * TURN_RATE * dt;
        // Klavyeyle dönünce kamera da döner (sol tıkla serbest bakış hariç).
        if let Ok(mut c) = cam.single_mut() {
            if !lmb {
                c.yaw += turn * TURN_RATE * dt;
            }
        }
    }
    xf.rotation = Quat::from_rotation_y(yaw);
    let (fwd, side) = (xf.rotation * Vec3::NEG_Z, xf.rotation * Vec3::X);
    let mut dir = fwd * fwd_in.clamp(-1.0, 1.0) + side * strafe.clamp(-1.0, 1.0);
    let manual = dir != Vec3::ZERO;
    if manual {
        move_to.0 = None;
    } else if let Some(goal) = move_to.0 {
        // Betikli yürüme (test otopilotu).
        let flat = Vec3::new(goal.x - xf.translation.x, 0.0, goal.z - xf.translation.z);
        if flat.length() < 0.4 {
            move_to.0 = None;
        } else {
            dir = flat.normalize_or_zero();
            xf.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, dir);
        }
    }
    p.backward = manual && fwd_in < 0.0;
    p.moving = dir != Vec3::ZERO;
    if p.moving {
        // Geri yürüme WoW'daki gibi yavaş (4.5 / 7).
        let speed = if p.backward { p.speed * BACK_MUL } else { p.speed };
        let before = xf.translation;
        xf.translation = super::physics::walk(&collision, before, dir.normalize() * speed * dt, 0.35);
        if move_to.0.is_some() && xf.translation.distance(before) < speed * dt * 0.1 {
            move_to.0 = None;
        }
    }
    // Zıplama + yerçekimi.
    if keys.just_pressed(KeyCode::Space) && p.grounded && !p.airborne {
        p.vy = JUMP_SPEED;
        p.airborne = true;
    }
    let ground = ground_at(&collision, xf.translation);
    if p.airborne {
        p.vy -= GRAVITY * dt;
        xf.translation.y += p.vy * dt;
        if let Some(g) = ground {
            if xf.translation.y <= g && p.vy <= 0.0 {
                xf.translation.y = g;
                p.airborne = false;
                p.vy = 0.0;
            }
        }
    } else if let Some(g) = ground {
        if !p.grounded || (g - xf.translation.y).abs() < 3.0 {
            xf.translation.y = g;
            p.grounded = true;
        }
    }
    *focus = ViewFocus::body(bevy_to_wow(xf.translation), p.grounded);
}

/// Klavyeyle dönüş hızı (rad/sn, WoW ≈ 180°/sn).
const TURN_RATE: f32 = std::f32::consts::PI;
/// Geri yürüme hız çarpanı.
const BACK_MUL: f32 = 0.64;
const JUMP_SPEED: f32 = 7.5;
const GRAVITY: f32 = 22.0;

/// Dönüşün Y ekseni açısı (modeller -Z'ye bakar).
pub fn yaw_of(q: Quat) -> f32 {
    let f = q * Vec3::NEG_Z;
    (-f.x).atan2(-f.z)
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
    let idle = if stats.riding {
        anim::MOUNT
    } else if p.airborne {
        anim::JUMP
    } else if p.moving && p.backward {
        anim::WALK_BACK
    } else if p.moving {
        anim::RUN
    } else if look.weapon.is_some() {
        if look.two_hand { anim::READY_2H } else { anim::READY_1H }
    } else {
        anim::STAND
    };
    st.set_loop(idle);
}
