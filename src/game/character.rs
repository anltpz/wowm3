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

/// Karakter gövdesi için `WowModel` kurar (bileşik doku + geosetler).
pub fn build_body(
    db: &CharDb,
    wa: &mut WorldAssets,
    images: &mut Assets<Image>,
    server: &AssetServer,
    armor: &ArmorSet,
) -> WowModel {
    let equipment: [Option<&ItemDisplay>; 8] =
        std::array::from_fn(|i| armor[i].and_then(|id| db.items.get(id)));
    let atlas = {
        let mut chain = wa.chain.lock_recover();
        db.sections.composite_body(
            &mut chain,
            RACE,
            SEX,
            SKIN,
            FACE,
            FACIAL_HAIR,
            HAIR_STYLE,
            HAIR_COLOR,
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
        .hair_texture(RACE, SEX, HAIR_STYLE, HAIR_COLOR)
        .map(str::to_owned)
        .and_then(|p| wa.texture(&p, (true, true), images));

    let bodyslots = std::array::from_fn(|i| equipment[i].map(|d| d.geoset_groups));
    let eq = EquipGeosets {
        bodyslots,
        cloak: None,
        helm_vis: None,
        forearm_dressed: forearm_dressed(&equipment),
        tabard_preview: false,
    };
    let vis = db
        .geosets
        .visible_geosets(RACE, SEX, HAIR_STYLE, FACIAL_HAIR, &eq);
    let geosets: Vec<u16> = (0u16..=1700).filter(|g| vis.contains(g)).collect();

    let mut wm = WowModel::new(load_m2(server, "Character/Human/Male/HumanMale.m2"));
    wm.body = body;
    wm.hair = hair;
    wm.geosets = Some(geosets);
    wm
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
    let d = db.items.get(display)?;
    let model = d.model[0].as_deref()?;
    let base = model.rsplit(['\\', '/']).next().unwrap_or(model);
    let path = format!("Item/ObjectComponents/{dir}/{base}");
    let mut wm = WowModel::new(load_m2(server, &path));
    if let Some(tex) = d.model_texture[0].as_deref() {
        wm.object = wa.texture(
            &format!("Item\\ObjectComponents\\{dir}\\{tex}.blp"),
            (true, true),
            images,
        );
    }
    Some(wm)
}

/// Bir ItemDisplay'in envanter ikonunun yolu (`Interface\Icons\X.blp`).
pub fn icon_path(db: &CharDb, display: u32) -> Option<String> {
    db.items.get(display)?.icon.as_ref().map(|i| format!("{i}.blp"))
}
