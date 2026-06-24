use crate::Rect;
use jetty_core::Theme;

/// Height of the tab bar in physical pixels. The terminal grid is offset down by
/// this amount; pixel↔cell math for the main grid subtracts it.
pub const TABBAR_H: f32 = 32.0;

/// Width of a single tab in physical pixels.
const TAB_W: f32 = 140.0;
/// Width of the "+" new-tab button.
const PLUS_W: f32 = 32.0;
/// Size of the "×" close hit box at the right of each tab.
const CLOSE_W: f32 = 18.0;

/// Geometry + draw data for the tab bar.
pub struct TabBar {
    /// Quads in draw order: bar background, then per-tab backgrounds + plus button.
    pub quads: Vec<Rect>,
    /// Text labels: (text, x, y, rgb) — tab titles, close glyphs, the plus glyph.
    pub labels: Vec<(String, f32, f32, [u8; 3])>,
    /// One hit-test rect per tab (full tab area, for switching).
    pub tab_rects: Vec<Rect>,
    /// One hit-test rect per tab for its "×" close affordance.
    pub close_rects: Vec<Rect>,
    /// Hit-test rect for the "+" new-tab button.
    pub plus_rect: Rect,
}

/// Build the tab bar across the top of the window.
///
/// `tabs` is `(title, is_active)` per tab in order; `theme` supplies colors so
/// the bar matches the active terminal theme. The active tab is highlighted with
/// the theme accent (palette blue); inactive tabs are dimmer.
pub fn build_tab_bar(width: u32, tabs: &[(String, bool)], theme: &Theme) -> TabBar {
    let sw = width as f32;
    let h = TABBAR_H;

    // Theme-derived colors.
    let bg = [theme.bg[0], theme.bg[1], theme.bg[2], 255];
    let accent = theme.palette[4]; // blue
    let active_bg = [accent[0], accent[1], accent[2], 255];
    // Inactive tab: a dim blend toward the accent so it reads as part of the bar.
    let inactive_bg = [
        ((bg[0] as u16 + accent[0] as u16) / 3) as u8,
        ((bg[1] as u16 + accent[1] as u16) / 3) as u8,
        ((bg[2] as u16 + accent[2] as u16) / 3) as u8,
        255,
    ];
    let fg = theme.fg;
    let dim_fg = [
        (fg[0] as u16 * 2 / 3) as u8,
        (fg[1] as u16 * 2 / 3) as u8,
        (fg[2] as u16 * 2 / 3) as u8,
    ];

    let mut quads: Vec<Rect> = Vec::new();
    let mut labels: Vec<(String, f32, f32, [u8; 3])> = Vec::new();
    let mut tab_rects: Vec<Rect> = Vec::new();
    let mut close_rects: Vec<Rect> = Vec::new();

    // Bar background spanning the full width.
    quads.push(Rect { x: 0.0, y: 0.0, w: sw, h, color: bg });

    let mut x = 0.0_f32;
    for (title, active) in tabs.iter() {
        let tab_rect = Rect { x, y: 0.0, w: TAB_W, h, color: if *active { active_bg } else { inactive_bg } };
        // Tab background (inset by 1px at the bottom to leave a separator line look).
        quads.push(Rect { x: x + 1.0, y: 2.0, w: TAB_W - 2.0, h: h - 2.0, color: tab_rect.color });

        // Truncated title label.
        let max_chars = 14usize;
        let shown: String = if title.chars().count() > max_chars {
            let mut s: String = title.chars().take(max_chars - 1).collect();
            s.push('…');
            s
        } else {
            title.clone()
        };
        let label_color = if *active { fg } else { dim_fg };
        labels.push((shown, x + 10.0, 8.0, label_color));

        // Close "×" at the tab's right.
        let close_x = x + TAB_W - CLOSE_W - 4.0;
        let close_rect = Rect { x: close_x, y: 0.0, w: CLOSE_W, h, color: tab_rect.color };
        labels.push(("×".to_string(), close_x + 4.0, 8.0, label_color));
        close_rects.push(close_rect);

        tab_rects.push(tab_rect);
        x += TAB_W;
    }

    // "+" new-tab button after the last tab.
    let plus_rect = Rect { x, y: 0.0, w: PLUS_W, h, color: inactive_bg };
    quads.push(Rect { x: x + 1.0, y: 2.0, w: PLUS_W - 2.0, h: h - 2.0, color: inactive_bg });
    labels.push(("+".to_string(), x + 11.0, 7.0, fg));

    TabBar { quads, labels, tab_rects, close_rects, plus_rect }
}
