//! Turns the saved `Appearance` into everything the view paints with: the rune-kit `Kit`, the
//! table style, and the few app colours the glass layout needs.

use rune::Color;
use rune_kit::{Kit, PaletteSeed, TableStyle};

use crate::model::{Appearance, ThemeMode};
use crate::ui::palettes::{self, Palette, Variant};

pub fn color(hex: u32) -> Color {
    rune::color::hex(&format!("#{hex:06x}")).unwrap_or(Color::WHITE)
}

/// The resolved look for one frame.
#[derive(Clone)]
pub struct Look {
    pub kit: Kit,
    pub dark: bool,
    pub palette: &'static Palette,
    pub v: Variant,
    pub accent: Color,
    pub accent_hex: u32,
    pub text: Color,
    pub muted: Color,
    /// A faint fill for rows and cards sitting on glass.
    pub card: Color,
    pub card_hover: Color,
    /// Hairlines on glass.
    pub line: Color,
    /// Readable ink on top of the accent.
    pub on_accent: Color,
}

pub fn resolve_dark(a: &Appearance, system_dark: bool) -> bool {
    match a.mode {
        ThemeMode::System => system_dark,
        ThemeMode::Light => false,
        ThemeMode::Dark => true,
    }
}

pub fn look(a: &Appearance, system_dark: bool) -> Look {
    let dark = resolve_dark(a, system_dark);
    let palette = palettes::find(&a.palette);
    let v = *palette.variant(dark);
    let accent_hex = v.accents[a.accent % v.accents.len()].1;
    let accent = color(accent_hex);
    let text = color(v.text);
    let mut kit = Kit::from_palette(PaletteSeed { bg: color(v.bg), fg: text, accent });
    kit.radius.sm = 8.0;
    kit.radius.md = 12.0;
    let ink = |a: f32| text.alpha(a);
    // Pick black or white ink for text set on the accent, by the accent's luminance.
    let lum = {
        let f = |s: u32| ((accent_hex >> s) & 0xff) as f32 / 255.0;
        0.2126 * f(16) + 0.7152 * f(8) + 0.0722 * f(0)
    };
    Look {
        kit,
        dark,
        palette,
        v,
        accent,
        accent_hex,
        text,
        // Palette "muted" tones are tuned for flat backgrounds; on tinted glass they need to sit
        // closer to the text colour to stay readable, light themes most of all.
        muted: color(palettes::mix(v.muted, v.text, if dark { 0.22 } else { 0.5 })),
        card: ink(if dark { 0.06 } else { 0.05 }),
        card_hover: ink(if dark { 0.1 } else { 0.08 }),
        line: ink(if dark { 0.12 } else { 0.1 }),
        on_accent: if lum > 0.6 { color(0x14121a) } else { Color::WHITE },
    }
}

impl Look {
    /// Table colours for a table drawn straight on glass: no opaque fills, palette ink.
    pub fn table_style(&self) -> TableStyle {
        let mut t = TableStyle::from_kit(&self.kit);
        t.row_height = 30.0;
        t.header_height = 32.0;
        t.text = self.text;
        t.muted = self.muted;
        t.header_text = self.muted;
        t.rule = self.line;
        t.selected = self.accent.alpha(if self.dark { 0.28 } else { 0.22 });
        t.stripe = self.text.alpha(if self.dark { 0.035 } else { 0.03 });
        t
    }
}
