//! Eşya bilgi kutusu: imleç bir eşyanın üstündeyken adı (+artı, renkli), gereksinimler,
//! saldırı/savunma, sonraki artının etkisi ve fiyat.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::inventory_ui::{Drag, Hovered, Spot};
use super::skin::{self, Skin, Style};
use super::{UiAssets, TEXT, TEXT_DIM};

const GOLD_TIP: Color = Color::srgb(1.0, 0.82, 0.35);
use crate::game::inventory::Inventory;
use crate::game::items::{Item, Kind, MAX_PLUS};
use crate::game::loot::item_color;
use crate::game::stats::PlayerStats;

#[derive(Component)]
struct Tip;
/// Satırların eklendiği iç düğüm (çerçeve katmanları silinmesin diye ayrı).
#[derive(Component)]
struct TipBody;

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            spawn.run_if(resource_exists::<UiAssets>.and(resource_exists::<Skin>)),
            update.run_if(resource_exists::<UiAssets>),
        ),
    );
}

fn spawn(mut commands: Commands, skin: Res<Skin>, mut done: Local<bool>) {
    if *done {
        return;
    }
    *done = true;
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                padding: UiRect::axes(px(11), px(10)),
                max_width: px(290),
                ..default()
            },
            GlobalZIndex(90),
            Visibility::Hidden,
            Pickable::IGNORE,
            Tip,
        ))
        .with_children(|p| {
            skin::frame_layers(p, &skin, Style::Tooltip);
            p.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(2),
                    ..default()
                },
                Pickable::IGNORE,
                TipBody,
            ));
        });
}

/// Bilgi kutusu satırları.
pub fn lines(it: &Item, my_level: u32) -> Vec<(String, Color, f32)> {
    let d = it.def();
    let mut v = vec![(it.display_name(), item_color(it), 15.0)];
    let p = it.plus as i32;
    match d.kind {
        Kind::Weapon { dmg, per_plus, two_hand, .. } => {
            v.push((
                if two_hand { "Çift el kılıç".into() } else { "Tek el kılıç".into() },
                TEXT_DIM,
                12.0,
            ));
            v.push((
                format!("Saldırı değeri {} - {}", dmg.0 + per_plus * p, dmg.1 + per_plus * p),
                TEXT,
                13.0,
            ));
            if it.plus < MAX_PLUS {
                v.push((format!("Sonraki artı: +{} saldırı", per_plus), TEXT_DIM, 12.0));
            }
        }
        Kind::Armor { def, per_plus, .. } => {
            v.push(("Zırh".into(), TEXT_DIM, 12.0));
            v.push((format!("Savunma +{}", def + per_plus * p), TEXT, 13.0));
            if it.plus < MAX_PLUS {
                v.push((format!("Sonraki artı: +{} savunma", per_plus), TEXT_DIM, 12.0));
            }
        }
        Kind::Wear { slot, def, hp, per_plus, .. } => {
            v.push((slot.name().into(), TEXT_DIM, 12.0));
            if def > 0 {
                v.push((format!("Savunma +{}", def), TEXT, 13.0));
            }
            if hp > 0 {
                v.push((format!("Max HP +{}", hp + per_plus * p), Color::srgb(0.6, 1.0, 0.6), 13.0));
            } else if p > 0 {
                v.push((format!("Savunma +{} (artı)", per_plus * p), TEXT, 13.0));
            }
        }
        Kind::Potion { hp, sp } => {
            if hp > 0 {
                v.push((format!("HP {} yeniler", hp), Color::srgb(1.0, 0.5, 0.5), 13.0));
            }
            if sp > 0 {
                v.push((format!("SP {} yeniler", sp), Color::srgb(0.5, 0.7, 1.0), 13.0));
            }
            v.push(("Sağ tık ya da hızlı slot ile kullan".into(), TEXT_DIM, 11.0));
        }
        Kind::Material => v.push(("Demirci'de artı basmak için malzeme".into(), TEXT_DIM, 12.0)),
        Kind::Blessing => v.push((
            "Yükseltme başarısız olursa eşya yok olmaz, bir artı düşer.".into(),
            Color::srgb(1.0, 0.85, 0.4),
            12.0,
        )),
        Kind::Handbook => v.push(("Yükseltme şansını %10 artırır.".into(), Color::srgb(1.0, 0.85, 0.4), 12.0)),
        Kind::DragonScale => v.push((
            "Şansı %20 artırır ve başarısızlıkta eşyayı korur.".into(),
            Color::srgb(1.0, 0.6, 0.3),
            12.0,
        )),
        Kind::MountTicket { secs } => {
            v.push((format!("{} dakika Ölümsüz Savaş Atı", secs / 60), Color::srgb(0.6, 0.9, 1.0), 13.0));
            v.push(("Sağ tık: bin (süre eklenir). Ctrl+G: in / bin".into(), TEXT_DIM, 11.0));
        }
        Kind::MountMedal => {
            v.push(("Süresiz Ölümsüz Savaş Atı".into(), Color::srgb(0.6, 0.9, 1.0), 13.0));
            v.push(("Sağ tık ya da Ctrl+G: in / bin".into(), TEXT_DIM, 11.0));
        }
    }
    if d.level > 1 {
        let ok = my_level >= d.level;
        v.push((
            format!("Seviye {} gerekli", d.level),
            if ok { TEXT_DIM } else { Color::srgb(1.0, 0.35, 0.3) },
            12.0,
        ));
    }
    if it.count > 1 {
        v.push((format!("Adet: {}", it.count), TEXT_DIM, 12.0));
    }
    v.push((format!("Satış: {} Yang", d.price * it.count as u32), Color::srgb(1.0, 0.82, 0.35), 11.0));
    v
}

fn update(
    mut commands: Commands,
    ui: Res<UiAssets>,
    hovered: Res<Hovered>,
    drag: Res<Drag>,
    inv: Res<Inventory>,
    quick: Res<super::hud::QuickSlots>,
    up: Res<super::upgrade_ui::UpgradeState>,
    stats: Res<PlayerStats>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut tip: Query<(&mut Node, &mut Visibility), With<Tip>>,
    body: Query<Entity, With<TipBody>>,
    mut last: Local<Option<(Spot, Item)>>,
    skill_slots: Query<(&super::hud::SkillSlotNode, &bevy::ui::RelativeCursorPosition)>,
    mut last_skill: Local<Option<usize>>,
) {
    let Ok((mut n, mut vis)) = tip.single_mut() else { return };
    let Ok(e) = body.single() else { return };
    // Beceri slotu bilgisi.
    if let Some(i) = skill_slots.iter().find(|(_, r)| r.cursor_over()).map(|(s, _)| s.0) {
        let sk = &crate::game::skills::SKILLS[i];
        if *last_skill != Some(i) {
            *last_skill = Some(i);
            *last = None;
            commands.entity(e).despawn_children();
            let lines = [
                (sk.name.to_string(), GOLD_TIP, 15.0),
                (sk.desc.to_string(), TEXT, 12.0),
                (format!("SP: {}   Bekleme: {:.0} sn", sk.sp, sk.cooldown), TEXT_DIM, 12.0),
                (format!("Seviye {} gerekli   Tuş: {}", sk.level, i + 1), if stats.level >= sk.level { TEXT_DIM } else { Color::srgb(1.0, 0.35, 0.3) }, 12.0),
            ];
            commands.entity(e).with_children(|p| {
                for (s, c, size) in lines {
                    p.spawn((Text::new(s), TextFont { font: ui.font.clone(), font_size: size, ..default() }, TextColor(c)));
                }
            });
        }
        if let Some((c, w)) = window.single().ok().and_then(|w| w.cursor_position().map(|c| (c, w.height()))) {
            n.left = px(c.x + 14.0);
            n.top = px((c.y - 110.0).min(w - 120.0));
        }
        *vis = Visibility::Inherited;
        return;
    }
    *last_skill = None;
    let item = match hovered.0 {
        _ if drag.0.is_some() => None,
        Some(Spot::Cell(i)) => inv.item_at(i),
        Some(Spot::Equip(s)) => inv.equipped(s),
        Some(Spot::Quick(q)) => quick.0[q].map(|v| Item::stack(v, inv.count(v).max(1) as u16)),
        Some(Spot::Upgrade) => up.cell.and_then(|c| inv.item_at(c)),
        _ => None,
    };
    let Some(item) = item else {
        *vis = Visibility::Hidden;
        *last = None;
        return;
    };
    let key = (hovered.0.unwrap(), item);
    if *last != Some(key) {
        *last = Some(key);
        commands.entity(e).despawn_children();
        commands.entity(e).with_children(|p| {
            for (s, c, size) in lines(&item, stats.level) {
                p.spawn((
                    Text::new(s),
                    TextFont {
                        font: ui.font.clone(),
                        font_size: size,
                        ..default()
                    },
                    TextColor(c),
                ));
            }
        });
    }
    if let Some(cur) = window.single().ok().and_then(|w| w.cursor_position().map(|c| (c, w.width()))) {
        let (c, w) = cur;
        let x = if c.x + 300.0 > w { c.x - 290.0 } else { c.x + 18.0 };
        n.left = px(x);
        n.top = px(c.y + 12.0);
    } else if let Ok(w) = window.single() {
        // İmleç pencerede yok (AZM_HOVER testi): envanterin soluna koy.
        n.left = px(w.width() - 500.0);
        n.top = px(330.0);
    }
    *vis = Visibility::Inherited;
}
