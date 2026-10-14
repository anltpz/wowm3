//! Ana HUD (Metin2/WoW karışımı tasarım):
//! - Altta: solda kırmızı HP küresi, sağda mavi SP küresi (prosedürel kürelər + grifon süsleri),
//!   ortada taş çubuk: üstte etkin güçlendirmeler, altta 1-4 beceri + F1-F4 iksir slotları,
//!   menü düğmeleri ve ince EXP çubuğu.
//! - Sol üstte portre + ad/seviye + HP/SP; üst ortada hedef çubuğu; sağ üstte radar, bölge adı
//!   ve görev takibi; sol altta sohbet/sistem kutusu; ortada duyuru; ölüm ve yardım katmanları.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::RelativeCursorPosition;

use benilla_assets::WorldAssets;

use super::{icon, text, thousands, BlocksWorld, UiAssets, Windows, GOLD, SLOT_BG, SLOT_BORDER, TEXT, TEXT_DIM};
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

const ORB: u32 = 128;
const ORB_PX: f32 = 118.0;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum HudText {
    HpOrb,
    SpOrb,
    FrameName,
    FrameHp,
    FrameSp,
    Exp,
    Yang,
    TargetName,
    TargetHp,
    Zone,
    Quest,
    RageTime,
    MountTime,
}

#[derive(Resource)]
struct OrbImages {
    hp: Handle<Image>,
    sp: Handle<Image>,
    last: (f32, f32),
}

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
                update_orbs,
                update_bars,
                update_target,
                update_log,
                update_quick,
                update_skills,
                use_quick,
                menu_buttons,
                radar,
                overlays,
            )
                .run_if(resource_exists::<OrbImages>),
        ),
    );
}

// ---------- prosedürel küre ----------

fn orb_pixels(frac: f32, base: [f32; 3], t: f32) -> Vec<u8> {
    let n = ORB as usize;
    let mut px = vec![0u8; n * n * 4];
    let inner = 0.84;
    for y in 0..n {
        for x in 0..n {
            let fx = (x as f32 + 0.5) / n as f32 * 2.0 - 1.0;
            let fy = (y as f32 + 0.5) / n as f32 * 2.0 - 1.0;
            let r = (fx * fx + fy * fy).sqrt();
            let i = (y * n + x) * 4;
            if r > 1.0 {
                continue;
            }
            let mut c;
            let mut a = 1.0f32;
            if r > inner {
                // Metal halka: altın-bronz, kabartmalı.
                let k = (r - inner) / (1.0 - inner);
                let bevel = 1.0 - (k * 2.0 - 1.0).abs();
                let light = 0.5 + 0.5 * (-fx * 0.6 - fy * 0.8);
                let g = 0.25 + 0.55 * bevel * light;
                c = [g * 1.05, g * 0.82, g * 0.45];
                if r > 0.985 {
                    a = (1.0 - r) / 0.015;
                }
            } else {
                let lvl = 1.0 - (fy + inner) / (2.0 * inner); // 0 altta, 1 üstte
                let wave = (fx * 9.0 + t * 2.5).sin() * 0.012;
                let rr = r / inner;
                let shade = (1.0 - rr * rr).max(0.0).sqrt();
                if lvl < frac + wave {
                    let depth = 0.45 + 0.55 * lvl.min(frac) / frac.max(0.05);
                    let k = (0.25 + 0.75 * shade) * (0.6 + 0.4 * depth);
                    c = [base[0] * k, base[1] * k, base[2] * k];
                    // Yüzey çizgisi parlak.
                    if (lvl - frac - wave).abs() < 0.018 && frac < 0.995 {
                        c = [c[0] + 0.25, c[1] + 0.2, c[2] + 0.2];
                    }
                } else {
                    let k = 0.05 + 0.08 * shade;
                    c = [k + base[0] * 0.04, k * 0.9 + base[1] * 0.04, k * 0.9 + base[2] * 0.04];
                }
                // Cam parlaması.
                let hl = (-((fx + 0.32).powi(2) / 0.05 + (fy + 0.42).powi(2) / 0.018)).exp() * 0.55;
                let rim = rr.powi(8) * 0.25;
                c = [c[0] + hl + rim * 0.3, c[1] + hl + rim * 0.3, c[2] + hl + rim * 0.35];
            }
            px[i] = (c[0].clamp(0.0, 1.0) * 255.0) as u8;
            px[i + 1] = (c[1].clamp(0.0, 1.0) * 255.0) as u8;
            px[i + 2] = (c[2].clamp(0.0, 1.0) * 255.0) as u8;
            px[i + 3] = (a.clamp(0.0, 1.0) * 255.0) as u8;
        }
    }
    px
}

const HP_COL: [f32; 3] = [0.95, 0.08, 0.05];
const SP_COL: [f32; 3] = [0.1, 0.3, 1.0];

fn orb_image(frac: f32, base: [f32; 3]) -> Image {
    Image::new(
        Extent3d {
            width: ORB,
            height: ORB,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        orb_pixels(frac, base, 0.0),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

// ---------- kurulum ----------

fn slot_box(size: f32) -> impl Bundle {
    (
        Node {
            width: px(size),
            height: px(size),
            border: UiRect::all(px(1)),
            ..default()
        },
        BackgroundColor(SLOT_BG),
        BorderColor::all(Color::srgb(0.45, 0.36, 0.2)),
        RelativeCursorPosition::default(),
    )
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
    let hp_img = images.add(orb_image(1.0, HP_COL));
    let sp_img = images.add(orb_image(1.0, SP_COL));
    commands.insert_resource(OrbImages {
        hp: hp_img.clone(),
        sp: sp_img.clone(),
        last: (-1.0, -1.0),
    });
    let mut ic = |p: &str| icon(&mut wa, &mut images, p).unwrap_or_default();
    let endcap = ic("Interface\\MainMenuBar\\UI-MainMenuBar-EndCap-Human.blp");
    let portrait = ic("Interface\\CharacterFrame\\TemporaryPortrait-Male-Human.blp");
    let skill_icons: Vec<Handle<Image>> = SKILLS
        .iter()
        .map(|s| ic(&format!("Interface\\Icons\\{}.blp", s.icon)))
        .collect();
    let bag = ic("Interface\\Icons\\INV_Misc_Bag_08.blp");
    let book = ic("Interface\\Icons\\INV_Misc_Book_09.blp");
    let rage = ic("Interface\\Icons\\Ability_Racial_BloodRage.blp");
    let steed = ic("Interface\\Icons\\Ability_Mount_Undeadhorse.blp");

    // ===== Alt çubuk =====
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(0),
                left: Val::Percent(50.0),
                margin: UiRect::left(px(-470)),
                width: px(940),
                height: px(132),
                align_items: AlignItems::FlexEnd,
                ..default()
            },
            RelativeCursorPosition::default(),
            BlocksWorld,
        ))
        .with_children(|bar| {
            // Sol süs + HP küresi
            bar.spawn((
                Node { width: px(96), height: px(96), margin: UiRect::right(px(-40)), ..default() },
                ImageNode::new(endcap.clone()),
                ZIndex(0),
            ));
            orb(bar, &ui, hp_img.clone(), HudText::HpOrb);
            // Orta taş panel
            bar.spawn((
                Node {
                    flex_grow: 1.0,
                    height: px(92),
                    margin: UiRect::horizontal(px(-14)),
                    padding: UiRect::new(px(22), px(22), px(6), px(4)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    border: UiRect::all(px(2)),
                    border_radius: BorderRadius::top(px(10)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.09, 0.08, 0.07, 0.94)),
                BorderColor::all(Color::srgb(0.5, 0.4, 0.24)),
                ZIndex(-1),
            ))
            .with_children(|mid| {
                // Üst sıra: güçlendirmeler
                mid.spawn(Node { column_gap: px(4), height: px(30), align_items: AlignItems::Center, ..default() })
                    .with_children(|r| {
                        r.spawn((
                            Node { width: px(28), height: px(28), border: UiRect::all(px(1)), ..default() },
                            ImageNode::new(rage),
                            BorderColor::all(Color::srgb(0.9, 0.3, 0.2)),
                            Visibility::Hidden,
                            RageBuff,
                        ))
                        .with_child((
                            Node { position_type: PositionType::Absolute, bottom: px(-2), right: px(1), ..default() },
                            text(&ui, "", 11.0, TEXT),
                            TextShadow::default(),
                            HudText::RageTime,
                        ));
                        r.spawn((
                            Node { width: px(28), height: px(28), border: UiRect::all(px(1)), ..default() },
                            ImageNode::new(steed),
                            BorderColor::all(Color::srgb(0.45, 0.75, 1.0)),
                            Visibility::Hidden,
                            MountBuff,
                        ))
                        .with_child((
                            Node { position_type: PositionType::Absolute, bottom: px(-2), right: px(1), ..default() },
                            text(&ui, "", 11.0, TEXT),
                            TextShadow::default(),
                            HudText::MountTime,
                        ));
                        r.spawn((Node { flex_grow: 1.0, ..default() },));
                        r.spawn((text(&ui, "", 13.0, GOLD), HudText::Yang));
                    });
                // Alt sıra: beceriler + iksirler + menü
                mid.spawn(Node { column_gap: px(4), align_items: AlignItems::Center, ..default() })
                    .with_children(|r| {
                        for i in 0..4 {
                            r.spawn((slot_box(42.0), SkillSlotNode(i))).with_children(|s| {
                                s.spawn((Node { width: px(40), height: px(40), ..default() }, ImageNode::new(skill_icons[i].clone())));
                                s.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: px(0),
                                        bottom: px(0),
                                        width: Val::Percent(100.0),
                                        height: Val::Percent(0.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                                    SkillShade(i),
                                ));
                                s.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        width: Val::Percent(100.0),
                                        height: Val::Percent(100.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    children![(text(&ui, "", 15.0, TEXT), TextShadow::default(), SkillCdText(i))],
                                ));
                                s.spawn((
                                    Node { position_type: PositionType::Absolute, left: px(2), top: px(0), ..default() },
                                    text(&ui, (i + 1).to_string(), 11.0, GOLD),
                                    TextShadow::default(),
                                ));
                            });
                        }
                        r.spawn(Node { width: px(8), ..default() });
                        for i in 0..4 {
                            r.spawn((slot_box(42.0), QuickSlotNode(i))).with_children(|s| {
                                s.spawn((
                                    Node { width: px(40), height: px(40), ..default() },
                                    ImageNode::default(),
                                    Visibility::Hidden,
                                    QuickIcon(i),
                                ));
                                s.spawn((
                                    Node { position_type: PositionType::Absolute, left: px(2), top: px(0), ..default() },
                                    text(&ui, QUICK_LABELS[i], 10.0, TEXT_DIM),
                                    TextShadow::default(),
                                ));
                                s.spawn((
                                    Node { position_type: PositionType::Absolute, right: px(2), bottom: px(0), ..default() },
                                    text(&ui, "", 12.0, TEXT),
                                    TextShadow::default(),
                                    QuickCount(i),
                                ));
                            });
                        }
                        r.spawn(Node { flex_grow: 1.0, ..default() });
                        for (b, img) in [(MenuBtn::Inventory, bag), (MenuBtn::Help, book)] {
                            r.spawn((
                                Button,
                                Node { width: px(30), height: px(30), border: UiRect::all(px(1)), ..default() },
                                BorderColor::all(SLOT_BORDER),
                                ImageNode::new(img),
                                b,
                            ));
                        }
                    });
                // EXP çubuğu
                mid.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: px(7),
                        border: UiRect::all(px(1)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.9)),
                    BorderColor::all(Color::srgb(0.3, 0.25, 0.15)),
                ))
                .with_children(|e| {
                    e.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(0),
                            top: px(0),
                            height: Val::Percent(100.0),
                            width: Val::Percent(0.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.65, 0.4, 1.0)),
                        ExpFill,
                    ));
                });
                mid.spawn((
                    Node { position_type: PositionType::Absolute, bottom: px(-1), right: px(24), ..default() },
                    text(&ui, "", 10.0, TEXT_DIM),
                    HudText::Exp,
                ));
            });
            orb(bar, &ui, sp_img.clone(), HudText::SpOrb);
            bar.spawn((
                Node { width: px(96), height: px(96), margin: UiRect::left(px(-40)), ..default() },
                ImageNode { image: endcap, flip_x: true, ..default() },
                ZIndex(0),
            ));
        });

    // ===== Sol üst: oyuncu çerçevesi =====
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: px(10),
            top: px(10),
            align_items: AlignItems::Center,
            column_gap: px(6),
            ..default()
        })
        .with_children(|f| {
            f.spawn((
                Node {
                    width: px(64),
                    height: px(64),
                    border: UiRect::all(px(3)),
                    border_radius: BorderRadius::all(px(32)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BorderColor::all(Color::srgb(0.75, 0.6, 0.3)),
                BackgroundColor(Color::BLACK),
                children![(Node { width: px(58), height: px(58), ..default() }, ImageNode::new(portrait))],
            ));
            f.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(3), ..default() })
                .with_children(|c| {
                    c.spawn((
                        Node {
                            padding: UiRect::new(px(8), px(8), px(1), px(1)),
                            border: UiRect::all(px(1)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.05, 0.04, 0.03, 0.85)),
                        BorderColor::all(Color::srgb(0.5, 0.4, 0.22)),
                        children![(text(&ui, "", 14.0, TEXT), HudText::FrameName)],
                    ));
                    c.spawn(frame_bar(150.0, 12.0, Color::srgb(0.1, 0.75, 0.15), FrameHpFill, HudText::FrameHp, &ui));
                    c.spawn(frame_bar(150.0, 9.0, Color::srgb(0.12, 0.3, 0.9), FrameSpFill, HudText::FrameSp, &ui));
                });
        });

    // ===== Üst orta: hedef =====
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(8),
                left: Val::Percent(50.0),
                margin: UiRect::left(px(-160)),
                width: px(320),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(2),
                ..default()
            },
            Visibility::Hidden,
            TargetFrame,
        ))
        .with_children(|p| {
            p.spawn((text(&ui, "", 16.0, TEXT), TextShadow::default(), HudText::TargetName));
            p.spawn((
                Node {
                    width: px(300),
                    height: px(14),
                    border: UiRect::all(px(2)),
                    border_radius: BorderRadius::all(px(3)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
                BorderColor::all(Color::srgb(0.45, 0.35, 0.2)),
            ))
            .with_children(|b| {
                b.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        top: px(0),
                        height: Val::Percent(100.0),
                        width: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.85, 0.08, 0.06)),
                    TargetFill,
                ));
                b.spawn((text(&ui, "", 10.0, TEXT), TextShadow::default(), HudText::TargetHp));
            });
        });

    // ===== Sağ üst: radar + bölge + görev =====
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            right: px(10),
            top: px(8),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexEnd,
            row_gap: px(4),
            ..default()
        })
        .with_children(|r| {
            r.spawn((
                Node { padding: UiRect::new(px(8), px(8), px(2), px(2)), border: UiRect::all(px(1)), ..default() },
                BackgroundColor(Color::srgba(0.05, 0.04, 0.03, 0.85)),
                BorderColor::all(Color::srgb(0.55, 0.45, 0.25)),
                children![(text(&ui, "", 12.0, GOLD), HudText::Zone)],
            ));
            r.spawn((
                Node {
                    width: px(150),
                    height: px(150),
                    border: UiRect::all(px(4)),
                    border_radius: BorderRadius::all(px(75)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.12, 0.22, 0.08, 0.85)),
                BorderColor::all(Color::srgb(0.7, 0.56, 0.3)),
                Radar,
            ))
            .with_children(|m| {
                m.spawn((
                    Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                    RadarDots,
                ));
            });
            r.spawn((
                Node {
                    padding: UiRect::all(px(6)),
                    margin: UiRect::top(px(4)),
                    max_width: px(240),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
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
            border: UiRect::all(px(1)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
        BorderColor::all(Color::srgba(0.4, 0.33, 0.2, 0.6)),
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

fn orb(parent: &mut ChildSpawnerCommands, ui: &UiAssets, img: Handle<Image>, label: HudText) {
    parent
        .spawn((
            Node {
                width: px(ORB_PX),
                height: px(ORB_PX),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::bottom(px(6)),
                ..default()
            },
            ImageNode::new(img),
            ZIndex(1),
        ))
        .with_child((text(ui, "", 13.0, TEXT), TextShadow::default(), label));
}

fn frame_bar(w: f32, h: f32, fill: Color, marker: impl Bundle, label: HudText, ui: &UiAssets) -> impl Bundle {
    (
        Node {
            width: px(w),
            height: px(h),
            border: UiRect::all(px(1)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
        BorderColor::all(Color::srgb(0.35, 0.3, 0.18)),
        children![
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    top: px(0),
                    height: Val::Percent(100.0),
                    width: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(fill),
                marker,
            ),
            (text(ui, "", h - 1.0, TEXT), TextShadow::default(), label),
        ],
    )
}

// ---------- güncellemeler ----------

fn update_orbs(
    time: Res<Time>,
    stats: Res<PlayerStats>,
    d: Res<Derived>,
    mut orbs: ResMut<OrbImages>,
    mut images: ResMut<Assets<Image>>,
    mut tick: Local<f32>,
) {
    let hpf = (stats.hp.max(0) as f32 / d.max_hp.max(1) as f32).clamp(0.0, 1.0);
    let spf = (stats.sp.max(0) as f32 / d.max_sp.max(1) as f32).clamp(0.0, 1.0);
    *tick += time.delta_secs();
    // Dalga animasyonu için ~15 fps, değer değişince hemen.
    let changed = (hpf - orbs.last.0).abs() > 0.003 || (spf - orbs.last.1).abs() > 0.003;
    if !changed && *tick < 0.066 {
        return;
    }
    *tick = 0.0;
    orbs.last = (hpf, spf);
    let t = time.elapsed_secs();
    if let Some(img) = images.get_mut(&orbs.hp) {
        img.data = Some(orb_pixels(hpf, HP_COL, t));
    }
    if let Some(img) = images.get_mut(&orbs.sp) {
        img.data = Some(orb_pixels(spf, SP_COL, t + 1.3));
    }
}

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
            HudText::HpOrb => format!("{}\n/{}", stats.hp.max(0), d.max_hp),
            HudText::SpOrb => format!("{}\n/{}", stats.sp.max(0), d.max_sp),
            HudText::FrameName => format!("{}  Lv {}", stats.name, stats.level),
            HudText::FrameHp => format!("{} / {}", stats.hp.max(0), d.max_hp),
            HudText::FrameSp => format!("{} / {}", stats.sp.max(0), d.max_sp),
            HudText::Exp => format!("EXP %{:.1}", ef * 100.0),
            HudText::Yang => format!("{} Yang", thousands(inv.yang)),
            HudText::Zone => if crate::game::terrain::data().is_some() { "PYUNGMOO (Jinno - Mavi Bayrak)".to_string() } else { "ELWYNN FOREST (Metin Bölgesi)".to_string() },
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
    enemies: Query<(&Enemy, &Health)>,
    stats: Res<PlayerStats>,
    mut frame: Query<&mut Visibility, With<TargetFrame>>,
    mut fill: Query<&mut Node, With<TargetFill>>,
    mut texts: Query<(&HudText, &mut Text, &mut TextColor)>,
) {
    let Ok(mut vis) = frame.single_mut() else { return };
    let Some((en, hp)) = target.0.and_then(|t| enemies.get(t).ok()) else {
        *vis = Visibility::Hidden;
        return;
    };
    *vis = Visibility::Inherited;
    if let Ok(mut n) = fill.single_mut() {
        n.width = Val::Percent(hp.hp.max(0) as f32 / hp.max.max(1) as f32 * 100.0);
    }
    for (t, mut s, mut c) in &mut texts {
        match t {
            HudText::TargetName => {
                s.0 = format!("{} (Lv. {})", en.name, en.level);
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

fn menu_buttons(mut q: Query<(&MenuBtn, &Interaction, &mut BorderColor), Changed<Interaction>>, mut w: ResMut<Windows>) {
    for (b, i, mut bc) in &mut q {
        *bc = BorderColor::all(if *i == Interaction::None { SLOT_BORDER } else { GOLD });
        if *i == Interaction::Pressed {
            match b {
                MenuBtn::Inventory => w.inventory = !w.inventory,
                MenuBtn::Help => w.help = !w.help,
            }
        }
    }
}

/// Radar: oyuncu ortada, kamera yönü yukarı; metin mor, mob kırmızı, NPC sarı.
fn radar(
    mut commands: Commands,
    time: Res<Time>,
    dots: Query<Entity, With<RadarDots>>,
    player: Query<&Transform, With<Player>>,
    cam: Query<&OrbitCam>,
    enemies: Query<(&Transform, Option<&Metin>), (With<Enemy>, Without<Dying>)>,
    npcs: Query<&Transform, With<Npc>>,
    mut acc: Local<f32>,
) {
    *acc += time.delta_secs();
    if *acc < 0.15 {
        return;
    }
    *acc = 0.0;
    let (Ok(root), Ok(p)) = (dots.single(), player.single()) else { return };
    let yaw = cam.single().map(|c| c.yaw).unwrap_or(0.0);
    let rot = Quat::from_rotation_y(-yaw);
    const RANGE: f32 = 80.0;
    const R: f32 = 71.0;
    commands.entity(root).despawn_children();
    let mut put = |c: &mut ChildSpawnerCommands, world: Vec3, size: f32, color: Color| {
        let d = rot * (world - p.translation);
        let v = Vec2::new(d.x, d.z) / RANGE * R;
        if v.length() > R - 3.0 {
            return;
        }
        c.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(R + v.x - size / 2.0),
                top: px(R + v.y - size / 2.0),
                width: px(size),
                height: px(size),
                border_radius: BorderRadius::all(px(size / 2.0)),
                ..default()
            },
            BackgroundColor(color),
        ));
    };
    commands.entity(root).with_children(|c| {
        for (xf, m) in &enemies {
            if m.is_some() {
                put(c, xf.translation, 9.0, Color::srgb(0.85, 0.3, 1.0));
            } else {
                put(c, xf.translation, 4.0, Color::srgb(0.95, 0.2, 0.15));
            }
        }
        for xf in &npcs {
            put(c, xf.translation, 7.0, Color::srgb(1.0, 0.9, 0.2));
        }
        put(c, p.translation, 8.0, Color::WHITE);
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
