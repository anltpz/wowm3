//! Bölge kaydı (orijinal WoW haritası): her bölge (`elwynn.rs`, `westfall.rs`, ...) kendi tablolarını bir
//! `ZoneDef` içinde verir ve burada `ZONES` listesine tek satırla eklenir. Moblar (`mobs.rs`), metin taşları
//! (`metin.rs`) ve HUD mini harita adı bölgeleri yalnızca bu kayıt üzerinden okur; yeni bölge eklemek için
//! `mobs.rs` / `metin.rs` / `hud.rs` dosyalarına dokunulmaz.
//!
//! Pyungmoo (`AZM_PYUNGMOO=1`) bu kaydı kullanmaz; kendi tabloları `mobs.rs` / `metin.rs` içindedir.

use super::{elwynn, westfall};

/// Alt bölge (mini harita adı): (ad, WoW x, WoW y, yarıçap).
pub type Subzone = (&'static str, f32, f32, f32);

/// Yaratık yerleşimi: (mob türleri, WoW x, y, z, yarıçap, sürü sayısı, sürü boyu en az, en çok).
pub type SpawnRow = (&'static [u32], f32, f32, f32, f32, usize, usize, usize);

/// Metin taşı alanı: (metin türü = `metin::METINS` indeksi, WoW x, y, z, yarıçap).
pub type MetinSlotRow = (usize, f32, f32, f32, f32);

/// Bir bölgenin içerik tabloları.
pub struct ZoneDef {
    /// Bölge adı; noktası hiçbir alt bölgeye düşmeyen ama `bounds` içinde kalan konumlarda gösterilir.
    pub name: &'static str,
    /// Yaklaşık bölge kutusu (WoW): [x en az, x en çok, y en az, y en çok].
    pub bounds: [f32; 4],
    /// Alt bölgeler; küçükler önce (ilk eşleşen kazanır).
    pub subzones: &'static [Subzone],
    pub spawns: &'static [SpawnRow],
    pub metin_slots: &'static [MetinSlotRow],
    /// Metin taşı dalgalarında çağrılan bölge yaratıkları (metin türüne göre).
    pub metin_summons: fn(usize) -> &'static [u32],
}

impl ZoneDef {
    pub fn contains(&self, wow: [f32; 3]) -> bool {
        let [x0, x1, y0, y1] = self.bounds;
        (x0..=x1).contains(&wow[0]) && (y0..=y1).contains(&wow[1])
    }

    /// Bu bölgede `wow` noktasını içeren alt bölgenin adı.
    pub fn subzone_at(&self, wow: [f32; 3]) -> Option<&'static str> {
        self.subzones
            .iter()
            .find(|(_, x, y, r)| (wow[0] - x).hypot(wow[1] - y) <= *r)
            .map(|(n, ..)| *n)
    }
}

/// Kayıtlı bölgeler. Yeni bölge = kendi dosyasında `pub const ZONE: ZoneDef` + buraya bir satır.
pub const ZONES: &[&ZoneDef] = &[&elwynn::ZONE, &westfall::ZONE];

/// Varsayılan (açılış) bölgesi: konum hiçbir bölgeye düşmezse mini haritada bu ad yazar.
pub const DEFAULT_ZONE: &ZoneDef = &elwynn::ZONE;

/// WoW konumunun mini harita adı: önce bütün bölgelerin alt bölgeleri, sonra bölge kutuları.
/// Hiçbiri tutmazsa `None` (çağıran `DEFAULT_ZONE.name` gösterir).
pub fn zone_name(wow: [f32; 3]) -> Option<&'static str> {
    ZONES
        .iter()
        .find_map(|z| z.subzone_at(wow))
        .or_else(|| ZONES.iter().find(|z| z.contains(wow)).map(|z| z.name))
}

/// Bütün bölgelerin yaratık yerleşimleri.
pub fn spawns() -> impl Iterator<Item = &'static SpawnRow> {
    ZONES.iter().flat_map(|z| z.spawns.iter())
}

/// Bütün bölgelerin metin alanları sırayla; `metin_slot(i)` ile aynı sıra.
pub fn metin_slots() -> impl Iterator<Item = (&'static ZoneDef, &'static MetinSlotRow)> {
    ZONES.iter().flat_map(|z| z.metin_slots.iter().map(move |s| (*z, s)))
}

pub fn metin_slot_count() -> usize {
    ZONES.iter().map(|z| z.metin_slots.len()).sum()
}

/// Düz indeksle metin alanı (bölgesiyle birlikte).
pub fn metin_slot(i: usize) -> Option<(&'static ZoneDef, &'static MetinSlotRow)> {
    metin_slots().nth(i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zone_names_match_elwynn() {
        assert_eq!(zone_name(elwynn::HUMAN_START), Some("Northshire Valley"));
        assert_eq!(zone_name([-9450.0, 60.0, 57.0]), Some("Goldshire"));
        // Elwynn içinde alt bölge dışı → bölge adı; harita dışı → None (HUD varsayılanı yazar).
        assert_eq!(zone_name([-9700.0, -300.0, 50.0]), Some("Elwynn Forest"));
        assert_eq!(zone_name([-12000.0, 0.0, 0.0]), None);
        assert_eq!(DEFAULT_ZONE.name, "Elwynn Forest");
        // Eski davranış: elwynn::subzone ile aynı ad.
        for p in [[-8640.0, -130.0, 90.0], [-9335.0, 477.0, 60.0], [-9700.0, -300.0, 50.0]] {
            assert_eq!(zone_name(p).unwrap_or(DEFAULT_ZONE.name), elwynn::subzone(p));
        }
    }

    #[test]
    fn registry_tables_are_valid() {
        for (kinds, ..) in spawns() {
            for k in *kinds {
                assert!(crate::game::mobs::find(*k).is_some(), "tanımsız mob {k}");
            }
        }
        assert_eq!(metin_slots().count(), metin_slot_count());
        for (z, s) in metin_slots() {
            assert!(s.0 < crate::game::metin::METINS.len(), "{}: geçersiz metin türü {}", z.name, s.0);
            for k in (z.metin_summons)(s.0) {
                assert!(crate::game::mobs::find(*k).is_some(), "{}: tanımsız çağrı {k}", z.name);
            }
        }
        assert!(metin_slot(metin_slot_count()).is_none());
        assert_eq!(metin_slot(0).map(|(z, _)| z.name), Some("Elwynn Forest"));
    }
}
