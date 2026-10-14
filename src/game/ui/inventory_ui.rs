//! Envanter penceresi (Metin2 düzeni): üstte ekipman yuvaları, altta 5×9 ızgara + I/II sayfa
//! sekmeleri, en altta Yang. Sürükle-bırak (hücre↔hücre, hücre→ekipman, →hızlı slot, →demirci,
//! →mağaza), sağ tık kuşan/kullan/çıkar, Shift+sürükle yığın böl.

use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use bevy::window::PrimaryWindow;

use benilla_assets::WorldAssets;

use super::hud::{QuickSlotNode, QuickSlots};
use super::skin::{self, Skin, Style};
use super::{icon, text, thousands, UiAssets, Windows, TEXT};
use crate::game::inventory::{index, Inventory, COLS, ROWS};
use crate::game::items::{EquipSlot, Item, Kind};
use crate::game::log::{GameLog, C_BAD, C_GOOD, C_INFO};
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
#[allow(dead_code)]
struct CellBg(usize);
#[derive(Component)]
struct InvClose;
#[derive(Component)]
struct GridHi;
#[derive(Component)]
struct EquipHi(EquipSlot);
#[derive(Component)]
struct EquipSil(EquipSlot);

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
                build.run_if(resource_exists::<UiAssets>.and(resource_exists::<Skin>)),
                (
                    show_hide,
                    page_tabs,
                    hover_and_mouse,
                    env_hover,
                    hover_visuals,
                    redraw_items,
                    ghost_follow,
                    use_items,
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

fn build(mut commands: Commands, ui: Res<UiAssets>, skin: Res<Skin>, mut done: Local<bool>) {
    if *done {
        return;
    }
    *done = true;
    let grid_w = COLS as f32 * PITCH - 2.0;
    let grid_h = ROWS as f32 * PITCH - 2.0;
    skin::spawn_window(
        &mut commands,
        &skin,
        &ui,
        Style::Dialog,
        Node {
            position_type: PositionType::Absolute,
            right: px(10),
            top: px(262),
            width: px(grid_w + 2.0 * skin::DIALOG_PAD),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(6),
            ..default()
        },
        "Envanter",
        InvClose,
    )
    .insert((Visibility::Hidden, InvWindow))
    .with_children(|w| {
        // Ekipman alanı: koyu mermer iç kutu + PaperDoll yuvaları.
        w.spawn(Node {
            width: px(grid_w),
            height: px(158),
            ..default()
        })
        .with_children(|e| {
            skin::inset_layers(e, &skin, true);
            for (slot, x, y, h) in EQUIP_LAYOUT {
                let hh = SLOT * h + (h - 1.0) * 2.0;
                e.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(x),
                        top: px(y + 1.0),
                        width: px(SLOT),
                        height: px(hh),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    skin::slot_image(&skin),
                    RelativeCursorPosition::default(),
                    EquipNode(slot),
                ))
                .with_children(|s| {
                    // Boş yuva silüeti (WoW PaperDoll).
                    s.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(2),
                            top: px((hh - (SLOT - 4.0)) / 2.0),
                            width: px(SLOT - 4.0),
                            height: px(SLOT - 4.0),
                            ..default()
                        },
                        ImageNode::new(skin.doll[slot.index()].clone()),
                        Pickable::IGNORE,
                        EquipSil(slot),
                    ));
                    s.spawn((
                        Node {
                            width: px(SLOT - 2.0),
                            height: px(hh - 2.0),
                            ..default()
                        },
                        ImageNode::default(),
                        Visibility::Hidden,
                        Pickable::IGNORE,
                        EquipIcon(slot),
                    ));
                    s.spawn((skin::slot_hilight(&skin), EquipHi(slot)));
                });
            }
        });
        // Sayfa sekmeleri (WoW karakter sekmeleri).
        w.spawn(Node {
            column_gap: px(2),
            ..default()
        })
        .with_children(|t| {
            for (i, label) in ["I", "II"].iter().enumerate() {
                t.spawn((skin::tab_button(&skin, &ui, label, 56.0), PageTab(i)));
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
                            ..default()
                        },
                        skin::slot_image(&skin),
                        Pickable::IGNORE,
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
            // Üstüne gelinen eşyanın vurgusu (ikonun üstünde, WoW gibi).
            g.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: px(SLOT),
                    height: px(SLOT),
                    ..default()
                },
                ImageNode::new(skin.slot_hi.clone()).with_mode(NodeImageMode::Stretch),
                Visibility::Hidden,
                Pickable::IGNORE,
                GridHi,
            ));
        });
        // Yang (WoW altın sikkesiyle)
        w.spawn(Node {
            width: px(grid_w),
            justify_content: JustifyContent::FlexEnd,
            align_items: AlignItems::Center,
            padding: UiRect::right(px(4)),
            ..default()
        })
        .with_children(|y| {
            y.spawn((text(&ui, "", 14.0, TEXT), skin::shadow(), YangText));
            y.spawn(skin::coin(&skin));
        });
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
    skin: Option<Res<Skin>>,
    mut page: ResMut<InvPage>,
    mut q: Query<(&PageTab, &Interaction, &mut ImageNode, &Children), With<Button>>,
    mut his: Query<&mut Visibility, With<skin::SkinHi>>,
    mut texts: Query<&mut TextColor>,
) {
    let Some(skin) = skin else { return };
    for (t, i, _, _) in &q {
        if *i == Interaction::Pressed && page.0 != t.0 {
            page.0 = t.0;
        }
    }
    for (t, i, mut img, ch) in &mut q {
        skin::set_tab(&skin, page.0 == t.0, *i != Interaction::None, &mut img, ch, &mut his, &mut texts);
    }
}

/// Test kancası: `AZM_HOVER=cell:3` ya da `equip:weapon` imleç yokken o noktayı "üstünde"
/// sayar (ekran görüntüsünde vurgu + tooltip doğrulaması için). Gerçek imleç her zaman önceliklidir.
fn env_hover(mut hovered: ResMut<Hovered>, page: Res<InvPage>, mut spot: Local<Option<Option<Spot>>>) {
    let want = *spot.get_or_insert_with(|| {
        let v = std::env::var("AZM_HOVER").ok()?.to_lowercase();
        let (k, arg) = v.split_once(':')?;
        match k {
            "cell" => arg.trim().parse().ok().map(Spot::Cell),
            "equip" => EquipSlot::ALL
                .iter()
                .find(|s| format!("{:?}", s).to_lowercase() == arg.trim())
                .map(|s| Spot::Equip(*s)),
            _ => None,
        }
    });
    if hovered.0.is_none() {
        if let Some(Spot::Cell(i)) = want {
            hovered.0 = Some(Spot::Cell(page.0 * COLS * ROWS + i % (COLS * ROWS)));
        } else if want.is_some() {
            hovered.0 = want;
        }
    }
}

/// Kapat düğmesi ve vurgular: üstüne gelinen hücre/eşya ve ekipman yuvası parlar, dolu yuvada
/// silüet gizlenir.
fn hover_visuals(
    hovered: Res<Hovered>,
    inv: Res<Inventory>,
    page: Res<InvPage>,
    mut w: ResMut<Windows>,
    close: Query<&Interaction, (With<InvClose>, Changed<Interaction>)>,
    mut grid_hi: Query<(&mut Node, &mut Visibility), With<GridHi>>,
    mut equip_hi: Query<(&EquipHi, &mut Visibility), (Without<GridHi>, Without<EquipSil>)>,
    mut sils: Query<(&EquipSil, &mut Visibility), (Without<GridHi>, Without<EquipHi>)>,
) {
    if close.iter().any(|i| *i == Interaction::Pressed) {
        w.inventory = false;
    }
    if let Ok((mut n, mut v)) = grid_hi.single_mut() {
        let base = page.0 * COLS * ROWS;
        let target = match hovered.0 {
            Some(Spot::Cell(i)) if i >= base && i < base + COLS * ROWS => {
                let head = inv.head_of(i).unwrap_or(i);
                let h = inv.item_at(head).map(|it| it.def().height as usize).unwrap_or(1);
                let k = head.saturating_sub(base);
                Some((k % COLS, k / COLS, h))
            }
            _ => None,
        };
        match target {
            Some((c, r, h)) => {
                n.left = px(c as f32 * PITCH);
                n.top = px(r as f32 * PITCH);
                n.height = px(h as f32 * PITCH - 2.0);
                *v = Visibility::Inherited;
            }
            None => *v = Visibility::Hidden,
        }
    }
    for (e, mut v) in &mut equip_hi {
        let want = if hovered.0 == Some(Spot::Equip(e.0)) { Visibility::Inherited } else { Visibility::Hidden };
        if *v != want {
            *v = want;
        }
    }
    for (s, mut v) in &mut sils {
        let want = if inv.equipped(s.0).is_some() { Visibility::Hidden } else { Visibility::Inherited };
        if *v != want {
            *v = want;
        }
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
                        skin::shadow(),
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
                        skin::shadow(),
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
        t.0 = thousands(inv.yang);
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
    mut mount: MessageWriter<crate::game::mount::MountRequest>,
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
                log.push(now, C_INFO, "Bu eşya Demirci'de yükseltme yaparken kullanılır.");
            }
            Kind::Material => {
                log.push(now, C_INFO, "Yükseltme malzemesi. Demirci'ye götür.");
            }
            Kind::MountTicket { .. } | Kind::MountMedal => {
                mount.write(crate::game::mount::MountRequest::UseItem(it.vnum));
            }
        }
    }
}
