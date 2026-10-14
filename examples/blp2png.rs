//! Geliştirme aracı: BLP dokularını PNG'ye çevirir (UI atlaslarını incelemek için).
//! Kullanım: `cargo run --release --example blp2png -- a.blp b.blp ...` → yanlarına `a.png`, `b.png`.

fn main() {
    for arg in std::env::args().skip(1) {
        let src = std::path::Path::new(&arg);
        let bytes = std::fs::read(src).expect("BLP okunamadı");
        let out = src.with_extension("png");
        match benilla_formats::blp_to_png(&bytes, &out) {
            Ok((w, h)) => println!("{} {w}x{h}", out.display()),
            Err(e) => println!("{}: HATA {e}", src.display()),
        }
    }
}
