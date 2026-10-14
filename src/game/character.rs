//! Oyuncu karakteri görünümü: İnsan erkek gövde atlası (CharSections + zırh dokuları), geoset
//! seçimi ve sağ ele takılan silah modeli. DBC'ler bir kez açılış zincirinden okunur.

use bevy::prelude::*;

use benilla_assets::{LockRecover, WorldAssets};
use benilla_formats::{
    forearm_dressed, CharSections, CharacterGeosets, EquipGeosets, ItemDisplay, ItemDisplayCatalog,
};

use super::model::{load_m2, WowModel};

/// Görünüm: Human male savaşçı.
pub const RACE: u8 = 1;
pub const SEX: u8 = 0;
pub const SKIN: u8 = 1;
pub const FACE: u8 = 2;
pub const HAIR_STYLE: u8 = 5;
pub const HAIR_COLOR: u8 = 3;
pub const FACIAL_HAIR: u8 = 2;

/// Karakter DBC tabloları.
#[derive(Resource)]
pub struct CharDb {
    pub sections: CharSections,
    pub geosets: CharacterGeosets,
    pub items: ItemDisplayCatalog,
}

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, load_db.after(benilla_assets::AssetSet::Open));
}

fn load_db(mut commands: Commands, wa: Option<Res<WorldAssets>>) {
    let Some(wa) = wa else {
        error!("character: WorldAssets yok");
        return;
    };
    let mut chain = wa.chain.lock_recover();
    let sections = CharSections::load(&mut chain);
    let geosets = CharacterGeosets::load(&mut chain);
    let items = benilla_formats::load_item_display_catalog(&mut chain);
    match (sections, geosets, items) {
        (Ok(sections), Ok(geosets), Ok(items)) => {
            info!("character: CharSections + geoset + {} ItemDisplay yüklendi", items.len());
            commands.insert_resource(CharDb {
                sections,
                geosets,
                items,
            });
        }
        (a, b, c) => error!(
            "character: DBC yüklenemedi: {:?} {:?} {:?}",
            a.err(),
            b.err(),
            c.err()
        ),
    }
}

/// Zırh seti: 8 gövde yuvası için ItemDisplayInfo kimlikleri
/// (gömlek, göğüs, kemer, pantolon, çizme, bileklik, eldiven, tabard).
pub type ArmorSet = [Option<u32>; 8];

/// İnsan görünümü (cinsiyet, ten, yüz, saç). Oyuncu `Appearance::PLAYER` kullanır; NPC'ler farklı.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Appearance {
    pub female: bool,
    pub skin: u8,
    pub face: u8,
    pub hair_style: u8,
    pub hair_color: u8,
    /// Erkekte sakal, kadında küpe/takı stili.
    pub facial: u8,
}

impl Appearance {
    pub const PLAYER: Self = Self {
        female: SEX == 1,
        skin: SKIN,
        face: FACE,
        hair_style: HAIR_STYLE,
        hair_color: HAIR_COLOR,
        facial: FACIAL_HAIR,
    };
    pub const fn male(skin: u8, face: u8, hair_style: u8, hair_color: u8, facial: u8) -> Self {
        Self { female: false, skin, face, hair_style, hair_color, facial }
    }
    pub const fn female(skin: u8, face: u8, hair_style: u8, hair_color: u8, facial: u8) -> Self {
        Self { female: true, skin, face, hair_style, hair_color, facial }
    }
    fn sex(&self) -> u8 {
        self.female as u8
    }
    fn model(&self) -> &'static str {
        if self.female {
            "Character/Human/Female/HumanFemale.m2"
        } else {
            "Character/Human/Male/HumanMale.m2"
        }
    }
}

/// Karakter gövdesi için `WowModel` kurar (bileşik doku + geosetler), oyuncu görünümüyle.
pub fn build_body(
    db: &CharDb,
    wa: &mut WorldAssets,
    images: &mut Assets<Image>,
    server: &AssetServer,
    armor: &ArmorSet,
) -> WowModel {
    build_body_as(db, wa, images, server, armor, &Appearance::PLAYER)
}

/// `build_body` + verilen görünüm (NPC'ler: kadın/erkek, ten, saç).
pub fn build_body_as(
    db: &CharDb,
    wa: &mut WorldAssets,
    images: &mut Assets<Image>,
    server: &AssetServer,
    armor: &ArmorSet,
    look: &Appearance,
) -> WowModel {
    build_body_full(db, wa, images, server, armor, look, None)
}

/// Gövde + kask: `helm` (ItemDisplay) verilirse kaskın `HelmetGeosetVisData` maskesi saç, sakal ve
/// kulak geosetlerini gizler (saç kaskın içinden çıkmaz). Kask modelinin kendisi takma noktası
/// 11'e ayrıca takılır (`equipment::attach_items`).
pub fn build_body_full(
    db: &CharDb,
    wa: &mut WorldAssets,
    images: &mut Assets<Image>,
    server: &AssetServer,
    armor: &ArmorSet,
    look: &Appearance,
    helm: Option<u32>,
) -> WowModel {
    let sex = look.sex();
    let equipment: [Option<&ItemDisplay>; 8] =
        std::array::from_fn(|i| armor[i].and_then(|id| display_or_warn(db, id, "gövde")));
    let atlas = {
        let mut chain = wa.chain.lock_recover();
        db.sections.composite_body(
            &mut chain,
            RACE,
            sex,
            look.skin,
            look.face,
            look.facial,
            look.hair_style,
            look.hair_color,
            equipment,
            None,
            false,
        )
    };
    let body = match atlas {
        Ok(Some(chain)) => Some(images.add(benilla_assets::repeat_texture_authored(
            benilla_assets::for_upload(chain),
            (false, false),
        ))),
        Ok(None) => None,
        Err(e) => {
            warn!("character: gövde atlası kurulamadı: {e:#}");
            None
        }
    };
    let hair = db
        .sections
        .hair_texture(RACE, sex, look.hair_style, look.hair_color)
        .map(str::to_owned)
        .and_then(|p| wa.texture(&p, (true, true), images));

    let bodyslots = std::array::from_fn(|i| equipment[i].map(|d| d.geoset_groups));
    let eq = EquipGeosets {
        bodyslots,
        cloak: None,
        helm_vis: helm
            .and_then(|h| display_or_warn(db, h, "kask"))
            .and_then(ItemDisplay::worn_helm_vis),
        forearm_dressed: forearm_dressed(&equipment),
        tabard_preview: false,
    };
    let vis = db
        .geosets
        .visible_geosets(RACE, sex, look.hair_style, look.facial, &eq);
    let geosets: Vec<u16> = (0u16..=1700).filter(|g| vis.contains(g)).collect();

    let mut wm = WowModel::new(load_m2(server, look.model()));
    wm.body = body;
    wm.hair = hair;
    wm.geosets = Some(geosets);
    wm
}

/// ItemDisplay satırı; yoksa kimlik başına bir kez `warn!` (geçersiz kimlik eskiden sessizce çıplak
/// görünüm verirdi).
pub fn display_or_warn<'a>(db: &'a CharDb, id: u32, what: &str) -> Option<&'a ItemDisplay> {
    let d = db.items.get(id);
    if d.is_none() {
        use std::sync::{Mutex, OnceLock};
        static WARNED: OnceLock<Mutex<std::collections::HashSet<u32>>> = OnceLock::new();
        let first = WARNED
            .get_or_init(Default::default)
            .lock()
            .map(|mut w| w.insert(id))
            .unwrap_or(false);
        if first {
            warn!("character: ItemDisplay {id} ({what}) DBC'de yok, karakterde görünmeyecek");
        }
    }
    d
}

/// Silah (veya kalkan) modeli: ItemDisplayInfo satırından yol + doku.
pub fn build_item_model(
    db: &CharDb,
    wa: &mut WorldAssets,
    images: &mut Assets<Image>,
    server: &AssetServer,
    display: u32,
    dir: &str,
) -> Option<WowModel> {
    build_item_model_at(db, wa, images, server, display, dir, 0, None)
}

/// Eşya modeli: `dir` klasörü (`Weapon`, `Shield`, `Head`, `Shoulder`), `col` model/doku sütunu
/// (omuzluk: 0 = sol, 1 = sağ), `helm_suffix` kask ırk+cinsiyet eki (`HuM` → `Head/{stem}_HuM.m2`).
/// Benilla `ensure_item_model` ile aynı kural; doku satırın kendi sütunundan, modelden türetilmez.
#[allow(clippy::too_many_arguments)]
pub fn build_item_model_at(
    db: &CharDb,
    wa: &mut WorldAssets,
    images: &mut Assets<Image>,
    server: &AssetServer,
    display: u32,
    dir: &str,
    col: usize,
    helm_suffix: Option<&str>,
) -> Option<WowModel> {
    let d = display_or_warn(db, display, dir)?;
    let Some(model) = d.model.get(col).and_then(|m| m.as_deref()) else {
        let id = display;
        warn!("character: ItemDisplay {id} ({dir}) {col}. sütunda model yok");
        return None;
    };
    let mut wm = WowModel::new(load_m2(server, &item_model_path(dir, model, helm_suffix)));
    if let Some(tex) = d.model_texture.get(col).and_then(|t| t.as_deref()) {
        wm.object = wa.texture(
            &format!("Item\\ObjectComponents\\{dir}\\{tex}.blp"),
            (true, true),
            images,
        );
    }
    Some(wm)
}

/// `Item/ObjectComponents/{dir}/{model}`; kask için `{stem}_{ek}.m2`.
pub fn item_model_path(dir: &str, model: &str, helm_suffix: Option<&str>) -> String {
    let base = model.rsplit(['\\', '/']).next().unwrap_or(model);
    match helm_suffix {
        Some(sfx) => {
            let lower = base.to_ascii_lowercase();
            let stem = if lower.ends_with(".m2") {
                &base[..base.len() - 3]
            } else if lower.ends_with(".mdx") {
                &base[..base.len() - 4]
            } else {
                base
            };
            format!("Item/ObjectComponents/{dir}/{stem}_{sfx}.m2")
        }
        None => format!("Item/ObjectComponents/{dir}/{base}"),
    }
}

/// Kask modeli eki: İnsan erkek `HuM`, kadın `HuF` (benilla `RACE_PREFIX`).
pub fn helm_suffix(female: bool) -> &'static str {
    if female {
        "HuF"
    } else {
        "HuM"
    }
}

/// Bir ItemDisplay'in envanter ikonunun yolu (`Interface\Icons\X.blp`).
pub fn icon_path(db: &CharDb, display: u32) -> Option<String> {
    db.items.get(display)?.icon.as_ref().map(|i| format!("{i}.blp"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_model_paths() {
        assert_eq!(
            item_model_path("Weapon", "Sword_1H_Short_A_01.m2", None),
            "Item/ObjectComponents/Weapon/Sword_1H_Short_A_01.m2"
        );
        assert_eq!(
            item_model_path("Head", "Helm_Plate_D_03.m2", Some(helm_suffix(false))),
            "Item/ObjectComponents/Head/Helm_Plate_D_03_HuM.m2"
        );
        assert_eq!(
            item_model_path("Head", "Item\\ObjectComponents\\Head\\helm_mail_a_02.M2", Some(helm_suffix(true))),
            "Item/ObjectComponents/Head/helm_mail_a_02_HuF.m2"
        );
    }
}
