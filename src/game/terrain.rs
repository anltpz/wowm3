//! Pyungmoo (Metin2 Jinno Map1) arazisi. Veri `tools/build_pyungmoo.py` ile wiki kuşbakışı
//! haritasından (1 px = 1 m) üretilir ve exe'ye gömülür: yükseklik ızgarası (2 m), splat, su
//! maskesi, ağaçlar, canavar/metin bölgeleri. Harita koordinatı (x sağa, y aşağı, metre) Bevy'de
//! `ORIGIN + (x, h, y)`'dir; dünya akışı boş okyanusa odaklanır, sahne yalnızca bu arazidir.

use std::sync::OnceLock;

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Extent3d, TextureDimension, TextureFormat};
use bevy::shader::ShaderRef;

use benilla_assets::WorldAssets;

use super::model::{load_m2, load_wmo, WowModel, WowWmo};

pub const MAP_W: f32 = 1024.0;
pub const MAP_H: f32 = 1280.0;
const GW: usize = 513;
const GH: usize = 641;
const CELL: f32 = 2.0;
/// Su yüzeyi yüksekliği (arazi yükseklik birimiyle).
pub const WATER_LEVEL: f32 = -0.35;
/// Haritanın dışına uzanan dağ eteği (m).
const MARGIN: f32 = 192.0;

/// Pyungmoo'nun WoW dünyasındaki yeri: boş okyanus (ADT yok), harita merkezi burada.
pub const FOCUS_WOW: [f32; 3] = [-8000.0, 14000.0, 0.0];

static HEIGHT: &[u8] = include_bytes!("../../assets/pyungmoo/height.bin");
static SPLAT: &[u8] = include_bytes!("../../assets/pyungmoo/splat.png");
static WATER: &[u8] = include_bytes!("../../assets/pyungmoo/water.png");
static TREES: &str = include_str!("../../assets/pyungmoo/trees.json");
static METINZ: &[u8] = include_bytes!("../../assets/pyungmoo/metinzones.png");

pub struct TerrainData {
    pub origin: Vec3,
    heights: Vec<f32>,
    water: Vec<u8>,
    water_w: usize,
    water_h: usize,
    pub metin_zones: Vec<u8>,
    pub zone_w: usize,
    pub zone_h: usize,
    bridges: Vec<Bridge>,
    blockers: Vec<Blocker>,
}

#[derive(Clone, Copy)]
struct Bridge {
    a: Vec2,
    b: Vec2,
    half_w: f32,
    deck: f32,
}

/// Bina ayak izi (yönlü kutu, harita metresi).
#[derive(Clone, Copy)]
struct Blocker {
    c: Vec2,
    half: Vec2,
    yaw: f32,
}

static DATA: OnceLock<TerrainData> = OnceLock::new();

/// Pyungmoo modu açık mı (varsayılan açık; `AZM_ELWYNN=1` eski Northshire).
pub fn enabled() -> bool {
    std::env::var("AZM_ELWYNN").is_err()
}

pub fn data() -> Option<&'static TerrainData> {
    DATA.get()
}

fn decode_png(bytes: &[u8]) -> image::DynamicImage {
    image::load_from_memory_with_format(bytes, image::ImageFormat::Png).expect("gömülü png")
}

/// Köprüler (harita metresi): iki uç, yarı genişlik.
const BRIDGES: &[((f32, f32), (f32, f32), f32)] = &[
    ((148.0, 445.0), (148.0, 495.0), 3.5),
    ((357.0, 480.0), (357.0, 528.0), 4.0),
    ((482.0, 638.0), (532.0, 638.0), 4.0),
    ((420.0, 754.0), (420.0, 803.0), 4.0),
    ((665.0, 538.0), (665.0, 608.0), 3.5),
    ((716.0, 314.0), (779.0, 314.0), 3.0),
    ((234.0, 1063.0), (290.0, 1063.0), 3.5),
    ((276.0, 903.0), (306.0, 957.0), 3.0),
];

/// Köy binaları: (x, y, yaw°, WMO yolu, ölçek, ayak izi yarı boyu x/y).
pub const BUILDINGS: &[(f32, f32, f32, &str, f32, f32, f32)] = &[
    (279.0, 640.0, 90.0, "World/wmo/Azeroth/Buildings/Human_Barracks/Human_Barracks.wmo", 0.55, 9.0, 11.0),
    (284.0, 674.0, 90.0, "World/wmo/Azeroth/Buildings/HumanTwoStory/HumanTwoStory.wmo", 0.55, 6.0, 5.0),
    (305.0, 560.0, 0.0, "World/wmo/Azeroth/Buildings/Westfall_HumanTwoStory/Westfall_HumanTwoStory.wmo", 0.6, 7.0, 10.0),
    (406.0, 573.0, 160.0, "World/wmo/Azeroth/Buildings/GoldshireInn/GoldshireInn.wmo", 0.45, 10.0, 8.0),
    (443.0, 603.0, 200.0, "World/wmo/Azeroth/Buildings/HumanTwoStory/HumanTwoStory.wmo", 0.6, 8.0, 6.0),
    (459.0, 627.0, 270.0, "World/wmo/Azeroth/Buildings/HumanTwoStory/HumanTwoStory.wmo", 0.5, 6.0, 6.0),
    (459.0, 652.0, 270.0, "World/wmo/Azeroth/Buildings/Duskwood_HumanTwoStory/Duskwood_HumanTwoStory.wmo", 0.5, 6.0, 6.0),
    (428.0, 680.0, 225.0, "World/wmo/Azeroth/Buildings/Stable/stable.wmo", 0.5, 7.0, 5.0),
    (386.0, 706.0, 0.0, "World/wmo/Azeroth/Buildings/Duskwood_Blacksmith/Duskwood_Blacksmith.wmo", 0.55, 8.0, 5.0),
    (346.0, 746.0, 0.0, "World/wmo/Azeroth/Buildings/GuardTower/GuardTower.wmo", 0.5, 4.0, 4.0),
    (292.0, 808.0, 90.0, "World/wmo/Azeroth/Buildings/human_farm/farm.wmo", 0.5, 7.0, 6.0),
    (296.0, 840.0, 90.0, "World/wmo/Azeroth/Buildings/Human_Barn_Silo/barn.wmo", 0.45, 7.0, 6.0),
];

/// Garnizon duvarı (dikdörtgen) ve meydan dekoru.
const COMPOUND: (f32, f32, f32, f32) = (255.0, 612.0, 312.0, 695.0);

impl TerrainData {
    fn load() -> Self {
        let mut heights = Vec::with_capacity(GW * GH);
        for c in HEIGHT.chunks_exact(4) {
            heights.push(f32::from_le_bytes([c[0], c[1], c[2], c[3]]));
        }
        let water = decode_png(WATER).to_luma8();
        let (water_w, water_h) = (water.width() as usize, water.height() as usize);
        let mz = decode_png(METINZ).to_luma8();
        let (zone_w, zone_h) = (mz.width() as usize, mz.height() as usize);
        let focus = benilla_assets::coords::wow_to_bevy(FOCUS_WOW);
        let origin = Vec3::new(focus.x - MAP_W / 2.0, 0.0, focus.z - MAP_H / 2.0);
        let mut d = Self {
            origin,
            heights,
            water: water.into_raw(),
            water_w,
            water_h,
            metin_zones: mz.into_raw(),
            zone_w,
            zone_h,
            bridges: Vec::new(),
            blockers: Vec::new(),
        };
        for &((ax, ay), (bx, by), w) in BRIDGES {
            let a = Vec2::new(ax, ay);
            let b = Vec2::new(bx, by);
            let deck = d.raw_height(ax, ay).max(d.raw_height(bx, by)).max(0.4) + 0.35;
            d.bridges.push(Bridge { a, b, half_w: w, deck });
        }
        for &(x, y, yaw, _, _, hx, hy) in BUILDINGS {
            d.blockers.push(Blocker {
                c: Vec2::new(x, y),
                half: Vec2::new(hx, hy),
                yaw: yaw.to_radians(),
            });
        }
        d
    }

    /// Izgaradan çift doğrusal yükseklik; harita dışı → kenar + dağ yükselmesi (doğu: deniz).
    fn raw_height(&self, mx: f32, my: f32) -> f32 {
        let cx = mx.clamp(0.0, MAP_W);
        let cy = my.clamp(0.0, MAP_H);
        let fx = (cx / CELL).min((GW - 1) as f32 - 1e-3);
        let fy = (cy / CELL).min((GH - 1) as f32 - 1e-3);
        let (x0, y0) = (fx as usize, fy as usize);
        let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
        let h = |x: usize, y: usize| self.heights[y * GW + x];
        let a = h(x0, y0) * (1.0 - tx) + h(x0 + 1, y0) * tx;
        let b = h(x0, y0 + 1) * (1.0 - tx) + h(x0 + 1, y0 + 1) * tx;
        let mut v = a * (1.0 - ty) + b * ty;
        // Kenar dışı: batı/kuzey/güney dağlar yükselir, doğu denize iner.
        let out_w = (-mx).max(0.0);
        let out_n = (-my).max(0.0);
        let out_s = (my - MAP_H).max(0.0);
        let out_e = (mx - MAP_W).max(0.0);
        let rise = out_w.max(out_n).max(out_s);
        if rise > 0.0 && cx < 900.0 {
            v = v.max(10.0) + rise * 0.45;
        }
        if out_e > 0.0 {
            v = v.min(-4.0);
        }
        v
    }

    /// Bevy konumu → harita metresi.
    pub fn to_map(&self, p: Vec3) -> Vec2 {
        Vec2::new(p.x - self.origin.x, p.z - self.origin.z)
    }

    /// Harita metresi → Bevy konumu (zemin yüksekliğinde).
    pub fn to_world(&self, mx: f32, my: f32) -> Vec3 {
        Vec3::new(self.origin.x + mx, self.ground_map(mx, my), self.origin.z + my)
    }

    fn bridge_at(&self, m: Vec2) -> Option<f32> {
        for b in &self.bridges {
            let ab = b.b - b.a;
            let t = ((m - b.a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
            let q = b.a + ab * t;
            if q.distance(m) <= b.half_w {
                return Some(b.deck);
            }
        }
        None
    }

    /// Yürünebilir zemin yüksekliği (köprüler dahil).
    pub fn ground_map(&self, mx: f32, my: f32) -> f32 {
        let h = self.raw_height(mx, my);
        match self.bridge_at(Vec2::new(mx, my)) {
            Some(d) => d.max(h),
            None => h,
        }
    }

    pub fn ground(&self, p: Vec3) -> Option<f32> {
        let m = self.to_map(p);
        if m.x < -MARGIN || m.y < -MARGIN || m.x > MAP_W + MARGIN || m.y > MAP_H + MARGIN {
            return None;
        }
        Some(self.ground_map(m.x, m.y))
    }

    /// Su derinliği (köprü üstü 0).
    pub fn water_depth(&self, p: Vec3) -> f32 {
        let m = self.to_map(p);
        if self.bridge_at(m).is_some() {
            return 0.0;
        }
        (WATER_LEVEL - self.raw_height(m.x, m.y)).max(0.0)
    }

    pub fn is_water(&self, m: Vec2) -> bool {
        if m.x < 0.0 || m.y < 0.0 || m.x >= self.water_w as f32 || m.y >= self.water_h as f32 {
            return m.x > MAP_W;
        }
        self.water[m.y as usize * self.water_w + m.x as usize] > 127
    }

    /// Bina ayak izinin içinde mi.
    pub fn blocked(&self, m: Vec2) -> bool {
        self.blockers.iter().any(|b| {
            let d = m - b.c;
            let (s, c) = b.yaw.sin_cos();
            let local = Vec2::new(d.x * c - d.y * s, d.x * s + d.y * c);
            local.x.abs() < b.half.x && local.y.abs() < b.half.y
        }) || {
            // Garnizon duvarı (kapı: batı ve doğu ortası).
            let (x0, y0, x1, y1) = COMPOUND;
            let near = |v: f32, w: f32| (v - w).abs() < 0.8;
            let inside_x = m.x > x0 - 0.8 && m.x < x1 + 0.8;
            let inside_y = m.y > y0 - 0.8 && m.y < y1 + 0.8;
            let gate_y = (m.y - 653.0).abs() < 5.0;
            let gate_x = (m.x - 283.0).abs() < 5.0;
            ((near(m.x, x0) || near(m.x, x1)) && inside_y && !gate_y)
                || ((near(m.y, y0) || near(m.y, y1)) && inside_x && !gate_x)
        }
    }

    /// Metin bölgesi (0 yok, 1..5).
    pub fn metin_zone(&self, m: Vec2) -> u8 {
        let x = (m.x / MAP_W * self.zone_w as f32) as isize;
        let y = (m.y / MAP_H * self.zone_h as f32) as isize;
        if x < 0 || y < 0 || x >= self.zone_w as isize || y >= self.zone_h as isize {
            return 0;
        }
        self.metin_zones[y as usize * self.zone_w + x as usize]
    }

    /// Yürünebilir açık nokta mı (su, bina, dik yamaç değil).
    pub fn open(&self, m: Vec2) -> bool {
        if m.x < 4.0 || m.y < 4.0 || m.x > MAP_W - 4.0 || m.y > MAP_H - 4.0 {
            return false;
        }
        if self.is_water(m) || self.blocked(m) {
            return false;
        }
        let h = self.raw_height(m.x, m.y);
        if h < WATER_LEVEL + 0.3 {
            return false;
        }
        [(2.0, 0.0), (-2.0, 0.0), (0.0, 2.0), (0.0, -2.0)]
            .iter()
            .all(|(dx, dy)| (self.raw_height(m.x + dx, m.y + dy) - h).abs() < 1.2)
    }
}

/// Global sorgu: Pyungmoo zemin yüksekliği.
pub fn ground(p: Vec3) -> Option<f32> {
    data().and_then(|d| d.ground(p))
}

// ---------------- Malzeme ----------------

#[derive(Asset, AsBindGroup, Clone, TypePath)]
pub struct TerrainExt {
    #[texture(100)]
    #[sampler(101)]
    pub splat: Handle<Image>,
    #[texture(102)]
    #[sampler(103)]
    pub grass: Handle<Image>,
    #[texture(104)]
    #[sampler(105)]
    pub road: Handle<Image>,
    #[texture(106)]
    #[sampler(107)]
    pub rock: Handle<Image>,
    #[texture(108)]
    #[sampler(109)]
    pub sand: Handle<Image>,
    #[texture(110)]
    #[sampler(111)]
    pub bed: Handle<Image>,
    #[texture(112)]
    #[sampler(113)]
    pub flower: Handle<Image>,
}

impl MaterialExtension for TerrainExt {
    fn fragment_shader() -> ShaderRef {
        "embedded://azeroth_metin/game/terrain.wgsl".into()
    }
}

pub type TerrainMaterial = ExtendedMaterial<StandardMaterial, TerrainExt>;

#[derive(Component)]
pub struct TerrainChunk;

pub fn plugin(app: &mut App) {
    if !enabled() {
        return;
    }
    bevy::asset::embedded_asset!(app, "terrain.wgsl");
    let _ = DATA.set(TerrainData::load());
    app.add_plugins(MaterialPlugin::<TerrainMaterial>::default())
        .add_systems(Startup, build.after(benilla_assets::AssetSet::Open));
}

fn chunk_mesh(d: &TerrainData, x0: f32, y0: f32, size: f32) -> Mesh {
    let n = (size / CELL) as usize;
    let mut pos = Vec::with_capacity((n + 1) * (n + 1));
    let mut nor = Vec::with_capacity((n + 1) * (n + 1));
    let mut uv = Vec::with_capacity((n + 1) * (n + 1));
    for j in 0..=n {
        for i in 0..=n {
            let mx = x0 + i as f32 * CELL;
            let my = y0 + j as f32 * CELL;
            let h = d.raw_height(mx, my);
            pos.push([mx, h, my]);
            let hx = d.raw_height(mx + 1.0, my) - d.raw_height(mx - 1.0, my);
            let hz = d.raw_height(mx, my + 1.0) - d.raw_height(mx, my - 1.0);
            nor.push(Vec3::new(-hx, 2.0, -hz).normalize().to_array());
            uv.push([(mx / MAP_W).clamp(0.0, 1.0), (my / MAP_H).clamp(0.0, 1.0)]);
        }
    }
    let mut idx = Vec::with_capacity(n * n * 6);
    let w = (n + 1) as u32;
    for j in 0..n as u32 {
        for i in 0..n as u32 {
            let a = j * w + i;
            let b = a + 1;
            let c = a + w;
            let e = c + 1;
            idx.extend_from_slice(&[a, c, b, b, c, e]);
        }
    }
    let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, nor);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    m.insert_indices(Indices::U32(idx));
    m
}

fn build(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut tmats: ResMut<Assets<TerrainMaterial>>,
    mut smats: ResMut<Assets<StandardMaterial>>,
    mut wa: Option<ResMut<WorldAssets>>,
    server: Res<AssetServer>,
) {
    let (Some(d), Some(wa)) = (data(), wa.as_deref_mut()) else {
        error!("terrain: veri ya da WorldAssets yok");
        return;
    };
    // Splat (doğrusal, kenarda kenetli).
    let sp = decode_png(SPLAT).to_rgba8();
    let mut splat = Image::new(
        Extent3d {
            width: sp.width(),
            height: sp.height(),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        sp.into_raw(),
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    splat.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        ..ImageSamplerDescriptor::linear()
    });
    let splat = images.add(splat);
    let mut tex = |p: &str| wa.texture(p, (true, true), &mut images).unwrap_or_default();
    let mat = tmats.add(ExtendedMaterial {
        base: StandardMaterial {
            perceptual_roughness: 0.92,
            reflectance: 0.02,
            ..default()
        },
        extension: TerrainExt {
            splat,
            grass: tex("Tileset\\Elwynn\\ElwynnGrassBase_s.blp"),
            road: tex("Tileset\\Westwood\\WestfallDirtBase_s.blp"),
            rock: tex("Tileset\\AlteracMtns\\AlteracRockBase_s.blp"),
            sand: tex("Tileset\\Westwood\\WestfallSandBase_s.blp"),
            bed: tex("Tileset\\Generic\\UnderwaterRock01_s.blp"),
            flower: tex("Tileset\\Elwynn\\ElwynnFlowerBase_s.blp"),
        },
    });
    // Parça parça arazi (64 m).
    let size = 64.0;
    let mut y = -MARGIN;
    let mut count = 0;
    while y < MAP_H + MARGIN {
        let mut x = -MARGIN;
        while x < MAP_W + 64.0 {
            let mesh = meshes.add(chunk_mesh(d, x, y, size));
            commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(mat.clone()),
                Transform::from_translation(Vec3::new(d.origin.x, 0.0, d.origin.z)),
                TerrainChunk,
            ));
            count += 1;
            x += size;
        }
        y += size;
    }
    // Su: deniz + nehir tek düzlem.
    let water = smats.add(StandardMaterial {
        base_color: Color::srgba(0.07, 0.2, 0.26, 0.86),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.06,
        reflectance: 0.25,
        metallic: 0.0,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(MAP_W + 2400.0, MAP_H + 2400.0))),
        MeshMaterial3d(water),
        Transform::from_translation(Vec3::new(
            d.origin.x + MAP_W / 2.0 + 600.0,
            WATER_LEVEL,
            d.origin.z + MAP_H / 2.0,
        )),
    ));
    // Köprüler: taş döşeme.
    let deck_mat = smats.add(StandardMaterial {
        base_color_texture: Some(tex("Tileset\\Elwynn\\ElwynnCobblestoneBase_s.blp")),
        perceptual_roughness: 0.85,
        ..default()
    });
    let rail_mat = smats.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.42, 0.38),
        perceptual_roughness: 0.9,
        ..default()
    });
    for b in &d.bridges {
        let ab = b.b - b.a;
        let len = ab.length();
        let mid = (b.a + b.b) / 2.0;
        let yaw = -ab.y.atan2(ab.x);
        let at = Vec3::new(d.origin.x + mid.x, b.deck - 0.25, d.origin.z + mid.y);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(len, 0.5, b.half_w * 2.0))),
            MeshMaterial3d(deck_mat.clone()),
            Transform::from_translation(at).with_rotation(Quat::from_rotation_y(yaw)),
        ));
        for side in [-1.0f32, 1.0] {
            let off = Quat::from_rotation_y(yaw) * Vec3::new(0.0, 0.6, side * (b.half_w - 0.2));
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(len, 0.7, 0.3))),
                MeshMaterial3d(rail_mat.clone()),
                Transform::from_translation(at + off).with_rotation(Quat::from_rotation_y(yaw)),
            ));
        }
    }
    // Ağaçlar.
    #[derive(serde::Deserialize)]
    struct Trees {
        trees: Vec<(f32, f32, u8)>,
    }
    let trees: Trees = serde_json::from_str(TREES).expect("trees.json");
    let leafy = [
        "World/Azeroth/Elwynn/PassiveDoodads/Trees/ElwynnTreeCanopy01.m2",
        "World/Azeroth/Elwynn/PassiveDoodads/Trees/ElwynnTreeCanopy02.m2",
        "World/Azeroth/Elwynn/PassiveDoodads/Trees/ElwynnTreeCanopy03.m2",
        "World/Azeroth/Elwynn/PassiveDoodads/Trees/ElwynnTreeMid01.m2",
    ];
    let pines = [
        "World/Azeroth/Elwynn/PassiveDoodads/Trees/ElwynnTree01/ElwynnPine01.m2",
        "World/Azeroth/Elwynn/PassiveDoodads/Trees/ElwynnTree01/ElwynnPine02.m2",
        "World/Azeroth/Elwynn/PassiveDoodads/Trees/ElwynnFirTree01.m2",
    ];
    let leafy_h: Vec<_> = leafy.iter().map(|p| load_m2(&server, p)).collect();
    let pine_h: Vec<_> = pines.iter().map(|p| load_m2(&server, p)).collect();
    for (i, (x, y, kind)) in trees.trees.iter().enumerate() {
        let r = ((i as u32).wrapping_mul(2654435761) >> 8) as f32 / 16_777_216.0;
        let (h, scale, tint) = match kind {
            1 => (pine_h[i % pine_h.len()].clone(), 0.55 + r * 0.3, Color::WHITE),
            2 => (leafy_h[i % leafy_h.len()].clone(), 0.32 + r * 0.12, Color::srgb(1.35, 0.75, 0.45)),
            _ => (leafy_h[i % leafy_h.len()].clone(), 0.3 + r * 0.14, Color::srgb(0.75, 0.95, 0.65)),
        };
        let mut wm = WowModel::new(h);
        wm.shared = true;
        wm.tint = tint;
        let at = Vec3::new(d.origin.x + x, d.raw_height(*x, *y) - 0.2, d.origin.z + y);
        commands.spawn((
            wm,
            Transform::from_translation(at)
                .with_rotation(Quat::from_rotation_y(r * std::f32::consts::TAU))
                .with_scale(Vec3::splat(scale)),
            Visibility::default(),
        ));
    }
    // Köy binaları (WMO).
    for (x, y, yaw, path, scale, _, _) in BUILDINGS {
        let at = Vec3::new(d.origin.x + x, d.ground_map(*x, *y) - 0.15, d.origin.z + y);
        commands.spawn((
            WowWmo {
                handle: load_wmo(&server, path),
            },
            Transform::from_translation(at)
                .with_rotation(Quat::from_rotation_y(yaw.to_radians()))
                .with_scale(Vec3::splat(*scale)),
            Visibility::default(),
        ));
    }
    // Garnizon duvarı: taş çit parçaları.
    let fence = load_m2(&server, "World/Azeroth/Elwynn/PassiveDoodads/ElwynnFences/ElwynnStoneFence.m2");
    let (x0, y0, x1, y1) = COMPOUND;
    let mut seg = |ax: f32, ay: f32, bx: f32, by: f32| {
        let a = Vec2::new(ax, ay);
        let b = Vec2::new(bx, by);
        let len = a.distance(b);
        let n = (len / 4.0).ceil() as i32;
        let dir = (b - a) / len;
        let yaw = -dir.y.atan2(dir.x);
        for k in 0..n {
            let p = a + dir * (k as f32 + 0.5) * (len / n as f32);
            // Kapı boşlukları.
            if ((p.y - 653.0).abs() < 5.0 && (dir.x.abs() < 0.1)) || ((p.x - 283.0).abs() < 5.0 && dir.y.abs() < 0.1) {
                continue;
            }
            let mut wm = WowModel::new(fence.clone());
            wm.shared = true;
            commands.spawn((
                wm,
                Transform::from_translation(Vec3::new(d.origin.x + p.x, d.ground_map(p.x, p.y), d.origin.z + p.y))
                    .with_rotation(Quat::from_rotation_y(yaw))
                    .with_scale(Vec3::splat(0.8)),
                Visibility::default(),
            ));
        }
    };
    seg(x0, y0, x1, y0);
    seg(x1, y0, x1, y1);
    seg(x1, y1, x0, y1);
    seg(x0, y1, x0, y0);
    info!("terrain: Pyungmoo {count} parça, {} ağaç, {} bina", trees.trees.len(), BUILDINGS.len());
}
