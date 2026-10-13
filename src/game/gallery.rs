//! Geliştirme: `AZM_GALLERY=1` ile başlangıç noktasının önüne aday modelleri dizer.

use bevy::prelude::*;

use super::model::{load_m2, AnimState, WowModel};

const ITEMS: &[(&str, &[&str], f32)] = &[
    ("World/Azeroth/BurningSteppes/PassiveDoodads/LavaPillar/LavaPillar01.m2", &[], 0.3),
    ("World/Azeroth/BurningSteppes/PassiveDoodads/LavaPillar/LavaPillar03.m2", &[], 0.3),
    ("World/Azeroth/BurningSteppes/PassiveDoodads/Rocks/LavaRock01.m2", &[], 0.5),
    ("World/Azeroth/BurningSteppes/PassiveDoodads/LavaFalls/LavaFallsBlackRock01.m2", &[], 0.2),
    ("World/Azeroth/BurningSteppes/PassiveDoodads/WarlockShrine/WarlockShrine.m2", &[], 0.5),
    ("World/Azeroth/DeadwindPass/PassiveDoodads/DeadwindTotems/DeadwindTotem02.m2", &[], 0.6),
];

pub fn plugin(app: &mut App) {
    if std::env::var("AZM_GALLERY").is_ok() {
        app.add_systems(Startup, spawn.after(benilla_assets::AssetSet::Open));
    }
}

fn spawn(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut wa: ResMut<benilla_assets::WorldAssets>,
    mut images: ResMut<Assets<Image>>,
) {
    let start = benilla_assets::coords::wow_to_bevy(super::start_wow());
    for (i, (path, skins, scale)) in ITEMS.iter().enumerate() {
        let mut wm = WowModel::new(load_m2(&server, path));
        for (k, s) in skins.iter().enumerate() {
            wm.skins[k] = wa.texture(s, (true, true), &mut images);
        }
        let x = (i as f32 - ITEMS.len() as f32 / 2.0) * 5.0;
        commands.spawn((
            wm,
            AnimState::default(),
            super::GroundSnap,
            Transform::from_translation(start + Vec3::new(-x, 0.0, 9.0))
                .with_scale(Vec3::splat(*scale)),
            Visibility::default(),
        ));
    }
}
