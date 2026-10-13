//! Tüccar penceresi: iksir, kağıt ve başlangıç ekipmanı satar. Satış: eşyayı pencereye sürükle
//! ya da pencere açıkken envanterde sağ tıkla.

use bevy::prelude::*;

use benilla_assets::WorldAssets;

use super::inventory_ui::SellItem;
use super::{icon, panel, text, thousands, UiAssets, Windows, GOLD, SLOT_BG, SLOT_BORDER, TEXT, TEXT_DIM, TITLE_BG};
use crate::game::inventory::Inventory;
use crate::game::items::{self, Item};
use crate::game::log::{GameLog, C_BAD, C_ITEM};

/// Satılan mallar: (vnum, adet, fiyat).
pub const GOODS: &[(u32, u16, u32)] = &[
    (items::V_RED_POT_S, 10, 200),
    (items::V_RED_POT_M, 10, 700),
    (items::V_RED_POT_L, 5, 1500),
    (items::V_BLUE_POT_S, 10, 200),
    (items::V_BLESSING, 1, 6000),
    (items::V_HANDBOOK, 1, 15000),
    (20, 1, 1200),
    (12200, 1, 400),
    (13000, 1, 450),
    (15000, 1, 300),
    (16000, 1, 350),
    (14000, 1, 350),
];

#[derive(Component)]
pub struct ShopWindow;
#[derive(Component)]
struct BuyButton(usize);
#[derive(Component)]
struct CloseShop;

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            build.run_if(resource_exists::<UiAssets>.and(resource_exists::<WorldAssets>)),
            (show_hide, buy, sell).run_if(resource_exists::<UiAssets>),
        ),
    );
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
    commands
        .spawn((
            panel(Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: px(100),
                margin: UiRect::left(px(-360)),
                width: px(360),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::bottom(px(8)),
                row_gap: px(3),
                ..default()
            }),
            Visibility::Hidden,
            ShopWindow,
        ))
        .with_children(|w| {
            w.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: px(26),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(TITLE_BG),
            ))
            .with_child(text(&ui, "Tüccar — Genel Mağaza", 15.0, GOLD));
            for (i, (v, n, price)) in GOODS.iter().enumerate() {
                let it = Item::stack(*v, *n);
                let img = icon(&mut wa, &mut images, &it.icon_path()).unwrap_or_default();
                w.spawn((
                    Button,
                    Node {
                        width: px(340),
                        height: px(30),
                        border: UiRect::all(px(1)),
                        padding: UiRect::horizontal(px(4)),
                        column_gap: px(8),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(SLOT_BG),
                    BorderColor::all(SLOT_BORDER),
                    BuyButton(i),
                ))
                .with_children(|r| {
                    r.spawn((Node { width: px(24), height: px(24), ..default() }, ImageNode::new(img)));
                    let name = if *n > 1 { format!("{} x{}", it.def().name, n) } else { it.def().name.to_string() };
                    r.spawn((Node { flex_grow: 1.0, ..default() }, children![text(&ui, name, 13.0, TEXT)]));
                    r.spawn(text(&ui, format!("{} Yang", thousands(*price as u64)), 12.0, GOLD));
                });
            }
            w.spawn(text(&ui, "Satmak için: eşyayı buraya sürükle ya da sağ tıkla", 11.0, TEXT_DIM));
            w.spawn((
                Button,
                Node {
                    width: px(110),
                    height: px(26),
                    border: UiRect::all(px(1)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.3, 0.2, 0.08)),
                BorderColor::all(Color::srgb(0.8, 0.62, 0.3)),
                CloseShop,
            ))
            .with_child(text(&ui, "Kapat", 14.0, GOLD));
        });
}

fn show_hide(w: Res<Windows>, mut q: Query<&mut Visibility, With<ShopWindow>>) {
    if let Ok(mut v) = q.single_mut() {
        let want = if w.shop { Visibility::Inherited } else { Visibility::Hidden };
        if *v != want {
            *v = want;
        }
    }
}

fn buy(
    time: Res<Time>,
    mut q: Query<(&Interaction, &mut BackgroundColor, Option<&BuyButton>, Option<&CloseShop>), Changed<Interaction>>,
    mut inv: ResMut<Inventory>,
    mut w: ResMut<Windows>,
    mut log: ResMut<GameLog>,
) {
    let now = time.elapsed_secs();
    for (i, mut bg, b, close) in &mut q {
        bg.0 = match i {
            Interaction::Hovered => Color::srgb(0.25, 0.18, 0.08),
            Interaction::Pressed => Color::srgb(0.4, 0.28, 0.1),
            Interaction::None if close.is_some() => Color::srgb(0.3, 0.2, 0.08),
            Interaction::None => SLOT_BG,
        };
        if *i != Interaction::Pressed {
            continue;
        }
        if close.is_some() {
            w.shop = false;
            continue;
        }
        let Some(b) = b else { continue };
        let (v, n, price) = GOODS[b.0];
        if inv.yang < price as u64 {
            log.push(now, C_BAD, "Yeterli Yang yok.");
            continue;
        }
        let it = Item::stack(v, n);
        match inv.add(it) {
            None => {
                inv.yang -= price as u64;
                log.push(now, C_ITEM, format!("{} satın aldın.", it.def().name));
            }
            Some(left) => {
                // Sığmayan kısmı geri al.
                inv.consume(v, (n - left.count) as u32);
                log.push(now, C_BAD, "Envanterde yer yok.");
            }
        }
    }
}

fn sell(time: Res<Time>, mut ev: MessageReader<SellItem>, mut inv: ResMut<Inventory>, mut log: ResMut<GameLog>) {
    for SellItem(c) in ev.read() {
        let Some(h) = inv.head_of(*c) else { continue };
        let Some(it) = inv.take(h) else { continue };
        let value = it.def().price as u64 * it.count as u64 * (1 + it.plus as u64);
        inv.yang += value;
        log.push(
            time.elapsed_secs(),
            C_ITEM,
            format!("{} satıldı: +{} Yang", it.display_name(), thousands(value)),
        );
    }
}
