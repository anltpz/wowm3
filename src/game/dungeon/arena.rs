//! Zindan arenası: katın WMO'su (render: `model::WowWmo`), çarpışması (benilla trimesh yolu), yürünebilir
//! noktalar, canavar/metin/boss doğurma, çıkış portalı ve temizlik.
//!
//! Çarpışma: `WmoModel.collision` (yürüme yüzleri, DETAIL hariç) → `placement_collider_data` (render ile aynı
//! `wow_to_bevy` + yerleşim `Transform`'u: eksen hizası garanti) → `build_collider_task` → `PendingCollider`
//! (benilla `finish_colliders` eklerken `RigidBody::Static`). Katman verilmez (Default): `WorldCollision`'ın
//! gövde (`ray_body`/`cast_body`) ve kamera (`cast_camera`) sorgularının ikisi de görür.

use bevy::asset::LoadState;
use bevy::prelude::*;

use avian3d::prelude::Collider;
use benilla_assets::coords::{bevy_to_wow, wow_to_bevy};
use benilla_assets::{WmoModel, WorldAssets};
use benilla_world::collision::WorldCollision;
use benilla_world::terrain_stream::{build_collider_task, placement_collider_data, PendingCollider};

use super::defs::{FloorDef, Objective};
use super::{Action, DungeonEntity, DungeonEvent, DungeonMetin, DungeonMob, DungeonRun, MobRole};
use crate::game::character::CharDb;
use crate::game::inventory::Inventory;
use crate::game::log::{GameLog, C_BAD, C_GOOD, C_INFO, C_ITEM};
use crate::game::model::{load_m2, load_wmo, AnimState, WowModel, WowWmo};
use crate::game::player::Player;

/// Yürünebilir yüzey: normalin dikey bileşeni en az bu.
pub const FLOOR_NORMAL_Y: f32 = 0.75;
/// İçeride sayılmak için tavanın en fazla bu kadar yukarıda olması gerekir (m).
pub const CEILING_MAX: f32 = 12.0;
/// Tavan en az bu kadar yukarıda (alçak geçit/kiriş altı değil).
pub const HEADROOM: f32 = 2.0;
/// Kat başına örneklenen yürünebilir nokta sayısı.
pub const WALKABLE_SAMPLES: usize = 320;
/// Kat yükleme zaman aşımı (sn): WMO/çarpışma bu sürede hazır olmazsa zindandan çıkılır.
const LOAD_TIMEOUT: f32 = 45.0;
/// Çıkış portalı modeli (D-23 değiştirebilir).
pub const PORTAL_M2: &str = "World/Generic/ActiveDoodads/InstancePortal/InstancePortal.m2";

/// Arena sistemlerinin kümesi (giriş sistemlerinden sonra, durum makinesinden önce).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArenaSet;

/// Açık katın arenası.
#[derive(Resource, Default)]
pub struct Arena {
    pub floor: usize,
    /// WMO kök varlığı.
    pub root: Option<Entity>,
    handle: Option<Handle<WmoModel>>,
    /// WMO'nun yerleşimi (render ve çarpışma aynı dönüşümü kullanır).
    pub transform: Transform,
    /// Çarpışma varlığı (avian `Collider` eklenince hazır).
    pub collider: Option<Entity>,
    /// Yürünebilir ve içeride (tavanlı) noktalar, dünya koordinatı.
    pub walkable: Vec<Vec3>,
    /// `walkable`'ın girişten düz çizgiyle görünenleri (yol bulma yok: önce bunlara doğurulur).
    pub visible: Vec<Vec3>,
    /// Kat başı (giriş ve ölümden sonra doğma noktası).
    pub entry: Option<Vec3>,
    /// Yürünebilir noktaların en alçağı (düşme koruması).
    pub min_y: f32,
    /// Kat kurulurken oyuncunun asılı tutulduğu nokta.
    pub hold: Option<Vec3>,
    /// Çarpışma eklendi, giriş noktasında zemin bulundu.
    pub ready: bool,
    /// Son zeminli oyuncu konumu (boşluğa adım engeli).
    pub last_good: Option<Vec3>,
    load_t: f32,
    ready_wait: f32,
}

/// Zafer sonrası çıkış portalı: içine yürüyünce zindandan çıkılır.
#[derive(Component)]
pub struct DungeonPortal;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Arena>()
        .configure_sets(Update, ArenaSet.in_set(super::DungeonSet))
        .add_systems(
            Update,
            (apply_actions, build_collider, floor_ready, tag_strays, portal_touch).chain().in_set(ArenaSet),
        );
}

/// Katın WMO yerleşimi: WoW ofseti + dikey eksen dönüşü.
pub fn floor_transform(f: &FloorDef) -> Transform {
    Transform::from_translation(wow_to_bevy(f.offset)).with_rotation(Quat::from_rotation_y(f.yaw.to_radians()))
}

// ---------------------------------------------------------------------------------------------------
// Yürünebilir noktalar (saf; birim testli)
// ---------------------------------------------------------------------------------------------------

/// Basit belirlenimci rastgele üreteç (testler tekrarlanabilir olsun).
pub struct Lcg(pub u64);

impl Lcg {
    pub fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as u32
    }
    /// [0, 1)
    pub fn unit(&mut self) -> f32 {
        self.next_u32() as f32 / (1u64 << 31) as f32
    }
}

/// Üçgen normali (avian/parry ile aynı sarım: `(b-a)×(c-a)`), yüzölçümünün iki katı uzunlukta.
fn tri_cross(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    (b - a).cross(c - a)
}

/// Möller–Trumbore, iki yüzlü: ışın `o + t·d` (t > 0) üçgeni kesiyorsa t.
fn ray_tri(o: Vec3, d: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let e1 = b - a;
    let e2 = c - a;
    let p = d.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-8 {
        return None;
    }
    let inv = 1.0 / det;
    let s = o - a;
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = d.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t > 1e-4).then_some(t)
}

/// Yürünebilir ve "içeride" noktalar (dünya koordinatı): yukarı bakan (normal.y > 0.75) üçgenlerden
/// alana göre örneklenir; `need_ceiling` ise noktanın 2–12 m üstünde aşağı bakan bir tavan olmalı (mağara
/// ağzı, açık avlu ve dış çatılar elenir). En fazla `max` nokta.
pub fn walkable_points(verts: &[Vec3], tris: &[[u32; 3]], max: usize, seed: u64, need_ceiling: bool) -> Vec<Vec3> {
    let v = |i: u32| verts.get(i as usize).copied();
    let mut floors: Vec<([Vec3; 3], f32)> = Vec::new();
    // Tavan adayları + xz kutusu (hızlı eleme).
    let mut ceilings: Vec<([Vec3; 3], Vec2, Vec2)> = Vec::new();
    let mut total = 0.0f32;
    for t in tris {
        let (Some(a), Some(b), Some(c)) = (v(t[0]), v(t[1]), v(t[2])) else { continue };
        let n = tri_cross(a, b, c);
        let len = n.length();
        if len < 1e-6 {
            continue;
        }
        let ny = n.y / len;
        if ny > FLOOR_NORMAL_Y {
            total += len * 0.5;
            floors.push(([a, b, c], total));
        } else if ny < -0.2 {
            let lo = a.xz().min(b.xz()).min(c.xz());
            let hi = a.xz().max(b.xz()).max(c.xz());
            ceilings.push(([a, b, c], lo, hi));
        }
    }
    let mut out = Vec::new();
    if floors.is_empty() || total <= 0.0 {
        return out;
    }
    let mut rng = Lcg(seed ^ 0x9E37_79B9_7F4A_7C15);
    for _ in 0..max * 8 {
        if out.len() >= max {
            break;
        }
        let pick = rng.unit() * total;
        let i = floors.partition_point(|(_, cum)| *cum < pick).min(floors.len() - 1);
        let [a, b, c] = floors[i].0;
        let (mut r1, mut r2) = (rng.unit(), rng.unit());
        if r1 + r2 > 1.0 {
            r1 = 1.0 - r1;
            r2 = 1.0 - r2;
        }
        let p = a + (b - a) * r1 + (c - a) * r2;
        if need_ceiling {
            let o = p + Vec3::Y * 0.05;
            let mut best: Option<f32> = None;
            for (t, lo, hi) in &ceilings {
                if o.x < lo.x || o.x > hi.x || o.z < lo.y || o.z > hi.y {
                    continue;
                }
                if let Some(d) = ray_tri(o, Vec3::Y, t[0], t[1], t[2]) {
                    if best.is_none_or(|b| d < b) {
                        best = Some(d);
                    }
                }
            }
            match best {
                Some(d) if (HEADROOM..=CEILING_MAX).contains(&d) => {}
                _ => continue,
            }
        }
        out.push(p);
    }
    out
}

/// `a`→`b` doğru parçası hiçbir üçgeni kesmiyor mu (iki yüzlü; uçlardaki 0.3 m sayılmaz).
pub fn line_clear(verts: &[Vec3], tris: &[[u32; 3]], a: Vec3, b: Vec3) -> bool {
    let d = b - a;
    let len = d.length();
    if len < 1e-3 {
        return true;
    }
    let dir = d / len;
    let (lo, hi) = (a.min(b) - Vec3::splat(0.01), a.max(b) + Vec3::splat(0.01));
    for t in tris {
        let (Some(&p0), Some(&p1), Some(&p2)) = (verts.get(t[0] as usize), verts.get(t[1] as usize), verts.get(t[2] as usize)) else {
            continue;
        };
        let (tlo, thi) = (p0.min(p1).min(p2), p0.max(p1).max(p2));
        if thi.x < lo.x || tlo.x > hi.x || thi.y < lo.y || tlo.y > hi.y || thi.z < lo.z || tlo.z > hi.z {
            continue;
        }
        if ray_tri(a, dir, p0, p1, p2).is_some_and(|t| t > 0.3 && t < len - 0.3) {
            return false;
        }
    }
    true
}

/// Girişten (göz hizası) düz çizgiyle görülen noktalar (zemin + 1 m).
pub fn visible_from(verts: &[Vec3], tris: &[[u32; 3]], points: &[Vec3], from: Vec3) -> Vec<Vec3> {
    let eye = from + Vec3::Y * 1.4;
    points.iter().copied().filter(|p| line_clear(verts, tris, eye, *p + Vec3::Y * 1.0)).collect()
}

/// Otomatik giriş noktası: yürünebilir noktaların ağırlık merkezine en yakın nokta.
pub fn pick_entry(points: &[Vec3]) -> Option<Vec3> {
    if points.is_empty() {
        return None;
    }
    let c = points.iter().copied().sum::<Vec3>() / points.len() as f32;
    points.iter().copied().min_by(|a, b| a.distance_squared(c).total_cmp(&b.distance_squared(c)))
}

/// `from`'dan en uzak nokta (boss yeri).
pub fn farthest(points: &[Vec3], from: Vec3) -> Option<Vec3> {
    points.iter().copied().max_by(|a, b| a.distance_squared(from).total_cmp(&b.distance_squared(from)))
}

/// `n` doğma noktası: `from`'a en az `min_from`, birbirine en az `min_sep` uzak; sığmazsa şartlar gevşer,
/// nokta azsa döngüyle tekrar eder. Nokta yoksa `from` tekrarlanır.
pub fn spread_points(points: &[Vec3], from: Vec3, n: usize, min_from: f32, min_sep: f32, seed: u64) -> Vec<Vec3> {
    if n == 0 {
        return Vec::new();
    }
    if points.is_empty() {
        return vec![from; n];
    }
    let mut order: Vec<usize> = (0..points.len()).collect();
    let mut rng = Lcg(seed);
    for i in (1..order.len()).rev() {
        let j = rng.next_u32() as usize % (i + 1);
        order.swap(i, j);
    }
    let mut out: Vec<Vec3> = Vec::with_capacity(n);
    let mut used = vec![false; points.len()];
    for (lim, sep) in [(min_from, min_sep), (min_from * 0.5, min_sep * 0.5), (0.0, 0.0)] {
        for &i in &order {
            if out.len() >= n {
                break;
            }
            let p = points[i];
            if used[i] || p.distance(from) < lim || out.iter().any(|q| q.distance(p) < sep) {
                continue;
            }
            used[i] = true;
            out.push(p);
        }
    }
    let mut k = 0;
    while out.len() < n {
        out.push(points[order[k % order.len()]]);
        k += 1;
    }
    out
}

// ---------------------------------------------------------------------------------------------------
// ECS
// ---------------------------------------------------------------------------------------------------

fn pin(xf: &mut Transform, p: &mut Player, at: Vec3) {
    xf.translation = at;
    p.airborne = false;
    p.vy = 0.0;
    p.grounded = true;
}

/// Durum makinesinin işleri: kat kur, hedefi doğur, kat/zafer/başarısızlık duyurusu, çıkış.
fn apply_actions(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<DungeonRun>,
    mut arena: ResMut<Arena>,
    (server, mut wa, mut images, db): (Res<AssetServer>, Option<ResMut<WorldAssets>>, ResMut<Assets<Image>>, Option<Res<CharDb>>),
    ents: Query<Entity, With<DungeonEntity>>,
    mut player: Query<(&mut Transform, &mut Player)>,
    (mut inv, mut log, mut events): (ResMut<Inventory>, ResMut<GameLog>, MessageWriter<DungeonEvent>),
) {
    if run.pending.is_empty() {
        return;
    }
    let actions = std::mem::take(&mut run.pending);
    let now = time.elapsed_secs();
    let Ok((mut pxf, mut pl)) = player.single_mut() else { return };
    for a in actions {
        match a {
            Action::LoadFloor(i) => {
                let Some(f) = run.run.as_ref().and_then(|st| st.def.floor(i)) else { continue };
                for e in &ents {
                    commands.entity(e).despawn();
                }
                let xf = floor_transform(f);
                let handle = load_wmo(&server, f.wmo);
                let root = commands
                    .spawn((WowWmo { handle: handle.clone() }, xf, Visibility::default(), DungeonEntity))
                    .id();
                let hold = xf.translation + Vec3::Y * 1.0;
                *arena = Arena { floor: i, root: Some(root), handle: Some(handle), transform: xf, hold: Some(hold), ..default() };
                pin(&mut pxf, &mut pl, hold);
                info!("dungeon: kat {} yükleniyor: {} @ WoW {:?}", i + 1, f.wmo, f.offset);
            }
            Action::StartObjective(i) => {
                let Some(st) = run.run.as_ref() else { continue };
                let Some(f) = st.def.floor(i) else { continue };
                let Some(wa) = wa.as_deref_mut() else {
                    warn!("dungeon: WorldAssets yok, kat doğurulamadı");
                    continue;
                };
                // QA: `AZM_DUNGEON_NOMOBS=1` boş kat (yürüme/çarpışma testi; hedef bitmez).
                if std::env::var_os("AZM_DUNGEON_NOMOBS").is_some() {
                    info!("dungeon: AZM_DUNGEON_NOMOBS, kat {} boş", i + 1);
                    continue;
                }
                let seed = (now * 1000.0) as u64 ^ (i as u64) << 20;
                populate(&mut commands, &server, wa, &mut images, db.as_deref(), &arena, f, i, seed);
                log.banner(now, C_INFO, f.objective.describe());
            }
            Action::FloorCleared(i) => {
                if let Some(f) = run.run.as_ref().and_then(|st| st.def.floor(i)) {
                    if let Objective::FindKey { key_vnum, .. } = f.objective {
                        if key_vnum != 0 && inv.consume(key_vnum, 1) {
                            log.push(now, C_ITEM, "Anahtar mührü açtı.");
                        }
                    }
                }
                log.banner(now, C_GOOD, "Kat temizlendi!");
                log.push(now, C_GOOD, format!("{}. kat temizlendi. Sonraki kata geçiliyor...", i + 1));
            }
            Action::Victory => {
                let name = run.run.as_ref().map_or("Zindan", |st| st.def.name);
                log.banner(now, C_GOOD, format!("{name} tamamlandı!"));
                log.push(now, C_GOOD, "Çıkış portalı açıldı (60 sn sonra otomatik çıkış).");
                let at = portal_spot(&arena, pxf.translation);
                commands.spawn((
                    WowModel::new(load_m2(&server, PORTAL_M2)),
                    AnimState::default(),
                    Transform::from_translation(at).with_scale(Vec3::splat(0.6)),
                    Visibility::default(),
                    DungeonPortal,
                    DungeonEntity,
                ));
            }
            Action::Failed => {
                log.banner(now, C_BAD, "Süre doldu!");
                log.push(now, C_BAD, "Zindan başarısız. Birazdan dışarı çıkarılacaksın.");
            }
            Action::Leave => {
                for e in &ents {
                    commands.entity(e).despawn();
                }
                *arena = Arena::default();
                crate::game::physics::set_indoor(false);
                let Some(st) = run.run.take() else { continue };
                let pos = wow_to_bevy(st.return_wow);
                pin(&mut pxf, &mut pl, pos);
                run.exit_hold = Some(super::ExitHold { pos, t: 0.0 });
                log.push(now, C_INFO, format!("{} terk edildi.", st.def.name));
                events.write(DungeonEvent::Left { id: st.def.id });
                info!("dungeon: çıkış {} → WoW {:?}", st.def.name, st.return_wow);
                // Kalan işler eski koşuya ait.
                break;
            }
        }
    }
}

/// Portal: oyuncuya en yakın, en az 3 m ötedeki yürünebilir nokta (yoksa giriş).
fn portal_spot(arena: &Arena, player: Vec3) -> Vec3 {
    arena
        .walkable
        .iter()
        .copied()
        .filter(|p| p.distance(player) >= 3.0)
        .min_by(|a, b| a.distance_squared(player).total_cmp(&b.distance_squared(player)))
        .or(arena.entry)
        .unwrap_or(player)
}

/// Katın canavarlarını, metin taşlarını ve boss'unu yürünebilir noktalara doğurur.
#[allow(clippy::too_many_arguments)]
fn populate(
    commands: &mut Commands,
    server: &AssetServer,
    wa: &mut WorldAssets,
    images: &mut Assets<Image>,
    db: Option<&CharDb>,
    arena: &Arena,
    f: &FloorDef,
    floor: usize,
    seed: u64,
) {
    use crate::game::mobs::{mob, spawn_mob};
    // Yol bulma yok: girişten görünen noktalar yeterliyse onlar (canavar ve otopilot düz yürür).
    let pts = if arena.visible.len() >= 12 { &arena.visible } else { &arena.walkable };
    let entry = arena.entry.unwrap_or(arena.transform.translation);
    let n: usize = f.mobs.iter().map(|(_, c)| *c as usize).sum();
    let spots = spread_points(pts, entry, n, 10.0, 2.5, seed);
    let carriers: &[u32] = match f.objective {
        Objective::FindKey { carriers, .. } => carriers,
        _ => &[],
    };
    let mut k = 0;
    for &(id, count) in f.mobs {
        for _ in 0..count {
            let at = spots[k];
            k += 1;
            let role = if carriers.contains(&id) { MobRole::Carrier } else { MobRole::Normal };
            let e = spawn_mob(commands, server, wa, images, db, mob(id), at, None, false);
            commands.entity(e).insert((DungeonEntity, DungeonMob { floor, role }));
        }
    }
    match f.objective {
        Objective::BreakMetin { metin, count, summons } => {
            let Some(def) = crate::game::metin::METINS.get(metin) else { return };
            let spots = spread_points(pts, entry, count as usize, 14.0, 10.0, seed.wrapping_add(1));
            let summons = (!summons.is_empty()).then_some(summons);
            for (j, at) in spots.into_iter().enumerate() {
                let e = crate::game::metin::spawn_metin(commands, server, def, at, j as f32 * 1.7, summons);
                commands.entity(e).insert((DungeonEntity, DungeonMetin { floor }));
            }
        }
        Objective::Boss { id, adds } => {
            let at = farthest(pts, entry).unwrap_or(entry);
            let e = spawn_mob(commands, server, wa, images, db, mob(id), at, None, false);
            commands.entity(e).insert((DungeonEntity, DungeonMob { floor, role: MobRole::Boss }));
            info!("dungeon: boss {} @ {at}", mob(id).name);
            let near: Vec<Vec3> = pts.iter().copied().filter(|p| p.distance(at) < 12.0).collect();
            let total: usize = adds.iter().map(|(_, c)| *c as usize).sum();
            let spots = spread_points(&near, at, total, 2.5, 1.5, seed.wrapping_add(2));
            let mut k = 0;
            for &(aid, count) in adds {
                for _ in 0..count {
                    let e = spawn_mob(commands, server, wa, images, db, mob(aid), spots[k], None, false);
                    k += 1;
                    commands.entity(e).insert((DungeonEntity, DungeonMob { floor, role: MobRole::Add }));
                }
            }
        }
        Objective::KillAll | Objective::FindKey { .. } => {}
    }
    info!("dungeon: kat {} doğdu: {n} canavar, hedef {:?}", floor + 1, f.objective);
}

/// WMO yüklenince çarpışmayı kurar ve yürünebilir noktaları hesaplar.
fn build_collider(
    mut commands: Commands,
    time: Res<Time>,
    server: Res<AssetServer>,
    wmos: Res<Assets<WmoModel>>,
    mut arena: ResMut<Arena>,
    mut run: ResMut<DungeonRun>,
    mut log: ResMut<GameLog>,
) {
    if arena.root.is_none() || arena.collider.is_some() || arena.ready {
        return;
    }
    let Some(handle) = arena.handle.clone() else { return };
    let Some(st) = run.run.as_ref() else { return };
    let f = st.floor_def();
    arena.load_t += time.delta_secs();
    let failed = matches!(server.get_load_state(&handle), Some(LoadState::Failed(_)));
    let Some(model) = wmos.get(&handle) else {
        if failed || arena.load_t > LOAD_TIMEOUT {
            abort(&mut run, &mut log, time.elapsed_secs(), &format!("{} yüklenemedi", f.wmo));
        }
        return;
    };
    let xf = arena.transform;
    let Some((verts, tris)) = placement_collider_data(model.collision.as_ref(), &xf) else {
        abort(&mut run, &mut log, time.elapsed_secs(), &format!("{} çarpışma verisi yok", f.wmo));
        return;
    };
    let t0 = std::time::Instant::now();
    let seed = 0x5EED ^ arena.floor as u64;
    let mut pts = walkable_points(&verts, &tris, WALKABLE_SAMPLES, seed, true);
    if pts.len() < 8 {
        warn!("dungeon: {} tavanlı yürünebilir nokta az ({}), tavan şartı kaldırıldı", f.wmo, pts.len());
        pts = walkable_points(&verts, &tris, WALKABLE_SAMPLES, seed, false);
    }
    let entry = f.entry.map(|e| xf.transform_point(wow_to_bevy(e))).or_else(|| pick_entry(&pts));
    let Some(entry) = entry else {
        abort(&mut run, &mut log, time.elapsed_secs(), &format!("{} yürünebilir zemin yok", f.wmo));
        return;
    };
    let local = bevy_to_wow(xf.compute_affine().inverse().transform_point3(entry));
    let visible = visible_from(&verts, &tris, &pts, entry);
    info!(
        "arena: {} {} üçgen, {} yürünebilir nokta ({} görünür, {:.1} ms), giriş yerel WoW [{:.1}, {:.1}, {:.1}]",
        f.wmo,
        tris.len(),
        pts.len(),
        visible.len(),
        t0.elapsed().as_secs_f32() * 1000.0,
        local[0],
        local[1],
        local[2]
    );
    arena.min_y = pts.iter().map(|p| p.y).fold(entry.y, f32::min);
    arena.entry = Some(entry);
    arena.walkable = pts;
    arena.visible = visible;
    let e = commands
        .spawn((PendingCollider::new(build_collider_task(verts, tris), None, true), Transform::default(), DungeonEntity))
        .id();
    arena.collider = Some(e);
}

fn abort(run: &mut DungeonRun, log: &mut GameLog, now: f32, why: &str) {
    error!("dungeon: {why}; zindandan çıkılıyor");
    log.push(now, C_BAD, "Zindan açılamadı, dışarı çıkarılıyorsun.");
    if let Some(a) = run.run.as_mut().and_then(|r| r.abandon()) {
        run.pending.push(a);
    }
}

/// Çarpışma eklenip giriş noktasında zemin bulununca oyuncuyu kat başına koyar, brifingi başlatır.
fn floor_ready(
    time: Res<Time>,
    mut arena: ResMut<Arena>,
    mut run: ResMut<DungeonRun>,
    colliders: Query<(), With<Collider>>,
    col: WorldCollision,
    mut player: Query<(&mut Transform, &mut Player)>,
    mut log: ResMut<GameLog>,
    mut events: MessageWriter<DungeonEvent>,
) {
    if arena.ready {
        return;
    }
    let (Some(c), Some(entry)) = (arena.collider, arena.entry) else { return };
    if colliders.get(c).is_err() {
        arena.load_t += time.delta_secs();
        if arena.load_t > LOAD_TIMEOUT {
            abort(&mut run, &mut log, time.elapsed_secs(), "çarpışma kurulamadı");
        }
        return;
    }
    arena.ready_wait += time.delta_secs();
    let ground = crate::game::physics::ground_at(&col, entry);
    let at = match ground {
        Some(g) if (g - entry.y).abs() < 1.5 => Vec3::new(entry.x, g, entry.z),
        // Uzaysal sorgu ağacı bir kare gecikebilir; 3 sn sonra noktanın kendisi kullanılır.
        _ if arena.ready_wait > 3.0 => {
            warn!("dungeon: giriş noktasında zemin ışını tutmadı ({ground:?}), örnek nokta kullanılıyor");
            entry + Vec3::Y * 0.05
        }
        _ => return,
    };
    arena.ready = true;
    arena.entry = Some(at);
    arena.last_good = Some(at);
    if let Ok((mut xf, mut p)) = player.single_mut() {
        pin(&mut xf, &mut p, at);
    }
    let Some(st) = run.run.as_mut() else { return };
    st.on_floor_ready();
    let f = st.floor_def();
    let now = time.elapsed_secs();
    info!("dungeon: kat {} hazır, oyuncu {at} (zemin {:?})", st.floor + 1, ground);
    log.banner(now, C_GOOD, f.name);
    log.push(now, C_INFO, format!("{} — {}", f.name, f.objective.describe()));
    events.write(DungeonEvent::FloorReady { floor: st.floor });
}

/// Zindanda başka sistemlerin doğurduğu varlıklar (metin dalgası canavarları, yerdeki ganimet) da koşuya
/// aittir: kat değişince ve çıkışta silinsin.
fn tag_strays(
    mut commands: Commands,
    arena: Res<Arena>,
    run: Res<DungeonRun>,
    q: Query<(Entity, &Transform), (Or<(Added<crate::game::mobs::Mob>, Added<crate::game::loot::GroundItem>)>, Without<DungeonEntity>)>,
) {
    if !run.active() || arena.root.is_none() {
        return;
    }
    let o = arena.transform.translation;
    for (e, xf) in &q {
        if xf.translation.distance(o) < 600.0 {
            commands.entity(e).insert(DungeonEntity);
        }
    }
}

/// Portala yürüyen oyuncu çıkar.
fn portal_touch(
    mut run: ResMut<DungeonRun>,
    portals: Query<&Transform, With<DungeonPortal>>,
    player: Query<&Transform, With<Player>>,
) {
    let Ok(p) = player.single() else { return };
    for xf in &portals {
        let d = xf.translation - p.translation;
        if Vec2::new(d.x, d.z).length() < 1.6 && d.y.abs() < 3.0 {
            if let Some(a) = run.run.as_mut().and_then(|r| r.abandon()) {
                run.pending.push(a);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (verts, tris) ekleyen yardımcı: dikdörtgen (y sabit), `up` ise normal yukarı.
    fn quad(v: &mut Vec<Vec3>, t: &mut Vec<[u32; 3]>, x0: f32, z0: f32, x1: f32, z1: f32, y: f32, up: bool) {
        let b = v.len() as u32;
        v.extend([Vec3::new(x0, y, z0), Vec3::new(x0, y, z1), Vec3::new(x1, y, z1), Vec3::new(x1, y, z0)]);
        if up {
            // (b-a)×(c-a): a=(x0,z0) b=(x0,z1) c=(x1,z1) → +y
            t.push([b, b + 1, b + 2]);
            t.push([b, b + 2, b + 3]);
        } else {
            t.push([b, b + 2, b + 1]);
            t.push([b, b + 3, b + 2]);
        }
    }

    /// Oda (0..10, tavan 5 m) + tavansız avlu (20..30) + yüksek salon (40..50, tavan 15 m) +
    /// alçak geçit (60..70, tavan 1.2 m).
    fn scene() -> (Vec<Vec3>, Vec<[u32; 3]>) {
        let (mut v, mut t) = (Vec::new(), Vec::new());
        quad(&mut v, &mut t, 0.0, 0.0, 10.0, 10.0, 0.0, true);
        quad(&mut v, &mut t, 0.0, 0.0, 10.0, 10.0, 5.0, false);
        quad(&mut v, &mut t, 20.0, 0.0, 30.0, 10.0, 0.0, true);
        quad(&mut v, &mut t, 40.0, 0.0, 50.0, 10.0, 0.0, true);
        quad(&mut v, &mut t, 40.0, 0.0, 50.0, 10.0, 15.0, false);
        quad(&mut v, &mut t, 60.0, 0.0, 70.0, 10.0, 0.0, true);
        quad(&mut v, &mut t, 60.0, 0.0, 70.0, 10.0, 1.2, false);
        (v, t)
    }

    #[test]
    fn floor_winding_is_up() {
        let (v, t) = scene();
        let n = tri_cross(v[t[0][0] as usize], v[t[0][1] as usize], v[t[0][2] as usize]);
        assert!(n.y > 0.0);
    }

    #[test]
    fn walkable_points_only_inside_rooms() {
        let (v, t) = scene();
        let pts = walkable_points(&v, &t, 200, 7, true);
        assert!(pts.len() >= 150, "{}", pts.len());
        for p in &pts {
            assert!(p.x >= 0.0 && p.x <= 10.0, "oda dışında nokta: {p}");
            assert!(p.y.abs() < 1e-4 && (0.0..=10.0).contains(&p.z));
        }
        // Tavan şartı yoksa her zemin.
        let all = walkable_points(&v, &t, 400, 7, false);
        assert!(all.iter().any(|p| p.x > 20.0 && p.x < 30.0));
        assert!(all.iter().any(|p| p.x > 60.0));
        // Belirlenimci.
        assert_eq!(walkable_points(&v, &t, 50, 3, true), walkable_points(&v, &t, 50, 3, true));
        // Boş girdi.
        assert!(walkable_points(&[], &[], 10, 1, true).is_empty());
    }

    #[test]
    fn entry_and_spread() {
        let (v, t) = scene();
        let pts = walkable_points(&v, &t, 200, 11, true);
        let e = pick_entry(&pts).unwrap();
        assert!(e.distance(Vec3::new(5.0, 0.0, 5.0)) < 2.0, "{e}");
        let s = spread_points(&pts, e, 6, 3.0, 2.0, 5);
        assert_eq!(s.len(), 6);
        for (i, a) in s.iter().enumerate() {
            assert!(a.distance(e) >= 3.0 - 1e-3);
            for b in &s[i + 1..] {
                assert!(a.distance(*b) >= 2.0 - 1e-3);
            }
        }
        // Nokta sayısından fazla istenirse yine n tane (gevşer, tekrar eder).
        assert_eq!(spread_points(&pts[..3], e, 10, 50.0, 50.0, 1).len(), 10);
        assert_eq!(spread_points(&[], e, 2, 1.0, 1.0, 1), vec![e, e]);
        let far = farthest(&pts, e).unwrap();
        assert!(far.distance(e) > 4.0);
    }

    #[test]
    fn line_of_sight() {
        let (mut v, mut t) = scene();
        // Odanın ortasına x = 5'te bir duvar (z 0..6), yalnızca z > 6 açık.
        let b = v.len() as u32;
        v.extend([Vec3::new(5.0, 0.0, 0.0), Vec3::new(5.0, 5.0, 0.0), Vec3::new(5.0, 5.0, 6.0), Vec3::new(5.0, 0.0, 6.0)]);
        t.push([b, b + 1, b + 2]);
        t.push([b, b + 2, b + 3]);
        let from = Vec3::new(2.0, 0.0, 2.0);
        assert!(!line_clear(&v, &t, from + Vec3::Y, Vec3::new(8.0, 1.0, 2.0)));
        assert!(line_clear(&v, &t, from + Vec3::Y, Vec3::new(4.0, 1.0, 4.0)));
        assert!(line_clear(&v, &t, Vec3::new(2.0, 1.0, 8.0), Vec3::new(8.0, 1.0, 8.0)));
        let pts = walkable_points(&v, &t, 200, 3, true);
        let vis = visible_from(&v, &t, &pts, from);
        assert!(!vis.is_empty() && vis.len() < pts.len());
        assert!(vis.iter().all(|p| p.x < 5.0 || p.z > 6.0 - 1e-3 || line_clear(&v, &t, from + Vec3::Y * 1.4, *p + Vec3::Y)));
    }

    #[test]
    fn ray_triangle() {
        let (a, b, c) = (Vec3::new(0.0, 2.0, 0.0), Vec3::new(1.0, 2.0, 0.0), Vec3::new(0.0, 2.0, 1.0));
        assert!((ray_tri(Vec3::new(0.2, 0.0, 0.2), Vec3::Y, a, b, c).unwrap() - 2.0).abs() < 1e-5);
        assert!(ray_tri(Vec3::new(0.9, 0.0, 0.9), Vec3::Y, a, b, c).is_none());
        assert!(ray_tri(Vec3::new(0.2, 3.0, 0.2), Vec3::Y, a, b, c).is_none());
    }

    /// Gerçek WMO'lar (MPQ gerekir: `WOW_DATA`): her katta tavanlı yürünebilir nokta ve giriş bulunur.
    /// `cargo test --release -- --ignored real_floor_wmos --nocapture` giriş noktalarını yazar.
    #[test]
    #[ignore]
    fn real_floor_wmos() {
        let data = benilla_formats::wow_data().expect("WOW_DATA");
        let mut chain = benilla_formats::open_chain(&data).expect("MPQ zinciri");
        for d in super::super::defs::DUNGEONS {
            for f in d.floors {
                let m = benilla_formats::load_wmo_collision_tris(&mut chain, f.wmo).expect(f.wmo);
                let xf = floor_transform(f);
                let (v, t) = placement_collider_data(Some(&m), &xf).expect("çarpışma");
                let t0 = std::time::Instant::now();
                let pts = walkable_points(&v, &t, WALKABLE_SAMPLES, 1, true);
                let e = pick_entry(&pts).expect("giriş");
                let local = bevy_to_wow(xf.compute_affine().inverse().transform_point3(e));
                let vis = visible_from(&v, &t, &pts, e).len();
                let ys = pts.iter().map(|p| p.y);
                let (lo, hi) = ys.fold((f32::MAX, f32::MIN), |(l, h), y| (l.min(y), h.max(y)));
                println!(
                    "{} / {}: {} üçgen, {} nokta ({vis} görünür, {:.0} ms), y {lo:.1}..{hi:.1}, giriş yerel {:?}",
                    d.name,
                    f.name,
                    t.len(),
                    pts.len(),
                    t0.elapsed().as_secs_f32() * 1000.0,
                    local.map(|x| (x * 10.0).round() / 10.0)
                );
                assert!(pts.len() >= 40, "{}: {} nokta", f.wmo, pts.len());
            }
        }
    }
}
