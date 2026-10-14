//! Satıcı pencereleri (WoW eşyaları): Tüccar iksir ve parşömen, Silah Satıcısı silah, Zırh Satıcısı zırh
//! ve takı satar. Her satıcının kendi penceresi var; `Windows::shop_kind` hangisinin görüneceğini seçer.
//! Satış: eşyayı pencereye sürükle ya da pencere açıkken envanterde sağ tıkla.

use bevy::prelude::*;

use benilla_assets::WorldAssets;

use super::inventory_ui::SellItem;
use super::skin::{self, Skin, Style};
use super::{icon, text, thousands, ShopKind, UiAssets, Windows, TEXT, TEXT_DIM};
use crate::game::inventory::Inventory;
use crate::game::items::{self, Item};
use crate::game::log::{GameLog, C_BAD, C_ITEM};

/// Satılan mallar: (vnum, adet, fiyat).
pub const GENERAL: &[(u32, u16, u32)] = &[
    (items::V_RED_POT_S, 10, 200),
    (items::V_RED_POT_M, 10, 700),
    (items::V_RED_POT_L, 5, 1500),
    (items::V_BLUE_POT_S, 10, 200),
    (items::V_BLESSING, 1, 6000),
    (items::V_HANDBOOK, 1, 15000),
];

/// Silah Satıcısı: başlangıç ve orta seviye WoW silahları.
pub const WEAPONS: &[(u32, u16, u32)] = &[
    (10, 1, 300),
    (15, 1, 700),
    (20, 1, 1200),
    (25, 1, 2000),
    (30, 1, 3500),
    (35, 1, 5000),
    (40, 1, 8000),
    (45, 1, 11000),
    (50, 1, 15000),
];

/// Zırh Satıcısı: zırh setleri ve takılar.
pub const ARMOR: &[(u32, u16, u32)] = &[
    (11200, 1, 600),
    (11210, 1, 12000),
    (12200, 1, 400),
    (13000, 1, 450),
    (15000, 1, 300),
    (14000, 1, 350),
    (16000, 1, 350),
    (17000, 1, 350),
    (12210, 1, 9000),
    (15010, 1, 11000),
];

fn stock(kind: ShopKind) -> &'static [(u32, u16, u32)] {
    match kind {
        ShopKind::General => GENERAL,
        ShopKind::Weapons => WEAPONS,
        ShopKind::Armor => ARMOR,
    }
}

/// Satıcı penceresi (türüne göre).
#[derive(Component)]
pub struct ShopWindow(ShopKind);
#[derive(Component)]
struct BuyButton(ShopKind, usize);
#[derive(Component)]
struct CloseShop;

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            build.run_if(resource_exists::<UiAssets>.and(resource_exists::<WorldAssets>).and(resource_exists::<Skin>)),
            (show_hide, buy, sell).run_if(resource_exists::<UiAssets>),
        ),
    );
}

fn build(
    mut commands: Commands,
    ui: Res<UiAssets>,
    skin: Res<Skin>,
    mut wa: ResMut<WorldAssets>,
    mut images: ResMut<Assets<Image>>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    *done = true;
    for (kind, title) in [
        (ShopKind::General, "Tüccar — İksir ve Parşömen"),
        (ShopKind::Weapons, "Silah Satıcısı"),
        (ShopKind::Armor, "Zırh Satıcısı"),
    ] {
    skin::spawn_window(
        &mut commands,
        &skin,
        &ui,
        Style::Dialog,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: px(100),
            margin: UiRect::left(px(-380)),
            width: px(380),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(3),
            ..default()
        },
        title,
        CloseShop,
    )
    .insert((Visibility::Hidden, ShopWindow(kind)))
    .with_children(|w| {
        for (i, (v, n, price)) in stock(kind).iter().enumerate() {
            let it = Item::stack(*v, *n);
            let img = icon(&mut wa, &mut images, &it.icon_path()).unwrap_or_default();
            // WoW satıcı satırı: altın kenarlı ikon yuvası + ad etiketi + fiyat (sikke) + Satın Al.
            w.spawn(Node {
                width: Val::Percent(100.0),
                height: px(32),
                column_gap: px(6),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|r| {
                r.spawn((
                    Node {
                        width: px(30),
                        height: px(30),
                        flex_shrink: 0.0,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    skin::sliced(skin.empty_slot.clone(), 10.0, 0.6),
                ))
                .with_child((Node { width: px(24), height: px(24), ..default() }, ImageNode::new(img)));
                r.spawn((
                    Node {
                        flex_grow: 1.0,
                        height: px(28),
                        padding: UiRect::horizontal(px(8)),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    skin::sliced(skin.row_bg.clone(), 8.0, 1.0),
                ))
                .with_children(|l| {
                    let name = if *n > 1 { format!("{} x{}", it.def().name, n) } else { it.def().name.to_string() };
                    l.spawn((Node { flex_grow: 1.0, ..default() }, children![(text(&ui, name, 13.0, TEXT), skin::shadow())]));
                    l.spawn((text(&ui, thousands(*price as u64), 12.0, TEXT), skin::shadow()));
                    l.spawn(skin::coin(&skin));
                });
                r.spawn((skin::panel_button(&skin, &ui, "Satın Al", 72.0, 22.0, ()), BuyButton(kind, i)));
            });
        }
        w.spawn(Node { height: px(4), ..default() });
        w.spawn(text(&ui, "Satmak için: eşyayı buraya sürükle ya da sağ tıkla", 11.0, TEXT_DIM));
        w.spawn((skin::panel_button(&skin, &ui, "Kapat", 96.0, 22.0, ()), CloseShop));
    });
    }
}

fn show_hide(w: Res<Windows>, mut q: Query<(&ShopWindow, &mut Visibility)>) {
    for (sw, mut v) in &mut q {
        let want = if w.shop && w.shop_kind == sw.0 { Visibility::Inherited } else { Visibility::Hidden };
        if *v != want {
            *v = want;
        }
    }
}

fn buy(
    time: Res<Time>,
    q: Query<(&Interaction, Option<&BuyButton>, Option<&CloseShop>), Changed<Interaction>>,
    mut inv: ResMut<Inventory>,
    mut w: ResMut<Windows>,
    mut log: ResMut<GameLog>,
) {
    let now = time.elapsed_secs();
    for (i, b, close) in &q {
        if *i != Interaction::Pressed {
            continue;
        }
        if close.is_some() {
            w.shop = false;
            continue;
        }
        let Some(b) = b else { continue };
        let (v, n, price) = stock(b.0)[b.1];
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
