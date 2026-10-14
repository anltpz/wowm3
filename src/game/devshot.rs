//! Test aracı: `AZM_SHOT=<png>` ile `AZM_SHOT_AT` saniyede (varsayılan 25) tek kare yazıp çıkar.
//! `AZM_KEYS` ile basit otomatik girdi senaryosu (ör. "w:2" = 2 sn W bas) da çalıştırılabilir.

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

pub fn plugin(app: &mut App) {
    if std::env::var("AZM_SHOT").is_ok() {
        app.add_systems(Update, shoot_and_exit);
    }
}

fn shoot_and_exit(
    time: Res<Time>,
    mut commands: Commands,
    mut fired: Local<Vec<f32>>,
    mut exit: MessageWriter<AppExit>,
) {
    // AZM_SHOT_AT="20,30,40" birden çok kare: dosya adına _0, _1 ... eklenir.
    let ats: Vec<f32> = std::env::var("AZM_SHOT_AT")
        .ok()
        .map(|v| v.split(',').filter_map(|p| p.trim().parse().ok()).collect())
        .filter(|v: &Vec<f32>| !v.is_empty())
        .unwrap_or_else(|| vec![25.0]);
    let t = time.elapsed_secs();
    let i = fired.len();
    if i < ats.len() && t >= ats[i] {
        let base = std::env::var("AZM_SHOT").unwrap_or_default();
        let path = if ats.len() > 1 {
            base.replace(".png", &format!("_{i}.png"))
        } else {
            base
        };
        info!("devshot: {path}");
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
        fired.push(t);
    }
    if fired.len() == ats.len() && t >= fired[fired.len() - 1] + 2.0 {
        exit.write(AppExit::Success);
    }
}

/// Test otopilotu (`AZM_AUTO=1`): en yakın düşmanı hedefler, becerileri sırayla kullanır.
pub fn auto_plugin(app: &mut App) {
    if std::env::var("AZM_AUTO").is_ok() {
        app.add_systems(Update, autopilot);
    }
}

fn autopilot(
    time: Res<Time>,
    mut target: ResMut<super::combat::PlayerTarget>,
    player: Query<&Transform, With<super::player::Player>>,
    enemies: Query<(Entity, &Transform, &super::combat::Enemy), Without<super::combat::Dying>>,
    mut cast: MessageWriter<super::skills::CastSkill>,
    mut stats: ResMut<super::stats::PlayerStats>,
    (mut auto, mut move_to): (ResMut<super::combat::AutoAttack>, ResMut<super::combat::MoveTo>),
    mut acc: Local<f32>,
    mut k: Local<usize>,
) {
    *acc += time.delta_secs();
    if *acc < 1.2 || time.elapsed_secs() < 8.0 {
        return;
    }
    *acc = 0.0;
    // Testte becerilerin hepsi açık olsun.
    if stats.level < 6 {
        stats.level = 6;
    }
    let Ok(p) = player.single() else { return };
    if target.0.is_none() {
        target.0 = enemies
            .iter()
            .filter(|(_, xf, _)| xf.translation.distance(p.translation) < 60.0)
            .min_by(|a, b| a.1.translation.distance(p.translation).total_cmp(&b.1.translation.distance(p.translation)))
            .map(|(e, _, _)| e);
    }
    // WoW kontrolleri tıkla-yürü yapmaz: otopilot hedefe kendisi yürür ve saldırıyı açar.
    if let Some((_, txf, _)) = target.0.and_then(|t| enemies.get(t).ok()) {
        auto.0 = true;
        move_to.0 = (txf.translation.distance(p.translation) > 2.5).then_some(txf.translation);
    }
    if target.0.is_some() {
        cast.write(super::skills::CastSkill(*k % 4));
        *k += 1;
    }
}

/// `AZM_MOBGAL=1`: başlangıcın önüne her türden bir canavar dizer (hareketsiz, edilgen).
pub fn mobgal_plugin(app: &mut App) {
    if std::env::var("AZM_MOBGAL").is_ok() {
        app.add_systems(Update, mob_gallery);
    }
}

fn mob_gallery(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut wa: Option<ResMut<benilla_assets::WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    db: Option<Res<super::character::CharDb>>,
    col: benilla_world::collision::WorldCollision,
    player: Query<(&Transform, &super::player::Player)>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    let (Some(wa), Some(db), Ok((pxf, p))) = (wa.as_deref_mut(), db, player.single()) else { return };
    if !p.grounded {
        return;
    }
    *done = true;
    let ids = [103u32, 109, 115, 123, 126, 133, 194];
    for (i, id) in ids.iter().enumerate() {
        let x = (i as f32 - 3.0) * 3.5;
        let mut at = pxf.translation + Vec3::new(x, 0.0, 8.0);
        if let Some(g) = super::physics::ground_at(&col, at) {
            at.y = g;
        }
        let e = super::mobs::spawn_mob(&mut commands, &server, wa, &mut images, Some(&db), super::mobs::mob(*id), at, None, false);
        commands
            .entity(e)
            .insert((Transform::from_translation(at).with_scale(Vec3::splat(super::mobs::mob(*id).scale)), super::mobs::Passive));
    }
}
