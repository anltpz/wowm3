//! Zindan arayüzü (D-22) — WoW 1.12 FrameXML birebir:
//! - **Giriş diyaloğu:** rehberle konuşunca (`DungeonOffer`) WoW DialogBox penceresi (`skin::spawn_window`:
//!   UI-DialogBox-Background/Border/Header). Şartlar satır satır, iç kutuda (UI-Tooltip-Border): karşılanan
//!   şart yeşil (GameFontGreen) + UI-CheckBox-Check, karşılanmayan kırmızı (GameFontRed) + UI-GroupLoot-Pass-Up.
//!   Düğmeler UI-Panel-Button (Gir kapalıyken UI-Panel-Button-Disabled): "Gir" → `DungeonEnter`,
//!   "Bilet Al" → `DungeonBuyTicket`, "Kapat" → teklifi kapatır. Eski Y/N/Esc tuşları da çalışmaya devam eder.
//! - **İzleyici (sağda, mini haritanın altında):** WoW QuestTimerFrame (160×72 DialogBox, başlık plakası,
//!   GameFontHighlightSmall süre) + QuestWatchFrame düzeni (başlık GameFontNormal altın, satırlar " - " önekli;
//!   tamamlanan hedef beyaz, süren 0.8 gri; başlık hedef bitince parlak altın, değilse 0.75/0.61/0).
//! - **Bantlar:** WoW ZoneText (ZoneTextFont/SubZoneTextFont, THICK outline, renk 1/.9294/.7607; 0.5 sn
//!   belirme, bekleme, 2 sn sönme) → giriş, brifing (kat adı + hedef), kat temizlendi, ZAFER, BAŞARISIZ; zafer/
//!   başarısızlık sonrası çıkış geri sayımı (GameFontNormalHuge). Çekirdeğin (`arena.rs`/`entry.rs`) aynı anlar
//!   için yazdığı genel duyuru (`GameLog::banner`) zindandayken bu bantlarla değiştirilir (çift yazı olmasın).
//! - Test kancası: `AZM_OPEN=zindan` (Örümcek rehberi) / `AZM_OPEN=kule` (Şeytan Kulesi) diyaloğu açılışta
//!   açar; oyuncu rehberin 8 m yakınında başlamalı (`AZM_FOCUS`).

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use benilla_assets::WorldAssets;

use super::skin::{self, Skin, Style, WOW_GOLD, WOW_WHITE};
use super::{thousands, UiAssets, TEXT, TEXT_DIM};
use crate::game::dungeon::entry::EntryCtx;
use crate::game::dungeon::{
    self, DungeonBuyTicket, DungeonCooldowns, DungeonDef, DungeonEnter, DungeonEvent, DungeonOffer, DungeonRun, Phase,
    RunState,
};
use crate::game::inventory::Inventory;
use crate::game::log::{GameLog, C_BAD};
use crate::game::npc::{Npc, NpcKind, NpcTalked};
use crate::game::stats::PlayerStats;

/// WoW renkleri (FrameXML Fonts.xml / ZoneText.xml / QuestLogFrame.lua).
const WOW_GREEN: Color = Color::srgb(0.1, 1.0, 0.1);
const WOW_RED: Color = Color::srgb(1.0, 0.1, 0.1);
const WOW_DISABLE: Color = Color::srgb(0.5, 0.5, 0.5);
const ZONE_TEXT: Color = Color::srgb(1.0, 0.9294, 0.7607);
const WATCH_DIM: Color = Color::srgb(0.8, 0.8, 0.8);
const WATCH_TITLE_DIM: Color = Color::srgb(0.75, 0.61, 0.0);

/// ZoneText süreleri (ZoneText.xml: ZoneFadeInDuration / ZoneFadeOutDuration).
const FADE_IN: f32 = 0.5;
const FADE_OUT: f32 = 2.0;
/// Bantların ekran üstünden uzaklığı (%). Genel duyuru (%22) ile çakışmasın diye biraz yukarıda.
const BAND_TOP: f32 = 9.0;
/// İzleyicinin üst kenarı (px): mini harita (240) + altındaki görev kutusunun altı.
const TRACK_TOP: f32 = 250.0;

// ---------------------------------------------------------------------------------------------------
// Saf yardımcılar (birim testli)
// ---------------------------------------------------------------------------------------------------

/// Diyalogdaki bir giriş şartı satırı.
#[derive(Clone, Debug, PartialEq)]
pub struct Cond {
    pub met: bool,
    pub text: String,
}

fn ticket_name(vnum: u32) -> &'static str {
    crate::game::items::def(vnum).map_or("Giriş bileti", |d| d.name)
}

/// Giriş şartları satır satır (`check_entry` ile aynı kurallar, ama ilk hatada durmadan): seviye, bilet,
/// bekleme süresi, ölü değil, binekli değil.
pub fn conditions(def: &DungeonDef, ctx: &EntryCtx, cds: &DungeonCooldowns) -> Vec<Cond> {
    let mut v = Vec::with_capacity(5);
    let lv = ctx.level >= def.level.0 && ctx.level <= def.level.1;
    v.push(Cond { met: lv, text: format!("Seviye {}-{} (seviyen: {})", def.level.0, def.level.1, ctx.level) });
    if def.ticket != 0 {
        let name = ticket_name(def.ticket);
        v.push(if ctx.tickets > 0 {
            Cond { met: true, text: format!("{name} (çantada {})", ctx.tickets) }
        } else {
            Cond { met: false, text: format!("{name} yok") }
        });
    }
    let secs = cds.remaining(def.id, ctx.now, def.cooldown_secs);
    v.push(if secs == 0 {
        Cond { met: true, text: "Bekleme süresi yok".into() }
    } else {
        Cond { met: false, text: format!("Bekleme süresi: {}", dungeon::fmt_secs(secs)) }
    });
    v.push(Cond { met: !ctx.dead, text: if ctx.dead { "Ölüyken giremezsin" } else { "Hayattasın" }.into() });
    v.push(Cond { met: !ctx.riding, text: if ctx.riding { "Binekten inmelisin" } else { "Binekte değilsin" }.into() });
    v
}

/// "Bilet Al" düğmesi gösterilir mi (bilet isteyen ve rehberin sattığı zindan).
pub fn sells_ticket(def: &DungeonDef) -> bool {
    def.ticket != 0 && def.ticket_price > 0
}

/// Geri sayım: "mm:ss" (bir saati geçerse "s:mm:ss"); kesirli saniye yukarı yuvarlanır.
pub fn clock(secs: f32) -> String {
    let s = secs.max(0.0).ceil() as u64;
    let (h, m, sec) = (s / 3600, (s % 3600) / 60, s % 60);
    if h > 0 {
        format!("{h}:{m:02}:{sec:02}")
    } else {
        format!("{m:02}:{sec:02}")
    }
}

/// Süre: tam dakikaysa "30 dk", değilse `fmt_secs` ("12 dk 5 sn", "1 sa 2 dk").
pub fn duration(secs: u64) -> String {
    if secs % 60 == 0 && secs < 3600 {
        format!("{} dk", secs / 60)
    } else {
        dungeon::fmt_secs(secs)
    }
}

/// Kat adından "2. Kat: " önekini atar ("Kat 2/3: <ad>" satırında tekrar etmesin).
pub fn floor_title(name: &str) -> &str {
    if let Some((head, rest)) = name.split_once(". Kat: ") {
        if !head.is_empty() && head.chars().all(|c| c.is_ascii_digit()) {
            return rest;
        }
    }
    name
}

/// İzleyicinin kat satırı: "Kat 2/3: Ağlarla Örülü Tüneller".
pub fn floor_line(r: &RunState) -> String {
    format!("Kat {}/{}: {}", r.floor + 1, r.def.floors.len(), floor_title(r.floor_def().name))
}

/// ZoneText saydamlığı (FadingFrame): `FADE_IN` sn belirir, `hold` sn kalır, `FADE_OUT` sn söner.
pub fn band_alpha(t: f32, hold: f32) -> f32 {
    if t < 0.0 {
        0.0
    } else if t < FADE_IN {
        t / FADE_IN
    } else if t < FADE_IN + hold {
        1.0
    } else {
        (1.0 - (t - FADE_IN - hold) / FADE_OUT).clamp(0.0, 1.0)
    }
}

/// Zindandayken çekirdeğin yazdığı genel duyuru bu UI'nin bantlarından biriyle aynı anı mı anlatıyor.
fn is_dungeon_banner(text: &str, r: &RunState) -> bool {
    text == r.def.name
        || text == "Kat temizlendi!"
        || text == "Süre doldu!"
        || text == format!("{} tamamlandı!", r.def.name)
        || r.def.floors.iter().any(|f| f.name == text || f.objective.describe() == text)
}

// ---------------------------------------------------------------------------------------------------
// Bileşenler / kaynaklar
// ---------------------------------------------------------------------------------------------------

/// Açık giriş diyaloğu (hangi teklif için kurulduğu).
#[derive(Component)]
struct DungeonDialog {
    id: u32,
    npc: Entity,
}
#[derive(Component)]
struct CondIcon(usize);
#[derive(Component)]
struct CondText(usize);
#[derive(Component)]
struct EnterButton;
#[derive(Component)]
struct EnterLabel;
#[derive(Component)]
struct BuyButton;
#[derive(Component)]
struct CloseDialog;

#[derive(Component)]
struct TrackerRoot;
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Track {
    Timer,
    Title,
    Floor,
    Objective,
    Progress,
    Status,
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum BandLine {
    Big,
    Sub,
    Exit,
}
/// Bant yazısının kendisi.
#[derive(Component)]
struct BandText(BandLine);
/// WoW THICK outline'ı: yazının arkasında 8 yöne kaydırılmış siyah kopyaları.
#[derive(Component)]
struct BandOutline(BandLine);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BandKind {
    Zone,
    Briefing,
    Cleared,
    Victory,
    Failed,
}

#[derive(Clone, Debug)]
struct Band {
    kind: BandKind,
    big: String,
    sub: String,
    t0: f32,
    hold: f32,
}

#[derive(Resource, Default)]
struct DungeonUi {
    band: Option<Band>,
}

/// Diyaloğun ek WoW dokuları (skin'de olmayanlar).
#[derive(Resource, Clone)]
struct DialogTex {
    check: Handle<Image>,
    pass: Handle<Image>,
    btn_up: Handle<Image>,
    btn_down: Handle<Image>,
    btn_off: Handle<Image>,
    btn_off_down: Handle<Image>,
}

#[derive(Resource)]
struct DungeonUiBuilt;

pub fn plugin(app: &mut App) {
    app.init_resource::<DungeonUi>().add_systems(
        Update,
        (
            build.run_if(
                resource_exists::<UiAssets>
                    .and(resource_exists::<Skin>)
                    .and(resource_exists::<WorldAssets>)
                    .and(not(resource_exists::<DungeonUiBuilt>)),
            ),
            (dialog_sync, dialog_update, dialog_buttons, tracker, band_events, band_draw, env_open)
                .chain()
                .run_if(resource_exists::<DungeonUiBuilt>),
        ),
    );
}

// ---------------------------------------------------------------------------------------------------
// Kurulum
// ---------------------------------------------------------------------------------------------------

/// MPQ dokusunun sol üst `w`×`h` parçası (UI-Panel-Button 128×32 dokusunun 80×22'lik düğme alanı).
fn crop_tex(wa: &mut WorldAssets, images: &mut Assets<Image>, path: &str, w: u32, h: u32) -> Handle<Image> {
    let (sw, sh, px) = wa.decode_rgba(path).unwrap_or_else(|| {
        warn!("dungeon_ui: doku yok: {path}");
        (1, 1, vec![0; 4])
    });
    let (w, h) = (w.min(sw), h.min(sh));
    let mut out = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        let src = ((y * sw) * 4) as usize;
        let dst = ((y * w) * 4) as usize;
        out[dst..dst + (w * 4) as usize].copy_from_slice(&px[src..src + (w * 4) as usize]);
    }
    images.add(Image::new(
        Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
        TextureDimension::D2,
        out,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}

fn build(
    mut commands: Commands,
    ui: Res<UiAssets>,
    skin: Res<Skin>,
    mut wa: ResMut<WorldAssets>,
    mut images: ResMut<Assets<Image>>,
) {
    commands.insert_resource(DungeonUiBuilt);
    const BT: &str = "Interface\\Buttons\\";
    let check = super::icon(&mut wa, &mut images, &format!("{BT}UI-CheckBox-Check.blp")).unwrap_or_default();
    let pass = super::icon(&mut wa, &mut images, &format!("{BT}UI-GroupLoot-Pass-Up.blp")).unwrap_or_default();
    commands.insert_resource(DialogTex {
        check,
        pass,
        btn_up: skin.btn_up.clone(),
        btn_down: skin.btn_down.clone(),
        btn_off: crop_tex(&mut wa, &mut images, &format!("{BT}UI-Panel-Button-Disabled.blp"), 80, 22),
        btn_off_down: crop_tex(&mut wa, &mut images, &format!("{BT}UI-Panel-Button-Disabled-Down.blp"), 80, 22),
    });

    // ===== İzleyici: QuestTimerFrame + QuestWatchFrame (mini haritanın altında, sağa yaslı) =====
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(10),
                top: px(TRACK_TOP),
                width: px(280),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                row_gap: px(10),
                ..default()
            },
            Visibility::Hidden,
            Pickable::IGNORE,
            TrackerRoot,
        ))
        .with_children(|t| {
            // QuestTimerFrame: DialogBox arka planı + kenarı, UI-DialogBox-Header plakası, süre yazısı.
            t.spawn((
                Node {
                    width: px(160),
                    height: px(54),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::top(px(8)),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|f| {
                skin::frame_layers(f, &skin, Style::Dialog);
                skin::header(f, &skin, &ui, Style::Dialog, "Zindan Süresi");
                f.spawn((
                    Text::new(""),
                    TextFont { font: ui.font.clone(), font_size: 16.0, ..default() },
                    TextColor(WOW_WHITE),
                    skin::shadow(),
                    Pickable::IGNORE,
                    Track::Timer,
                ));
            });
            // QuestWatchFrame: başlık + " - " önekli satırlar, sola yaslı. Zemin, üstteki HUD görev kutusuyla aynı
            // yarı saydam siyah (aydınlık zindan duvarlarında okunsun).
            t.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    max_width: px(280),
                    padding: UiRect::all(px(6)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
                Pickable::IGNORE,
            ))
            .with_children(|w| {
                let line = |size: f32, color: Color, which: Track, font: &Handle<Font>| {
                    (
                        Text::new(""),
                        TextFont { font: font.clone(), font_size: size, ..default() },
                        TextColor(color),
                        skin::shadow(),
                        Pickable::IGNORE,
                        which,
                    )
                };
                w.spawn(line(15.0, WOW_GOLD, Track::Title, &ui.title_font));
                w.spawn(line(14.0, WOW_WHITE, Track::Floor, &ui.font));
                w.spawn(line(13.0, WATCH_DIM, Track::Objective, &ui.font));
                w.spawn(line(13.0, WATCH_DIM, Track::Progress, &ui.font));
                w.spawn(line(13.0, WATCH_DIM, Track::Status, &ui.font));
            });
        });

    // ===== Bantlar: ZoneText (büyük + alt satır) ve çıkış geri sayımı =====
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(BAND_TOP),
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(4),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|b| {
            outlined(b, &ui.title_font, 52.0, BandLine::Big, 2.5);
            outlined(b, &ui.title_font, 26.0, BandLine::Sub, 1.5);
            b.spawn((Node { height: px(6), ..default() }, Pickable::IGNORE));
            outlined(b, &ui.font, 20.0, BandLine::Exit, 1.2);
        });
}

/// Kalın kenarlı (WoW THICK outline) yazı: 8 siyah kopya + üstte asıl yazı.
fn outlined(p: &mut ChildSpawnerCommands, font: &Handle<Font>, size: f32, line: BandLine, w: f32) {
    const OFFS: [(f32, f32); 8] = [(-1., -1.), (0., -1.), (1., -1.), (-1., 0.), (1., 0.), (-1., 1.), (0., 1.), (1., 1.)];
    p.spawn((Node::default(), Pickable::IGNORE)).with_children(|c| {
        for (dx, dy) in OFFS {
            c.spawn((
                Node { position_type: PositionType::Absolute, left: px(dx * w), top: px(dy * w), ..default() },
                Text::new(""),
                TextFont { font: font.clone(), font_size: size, ..default() },
                TextColor(Color::NONE),
                Pickable::IGNORE,
                BandOutline(line),
            ));
        }
        c.spawn((
            Text::new(""),
            TextFont { font: font.clone(), font_size: size, ..default() },
            TextColor(Color::NONE),
            Pickable::IGNORE,
            BandText(line),
        ));
    });
}

// ---------------------------------------------------------------------------------------------------
// Giriş diyaloğu
// ---------------------------------------------------------------------------------------------------

fn entry_ctx(stats: &PlayerStats, inv: &Inventory, def: &DungeonDef, inside: bool) -> EntryCtx {
    EntryCtx {
        level: stats.level,
        tickets: if def.ticket == 0 { 0 } else { inv.count(def.ticket) },
        now: dungeon::unix_now(),
        dead: stats.dead,
        riding: stats.riding,
        inside,
    }
}

/// Teklif açılınca pencereyi kurar, kapanınca/değişince siler.
fn dialog_sync(
    mut commands: Commands,
    offer: Res<DungeonOffer>,
    (ui, skin, tex): (Res<UiAssets>, Res<Skin>, Res<DialogTex>),
    (stats, inv, cds, run): (Res<PlayerStats>, Res<Inventory>, Res<DungeonCooldowns>, Res<DungeonRun>),
    open: Query<(Entity, &DungeonDialog)>,
) {
    let want = offer.0.as_ref().map(|o| (o.dungeon.id, o.npc));
    let mut have = None;
    for (e, d) in &open {
        if Some((d.id, d.npc)) == want && have.is_none() {
            have = Some(e);
        } else {
            commands.entity(e).despawn();
        }
    }
    let Some(o) = offer.0.as_ref() else { return };
    if have.is_some() {
        return;
    }
    let def = o.dungeon;
    let conds = conditions(def, &entry_ctx(&stats, &inv, def, run.active()), &cds);
    let greet = dungeon::NPCS.iter().find(|n| n.kind == NpcKind::DungeonGuide(def.id)).map(|n| n.greet);
    let font = |size: f32| TextFont { font: ui.font.clone(), font_size: size, ..default() };
    skin::spawn_window(
        &mut commands,
        &skin,
        &ui,
        Style::Dialog,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: px(150),
            margin: UiRect::left(px(-190)),
            width: px(380),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(6),
            ..default()
        },
        def.name,
        CloseDialog,
    )
    .insert((DungeonDialog { id: def.id, npc: o.npc }, ZIndex(20)))
    .with_children(|w| {
        // Alt başlık: seviye aralığı, kat sayısı, süre (GameFontHighlightSmall).
        w.spawn((Node { justify_content: JustifyContent::Center, ..default() }, Pickable::IGNORE)).with_child((
            Text::new(format!(
                "Seviye {}-{}   ·   {} kat   ·   Süre {}",
                def.level.0,
                def.level.1,
                def.floors.len(),
                duration(def.time_limit_secs as u64)
            )),
            font(13.0),
            TextColor(WOW_WHITE),
            skin::shadow(),
        ));
        if let Some(g) = greet {
            w.spawn((
                Node { margin: UiRect::vertical(px(2)), ..default() },
                Text::new(format!("\u{201C}{g}\u{201D}")),
                font(13.0),
                TextColor(TEXT),
                skin::shadow(),
                Pickable::IGNORE,
            ));
        }
        w.spawn((
            Node { margin: UiRect::top(px(2)), ..default() },
            Text::new("Giriş Şartları"),
            TextFont { font: ui.title_font.clone(), font_size: 14.0, ..default() },
            TextColor(WOW_GOLD),
            skin::shadow(),
            Pickable::IGNORE,
        ));
        // Şart kutusu (WoW iç kutusu: UI-Tooltip-Border + koyu zemin).
        w.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                padding: UiRect { left: px(10), right: px(10), top: px(8), bottom: px(8) },
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|b| {
            skin::inset_layers(b, &skin, false);
            for (i, c) in conds.iter().enumerate() {
                b.spawn((Node { align_items: AlignItems::Center, column_gap: px(6), ..default() }, Pickable::IGNORE))
                    .with_children(|r| {
                        r.spawn((
                            Node { width: px(16), height: px(16), flex_shrink: 0.0, ..default() },
                            ImageNode::new(if c.met { tex.check.clone() } else { tex.pass.clone() }),
                            Pickable::IGNORE,
                            CondIcon(i),
                        ));
                        r.spawn((
                            Text::new(c.text.clone()),
                            font(13.0),
                            TextColor(if c.met { WOW_GREEN } else { WOW_RED }),
                            skin::shadow(),
                            Pickable::IGNORE,
                            CondText(i),
                        ));
                    });
            }
        });
        // Düğmeler: Gir · Bilet Al (fiyat) · Kapat.
        w.spawn((
            Node { justify_content: JustifyContent::Center, column_gap: px(8), margin: UiRect::top(px(4)), ..default() },
            Pickable::IGNORE,
        ))
        .with_children(|r| {
            r.spawn((skin::panel_button(&skin, &ui, "Gir", 80.0, 22.0, EnterLabel), EnterButton));
            if sells_ticket(def) {
                let label = format!("Bilet Al ({} Yang)", thousands(def.ticket_price as u64));
                r.spawn((skin::panel_button(&skin, &ui, &label, 150.0, 22.0, ()), BuyButton));
            }
            r.spawn((skin::panel_button(&skin, &ui, "Kapat", 80.0, 22.0, ()), CloseDialog));
        });
        w.spawn((Node { justify_content: JustifyContent::Center, ..default() }, Pickable::IGNORE)).with_child((
            Text::new("Y: gir   ·   N / Esc: kapat"),
            font(11.0),
            TextColor(TEXT_DIM),
        ));
    });
}

/// Şartları her karede yeniden değerlendirir (bilet alınca / seviye atlayınca satır yeşile döner).
fn dialog_update(
    offer: Res<DungeonOffer>,
    tex: Res<DialogTex>,
    (stats, inv, cds, run): (Res<PlayerStats>, Res<Inventory>, Res<DungeonCooldowns>, Res<DungeonRun>),
    mut icons: Query<(&CondIcon, &mut ImageNode), Without<EnterButton>>,
    mut texts: Query<(&CondText, &mut Text, &mut TextColor), Without<EnterLabel>>,
    mut enter: Query<(&mut ImageNode, &mut skin::SkinButton, &Children), With<EnterButton>>,
    mut label: Query<&mut TextColor, With<EnterLabel>>,
    mut his: Query<&mut Visibility, With<skin::SkinHi>>,
) {
    let Some(o) = offer.0.as_ref() else { return };
    let conds = conditions(o.dungeon, &entry_ctx(&stats, &inv, o.dungeon, run.active()), &cds);
    for (i, mut img) in &mut icons {
        let Some(c) = conds.get(i.0) else { continue };
        let want = if c.met { &tex.check } else { &tex.pass };
        if img.image != *want {
            img.image = want.clone();
        }
    }
    for (i, mut t, mut col) in &mut texts {
        let Some(c) = conds.get(i.0) else { continue };
        if t.0 != c.text {
            t.0 = c.text.clone();
        }
        let want = if c.met { WOW_GREEN } else { WOW_RED };
        if col.0 != want {
            col.0 = want;
        }
    }
    // "Gir": şartlar tutmuyorsa WoW kapalı düğmesi (UI-Panel-Button-Disabled, gri yazı, vurgu yok).
    let ok = o.check.is_ok();
    for (mut img, mut b, children) in &mut enter {
        let (up, down) = if ok { (&tex.btn_up, &tex.btn_down) } else { (&tex.btn_off, &tex.btn_off_down) };
        if b.up.as_ref() != Some(up) {
            b.up = Some(up.clone());
            b.down = Some(down.clone());
            img.image = up.clone();
        }
        if !ok {
            for c in children.iter() {
                if let Ok(mut v) = his.get_mut(c) {
                    if *v != Visibility::Hidden {
                        *v = Visibility::Hidden;
                    }
                }
            }
        }
    }
    for mut c in &mut label {
        let want = if ok { WOW_GOLD } else { WOW_DISABLE };
        if c.0 != want {
            c.0 = want;
        }
    }
}

fn dialog_buttons(
    time: Res<Time>,
    q: Query<(&Interaction, Has<EnterButton>, Has<BuyButton>, Has<CloseDialog>), Changed<Interaction>>,
    mut offer: ResMut<DungeonOffer>,
    (mut enter, mut buy): (MessageWriter<DungeonEnter>, MessageWriter<DungeonBuyTicket>),
    mut log: ResMut<GameLog>,
) {
    for (i, is_enter, is_buy, is_close) in &q {
        if *i != Interaction::Pressed {
            continue;
        }
        let Some(o) = offer.0.as_ref() else { continue };
        if is_close {
            offer.0 = None;
            return;
        }
        if is_enter {
            match o.check {
                Ok(()) => {
                    enter.write(DungeonEnter { id: o.dungeon.id, force: false });
                }
                Err(e) => log.push(time.elapsed_secs(), C_BAD, e.text()),
            }
        }
        if is_buy {
            buy.write(DungeonBuyTicket { id: o.dungeon.id });
        }
    }
}

// ---------------------------------------------------------------------------------------------------
// İzleyici
// ---------------------------------------------------------------------------------------------------

fn tracker(
    run: Res<DungeonRun>,
    mut root: Query<&mut Visibility, With<TrackerRoot>>,
    mut texts: Query<(&Track, &mut Text, &mut TextColor, &mut Node)>,
) {
    let Ok(mut vis) = root.single_mut() else { return };
    let st = run.state().filter(|r| r.phase != Phase::Exit);
    let want = if st.is_some() { Visibility::Inherited } else { Visibility::Hidden };
    if *vis != want {
        *vis = want;
    }
    let Some(r) = st else { return };
    let f = r.floor_def();
    let done = dungeon::objective_met(&f.objective, &r.progress) && !matches!(r.phase, Phase::Loading | Phase::Briefing { .. });
    for (k, mut t, mut c, mut node) in &mut texts {
        let (s, col) = match k {
            Track::Timer => match r.phase {
                Phase::Victory { left } => (format!("Çıkış {}", clock(left)), WOW_GREEN),
                Phase::Failed { .. } => (clock(0.0), WOW_RED),
                _ => {
                    let left = r.time_left();
                    (clock(left), if left < 60.0 { WOW_RED } else { WOW_WHITE })
                }
            },
            Track::Title => (r.def.name.to_string(), if done || matches!(r.phase, Phase::Victory { .. }) { WOW_GOLD } else { WATCH_TITLE_DIM }),
            Track::Floor => (floor_line(r), WOW_WHITE),
            Track::Objective => (format!(" - {}", f.objective.describe()), if done { WOW_WHITE } else { WATCH_DIM }),
            Track::Progress => match r.phase {
                Phase::Loading | Phase::Briefing { .. } => (String::new(), WATCH_DIM),
                _ => (format!(" - {}", r.progress_text()), if done { WOW_WHITE } else { WATCH_DIM }),
            },
            Track::Status => match r.phase {
                Phase::Loading => (" - Kat hazırlanıyor...".into(), WATCH_DIM),
                Phase::Briefing { left } => (format!(" - Hazırlan: {} sn", left.max(0.0).ceil() as u32), WATCH_DIM),
                Phase::Cleared { left } => (format!(" - Kat temizlendi, sonraki kat {} sn", left.max(0.0).ceil() as u32), WOW_GREEN),
                Phase::Victory { .. } => (" - Zindan tamamlandı".into(), WOW_GREEN),
                Phase::Failed { .. } => (" - Süre doldu".into(), WOW_RED),
                _ if r.deaths > 0 => (format!(" - Ölüm: {}", r.deaths), WATCH_DIM),
                _ => (String::new(), WATCH_DIM),
            },
        };
        // Boş satır yer kaplamasın (WoW izleyicisi yalnız dolu satırları dizer).
        let display = if s.is_empty() { Display::None } else { Display::Flex };
        if node.display != display {
            node.display = display;
        }
        if t.0 != s {
            t.0 = s;
        }
        if c.0 != col {
            c.0 = col;
        }
    }
}

// ---------------------------------------------------------------------------------------------------
// Bantlar
// ---------------------------------------------------------------------------------------------------

fn band_events(
    time: Res<Time>,
    mut events: MessageReader<DungeonEvent>,
    run: Res<DungeonRun>,
    mut ui: ResMut<DungeonUi>,
    mut log: ResMut<GameLog>,
) {
    let now = time.elapsed_secs();
    for ev in events.read() {
        let r = run.state();
        let band = match (ev, r) {
            (DungeonEvent::Entered { .. }, Some(r)) => {
                Some(Band { kind: BandKind::Zone, big: r.def.name.into(), sub: String::new(), t0: now, hold: 1.0 })
            }
            (DungeonEvent::FloorReady { floor }, Some(r)) => r.def.floor(*floor).map(|f| Band {
                kind: BandKind::Briefing,
                big: floor_title(f.name).into(),
                sub: format!("Kat {}/{}  —  {}", floor + 1, r.def.floors.len(), f.objective.describe()),
                t0: now,
                hold: dungeon::BRIEFING_SECS,
            }),
            (DungeonEvent::FloorCleared { .. }, Some(_)) => Some(Band {
                kind: BandKind::Cleared,
                big: "Kat Temizlendi".into(),
                sub: String::new(),
                t0: now,
                hold: dungeon::CLEARED_SECS - FADE_OUT - FADE_IN,
            }),
            (DungeonEvent::Victory, Some(r)) => Some(Band {
                kind: BandKind::Victory,
                big: "ZAFER".into(),
                sub: format!("{} tamamlandı", r.def.name),
                t0: now,
                hold: 4.0,
            }),
            (DungeonEvent::Failed, Some(_)) => Some(Band {
                kind: BandKind::Failed,
                big: "BAŞARISIZ".into(),
                sub: "Süre doldu".into(),
                t0: now,
                hold: 2.5,
            }),
            (DungeonEvent::Left { .. }, _) => {
                // Çıkışta kalan bant (zafer/başarısızlık) dış dünyada sürmesin.
                ui.band = None;
                None
            }
            _ => None,
        };
        if band.is_some() {
            ui.band = band;
        }
    }
    // Çekirdeğin aynı anlar için yazdığı genel duyuruyu bantlarla değiştir.
    if let (Some(r), Some((text, _, _))) = (run.state(), log.banner.as_ref()) {
        if is_dungeon_banner(text, r) {
            log.banner = None;
        }
    }
}

fn band_draw(
    time: Res<Time>,
    run: Res<DungeonRun>,
    ui: Res<DungeonUi>,
    mut texts: Query<
        (Option<&BandText>, Option<&BandOutline>, &mut Text, &mut TextColor, &mut TextFont),
        Or<(With<BandText>, With<BandOutline>)>,
    >,
) {
    let now = time.elapsed_secs();
    let phase = run.state().map(|r| r.phase);
    let (big, sub, color, sub_color, alpha, big_size) = match &ui.band {
        Some(b) => {
            let a = band_alpha(now - b.t0, b.hold);
            let sub = match (b.kind, phase) {
                (BandKind::Cleared, Some(Phase::Cleared { left })) => {
                    format!("Sonraki kat {} sn içinde", left.max(0.0).ceil() as u32)
                }
                (BandKind::Cleared, _) => String::new(),
                _ => b.sub.clone(),
            };
            let (c, sc, size) = match b.kind {
                BandKind::Zone | BandKind::Briefing => (ZONE_TEXT, ZONE_TEXT, 52.0),
                BandKind::Cleared => (WOW_GOLD, ZONE_TEXT, 52.0),
                BandKind::Victory => (WOW_GOLD, ZONE_TEXT, 72.0),
                BandKind::Failed => (WOW_RED, WOW_RED, 72.0),
            };
            (b.big.clone(), sub, c, sc, a, size)
        }
        None => (String::new(), String::new(), ZONE_TEXT, ZONE_TEXT, 0.0, 52.0),
    };
    // Çıkış geri sayımı (zafer / başarısızlık süresince sabit, GameFontNormalHuge).
    let exit = match phase {
        Some(Phase::Victory { left }) => format!("Çıkış portalı açıldı  ·  {} sonra dışarı ışınlanacaksın", clock(left)),
        Some(Phase::Failed { left }) => format!("{} sn sonra dışarı ışınlanacaksın", left.max(0.0).ceil() as u32),
        _ => String::new(),
    };
    for (main, outline, mut t, mut c, mut f) in &mut texts {
        let Some(line) = main.map(|m| m.0).or(outline.map(|o| o.0)) else { continue };
        let (s, col, a) = match line {
            BandLine::Big => (&big, color, alpha),
            BandLine::Sub => (&sub, sub_color, alpha),
            BandLine::Exit => (&exit, WOW_GOLD, 1.0),
        };
        if t.0 != *s {
            t.0 = s.clone();
        }
        let want = if main.is_some() { col.with_alpha(a) } else { Color::srgba(0.0, 0.0, 0.0, 0.85 * a) };
        if c.0 != want {
            c.0 = want;
        }
        if line == BandLine::Big && f.font_size != big_size {
            f.font_size = big_size;
        }
    }
}

// ---------------------------------------------------------------------------------------------------
// Test kancası
// ---------------------------------------------------------------------------------------------------

/// `AZM_OPEN=zindan` / `kule`: rehberle konuşulmuş gibi `NpcTalked` yayınlar (rehber doğana kadar dener).
fn env_open(
    time: Res<Time>,
    npcs: Query<(Entity, &Npc)>,
    mut out: MessageWriter<NpcTalked>,
    mut want: Local<Option<Option<u32>>>,
) {
    let id = *want.get_or_insert_with(|| {
        let v = std::env::var("AZM_OPEN").unwrap_or_default().to_lowercase();
        v.split(',').map(str::trim).find_map(|k| match k {
            "zindan" | "dungeon" | "orumcek" => Some(dungeon::spider::DUNGEON.id),
            "kule" | "tower" => Some(dungeon::tower::DUNGEON.id),
            _ => None,
        })
    });
    let Some(id) = id else { return };
    // Oyuncu ve rehber yerine otursun.
    if time.elapsed_secs() < 4.0 {
        return;
    }
    if let Some((e, n)) = npcs.iter().find(|(_, n)| n.kind == NpcKind::DungeonGuide(id)) {
        info!("dungeon_ui: AZM_OPEN → {} ile konuşuldu", n.name);
        out.write(NpcTalked { entity: e, kind: n.kind });
        *want = Some(None);
    } else if time.elapsed_secs() > 30.0 {
        warn!("dungeon_ui: AZM_OPEN: rehber bulunamadı");
        *want = Some(None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::dungeon::defs::{FloorDef, Objective};
    use crate::game::dungeon::entry::check_entry;

    const D: DungeonDef = DungeonDef {
        id: 5,
        key: "d",
        name: "D",
        level: (10, 25),
        ticket: 71000,
        ticket_price: 2000,
        cooldown_secs: 1800,
        time_limit_secs: 600.0,
        floors: &[
            FloorDef {
                name: "1. Kat: Örümcek Yuvası",
                wmo: "x.wmo",
                offset: [0.0; 3],
                yaw: 0.0,
                entry: None,
                mobs: &[(210, 2)],
                objective: Objective::KillAll,
            },
            FloorDef {
                name: "Deneme Madeni",
                wmo: "y.wmo",
                offset: [0.0; 3],
                yaw: 0.0,
                entry: None,
                mobs: &[],
                objective: Objective::Boss { id: 225, adds: &[] },
            },
        ],
    };

    fn ok_ctx() -> EntryCtx {
        EntryCtx { level: 12, tickets: 1, now: 10_000, dead: false, riding: false, inside: false }
    }

    fn mets(c: &[Cond]) -> Vec<bool> {
        c.iter().map(|c| c.met).collect()
    }

    #[test]
    fn condition_colors() {
        let cds = DungeonCooldowns::default();
        // Hepsi yeşil: seviye, bilet, bekleme, hayatta, binekte değil.
        let c = conditions(&D, &ok_ctx(), &cds);
        assert_eq!(mets(&c), vec![true; 5]);
        assert!(c[1].text.contains("çantada 1"));
        // Her şart kendi satırında kırmızıya döner, diğerleri yeşil kalır (ilk hatada durmaz).
        let c = conditions(&D, &EntryCtx { level: 9, tickets: 0, dead: true, riding: true, ..ok_ctx() }, &cds);
        assert_eq!(mets(&c), vec![false, false, true, false, false]);
        assert!(c[0].text.contains("10-25") && c[0].text.contains('9'));
        assert!(c[1].text.ends_with("yok"));
        let mut cds2 = DungeonCooldowns::default();
        cds2.start(D.id, 10_000, D.cooldown_secs);
        let c = conditions(&D, &EntryCtx { now: 10_600, ..ok_ctx() }, &cds2);
        assert_eq!(mets(&c), vec![true, true, false, true, true]);
        assert!(c[2].text.contains("20 dk"));
        // Biletsiz zindanda bilet satırı yok.
        let free = DungeonDef { ticket: 0, ticket_price: 0, ..D };
        assert_eq!(conditions(&free, &EntryCtx { tickets: 0, ..ok_ctx() }, &cds).len(), 4);
        assert!(!sells_ticket(&free) && sells_ticket(&D));
    }

    /// Bütün satırlar yeşil ⇔ çekirdeğin `check_entry`'si geçer ("Gir" düğmesi ile renkler tutarlı).
    #[test]
    fn conditions_agree_with_check_entry() {
        let mut cds = DungeonCooldowns::default();
        cds.start(D.id, 10_000, D.cooldown_secs);
        for level in [9, 10, 25, 26] {
            for tickets in [0, 2] {
                for now in [10_100, 20_000] {
                    for dead in [false, true] {
                        for riding in [false, true] {
                            let ctx = EntryCtx { level, tickets, now, dead, riding, inside: false };
                            let all = conditions(&D, &ctx, &cds).iter().all(|c| c.met);
                            assert_eq!(all, check_entry(&D, &ctx, &cds).is_ok(), "{ctx:?}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn clock_and_titles() {
        assert_eq!(clock(1800.0), "30:00");
        assert_eq!(clock(61.2), "01:02");
        assert_eq!(clock(0.0), "00:00");
        assert_eq!(clock(-3.0), "00:00");
        assert_eq!(clock(3725.0), "1:02:05");
        assert_eq!(duration(1800), "30 dk");
        assert_eq!(duration(2700), "45 dk");
        assert_eq!(duration(125), "2 dk 5 sn");
        assert_eq!(duration(3600), "1 sa 0 dk");
        assert_eq!(floor_title("2. Kat: Ağlarla Örülü Tüneller"), "Ağlarla Örülü Tüneller");
        assert_eq!(floor_title("Deneme Madeni"), "Deneme Madeni");
        assert_eq!(floor_title("Kat: X"), "Kat: X");
        let mut r = RunState::new(&D, [0.0; 3]);
        assert_eq!(floor_line(&r), "Kat 1/2: Örümcek Yuvası");
        r.floor = 1;
        assert_eq!(floor_line(&r), "Kat 2/2: Deneme Madeni");
    }

    #[test]
    fn band_fades_like_zone_text() {
        assert_eq!(band_alpha(-1.0, 1.0), 0.0);
        assert!((band_alpha(0.25, 1.0) - 0.5).abs() < 1e-5);
        assert_eq!(band_alpha(1.0, 1.0), 1.0);
        assert!((band_alpha(2.5, 1.0) - 0.5).abs() < 1e-5);
        assert_eq!(band_alpha(3.5, 1.0), 0.0);
        assert_eq!(band_alpha(100.0, 1.0), 0.0);
    }

    #[test]
    fn core_banners_are_replaced() {
        let r = RunState::new(&D, [0.0; 3]);
        assert!(is_dungeon_banner("D", &r));
        assert!(is_dungeon_banner("Kat temizlendi!", &r));
        assert!(is_dungeon_banner("D tamamlandı!", &r));
        assert!(is_dungeon_banner("Süre doldu!", &r));
        assert!(is_dungeon_banner("Bütün canavarları öldür", &r));
        assert!(is_dungeon_banner("1. Kat: Örümcek Yuvası", &r));
        assert!(!is_dungeon_banner("Mühür anahtarını buldun!", &r));
        assert!(!is_dungeon_banner("Seviye atladın!", &r));
    }

    #[test]
    fn death_hint_text() {
        assert!(crate::game::ui::hud::death_hint(true).starts_with("Kat başında"));
        assert!(crate::game::ui::hud::death_hint(false).starts_with("Köyde"));
    }
}
