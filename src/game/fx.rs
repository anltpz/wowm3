//! WoW büyü efektleri (`Spells\*.m2`): modelin kendisi (`model.rs`), parçacık yayıcıları ve şeritleri
//! (benilla-world'ün 1.12 parçacık sistemi). Belirli süre sonra kök silinir; yayıcılar sahibini
//! kaybedince (`OwnerLoss::Drain`) kalan parçacıkları yerinde söndürür.

use bevy::prelude::*;

use benilla_assets::M2Model;
use benilla_world::particles::{self, EmitClock, EmitterFrames, OwnerLoss};
use benilla_world::ribbons::{self, RibbonSeq};

use super::model::{load_m2, AnimState, ModelReady, WowModel};

/// Savaşçı becerileri ve olaylar için WoW efekt modelleri.
pub mod path {
    pub const CLEAVE_CAST: &str = "Spells/Cleave_Cast_Base.m2";
    pub const CLEAVE_IMPACT: &str = "Spells/Cleave_Impact_Chest.m2";
    pub const WHIRLWIND: &str = "Spells/Whirlwind_State_Base.m2";
    pub const BATTLE_SHOUT: &str = "Spells/BattleShout_Cast_Base.m2";
    pub const BLOODLUST_HAND: &str = "Spells/Bloodlust_State_Hand.m2";
    pub const CHARGE_TRAIL: &str = "Spells/ChargeTrail.m2";
    pub const STUN: &str = "Spells/StunSwirl_State_Head.m2";
    pub const LEVEL_UP: &str = "Spells/LevelUp/LevelUp.m2";
}

/// Süreli efekt örneği.
#[derive(Component)]
pub struct SpellFx {
    pub life: f32,
    age: f32,
    armed: bool,
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (arm_emitters, age_out, level_up));
}

/// Seviye atlayınca oyuncunun üstünde WoW seviye efekti (ilk okumada / kayıt yüklenince değil).
fn level_up(
    mut commands: Commands,
    server: Res<AssetServer>,
    stats: Res<super::stats::PlayerStats>,
    player: Query<Entity, With<super::player::Player>>,
    mut last: Local<Option<u32>>,
) {
    let Ok(p) = player.single() else { return };
    if last.is_some_and(|l| stats.level > l) {
        spawn(&mut commands, &server, path::LEVEL_UP, Transform::default(), Some(p), 3.0);
    }
    *last = Some(stats.level);
}

/// `path` efektini `at` konumunda (ya da `parent`'a bağlı, yerel `at`) `life` saniye oynatır.
pub fn spawn(
    commands: &mut Commands,
    server: &AssetServer,
    path: &str,
    at: Transform,
    parent: Option<Entity>,
    life: f32,
) -> Entity {
    let mut e = commands.spawn((
        WowModel::new(load_m2(server, path)),
        AnimState::default(),
        SpellFx { life, age: 0.0, armed: false },
        at,
        Visibility::default(),
    ));
    if let Some(p) = parent {
        e.insert(ChildOf(p));
    }
    e.id()
}

/// Model kurulunca (eklemler hazır) yayıcıları ve şeritleri bağla.
fn arm_emitters(
    mut commands: Commands,
    models: Res<Assets<M2Model>>,
    mut q: Query<(Entity, &WowModel, &ModelReady, &mut SpellFx)>,
) {
    for (root, wm, ready, mut fx) in &mut q {
        if fx.armed {
            continue;
        }
        let Some(m) = models.get(&wm.handle) else { continue };
        fx.armed = true;
        for em in &m.emitters {
            let owner = ready
                .joints
                .get(em.def.bone as usize)
                .map_or((root, [0.0; 3]), |&j| (j, em.bone_pivot));
            particles::spawn_emitter(
                &mut commands,
                em,
                Transform::IDENTITY,
                EmitterFrames {
                    owner: Some(owner),
                    anchor: Some(root),
                    on_owner_loss: OwnerLoss::Drain,
                    ..default()
                },
                EmitClock::Effect(None),
            );
        }
        for rb in &m.ribbons {
            let (owner, use_pivot) = ready.joints.get(rb.def.bone as usize).map_or((root, false), |&j| (j, true));
            ribbons::spawn_ribbon(&mut commands, rb, owner, use_pivot, 1.0, RibbonSeq::Fixed(0), None, None);
        }
    }
}

fn age_out(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut SpellFx)>) {
    for (e, mut fx) in &mut q {
        fx.age += time.delta_secs();
        if fx.age >= fx.life {
            commands.entity(e).despawn();
        }
    }
}
