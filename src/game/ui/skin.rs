//! WoW 1.12 pencere derisi: gerçek `Interface\` dokularından (MPQ) pencere çerçevesi, başlık
//! plakası, slot, düğme, kapat düğmesi, sekme, onay kutusu ve Yang sikkesi.
//!
//! WoW backdrop kenar dokusu ("edgeFile", ör. `UI-DialogBox-Border.blp` 256×32) tek satırda 8 kare
//! parçadır: sol, sağ, üst, alt kenar, sol üst, sağ üst, sol alt, sağ alt köşe. Üst/alt kenarlar
//! dikey şerit olarak saklanır (istemci UV'yi döndürür); burada CPU'da transpoze edilir. Parçalar
//! 3E×3E'lik bir 9-dilim dokusunda birleştirilir ve `NodeImageMode::Sliced` ile çizilir.
//! "Highlight" dokuları WoW'da toplamalı (ADD) karışır; Bevy UI'da alfa karışımına çevrilir.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite::{BorderRect, SliceScaleMode, TextureSlicer};
use bevy::ui::RelativeCursorPosition;

use benilla_assets::WorldAssets;

use super::{BlocksWorld, UiAssets};
use crate::game::items::EquipSlot;

/// Pencere kenarının ekrandaki ölçeği (32 px doku → 24 px).
pub const DIALOG_SCALE: f32 = 0.75;
/// WoW DialogBox arka plan içe kayması (11 px @ 32) ölçekli.
const DIALOG_INSET: f32 = 11.0 * DIALOG_SCALE;
/// İçerik dolgusu (kenarın hemen içi).
pub const DIALOG_PAD: f32 = 17.0;
/// Başlık plakasının pencere üstünden taşma miktarı.
const HEADER_RISE: f32 = 12.0;
const HEADER_SCALE: f32 = 0.85;
pub const HEADER_H: f32 = 40.0 * HEADER_SCALE;

/// WoW altın yazı rengi (NORMAL_FONT_COLOR 1, .82, 0).
pub const WOW_GOLD: Color = Color::srgb(1.0, 0.82, 0.0);
/// WoW beyaz/yüksek vurgu (HIGHLIGHT_FONT_COLOR).
pub const WOW_WHITE: Color = Color::srgb(1.0, 1.0, 1.0);
/// Seçili olmayan sekme yazısı (soluk altın).
const TAB_DIM: Color = Color::srgb(0.78, 0.64, 0.1);
/// WoW tooltip arka plan rengi (TOOLTIP_DEFAULT_BACKGROUND_COLOR .09,.09,.19; doku 0.54 gri olduğu için
/// çarpım sonucu ona denk gelecek şekilde açıldı).
const TOOLTIP_TINT: Color = Color::srgb(0.12, 0.12, 0.25);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style {
    /// Gri taş çerçeve (StaticPopup / DialogBox).
    Dialog,
    /// Altın çerçeve (Demirci).
    Gold,
    /// Tooltip (ince gümüş kenar, lacivert zemin).
    Tooltip,
}

/// Hazırlanmış deri dokuları.
#[derive(Resource, Clone)]
pub struct Skin {
    pub dialog_bg: Handle<Image>,
    pub dialog_edge: Handle<Image>,
    pub dialog_header: Handle<Image>,
    pub gold_bg: Handle<Image>,
    pub gold_edge: Handle<Image>,
    pub gold_header: Handle<Image>,
    pub gold_dragon: Handle<Image>,
    pub tooltip_bg: Handle<Image>,
    pub tooltip_edge: Handle<Image>,
    /// Koyu mermer (UI-PaperDoll-SlotBackground'un dolu bölgesi).
    pub marble: Handle<Image>,
    /// Çanta slotu (UI-Slot-Background, gri kenar + kanat deseni).
    pub slot_bg: Handle<Image>,
    /// Altın kenarlı boş slot (UI-EmptySlot).
    pub empty_slot: Handle<Image>,
    /// Slot vurgusu (ButtonHilight-Square, alfaya çevrilmiş).
    pub slot_hi: Handle<Image>,
    pub btn_up: Handle<Image>,
    pub btn_down: Handle<Image>,
    pub btn_hi: Handle<Image>,
    pub close_up: Handle<Image>,
    pub close_down: Handle<Image>,
    pub close_hi: Handle<Image>,
    pub tab_active: Handle<Image>,
    pub tab_inactive: Handle<Image>,
    pub tab_hi: Handle<Image>,
    pub check_up: Handle<Image>,
    pub check_mark: Handle<Image>,
    /// Satıcı satırı zemini (UI-Merchant-LabelSlots).
    pub row_bg: Handle<Image>,
    /// Satır vurgusu (UI-QuestTitleHighlight, alfaya çevrilmiş).
    pub row_hi: Handle<Image>,
    /// Yang için WoW altın sikkesi (UI-MoneyIcons'un ilk karesi).
    pub coin: Handle<Image>,
    /// Boş ekipman yuvası silüetleri, `EquipSlot::index()` sırasıyla.
    pub doll: [Handle<Image>; 8],
}

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            build_skin.run_if(resource_exists::<WorldAssets>.and(not(resource_exists::<Skin>))),
            (button_states, check_states).run_if(resource_exists::<Skin>),
        ),
    );
}

// ---------------------------------------------------------------------------------------------
// Doku hazırlama (CPU)
// ---------------------------------------------------------------------------------------------

#[derive(Clone)]
struct Rgba {
    w: u32,
    h: u32,
    px: Vec<u8>,
}

impl Rgba {
    fn blank(w: u32, h: u32) -> Self {
        Self { w, h, px: vec![0; (w * h * 4) as usize] }
    }
    fn get(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.w + x) * 4) as usize;
        [self.px[i], self.px[i + 1], self.px[i + 2], self.px[i + 3]]
    }
    fn put(&mut self, x: u32, y: u32, c: [u8; 4]) {
        let i = ((y * self.w + x) * 4) as usize;
        self.px[i..i + 4].copy_from_slice(&c);
    }
    fn crop(&self, x0: u32, y0: u32, w: u32, h: u32) -> Self {
        let (w, h) = (w.min(self.w.saturating_sub(x0)), h.min(self.h.saturating_sub(y0)));
        let mut o = Self::blank(w, h);
        for y in 0..h {
            for x in 0..w {
                o.put(x, y, self.get(x0 + x, y0 + y));
            }
        }
        o
    }
    fn transpose(&self) -> Self {
        let mut o = Self::blank(self.h, self.w);
        for y in 0..self.h {
            for x in 0..self.w {
                o.put(y, x, self.get(x, y));
            }
        }
        o
    }
    fn blit(&mut self, src: &Rgba, dx: u32, dy: u32) {
        for y in 0..src.h.min(self.h.saturating_sub(dy)) {
            for x in 0..src.w.min(self.w.saturating_sub(dx)) {
                self.put(dx + x, dy + y, src.get(x, y));
            }
        }
    }
    /// WoW'un toplamalı (ADD) vurgu dokusunu alfa karışımına çevirir: siyah = saydam.
    fn additive(&self, gain: f32) -> Self {
        let mut o = self.clone();
        for p in o.px.chunks_exact_mut(4) {
            let m = p[0].max(p[1]).max(p[2]);
            if m == 0 {
                p.copy_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            let k = 255.0 / m as f32;
            let src_a = p[3] as f32 / 255.0;
            for c in &mut p[..3] {
                *c = (*c as f32 * k).min(255.0) as u8;
            }
            p[3] = ((m as f32 * gain * src_a).min(255.0)) as u8;
        }
        o
    }
}

/// 8 parçalı WoW kenar şeridini 3E×3E 9-dilim dokusuna çevirir.
fn edge_composite(strip: &Rgba) -> Rgba {
    let e = strip.h;
    let piece = |i: u32| strip.crop(i * e, 0, e, e);
    let mut o = Rgba::blank(3 * e, 3 * e);
    o.blit(&piece(0), 0, e); // sol
    o.blit(&piece(1), 2 * e, e); // sağ
    o.blit(&piece(2).transpose(), e, 0); // üst
    o.blit(&piece(3).transpose(), e, 2 * e); // alt
    o.blit(&piece(4), 0, 0); // sol üst
    o.blit(&piece(5), 2 * e, 0); // sağ üst
    o.blit(&piece(6), 0, 2 * e); // sol alt
    o.blit(&piece(7), 2 * e, 2 * e); // sağ alt
    o
}

fn upload(images: &mut Assets<Image>, img: Rgba) -> Handle<Image> {
    images.add(Image::new(
        Extent3d { width: img.w.max(1), height: img.h.max(1), depth_or_array_layers: 1 },
        TextureDimension::D2,
        if img.px.is_empty() { vec![0; 4] } else { img.px },
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}

fn build_skin(mut commands: Commands, mut wa: ResMut<WorldAssets>, mut images: ResMut<Assets<Image>>) {
    let mut missing = Vec::new();
    let mut load = |p: &str| -> Rgba {
        match wa.decode_rgba(p) {
            Some((w, h, px)) => Rgba { w, h, px },
            None => {
                missing.push(p.to_string());
                Rgba::blank(1, 1)
            }
        }
    };
    const DF: &str = "Interface\\DialogFrame\\";
    const BT: &str = "Interface\\Buttons\\";
    let dialog_bg = load(&format!("{DF}UI-DialogBox-Background.blp"));
    let dialog_edge = edge_composite(&load(&format!("{DF}UI-DialogBox-Border.blp")));
    let dialog_header = load(&format!("{DF}UI-DialogBox-Header.blp")).crop(58, 0, 139, 40);
    let gold_bg = load(&format!("{DF}UI-DialogBox-Gold-Background.blp"));
    let gold_edge = edge_composite(&load(&format!("{DF}UI-DialogBox-Gold-Border.blp")));
    let gold_header = load(&format!("{DF}UI-DialogBox-Gold-Header.blp")).crop(58, 0, 139, 40);
    let gold_dragon = load(&format!("{DF}UI-DialogBox-Gold-Dragon.blp"));
    let tooltip_bg = load("Interface\\Tooltips\\UI-Tooltip-Background.blp");
    let tooltip_edge = edge_composite(&load("Interface\\Tooltips\\UI-Tooltip-Border.blp"));
    let marble = load("Interface\\PaperDoll\\UI-PaperDoll-SlotBackground.blp").crop(140, 68, 262, 362);
    let slot_bg = load(&format!("{BT}UI-Slot-Background.blp")).crop(0, 0, 41, 41);
    let empty_slot = load(&format!("{BT}UI-EmptySlot.blp")).crop(9, 9, 45, 45);
    let slot_hi = load(&format!("{BT}ButtonHilight-Square.blp")).additive(1.6);
    let btn = |l: &mut dyn FnMut(&str) -> Rgba, n: &str| l(&format!("{BT}UI-Panel-Button-{n}.blp")).crop(0, 0, 80, 22);
    let btn_up = btn(&mut load, "Up");
    let btn_down = btn(&mut load, "Down");
    let btn_hi = btn(&mut load, "Highlight").additive(1.4);
    let close = |l: &mut dyn FnMut(&str) -> Rgba, n: &str| l(&format!("{BT}UI-Panel-MinimizeButton-{n}.blp"));
    let close_up = close(&mut load, "Up");
    let close_down = close(&mut load, "Down");
    let close_hi = close(&mut load, "Highlight").additive(1.4);
    const PD: &str = "Interface\\PaperDollInfoFrame\\";
    let tab_active = load(&format!("{PD}UI-Character-ActiveTab.blp"));
    let tab_inactive = load(&format!("{PD}UI-Character-InactiveTab.blp"));
    let tab_hi = load(&format!("{PD}UI-Character-Tab-Highlight.blp")).additive(1.3);
    let check_up = load(&format!("{BT}UI-CheckBox-Up.blp"));
    let check_mark = load(&format!("{BT}UI-CheckBox-Check.blp"));
    let row_bg = load("Interface\\MerchantFrame\\UI-Merchant-LabelSlots.blp").crop(0, 0, 108, 41);
    let row_hi = load("Interface\\QuestFrame\\UI-QuestTitleHighlight.blp").additive(1.2);
    let coin = load("Interface\\MoneyFrame\\UI-MoneyIcons.blp").crop(0, 0, 16, 16);
    let doll_name = |s: EquipSlot| match s {
        EquipSlot::Weapon => "MainHand",
        EquipSlot::Armor => "Chest",
        EquipSlot::Helmet => "Head",
        EquipSlot::Shield => "SecondaryHand",
        EquipSlot::Shoes => "Feet",
        EquipSlot::Bracelet => "Wrists",
        EquipSlot::Necklace => "Neck",
        EquipSlot::Earring => "REar",
    };
    let doll: Vec<Rgba> = EquipSlot::ALL
        .iter()
        .map(|s| load(&format!("Interface\\PaperDoll\\UI-PaperDoll-Slot-{}.blp", doll_name(*s))).crop(4, 4, 56, 56))
        .collect();
    if missing.is_empty() {
        info!("skin: WoW arayüz dokuları hazır");
    } else {
        warn!("skin: {} doku bulunamadı: {:?}", missing.len(), missing);
    }
    let mut up = |r: Rgba| upload(&mut images, r);
    let doll: Vec<Handle<Image>> = doll.into_iter().map(&mut up).collect();
    commands.insert_resource(Skin {
        dialog_bg: up(dialog_bg),
        dialog_edge: up(dialog_edge),
        dialog_header: up(dialog_header),
        gold_bg: up(gold_bg),
        gold_edge: up(gold_edge),
        gold_header: up(gold_header),
        gold_dragon: up(gold_dragon),
        tooltip_bg: up(tooltip_bg),
        tooltip_edge: up(tooltip_edge),
        marble: up(marble),
        slot_bg: up(slot_bg),
        empty_slot: up(empty_slot),
        slot_hi: up(slot_hi),
        btn_up: up(btn_up),
        btn_down: up(btn_down),
        btn_hi: up(btn_hi),
        close_up: up(close_up),
        close_down: up(close_down),
        close_hi: up(close_hi),
        tab_active: up(tab_active),
        tab_inactive: up(tab_inactive),
        tab_hi: up(tab_hi),
        check_up: up(check_up),
        check_mark: up(check_mark),
        row_bg: up(row_bg),
        row_hi: up(row_hi),
        coin: up(coin),
        doll: std::array::from_fn(|i| doll[i].clone()),
    });
}

// ---------------------------------------------------------------------------------------------
// Düğüm yardımcıları
// ---------------------------------------------------------------------------------------------

/// WoW yazı gölgesi (1 px sağ-alt, koyu).
pub fn shadow() -> TextShadow {
    TextShadow { offset: Vec2::new(1.0, 1.0), color: Color::srgba(0.0, 0.0, 0.0, 0.9) }
}

fn slicer(border: f32, scale: f32) -> NodeImageMode {
    NodeImageMode::Sliced(TextureSlicer {
        border: BorderRect::all(border),
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Tile { stretch_value: scale },
        max_corner_scale: scale,
    })
}

/// Yatay 9-dilim (yalnız sol/sağ uçlar sabit): düğme, sekme, başlık plakası.
fn h_slicer(border: f32, scale: f32) -> NodeImageMode {
    NodeImageMode::Sliced(TextureSlicer {
        border: BorderRect { min_inset: Vec2::new(border, 0.0), max_inset: Vec2::new(border, 0.0) },
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: scale,
    })
}

/// Genel 9-dilim (kenar kalınlığı `border` doku pikseli, ölçek `scale`).
pub fn sliced(img: Handle<Image>, border: f32, scale: f32) -> ImageNode {
    ImageNode::new(img).with_mode(NodeImageMode::Sliced(TextureSlicer {
        border: BorderRect::all(border),
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: scale,
    }))
}

/// Bir düğümü ebeveyninin tamamına (ya da `inset` içine) yayan mutlak yerleşim.
pub fn fill(inset: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(inset),
        right: px(inset),
        top: px(inset),
        bottom: px(inset),
        ..default()
    }
}

/// Pencere çerçevesinin arka plan + kenar katmanları. Kök düğümün İLK çocukları olmalı.
pub fn frame_layers(p: &mut ChildSpawnerCommands, skin: &Skin, style: Style) {
    match style {
        Style::Dialog | Style::Gold => {
            // Koyu mermer taban + WoW DialogBox arka planı (yarı saydam siyah / altın-kahve) döşenmiş.
            p.spawn((
                fill(DIALOG_INSET),
                ImageNode::new(skin.marble.clone()).with_color(Color::srgba(1.0, 1.0, 1.0, 0.92)),
                Pickable::IGNORE,
            ));
            let bg = if style == Style::Gold { skin.gold_bg.clone() } else { skin.dialog_bg.clone() };
            p.spawn((
                fill(DIALOG_INSET),
                ImageNode::new(bg)
                    .with_color(Color::srgba(1.0, 1.0, 1.0, if style == Style::Gold { 0.75 } else { 0.45 }))
                    .with_mode(NodeImageMode::Tiled { tile_x: true, tile_y: true, stretch_value: 0.5 }),
                Pickable::IGNORE,
            ));
            let edge = if style == Style::Gold { skin.gold_edge.clone() } else { skin.dialog_edge.clone() };
            p.spawn((fill(0.0), ImageNode::new(edge).with_mode(slicer(32.0, DIALOG_SCALE)), Pickable::IGNORE));
        }
        Style::Tooltip => {
            p.spawn((
                fill(4.0),
                ImageNode::new(skin.tooltip_bg.clone())
                    .with_color(TOOLTIP_TINT)
                    .with_mode(NodeImageMode::Tiled { tile_x: true, tile_y: true, stretch_value: 0.25 }),
                Pickable::IGNORE,
            ));
            // İkinci kat: WoW tooltip'i oyunda neredeyse opak görünür.
            p.spawn((
                fill(4.0),
                ImageNode::new(skin.tooltip_bg.clone())
                    .with_color(TOOLTIP_TINT.with_alpha(0.8))
                    .with_mode(NodeImageMode::Tiled { tile_x: true, tile_y: true, stretch_value: 0.25 }),
                Pickable::IGNORE,
            ));
            p.spawn((fill(0.0), ImageNode::new(skin.tooltip_edge.clone()).with_mode(slicer(16.0, 1.0)), Pickable::IGNORE));
        }
    }
}

/// İç kutu (WoW "inset"): tooltip kenarlı, koyu zeminli; ekipman ve bilgi alanları için.
pub fn inset_layers(p: &mut ChildSpawnerCommands, skin: &Skin, marble: bool) {
    if marble {
        p.spawn((fill(3.0), ImageNode::new(skin.marble.clone()), Pickable::IGNORE));
    } else {
        p.spawn((fill(3.0), BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)), Pickable::IGNORE));
    }
    p.spawn((
        fill(0.0),
        ImageNode::new(skin.tooltip_edge.clone())
            .with_color(Color::srgb(0.6, 0.6, 0.6))
            .with_mode(slicer(16.0, 0.75)),
        Pickable::IGNORE,
    ));
}

/// WoW penceresi kök düğümü: dünya tıklamasını engeller, padding kenarın içine ayarlanır.
/// Çocukları eklerken önce [`frame_layers`] çağrılmalı (bkz. [`spawn_window`]).
pub fn window_root(node: Node) -> impl Bundle {
    (
        Node {
            padding: UiRect {
                left: px(DIALOG_PAD),
                right: px(DIALOG_PAD),
                top: px(DIALOG_PAD + 12.0),
                bottom: px(DIALOG_PAD),
            },
            ..node
        },
        RelativeCursorPosition::default(),
        BlocksWorld,
    )
}

/// Çerçeveli, başlıklı, kapat düğmeli WoW penceresi. `close` kapat düğmesine eklenecek bileşen.
pub fn spawn_window<'a>(
    commands: &'a mut Commands,
    skin: &Skin,
    ui: &UiAssets,
    style: Style,
    node: Node,
    title: &str,
    close: impl Bundle,
) -> EntityCommands<'a> {
    let mut e = commands.spawn(window_root(node));
    e.with_children(|p| {
        frame_layers(p, skin, style);
        if style == Style::Gold {
            // Altın ejderha süsü sol üst köşede (WoW: TOPLEFT -24, +18).
            p.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(-30),
                    top: px(-26),
                    width: px(96),
                    height: px(96),
                    ..default()
                },
                ImageNode::new(skin.gold_dragon.clone()),
                Pickable::IGNORE,
            ));
        }
        header(p, skin, ui, style, title);
        p.spawn((close_button(skin), close));
    });
    e
}

/// Başlık plakası (UI-DialogBox-Header / Gold-Header), pencerenin üst kenarına oturur.
pub fn header(p: &mut ChildSpawnerCommands, skin: &Skin, ui: &UiAssets, style: Style, title: &str) {
    let img = if style == Style::Gold { skin.gold_header.clone() } else { skin.dialog_header.clone() };
    p.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            right: px(0),
            top: px(-HEADER_RISE),
            justify_content: JustifyContent::Center,
            ..default()
        },
        Pickable::IGNORE,
    ))
    .with_children(|r| {
        r.spawn((
            Node {
                height: px(HEADER_H),
                min_width: px(120),
                padding: UiRect { left: px(26), right: px(26), top: px(0), bottom: px(3) },
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ImageNode::new(img).with_mode(h_slicer(24.0, HEADER_SCALE)),
            Pickable::IGNORE,
        ))
        .with_child((
            Text::new(title),
            TextFont { font: ui.title_font.clone(), font_size: 15.0, ..default() },
            TextColor(WOW_GOLD),
            shadow(),
        ));
    });
}

/// Düğme dokuları (normal/basılı; `None` = doku değişmez) — doğrudan `SkinHi` çocukları üstüne
/// gelince görünür.
#[derive(Component, Clone, Default)]
pub struct SkinButton {
    pub up: Option<Handle<Image>>,
    pub down: Option<Handle<Image>>,
}

/// Düğmenin vurgu katmanı.
#[derive(Component)]
pub struct SkinHi;

/// Vurgu katmanı (doğrudan düğme çocuğu olarak eklenir).
pub fn hi_layer(img: Handle<Image>, mode: NodeImageMode, inset: f32) -> impl Bundle {
    (fill(inset), ImageNode::new(img).with_mode(mode), Visibility::Hidden, Pickable::IGNORE, SkinHi)
}

/// WoW kapat düğmesi (UI-Panel-MinimizeButton, kırmızı X), pencerenin sağ üst köşesi.
pub fn close_button(skin: &Skin) -> impl Bundle {
    (
        Button,
        Node {
            position_type: PositionType::Absolute,
            right: px(-1),
            top: px(-1),
            width: px(30),
            height: px(30),
            ..default()
        },
        ImageNode::new(skin.close_up.clone()).with_mode(NodeImageMode::Stretch),
        SkinButton { up: Some(skin.close_up.clone()), down: Some(skin.close_down.clone()) },
        children![hi_layer(skin.close_hi.clone(), NodeImageMode::Stretch, 0.0)],
    )
}

/// Kırmızı WoW panel düğmesi (UI-Panel-Button) + yazı. Yazının bileşenleri `label` ile eklenebilir.
pub fn panel_button(skin: &Skin, ui: &UiAssets, label: &str, width: f32, height: f32, label_extra: impl Bundle) -> impl Bundle {
    let mode = h_slicer(12.0, 1.0);
    (
        Button,
        Node {
            width: px(width),
            height: px(height),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        ImageNode::new(skin.btn_up.clone()).with_mode(mode.clone()),
        SkinButton { up: Some(skin.btn_up.clone()), down: Some(skin.btn_down.clone()) },
        children![
            hi_layer(skin.btn_hi.clone(), mode, 0.0),
            (
                Text::new(label),
                TextFont { font: ui.font.clone(), font_size: 13.0, ..default() },
                TextColor(WOW_GOLD),
                shadow(),
                Pickable::IGNORE,
                label_extra,
            )
        ],
    )
}

/// Sekme (UI-Character-ActiveTab / InactiveTab). Durumu [`set_tab`] ile değişir.
#[derive(Component)]
pub struct SkinTab;

pub fn tab_button(skin: &Skin, ui: &UiAssets, label: &str, width: f32) -> impl Bundle {
    let mode = h_slicer(20.0, 0.8);
    (
        Button,
        Node {
            width: px(width),
            height: px(26),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::bottom(px(3)),
            ..default()
        },
        ImageNode::new(skin.tab_inactive.clone()).with_mode(mode.clone()),
        SkinTab,
        children![
            hi_layer(skin.tab_hi.clone(), mode, 3.0),
            (
                Text::new(label),
                TextFont { font: ui.title_font.clone(), font_size: 13.0, ..default() },
                TextColor(WOW_GOLD),
                shadow(),
                Pickable::IGNORE,
            )
        ],
    )
}

/// Sekmenin etkin/edilgin görünümü ve vurgusu.
pub fn set_tab(
    skin: &Skin,
    active: bool,
    hovered: bool,
    img: &mut ImageNode,
    children: &Children,
    his: &mut Query<&mut Visibility, With<SkinHi>>,
    texts: &mut Query<&mut TextColor>,
) {
    let want = if active { &skin.tab_active } else { &skin.tab_inactive };
    if img.image != *want {
        img.image = want.clone();
    }
    for c in children.iter() {
        if let Ok(mut v) = his.get_mut(c) {
            let w = if hovered && !active { Visibility::Inherited } else { Visibility::Hidden };
            if *v != w {
                *v = w;
            }
        }
        if let Ok(mut t) = texts.get_mut(c) {
            let w = if active { WOW_WHITE } else { TAB_DIM };
            if t.0 != w {
                t.0 = w;
            }
        }
    }
}

/// Onay kutusu (UI-CheckBox-Up + Check). İşaret görünürlüğü `SkinCheck.0` ile.
#[derive(Component)]
pub struct SkinCheck(pub bool);
#[derive(Component)]
struct SkinCheckMark;

pub fn checkbox(skin: &Skin, size: f32) -> impl Bundle {
    (
        Node { width: px(size), height: px(size), flex_shrink: 0.0, ..default() },
        ImageNode::new(skin.check_up.clone()).with_mode(NodeImageMode::Stretch),
        SkinCheck(false),
        Pickable::IGNORE,
        children![(fill(0.0), ImageNode::new(skin.check_mark.clone()), Visibility::Hidden, Pickable::IGNORE, SkinCheckMark)],
    )
}

/// Slot zemini: çanta slotu dokusu (gri kenar, koyu kanat deseni).
pub fn slot_image(skin: &Skin) -> ImageNode {
    sliced(skin.slot_bg.clone(), 8.0, 1.0)
}

/// Slot vurgusu (üstüne gelince). Görünürlüğü çağıran yönetir.
pub fn slot_hilight(skin: &Skin) -> impl Bundle {
    (fill(0.0), ImageNode::new(skin.slot_hi.clone()).with_mode(NodeImageMode::Stretch), Visibility::Hidden, Pickable::IGNORE)
}

/// Yang sikkesi (13×13, WoW MoneyFrame boyutu).
pub fn coin(skin: &Skin) -> impl Bundle {
    (
        Node { width: px(13), height: px(13), margin: UiRect::left(px(3)), flex_shrink: 0.0, ..default() },
        ImageNode::new(skin.coin.clone()),
        Pickable::IGNORE,
    )
}

// ---------------------------------------------------------------------------------------------
// Durum sistemleri
// ---------------------------------------------------------------------------------------------

fn button_states(
    mut q: Query<(&Interaction, &SkinButton, Option<&mut ImageNode>, &Children), Changed<Interaction>>,
    mut his: Query<&mut Visibility, With<SkinHi>>,
) {
    for (i, b, img, ch) in &mut q {
        if let (Some(mut img), Some(up), Some(down)) = (img, b.up.as_ref(), b.down.as_ref()) {
            let want = if *i == Interaction::Pressed { down } else { up };
            if img.image != *want {
                img.image = want.clone();
            }
        }
        let show = matches!(i, Interaction::Hovered | Interaction::Pressed);
        for c in ch.iter() {
            if let Ok(mut v) = his.get_mut(c) {
                *v = if show { Visibility::Inherited } else { Visibility::Hidden };
            }
        }
    }
}

fn check_states(
    q: Query<(&SkinCheck, &Children), Changed<SkinCheck>>,
    mut marks: Query<&mut Visibility, With<SkinCheckMark>>,
) {
    for (c, ch) in &q {
        for e in ch.iter() {
            if let Ok(mut v) = marks.get_mut(e) {
                *v = if c.0 { Visibility::Inherited } else { Visibility::Hidden };
            }
        }
    }
}
