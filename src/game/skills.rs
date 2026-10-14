//! Savaşçı becerileri (Metin2 esinli), 1-4 tuşları:
//! 1 Üç Yönlü Kesme (önde geniş ağır vuruş), 2 Kılıç Çevirme (360° alan), 3 Öfke (20 sn güç +
//! saldırı hızı), 4 Hamle (hedefe atılıp vurma). SP harcar, bekleme süresi vardır.

use bevy::prelude::*;

use super::combat::{roll_damage, Dying, Enemy, FloatText, Hit, PlayerTarget, SkillLock};
use super::log::{GameLog, C_BAD};
use super::fx;
use super::model::{anim, AnimState, ModelReady};
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

/// Öfke süresince ellerdeki WoW kan hırsı efekti (süre bitince silinir).
#[derive(Component)]
struct RageAura;

pub fn plugin(app: &mut App) {
    app.init_resource::<SkillCooldowns>()
        .init_resource::<Active>()
        .add_message::<CastSkill>()
        .add_systems(Update, (keys, cast, run_active, rage_aura).chain());
}

fn keys(input: Res<ButtonInput<KeyCode>>, mut ev: MessageWriter<CastSkill>) {
    for (i, k) in SKILL_KEYS.iter().enumerate() {
        if input.just_pressed(*k) {
            ev.write(CastSkill(i));
        }
    }
}

fn cast(
    time: Res<Time>,
    mut ev: MessageReader<CastSkill>,
    mut stats: ResMut<PlayerStats>,
    mut cds: ResMut<SkillCooldowns>,
    mut active: ResMut<Active>,
    mut log: ResMut<GameLog>,
    target: Res<PlayerTarget>,
    player: Query<(Entity, &Transform), With<Player>>,
    enemies: Query<&Transform, (With<Enemy>, Without<Dying>, Without<Player>)>,
    mut body: Query<&mut AnimState, With<Body>>,
    mut lock: ResMut<SkillLock>,
    mut commands: Commands,
    (server, hands): (Res<AssetServer>, Query<&ModelReady, With<Body>>),
    mut auto: ResMut<super::combat::AutoAttack>,
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
        let Ok((player_e, pxf)) = player.single() else { continue };
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
        // WoW: saldırı becerisi otomatik saldırıyı da başlatır.
        if target.0.is_some() && def.id != SkillId::Rage {
            auto.0 = true;
        }
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
        // WoW efektleri: döküm anında.
        match def.id {
            SkillId::Cleave => {
                let at = Transform::from_translation(pxf.translation).with_rotation(pxf.rotation);
                fx::spawn(&mut commands, &server, fx::path::CLEAVE_CAST, at, None, 1.5);
            }
            SkillId::Whirl => {
                fx::spawn(&mut commands, &server, fx::path::WHIRLWIND, Transform::default(), Some(player_e), 1.0);
            }
            SkillId::Rage => {
                stats.rage_until = now + 20.0;
                let at = Transform::from_translation(pxf.translation);
                fx::spawn(&mut commands, &server, fx::path::BATTLE_SHOUT, at, None, 2.5);
                // İki elde kan hırsı (takma noktası 1 sağ, 2 sol el).
                if let Ok(ready) = hands.single() {
                    for id in [1u16, 2] {
                        if let Some((joint, off)) = ready.attachment(id) {
                            let at = Transform::from_translation(off);
                            let e = fx::spawn(&mut commands, &server, fx::path::BLOODLUST_HAND, at, Some(joint), 20.0);
                            commands.entity(e).insert(RageAura);
                        }
                    }
                }
            }
            SkillId::Charge => {
                fx::spawn(&mut commands, &server, fx::path::CHARGE_TRAIL, Transform::default(), Some(player_e), 1.0);
            }
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
) -> Vec<(Entity, Vec3)> {
    let mut struck = Vec::new();
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
        struck.push((e, xf.translation + Vec3::Y * en.height * xf.scale.y * 0.6));
    }
    struck
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
    server: Res<AssetServer>,
    col: benilla_world::collision::WorldCollision,
    stats: Res<PlayerStats>,
) {
    let Some(skill) = active.skill else { return };
    let Ok(mut pxf) = player.single_mut() else { return };
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
                for (_, at) in strike(pos, fwd, 3.6, 0.0, 2.2, &derived, &enemies, &mut hits, &mut commands) {
                    fx::spawn(&mut commands, &server, fx::path::CLEAVE_IMPACT, Transform::from_translation(at), None, 1.2);
                }
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
            } else if active.hits_done == 0 {
                active.hits_done = 1;
                let f = flat.normalize_or_zero();
                for (e, at) in strike(pxf.translation, f, 2.8, 0.2, 1.8, &derived, &enemies, &mut hits, &mut commands) {
                    // Hamle hedefi sersemler (WoW Charge Stun).
                    if Some(e) == active.charge_target {
                        let stun = Transform::from_translation(at + Vec3::Y * 0.9);
                        fx::spawn(&mut commands, &server, fx::path::STUN, stun, None, 1.5);
                    }
                }
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

/// Öfke erken biterse (ölüm) ya da süre dolunca el efektlerini kaldır.
fn rage_aura(mut commands: Commands, time: Res<Time>, stats: Res<PlayerStats>, q: Query<Entity, With<RageAura>>) {
    if time.elapsed_secs() <= stats.rage_until {
        return;
    }
    for e in &q {
        commands.entity(e).despawn();
    }
}
