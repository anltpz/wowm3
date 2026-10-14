//! Orijinal WoW haritası: Elwynn Forest (Azeroth). Konumlar WoW dünya koordinatı (x kuzey, y batı, z yukarı).
//! Alt bölge merkezleri MPQ'daki ADT dosyalarından (MCNK areaId) ölçüldü; bina konumları MODF'tan
//! (Northshire Manastırı -8897,-178; Goldshire hanı -9464,24; demirhane -9460,94; Echo Ridge girişi -8646,-132).
//! Yaratıklar WoW 1.12 Elwynn'deki türler ve yerleri (Northshire: Genç Kurt, kobold, Defias; Elwynn: domuz,
//! kurt, örümcek, maden koboldları, murloc, Defias, Riverpaw gnolları; patronlar Garrick Padfoot, Princess,
//! Mother Fang, Hogger).

/// İnsan karakterin WoW başlangıç noktası (Northshire Manastırı önü).
pub const HUMAN_START: [f32; 3] = [-8949.95, -132.49, 83.53];

/// Alt bölgeler (mini harita adı): (ad, x, y, yarıçap). Küçükler önce; hiçbiri değilse "Elwynn Forest".
pub const SUBZONES: &[(&str, f32, f32, f32)] = &[
    ("Echo Ridge Mine", -8631.0, -130.0, 70.0),
    ("Northshire Vineyards", -9072.0, -371.0, 90.0),
    ("Heroes' Vigil", -9111.0, -1029.0, 60.0),
    ("Jerod's Landing", -9914.0, -124.0, 90.0),
    ("Westbrook Garrison", -9594.0, 700.0, 90.0),
    ("Goldshire", -9438.0, 64.0, 110.0),
    ("Mirror Lake", -9335.0, 477.0, 100.0),
    ("The Maclure Vineyards", -9971.0, 81.0, 100.0),
    ("Tower of Azora", -9494.0, -692.0, 110.0),
    ("Fargodeep Mine", -9909.0, 235.0, 120.0),
    ("The Stonefield Farm", -9905.0, 427.0, 120.0),
    ("Ridgepoint Tower", -9760.0, -1388.0, 120.0),
    ("Thunder Falls", -9321.0, 822.0, 120.0),
    ("Jasperlode Mine", -9240.0, -628.0, 130.0),
    ("Brackwell Pumpkin Patch", -9787.0, -858.0, 150.0),
    ("Crystal Lake", -9393.0, -312.0, 150.0),
    ("Northshire Valley", -8833.0, -269.0, 250.0),
    ("Eastvale Logging Camp", -9437.0, -1387.0, 250.0),
    ("Forest's Edge", -9781.0, 698.0, 250.0),
    ("Stone Cairn Lake", -8979.0, -1045.0, 300.0),
    ("Stormwind City", -8642.0, 537.0, 400.0),
];

/// WoW konumundaki alt bölge adı.
pub fn subzone(wow: [f32; 3]) -> &'static str {
    SUBZONES
        .iter()
        .find(|(_, x, y, r)| (wow[0] - x).hypot(wow[1] - y) <= *r)
        .map_or("Elwynn Forest", |(n, ..)| n)
}

/// Yaratık yerleşimi: (mob türleri, x, y, z, yarıçap, sürü sayısı, sürü boyu en az, en çok).
pub const SPAWNS: &[(&[u32], f32, f32, f32, f32, usize, usize, usize)] = &[
    // --- Northshire Valley (Sv 1-5) ---
    (&[201], -8925.0, -250.0, 85.0, 80.0, 6, 1, 2),
    (&[201], -8780.0, -320.0, 105.0, 60.0, 4, 1, 2),
    (&[202], -8740.0, -170.0, 100.0, 50.0, 5, 1, 2),
    (&[203, 204], -8665.0, -145.0, 92.0, 35.0, 4, 1, 2),
    (&[205], -9072.0, -371.0, 77.0, 55.0, 6, 1, 2),
    (&[206], -9105.0, -395.0, 75.0, 15.0, 1, 1, 1),
    // --- Elwynn Forest (Sv 5-11) ---
    (&[207], -9300.0, -100.0, 62.0, 150.0, 6, 1, 2),
    (&[207, 208], -9550.0, 300.0, 55.0, 150.0, 5, 1, 2),
    (&[208], -9200.0, -200.0, 70.0, 150.0, 5, 1, 2),
    (&[209], -9650.0, -400.0, 50.0, 150.0, 5, 1, 2),
    (&[210], -9250.0, -550.0, 75.0, 120.0, 5, 1, 2),
    (&[210], -9450.0, -900.0, 60.0, 150.0, 4, 1, 2),
    (&[211, 212, 213], -9850.0, 180.0, 32.0, 60.0, 6, 1, 3),
    (&[211, 212, 213], -9170.0, -610.0, 63.0, 50.0, 4, 1, 2),
    (&[225], -9160.0, -600.0, 63.0, 10.0, 1, 1, 1),
    (&[214], -9393.0, -312.0, 68.0, 80.0, 5, 1, 3),
    (&[215], -9914.0, -124.0, 30.0, 60.0, 5, 2, 3),
    (&[216], -9930.0, 250.0, 33.0, 120.0, 5, 1, 2),
    (&[217], -9787.0, -858.0, 37.0, 80.0, 6, 1, 2),
    (&[218], -9800.0, -870.0, 37.0, 20.0, 1, 1, 1),
    (&[219], -9300.0, -1000.0, 70.0, 200.0, 5, 1, 2),
    (&[220], -9450.0, -1300.0, 65.0, 150.0, 5, 1, 2),
    (&[221], -9200.0, -1300.0, 75.0, 150.0, 4, 1, 1),
    (&[222], -9600.0, -1200.0, 55.0, 150.0, 4, 1, 2),
    (&[223], -9781.0, 698.0, 52.0, 120.0, 6, 1, 3),
    (&[224], -9760.0, 690.0, 52.0, 25.0, 1, 1, 1),
];

/// Metin taşı alanları: (metin türü, x, y, z, yarıçap). Northshire'dan uzaklaştıkça zorlaşır.
pub const METIN_SLOTS: &[(usize, f32, f32, f32, f32)] = &[
    (0, -8850.0, -280.0, 100.0, 90.0),
    (0, -9000.0, -250.0, 85.0, 70.0),
    (0, -8780.0, -330.0, 110.0, 60.0),
    (1, -9300.0, -150.0, 62.0, 150.0),
    (1, -9500.0, 250.0, 55.0, 150.0),
    (1, -9250.0, 150.0, 70.0, 120.0),
    (2, -9300.0, -650.0, 70.0, 150.0),
    (2, -9550.0, -500.0, 55.0, 150.0),
    (2, -9650.0, 100.0, 40.0, 120.0),
    (3, -9450.0, -1250.0, 65.0, 150.0),
    (3, -9750.0, -900.0, 37.0, 120.0),
    (4, -9780.0, 650.0, 50.0, 120.0),
    (4, -9000.0, -1000.0, 100.0, 150.0),
];

/// Metin taşı dalgalarında çağrılan Elwynn yaratıkları (metin türüne göre).
pub fn metin_summons(kind: usize) -> &'static [u32] {
    match kind {
        0 => &[201, 202, 203],
        1 => &[207, 208, 205, 204],
        2 => &[209, 210, 211, 214, 216],
        3 => &[212, 213, 217, 219, 221],
        _ => &[220, 222, 223, 215],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subzones_resolve() {
        assert_eq!(subzone(HUMAN_START), "Northshire Valley");
        assert_eq!(subzone([-9450.0, 60.0, 57.0]), "Goldshire");
        assert_eq!(subzone([-8640.0, -130.0, 90.0]), "Echo Ridge Mine");
        assert_eq!(subzone([-12000.0, 0.0, 0.0]), "Elwynn Forest");
    }

    #[test]
    fn spawns_reference_known_mobs() {
        for (kinds, ..) in SPAWNS {
            for k in *kinds {
                assert!(crate::game::mobs::MOBS.iter().any(|m| m.id == *k), "tanımsız mob {k}");
            }
        }
        for kind in 0..5 {
            for k in metin_summons(kind) {
                assert!(crate::game::mobs::MOBS.iter().any(|m| m.id == *k), "tanımsız mob {k}");
            }
        }
        assert!(METIN_SLOTS.iter().all(|s| s.0 < crate::game::metin::METINS.len()));
    }
}
