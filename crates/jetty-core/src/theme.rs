/// Ordered list of built-in theme preset names. Indices are stable so they
/// can be used as `theme_idx` in the app state (Ctrl+Shift+T cycles through).
pub const PRESETS: [&str; 4] = ["default_dark", "gruvbox_dark", "solarized_dark", "light"];

/// Terminal color theme: background, foreground, cursor, and the 16-color ANSI palette.
#[derive(Clone, Debug)]
pub struct Theme {
    pub name: &'static str,
    pub bg: [u8; 4],           // background RGBA (alpha < 255 => transparent)
    pub fg: [u8; 3],           // default foreground
    pub cursor: [u8; 3],       // cursor block color
    pub palette: [[u8; 3]; 16], // standard ANSI 0..=15
}

impl Theme {
    /// Resolve a theme by name string. Unknown names fall back to default_dark.
    pub fn by_name(name: &str) -> Theme {
        match name {
            "gruvbox_dark" => gruvbox_dark(),
            "solarized_dark" => solarized_dark(),
            "light" => light(),
            _ => default_dark(),
        }
    }
}

/// Default dark theme — values match the old hardcoded constants so existing
/// color tests continue to pass (red == [205,0,0], etc.).
pub fn default_dark() -> Theme {
    Theme {
        name: "default_dark",
        bg: [18, 18, 23, 255],
        fg: [220, 220, 220],
        cursor: [200, 200, 200],
        palette: [
            [0, 0, 0],       // 0  black
            [205, 0, 0],     // 1  red
            [0, 205, 0],     // 2  green
            [205, 205, 0],   // 3  yellow
            [0, 0, 238],     // 4  blue
            [205, 0, 205],   // 5  magenta
            [0, 205, 205],   // 6  cyan
            [229, 229, 229], // 7  white
            [127, 127, 127], // 8  bright black
            [255, 0, 0],     // 9  bright red
            [0, 255, 0],     // 10 bright green
            [255, 255, 0],   // 11 bright yellow
            [92, 92, 255],   // 12 bright blue
            [255, 0, 255],   // 13 bright magenta
            [0, 255, 255],   // 14 bright cyan
            [255, 255, 255], // 15 bright white
        ],
    }
}

/// Gruvbox Dark theme — well-known retro-groove color scheme.
pub fn gruvbox_dark() -> Theme {
    Theme {
        name: "gruvbox_dark",
        bg: [40, 40, 40, 255],
        fg: [235, 219, 178],
        cursor: [251, 241, 199],
        palette: [
            [40, 40, 40],    // 0  black (dark0)
            [204, 36, 29],   // 1  red
            [152, 151, 26],  // 2  green
            [215, 153, 33],  // 3  yellow
            [69, 133, 136],  // 4  blue
            [177, 98, 134],  // 5  magenta
            [104, 157, 106], // 6  cyan
            [168, 153, 132], // 7  white (light4)
            [146, 131, 116], // 8  bright black (gray)
            [251, 73, 52],   // 9  bright red
            [184, 187, 38],  // 10 bright green
            [250, 189, 47],  // 11 bright yellow
            [131, 165, 152], // 12 bright blue
            [211, 134, 155], // 13 bright magenta
            [142, 192, 124], // 14 bright cyan
            [235, 219, 178], // 15 bright white (fg1)
        ],
    }
}

/// Solarized Dark theme — Ethan Schoonover's classic low-contrast design.
pub fn solarized_dark() -> Theme {
    Theme {
        name: "solarized_dark",
        bg: [0, 43, 54, 255],
        fg: [131, 148, 150],
        cursor: [147, 161, 161],
        palette: [
            [7, 54, 66],     // 0  black (base02)
            [220, 50, 47],   // 1  red
            [133, 153, 0],   // 2  green
            [181, 137, 0],   // 3  yellow
            [38, 139, 210],  // 4  blue
            [211, 54, 130],  // 5  magenta
            [42, 161, 152],  // 6  cyan
            [238, 232, 213], // 7  white (base2)
            [0, 43, 54],     // 8  bright black (base03)
            [203, 75, 22],   // 9  bright red (orange)
            [88, 110, 117],  // 10 bright green (base01)
            [101, 123, 131], // 11 bright yellow (base00)
            [131, 148, 150], // 12 bright blue (base0)
            [108, 113, 196], // 13 bright magenta (violet)
            [147, 161, 161], // 14 bright cyan (base1)
            [253, 246, 227], // 15 bright white (base3)
        ],
    }
}

/// Light theme — clean bright background suitable for daylight use.
pub fn light() -> Theme {
    Theme {
        name: "light",
        bg: [253, 246, 227, 255],
        fg: [101, 123, 131],
        cursor: [88, 110, 117],
        palette: [
            [7, 54, 66],     // 0  black
            [220, 50, 47],   // 1  red
            [133, 153, 0],   // 2  green
            [181, 137, 0],   // 3  yellow
            [38, 139, 210],  // 4  blue
            [211, 54, 130],  // 5  magenta
            [42, 161, 152],  // 6  cyan
            [238, 232, 213], // 7  white
            [0, 43, 54],     // 8  bright black
            [203, 75, 22],   // 9  bright red
            [88, 110, 117],  // 10 bright green
            [101, 123, 131], // 11 bright yellow
            [131, 148, 150], // 12 bright blue
            [108, 113, 196], // 13 bright magenta
            [147, 161, 161], // 14 bright cyan
            [253, 246, 227], // 15 bright white
        ],
    }
}
