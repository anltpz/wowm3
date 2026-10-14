//! WoW M2 modellerini oyun birimi olarak sahneye koyma: benilla-assets'in yüklediği `M2Model`'den
//! Bevy'nin standart `SkinnedMesh` + `AnimationPlayer` yoluyla iskeletli, animasyonlu örnek kurar.
//! (benilla'nın kendi GPU palet yolu istemciye sıkı bağlı; birimlerimiz için standart yol yeterli.)

use std::collections::HashMap;
use std::time::Duration;

use bevy::animation::transition::AnimationTransitions;
use bevy::animation::AnimatedBy;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;

use benilla_assets::coords::wow_to_bevy;
use benilla_assets::{bone_target_id, M2Model, ModelAnimations, ModelAttachment};
use benilla_formats::{CharSkinSlot, ModelBlend};

/// WoW AnimationData.dbc kimlikleri (kullandıklarımız).
pub mod anim {
    pub const STAND: u16 = 0;
    pub const DEATH: u16 = 1;
    pub const WALK: u16 = 4;
    pub const RUN: u16 = 5;
    pub const DEAD: u16 = 6;
    pub const COMBAT_WOUND: u16 = 9;
    pub const ATTACK_UNARMED: u16 = 16;
    pub const ATTACK_1H: u16 = 17;
    pub const ATTACK_2H: u16 = 18;
    pub const READY_1H: u16 = 26;
    pub const READY_2H: u16 = 27;
    pub const SPELL_CAST: u16 = 32;
    pub const SPELL_CAST_AREA: u16 = 33;
    pub const SPELL_OMNI: u16 = 52;
    pub const BATTLE_ROAR: u16 = 55;
    /// Binek üstünde oturma.
    pub const MOUNT: u16 = 91;
}

/// Bir M2 örneği: model + doku seçimi. Yüklenince `ModelReady` eklenir.
#[derive(Component, Clone)]
pub struct WowModel {
    pub handle: Handle<M2Model>,
    /// Yaratık deri yuvaları (Monster1/2/3).
    pub skins: [Option<Handle<Image>>; 3],
    /// Karakter gövde atlası (CharSkinSlot::Body).
    pub body: Option<Handle<Image>>,
    /// Saç dokusu (CharSkinSlot::Hair).
    pub hair: Option<Handle<Image>>,
    /// Silah/kalkan dokusu (CharSkinSlot::Object, tip 2).
    pub object: Option<Handle<Image>>,
    /// Görünür geoset kümesi; `None` = hepsi.
    pub geosets: Option<Vec<u16>>,
    /// Malzeme çarpanı (ör. elit mob kırmızımsı).
    pub tint: Color,
    /// Kendinden ışıklı ekstra parlama (artı basılmış silah parıltısı).
    pub emissive: LinearRgba,
    /// Malzemeleri diğer örneklerle paylaş (ağaç/bina gibi değişmeyen modeller).
    pub shared: bool,
}

impl WowModel {
    pub fn new(handle: Handle<M2Model>) -> Self {
        Self {
            handle,
            skins: [None, None, None],
            body: None,
            hair: None,
            object: None,
            geosets: None,
            tint: Color::WHITE,
            emissive: LinearRgba::BLACK,
            shared: false,
        }
    }
}

/// Statik WMO (bina) örneği.
#[derive(Component, Clone)]
pub struct WowWmo {
    pub handle: Handle<benilla_assets::WmoModel>,
}

#[derive(Component)]
pub struct WmoReady;

/// Paylaşılan malzeme anahtarı.
#[derive(Hash, PartialEq, Eq, Clone)]
struct MatKey {
    tex: Option<AssetId<Image>>,
    alpha: u8,
    two_sided: bool,
    unlit: bool,
    color: [u32; 4],
    emissive: [u32; 3],
}

#[derive(Resource, Default)]
struct MatCache(HashMap<MatKey, Handle<StandardMaterial>>);

/// Kurulmuş örnek: eklem varlıkları, animasyonlar, takma noktaları.
#[derive(Component)]
pub struct ModelReady {
    pub joints: Vec<Entity>,
    pub anims: Option<ModelAnimations>,
    pub attachments: Vec<ModelAttachment>,
    pub parts: Vec<Entity>,
    pub materials: Vec<Handle<StandardMaterial>>,
    /// Modelin bağlı kutusu yarıçapı (seçim/isim yüksekliği için).
    pub height: f32,
}

impl ModelReady {
    /// Takma noktası (1 = sağ el, 2 = sol el, 0 = kalkan ...) için eklem + ofset.
    pub fn attachment(&self, id: u16) -> Option<(Entity, Vec3)> {
        let a = self.attachments.iter().find(|a| a.id == id)?;
        Some((*self.joints.get(a.bone as usize)?, a.offset))
    }
}

/// İstenen animasyon. `once` = bir kez oynat, bitince `idle`'a dön.
#[derive(Component, Clone)]
pub struct AnimState {
    pub want: u16,
    pub idle: u16,
    pub once: bool,
    pub speed: f32,
    /// Bir kerelik isteği tekrar tetiklemek için sayaç (aynı saldırı üst üste).
    pub serial: u32,
    playing: Option<(u16, u32)>,
    node: Option<AnimationNodeIndex>,
}

impl Default for AnimState {
    fn default() -> Self {
        Self {
            want: anim::STAND,
            idle: anim::STAND,
            once: false,
            speed: 1.0,
            serial: 0,
            playing: None,
            node: None,
        }
    }
}

impl AnimState {
    pub fn set_loop(&mut self, id: u16) {
        if !self.once {
            self.want = id;
        }
        self.idle = id;
    }
    pub fn play_once(&mut self, id: u16) {
        self.want = id;
        self.once = true;
        self.serial = self.serial.wrapping_add(1);
    }
    /// Ölüm gibi kalıcı tek seferlik: bitince son karede kalır.
    pub fn hold(&mut self, id: u16) {
        self.want = id;
        self.idle = id;
        self.once = false;
        self.serial = self.serial.wrapping_add(1);
    }
    pub fn busy(&self) -> bool {
        self.once
    }
}

/// Model başına bir kez kurulan mesh'ler (örnekler paylaşır).
#[derive(Resource, Default)]
struct MeshCache(HashMap<AssetId<M2Model>, Vec<Option<Handle<Mesh>>>>);

pub fn plugin(app: &mut App) {
    app.init_resource::<MeshCache>()
        .init_resource::<MatCache>()
        .init_resource::<WmoMeshCache>()
        .add_systems(Update, (realize_models, realize_wmos, drive_anims).chain());
}

pub fn load_m2(server: &AssetServer, path: &str) -> Handle<M2Model> {
    server.load(format!("mpq://{}", path.replace('\\', "/")))
}

fn build_mesh(sub: &benilla_formats::RenderSubmesh) -> Mesh {
    let center = sub
        .billboard
        .as_ref()
        .map_or(Vec3::ZERO, |b| wow_to_bevy(b.pivot));
    let positions: Vec<[f32; 3]> = sub
        .positions
        .iter()
        .map(|p| (wow_to_bevy(*p) - center).to_array())
        .collect();
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, sub.uvs.clone());
    mesh.insert_indices(Indices::U32(sub.indices.clone()));
    if sub.normals.len() == sub.positions.len() {
        let n: Vec<[f32; 3]> = sub.normals.iter().map(|n| wow_to_bevy(*n).to_array()).collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, n);
    } else {
        mesh.compute_normals();
    }
    if sub.joints.len() == sub.positions.len() && !sub.joints.is_empty() {
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            VertexAttributeValues::Uint16x4(sub.joints.clone()),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, sub.weights.clone());
    }
    mesh
}

fn realize_models(
    mut commands: Commands,
    models: Res<Assets<M2Model>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut cache: ResMut<MeshCache>,
    mut mat_cache: ResMut<MatCache>,
    pending: Query<(Entity, &WowModel), Without<ModelReady>>,
) {
    for (root, wm) in &pending {
        let Some(model) = models.get(&wm.handle) else {
            continue;
        };
        let id = wm.handle.id();
        let built = cache.0.entry(id).or_insert_with(|| {
            model
                .submeshes
                .iter()
                .map(|s| {
                    // Zemin dekalı ve boş batch'ler çizilmez.
                    (s.ground_quad.is_none() && !s.geometry.indices.is_empty())
                        .then(|| meshes.add(build_mesh(&s.geometry)))
                })
                .collect()
        });

        // Eklemler: kök varlığın altında hiyerarşi, her biri animasyon hedefi.
        let skel = &model.skeleton;
        let has_rig = !skel.joints.is_empty();
        let joints: Vec<Entity> = skel
            .joints
            .iter()
            .enumerate()
            .map(|(i, j)| {
                commands
                    .spawn((
                        Transform::from_translation(j.local_translation),
                        Visibility::default(),
                        bone_target_id(i as u16),
                        AnimatedBy(root),
                    ))
                    .id()
            })
            .collect();
        for (i, j) in skel.joints.iter().enumerate() {
            let parent = usize::try_from(j.parent)
                .ok()
                .and_then(|p| joints.get(p).copied())
                .unwrap_or(root);
            commands.entity(parent).add_child(joints[i]);
        }

        let mut parts = Vec::new();
        let mut materials = Vec::new();
        for (i, sub) in model.submeshes.iter().enumerate() {
            let Some(Some(mesh)) = built.get(i) else {
                continue;
            };
            if let Some(vis) = &wm.geosets {
                if sub.geoset_id <= 1700 && !vis.contains(&sub.geoset_id) {
                    continue;
                }
            }
            let tex = match (sub.char_slot, sub.skin_slot) {
                (Some(CharSkinSlot::Body), _) => wm.body.clone(),
                (Some(CharSkinSlot::Hair), _) => wm.hair.clone(),
                (Some(CharSkinSlot::Object), _) => wm.object.clone().or(sub.texture.clone()),
                (Some(CharSkinSlot::SkinExtra), _) => None,
                (None, Some(slot)) => wm.skins[slot as usize].clone(),
                (None, None) => sub.texture.clone(),
            };
            // Dokusu olmayan yuva (ör. pelerin) çizilmez.
            if tex.is_none() {
                continue;
            }
            let g = &sub.geometry;
            let alpha_mode = if sub.additive {
                AlphaMode::Add
            } else {
                match sub.blend {
                    ModelBlend::Opaque => AlphaMode::Opaque,
                    ModelBlend::AlphaTest => AlphaMode::Mask(0.5),
                    ModelBlend::Blend => AlphaMode::Blend,
                    ModelBlend::Mod | ModelBlend::Mod2x => AlphaMode::Multiply,
                }
            };
            let tint = g
                .vertex_colors
                .first()
                .map(|c| Color::linear_rgba(c[0], c[1], c[2], c[3]))
                .unwrap_or(Color::WHITE);
            let (a, b) = (wm.tint.to_linear(), tint.to_linear());
            let base = LinearRgba::new(a.red * b.red, a.green * b.green, a.blue * b.blue, a.alpha * b.alpha);
            let make = || StandardMaterial {
                base_color: base.into(),
                base_color_texture: tex.clone(),
                alpha_mode,
                unlit: sub.emissive || sub.additive,
                cull_mode: if sub.two_sided {
                    None
                } else {
                    Some(bevy::render::render_resource::Face::Back)
                },
                double_sided: sub.two_sided,
                perceptual_roughness: 0.9,
                reflectance: 0.05,
                emissive: wm.emissive,
                ..default()
            };
            let mat = if wm.shared {
                let key = mat_key(&tex, alpha_mode, sub.two_sided, sub.emissive || sub.additive, base, wm.emissive);
                mat_cache.0.entry(key).or_insert_with(|| mats.add(make())).clone()
            } else {
                mats.add(make())
            };
            materials.push(mat.clone());
            let mut e = commands.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(mat),
                Transform::default(),
                ChildOf(root),
            ));
            if has_rig && !g.joints.is_empty() {
                e.insert(SkinnedMesh {
                    inverse_bindposes: model.inverse_bindposes.clone(),
                    joints: joints.clone(),
                });
            }
            parts.push(e.id());
        }

        let anims = model.animations.clone();
        if let Some(a) = &anims {
            commands.entity(root).insert((
                AnimationPlayer::default(),
                AnimationTransitions::new(),
                AnimationGraphHandle(a.graph.clone()),
            ));
        }
        let height = model
            .bounds
            .as_ref()
            .map(|b| (b.bbox_max[2] - b.bbox_min[2]).abs().max(0.5))
            .unwrap_or(2.0);
        commands.entity(root).insert(ModelReady {
            joints,
            anims,
            attachments: model.attachments.clone(),
            parts,
            materials,
            height,
        });
    }
}

/// Yedek zinciri: istenen kimlik yoksa makul alternatifler.
fn fallback(id: u16) -> &'static [u16] {
    match id {
        anim::RUN => &[anim::RUN, anim::WALK, anim::STAND],
        anim::WALK => &[anim::WALK, anim::RUN, anim::STAND],
        anim::ATTACK_1H => &[anim::ATTACK_1H, anim::ATTACK_UNARMED, anim::ATTACK_2H],
        anim::ATTACK_2H => &[anim::ATTACK_2H, anim::ATTACK_1H, anim::ATTACK_UNARMED],
        anim::ATTACK_UNARMED => &[anim::ATTACK_UNARMED, anim::ATTACK_1H, anim::ATTACK_2H],
        anim::DEATH => &[anim::DEATH, anim::DEAD],
        anim::READY_1H => &[anim::READY_1H, anim::READY_2H, anim::STAND],
        anim::READY_2H => &[anim::READY_2H, anim::READY_1H, anim::STAND],
        anim::SPELL_CAST_AREA => &[anim::SPELL_CAST_AREA, anim::SPELL_OMNI, anim::SPELL_CAST, anim::BATTLE_ROAR],
        anim::BATTLE_ROAR => &[anim::BATTLE_ROAR, anim::SPELL_OMNI, anim::SPELL_CAST_AREA],
        anim::COMBAT_WOUND => &[anim::COMBAT_WOUND],
        anim::MOUNT => &[anim::MOUNT, anim::STAND],
        _ => &[],
    }
}

fn drive_anims(
    mut q: Query<(
        &ModelReady,
        &mut AnimState,
        &mut AnimationPlayer,
        &mut AnimationTransitions,
    )>,
) {
    for (ready, mut st, mut player, mut tr) in &mut q {
        let Some(anims) = &ready.anims else { continue };
        // Bir kerelik bitti mi?
        if st.once {
            if let Some(node) = st.node {
                let done = player.animation(node).is_none_or(|a| a.is_finished());
                if done {
                    st.once = false;
                    st.want = st.idle;
                }
            }
        }
        let key = (st.want, if st.once || st.want == anim::DEATH { st.serial } else { 0 });
        if st.playing == Some(key) {
            if let Some(node) = st.node {
                if let Some(a) = player.animation_mut(node) {
                    a.set_speed(st.speed);
                }
            }
            continue;
        }
        let ids: Vec<u16> = {
            let f = fallback(st.want);
            if f.is_empty() { vec![st.want] } else { f.to_vec() }
        };
        let Some(clip) = ids.iter().find_map(|&i| anims.find(i)) else {
            st.playing = Some(key);
            // Klip yoksa bir kerelik isteği hemen bitir.
            if st.once {
                st.once = false;
                st.want = st.idle;
            }
            continue;
        };
        let blend = if st.want == anim::DEATH { 80 } else { 150 };
        let active = tr.play(&mut player, clip.node, Duration::from_millis(blend));
        active.set_speed(st.speed);
        let looping = clip.looping && !st.once && st.want != anim::DEATH;
        if looping {
            active.repeat();
        } else {
            active.set_repeat(bevy::animation::RepeatAnimation::Never);
        }
        st.node = Some(clip.node);
        st.playing = Some(key);
    }
}

fn mat_key(
    tex: &Option<Handle<Image>>,
    alpha: AlphaMode,
    two_sided: bool,
    unlit: bool,
    c: LinearRgba,
    e: LinearRgba,
) -> MatKey {
    MatKey {
        tex: tex.as_ref().map(|h| h.id()),
        alpha: match alpha {
            AlphaMode::Opaque => 0,
            AlphaMode::Mask(_) => 1,
            AlphaMode::Blend => 2,
            AlphaMode::Add => 3,
            AlphaMode::Multiply => 4,
            _ => 5,
        },
        two_sided,
        unlit,
        color: [c.red.to_bits(), c.green.to_bits(), c.blue.to_bits(), c.alpha.to_bits()],
        emissive: [e.red.to_bits(), e.green.to_bits(), e.blue.to_bits()],
    }
}

#[derive(Resource, Default)]
struct WmoMeshCache(HashMap<AssetId<benilla_assets::WmoModel>, Vec<Option<Handle<Mesh>>>>);

/// WMO binaları: statik mesh'ler, paylaşılan malzemeler (iç mekân MOCV ışığı köşe rengiyle gelir).
fn realize_wmos(
    mut commands: Commands,
    wmos: Res<Assets<benilla_assets::WmoModel>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut cache: ResMut<WmoMeshCache>,
    mut mat_cache: ResMut<MatCache>,
    pending: Query<(Entity, &WowWmo), Without<WmoReady>>,
) {
    for (root, w) in &pending {
        let Some(model) = wmos.get(&w.handle) else { continue };
        let built = cache.0.entry(w.handle.id()).or_insert_with(|| {
            model
                .submeshes
                .iter()
                .map(|s| (!s.geometry.indices.is_empty()).then(|| meshes.add(build_mesh(&s.geometry))))
                .collect()
        });
        for (i, sub) in model.submeshes.iter().enumerate() {
            let Some(Some(mesh)) = built.get(i) else { continue };
            let Some(tex) = sub.texture.clone() else { continue };
            let alpha_mode = match sub.blend {
                ModelBlend::Opaque => AlphaMode::Opaque,
                ModelBlend::AlphaTest => AlphaMode::Mask(0.5),
                ModelBlend::Blend => AlphaMode::Blend,
                ModelBlend::Mod | ModelBlend::Mod2x => AlphaMode::Multiply,
            };
            let unlit = sub.emissive;
            let base = LinearRgba::WHITE;
            let key = mat_key(&Some(tex.clone()), alpha_mode, sub.two_sided, unlit, base, LinearRgba::BLACK);
            let mat = mat_cache
                .0
                .entry(key)
                .or_insert_with(|| {
                    mats.add(StandardMaterial {
                        base_color: Color::WHITE,
                        base_color_texture: Some(tex.clone()),
                        alpha_mode,
                        unlit,
                        cull_mode: if sub.two_sided { None } else { Some(bevy::render::render_resource::Face::Back) },
                        double_sided: sub.two_sided,
                        perceptual_roughness: 0.9,
                        reflectance: 0.04,
                        ..default()
                    })
                })
                .clone();
            commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(mat), Transform::default(), ChildOf(root)));
        }
        commands.entity(root).insert(WmoReady);
    }
}

pub fn load_wmo(server: &AssetServer, path: &str) -> Handle<benilla_assets::WmoModel> {
    server.load(format!("mpq://{}", path.replace('\\', "/")))
}
