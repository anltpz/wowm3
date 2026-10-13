//! Savaş çekirdeği: düşman/HP bileşenleri, hedefleme (sol tık), tıkla-yürü, saldırı (Space /
//! hedefe otomatik), hasar hesabı, ölüm ve EXP.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use rand::Rng;

use super::camera::OrbitCam;
use super::log::{GameLog, C_EXP, C_GOOD};
use super::model::{anim, AnimState};
use super::player::{Body, Player};
use super::stats::{gain_exp, Derived, PlayerStats};

/// Düşman olarak hedeflenebilen her şey (mob, metin taşı).
#[derive(Component, Clone)]
pub struct Enemy {
    pub name: String,
    pub level: u32,
    pub radius: f32,
    pub height: f32,
    pub exp: u32,
    pub def: i32,
    /// Metin taşı mı (isim rengi, saldırmaz).
    pub is_metin: bool,
}

#[derive(Component, Clone, Copy)]
pub struct Health {
    pub hp: i32,
    pub max: i32,
}

/// Ölüm animasyonu/çürüme sayacı.
#[derive(Component)]
pub struct Dying {
    pub t: f32,
}

/// Bir düşmana isabet.
#[derive(Message, Clone, Copy)]
pub struct Hit {
    pub target: Entity,
    pub amount: i32,
    pub crit: bool,
}

/// Düşman öldü (ganimet/yeniden doğma sistemleri dinler).
#[derive(Message, Clone, Copy)]
pub struct Killed {
    pub entity: Entity,
    pub pos: Vec3,
    pub level: u32,
}

/// Oyuncu hasar aldı.
#[derive(Message, Clone, Copy)]
pub struct PlayerHurt {
    pub amount: i32,
    pub from: Entity,
}

/// Ekranda yükselen hasar sayısı (UI çizer).
#[derive(Component)]
pub struct FloatText {
    pub world: Vec3,
    pub t: f32,
    pub text: String,
    pub color: Color,
    pub big: bool,
}

#[derive(Resource, Default)]
pub struct PlayerTarget(pub Option<Entity>);

/// Tıkla-yürü hedefi.
#[derive(Resource, Default)]
pub struct MoveTo(pub Option<Vec3>);

/// Bir beceri oynarken normal saldırıyı durdurur (skills yazar).
#[derive(Resource, Default)]
pub struct SkillLock(pub bool);

/// İmleç bir UI penceresinin üstünde mi (UI modülü yazar).
#[derive(Resource, Default)]
pub struct UiHover(pub bool);

/// Tıklama işleme kümesi (NPC tıklaması bundan sonra çalışır).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CombatInput;

#[derive(Resource, Default)]
struct Swing {
    cd: f32,
    hit_at: Option<f32>,
    combo: u8,
}

pub fn plugin(app: &mut App) {
    app.add_message::<Hit>()
        .add_message::<Killed>()
        .add_message::<PlayerHurt>()
        .init_resource::<PlayerTarget>()
        .init_resource::<MoveTo>()
        .init_resource::<UiHover>()
        .init_resource::<SkillLock>()
        .init_resource::<Swing>()
        .add_systems(
            Update,
            (
                click_select.in_set(CombatInput),
                attack_input,
                apply_hits,
                apply_player_hurt,
                dying,
                float_texts,
            )
                .chain(),
        );
}

/// İmleç ışını.
pub fn cursor_ray(
    window: &Window,
    cam: (&Camera, &GlobalTransform),
) -> Option<Ray3d> {
    let pos = window.cursor_position()?;
    cam.0.viewport_to_world(cam.1, pos).ok()
}

/// Işın-küre kesişimi (t, yoksa None).
fn ray_sphere(ray: &Ray3d, c: Vec3, r: f32) -> Option<f32> {
    let oc = ray.origin - c;
    let d = *ray.direction;
    let b = oc.dot(d);
    let cc = oc.length_squared() - r * r;
    let disc = b * b - cc;
    if disc < 0.0 {
        return None;
    }
    let t = -b - disc.sqrt();
    (t > 0.0).then_some(t)
}

/// İmlecin altındaki düşman.
pub fn pick_enemy(
    ray: &Ray3d,
    enemies: &Query<(Entity, &Transform, &Enemy), Without<Dying>>,
) -> Option<Entity> {
    let mut best: Option<(f32, Entity)> = None;
    for (e, xf, en) in enemies.iter() {
        let s = xf.scale.x;
        let c = xf.translation + Vec3::Y * en.height * 0.5 * s;
        let r = (en.radius * s).max(0.7) + en.height * 0.25 * s;
        if let Some(t) = ray_sphere(ray, c, r) {
            if best.is_none_or(|(bt, _)| t < bt) {
                best = Some((t, e));
            }
        }
    }
    best.map(|(_, e)| e)
}

fn click_select(
    buttons: Res<ButtonInput<MouseButton>>,
    ui: Res<UiHover>,
    window: Query<&Window, With<PrimaryWindow>>,
    cam: Query<(&Camera, &GlobalTransform), With<OrbitCam>>,
    enemies: Query<(Entity, &Transform, &Enemy), Without<Dying>>,
    collision: benilla_world::collision::WorldCollision,
    mut target: ResMut<PlayerTarget>,
    mut move_to: ResMut<MoveTo>,
    stats: Res<PlayerStats>,
) {
    if stats.dead || ui.0 || !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let (Ok(window), Ok(cam)) = (window.single(), cam.single()) else {
        return;
    };
    let Some(ray) = cursor_ray(window, cam) else {
        return;
    };
    if let Some(e) = pick_enemy(&ray, &enemies) {
        target.0 = Some(e);
        move_to.0 = None;
        return;
    }
    // Zemine tıklandı: oraya yürü.
    if let Some(hit) = collision.ray_body(ray.origin, ray.direction, 250.0) {
        move_to.0 = Some(ray.origin + *ray.direction * hit.distance);
        target.0 = None;
    }
}

/// Hasar zarı: (hasar, kritik, ıska). `mult` beceri çarpanı.
pub fn roll_damage(d: &Derived, en: &Enemy, mult: f32) -> (i32, bool, bool) {
    let mut rng = rand::rng();
    if !en.is_metin && rng.random_bool(0.06) {
        return (0, false, true);
    }
    let raw = rng.random_range(d.atk_min..=d.atk_max.max(d.atk_min)) as f32 * mult;
    let crit = rng.random_bool(0.12);
    let mut dmg = ((raw as i32) - en.def / 2).max(1);
    if crit {
        dmg *= 2;
    }
    (dmg, crit, false)
}

/// Oyuncunun saldırı menzili (silah + hedef yarıçapı).
pub fn reach(en: &Enemy, scale: f32) -> f32 {
    2.2 + en.radius * scale
}

fn attack_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    derived: Res<Derived>,
    stats: Res<PlayerStats>,
    mut swing: ResMut<Swing>,
    mut target: ResMut<PlayerTarget>,
    mut player: Query<&mut Transform, With<Player>>,
    mut body: Query<&mut AnimState, With<Body>>,
    enemies: Query<(Entity, &Transform, &Enemy, &Health), (Without<Player>, Without<Dying>)>,
    mut hits: MessageWriter<Hit>,
    mut commands: Commands,
    mut pending_miss: Local<Vec<Vec3>>,
    skill_lock: Res<SkillLock>,
) {
    let dt = time.delta_secs();
    swing.cd -= dt;
    let Ok(mut pxf) = player.single_mut() else {
        return;
    };
    if stats.dead {
        swing.hit_at = None;
        return;
    }
    if let Some(t) = target.0 {
        if enemies.get(t).is_err() {
            target.0 = None;
        }
    }
    let ppos = pxf.translation;
    // Hedefteyken menzilde olunca otomatik saldırı; Space her zaman saldırır.
    let in_range_target = target.0.and_then(|t| enemies.get(t).ok()).filter(|(_, xf, en, _)| {
        xf.translation.distance(ppos) <= reach(en, xf.scale.x) + 0.3
    });
    let want = keys.pressed(KeyCode::Space) || in_range_target.is_some();

    // Vuruş anı.
    if let Some(at) = swing.hit_at {
        if time.elapsed_secs() >= at {
            swing.hit_at = None;
            let fwd = pxf.rotation * Vec3::NEG_Z;
            let mut misses = Vec::new();
            for (e, xf, en, _) in &enemies {
                let to = xf.translation - ppos;
                let dist = Vec3::new(to.x, 0.0, to.z).length();
                if dist > reach(en, xf.scale.x) + 0.4 {
                    continue;
                }
                let dir = Vec3::new(to.x, 0.0, to.z).normalize_or_zero();
                // Ön yarım daire (Metin2 savaşçısı geniş vurur) ya da seçili hedef.
                if fwd.dot(dir) < 0.2 && Some(e) != target.0 && dist > en.radius + 0.6 {
                    continue;
                }
                let (dmg, crit, miss) = roll_damage(&derived, en, 1.0);
                if miss {
                    misses.push(xf.translation + Vec3::Y * (en.height * xf.scale.y + 0.3));
                    continue;
                }
                hits.write(Hit {
                    target: e,
                    amount: dmg,
                    crit,
                });
            }
            pending_miss.extend(misses);
        }
    }

    for p in std::mem::take(&mut *pending_miss) {
        commands.spawn(FloatText { world: p, t: 0.0, text: "Miss".into(), color: Color::srgb(0.9, 0.9, 0.95), big: false });
    }
    if want && swing.cd <= 0.0 && !skill_lock.0 {
        // En yakın düşmana dön (otomatik nişan).
        let face = in_range_target.map(|(_, xf, _, _)| xf.translation).or_else(|| {
            enemies
                .iter()
                .map(|(_, xf, en, _)| (xf.translation, xf.translation.distance(ppos) - en.radius))
                .filter(|(_, d)| *d < 4.0)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(p, _)| p)
        });
        if let Some(p) = face {
            let d = Vec3::new(p.x - ppos.x, 0.0, p.z - ppos.z);
            if d.length_squared() > 0.01 {
                pxf.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, d.normalize());
            }
        }
        swing.cd = derived.swing;
        swing.combo = swing.combo.wrapping_add(1);
        swing.hit_at = Some(time.elapsed_secs() + derived.swing * 0.38);
        if let Ok(mut st) = body.single_mut() {
            let a = if derived.two_hand {
                anim::ATTACK_2H
            } else {
                anim::ATTACK_1H
            };
            st.speed = 1.15;
            st.play_once(a);
        }
    }
}

fn apply_hits(
    mut commands: Commands,
    time: Res<Time>,
    mut hits: MessageReader<Hit>,
    mut q: Query<(&mut Health, &Transform, &Enemy, Option<&mut AnimState>), Without<Dying>>,
    mut killed: MessageWriter<Killed>,
    mut stats: ResMut<PlayerStats>,
    mut log: ResMut<GameLog>,
    mut target: ResMut<PlayerTarget>,
) {
    let now = time.elapsed_secs();
    for h in hits.read() {
        let Ok((mut hp, xf, en, st)) = q.get_mut(h.target) else {
            continue;
        };
        hp.hp -= h.amount;
        commands.spawn(FloatText {
            world: xf.translation + Vec3::Y * (en.height * xf.scale.y + 0.3),
            t: 0.0,
            text: if h.crit {
                format!("CRITICAL!
{}", h.amount)
            } else {
                h.amount.to_string()
            },
            color: if h.crit {
                Color::srgb(1.0, 0.55, 0.1)
            } else {
                Color::srgb(1.0, 1.0, 0.9)
            },
            big: h.crit,
        });
        if hp.hp <= 0 {
            hp.hp = 0;
            commands.entity(h.target).insert(Dying { t: 0.0 });
            if let Some(mut st) = st {
                st.speed = 1.0;
                st.hold(anim::DEATH);
            }
            killed.write(Killed {
                entity: h.target,
                pos: xf.translation,
                level: en.level,
            });
            if target.0 == Some(h.target) {
                target.0 = None;
            }
            // EXP: seviye farkı cezası (Metin2 benzeri).
            let diff = en.level as i32 - stats.level as i32;
            let mult = (1.0 + diff as f32 * 0.1).clamp(0.1, 1.5);
            let exp = (en.exp as f32 * mult).round() as u64;
            let ups = gain_exp(&mut stats, exp);
            log.push(now, C_EXP, format!("{} öldü. +{} EXP", en.name, exp));
            if ups > 0 {
                let lvl = stats.level;
                log.banner(now, C_GOOD, format!("Seviye atladın! Seviye {lvl}"));
                log.push(now, C_GOOD, format!("Tebrikler! Seviye {lvl} oldun."));
                stats.hp = i32::MAX / 4; // update_derived sonrası max'a kırpılır
            }
        }
    }
}

fn apply_player_hurt(
    time: Res<Time>,
    mut hurt: MessageReader<PlayerHurt>,
    mut stats: ResMut<PlayerStats>,
    derived: Res<Derived>,
    mut commands: Commands,
    player: Query<&Transform, With<Player>>,
    mut body: Query<&mut AnimState, With<Body>>,
    mut log: ResMut<GameLog>,
    mut target: ResMut<PlayerTarget>,
) {
    let now = time.elapsed_secs();
    for h in hurt.read() {
        if stats.dead {
            continue;
        }
        let dmg = (h.amount - derived.def * 2 / 5).max(1);
        stats.hp -= dmg;
        stats.last_hurt = now;
        // Saldırana otomatik karşılık (hedef yoksa).
        if target.0.is_none() {
            target.0 = Some(h.from);
        }
        if let Ok(xf) = player.single() {
            commands.spawn(FloatText {
                world: xf.translation + Vec3::Y * 2.3,
                t: 0.0,
                text: format!("-{dmg}"),
                color: Color::srgb(1.0, 0.25, 0.2),
                big: false,
            });
        }
        if stats.hp <= 0 {
            stats.hp = 0;
            stats.dead = true;
            if let Ok(mut st) = body.single_mut() {
                st.speed = 1.0;
                st.hold(anim::DEATH);
            }
            log.banner(now, super::log::C_BAD, "Öldün!");
            log.push(now, super::log::C_BAD, "Öldün. Yeniden doğmak için R tuşuna bas.");
        }
    }
}

/// Ölen düşmanlar: animasyon sonrası yere gömülüp kaybolur.
fn dying(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Dying, &mut Transform, &Enemy)>,
) {
    for (e, mut d, mut xf, en) in &mut q {
        d.t += time.delta_secs();
        if en.is_metin {
            // Taş çöker.
            let k = (1.0 - d.t / 1.2).max(0.0);
            xf.scale.y = xf.scale.x * k.max(0.05);
            if d.t > 1.3 {
                commands.entity(e).despawn();
            }
        } else if d.t > 4.0 {
            xf.translation.y -= time.delta_secs() * 0.6;
            if d.t > 6.5 {
                commands.entity(e).despawn();
            }
        }
    }
}

fn float_texts(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut FloatText)>) {
    for (e, mut f) in &mut q {
        f.t += time.delta_secs();
        f.world.y += time.delta_secs() * 1.2;
        if f.t > 1.2 {
            commands.entity(e).despawn();
        }
    }
}
