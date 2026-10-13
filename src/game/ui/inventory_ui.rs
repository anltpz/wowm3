//! Envanter penceresi (Metin2 düzeni): üstte ekipman yuvaları, altta 5×9 ızgara + I/II sayfa
//! sekmeleri, en altta Yang. Sürükle-bırak (hücre↔hücre, hücre→ekipman, →hızlı slot, →demirci,
//! →mağaza), sağ tık kuşan/kullan/çıkar, Shift+sürükle yığın böl.

use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use bevy::window::PrimaryWindow;

use benilla_assets::WorldAssets;

use super::hud::{QuickSlotNode, QuickSlots};
use super::{icon, panel, text, thousands, UiAssets, Windows, GOLD, SLOT_BG, SLOT_BORDER, SLOT_HOVER, TEXT, TEXT_DIM, TITLE_BG};
use crate::game::inventory::{index, Inventory, COLS, ROWS};
use crate::game::items::{EquipSlot, Item, Kind};
use crate::game::log::{GameLog, C_BAD, C_GOOD, C_INFO};
use crate::game::player::PlayerLook;
use crate::game::stats::{Derived, PlayerStats};

pub const PITCH: f32 = 34.0;
pub const SLOT: f32 = 32.0;

/// Hücredeki eşyayı kullan (iksir) ya da kuşan.
#[derive(Message, Clone, Copy)]
pub struct UseItem(pub usize);

/// İmlecin altındaki şey (tooltip ve bırakma hedefi).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Spot {
    Cell(usize),
    Equip(EquipSlot),
    Quick(usize),
    Upgrade,
    Shop,
}

#[derive(Resource, Default)]
pub struct Hovered(pub Option<Spot>);

#[derive(Resource, Default)]
pub struct InvPage(pub usize);

#[derive(Clone, Copy)]
pub struct DragState {
    pub from: Spot,
    pub item: Item,
    pub split: bool,
}

#[derive(Resource, Default)]
pub struct Drag(pub Option<DragState>);

/// Demirci penceresine bırakılan hücre (upgrade_ui okur).
#[derive(Message, Clone, Copy)]
pub struct DropToUpgrade(pub usize);

/// Mağazaya satış.
#[derive(Message, Clone, Copy)]
pub struct SellItem(pub usize);

#[derive(Component)]
struct InvWindow;
#[derive(Component)]
struct GridArea;
#[derive(Component)]
struct ItemLayer;
#[derive(Component)]
pub struct EquipNode(pub EquipSlot);
#[derive(Component)]
struct EquipIcon(EquipSlot);
#[derive(Component)]
struct PageTab(usize);
#[derive(Component)]
struct YangText;
#[derive(Component)]
struct Ghost;
#[derive(Component)]
struct CellBg(usize);

/// Pencere sağ üstte; bu düzen sabitleri tooltip/ghost hizası için de kullanılır.
const EQUIP_LAYOUT: [(EquipSlot, f32, f32, f32); 8] = [
    (EquipSlot::Earring, 14.0, 6.0, 1.0),
    (EquipSlot::Helmet, 77.0, 6.0, 1.0),
    (EquipSlot::Necklace, 140.0, 6.0, 1.0),
    (EquipSlot::Weapon, 14.0, 46.0, 2.0),
    (EquipSlot::Armor, 77.0, 46.0, 2.0),
    (EquipSlot::Shield, 140.0, 46.0, 2.0),
    (EquipSlot::Bracelet, 14.0, 120.0, 1.0),
    (EquipSlot::Shoes, 77.0, 120.0, 1.0),
];

pub fn plugin(app: &mut App) {
    app.add_message::<UseItem>()
        .add_message::<DropToUpgrade>()
        .add_message::<SellItem>()
        .init_resource::<Hovered>()
        .init_resource::<InvPage>()
        .init_resource::<Drag>()
        .add_systems(Startup, starter_kit)
        .add_systems(
            Update,
            (
                build.run_if(resource_added::<UiAssets>),
                (
                    show_hide,
                    page_tabs,
                    hover_and_mouse,
                    redraw_items,
                    ghost_follow,
                    use_items,
                    sync_look,
                )
                    .chain()
                    .run_if(resource_exists::<UiAssets>),
            ),
        );
}

/// Yeni oyun başlangıç eşyaları (kayıt yoksa).
fn starter_kit(mut inv: ResMut<Inventory>) {
    if inv.equip.iter().any(|e| e.is_some()) || inv.find(crate::game::items::V_RED_POT_S).is_some() {
        return;
    }
    inv.equip[EquipSlot::Weapon.index()] = Some(Item::new(10));
    inv.equip[EquipSlot::Armor.index()] = Some(Item::new(11200));
    inv.add(Item::stack(crate::game::items::V_RED_POT_S, 30));
    inv.add(Item::stack(crate::game::items::V_BLUE_POT_S, 10));
    inv.add(Item::new(12200));
    inv.add(Item::new(15000));
    inv.add(Item::stack(crate::game::items::V_BLESSING, 1));
    inv.yang = 2000;
    inv.touch();
}

fn build(mut commands: Commands, ui: Res<UiAssets>) {
    let grid_w = COLS as f32 * PITCH - 2.0;
    let grid_h = ROWS as f32 * PITCH - 2.0;
    commands
        .spawn((
            panel(Node {
                position_type: PositionType::Absolute,
                right: px(10),
                top: px(262),
                width: px(grid_w + 20.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::bottom(px(8)),
                row_gap: px(6),
                ..default()
            }),
            Visibility::Hidden,
            InvWindow,
        ))
        .with_children(|w| {
            // Başlık
            w.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: px(24),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(TITLE_BG),
            ))
            .with_child(text(&ui, "Envanter", 15.0, GOLD));
            // Ekipman alanı
            w.spawn((
                Node {
                    width: px(grid_w),
                    height: px(156),
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.12, 0.09, 0.06, 0.8)),
                BorderColor::all(SLOT_BORDER),
            ))
            .with_children(|e| {
                for (slot, x, y, h) in EQUIP_LAYOUT {
                    let hh = SLOT * h + (h - 1.0) * 2.0;
                    e.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(x),
                            top: px(y),
                            width: px(SLOT),
                            height: px(hh),
                            border: UiRect::all(px(1)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(SLOT_BG),
                        BorderColor::all(SLOT_BORDER),
                        RelativeCursorPosition::default(),
                        EquipNode(slot),
                    ))
                    .with_children(|s| {
                        s.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            children![text(&ui, slot.name(), 8.0, TEXT_DIM)],
                        ));
                        s.spawn((
                            Node {
                                width: px(SLOT - 2.0),
                                height: px(hh - 2.0),
                                ..default()
                            },
                            ImageNode::default(),
                            Visibility::Hidden,
                            EquipIcon(slot),
                        ));
                    });
                }
            });
            // Sayfa sekmeleri
            w.spawn(Node {
                column_gap: px(6),
                ..default()
            })
            .with_children(|t| {
                for (i, label) in ["I", "II"].iter().enumerate() {
                    t.spawn((
                        Button,
                        Node {
                            width: px(44),
                            height: px(20),
                            border: UiRect::all(px(1)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(SLOT_BG),
                        BorderColor::all(SLOT_BORDER),
                        PageTab(i),
                    ))
                    .with_child(text(&ui, *label, 13.0, TEXT));
                }
            });
            // Izgara
            w.spawn((
                Node {
                    width: px(grid_w),
                    height: px(grid_h),
                    ..default()
                },
                RelativeCursorPosition::default(),
                GridArea,
            ))
            .with_children(|g| {
                for r in 0..ROWS {
                    for c in 0..COLS {
                        g.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: px(c as f32 * PITCH),
                                top: px(r as f32 * PITCH),
                                width: px(SLOT),
                                height: px(SLOT),
                                border: UiRect::all(px(1)),
                                ..default()
                            },
                            BackgroundColor(SLOT_BG),
                            BorderColor::all(SLOT_BORDER),
                            CellBg(r * COLS + c),
                        ));
                    }
                }
                g.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        top: px(0),
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    ItemLayer,
                ));
            });
            // Yang
            w.spawn(Node {
                width: px(grid_w),
                justify_content: JustifyContent::FlexEnd,
                ..default()
            })
            .with_child((text(&ui, "", 14.0, GOLD), YangText));
        });

    // Sürüklenen ikon.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: px(SLOT),
            height: px(SLOT),
            ..default()
        },
        ImageNode::default(),
        Visibility::Hidden,
        GlobalZIndex(100),
        Ghost,
    ));
}

fn show_hide(w: Res<Windows>, mut q: Query<&mut Visibility, With<InvWindow>>) {
    if let Ok(mut v) = q.single_mut() {
        let want = if w.inventory { Visibility::Inherited } else { Visibility::Hidden };
        if *v != want {
            *v = want;
        }
    }
}

fn page_tabs(
    mut page: ResMut<InvPage>,
    mut q: Query<(&PageTab, &Interaction, &mut BackgroundColor), With<Button>>,
) {
    for (t, i, mut bg) in &mut q {
        if *i == Interaction::Pressed && page.0 != t.0 {
            page.0 = t.0;
        }
        bg.0 = if page.0 == t.0 {
            Color::srgb(0.35, 0.26, 0.12)
        } else if *i == Interaction::Hovered {
            Color::srgb(0.2, 0.15, 0.08)
        } else {
            SLOT_BG
        };
    }
}

fn hover_and_mouse(
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    w: Res<Windows>,
    page: Res<InvPage>,
    grid: Query<(&RelativeCursorPosition, &InheritedVisibility), With<GridArea>>,
    equips: Query<(&RelativeCursorPosition, &EquipNode, &InheritedVisibility)>,
    quicks: Query<(&RelativeCursorPosition, &QuickSlotNode)>,
    (upgrade, shop): (
        Query<(&RelativeCursorPosition, &InheritedVisibility), With<super::upgrade_ui::UpgradeSlot>>,
        Query<(&RelativeCursorPosition, &InheritedVisibility), With<super::shop_ui::ShopWindow>>,
    ),
    mut hovered: ResMut<Hovered>,
    mut drag: ResMut<Drag>,
    mut inv: ResMut<Inventory>,
    mut quick: ResMut<QuickSlots>,
    stats: Res<PlayerStats>,
    (mut use_ev, mut to_upgrade, mut sell): (
        MessageWriter<UseItem>,
        MessageWriter<DropToUpgrade>,
        MessageWriter<SellItem>,
    ),
    (mut log, time): (ResMut<GameLog>, Res<Time>),
    mut cells: Query<(&CellBg, &mut BorderColor)>,
) {
    // İmlecin altını bul.
    let mut spot = None;
    if w.inventory {
        if let Ok((r, v)) = grid.single() {
            if v.get() && r.cursor_over() {
                if let Some(n) = r.normalized {
                    // Bevy 0.18: merkez (0,0), sol üst (-0.5,-0.5), sağ alt (0.5,0.5).
                    let (nx, ny) = (n.x + 0.5, n.y + 0.5);
                    let c = ((nx * COLS as f32) as usize).min(COLS - 1);
                    let rr = ((ny * ROWS as f32) as usize).min(ROWS - 1);
                    spot = Some(Spot::Cell(index(page.0, c, rr)));
                }
            }
        }
        for (r, e, v) in &equips {
            if v.get() && r.cursor_over() {
                spot = Some(Spot::Equip(e.0));
            }
        }
    }
    for (r, q) in &quicks {
        if r.cursor_over() {
            spot = Some(Spot::Quick(q.0));
        }
    }
    if let Ok((r, v)) = upgrade.single() {
        if v.get() && r.cursor_over() {
            spot = Some(Spot::Upgrade);
        }
    }
    if spot.is_none() {
        if let Ok((r, v)) = shop.single() {
            if v.get() && r.cursor_over() {
                spot = Some(Spot::Shop);
            }
        }
    }
    hovered.0 = spot;

    // Hücre vurgusu.
    let hl_head = match spot {
        Some(Spot::Cell(i)) => Some(inv.head_of(i).unwrap_or(i)),
        _ => None,
    };
    for (c, mut b) in &mut cells {
        let i = page.0 * COLS * ROWS + c.0;
        let on = hl_head.is_some_and(|h| inv.head_of(i) == Some(h) || i == h);
        *b = BorderColor::all(if on { SLOT_HOVER } else { SLOT_BORDER });
    }

    let now = time.elapsed_secs();
    // Sürükleme başlat.
    if buttons.just_pressed(MouseButton::Left) && drag.0.is_none() {
        let start = match spot {
            Some(Spot::Cell(i)) => inv.head_of(i).and_then(|h| inv.item_at(h).map(|it| (Spot::Cell(h), it))),
            Some(Spot::Equip(s)) => inv.equipped(s).map(|it| (Spot::Equip(s), it)),
            Some(Spot::Quick(q)) => quick.0[q].map(|v| (Spot::Quick(q), Item::new(v))),
            _ => None,
        };
        if let Some((from, item)) = start {
            drag.0 = Some(DragState {
                from,
                item,
                split: keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight),
            });
        }
    }
    // Bırak.
    if buttons.just_released(MouseButton::Left) {
        if let Some(d) = drag.0.take() {
            match (d.from, spot) {
                (Spot::Cell(a), Some(Spot::Cell(b))) => {
                    let ok = if d.split { inv.split(a, b) } else { inv.move_item(a, b) };
                    if !ok {
                        log.push(now, C_BAD, "Oraya yerleşmiyor.");
                    }
                }
                (Spot::Cell(a), Some(Spot::Equip(s))) => {
                    if inv.item_at(a).and_then(|it| it.def().equip_slot()) == Some(s) {
                        if let Err(e) = inv.equip_from(a, stats.level) {
                            log.push(now, C_BAD, e);
                        }
                    } else {
                        log.push(now, C_BAD, "Bu eşya o yuvaya takılmaz.");
                    }
                }
                (Spot::Equip(s), Some(Spot::Cell(b))) => {
                    if let Err(e) = inv.unequip(s, Some(b)) {
                        log.push(now, C_BAD, e);
                    }
                }
                (Spot::Cell(_), Some(Spot::Quick(q))) => {
                    if d.item.def().upgradable() {
                        log.push(now, C_BAD, "Hızlı slota sadece kullanılabilir eşyalar konur.");
                    } else {
                        quick.0[q] = Some(d.item.vnum);
                    }
                }
                (Spot::Quick(a), Some(Spot::Quick(b))) => quick.0.swap(a, b),
                (Spot::Quick(a), None) => quick.0[a] = None,
                (Spot::Cell(a), Some(Spot::Upgrade)) => {
                    to_upgrade.write(DropToUpgrade(a));
                }
                (Spot::Cell(a), Some(Spot::Shop)) => {
                    sell.write(SellItem(a));
                }
                _ => {}
            }
        }
    }
    // Sağ tık.
    if buttons.just_pressed(MouseButton::Right) {
        match spot {
            Some(Spot::Cell(i)) => {
                if let Some(h) = inv.head_of(i) {
                    if w.shop {
                        sell.write(SellItem(h));
                    } else if w.upgrade && inv.item_at(h).is_some_and(|it| it.def().upgradable()) {
                        to_upgrade.write(DropToUpgrade(h));
                    } else {
                        use_ev.write(UseItem(h));
                    }
                }
            }
            Some(Spot::Equip(s)) => {
                if let Err(e) = inv.unequip(s, None) {
                    log.push(now, C_BAD, e);
                }
            }
            Some(Spot::Quick(q)) => {
                if let Some(v) = quick.0[q] {
                    if let Some(c) = inv.find(v) {
                        use_ev.write(UseItem(c));
                    }
                }
            }
            _ => {}
        }
    }
}

fn redraw_items(
    mut commands: Commands,
    inv: Res<Inventory>,
    page: Res<InvPage>,
    ui: Res<UiAssets>,
    mut wa: Option<ResMut<WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    layer: Query<Entity, With<ItemLayer>>,
    mut equip_icons: Query<(&EquipIcon, &mut ImageNode, &mut Visibility)>,
    mut yang: Query<&mut Text, With<YangText>>,
    mut last: Local<(u64, usize)>,
) {
    if *last == (inv.rev, page.0) {
        return;
    }
    let Some(wa) = wa.as_deref_mut() else { return };
    let Ok(layer) = layer.single() else { return };
    *last = (inv.rev, page.0);
    commands.entity(layer).despawn_children();
    let base = page.0 * COLS * ROWS;
    for k in 0..COLS * ROWS {
        let i = base + k;
        let crate::game::inventory::Cell::Head(it) = inv.cells[i] else {
            continue;
        };
        let (c, r) = (k % COLS, k / COLS);
        let h = it.def().height as f32;
        let img = icon(wa, &mut images, &it.icon_path()).unwrap_or_default();
        commands.entity(layer).with_children(|l| {
            l.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(c as f32 * PITCH + 1.0),
                    top: px(r as f32 * PITCH + 1.0),
                    width: px(SLOT - 2.0),
                    height: px(h * PITCH - 4.0),
                    ..default()
                },
                ImageNode::new(img),
                Pickable::IGNORE,
            ))
            .with_children(|n| {
                if it.count > 1 {
                    n.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            right: px(1),
                            bottom: px(0),
                            ..default()
                        },
                        text(&ui, it.count.to_string(), 11.0, TEXT),
                        TextShadow::default(),
                    ));
                }
                if it.def().upgradable() && it.plus > 0 {
                    n.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(1),
                            top: px(0),
                            ..default()
                        },
                        text(&ui, format!("+{}", it.plus), 11.0, crate::game::loot::item_color(&it)),
                        TextShadow::default(),
                    ));
                }
            });
        });
    }
    for (e, mut img, mut vis) in &mut equip_icons {
        match inv.equipped(e.0) {
            Some(it) => {
                if let Some(h) = icon(wa, &mut images, &it.icon_path()) {
                    img.image = h;
                    *vis = Visibility::Inherited;
                }
            }
            None => *vis = Visibility::Hidden,
        }
    }
    if let Ok(mut t) = yang.single_mut() {
        t.0 = format!("{} Yang", thousands(inv.yang));
    }
}

fn ghost_follow(
    drag: Res<Drag>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut wa: Option<ResMut<WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    mut ghost: Query<(&mut Node, &mut ImageNode, &mut Visibility), With<Ghost>>,
) {
    let Ok((mut n, mut img, mut vis)) = ghost.single_mut() else { return };
    let (Some(d), Ok(win), Some(wa)) = (drag.0, window.single(), wa.as_deref_mut()) else {
        *vis = Visibility::Hidden;
        return;
    };
    let Some(cur) = win.cursor_position() else { return };
    if drag.is_changed() {
        if let Some(h) = icon(wa, &mut images, &d.item.icon_path()) {
            img.image = h;
        }
        n.height = px(d.item.def().height as f32 * PITCH - 4.0);
    }
    img.color = Color::srgba(1.0, 1.0, 1.0, 0.8);
    n.left = px(cur.x - SLOT / 2.0);
    n.top = px(cur.y - SLOT / 2.0);
    *vis = Visibility::Inherited;
}

fn use_items(
    time: Res<Time>,
    mut ev: MessageReader<UseItem>,
    mut inv: ResMut<Inventory>,
    mut stats: ResMut<PlayerStats>,
    d: Res<Derived>,
    mut log: ResMut<GameLog>,
    mut cd: Local<f32>,
) {
    let now = time.elapsed_secs();
    for UseItem(cell) in ev.read() {
        let Some(it) = inv.item_at(*cell) else { continue };
        match it.def().kind {
            Kind::Potion { hp, sp } => {
                if stats.dead || now < *cd {
                    continue;
                }
                if (hp > 0 && stats.hp >= d.max_hp) && (sp == 0 || stats.sp >= d.max_sp) {
                    log.push(now, C_INFO, "Can zaten dolu.");
                    continue;
                }
                *cd = now + 0.4;
                stats.hp = (stats.hp + hp).min(d.max_hp);
                stats.sp = (stats.sp + sp).min(d.max_sp);
                inv.consume(it.vnum, 1);
            }
            Kind::Weapon { .. } | Kind::Armor { .. } | Kind::Wear { .. } => {
                match inv.equip_from(*cell, stats.level) {
                    Ok(()) => log.push(now, C_GOOD, format!("{} kuşanıldı.", it.display_name())),
                    Err(e) => log.push(now, C_BAD, e),
                }
            }
            Kind::Blessing | Kind::Handbook | Kind::DragonScale => {
                log.push(now, C_INFO, "Bu kağıt Demirci'de yükseltme yaparken kullanılır.");
            }
            Kind::Material => {
                log.push(now, C_INFO, "Yükseltme malzemesi. Demirci'ye götür.");
            }
        }
    }
}

/// Ekipman değişince karakter görünümünü güncelle.
fn sync_look(inv: Res<Inventory>, mut look: ResMut<PlayerLook>, mut last: Local<Option<[Option<Item>; 8]>>) {
    if last.as_ref() == Some(&inv.equip) {
        return;
    }
    *last = Some(inv.equip);
    let mut armor = crate::game::character::ArmorSet::default();
    let mut weapon = None;
    let mut two = false;
    for it in inv.equip.iter().flatten() {
        match it.def().kind {
            Kind::Armor { set, .. } => armor = set,
            Kind::Weapon { display, two_hand, .. } => {
                weapon = Some(display);
                two = two_hand;
            }
            _ => {}
        }
    }
    let plus = inv.equipped(EquipSlot::Weapon).map(|w| w.plus).unwrap_or(0);
    if look.armor != armor || look.weapon != weapon || look.two_hand != two || look.weapon_plus != plus {
        look.armor = armor;
        look.weapon = weapon;
        look.two_hand = two;
        look.weapon_plus = plus;
        look.dirty = true;
    }
}
