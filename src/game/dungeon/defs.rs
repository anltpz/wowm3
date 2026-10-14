//! Zindan tanımları (veri): `DungeonDef` → katlar (`FloorDef`) → kat hedefi (`Objective`).
//!
//! İçerik dosyaları (`spider.rs` G-28, `tower.rs` G-29) kendi `DUNGEON` sabitini doldurur; kayıt `DUNGEONS`
//! tablosundadır (buraya yeni zindan = bir satır). Yerleşim: her kat kendi WMO'sunu ADT'siz batı okyanusunda
//! (`instance_origin`) kendi ofsetinde açar; çarpışma o WMO'nun yürüme yüzlerinden kurulur (`arena.rs`).

use crate::game::metin::{MetinDef, METINS};

/// Instance bölgesinin köşesi (WoW x, y): Azeroth haritasının ADT'siz batı okyanusu. Pyungmoo odağı
/// (-8000, 14000) ile çakışmaz; katlar x boyunca, zindanlar y boyunca dizilir.
pub const INSTANCE_BASE: [f32; 2] = [-3000.0, 15500.0];
/// Kat aralığı (m, WoW x ekseninde): bir katın WMO'su diğerinin görüş alanına girmesin.
pub const FLOOR_SPACING: f32 = 800.0;
/// Zindan aralığı (m, WoW y ekseninde).
pub const DUNGEON_SPACING: f32 = 1200.0;

/// `slot`. zindanın `floor`. katının WMO kökü (WoW dünya koordinatı). İçerik tabloları `offset`'i bununla
/// verir: `offset: instance_origin(1, 0)`.
pub const fn instance_origin(slot: u32, floor: u32) -> [f32; 3] {
    [
        INSTANCE_BASE[0] - FLOOR_SPACING * floor as f32,
        INSTANCE_BASE[1] - DUNGEON_SPACING * slot as f32,
        0.0,
    ]
}

/// Bir katın tamamlanma şartı.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Objective {
    /// Kattaki bütün canavarları (`FloorDef::mobs`) öldür.
    KillAll,
    /// `count` metin taşı kır (`metin` = `metin::METINS` indeksi). Dalgalarda `summons` çağrılır
    /// (boşsa taşın kendi listesi). `FloorDef::mobs` ayrıca doğar ama şart değildir.
    BreakMetin { metin: usize, count: u32, summons: &'static [u32] },
    /// Anahtarı bul: `carriers` kimlikli moblar öldüğünde `chance` olasılıkla `key_vnum` düşer (doğrudan
    /// envantere). Son taşıyıcı öldüğünde anahtar kesin düşer. Anahtar kat bitince mührü açar (tüketilir).
    /// Taşıyıcılar `FloorDef::mobs` içinde olmalı.
    FindKey { carriers: &'static [u32], key_vnum: u32, chance: f32 },
    /// Boss: `id` mob, yanında `adds` (mob, adet). Boss ölünce kat (son katsa zindan) biter.
    Boss { id: u32, adds: &'static [(u32, u32)] },
}

impl Objective {
    /// Hedefin oyuncuya gösterilen kısa açıklaması (brifing, izleyici).
    pub fn describe(&self) -> String {
        match *self {
            Objective::KillAll => "Bütün canavarları öldür".into(),
            Objective::BreakMetin { metin, count, .. } => {
                let name = METINS.get(metin).map_or("Metin Taşı", |m| m.name);
                if count == 1 { format!("Metin taşını kır: {name}") } else { format!("{count} metin taşı kır: {name}") }
            }
            Objective::FindKey { .. } => "Mühür anahtarını bul".into(),
            Objective::Boss { id, .. } => {
                let name = crate::game::mobs::find(id).map_or("Boss", |m| m.name);
                format!("Boss'u yen: {name}")
            }
        }
    }

    /// Boss katı mı (durum makinesi `Active` yerine `BossFight` kullanır).
    pub fn is_boss(&self) -> bool {
        matches!(self, Objective::Boss { .. })
    }

    /// `BreakMetin` hedefinin metin tanımı.
    #[allow(dead_code)] // D-22 izleyicisi (taş adı/rengi)
    pub fn metin_def(&self) -> Option<&'static MetinDef> {
        match *self {
            Objective::BreakMetin { metin, .. } => METINS.get(metin),
            _ => None,
        }
    }
}

/// Bir kat: kendi WMO'su, kendi ofseti, giriş noktası, canavarları ve hedefi.
#[derive(Clone, Copy, Debug)]
pub struct FloorDef {
    /// Brifingde görünen ad ("1. Kat: Örümcek Yuvası").
    pub name: &'static str,
    /// WMO yolu (`tools/mpq_present.txt` ile doğrula), ör. `World\wmo\Dungeon\MD_Spidermine\MD_Spidermine.wmo`.
    pub wmo: &'static str,
    /// WMO kökünün WoW dünya konumu (genelde `instance_origin(slot, kat)`).
    pub offset: [f32; 3],
    /// WMO'nun dikey eksen etrafında dönüşü (derece).
    pub yaw: f32,
    /// Giriş noktası, WMO yerel WoW koordinatı (x, y, z). `None` → yürünebilir noktaların ortasına en yakın
    /// nokta otomatik seçilir (log'a `arena: giriş` satırı yazılır; içerik yazarı oradan sabitleyebilir).
    pub entry: Option<[f32; 3]>,
    /// Katta doğan canavarlar: (mob kimliği, adet). Yürünebilir noktalara girişten uzağa dağıtılır.
    pub mobs: &'static [(u32, u32)],
    pub objective: Objective,
}

impl FloorDef {
    /// Hedefe sayılan kat canavarı sayısı (`mobs` toplamı; `KillAll` bunu bekler).
    pub const fn counted_mobs(&self) -> u32 {
        let mut n = 0;
        let mut i = 0;
        while i < self.mobs.len() {
            n += self.mobs[i].1;
            i += 1;
        }
        n
    }
}

/// Bir zindan.
#[derive(Clone, Copy, Debug)]
pub struct DungeonDef {
    /// Benzersiz kimlik (kayıttaki bekleme süresi anahtarı; değiştirme!).
    pub id: u32,
    /// Test/komut adı: `AZM_DUNGEON=<key>`.
    pub key: &'static str,
    pub name: &'static str,
    /// Giriş seviye aralığı (dahil).
    pub level: (u32, u32),
    /// Giriş bileti vnum'u (girişte 1 adet tüketilir); 0 = biletsiz.
    pub ticket: u32,
    /// Rehber NPC bileti bu fiyata satar (Yang); 0 = satmaz.
    pub ticket_price: u32,
    /// Giriş sonrası bekleme süresi (gerçek zaman, sn; oyun kapalıyken de işler).
    pub cooldown_secs: u64,
    /// Zindan süre sınırı (sn); dolunca başarısız → çıkış.
    pub time_limit_secs: f32,
    pub floors: &'static [FloorDef],
}

impl DungeonDef {
    pub fn floor(&self, i: usize) -> Option<&'static FloorDef> {
        self.floors.get(i)
    }
}

/// Fizibilite/duman testi (`AZM_DUNGEON=test`): tek odalı küçük maden, birkaç kobold. Rehberi yok.
pub const TEST_DUNGEON: DungeonDef = DungeonDef {
    id: 900,
    key: "test",
    name: "Deneme Madeni",
    level: (1, 99),
    ticket: 0,
    ticket_price: 0,
    cooldown_secs: 0,
    time_limit_secs: 1800.0,
    floors: &[FloorDef {
        name: "Deneme Madeni",
        wmo: r"World\wmo\Dungeon\MD_Goldmine\MD_Goldmine_1Room.wmo",
        offset: instance_origin(0, 0),
        yaw: 0.0,
        entry: None,
        mobs: &[(202, 2), (203, 2)],
        objective: Objective::KillAll,
    }],
};

/// Kayıtlı bütün zindanlar. Yeni zindan = kendi dosyasında `DUNGEON` + buraya bir satır.
pub const DUNGEONS: &[&DungeonDef] = &[&TEST_DUNGEON, &super::spider::DUNGEON, &super::tower::DUNGEON];

/// Kimlikten zindan.
pub fn find(id: u32) -> Option<&'static DungeonDef> {
    DUNGEONS.iter().copied().find(|d| d.id == id)
}

/// `AZM_DUNGEON` adından zindan (büyük/küçük harf duyarsız).
pub fn by_key(key: &str) -> Option<&'static DungeonDef> {
    let key = key.trim();
    DUNGEONS.iter().copied().find(|d| d.key.eq_ignore_ascii_case(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRESENT: &str = include_str!("../../../tools/mpq_present.txt");

    fn present(path: &str) -> bool {
        let p = path.replace('/', "\\").to_ascii_lowercase();
        PRESENT.lines().any(|l| l.trim().to_ascii_lowercase() == p)
    }

    /// Kayıt tutarlı: kimlik/ad benzersiz, her katın WMO'su (ve ilk grubu) MPQ'da, moblar ve metinler kayıtlı,
    /// bilet eşya tablosunda, katlar birbirinden ve Pyungmoo'dan uzak.
    #[test]
    fn registry_is_valid() {
        let mut ids = std::collections::HashSet::new();
        let mut keys = std::collections::HashSet::new();
        let mut origins: Vec<[f32; 3]> = Vec::new();
        for d in DUNGEONS {
            assert!(ids.insert(d.id), "zindan kimliği tekrar: {}", d.id);
            assert!(keys.insert(d.key.to_ascii_lowercase()), "zindan adı tekrar: {}", d.key);
            assert!(!d.floors.is_empty(), "{} katsız", d.name);
            assert!(d.level.0 <= d.level.1);
            assert!(d.time_limit_secs > 0.0);
            if d.ticket != 0 {
                assert!(crate::game::items::all_items().any(|i| i.vnum == d.ticket), "{} bileti yok: {}", d.name, d.ticket);
            }
            for f in d.floors {
                assert!(present(f.wmo), "{}: WMO MPQ'da yok: {}", d.name, f.wmo);
                let g0 = f.wmo.replace(".wmo", "_000.wmo");
                assert!(present(&g0), "{}: WMO grubu yok: {g0}", d.name);
                for (id, n) in f.mobs {
                    assert!(crate::game::mobs::find(*id).is_some(), "{}: mob {id} kayıtlı değil", d.name);
                    assert!(*n > 0);
                }
                match f.objective {
                    Objective::BreakMetin { metin, count, summons } => {
                        assert!(metin < METINS.len() && count > 0);
                        for id in summons {
                            assert!(crate::game::mobs::find(*id).is_some(), "{}: çağrı {id}", d.name);
                        }
                    }
                    Objective::FindKey { carriers, key_vnum, chance } => {
                        assert!(!carriers.is_empty() && (0.0..=1.0).contains(&chance));
                        for c in carriers {
                            assert!(f.mobs.iter().any(|(id, _)| id == c), "{}: taşıyıcı {c} katta yok", d.name);
                        }
                        if key_vnum != 0 {
                            assert!(crate::game::items::all_items().any(|i| i.vnum == key_vnum), "anahtar {key_vnum}");
                        }
                    }
                    Objective::Boss { id, adds } => {
                        assert!(crate::game::mobs::find(id).is_some(), "{}: boss {id}", d.name);
                        for (a, _) in adds {
                            assert!(crate::game::mobs::find(*a).is_some(), "{}: yardımcı {a}", d.name);
                        }
                    }
                    Objective::KillAll => assert!(!f.mobs.is_empty(), "{}: KillAll katında mob yok", d.name),
                }
                // Pyungmoo odağından ve haritanın kenarından uzak.
                assert!(Vec2::new(f.offset[0] + 8000.0, f.offset[1] - 14000.0).length() > 1500.0, "{} Pyungmoo'ya yakın", f.name);
                assert!(f.offset[1].abs() < 17000.0 && f.offset[0].abs() < 17000.0);
                origins.push(f.offset);
            }
        }
        // Aynı WMO konumunu iki kat paylaşmaz (zindanlar farklı slotlarda).
        for (i, a) in origins.iter().enumerate() {
            for b in &origins[i + 1..] {
                assert!(Vec2::new(a[0] - b[0], a[1] - b[1]).length() >= 400.0, "katlar çakışıyor: {a:?} {b:?}");
            }
        }
    }

    use bevy::math::Vec2;

    #[test]
    fn lookup_by_id_and_key() {
        assert_eq!(by_key("TEST").map(|d| d.id), Some(TEST_DUNGEON.id));
        assert_eq!(find(TEST_DUNGEON.id).map(|d| d.key), Some("test"));
        assert!(by_key("yok").is_none());
        assert!(find(12345).is_none());
    }

    #[test]
    fn objective_text_and_kind() {
        assert_eq!(Objective::KillAll.describe(), "Bütün canavarları öldür");
        assert!(Objective::Boss { id: 225, adds: &[] }.is_boss());
        assert!(!Objective::KillAll.is_boss());
        let m = Objective::BreakMetin { metin: 0, count: 2, summons: &[] };
        assert!(m.describe().starts_with("2 metin"));
        assert!(m.metin_def().is_some());
    }
}
