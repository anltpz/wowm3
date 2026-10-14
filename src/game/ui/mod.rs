//! Metin2 tarzı arayüz: alt HUD (HP/SP, EXP küreleri, hızlı slotlar), hedef çerçevesi, sistem
//! mesajları, isim plakaları/hasar sayıları, envanter, demirci (artı basma) ve mağaza pencereleri.

use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use benilla_assets::WorldAssets;

pub mod hud;
pub mod inventory_ui;
pub mod shop_ui;
pub mod skin;
pub mod tooltip;
pub mod upgrade_ui;
pub mod world_ui;

/// Ortak UI kaynakları.
#[derive(Resource)]
pub struct UiAssets {
    pub font: Handle<Font>,
    pub num_font: Handle<Font>,
    pub title_font: Handle<Font>,
}

/// Açık pencereler.
#[derive(Resource, Default)]
pub struct Windows {
    pub inventory: bool,
    pub upgrade: bool,
    pub shop: bool,
    pub help: bool,
    /// Açık mağazanın türü (hangi satıcı).
    pub shop_kind: ShopKind,
}

/// Satıcı türü: her birinin kendi stoku var.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShopKind {
    /// Tüccar: iksirler, parşömenler.
    #[default]
    General,
    /// Silah Satıcısı.
    Weapons,
    /// Zırh Satıcısı.
    Armor,
}

impl Windows {
    /// Demirci penceresi (+envanter); mağazayı kapatır. NPC pencereleri birbirini dışlar (QA H3).
    pub fn open_upgrade(&mut self) {
        self.inventory = true;
        self.upgrade = true;
        self.shop = false;
    }
    /// Mağaza penceresi (+envanter); Demirci'yi kapatır.
    pub fn open_shop(&mut self, kind: ShopKind) {
        self.inventory = true;
        self.shop = true;
        self.shop_kind = kind;
        self.upgrade = false;
    }
}

/// İmlecin üstündeyken dünyaya tıklamayı engelleyen UI düğümü.
#[derive(Component)]
pub struct BlocksWorld;

// Metin2 renk paleti: koyu kahve paneller, altın kenarlar.
pub const PANEL_BG: Color = Color::srgba(0.07, 0.055, 0.04, 0.93);
pub const PANEL_BORDER: Color = Color::srgb(0.62, 0.48, 0.24);
#[allow(dead_code)]
pub const TITLE_BG: Color = Color::srgba(0.18, 0.12, 0.06, 0.98);
#[allow(dead_code)]
pub const SLOT_HOVER: Color = Color::srgb(0.85, 0.7, 0.3);
pub const TEXT: Color = Color::srgb(0.93, 0.9, 0.82);
pub const TEXT_DIM: Color = Color::srgb(0.65, 0.6, 0.52);
pub const GOLD: Color = Color::srgb(1.0, 0.82, 0.35);

pub fn plugin(app: &mut App) {
    app.init_resource::<Windows>()
        .add_systems(Startup, setup.after(benilla_assets::AssetSet::Open))
        .add_systems(PreUpdate, update_hover.after(bevy::ui::UiSystems::Focus))
        .add_systems(Update, (toggle_windows, env_open))
        .add_plugins((
            hud::plugin,
            world_ui::plugin,
            inventory_ui::plugin,
            tooltip::plugin,
            upgrade_ui::plugin,
            shop_ui::plugin,
            skin::plugin,
        ));
}

/// Türkçe karakterli (ş, ğ, ı, İ) sistem fontu; WoW fontlarında bu glifler yok.
fn system_font(fonts: &mut Assets<Font>, names: &[&str]) -> Option<Handle<Font>> {
    let dir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
    for n in names {
        if let Ok(bytes) = std::fs::read(format!(r"{dir}\Fonts\{n}")) {
            if let Ok(f) = Font::try_from_bytes(bytes) {
                return Some(fonts.add(f));
            }
        }
    }
    None
}

fn setup(mut commands: Commands, server: Res<AssetServer>, mut fonts: ResMut<Assets<Font>>) {
    let font = system_font(&mut fonts, &["pala.ttf", "georgia.ttf", "segoeui.ttf"])
        .unwrap_or_else(|| server.load("mpq://Fonts/FRIZQT__.TTF"));
    let title = system_font(&mut fonts, &["palab.ttf", "georgiab.ttf"]).unwrap_or_else(|| font.clone());
    commands.insert_resource(UiAssets {
        font,
        num_font: server.load("mpq://Fonts/SKURRI.TTF"),
        title_font: title,
    });
}

fn update_hover(
    q: Query<(&RelativeCursorPosition, &InheritedVisibility), With<BlocksWorld>>,
    mut hover: ResMut<super::combat::UiHover>,
) {
    hover.0 = q
        .iter()
        .any(|(r, v)| v.get() && r.cursor_over());
}

fn toggle_windows(
    keys: Res<ButtonInput<KeyCode>>,
    mut w: ResMut<Windows>,
    mut target: ResMut<crate::game::combat::PlayerTarget>,
    mut auto: ResMut<crate::game::combat::AutoAttack>,
) {
    // WoW: B çantalar (I de çalışır); Tab hedef seçmeye ayrıldı.
    if keys.just_pressed(KeyCode::KeyB) || keys.just_pressed(KeyCode::KeyI) {
        w.inventory = !w.inventory;
    }
    if keys.just_pressed(KeyCode::KeyH) {
        w.help = !w.help;
    }
    if keys.just_pressed(KeyCode::Escape) {
        if w.upgrade || w.shop {
            w.upgrade = false;
            w.shop = false;
        } else if w.inventory || w.help {
            w.inventory = false;
            w.help = false;
        } else {
            // WoW: açık pencere yoksa Esc hedefi bırakır.
            target.0 = None;
            auto.0 = false;
        }
    }
}

/// `AZM_OPEN=inv,shop,upgrade` (virgülle; `help` de olur): pencereleri açılışta açar (ekran
/// görüntüsü testleri). NPC'ler doğana kadar (en çok 15 sn) her karede yeniden uygulanır; çünkü
/// `npc::auto_close` yakında NPC yokken mağaza/demirci pencerelerini hemen kapatır.
fn env_open(
    time: Res<Time>,
    mut w: ResMut<Windows>,
    npcs: Query<(), With<crate::game::npc::Npc>>,
    mut state: Local<Option<Vec<String>>>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    let list = state.get_or_insert_with(|| {
        std::env::var("AZM_OPEN")
            .map(|v| v.split(',').map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).collect())
            .unwrap_or_default()
    });
    if list.is_empty() {
        *done = true;
        return;
    }
    for k in list.iter() {
        match k.as_str() {
            "inv" | "inventory" | "envanter" => w.inventory = true,
            // NPC pencereleri birbirini dışlar: ikisi de verilirse sonuncusu açılır.
            "shop" | "tuccar" | "satici" => w.open_shop(ShopKind::General),
            "silah" | "weapons" => w.open_shop(ShopKind::Weapons),
            "zirh" | "armor" => w.open_shop(ShopKind::Armor),
            "upgrade" | "demirci" => w.open_upgrade(),
            "help" | "yardim" => w.help = true,
            _ => {}
        }
    }
    if !npcs.is_empty() || time.elapsed_secs() > 15.0 {
        info!("ui: AZM_OPEN uygulandı: {:?}", list);
        *done = true;
    }
}

/// İkon dokusu (WorldAssets önbelleğinden).
pub fn icon(
    wa: &mut WorldAssets,
    images: &mut Assets<Image>,
    path: &str,
) -> Option<Handle<Image>> {
    wa.sprite_texture(path, images)
}

pub fn text(ui: &UiAssets, s: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(s),
        TextFont {
            font: ui.font.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
    )
}

/// Altın kenarlı panel düğümü.
pub fn panel(node: Node) -> impl Bundle {
    (
        Node {
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(px(4)),
            ..node
        },
        BackgroundColor(PANEL_BG),
        BorderColor::all(PANEL_BORDER),
        RelativeCursorPosition::default(),
        BlocksWorld,
    )
}

/// Binlik ayraçlı sayı: 12.345
pub fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    out
}
