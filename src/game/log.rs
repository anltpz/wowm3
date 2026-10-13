//! Sistem mesajları (sol alttaki sohbet kutusu gibi): "Kurt Pençesi aldın" vb.

use bevy::prelude::*;

#[derive(Clone)]
pub struct LogLine {
    pub text: String,
    pub color: Color,
    pub at: f32,
}

#[derive(Resource, Default)]
pub struct GameLog {
    pub lines: Vec<LogLine>,
    pub rev: u64,
    /// Ekranın ortasında kısa süre görünen büyük duyuru (seviye atlama, yükseltme sonucu).
    pub banner: Option<(String, Color, f32)>,
}

pub const C_INFO: Color = Color::srgb(0.92, 0.92, 0.85);
pub const C_ITEM: Color = Color::srgb(1.0, 0.85, 0.35);
pub const C_GOOD: Color = Color::srgb(0.45, 1.0, 0.45);
pub const C_BAD: Color = Color::srgb(1.0, 0.4, 0.35);
pub const C_EXP: Color = Color::srgb(0.75, 0.6, 1.0);

impl GameLog {
    pub fn push(&mut self, now: f32, color: Color, text: impl Into<String>) {
        self.lines.push(LogLine {
            text: text.into(),
            color,
            at: now,
        });
        if self.lines.len() > 60 {
            self.lines.remove(0);
        }
        self.rev += 1;
    }
    pub fn banner(&mut self, now: f32, color: Color, text: impl Into<String>) {
        self.banner = Some((text.into(), color, now));
        self.rev += 1;
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<GameLog>();
}
