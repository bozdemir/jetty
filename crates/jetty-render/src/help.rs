use crate::Rect;

/// The keyboard-shortcut rows shown in the Help overlay, one string per line.
/// Two bindings per line (separated by "·") to keep the panel compact.
pub const HELP_ROWS: [&str; 13] = [
    "F9 — Summon / hide",
    "Ctrl+Shift+T — New tab    ·  Ctrl+Shift+W — Close tab",
    "Ctrl+Tab / Ctrl+Shift+Tab — Next / Prev tab",
    "Ctrl+1..9 — Jump to tab",
    "Double-click tab — Rename    ·  Double-click top bar — Maximize",
    "Ctrl+Shift+P — Settings",
    "Ctrl+= / Ctrl+- / Ctrl+0 — Font size",
    "Ctrl+Shift+C / Ctrl+Shift+V — Copy / Paste",
    "Right-click — Copy/Paste menu",
    "Drag top bar — Move window",
    "Drag edges/corners — Resize",
    "Ctrl+D — Close tab",
    "Esc — Close this help",
];

/// Geometry + draw data for the Help overlay.
pub struct HelpOverlay {
    /// Quads in draw order: full-screen dim, border, background panel.
    pub quads: Vec<Rect>,
    /// Text labels: (text, x, y, rgb) — title then one per shortcut row.
    pub labels: Vec<(String, f32, f32, [u8; 3])>,
    /// The panel rect (for hit-testing "click outside closes").
    pub panel: Rect,
}

/// Build the centered "Keyboard Shortcuts" help overlay for a window of size
/// `win_w`×`win_h` (physical pixels). The panel is sized to fit the rows and
/// clamped on-screen. A click outside `panel` (or Esc / the "?" button) closes it.
pub fn build_help_overlay(win_w: u32, win_h: u32) -> HelpOverlay {
    let sw = win_w as f32;
    let sh = win_h as f32;

    const PANEL_W: f32 = 520.0;
    const PAD: f32 = 20.0;
    const TITLE_H: f32 = 34.0;
    const ROW_H: f32 = 26.0;

    let rows = HELP_ROWS.len() as f32;
    let panel_h = (PAD + TITLE_H + rows * ROW_H + PAD).min(sh.max(0.0));
    let panel_w = PANEL_W.min(sw.max(0.0));

    let px = ((sw - panel_w) / 2.0).max(0.0).floor();
    let py = ((sh - panel_h) / 2.0).max(0.0).floor();

    let mut quads: Vec<Rect> = Vec::new();

    // Full-screen dim.
    quads.push(Rect { x: 0.0, y: 0.0, w: sw, h: sh, color: [0, 0, 0, 150] });
    // Border.
    quads.push(Rect {
        x: px - 2.0,
        y: py - 2.0,
        w: panel_w + 4.0,
        h: panel_h + 4.0,
        color: [80, 80, 110, 255],
    });
    // Background panel.
    let panel = Rect { x: px, y: py, w: panel_w, h: panel_h, color: [26, 26, 34, 245] };
    quads.push(panel);

    let mut labels: Vec<(String, f32, f32, [u8; 3])> = Vec::new();

    // Title.
    labels.push((
        "Keyboard Shortcuts".to_string(),
        px + PAD,
        py + PAD,
        [235, 235, 245],
    ));

    // Shortcut rows.
    let rows_top = py + PAD + TITLE_H;
    for (i, row) in HELP_ROWS.iter().enumerate() {
        let y = rows_top + i as f32 * ROW_H;
        labels.push((row.to_string(), px + PAD, y, [200, 205, 220]));
    }

    HelpOverlay { quads, labels, panel }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_is_centered_and_on_screen() {
        let h = build_help_overlay(1000, 700);
        assert!(h.panel.x >= 0.0 && h.panel.y >= 0.0);
        assert!(h.panel.x + h.panel.w <= 1000.0 + 0.5);
        assert!(h.panel.y + h.panel.h <= 700.0 + 0.5);
        // Title + one label per row.
        assert_eq!(h.labels.len(), HELP_ROWS.len() + 1);
        assert_eq!(h.labels[0].0, "Keyboard Shortcuts");
    }

    #[test]
    fn lists_core_bindings() {
        let h = build_help_overlay(1000, 700);
        let joined: String = h.labels.iter().map(|l| l.0.clone()).collect::<Vec<_>>().join("\n");
        assert!(joined.contains("F9"));
        assert!(joined.contains("Ctrl+Shift+P"));
        assert!(joined.contains("Ctrl+D"));
    }
}
