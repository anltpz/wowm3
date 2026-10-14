//! Motor açılışı: benilla-worldview'in `run`'ının oyunlu kopyası. MPQ kaynağını DefaultPlugins'ten
//! önce kaydeder, motorun eklenti grubunu ekler, ardından oyun eklentisini.

use bevy::prelude::*;
use std::path::PathBuf;

/// Varsayılan veri klasörü: AzerothSurvivors kurulumunun `Data`'sı (patch.MPQ + patch-2.MPQ).
const DEFAULT_DATA: &str = r"C:\Users\ANIL\Desktop\AzerothSurvivors\Data";

fn data_dir() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("WOW_DATA").map(PathBuf::from) {
        if p.is_dir() {
            return Some(p);
        }
    }
    // Exe'nin yanında bir `Data` klasörü varsa onu tercih et (dağıtım için).
    if let Some(p) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.join("Data")))
    {
        if p.is_dir() {
            return Some(p);
        }
    }
    let p = PathBuf::from(DEFAULT_DATA);
    p.is_dir().then_some(p)
}

pub fn run() -> AppExit {
    let Some(data) = data_dir() else {
        eprintln!("AzerothMetin: WoW Data klasörü bulunamadı (WOW_DATA ile belirtin)");
        return AppExit::error();
    };
    // benilla_formats::wow_data() aynı klasörü bulsun (DBC/zincir okuyucuları env'e bakar).
    // SAFETY: tek iş parçacıklı açılışta, başka thread yokken.
    unsafe { std::env::set_var("WOW_DATA", &data) };
    // Varsayılan grafik arka ucu DX12: Vulkan sürücüsü takılan bir süreçten sonra kilitlenebiliyor.
    if std::env::var_os("WGPU_BACKEND").is_none() {
        // SAFETY: yukarıdakiyle aynı, tek iş parçacıklı açılış.
        unsafe { std::env::set_var("WGPU_BACKEND", "dx12") };
    }

    let mut app = App::new();
    // Silinmiş bir varlığa ertelenmiş komut (ör. model kurulurken ömrü dolan efekt, ölen mob) oyunu
    // çökertmesin: Bevy'nin varsayılanı panik, burada uyarı yazılır.
    app.set_error_handler(bevy::ecs::error::warn);
    app.insert_resource(benilla_world::build_id::BuildId {
        version: env!("CARGO_PKG_VERSION"),
        ..Default::default()
    });

    if let Err(e) = benilla_assets::register_mpq_source(&mut app, &data) {
        eprintln!("AzerothMetin: mpq:// kaynağı açılamadı ({e:#})");
        return AppExit::error();
    }

    let win = std::env::var("WOW_WIN")
        .ok()
        .and_then(|v| {
            let (w, h) = v.split_once('x')?;
            Some(UVec2::new(w.parse().ok()?, h.parse().ok()?))
        })
        .unwrap_or(UVec2::new(1600, 900));
    app.add_plugins(benilla_world::boot::tuned_default_plugins(Window {
        title: "Azeroth Metin".into(),
        resolution: win.into(),
        ..default()
    }))
    .add_plugins(benilla_world::thread_qos::ThreadQosPlugin)
    .add_plugins(benilla_world::bgwin::BgWinPlugin);

    app.add_plugins(benilla_world::world_plugins::WorldPlugins);
    app.insert_resource(benilla_world::schedule::WorldLive(true));

    app.add_plugins(crate::game::GamePlugin);

    benilla_assets::register_asset_loaders(&mut app);
    app.run()
}
