//! Colour palettes. Each family has an official light and dark variant; the hex values are the
//! families' own published colours (sources noted per palette), except Rune Aurora, which is ours
//! and follows the showreel's glass scene.

/// One side (light or dark) of a palette.
#[derive(Clone, Copy, Debug)]
pub struct Variant {
    /// The window's base colour.
    pub bg: u32,
    /// The colour glass panels are tinted toward.
    pub surface: u32,
    pub text: u32,
    pub muted: u32,
    /// Accents the user can pick from, by name.
    pub accents: [(&'static str, u32); 6],
}

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub id: &'static str,
    pub name: &'static str,
    /// The variants' own names, e.g. "Latte" / "Mocha".
    pub light_name: &'static str,
    pub dark_name: &'static str,
    pub source: &'static str,
    pub light: Variant,
    pub dark: Variant,
}

impl Palette {
    pub fn variant(&self, dark: bool) -> &Variant {
        if dark { &self.dark } else { &self.light }
    }
}

pub const DEFAULT: &str = "aurora";

pub const ALL: [Palette; 6] = [
    Palette {
        id: "aurora",
        name: "Rune Aurora",
        light_name: "Dawn",
        dark_name: "Night",
        source: "winkillswitch",
        dark: Variant {
            bg: 0x120a1e,
            surface: 0x1d1430,
            text: 0xf1ecff,
            muted: 0xa99cc8,
            accents: [("Violet", 0x7c5cff), ("Pink", 0xff5ca8), ("Cyan", 0x22d3ee), ("Blue", 0x4f8dff), ("Mint", 0x34d399), ("Amber", 0xffb547)],
        },
        light: Variant {
            bg: 0xf6f1ff,
            surface: 0xffffff,
            text: 0x2a1f45,
            muted: 0x6f6390,
            accents: [("Violet", 0x6d4aff), ("Pink", 0xe0408d), ("Cyan", 0x0891b2), ("Blue", 0x2f6fe4), ("Mint", 0x059669), ("Amber", 0xd97706)],
        },
    },
    Palette {
        id: "catppuccin",
        name: "Catppuccin",
        light_name: "Latte",
        dark_name: "Mocha",
        source: "catppuccin.com/palette",
        dark: Variant {
            bg: 0x1e1e2e,
            surface: 0x313244,
            text: 0xcdd6f4,
            muted: 0xa6adc8,
            accents: [("Mauve", 0xcba6f7), ("Blue", 0x89b4fa), ("Pink", 0xf5c2e7), ("Peach", 0xfab387), ("Green", 0xa6e3a1), ("Teal", 0x94e2d5)],
        },
        light: Variant {
            bg: 0xeff1f5,
            surface: 0xe6e9ef,
            text: 0x4c4f69,
            muted: 0x6c6f85,
            accents: [("Mauve", 0x8839ef), ("Blue", 0x1e66f5), ("Pink", 0xea76cb), ("Peach", 0xfe640b), ("Green", 0x40a02b), ("Teal", 0x179299)],
        },
    },
    Palette {
        id: "rose-pine",
        name: "Rosé Pine",
        light_name: "Dawn",
        dark_name: "Moon",
        source: "github.com/rose-pine/palette",
        dark: Variant {
            bg: 0x232136,
            surface: 0x2a273f,
            text: 0xe0def4,
            muted: 0x908caa,
            accents: [("Iris", 0xc4a7e7), ("Foam", 0x9ccfd8), ("Rose", 0xea9a97), ("Love", 0xeb6f92), ("Gold", 0xf6c177), ("Pine", 0x3e8fb0)],
        },
        light: Variant {
            bg: 0xfaf4ed,
            surface: 0xfffaf3,
            text: 0x464261,
            muted: 0x797593,
            accents: [("Iris", 0x907aa9), ("Pine", 0x286983), ("Rose", 0xd7827e), ("Love", 0xb4637a), ("Gold", 0xea9d34), ("Foam", 0x56949f)],
        },
    },
    Palette {
        id: "tokyo-night",
        name: "Tokyo Night",
        light_name: "Day",
        dark_name: "Night",
        source: "github.com/folke/tokyonight.nvim",
        dark: Variant {
            bg: 0x1a1b26,
            surface: 0x292e42,
            text: 0xc0caf5,
            muted: 0x8790b8,
            accents: [("Blue", 0x7aa2f7), ("Magenta", 0xbb9af7), ("Cyan", 0x7dcfff), ("Orange", 0xff9e64), ("Green", 0x9ece6a), ("Red", 0xf7768e)],
        },
        light: Variant {
            bg: 0xe1e2e7,
            surface: 0xd0d5e3,
            text: 0x3760bf,
            muted: 0x848cb5,
            accents: [("Blue", 0x2e7de9), ("Magenta", 0x9854f1), ("Cyan", 0x007197), ("Orange", 0xb15c00), ("Green", 0x587539), ("Red", 0xf52a65)],
        },
    },
    Palette {
        id: "kanagawa",
        name: "Kanagawa",
        light_name: "Lotus",
        dark_name: "Wave",
        source: "github.com/rebelot/kanagawa.nvim",
        dark: Variant {
            bg: 0x1f1f28,
            surface: 0x2a2a37,
            text: 0xdcd7ba,
            muted: 0x9a9781,
            accents: [("Crystal Blue", 0x7e9cd8), ("Oni Violet", 0x957fb8), ("Sakura", 0xd27e99), ("Surimi", 0xffa066), ("Spring Green", 0x98bb6c), ("Wave Aqua", 0x7aa89f)],
        },
        light: Variant {
            bg: 0xf2ecbc,
            surface: 0xe7dba0,
            text: 0x545464,
            muted: 0x8a8980,
            accents: [("Blue", 0x4d699b), ("Violet", 0x624c83), ("Pink", 0xb35b79), ("Orange", 0xcc6d00), ("Green", 0x6f894e), ("Aqua", 0x597b75)],
        },
    },
    Palette {
        id: "everforest",
        name: "Everforest",
        light_name: "Light",
        dark_name: "Dark",
        source: "github.com/sainnhe/everforest",
        dark: Variant {
            bg: 0x2d353b,
            surface: 0x343f44,
            text: 0xd3c6aa,
            muted: 0x859289,
            accents: [("Green", 0xa7c080), ("Aqua", 0x83c092), ("Blue", 0x7fbbb3), ("Purple", 0xd699b6), ("Orange", 0xe69875), ("Yellow", 0xdbbc7f)],
        },
        light: Variant {
            bg: 0xfdf6e3,
            surface: 0xf4f0d9,
            text: 0x5c6a72,
            muted: 0x939f91,
            accents: [("Green", 0x8da101), ("Aqua", 0x35a77c), ("Blue", 0x3a94c5), ("Purple", 0xdf69ba), ("Orange", 0xf57d26), ("Yellow", 0xdfa000)],
        },
    },
];

pub fn find(id: &str) -> &'static Palette {
    ALL.iter().find(|p| p.id == id).unwrap_or(&ALL[0])
}

/// Linear mix of two 0xRRGGBB colours in sRGB, `t` toward `b`.
pub fn mix(a: u32, b: u32, t: f32) -> u32 {
    let ch = |c: u32, s: u32| ((c >> s) & 0xff) as f32;
    let m = |s: u32| (ch(a, s) + (ch(b, s) - ch(a, s)) * t).round().clamp(0.0, 255.0) as u32;
    (m(16) << 16) | (m(8) << 8) | m(0)
}

/// Five stops for the background mesh gradient: the base, three accents pulled toward the base so
/// they glow rather than shout, and a deeper base.
pub fn mesh(v: &Variant, accent: usize, dark: bool) -> [u32; 5] {
    let a = |i: usize| v.accents[(accent + i) % v.accents.len()].1;
    if dark {
        [mix(v.bg, 0x000000, 0.35), mix(a(0), v.bg, 0.42), mix(a(1), v.bg, 0.55), v.bg, mix(a(2), v.bg, 0.5)]
    } else {
        // Light: pastel, so the glass above stays bright enough for dark ink.
        [v.bg, mix(a(0), v.bg, 0.7), mix(a(1), v.bg, 0.76), v.surface, mix(a(2), v.bg, 0.74)]
    }
}
