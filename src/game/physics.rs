//! Basit karakter hareketi: dünyaya karşı yatay kayma (ağaç gövdesi, duvar) ve zemine oturtma.
//! Pyungmoo açıkken zemin, su, yamaç ve bina engeli arazi verisinden gelir.

use std::sync::atomic::{AtomicBool, Ordering};

use avian3d::prelude::Collider;
use bevy::prelude::*;
use benilla_world::collision::WorldCollision;

/// Oyuncu kapalı mekânda (zindan instance'ı) mı. true iken Pyungmoo arazi kısa yolu ve `ground_at`'taki
/// +60 m yedek ışın kapalıdır (tavana/üst kata ışınlanmayı önler); yalnızca dünya çarpışması kullanılır.
/// Zindan sistemi girişte true, çıkışta false yapar. Varsayılan false: davranış eskisiyle aynı.
pub static INDOOR: AtomicBool = AtomicBool::new(false);

/// `INDOOR` bayrağını okur.
pub fn indoor() -> bool {
    INDOOR.load(Ordering::Relaxed)
}

/// `INDOOR` bayrağını yazar.
#[allow(dead_code)]
pub fn set_indoor(on: bool) {
    INDOOR.store(on, Ordering::Relaxed);
}

/// Zemin yüksekliği: Pyungmoo arazisi, yoksa dizden aşağı ışın (ağaç tepesine ışınlanmayı önler).
pub fn ground_at(col: &WorldCollision, at: Vec3) -> Option<f32> {
    let indoor = indoor();
    if !indoor {
        if let Some(d) = super::terrain::data() {
            return d.ground(at);
        }
    }
    let near = Vec3::new(at.x, at.y + 1.6, at.z);
    if let Some(h) = col.ray_body(near, Dir3::NEG_Y, 60.0) {
        return Some(near.y - h.distance);
    }
    // İçeride üstten yedek ışın yok: tavan/üst kat zemini sanılır.
    if indoor {
        return None;
    }
    let far = Vec3::new(at.x, at.y + 60.0, at.z);
    col.ray_body(far, Dir3::NEG_Y, 200.0)
        .map(|h| far.y - h.distance)
}

/// `pos`'tan yatay `delta` kadar ilerler, engele çarpınca yüzey boyunca kayar.
pub fn slide(col: &WorldCollision, pos: Vec3, delta: Vec3, radius: f32) -> Vec3 {
    let shape = Collider::sphere(radius);
    let base = pos + Vec3::Y * (radius + 0.6);
    let mut moved = Vec3::ZERO;
    let mut d = Vec3::new(delta.x, 0.0, delta.z);
    for _ in 0..3 {
        let len = d.length();
        if len < 1e-5 {
            break;
        }
        match col.cast_body(&shape, base + moved, d, 0.02) {
            Some(hit) if !hit.intersects() => {
                let dir = d / len;
                let safe = hit.distance.min(len);
                moved += dir * safe;
                let rest = d - dir * safe;
                let mut n = Vec3::new(hit.normal2.x, 0.0, hit.normal2.z);
                if n.length_squared() < 1e-6 {
                    break;
                }
                n = n.normalize();
                d = rest - n * rest.dot(n);
            }
            // İçinde başladı ya da çarpmadı: serbest hareket.
            _ => {
                moved += d;
                break;
            }
        }
    }
    pos + moved
}

/// Pyungmoo: adım geçerli mi (su, bina, dik yamaç). Değilse eksenlerden birinde kaymayı dene.
fn terrain_step(d: &super::terrain::TerrainData, pos: Vec3, delta: Vec3) -> Vec3 {
    let ok = |p: Vec3| {
        let m = d.to_map(p);
        if d.blocked(m) || d.water_depth(p) > 0.6 {
            return None;
        }
        let g = d.ground(p)?;
        let run = Vec2::new(p.x - pos.x, p.z - pos.z).length().max(0.01);
        // ~50°'den dik yamaçlara tırmanılmaz (inişe izin var).
        if (g - pos.y) / run > 1.25 {
            return None;
        }
        Some(Vec3::new(p.x, g, p.z))
    };
    let full = Vec3::new(pos.x + delta.x, pos.y, pos.z + delta.z);
    if let Some(p) = ok(full) {
        return p;
    }
    for alt in [Vec3::new(delta.x, 0.0, 0.0), Vec3::new(0.0, 0.0, delta.z)] {
        if alt.length_squared() > 1e-8 {
            if let Some(p) = ok(pos + alt) {
                return p;
            }
        }
    }
    pos
}

/// Yürüyen bir gövdeyi ilerletir: kayma + basamak/yokuş sınırı + zemine oturtma.
pub fn walk(col: &WorldCollision, pos: Vec3, delta: Vec3, radius: f32) -> Vec3 {
    if !indoor() {
        if let Some(d) = super::terrain::data() {
            return terrain_step(d, pos, delta);
        }
    }
    let next = slide(col, pos, delta, radius);
    match ground_at(col, next) {
        // Çok dik yokuşa (1.3 yd'den yüksek basamak) çıkma.
        Some(g) if g - pos.y < 1.3 => Vec3::new(next.x, g, next.z),
        Some(_) => pos,
        None => next,
    }
}
