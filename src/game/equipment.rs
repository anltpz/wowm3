//! Ekipman görünümü (G-25): kuşanılan eşyalar karakterin üstünde görünür.
//!
//! - `look_from_equip(&Inventory) -> LookSpec` (saf): zırh seti taban; çizme `armor[4]`, bileklik
//!   `armor[5]` yuvasının üstüne yazılır; kask `helm`, kalkan `shield` (iki elli silahta kalkan yok
//!   sayılır); omuzluk zırh setinden; silah + artısı.
//! - `sync_look`: ekipman değişince `PlayerLook`'u günceller (UI yüklenmesinden bağımsız).
//! - `attach_items`: gövde hazır olunca eşya modellerini takma noktalarına asar: silah 1, kalkan 0,
//!   kask 11 (`Head/{stem}_HuM.m2`), sol omuz 6 (`model[0]`), sağ omuz 5 (`model[1]`).
//!   Referans: benilla-app `entities/equipment/mod.rs` (`ensure_item_model`, `attach_id`).
//! - Test kancası: `AZM_EQUIP=vnum,vnum,…` başlangıçta bu eşyaları kuşandırır (yalnızca kayıt
//!   kapalıyken: `AZM_NOSAVE=1` ya da `AZM_SHOT`; kullanıcı kaydı kirlenmez).

use bevy::prelude::*;

use benilla_assets::WorldAssets;

use super::character::{build_item_model_at, helm_suffix, Appearance, ArmorSet, CharDb};
use super::inventory::Inventory;
use super::items::{self, EquipSlot, Item, Kind};
use super::model::ModelReady;
use super::player::{Body, PlayerLook};

/// M2 takma noktası kimlikleri (benilla `attach_id`).
pub mod attach_id {
    /// Sol ön kol: kuşanılmış kalkan.
    pub const SHIELD: u16 = 0;
    pub const HAND_RIGHT: u16 = 1;
    pub const SHOULDER_RIGHT: u16 = 5;
    pub const SHOULDER_LEFT: u16 = 6;
    pub const HELM: u16 = 11;
}

/// Gövde atlası yuvaları (`ArmorSet` sırası): çizme ve bileklik.
const SLOT_BOOTS: usize = 4;
const SLOT_WRIST: usize = 5;

/// Kuşanılan eşyalardan çıkan görünüm (ItemDisplay kimlikleri).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct LookSpec {
    pub armor: ArmorSet,
    pub weapon: Option<u32>,
    pub two_hand: bool,
    pub weapon_plus: u8,
    pub shield: Option<u32>,
    pub helm: Option<u32>,
    pub shoulder: Option<u32>,
}

/// Bir `Wear` eşyasının görünümü (0 = görünümsüz).
fn wear_display(it: Option<Item>) -> Option<u32> {
    match it?.def().kind {
        Kind::Wear { display, .. } if display != 0 => Some(display),
        _ => None,
    }
}

/// Ekipmandan görünüm (saf).
pub fn look_from_equip(inv: &Inventory) -> LookSpec {
    let mut s = LookSpec::default();
    // Zırh seti taban + setin omuzluğu.
    if let Some(Kind::Armor { set, shoulder, .. }) = inv.equipped(EquipSlot::Armor).map(|i| i.def().kind) {
        s.armor = set;
        s.shoulder = shoulder;
    }
    // Çizme ve bileklik setin aynı yuvasının üstüne yazılır.
    if let Some(d) = wear_display(inv.equipped(EquipSlot::Shoes)) {
        s.armor[SLOT_BOOTS] = Some(d);
    }
    if let Some(d) = wear_display(inv.equipped(EquipSlot::Bracelet)) {
        s.armor[SLOT_WRIST] = Some(d);
    }
    s.helm = wear_display(inv.equipped(EquipSlot::Helmet));
    if let Some(w) = inv.equipped(EquipSlot::Weapon) {
        if let Kind::Weapon { display, two_hand, .. } = w.def().kind {
            s.weapon = Some(display);
            s.two_hand = two_hand;
            s.weapon_plus = w.plus;
        }
    }
    // İki elli silahla kalkan tutulmaz (görünmez).
    if !s.two_hand {
        s.shield = wear_display(inv.equipped(EquipSlot::Shield));
    }
    s
}

/// Gövde hazır olunca takılacak eşyalar (`player::rebuild_look` ekler).
#[derive(Component)]
pub struct WantsItems(pub LookSpec);

/// Gövdeye takılı eşya modeli (silah, kalkan, kask, omuzluk).
#[derive(Component)]
pub struct HeldItem;

pub fn plugin(app: &mut App) {
    app.add_systems(PostStartup, env_equip).add_systems(
        Update,
        (sync_look.before(super::player::rebuild_look), attach_items),
    );
}

/// Ekipman değişince karakter görünümünü güncelle (her kare ucuz bir karşılaştırma).
fn sync_look(inv: Res<Inventory>, mut look: ResMut<PlayerLook>) {
    let spec = look_from_equip(&inv);
    if look.spec() != spec {
        look.set_spec(spec);
    }
}

/// Bir takılacak model: ItemDisplay, takma noktası, klasör, sütun, kask eki.
struct Part {
    display: u32,
    attach: u16,
    dir: &'static str,
    col: usize,
    helm: bool,
}

/// Görünümden takılacak modeller (saf).
fn parts(s: &LookSpec) -> Vec<Part> {
    let mut v = Vec::new();
    let mut push = |display: Option<u32>, attach, dir, col, helm| {
        if let Some(display) = display {
            v.push(Part { display, attach, dir, col, helm });
        }
    };
    push(s.weapon, attach_id::HAND_RIGHT, "Weapon", 0, false);
    push(s.shield, attach_id::SHIELD, "Shield", 0, false);
    push(s.helm, attach_id::HELM, "Head", 0, true);
    push(s.shoulder, attach_id::SHOULDER_LEFT, "Shoulder", 0, false);
    push(s.shoulder, attach_id::SHOULDER_RIGHT, "Shoulder", 1, false);
    v
}

fn attach_items(
    mut commands: Commands,
    db: Option<Res<CharDb>>,
    mut wa: Option<ResMut<WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    server: Res<AssetServer>,
    q: Query<(Entity, &ModelReady, &WantsItems), With<Body>>,
) {
    let (Some(db), Some(wa)) = (db, wa.as_deref_mut()) else {
        return;
    };
    let suffix = helm_suffix(Appearance::PLAYER.female);
    for (e, ready, want) in &q {
        for p in parts(&want.0) {
            let Some((joint, offset)) = ready.attachment(p.attach) else {
                warn!("equipment: takma noktası {} yok ({} {})", p.attach, p.dir, p.display);
                continue;
            };
            let helm = p.helm.then_some(suffix);
            if let Some(wm) = build_item_model_at(&db, wa, &mut images, &server, p.display, p.dir, p.col, helm) {
                commands.spawn((
                    wm,
                    HeldItem,
                    Transform::from_translation(offset),
                    Visibility::default(),
                    ChildOf(joint),
                ));
            }
        }
        commands.entity(e).remove::<WantsItems>();
    }
}

/// `AZM_EQUIP=11220,12210,…` → vnum listesi.
fn parse_equip_env(v: &str) -> Vec<u32> {
    v.split([',', ';', ' ']).filter_map(|p| p.trim().parse().ok()).collect()
}

/// Eşyayı envantere koyup kuşandırır (seviye şartı yok; eski eşya çantaya döner). Sonuç mesajı.
fn force_equip(inv: &mut Inventory, vnum: u32) -> Result<EquipSlot, String> {
    let def = items::def(vnum).ok_or_else(|| format!("{vnum} tanımsız"))?;
    let slot = def.equip_slot().ok_or_else(|| format!("{} kuşanılamaz", def.name))?;
    if inv.add(Item::new(vnum)).is_some() {
        return Err("envanterde yer yok".into());
    }
    let cell = inv.find(vnum).ok_or("eklenen eşya bulunamadı")?;
    inv.equip_from(cell, u32::MAX).map_err(str::to_owned)?;
    Ok(slot)
}

/// Test kancası `AZM_EQUIP` (yalnızca kayıt kapalıyken).
fn env_equip(mut inv: ResMut<Inventory>) {
    let Ok(v) = std::env::var("AZM_EQUIP") else { return };
    if super::save::save_path().is_some() {
        warn!("equipment: AZM_EQUIP yok sayıldı — kayıt açık (AZM_NOSAVE=1 ile kullan)");
        return;
    }
    for vnum in parse_equip_env(&v) {
        match force_equip(&mut inv, vnum) {
            Ok(slot) => info!("equipment: AZM_EQUIP {vnum} → {}", slot.name()),
            Err(e) => warn!("equipment: AZM_EQUIP {vnum}: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn equip(vnums: &[u32]) -> Inventory {
        let mut inv = Inventory::default();
        for &v in vnums {
            force_equip(&mut inv, v).unwrap();
        }
        inv
    }

    fn wear(vnum: u32) -> u32 {
        match items::def(vnum).unwrap().kind {
            Kind::Wear { display, .. } => display,
            k => panic!("{vnum} giyilebilir değil: {k:?}"),
        }
    }

    #[test]
    fn empty_equipment_is_naked() {
        assert_eq!(look_from_equip(&Inventory::default()), LookSpec::default());
    }

    #[test]
    fn armor_set_is_base_and_brings_shoulder() {
        let s = look_from_equip(&equip(&[11220]));
        let Kind::Armor { set, shoulder, .. } = items::def(11220).unwrap().kind else { panic!() };
        assert_eq!(s.armor, set);
        assert_eq!(s.shoulder, shoulder);
        assert!(s.shoulder.is_some(), "Kudret setinin omuzluğu olmalı");
        assert_eq!((s.helm, s.shield, s.weapon), (None, None, None));
    }

    #[test]
    fn boots_and_bracers_override_set_slots() {
        let s = look_from_equip(&equip(&[11220, 15010, 14000]));
        let Kind::Armor { set, .. } = items::def(11220).unwrap().kind else { panic!() };
        assert_eq!(s.armor[SLOT_BOOTS], Some(wear(15010)));
        assert_eq!(s.armor[SLOT_WRIST], Some(wear(14000)));
        // Diğer yuvalar setten.
        for i in [0, 1, 2, 3, 6, 7] {
            assert_eq!(s.armor[i], set[i], "yuva {i}");
        }
        // Zırhsız da çizme/bileklik görünür.
        let s = look_from_equip(&equip(&[15000, 14010]));
        assert_eq!(s.armor[SLOT_BOOTS], Some(wear(15000)));
        assert_eq!(s.armor[SLOT_WRIST], Some(wear(14010)));
        assert_eq!(s.armor[1], None);
    }

    #[test]
    fn helm_and_shield_with_one_hand_weapon() {
        let mut inv = equip(&[30, 12210, 13010]);
        inv.equip[EquipSlot::Weapon.index()].as_mut().unwrap().plus = 7;
        let s = look_from_equip(&inv);
        assert_eq!(s.helm, Some(wear(12210)));
        assert_eq!(s.shield, Some(wear(13010)));
        assert_eq!(s.weapon, Some(5040));
        assert!(!s.two_hand);
        assert_eq!(s.weapon_plus, 7);
        let p = parts(&s);
        let ids: Vec<u16> = p.iter().map(|p| p.attach).collect();
        assert_eq!(ids, vec![attach_id::HAND_RIGHT, attach_id::SHIELD, attach_id::HELM]);
        assert!(p.iter().find(|p| p.attach == attach_id::HELM).unwrap().helm);
    }

    #[test]
    fn two_hand_weapon_hides_shield() {
        let s = look_from_equip(&equip(&[35, 13000]));
        assert!(s.two_hand);
        assert_eq!(s.shield, None, "iki elli silahta kalkan görünmez");
        // Tek ele dönünce kalkan geri gelir.
        let s = look_from_equip(&equip(&[35, 13000, 10]));
        assert!(!s.two_hand);
        assert_eq!(s.shield, Some(wear(13000)));
    }

    #[test]
    fn shoulders_go_left_and_right() {
        let s = look_from_equip(&equip(&[11230]));
        let p = parts(&s);
        let l = p.iter().find(|p| p.attach == attach_id::SHOULDER_LEFT).unwrap();
        let r = p.iter().find(|p| p.attach == attach_id::SHOULDER_RIGHT).unwrap();
        assert_eq!((l.col, r.col), (0, 1));
        assert_eq!((l.dir, r.dir), ("Shoulder", "Shoulder"));
        assert_eq!(l.display, r.display);
    }

    #[test]
    fn jewelry_has_no_look() {
        let s = look_from_equip(&equip(&[16000, 17000]));
        assert_eq!(s, LookSpec::default());
    }

    #[test]
    fn env_list_parses() {
        assert_eq!(parse_equip_env("11220, 12210;13010 x"), vec![11220, 12210, 13010]);
    }

    #[test]
    fn force_equip_ignores_level_and_returns_old_item() {
        let mut inv = equip(&[10]);
        assert_eq!(force_equip(&mut inv, 80).unwrap(), EquipSlot::Weapon); // Sv35 Külgetiren
        assert_eq!(inv.equipped(EquipSlot::Weapon).unwrap().vnum, 80);
        assert_eq!(inv.count(10), 1, "eski silah çantaya döner");
        assert!(force_equip(&mut inv, items::V_RED_POT_S).is_err());
    }
}
