//! Metin2 tarzı 3. şahıs kamera: sağ tık sürükle döndür, tekerlek yakınlaştır, oyuncuyu takip et.

use bevy::camera::{PerspectiveProjection, Projection};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::{AccumulatedMouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::render::view::Hdr;

use super::player::Player;

#[derive(Component)]
pub struct OrbitCam {
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
}

pub fn plugin(app: &mut App) {
    app.add_systems(
        Startup,
        spawn_camera.after(benilla_assets::AssetSet::Open),
    )
    .add_systems(
        Update,
        orbit
            .after(super::player::move_player)
            .in_set(benilla_world::schedule::WorldStage::Input),
    );
}

fn spawn_camera(mut commands: Commands, msaa: Res<benilla_world::view::MsaaSetting>) {
    let start = benilla_assets::coords::wow_to_bevy(super::start_wow());
    commands.spawn((
        Camera3d::default(),
        benilla_world::view::WorldCamera,
        msaa.level(),
        Projection::from(PerspectiveProjection {
            far: benilla_world::view::CAM_FAR,
            near: benilla_world::view::NEARCLIP_DEFAULT,
            fov: 1.0,
            ..default()
        }),
        Hdr,
        Tonemapping::None,
        benilla_world::ffx_glow::FfxGlow::WORLD,
        bevy::pbr::DistanceFog {
            color: Color::srgb(0.62, 0.74, 0.86),
            falloff: bevy::pbr::FogFalloff::Linear { start: 260.0, end: 1100.0 },
            ..default()
        },
        Transform::from_translation(start + Vec3::new(0.0, 8.0, 10.0)).looking_at(start, Vec3::Y),
        OrbitCam {
            yaw: std::env::var("AZM_YAW").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0),
            pitch: std::env::var("AZM_PITCH").ok().and_then(|v| v.parse().ok()).unwrap_or(-0.45),
            dist: std::env::var("AZM_DIST").ok().and_then(|v| v.parse().ok()).unwrap_or(9.0),
        },
    ));
}

fn orbit(
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    player: Query<&Transform, (With<Player>, Without<OrbitCam>)>,
    mut cam: Query<(&mut Transform, &mut OrbitCam)>,
    collision: benilla_world::collision::WorldCollision,
) {
    let Ok((mut xf, mut cam)) = cam.single_mut() else {
        return;
    };
    if buttons.pressed(MouseButton::Right) || buttons.pressed(MouseButton::Middle) {
        cam.yaw -= motion.delta.x * 0.005;
        cam.pitch = (cam.pitch - motion.delta.y * 0.005).clamp(-1.55, 0.25);
    }
    for ev in wheel.read() {
        cam.dist = (cam.dist * (1.0 - ev.y * 0.1)).clamp(3.0, 30.0);
    }
    if cam.dist > 30.0 && std::env::var("AZM_DIST").is_err() {
        cam.dist = 30.0;
    }
    let Ok(p) = player.single() else {
        return;
    };
    let target = p.translation + Vec3::Y * 1.6;
    let rot = Quat::from_euler(EulerRot::YXZ, cam.yaw, cam.pitch, 0.0);
    let back = rot * Vec3::Z;
    // Kamera kolu duvara/araziye çarparsa kısalt.
    let mut d = cam.dist;
    if super::terrain::data().is_none() {
        if let Some(hit) = collision.cast_camera(target, back * cam.dist, false) {
            d = (hit - 0.2).max(1.0);
        }
    }
    xf.translation = target + back * d;
    if let Some(g) = super::terrain::ground(xf.translation) {
        xf.translation.y = xf.translation.y.max(g + 0.6);
    }
    xf.look_at(target, Vec3::Y);
}
