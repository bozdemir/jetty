/// Ordered list of built-in theme preset names. Indices are stable so they
/// can be used as `theme_idx` in the app state (Ctrl+Shift+T cycles through).
///
/// These are widely-loved community terminal palettes used verbatim (exact
/// hex), not hand-tuned approximations.
pub const PRESETS: [&str; 4] = ["catppuccin_mocha", "tokyo_night", "gruvbox_dark", "dracula"];

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
    /// Resolve a theme by name string. Unknown names fall back to catppuccin_mocha.
    pub fn by_name(name: &str) -> Theme {
        match name {
            "catppuccin_mocha" => catppuccin_mocha(),
            "tokyo_night" => tokyo_night(),
            "gruvbox_dark" => gruvbox_dark(),
            "dracula" => dracula(),
            "solarized_dark" => solarized_dark(),
            "default_dark" => default_dark(),
            "light" => light(),
            _ => catppuccin_mocha(),
        }
    }
}

/// Catppuccin Mocha — the soothing pastel dark theme (catppuccin.com). Default.
pub fn catppuccin_mocha() -> Theme {
    Theme {
        name: "catppuccin_mocha",
        bg: [30, 30, 46, 255],   // base   #1e1e2e
        fg: [205, 214, 244],     // text   #cdd6f4
        cursor: [245, 224, 220], // rosewater #f5e0dc
        palette: [
            [69, 71, 90],    // 0  surface1 #45475a
            [243, 139, 168], // 1  red      #f38ba8
            [166, 227, 161], // 2  green    #a6e3a1
            [249, 226, 175], // 3  yellow   #f9e2af
            [137, 180, 250], // 4  blue     #89b4fa
            [245, 194, 231], // 5  pink     #f5c2e7
            [148, 226, 213], // 6  teal     #94e2d5
            [186, 194, 222], // 7  subtext1 #bac2de
            [88, 91, 112],   // 8  surface2 #585b70
            [243, 139, 168], // 9  red      #f38ba8
            [166, 227, 161], // 10 green    #a6e3a1
            [249, 226, 175], // 11 yellow   #f9e2af
            [137, 180, 250], // 12 blue     #89b4fa
            [245, 194, 231], // 13 pink     #f5c2e7
            [148, 226, 213], // 14 teal     #94e2d5
            [166, 173, 200], // 15 subtext0 #a6adc8
        ],
    }
}

/// Tokyo Night — the popular dark blue scheme (enkia/tokyonight).
pub fn tokyo_night() -> Theme {
    Theme {
        name: "tokyo_night",
        bg: [26, 27, 38, 255],   // #1a1b26
        fg: [192, 202, 245],     // #c0caf5
        cursor: [192, 202, 245], // #c0caf5
        palette: [
            [21, 22, 30],    // 0  #15161e
            [247, 118, 142], // 1  #f7768e
            [158, 206, 106], // 2  #9ece6a
            [224, 175, 104], // 3  #e0af68
            [122, 162, 247], // 4  #7aa2f7
            [187, 154, 247], // 5  #bb9af7
            [125, 207, 255], // 6  #7dcfff
            [169, 177, 214], // 7  #a9b1d6
            [65, 72, 104],   // 8  #414868
            [247, 118, 142], // 9  #f7768e
            [158, 206, 106], // 10 #9ece6a
            [224, 175, 104], // 11 #e0af68
            [122, 162, 247], // 12 #7aa2f7
            [187, 154, 247], // 13 #bb9af7
            [125, 207, 255], // 14 #7dcfff
            [192, 202, 245], // 15 #c0caf5
        ],
    }
}

/// Gruvbox Dark — Pavel Pertsev's retro-groove color scheme.
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

/// Dracula — the famous dark theme (draculatheme.com).
pub fn dracula() -> Theme {
    Theme {
        name: "dracula",
        bg: [40, 42, 54, 255],   // #282a36
        fg: [248, 248, 242],     // #f8f8f2
        cursor: [248, 248, 242], // #f8f8f2
        palette: [
            [33, 34, 44],    // 0  #21222c
            [255, 85, 85],   // 1  #ff5555
            [80, 250, 123],  // 2  #50fa7b
            [241, 250, 140], // 3  #f1fa8c
            [189, 147, 249], // 4  #bd93f9
            [255, 121, 198], // 5  #ff79c6
            [139, 233, 253], // 6  #8be9fd
            [248, 248, 242], // 7  #f8f8f2
            [98, 114, 164],  // 8  #6272a4
            [255, 110, 110], // 9  #ff6e6e
            [105, 255, 148], // 10 #69ff94
            [255, 255, 165], // 11 #ffffa5
            [214, 172, 255], // 12 #d6acff
            [255, 146, 223], // 13 #ff92df
            [164, 255, 255], // 14 #a4ffff
            [255, 255, 255], // 15 #ffffff
        ],
    }
}

/// Solarized Dark — Ethan Schoonover's classic low-contrast design.
/// Available via name (not in the default PRESETS chip row).
pub fn solarized_dark() -> Theme {
    Theme {
        name: "solarized_dark",
        bg: [0, 43, 54, 255],
        fg: [131, 148, 150],
        cursor: [147, 161, 161],
        palette: [
            [7, 54, 66],     // 0  base02
            [220, 50, 47],   // 1  red
            [133, 153, 0],   // 2  green
            [181, 137, 0],   // 3  yellow
            [38, 139, 210],  // 4  blue
            [211, 54, 130],  // 5  magenta
            [42, 161, 152],  // 6  cyan
            [238, 232, 213], // 7  base2
            [0, 43, 54],     // 8  base03
            [203, 75, 22],   // 9  orange
            [88, 110, 117],  // 10 base01
            [101, 123, 131], // 11 base00
            [131, 148, 150], // 12 base0
            [108, 113, 196], // 13 violet
            [147, 161, 161], // 14 base1
            [253, 246, 227], // 15 base3
        ],
    }
}

/// Generic xterm-style dark palette. Kept as a fallback / baseline option.
pub fn default_dark() -> Theme {
    Theme {
        name: "default_dark",
        bg: [18, 18, 23, 255],
        fg: [220, 220, 220],
        cursor: [200, 200, 200],
        palette: [
            [0, 0, 0],
            [205, 0, 0],
            [0, 205, 0],
            [205, 205, 0],
            [0, 0, 238],
            [205, 0, 205],
            [0, 205, 205],
            [229, 229, 229],
            [127, 127, 127],
            [255, 0, 0],
            [0, 255, 0],
            [255, 255, 0],
            [92, 92, 255],
            [255, 0, 255],
            [0, 255, 255],
            [255, 255, 255],
        ],
    }
}

/// Solarized Light — clean bright background. Available via name.
pub fn light() -> Theme {
    Theme {
        name: "light",
        bg: [253, 246, 227, 255],
        fg: [101, 123, 131],
        cursor: [88, 110, 117],
        palette: [
            [7, 54, 66],
            [220, 50, 47],
            [133, 153, 0],
            [181, 137, 0],
            [38, 139, 210],
            [211, 54, 130],
            [42, 161, 152],
            [238, 232, 213],
            [0, 43, 54],
            [203, 75, 22],
            [88, 110, 117],
            [101, 123, 131],
            [131, 148, 150],
            [108, 113, 196],
            [147, 161, 161],
            [253, 246, 227],
        ],
    }
}
