//! Oyun katmanı: oyuncu, kamera, moblar, metin taşları, envanter, artı basma.

use bevy::prelude::*;

pub mod camera;
pub mod character;
pub mod combat;
pub mod devshot;
pub mod gallery;
pub mod inventory;
pub mod items;
pub mod log;
pub mod loot;
pub mod metin;
pub mod mobs;
pub mod npc;
pub mod model;
pub mod physics;
pub mod player;
pub mod quest;
pub mod skills;
pub mod stats;
pub mod terrain;
pub mod ui;

/// Oyunun başladığı nokta (WoW dünya koordinatı): Northshire Abbey önü.
pub const START_WOW_DEFAULT: [f32; 3] = [-8935.0, -140.0, 82.0];

/// Başlangıç noktası (WoW koordinatı); `AZM_FOCUS=x,y,z` ile değiştirilebilir.
pub fn start_wow() -> [f32; 3] {
    static S: std::sync::OnceLock<[f32; 3]> = std::sync::OnceLock::new();
    *S.get_or_init(|| {
        std::env::var("AZM_FOCUS")
            .ok()
            .and_then(|v| {
                let c: Vec<f32> = v.split(',').filter_map(|p| p.trim().parse().ok()).collect();
                (c.len() == 3).then(|| [c[0], c[1], c[2]])
            })
            .or_else(|| {
                let v = std::env::var("AZM_MAPSTART").ok()?;
                let (x, y) = v.split_once(',')?;
                let d = terrain::data()?;
                Some(benilla_assets::coords::bevy_to_wow(d.to_world(x.trim().parse().ok()?, y.trim().parse().ok()?)))
            })
            .unwrap_or_else(|| match terrain::data() {
                // Pyungmoo: köy meydanı (wiki koordinatı 382, 655).
                Some(d) => benilla_assets::coords::bevy_to_wow(d.to_world(382.0, 662.0)),
                None => START_WOW_DEFAULT,
            })
    })
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(terrain::plugin);
        app.add_plugins((
            camera::plugin,
            character::plugin,
            player::plugin,
            devshot::plugin,
            devshot::auto_plugin,
            devshot::mobgal_plugin,
            model::plugin,
            gallery::plugin,
            combat::plugin,
            stats::plugin,
            log::plugin,
        ))
        .add_plugins((
            mobs::plugin,
            metin::plugin,
            loot::plugin,
            npc::plugin,
            ui::plugin,
            skills::plugin,
            quest::plugin,
        ))
        .init_resource::<inventory::Inventory>()
        .add_systems(Update, (snap_to_ground, mobs::heal_returning, npc::auto_close));
    }
}

/// Bu bileşeni taşıyan varlıklar her karede zemine oturtulur.
#[derive(Component)]
pub struct GroundSnap;

fn snap_to_ground(
    mut q: Query<&mut Transform, With<GroundSnap>>,
    collision: benilla_world::collision::WorldCollision,
) {
    for mut xf in &mut q {
        if let Some(g) = player::ground_at(&collision, xf.translation) {
            xf.translation.y = g;
        }
    }
}
