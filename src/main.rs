//! AzerothMetin — Metin2 tarzı aksiyon RPG. Dünya (arazi, WMO, doodad, gökyüzü, ışık) benilla-world
//! motorundan gelir; oyun katmanı (oyuncu, moblar, metin taşları, envanter, artı basma) bu crate'te.

mod boot;
mod game;

fn main() -> bevy::app::AppExit {
    boot::run()
}
