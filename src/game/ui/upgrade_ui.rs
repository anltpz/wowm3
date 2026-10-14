//! Demirci penceresi — artı basma (+0 … +9). Eşyayı yuvaya sürükle (ya da pencere açıkken sağ
//! tıkla); şans, Yang ve malzeme gösterilir. Başarısızlıkta eşya yok olur — Kutsama Kağıdı ya da
//! Ejderha Pulu kullanılırsa bir artı düşer. El Kitabı +%10, Ejderha Pulu +%20 şans.

use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use rand::Rng;

use benilla_assets::WorldAssets;

use super::inventory_ui::{DropToUpgrade, PITCH, SLOT};
use super::skin::{self, Skin, SkinButton, SkinCheck, Style};
use super::{icon, text, thousands, UiAssets, Windows, GOLD, TEXT, TEXT_DIM};
use crate::game::inventory::Inventory;
use crate::game::items::{self, upgrade_cost, Item, MAX_PLUS, UPGRADE_CHANCE};
use crate::game::log::{GameLog, C_BAD, C_GOOD, C_INFO};
use crate::game::loot::item_color;

#[derive(Resource, Default)]
pub struct UpgradeState {
    /// Yükseltilecek eşyanın envanter hücresi.
    pub cell: Option<usize>,
    /// Hücredeki eşya (taşınırsa yuvayı boşaltmak için).
    pub item: Option<Item>,
    pub blessing: bool,
    pub handbook: bool,
    pub scale: bool,
    /// Son sonuç (başarı, metin, zaman).
    pub result: Option<(bool, String, f32)>,
}

#[derive(Component)]
pub struct UpgradeSlot;
#[derive(Component)]
struct UpgradeWindow;
#[derive(Component)]
struct UpgradeIcon;
#[derive(Component)]
struct InfoBox;
#[derive(Component, Clone, Copy, PartialEq)]
enum Btn {
    Blessing,
    Handbook,
    Scale,
    Upgrade,
    Close,
}
#[derive(Component)]
struct BtnLabel(Btn);
/// Seçeneğin onay kutusu.
#[derive(Component)]
struct CheckFor(Btn);

pub fn plugin(app: &mut App) {
    app.init_resource::<UpgradeState>().add_systems(
        Update,
        (
            build.run_if(resource_exists::<UiAssets>.and(resource_exists::<Skin>)),
            (show_hide, accept_drop, validate, buttons, redraw)
                .chain()
                .run_if(resource_exists::<UiAssets>),
        ),
    );
}

fn build(mut commands: Commands, ui: Res<UiAssets>, skin: Res<Skin>, mut done: Local<bool>) {
    if *done {
        return;
    }
    *done = true;
    skin::spawn_window(
        &mut commands,
        &skin,
        &ui,
        Style::Gold,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: px(120),
            margin: UiRect::left(px(-340)),
            width: px(350),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(6),
            ..default()
        },
        "Demirci — Eşya Yükseltme",
        Btn::Close,
    )
    .insert((Visibility::Hidden, UpgradeWindow))
    .with_children(|w| {
        w.spawn(Node {
            column_gap: px(10),
            align_items: AlignItems::Center,
            width: Val::Percent(100.0),
            ..default()
        })
        .with_children(|r| {
            // Altın kenarlı yükseltme yuvası (UI-EmptySlot).
            r.spawn((
                Node {
                    width: px(SLOT + 8.0),
                    height: px(3.0 * PITCH + 6.0),
                    flex_shrink: 0.0,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                skin::sliced(skin.empty_slot.clone(), 10.0, 0.8),
                RelativeCursorPosition::default(),
                UpgradeSlot,
            ))
            .with_children(|s| {
                s.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    text(&ui, "Eşyayı\nbırak", 10.0, TEXT_DIM),
                    Pickable::IGNORE,
                ));
                s.spawn((
                    Node {
                        width: px(SLOT),
                        height: px(2.0 * PITCH - 4.0),
                        ..default()
                    },
                    ImageNode::default(),
                    Visibility::Hidden,
                    Pickable::IGNORE,
                    UpgradeIcon,
                ));
            });
            // Bilgi kutusu (WoW iç kutu).
            r.spawn(Node {
                flex_grow: 1.0,
                min_height: px(3.0 * PITCH + 6.0),
                padding: UiRect::all(px(9)),
                ..default()
            })
            .with_children(|b| {
                skin::inset_layers(b, &skin, false);
                b.spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(3),
                        flex_grow: 1.0,
                        ..default()
                    },
                    InfoBox,
                ));
            });
        });
        // Seçenekler (WoW onay kutuları).
        for (b, label) in [
            (Btn::Blessing, "Kutsama Kağıdı"),
            (Btn::Handbook, "Demircinin El Kitabı"),
            (Btn::Scale, "Ejderha Pulu"),
        ] {
            w.spawn((
                Button,
                Node {
                    width: Val::Percent(100.0),
                    height: px(24),
                    padding: UiRect::horizontal(px(4)),
                    column_gap: px(6),
                    align_items: AlignItems::Center,
                    ..default()
                },
                SkinButton::default(),
                b,
            ))
            .with_children(|r| {
                r.spawn(skin::hi_layer(skin.row_hi.clone(), NodeImageMode::Stretch, 0.0));
                r.spawn((skin::checkbox(&skin, 22.0), CheckFor(b)));
                r.spawn((text(&ui, label, 12.0, TEXT), skin::shadow(), Pickable::IGNORE, BtnLabel(b)));
            });
        }
        w.spawn(Node {
            column_gap: px(12),
            margin: UiRect::top(px(4)),
            ..default()
        })
        .with_children(|r| {
            r.spawn((skin::panel_button(&skin, &ui, "Yükselt", 112.0, 22.0, ()), Btn::Upgrade));
            r.spawn((skin::panel_button(&skin, &ui, "Kapat", 112.0, 22.0, ()), Btn::Close));
        });
    });
}

fn show_hide(w: Res<Windows>, mut q: Query<&mut Visibility, With<UpgradeWindow>>, mut st: ResMut<UpgradeState>) {
    if let Ok(mut v) = q.single_mut() {
        let want = if w.upgrade { Visibility::Inherited } else { Visibility::Hidden };
        if *v != want {
            *v = want;
        }
    }
    if !w.upgrade && st.cell.is_some() {
        st.cell = None;
        st.item = None;
    }
}

fn accept_drop(
    mut ev: MessageReader<DropToUpgrade>,
    inv: Res<Inventory>,
    mut st: ResMut<UpgradeState>,
    mut log: ResMut<GameLog>,
    time: Res<Time>,
) {
    for DropToUpgrade(c) in ev.read() {
        let Some(it) = inv.item_at(*c) else { continue };
        if !it.def().upgradable() {
            log.push(time.elapsed_secs(), C_BAD, "Bu eşya yükseltilemez.");
            continue;
        }
        if it.plus >= MAX_PLUS {
            log.push(time.elapsed_secs(), C_INFO, "Bu eşya zaten +9.");
            continue;
        }
        st.cell = inv.head_of(*c);
        st.item = Some(it);
        st.result = None;
    }
}

/// Eşya taşındıysa/yok olduysa yuvayı boşalt.
fn validate(inv: Res<Inventory>, mut st: ResMut<UpgradeState>) {
    if let (Some(c), Some(it)) = (st.cell, st.item) {
        if inv.item_at(c) != Some(it) {
            st.cell = None;
            st.item = None;
        }
    }
}

pub fn chance(it: &Item, handbook: bool, scale: bool) -> u32 {
    let base = UPGRADE_CHANCE[(it.plus as usize).min(8)];
    (base + if handbook { 10 } else { 0 } + if scale { 20 } else { 0 }).min(100)
}

fn buttons(
    time: Res<Time>,
    q: Query<(&Btn, &Interaction), Changed<Interaction>>,
    mut st: ResMut<UpgradeState>,
    mut inv: ResMut<Inventory>,
    mut w: ResMut<Windows>,
    mut log: ResMut<GameLog>,
) {
    let now = time.elapsed_secs();
    for (b, i) in &q {
        if *i != Interaction::Pressed {
            continue;
        }
        match b {
            Btn::Blessing => st.blessing = !st.blessing && inv.count(items::V_BLESSING) > 0,
            Btn::Handbook => st.handbook = !st.handbook && inv.count(items::V_HANDBOOK) > 0,
            Btn::Scale => st.scale = !st.scale && inv.count(items::V_DRAGON_SCALE) > 0,
            Btn::Close => w.upgrade = false,
            Btn::Upgrade => {
                let (Some(cell), Some(it)) = (st.cell, st.item) else {
                    log.push(now, C_INFO, "Önce yükseltilecek eşyayı yuvaya bırak.");
                    continue;
                };
                let (yang, mats) = upgrade_cost(&it);
                if inv.yang < yang as u64 {
                    log.push(now, C_BAD, "Yeterli Yang yok.");
                    continue;
                }
                if let Some((v, n)) = mats.iter().find(|(v, n)| inv.count(*v) < *n as u32) {
                    log.push(
                        now,
                        C_BAD,
                        format!("Malzeme eksik: {} ({}/{})", Item::new(*v).def().name, inv.count(*v), n),
                    );
                    continue;
                }
                // Bedeli öde.
                inv.yang -= yang as u64;
                for (v, n) in &mats {
                    inv.consume(*v, *n as u32);
                }
                let use_bless = st.blessing && inv.consume(items::V_BLESSING, 1);
                let use_book = st.handbook && inv.consume(items::V_HANDBOOK, 1);
                let use_scale = st.scale && inv.consume(items::V_DRAGON_SCALE, 1);
                let p = chance(&it, use_book, use_scale);
                let roll = rand::rng().random_range(1..=100);
                // Malzeme tüketimi hücreleri kaydırmaz ama yine de eşyayı yeniden bul.
                let cell = if inv.item_at(cell) == Some(it) { cell } else { inv.cells.iter().position(|c| matches!(c, crate::game::inventory::Cell::Head(x) if *x == it)).unwrap_or(cell) };
                if roll <= p {
                    let mut up = it;
                    up.plus += 1;
                    inv.set_at(cell, up);
                    st.item = Some(up);
                    let msg = format!("Yükseltme başarılı! {}", up.display_name());
                    log.banner(now, C_GOOD, msg.clone());
                    log.push(now, C_GOOD, msg.clone());
                    st.result = Some((true, msg, now));
                    if up.plus >= MAX_PLUS {
                        st.cell = None;
                        st.item = None;
                    }
                } else if use_bless || use_scale {
                    let mut down = it;
                    down.plus = down.plus.saturating_sub(1);
                    inv.set_at(cell, down);
                    st.item = Some(down);
                    let msg = format!("Yükseltme başarısız oldu. {} oldu.", down.display_name());
                    log.banner(now, C_BAD, "Yükseltme başarısız!");
                    log.push(now, C_BAD, msg.clone());
                    st.result = Some((false, msg, now));
                } else {
                    inv.take(cell);
                    st.cell = None;
                    st.item = None;
                    let msg = format!("Yükseltme başarısız! {} yok oldu.", it.display_name());
                    log.banner(now, C_BAD, "Eşya yok oldu!");
                    log.push(now, C_BAD, msg.clone());
                    st.result = Some((false, msg, now));
                }
                // Kağıt bittiyse seçimi kaldır.
                st.blessing &= inv.count(items::V_BLESSING) > 0;
                st.handbook &= inv.count(items::V_HANDBOOK) > 0;
                st.scale &= inv.count(items::V_DRAGON_SCALE) > 0;
            }
        }
    }
}

fn redraw(
    mut commands: Commands,
    ui: Res<UiAssets>,
    st: Res<UpgradeState>,
    inv: Res<Inventory>,
    w: Res<Windows>,
    mut wa: Option<ResMut<WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    boxq: Query<Entity, With<InfoBox>>,
    mut iconq: Query<(&mut ImageNode, &mut Visibility, &mut Node), With<UpgradeIcon>>,
    mut labels: Query<(&BtnLabel, &mut Text, &mut TextColor)>,
    mut checks: Query<(&CheckFor, &mut SkinCheck)>,
    mut last: Local<(u64, Option<Item>, [bool; 3], bool)>,
) {
    if !w.upgrade {
        return;
    }
    let key = (inv.rev, st.item, [st.blessing, st.handbook, st.scale], st.result.is_some());
    if *last == key {
        return;
    }
    let Some(wa) = wa.as_deref_mut() else { return };
    // Pencere henüz kurulmadıysa sonraki karede yeniden dene.
    let Ok(boxe) = boxq.single() else { return };
    *last = key;
    commands.entity(boxe).despawn_children();
    if let Ok((mut img, mut vis, mut n)) = iconq.single_mut() {
        match st.item {
            Some(it) => {
                if let Some(h) = icon(wa, &mut images, &it.icon_path()) {
                    img.image = h;
                }
                n.height = px(it.def().height as f32 * PITCH - 4.0);
                *vis = Visibility::Inherited;
            }
            None => *vis = Visibility::Hidden,
        }
    }
    for (l, mut t, mut c) in &mut labels {
        let (name, have, on) = match l.0 {
            Btn::Blessing => ("Kutsama Kağıdı", inv.count(items::V_BLESSING), st.blessing),
            Btn::Handbook => ("Demircinin El Kitabı (+%10)", inv.count(items::V_HANDBOOK), st.handbook),
            Btn::Scale => ("Ejderha Pulu (+%20, koruma)", inv.count(items::V_DRAGON_SCALE), st.scale),
            _ => continue,
        };
        t.0 = format!("{}  ({} adet)", name, have);
        c.0 = if have == 0 { TEXT_DIM } else if on { GOLD } else { TEXT };
    }
    for (f, mut c) in &mut checks {
        let on = match f.0 {
            Btn::Blessing => st.blessing,
            Btn::Handbook => st.handbook,
            Btn::Scale => st.scale,
            _ => false,
        };
        if c.0 != on {
            c.0 = on;
        }
    }
    commands.entity(boxe).with_children(|b| {
        let Some(it) = st.item else {
            b.spawn(text(&ui, "Yükseltmek istediğin silah, zırh\nya da takıyı soldaki yuvaya sürükle.", 12.0, TEXT_DIM));
            if let Some((ok, msg, _)) = &st.result {
                b.spawn(text(&ui, msg.clone(), 12.0, if *ok { Color::srgb(0.5, 1.0, 0.5) } else { Color::srgb(1.0, 0.45, 0.4) }));
            }
            return;
        };
        let mut next = it;
        next.plus += 1;
        b.spawn(text(&ui, it.display_name(), 14.0, item_color(&it)));
        b.spawn(text(&ui, format!("→ {}", next.display_name()), 14.0, item_color(&next)));
        let p = chance(&it, st.handbook, st.scale);
        let pc = if p >= 70 { Color::srgb(0.5, 1.0, 0.5) } else if p >= 40 { GOLD } else { Color::srgb(1.0, 0.45, 0.4) };
        b.spawn(text(&ui, format!("Başarı şansı: %{p}"), 13.0, pc));
        let (yang, mats) = upgrade_cost(&it);
        let enough = inv.yang >= yang as u64;
        b.spawn(text(
            &ui,
            format!("Ücret: {} Yang", thousands(yang as u64)),
            12.0,
            if enough { GOLD } else { Color::srgb(1.0, 0.45, 0.4) },
        ));
        if mats.is_empty() {
            b.spawn(text(&ui, "Malzeme gerekmiyor", 11.0, TEXT_DIM));
        }
        for (v, n) in mats {
            let have = inv.count(v);
            b.spawn(text(
                &ui,
                format!("{}  {}/{}", Item::new(v).def().name, have, n),
                12.0,
                if have >= n as u32 { Color::srgb(0.6, 1.0, 0.6) } else { Color::srgb(1.0, 0.45, 0.4) },
            ));
        }
        let risk = if st.blessing || st.scale {
            "Başarısızlıkta: bir artı düşer"
        } else {
            "Başarısızlıkta: EŞYA YOK OLUR!"
        };
        b.spawn(text(&ui, risk, 11.0, if st.blessing || st.scale { TEXT_DIM } else { Color::srgb(1.0, 0.4, 0.3) }));
        if let Some((ok, msg, _)) = &st.result {
            b.spawn(text(&ui, msg.clone(), 12.0, if *ok { Color::srgb(0.5, 1.0, 0.5) } else { Color::srgb(1.0, 0.45, 0.4) }));
        }
    });
}
