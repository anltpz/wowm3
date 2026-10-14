//! Ana HUD — WoW 1.12 arayüz dokularıyla (MPQ Interface/), kendi çizimimiz yok denecek kadar az:
//! - Altta: WoW 1.12 MainMenuBar birebir (FrameXML MainMenuBar.xml): 1024×43 dört atlas şeridi, üstünde
//!   1024×10 EXP çubuğu, iki uçta grifonlar; eylem slotları x = 8 + 42·i (1-4 beceri, F1-F4 iksir), yardım
//!   mikro düğme yerinde, çanta sırt çantası yerinde. Küre yok (WoW'da yok).
//! - Sol üstte WoW PlayerFrame, yanında TargetFrame (PlayerFrame.xml / TargetFrame.xml konumları); çubuk
//!   üstündeki sayılar WoW'daki gibi yalnızca fare çerçevenin üstündeyken görünür.
//!   sağ üstte WoW mini harita çerçevesi (UI-Minimap-Border, ObjectIcons, MinimapArrow) + görev,
//!   sol altta sohbet kutusu, ortada duyuru; ölüm ve yardım katmanları.

use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use benilla_assets::WorldAssets;

use super::{icon, text, BlocksWorld, UiAssets, Windows, GOLD, TEXT};
use crate::game::camera::OrbitCam;
use crate::game::combat::{Dying, Enemy, Health, PlayerTarget};
use crate::game::inventory::Inventory;
use crate::game::items::{self, Item};
use crate::game::log::GameLog;
use crate::game::metin::Metin;
use crate::game::npc::Npc;
use crate::game::player::Player;
use crate::game::quest::Quest;
use crate::game::skills::{SkillCooldowns, CastSkill, SKILLS};
use crate::game::stats::{exp_needed, Derived, PlayerStats};

/// İksir slotları (F1-F4): eşya vnum'ları, envanterden sürükleyerek değiştirilir.
#[derive(Resource, Clone, serde::Serialize, serde::Deserialize)]
pub struct QuickSlots(pub [Option<u32>; 4]);

impl Default for QuickSlots {
    fn default() -> Self {
        Self([
            Some(items::V_RED_POT_S),
            Some(items::V_RED_POT_M),
            Some(items::V_RED_POT_L),
            Some(items::V_BLUE_POT_S),
        ])
    }
}

pub const QUICK_KEYS: [KeyCode; 4] = [KeyCode::F1, KeyCode::F2, KeyCode::F3, KeyCode::F4];
const QUICK_LABELS: [&str; 4] = ["F1", "F2", "F3", "F4"];

/// WoW MainMenuBar genişliği (4 × 256 şerit).
const HUD_W: f32 = 1024.0;
const BAR_X: f32 = 0.0;
/// WoW eylem slotu: 36 px, şeritte x = 8 + 42·i.
const SLOT: f32 = 36.0;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum HudText {
    FrameName,
    FrameHp,
    FrameSp,
    Exp,
    TargetName,
    TargetHp,
    TargetLevel,
    FrameLevel,
    Zone,
    Quest,
    RageTime,
    MountTime,
}

/// HUD kuruldu (güncelleme sistemleri bundan sonra çalışır).
#[derive(Resource)]
struct HudBuilt;

/// Yalnızca fare üstündeyken görünen yazı (WoW çubuk metinleri): hangi çerçeveye bağlı.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum HoverText {
    Player,
    Target,
    Exp,
}
/// Fare üstüne gelince yazılarını gösteren alan.
#[derive(Component, Clone, Copy)]
struct HoverArea(HoverText);
#[derive(Component)]
struct PlayerArrow;

#[derive(Component)]
struct FrameHpFill;
#[derive(Component)]
struct FrameSpFill;
#[derive(Component)]
struct ExpFill;
#[derive(Component)]
struct TargetFrame;
#[derive(Component)]
struct TargetFill;
#[derive(Component)]
struct LogBox;
#[derive(Component)]
struct Banner;
#[derive(Component)]
struct DeathOverlay;
#[derive(Component)]
struct HelpPanel;
#[derive(Component)]
struct Radar;
#[derive(Component)]
struct RadarDots;
#[derive(Component)]
struct RageBuff;
#[derive(Component)]
struct MountBuff;

/// İksir slotu düğümü (sürükle-bırak hedefi).
#[derive(Component)]
pub struct QuickSlotNode(pub usize);
#[derive(Component)]
struct QuickIcon(usize);
#[derive(Component)]
struct QuickCount(usize);
/// Beceri slotu (tooltip için).
#[derive(Component)]
pub struct SkillSlotNode(pub usize);
#[derive(Component)]
struct SkillShade(usize);
#[derive(Component)]
struct SkillCdText(usize);
#[derive(Component, Clone, Copy)]
enum MenuBtn {
    Inventory,
    Help,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<QuickSlots>().add_systems(
        Update,
        (
            build.run_if(resource_exists::<UiAssets>.and(resource_exists::<WorldAssets>)),
            (
                update_bars,
                hover_texts,
                update_target,
                update_log,
                update_quick,
                update_skills,
                use_quick,
                menu_buttons,
                radar,
                overlays,
            )
                .run_if(resource_exists::<HudBuilt>),
        ),
    );
}

// ---------- kurulum ----------

/// WoW eylem slotu (şerit sanatının üstüne, arka plan çizmeden).
fn slot_node(x: f32) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            left: px(BAR_X + x),
            bottom: px(4),
            width: px(SLOT),
            height: px(SLOT),
            ..default()
        },
        RelativeCursorPosition::default(),
        ZIndex(3),
    )
}

/// i. slotun x konumu (0..12, iki şerit).
fn slot_x(i: usize) -> f32 {
    let (strip, k) = (i / 6, i % 6);
    strip as f32 * 256.0 + 8.0 + 42.0 * k as f32
}

fn build(
    mut commands: Commands,
    ui: Res<UiAssets>,
    mut wa: ResMut<WorldAssets>,
    mut images: ResMut<Assets<Image>>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    *done = true;
    commands.insert_resource(HudBuilt);
    let mut ic = |p: &str| icon(&mut wa, &mut images, p).unwrap_or_default();
    let endcap = ic("Interface\\MainMenuBar\\UI-MainMenuBar-EndCap-Human.blp");
    let bar = ic("Interface\\MainMenuBar\\UI-MainMenuBar-Human.blp");
    let status = ic("Interface\\TargetingFrame\\UI-StatusBar.blp");
    let frame = ic("Interface\\TargetingFrame\\UI-TargetingFrame.blp");
    let mask = ic("Interface\\CharacterFrame\\TempPortraitAlphaMask.blp");
    let level_bg = ic("Interface\\TargetingFrame\\UI-TargetingFrame-LevelBackground.blp");
    let portrait = ic("Interface\\CharacterFrame\\TemporaryPortrait-Male-Human.blp");
    let monster = ic("Interface\\CharacterFrame\\TemporaryPortrait-Monster.blp");
    let mm_border = ic("Interface\\Minimap\\UI-Minimap-Border.blp");
    let arrow = ic("Interface\\Minimap\\MinimapArrow.blp");
    let skill_icons: Vec<Handle<Image>> = SKILLS
        .iter()
        .map(|s| ic(&format!("Interface\\Icons\\{}.blp", s.icon)))
        .collect();
    let bag = ic("Interface\\Icons\\INV_Misc_Bag_08.blp");
    let book = ic("Interface\\Icons\\INV_Misc_Book_09.blp");
    let rage = ic("Interface\\Icons\\Ability_Racial_BloodRage.blp");
    let steed = ic("Interface\\Icons\\Ability_Mount_Undeadhorse.blp");
    let atlas = |y0: f32, y1: f32| ImageNode { image: bar.clone(), rect: Some(Rect::new(0.0, y0, 256.0, y1)), ..default() };
    let abs = |left: f32, bottom: f32, w: f32, h: f32| Node {
        position_type: PositionType::Absolute,
        left: px(left),
        bottom: px(bottom),
        width: px(w),
        height: px(h),
        ..default()
    };

    // ===== Alt çubuk: WoW MainMenuBar (1024×53) =====
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(0),
                left: Val::Percent(50.0),
                margin: UiRect::left(px(-HUD_W / 2.0)),
                width: px(HUD_W),
                height: px(53),
                ..default()
            },
            RelativeCursorPosition::default(),
            BlocksWorld,
        ))
        .with_children(|bar_root| {
            // Grifonlar: sol uç BOTTOM, çubuğun sol kenarı -32; sağ uç aynalı, +32 (MainMenuBar.xml).
            let cap = |flip: bool| ImageNode { image: endcap.clone(), flip_x: flip, ..default() };
            bar_root.spawn((abs(-96.0, 0.0, 128.0, 128.0), cap(false), ZIndex(3)));
            bar_root.spawn((abs(HUD_W - 32.0, 0.0, 128.0, 128.0), cap(true), ZIndex(3)));
            // Dört şerit (MainMenuBarTexture0-3: texcoord üstleri 0.832 / 0.582 / 0.332 / 0.082).
            for (k, y0) in [212.0f32, 148.0, 84.0, 20.0].into_iter().enumerate() {
                bar_root.spawn((abs(256.0 * k as f32, 0.0, 256.0, 44.0), atlas(y0, y0 + 44.0), ZIndex(1)));
            }
            // EXP çubuğu 1024×10: UI-StatusBar (WoW mor 0.58, 0, 0.55) + üstünde atlasın XP çerçeve satırları.
            bar_root
                .spawn((
                    abs(0.0, 43.0, HUD_W, 10.0),
                    ImageNode { image: status.clone(), color: Color::srgba(0.0, 0.0, 0.0, 0.5), ..default() },
                    RelativeCursorPosition::default(),
                    HoverArea(HoverText::Exp),
                    ZIndex(1),
                ))
                .with_children(|x| {
                    x.spawn((
                        Node { position_type: PositionType::Absolute, left: px(0), top: px(0), height: Val::Percent(100.0), width: Val::Percent(0.0), ..default() },
                        ImageNode { image: status.clone(), color: Color::srgb(0.58, 0.0, 0.55), ..default() },
                        ExpFill,
                    ));
                    for (k, y0) in [192.0f32, 128.0, 64.0, 0.0].into_iter().enumerate() {
                        x.spawn((
                            Node { position_type: PositionType::Absolute, left: px(256.0 * k as f32), top: px(0), width: px(256), height: px(10), ..default() },
                            ImageNode { image: bar.clone(), rect: Some(Rect::new(0.0, y0, 256.0, y0 + 11.0)), flip_y: true, ..default() },
                        ));
                    }
                    x.spawn((
                        Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                        children![(text(&ui, "", 10.0, TEXT), wow_shadow(), HudText::Exp, HoverText::Exp, Visibility::Hidden)],
                    ));
                });
            // Slotlar 0-3: beceriler.
            for i in 0..4 {
                bar_root.spawn((slot_node(slot_x(i)), SkillSlotNode(i))).with_children(|s| {
                    s.spawn((Node { width: px(SLOT), height: px(SLOT), ..default() }, ImageNode::new(skill_icons[i].clone())));
                    s.spawn((
                        Node { position_type: PositionType::Absolute, left: px(0), bottom: px(0), width: Val::Percent(100.0), height: Val::Percent(0.0), ..default() },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                        SkillShade(i),
                    ));
                    s.spawn((
                        Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                        children![(text(&ui, "", 15.0, TEXT), wow_shadow(), SkillCdText(i))],
                    ));
                    s.spawn((
                        Node { position_type: PositionType::Absolute, right: px(2), top: px(1), ..default() },
                        text(&ui, (i + 1).to_string(), 10.0, Color::srgb(0.6, 0.6, 0.6)),
                        wow_shadow(),
                    ));
                });
            }
            // Slotlar 4-7: iksirler (F1-F4).
            for i in 0..4 {
                bar_root.spawn((slot_node(slot_x(4 + i)), QuickSlotNode(i))).with_children(|s| {
                    s.spawn((Node { width: px(SLOT), height: px(SLOT), ..default() }, ImageNode::default(), Visibility::Hidden, QuickIcon(i)));
                    s.spawn((
                        Node { position_type: PositionType::Absolute, right: px(2), top: px(1), ..default() },
                        text(&ui, QUICK_LABELS[i], 10.0, Color::srgb(0.6, 0.6, 0.6)),
                        wow_shadow(),
                    ));
                    s.spawn((
                        Node { position_type: PositionType::Absolute, right: px(2), bottom: px(0), ..default() },
                        text(&ui, "", 12.0, TEXT),
                        wow_shadow(),
                        QuickCount(i),
                    ));
                });
            }
            // Yardım: ilk mikro düğme yerinde (x 552); çanta: sırt çantası yerinde (sağdan 6, 37×37).
            bar_root.spawn((
                Node { position_type: PositionType::Absolute, left: px(552), bottom: px(4), width: px(SLOT), height: px(SLOT), ..default() },
                Button,
                ImageNode::new(book),
                MenuBtn::Help,
                ZIndex(3),
            ));
            bar_root.spawn((
                Node { position_type: PositionType::Absolute, left: px(HUD_W - 6.0 - 37.0), bottom: px(3), width: px(37), height: px(37), ..default() },
                Button,
                ImageNode::new(bag),
                MenuBtn::Inventory,
                ZIndex(3),
            ));
        });

    // ===== Güçlendirmeler: WoW BuffFrame (sağ üst, mini haritanın solu) =====
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            right: px(MM_PX + 8.0),
            top: px(13),
            column_gap: px(4),
            ..default()
        })
        .with_children(|r| {
            for (img, marker, label) in [(rage.clone(), true, HudText::RageTime), (steed.clone(), false, HudText::MountTime)] {
                let mut e = r.spawn((
                    Node { width: px(32), height: px(32), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() },
                    Visibility::Hidden,
                ));
                if marker {
                    e.insert(RageBuff);
                } else {
                    e.insert(MountBuff);
                }
                e.with_children(|b| {
                    b.spawn((Node { width: px(32), height: px(32), ..default() }, ImageNode::new(img)));
                    b.spawn((text(&ui, "", 10.0, GOLD), wow_shadow(), label));
                });
            }
        });

    // ===== Sol üst: WoW PlayerFrame (232×100, UI-TargetingFrame aynalı) =====
    unit_frame(&mut commands, &ui, UnitFrame {
        left: -19.0,
        art: ImageNode { image: frame.clone(), rect: Some(Rect::new(24.0, 0.0, 256.0, 100.0)), flip_x: true, ..default() },
        portrait: portrait.clone(),
        portrait_x: 42.0,
        bars_x: 106.0,
        level_x: 41.0,
        status: status.clone(),
        name_bg: None,
        player: true,
    });

    // ===== Üst: WoW TargetFrame =====
    unit_frame(&mut commands, &ui, UnitFrame {
        left: 250.0,
        art: ImageNode { image: frame.clone(), rect: Some(Rect::new(24.0, 0.0, 256.0, 100.0)), ..default() },
        portrait: monster.clone(),
        portrait_x: 126.0,
        bars_x: 7.0,
        level_x: 167.0,
        status: status.clone(),
        name_bg: Some(level_bg.clone()),
        player: false,
    });

    // ===== Sağ üst: WoW mini harita (UI-Minimap-Border 240 px) + görev =====
    // Dokuda halka merkezi (168.5, 104), iç yarıçap ≈64; bölge şeridi x 83-255, y 1-26.
    let k = MM_PX / 256.0;
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            right: px(0),
            top: px(0),
            width: px(MM_PX),
            height: px(MM_PX),
            ..default()
        })
        .with_children(|m| {
            let r = 64.0 * k;
            m.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(168.5 * k - r),
                    top: px(104.0 * k - r),
                    width: px(2.0 * r),
                    height: px(2.0 * r),
                    border_radius: BorderRadius::all(px(r)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                ImageNode { image: mask.clone(), color: Color::srgba(0.1, 0.17, 0.07, 0.9), ..default() },
                Radar,
            ))
            .with_children(|c| {
                c.spawn((
                    Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                    RadarDots,
                ));
                c.spawn((
                    Node { position_type: PositionType::Absolute, left: px(r - 8.0), top: px(r - 8.0), width: px(16), height: px(16), ..default() },
                    ImageNode { image: arrow.clone(), rect: Some(Rect::new(0.0, 0.0, 32.0, 32.0)), ..default() },
                    UiTransform::default(),
                    ZIndex(2),
                    PlayerArrow,
                ));
            });
            m.spawn((Node { position_type: PositionType::Absolute, width: px(MM_PX), height: px(MM_PX), ..default() }, ImageNode::new(mm_border.clone()), ZIndex(3)));
            m.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(86.0 * k),
                    top: px(3.0 * k),
                    width: px(140.0 * k),
                    height: px(20.0 * k),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ZIndex(4),
                children![(text(&ui, "", 11.0, GOLD), TextShadow::default(), HudText::Zone)],
            ));
            m.spawn((
                Node { position_type: PositionType::Absolute, right: px(10), top: px(190.0 * k), max_width: px(240), padding: UiRect::all(px(6)), ..default() },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
                children![(text(&ui, "", 13.0, TEXT), TextShadow::default(), HudText::Quest)],
            ));
        });

    // ===== Sol alt: sohbet kutusu =====
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(8),
            bottom: px(140),
            width: px(430),
            min_height: px(120),
            padding: UiRect::all(px(6)),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::FlexEnd,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.3)),
        LogBox,
    ));

    // ===== Duyuru =====
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(22.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            Text::new(""),
            TextFont { font: ui.title_font.clone(), font_size: 40.0, ..default() },
            TextColor(GOLD),
            TextShadow::default(),
            Banner,
        )],
    ));

    // ===== Ölüm =====
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            row_gap: px(12),
            ..default()
        },
        BackgroundColor(Color::srgba(0.25, 0.0, 0.0, 0.35)),
        Visibility::Hidden,
        DeathOverlay,
        children![
            (
                Text::new("Öldün"),
                TextFont { font: ui.title_font.clone(), font_size: 56.0, ..default() },
                TextColor(Color::srgb(1.0, 0.3, 0.25)),
                TextShadow::default(),
            ),
            text(&ui, "Köyde yeniden doğmak için R tuşuna bas", 18.0, TEXT),
        ],
    ));

    // ===== Yardım =====
    commands.spawn((
        super::panel(Node {
            position_type: PositionType::Absolute,
            top: px(90),
            left: px(10),
            padding: UiRect::all(px(10)),
            flex_direction: FlexDirection::Column,
            row_gap: px(3),
            ..default()
        }),
        HelpPanel,
        children![
            text(&ui, "Kontroller (H: gizle/göster)", 15.0, GOLD),
            text(&ui, "W/S: ileri-geri   A/D: dön   Q/E: yan adım   Space: zıpla", 13.0, TEXT),
            text(&ui, "Sağ tık sürükle: kamera + karakter   Sol tık sürükle: serbest bakış", 13.0, TEXT),
            text(&ui, "İki fare tuşu: ileri koş   Num Lock: otomatik koşu   Tekerlek: yakınlaş", 13.0, TEXT),
            text(&ui, "Sol tık: hedefle   Sağ tık düşmana: saldır   Tab: en yakın düşman", 13.0, TEXT),
            text(&ui, "1-4: beceriler   F1-F4: iksirler   Z: yerdeki eşyaları topla", 13.0, TEXT),
            text(&ui, "B: çantalar (sağ tık: kuşan/kullan)   Ctrl+G: binek", 13.0, TEXT),
            text(&ui, "NPC'ye sağ tık: konuş (Demirci: artı basma, Tüccar: alışveriş)", 13.0, TEXT),
            text(&ui, "R: (ölünce) yeniden doğ   Esc: pencere kapat / hedefi bırak", 13.0, TEXT),
        ],
    ));
}

/// Mini harita dokusunun ekrandaki boyu.
const MM_PX: f32 = 240.0;

/// WoW yazı gölgesi (GameFont'lar: 1 px siyah).
fn wow_shadow() -> TextShadow {
    TextShadow { offset: Vec2::new(1.0, 1.0), color: Color::BLACK }
}

/// WoW birim çerçevesi (PlayerFrame.xml / TargetFrame.xml, 232×100): yarı saydam siyah arka plan
/// (119×41), portre (64×64), çerçeve dokusu, ad (GameFontNormalSmall, altın), seviye, UI-StatusBar can
/// (yeşil) ve mana (mavi) çubukları (119×12). Hedefte adın arkasında düşmanlık rengi şerit. Çubuk sayıları
/// yalnızca fare üstündeyken.
struct UnitFrame {
    left: f32,
    art: ImageNode,
    portrait: Handle<Image>,
    portrait_x: f32,
    bars_x: f32,
    level_x: f32,
    status: Handle<Image>,
    name_bg: Option<Handle<Image>>,
    player: bool,
}

fn unit_frame(commands: &mut Commands, ui: &UiAssets, f: UnitFrame) {
    let at = |x: f32, y: f32, w: f32, h: f32| Node {
        position_type: PositionType::Absolute,
        left: px(x),
        top: px(y),
        width: px(w),
        height: px(h),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };
    let hover = if f.player { HoverText::Player } else { HoverText::Target };
    let mut root = commands.spawn((
        Node { position_type: PositionType::Absolute, left: px(f.left), top: px(4), width: px(232), height: px(100), ..default() },
        RelativeCursorPosition::default(),
        HoverArea(hover),
    ));
    if !f.player {
        root.insert((Visibility::Hidden, TargetFrame));
    }
    root.with_children(|c| {
        c.spawn((at(f.bars_x, 22.0, 119.0, 41.0), BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)), ZIndex(0)));
        if let Some(bg) = &f.name_bg {
            c.spawn((at(f.bars_x, 22.0, 119.0, 19.0), ImageNode { image: bg.clone(), color: Color::srgb(1.0, 0.0, 0.0), ..default() }, TargetNameBg, ZIndex(0)));
        }
        c.spawn((
            Node { border_radius: BorderRadius::all(px(32)), overflow: Overflow::clip(), ..at(f.portrait_x, 12.0, 64.0, 64.0) },
            ZIndex(0),
            children![(Node { width: px(64), height: px(64), ..default() }, ImageNode::new(f.portrait.clone()))],
        ));
        let bar = |y: f32| (at(f.bars_x, y, 119.0, 12.0), ZIndex(0));
        let fill = |col: Color| {
            (
                Node { position_type: PositionType::Absolute, left: px(0), top: px(0), width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                ImageNode { image: f.status.clone(), color: col, ..default() },
            )
        };
        if f.player {
            c.spawn(bar(41.0)).with_children(|b| {
                b.spawn((fill(Color::srgb(0.0, 1.0, 0.0)), FrameHpFill));
            });
            c.spawn(bar(52.0)).with_children(|b| {
                b.spawn((fill(Color::srgb(0.0, 0.0, 1.0)), FrameSpFill));
            });
        } else {
            c.spawn(bar(41.0)).with_children(|b| {
                b.spawn((fill(Color::srgb(0.0, 1.0, 0.0)), TargetFill));
            });
        }
        c.spawn((at(0.0, 0.0, 232.0, 100.0), f.art.clone(), ZIndex(1), TargetArtIf(!f.player)));
        let (name, hp_text, level) = if f.player {
            (HudText::FrameName, HudText::FrameHp, HudText::FrameLevel)
        } else {
            (HudText::TargetName, HudText::TargetHp, HudText::TargetLevel)
        };
        let gold = Color::srgb(1.0, 0.82, 0.0);
        c.spawn((at(f.bars_x, 22.0, 119.0, 19.0), ZIndex(2), children![(text(ui, "", 11.0, gold), wow_shadow(), name)]));
        c.spawn((at(f.bars_x, 41.0, 119.0, 12.0), ZIndex(2), children![(text(ui, "", 9.0, TEXT), wow_shadow(), hp_text, hover, Visibility::Hidden)]));
        if f.player {
            c.spawn((at(f.bars_x, 52.0, 119.0, 12.0), ZIndex(2), children![(text(ui, "", 9.0, TEXT), wow_shadow(), HudText::FrameSp, hover, Visibility::Hidden)]));
        }
        c.spawn((at(f.level_x, 58.0, 24.0, 16.0), ZIndex(2), children![(text(ui, "", 10.0, gold), wow_shadow(), level)]));
    });
}

/// Hedef adının arkasındaki düşmanlık rengi şerit (TargetFrameNameBackground).
#[derive(Component)]
struct TargetNameBg;

/// WoW: çubuk sayıları yalnızca fare çerçevenin üstündeyken.
fn hover_texts(areas: Query<(&HoverArea, &RelativeCursorPosition)>, mut texts: Query<(&HoverText, &mut Visibility)>) {
    let over = |h: HoverText| areas.iter().any(|(a, r)| a.0 == h && r.cursor_over());
    let (p, t, x) = (over(HoverText::Player), over(HoverText::Target), over(HoverText::Exp));
    for (h, mut v) in &mut texts {
        let on = match h {
            HoverText::Player => p,
            HoverText::Target => t,
            HoverText::Exp => x,
        };
        let want = if on { Visibility::Inherited } else { Visibility::Hidden };
        if *v != want {
            *v = want;
        }
    }
}

/// Hedef çerçevesinin dokusu (metin taşında Elite çerçevesi).
#[derive(Component)]
struct TargetArtIf(bool);

// ---------- güncellemeler ----------

fn update_bars(
    time: Res<Time>,
    stats: Res<PlayerStats>,
    d: Res<Derived>,
    inv: Res<Inventory>,
    quest: Res<Quest>,
    mut texts: Query<(&HudText, &mut Text)>,
    mut hp: Query<&mut Node, (With<FrameHpFill>, Without<FrameSpFill>, Without<ExpFill>)>,
    mut sp: Query<&mut Node, (With<FrameSpFill>, Without<FrameHpFill>, Without<ExpFill>)>,
    mut exp: Query<&mut Node, (With<ExpFill>, Without<FrameHpFill>, Without<FrameSpFill>)>,
    mut rage: Query<&mut Visibility, (With<RageBuff>, Without<MountBuff>)>,
    mut steed: Query<&mut Visibility, (With<MountBuff>, Without<RageBuff>)>,
) {
    let now = time.elapsed_secs();
    let hpf = (stats.hp.max(0) as f32 / d.max_hp.max(1) as f32).clamp(0.0, 1.0);
    let spf = (stats.sp.max(0) as f32 / d.max_sp.max(1) as f32).clamp(0.0, 1.0);
    let ef = stats.exp as f32 / exp_needed(stats.level).max(1) as f32;
    if let Ok(mut n) = hp.single_mut() {
        n.width = Val::Percent(hpf * 100.0);
    }
    if let Ok(mut n) = sp.single_mut() {
        n.width = Val::Percent(spf * 100.0);
    }
    if let Ok(mut n) = exp.single_mut() {
        n.width = Val::Percent(ef.clamp(0.0, 1.0) * 100.0);
    }

    let raging = now < stats.rage_until;
    if let Ok(mut v) = rage.single_mut() {
        *v = if raging { Visibility::Inherited } else { Visibility::Hidden };
    }
    if let Ok(mut v) = steed.single_mut() {
        *v = if stats.riding { Visibility::Inherited } else { Visibility::Hidden };
    }
    for (t, mut s) in &mut texts {
        let v = match t {
            HudText::FrameName => stats.name.clone(),
            HudText::FrameLevel => stats.level.to_string(),
            HudText::FrameHp => format!("{} / {}", stats.hp.max(0), d.max_hp),
            HudText::FrameSp => format!("{} / {}", stats.sp.max(0), d.max_sp),
            HudText::Exp => format!("EXP %{:.1}", ef * 100.0),
            // WoW mini harita şeridine sığan kısa ad.
            HudText::Zone => if crate::game::terrain::data().is_some() { "Pyungmoo".to_string() } else { "Elwynn Forest".to_string() },
            HudText::Quest => quest.text(),
            HudText::RageTime => {
                if raging { format!("{:.0}", stats.rage_until - now) } else { String::new() }
            }
            HudText::MountTime => {
                if !stats.riding {
                    String::new()
                } else if crate::game::mount::has_medal(&stats, &inv) {
                    "∞".to_string()
                } else {
                    crate::game::mount::clock(stats.mount_left)
                }
            }
            _ => continue,
        };
        if s.0 != v {
            s.0 = v;
        }
    }
}

fn update_target(
    target: Res<PlayerTarget>,
    enemies: Query<(&Enemy, &Health, Option<&crate::game::mobs::Mob>)>,
    mut name_bg: Query<&mut ImageNode, (With<TargetNameBg>, Without<TargetArtIf>)>,
    stats: Res<PlayerStats>,
    (mut wa, mut images, mut art): (Option<ResMut<WorldAssets>>, ResMut<Assets<Image>>, Query<(&TargetArtIf, &mut ImageNode)>),
    mut frame: Query<&mut Visibility, With<TargetFrame>>,
    mut fill: Query<&mut Node, With<TargetFill>>,
    mut texts: Query<(&HudText, &mut Text, &mut TextColor)>,
) {
    let Ok(mut vis) = frame.single_mut() else { return };
    let Some((en, hp, mob)) = target.0.and_then(|t| enemies.get(t).ok()) else {
        *vis = Visibility::Hidden;
        return;
    };
    *vis = Visibility::Inherited;
    // WoW tepki rengi: saldırgan kırmızı, pasif (saldırılınca karşılık veren) sarı.
    let hostile = en.is_metin || mob.is_some_and(|m| m.def.aggressive || m.aggro);
    if let Ok(mut bg) = name_bg.single_mut() {
        bg.color = if hostile { Color::srgb(1.0, 0.0, 0.0) } else { Color::srgb(1.0, 1.0, 0.0) };
    }
    if let Some(wa) = wa.as_deref_mut() {
        let path = if en.is_metin {
            "Interface\\TargetingFrame\\UI-TargetingFrame-Elite.blp"
        } else {
            "Interface\\TargetingFrame\\UI-TargetingFrame.blp"
        };
        if let Some(h) = icon(wa, &mut images, path) {
            for (t, mut img) in &mut art {
                if t.0 && img.image != h {
                    img.image = h.clone();
                }
            }
        }
    }
    if let Ok(mut n) = fill.single_mut() {
        n.width = Val::Percent(hp.hp.max(0) as f32 / hp.max.max(1) as f32 * 100.0);
    }
    for (t, mut s, mut c) in &mut texts {
        match t {
            HudText::TargetLevel => s.0 = en.level.to_string(),
            HudText::TargetName => {
                s.0 = en.name.to_string();
                c.0 = if en.is_metin { Color::srgb(1.0, 0.35, 0.3) } else { name_color(en, stats.level) };
            }
            HudText::TargetHp => s.0 = format!("{} / {}", hp.hp.max(0), hp.max),
            _ => {}
        }
    }
}

/// İsim rengi: metin taşı mor, güçlü düşman kırmızı, zayıf yeşil.
pub fn name_color(en: &Enemy, my_level: u32) -> Color {
    if en.is_metin {
        return Color::srgb(0.85, 0.6, 1.0);
    }
    let diff = en.level as i32 - my_level as i32;
    if diff >= 5 {
        Color::srgb(1.0, 0.3, 0.25)
    } else if diff >= 2 {
        Color::srgb(1.0, 0.7, 0.3)
    } else if diff >= -3 {
        Color::srgb(1.0, 1.0, 0.85)
    } else {
        Color::srgb(0.55, 0.9, 0.55)
    }
}

fn update_log(
    mut commands: Commands,
    time: Res<Time>,
    log: Res<GameLog>,
    ui: Res<UiAssets>,
    q: Query<Entity, With<LogBox>>,
    mut banner: Query<(&mut Text, &mut TextColor), With<Banner>>,
    mut last: Local<(u64, f32)>,
) {
    let now = time.elapsed_secs();
    if let Ok((mut t, mut c)) = banner.single_mut() {
        match &log.banner {
            Some((s, col, at)) if now - at < 3.5 => {
                if t.0 != *s {
                    t.0 = s.clone();
                }
                let a = (1.0 - ((now - at - 2.5).max(0.0))).clamp(0.0, 1.0);
                c.0 = col.with_alpha(a);
            }
            _ => {
                if !t.0.is_empty() {
                    t.0.clear();
                }
            }
        }
    }
    if log.rev == last.0 && now - last.1 < 1.0 {
        return;
    }
    *last = (log.rev, now);
    let Ok(boxe) = q.single() else { return };
    commands.entity(boxe).despawn_children();
    let lines: Vec<_> = log.lines.iter().rev().filter(|l| now - l.at < 30.0).take(7).collect();
    commands.entity(boxe).with_children(|p| {
        for l in lines.into_iter().rev() {
            let a = (1.0 - (now - l.at - 25.0).max(0.0) / 5.0).clamp(0.0, 1.0);
            p.spawn((
                Text::new(if l.system { format!("[Sistem] {}", l.text) } else { l.text.clone() }),
                TextFont { font: ui.font.clone(), font_size: 14.0, ..default() },
                TextColor(l.color.with_alpha(a)),
                TextShadow::default(),
            ));
        }
    });
}

fn update_quick(
    quick: Res<QuickSlots>,
    inv: Res<Inventory>,
    mut wa: Option<ResMut<WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    mut icons: Query<(&QuickIcon, &mut ImageNode, &mut Visibility)>,
    mut counts: Query<(&QuickCount, &mut Text)>,
) {
    if !quick.is_changed() && !inv.is_changed() {
        return;
    }
    let Some(wa) = wa.as_deref_mut() else { return };
    for (q, mut img, mut vis) in &mut icons {
        match quick.0[q.0] {
            Some(v) => {
                if let Some(h) = icon(wa, &mut images, &Item::new(v).icon_path()) {
                    img.image = h;
                    *vis = Visibility::Inherited;
                }
                img.color = if inv.count(v) > 0 { Color::WHITE } else { Color::srgba(0.4, 0.4, 0.4, 0.8) };
            }
            None => *vis = Visibility::Hidden,
        }
    }
    for (q, mut t) in &mut counts {
        t.0 = match quick.0[q.0] {
            Some(v) => {
                let n = inv.count(v);
                if n > 0 { n.to_string() } else { String::new() }
            }
            None => String::new(),
        };
    }
}

fn update_skills(
    time: Res<Time>,
    cds: Res<SkillCooldowns>,
    stats: Res<PlayerStats>,
    mut shades: Query<(&SkillShade, &mut Node, &mut BackgroundColor)>,
    mut texts: Query<(&SkillCdText, &mut Text)>,
    buttons: Res<ButtonInput<MouseButton>>,
    slots: Query<(&SkillSlotNode, &RelativeCursorPosition)>,
    mut cast: MessageWriter<CastSkill>,
) {
    let now = time.elapsed_secs();
    for (s, mut n, mut bg) in &mut shades {
        let locked = stats.level < SKILLS[s.0].level;
        let f = if locked { 1.0 } else { cds.frac(s.0, now) };
        n.height = Val::Percent(f * 100.0);
        bg.0 = if locked || stats.sp < SKILLS[s.0].sp {
            Color::srgba(0.0, 0.0, 0.0, 0.75)
        } else {
            Color::srgba(0.0, 0.0, 0.0, 0.6)
        };
    }
    for (s, mut t) in &mut texts {
        let rem = cds.0[s.0] - now;
        let v = if stats.level < SKILLS[s.0].level {
            format!("Lv{}", SKILLS[s.0].level)
        } else if rem > 0.0 {
            format!("{:.0}", rem.ceil())
        } else {
            String::new()
        };
        if t.0 != v {
            t.0 = v;
        }
    }
    if buttons.just_pressed(MouseButton::Left) {
        for (s, r) in &slots {
            if r.cursor_over() {
                cast.write(CastSkill(s.0));
            }
        }
    }
}

fn use_quick(
    keys: Res<ButtonInput<KeyCode>>,
    quick: Res<QuickSlots>,
    mut usage: MessageWriter<super::inventory_ui::UseItem>,
    inv: Res<Inventory>,
) {
    for (i, k) in QUICK_KEYS.iter().enumerate() {
        if keys.just_pressed(*k) {
            if let Some(v) = quick.0[i] {
                if let Some(cell) = inv.find(v) {
                    usage.write(super::inventory_ui::UseItem(cell));
                }
            }
        }
    }
}

fn menu_buttons(mut q: Query<(&MenuBtn, &Interaction, &mut ImageNode), Changed<Interaction>>, mut w: ResMut<Windows>) {
    for (b, i, mut img) in &mut q {
        img.color = if *i == Interaction::None { Color::WHITE } else { Color::srgb(1.3, 1.2, 0.9) };
        if *i == Interaction::Pressed {
            match b {
                MenuBtn::Inventory => w.inventory = !w.inventory,
                MenuBtn::Help => w.help = !w.help,
            }
        }
    }
}

/// Mini harita: oyuncu ortada, kamera yönü yukarı. İşaretler WoW ObjectIcons (32 px hücreler:
/// 1 kırmızı = düşman, 3 sarı = NPC, 0 altın = metin taşı); oyuncu MinimapArrow, yönüne döner.
fn radar(
    mut commands: Commands,
    time: Res<Time>,
    dots: Query<Entity, With<RadarDots>>,
    player: Query<&Transform, With<Player>>,
    cam: Query<&OrbitCam>,
    enemies: Query<(&Transform, Option<&Metin>), (With<Enemy>, Without<Dying>)>,
    npcs: Query<&Transform, With<Npc>>,
    mut arrow: Query<&mut UiTransform, With<PlayerArrow>>,
    (mut wa, mut images, mut acc): (Option<ResMut<WorldAssets>>, ResMut<Assets<Image>>, Local<f32>),
) {
    let (Ok(root), Ok(p)) = (dots.single(), player.single()) else { return };
    let yaw = cam.single().map(|c| c.yaw).unwrap_or(0.0);
    if let Ok(mut t) = arrow.single_mut() {
        // Ok dokusu yukarı bakar; harita kamera yönüne göre döndüğü için fark kadar dön.
        let rel = crate::game::player::yaw_of(p.rotation) - yaw;
        t.rotation = Rot2::radians(-rel);
    }
    *acc += time.delta_secs();
    if *acc < 0.15 {
        return;
    }
    *acc = 0.0;
    let Some(wa) = wa.as_deref_mut() else { return };
    let Some(icons) = icon(wa, &mut images, "Interface\\Minimap\\ObjectIcons.blp") else { return };
    let rot = Quat::from_rotation_y(-yaw);
    const RANGE: f32 = 80.0;
    let r = 64.0 * MM_PX / 256.0;
    commands.entity(root).despawn_children();
    let put = |c: &mut ChildSpawnerCommands, world: Vec3, size: f32, cell: u32| {
        let d = rot * (world - p.translation);
        let v = Vec2::new(d.x, d.z) / RANGE * r;
        if v.length() > r - 4.0 {
            return;
        }
        let (cx, cy) = ((cell % 4) as f32 * 32.0, (cell / 4) as f32 * 32.0);
        c.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(r + v.x - size / 2.0),
                top: px(r + v.y - size / 2.0),
                width: px(size),
                height: px(size),
                ..default()
            },
            ImageNode { image: icons.clone(), rect: Some(Rect::new(cx, cy, cx + 32.0, cy + 32.0)), ..default() },
        ));
    };
    commands.entity(root).with_children(|c| {
        for (xf, m) in &enemies {
            if m.is_some() {
                put(c, xf.translation, 14.0, 0);
            } else {
                put(c, xf.translation, 9.0, 1);
            }
        }
        for xf in &npcs {
            put(c, xf.translation, 10.0, 3);
        }
    });
}

fn overlays(
    stats: Res<PlayerStats>,
    w: Res<Windows>,
    mut death: Query<&mut Visibility, (With<DeathOverlay>, Without<HelpPanel>)>,
    mut help: Query<&mut Visibility, (With<HelpPanel>, Without<DeathOverlay>)>,
) {
    if let Ok(mut v) = death.single_mut() {
        *v = if stats.dead { Visibility::Inherited } else { Visibility::Hidden };
    }
    if let Ok(mut v) = help.single_mut() {
        *v = if w.help { Visibility::Inherited } else { Visibility::Hidden };
    }
}
