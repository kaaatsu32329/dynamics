//! plotters' `ab_glyph` backend has no default font, so we load a single-face
//! system TTF and register it as "sans-serif".
//! (Plot labels are ASCII-only, so a Latin font is enough.)

use plotters::style::{FontStyle, register_font};

/// Candidate system fonts (tried top to bottom).
const CANDIDATES: &[&str] = &[
    "/System/Library/Fonts/Geneva.ttf",
    "/System/Library/Fonts/Monaco.ttf",
    "/System/Library/Fonts/SFNS.ttf",
    "/System/Library/Fonts/Supplemental/Arial.ttf",
    "/Library/Fonts/Arial.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
];

/// Register "sans-serif" for all font styles.
pub fn register() -> Result<(), String> {
    let path = CANDIDATES
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .ok_or("使えるフォントが見つかりませんでした")?;
    let bytes = std::fs::read(path).map_err(|e| format!("フォント読込失敗 {path}: {e}"))?;
    // register_font requires &'static [u8], so leak the bytes once.
    let leaked: &'static [u8] = Box::leak(bytes.into_boxed_slice());
    for style in [
        FontStyle::Normal,
        FontStyle::Bold,
        FontStyle::Italic,
        FontStyle::Oblique,
    ] {
        register_font("sans-serif", style, leaked)
            .map_err(|_| format!("フォント登録に失敗しました: {path}"))?;
    }
    Ok(())
}
