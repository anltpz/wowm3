//! Savaşçı becerileri (Metin2 esinli), 1-4 tuşları:
//! 1 Üç Yönlü Kesme (önde geniş ağır vuruş), 2 Kılıç Çevirme (360° alan), 3 Öfke (20 sn güç +
//! saldırı hızı), 4 Hamle (hedefe atılıp vurma). SP harcar, bekleme süresi vardır.

use bevy::prelude::*;

use super::combat::{roll_damage, Dying, Enemy, FloatText, Hit, PlayerTarget, SkillLock};
use super::log::{GameLog, C_BAD};
use super::model::{anim, AnimState};
use super::player::{Body, Player};
use super::stats::{Derived, PlayerStats};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillId {
    Cleave,
    Whirl,
    Rage,
    Charge,
}

#[derive(Clone, Copy, Debug)]
pub struct SkillDef {
    pub id: SkillId,
    pub name: &'static str,
    pub icon: &'static str,
    pub desc: &'static str,
    pub sp: i32,
    pub cooldown: f32,
    pub level: u32,
}

pub const SKILLS: [SkillDef; 4] = [
    SkillDef {
        id: SkillId::Cleave,
        name: "Üç Yönlü Kesme",
        icon: "Ability_Warrior_Cleave",
        desc: "Önündeki tüm düşmanlara %220 hasar.",
        sp: 18,
        cooldown: 4.0,
        level: 1,
    },
    SkillDef {
        id: SkillId::Whirl,
        name: "Kılıç Çevirme",
        icon: "Ability_Whirlwind",
        desc: "Etrafındaki herkese iki kez %140 hasar.",
        sp: 30,
        cooldown: 8.0,
        level: 2,
    },
    SkillDef {
        id: SkillId::Rage,
        name: "Öfke",
        icon: "Ability_Racial_BloodRage",
        desc: "20 sn boyunca +%25 saldırı, +%30 saldırı hızı.",
        sp: 40,
        cooldown: 45.0,
        level: 4,
    },
    SkillDef {
        id: SkillId::Charge,
        name: "Hamle",
        icon: "Ability_Warrior_Charge",
        desc: "Hedefe atılır ve %180 hasar verir. Menzil 14 m.",
        sp: 22,
        cooldown: 10.0,
        level: 6,
    },
];

pub const SKILL_KEYS: [KeyCode; 4] = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4];

/// Bekleme süreleri: her becerinin hazır olacağı zaman.
#[derive(Resource, Default)]
pub struct SkillCooldowns(pub [f32; 4]);

impl SkillCooldowns {
    /// Kalan oran 0..1 (UI kararması için).
    pub fn frac(&self, i: usize, now: f32) -> f32 {
        ((self.0[i] - now) / SKILLS[i].cooldown).clamp(0.0, 1.0)
    }
}

/// Hızlı slottan/tuştan beceri isteği.
#[derive(Message, Clone, Copy)]
pub struct CastSkill(pub usize);

/// Süren beceri.
#[derive(Resource, Default)]
struct Active {
    skill: Option<SkillId>,
    t: f32,
    /// Vuruş zamanları (beceri başlangıcına göre) ve atılmış olanlar.
    hits_done: u8,
    charge_from: Vec3,
    charge_to: Vec3,
    charge_target: Option<Entity>,
}

/// Geçici görsel efekt: genişleyip solan halka.
#[derive(Component)]
struct Ring {
    t: f32,
    life: f32,
    from: f32,
    to: f32,
    mat: Handle<StandardMaterial>,
    color: LinearRgba,
}

#[derive(Component)]
struct RageAura;

#[derive(Resource)]
struct FxAssets {
    ring: Handle<Mesh>,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<SkillCooldowns>()
        .init_resource::<Active>()
        .add_message::<CastSkill>()
        .add_systems(Startup, setup)
        .add_systems(Update, (keys, cast, run_active, rings, rage_aura).chain());
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(FxAssets {
        ring: meshes.add(Annulus::new(0.85, 1.0)),
    });
}

fn keys(input: Res<ButtonInput<KeyCode>>, mut ev: MessageWriter<CastSkill>) {
    for (i, k) in SKILL_KEYS.iter().enumerate() {
        if input.just_pressed(*k) {
            ev.write(CastSkill(i));
        }
    }
}

fn spawn_ring(
    commands: &mut Commands,
    fx: &FxAssets,
    mats: &mut Assets<StandardMaterial>,
    at: Vec3,
    from: f32,
    to: f32,
    life: f32,
    color: LinearRgba,
) {
    let mat = mats.add(StandardMaterial {
        base_color: Color::BLACK,
        emissive: color,
        unlit: true,
        alpha_mode: AlphaMode::Add,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        Mesh3d(fx.ring.clone()),
        MeshMaterial3d(mat.clone()),
        Transform::from_translation(at + Vec3::Y * 0.15)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
            .with_scale(Vec3::splat(from)),
        Ring {
            t: 0.0,
            life,
            from,
            to,
            mat,
            color,
        },
    ));
}

fn cast(
    time: Res<Time>,
    mut ev: MessageReader<CastSkill>,
    mut stats: ResMut<PlayerStats>,
    mut cds: ResMut<SkillCooldowns>,
    mut active: ResMut<Active>,
    mut log: ResMut<GameLog>,
    target: Res<PlayerTarget>,
    player: Query<&Transform, With<Player>>,
    enemies: Query<&Transform, (With<Enemy>, Without<Dying>, Without<Player>)>,
    mut body: Query<&mut AnimState, With<Body>>,
    mut lock: ResMut<SkillLock>,
    mut commands: Commands,
    fx: Option<Res<FxAssets>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let now = time.elapsed_secs();
    for CastSkill(i) in ev.read() {
        let def = &SKILLS[*i];
        if stats.dead || stats.riding || active.skill.is_some() {
            continue;
        }
        if stats.level < def.level {
            log.push(now, C_BAD, format!("{} için seviye {} gerekli.", def.name, def.level));
            continue;
        }
        if cds.0[*i] > now {
            log.push(now, C_BAD, format!("{} henüz hazır değil.", def.name));
            continue;
        }
        if stats.sp < def.sp {
            log.push(now, C_BAD, "Yeterli SP yok.");
            continue;
        }
        let Ok(pxf) = player.single() else { continue };
        let mut charge = None;
        if def.id == SkillId::Charge {
            let Some((t, txf)) = target.0.and_then(|t| enemies.get(t).ok().map(|x| (t, x))) else {
                log.push(now, C_BAD, "Hamle için bir hedef seç.");
                continue;
            };
            let d = txf.translation.distance(pxf.translation);
            if d > 14.0 {
                log.push(now, C_BAD, "Hedef çok uzak.");
                continue;
            }
            charge = Some((t, txf.translation));
        }
        stats.sp -= def.sp;
        cds.0[*i] = now + def.cooldown;
        *active = Active {
            skill: Some(def.id),
            t: 0.0,
            hits_done: 0,
            charge_from: pxf.translation,
            charge_to: charge.map(|c| c.1).unwrap_or(pxf.translation),
            charge_target: charge.map(|c| c.0),
        };
        lock.0 = true;
        if let Ok(mut st) = body.single_mut() {
            match def.id {
                SkillId::Cleave => {
                    st.speed = 1.6;
                    st.play_once(anim::ATTACK_2H);
                }
                SkillId::Whirl => {
                    st.speed = 1.4;
                    st.play_once(anim::ATTACK_1H);
                }
                SkillId::Rage => {
                    st.speed = 1.0;
                    st.play_once(anim::BATTLE_ROAR);
                }
                SkillId::Charge => {
                    st.speed = 1.0;
                    st.play_once(anim::RUN);
                }
            }
        }
        if let (SkillId::Rage, Some(fx)) = (def.id, fx.as_deref()) {
            stats.rage_until = now + 20.0;
            spawn_ring(&mut commands, fx, &mut mats, pxf.translation, 0.5, 4.0, 0.7, LinearRgba::rgb(3.0, 0.3, 0.1));
            commands.spawn((
                PointLight {
                    color: Color::srgb(1.0, 0.2, 0.1),
                    intensity: 40_000.0,
                    range: 6.0,
                    ..default()
                },
                Transform::from_translation(pxf.translation + Vec3::Y * 1.5),
                RageAura,
            ));
        }
    }
}

/// Becerinin isabet etkisi: alan içindeki düşmanlara Hit.
fn strike(
    origin: Vec3,
    fwd: Vec3,
    radius: f32,
    min_dot: f32,
    mult: f32,
    derived: &Derived,
    enemies: &Query<(Entity, &Transform, &Enemy), (Without<Dying>, Without<Player>, Without<Body>)>,
    hits: &mut MessageWriter<Hit>,
    commands: &mut Commands,
) {
    for (e, xf, en) in enemies.iter() {
        let to = xf.translation - origin;
        let flat = Vec3::new(to.x, 0.0, to.z);
        if flat.length() > radius + en.radius * xf.scale.x {
            continue;
        }
        if flat.normalize_or_zero().dot(fwd) < min_dot && flat.length() > 1.0 {
            continue;
        }
        let (dmg, crit, miss) = roll_damage(derived, en, mult);
        if miss {
            commands.spawn(FloatText {
                world: xf.translation + Vec3::Y * (en.height * xf.scale.y + 0.3),
                t: 0.0,
                text: "Miss".into(),
                color: Color::srgb(0.9, 0.9, 0.95),
                big: false,
            });
            continue;
        }
        hits.write(Hit { target: e, amount: dmg, crit });
    }
}

fn run_active(
    time: Res<Time>,
    mut active: ResMut<Active>,
    mut lock: ResMut<SkillLock>,
    derived: Res<Derived>,
    mut player: Query<&mut Transform, (With<Player>, Without<Body>)>,
    mut body: Query<(&mut Transform, &mut AnimState), (With<Body>, Without<Player>)>,
    enemies: Query<(Entity, &Transform, &Enemy), (Without<Dying>, Without<Player>, Without<Body>)>,
    mut hits: MessageWriter<Hit>,
    mut commands: Commands,
    fx: Option<Res<FxAssets>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    col: benilla_world::collision::WorldCollision,
    stats: Res<PlayerStats>,
) {
    let Some(skill) = active.skill else { return };
    let (Ok(mut pxf), Some(fx)) = (player.single_mut(), fx) else { return };
    if stats.dead {
        active.skill = None;
        lock.0 = false;
        return;
    }
    let dt = time.delta_secs();
    active.t += dt;
    let t = active.t;
    let fwd = pxf.rotation * Vec3::NEG_Z;
    let pos = pxf.translation;
    let mut finished = false;
    match skill {
        SkillId::Cleave => {
            if t > 0.25 && active.hits_done == 0 {
                active.hits_done = 1;
                strike(pos, fwd, 3.6, 0.0, 2.2, &derived, &enemies, &mut hits, &mut commands);
                spawn_ring(&mut commands, &fx, &mut mats, pos + fwd * 1.5, 0.6, 3.2, 0.35, LinearRgba::rgb(2.5, 2.0, 1.2));
            }
            finished = t > 0.6;
        }
        SkillId::Whirl => {
            // Gövdeyi döndür.
            if let Ok((mut bxf, _)) = body.single_mut() {
                bxf.rotation = Quat::from_rotation_y(-t * 18.0);
            }
            for (k, at) in [0.25f32, 0.6].iter().enumerate() {
                if t > *at && active.hits_done as usize == k {
                    active.hits_done += 1;
                    strike(pos, fwd, 4.5, -2.0, 1.4, &derived, &enemies, &mut hits, &mut commands);
                    spawn_ring(&mut commands, &fx, &mut mats, pos, 1.0, 5.0, 0.4, LinearRgba::rgb(0.6, 1.4, 3.0));
                }
            }
            finished = t > 0.85;
            if finished {
                if let Ok((mut bxf, _)) = body.single_mut() {
                    bxf.rotation = Quat::IDENTITY;
                }
            }
        }
        SkillId::Rage => finished = t > 0.9,
        SkillId::Charge => {
            let to = active
                .charge_target
                .and_then(|e| enemies.get(e).ok())
                .map(|(_, x, en)| (x.translation, en.radius * x.scale.x))
                .unwrap_or((active.charge_to, 0.5));
            let flat = Vec3::new(to.0.x - pos.x, 0.0, to.0.z - pos.z);
            let dist = flat.length();
            if dist > to.1 + 1.2 && t < 0.6 {
                let step = flat.normalize() * (32.0 * dt).min(dist - to.1 - 1.0);
                pxf.translation = super::physics::walk(&col, pos, step, 0.35);
                pxf.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, flat.normalize());
                if active.hits_done == 0 && (t * 20.0) as u32 % 2 == 0 {
                    spawn_ring(&mut commands, &fx, &mut mats, pos, 0.3, 1.2, 0.3, LinearRgba::rgb(2.0, 1.5, 0.6));
                }
            } else if active.hits_done == 0 {
                active.hits_done = 1;
                let f = flat.normalize_or_zero();
                strike(pxf.translation, f, 2.8, 0.2, 1.8, &derived, &enemies, &mut hits, &mut commands);
                spawn_ring(&mut commands, &fx, &mut mats, pxf.translation + f, 0.4, 2.5, 0.35, LinearRgba::rgb(3.0, 1.8, 0.5));
                if let Ok((_, mut st)) = body.single_mut() {
                    st.speed = 1.5;
                    st.play_once(anim::ATTACK_1H);
                }
                active.t = 0.6;
            }
            finished = active.hits_done == 1 && active.t > 0.95;
        }
    }
    if finished {
        active.skill = None;
        lock.0 = false;
    }
}

fn rings(mut commands: Commands, time: Res<Time>, mut mats: ResMut<Assets<StandardMaterial>>, mut q: Query<(Entity, &mut Ring, &mut Transform)>) {
    for (e, mut r, mut xf) in &mut q {
        r.t += time.delta_secs();
        let k = (r.t / r.life).min(1.0);
        xf.scale = Vec3::splat(r.from + (r.to - r.from) * k);
        if let Some(m) = mats.get_mut(&r.mat) {
            m.emissive = r.color * (1.0 - k);
        }
        if k >= 1.0 {
            commands.entity(e).despawn();
        }
    }
}

fn rage_aura(
    mut commands: Commands,
    time: Res<Time>,
    stats: Res<PlayerStats>,
    player: Query<&Transform, (With<Player>, Without<RageAura>)>,
    mut q: Query<(Entity, &mut Transform, &mut PointLight), With<RageAura>>,
) {
    let now = time.elapsed_secs();
    let Ok(p) = player.single() else { return };
    for (e, mut xf, mut l) in &mut q {
        if now > stats.rage_until {
            commands.entity(e).despawn();
            continue;
        }
        xf.translation = p.translation + Vec3::Y * 1.5;
        l.intensity = 30_000.0 + (now * 8.0).sin() * 12_000.0;
    }
}
