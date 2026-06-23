use crate::Rect;

/// The three items in the right-click context menu, in display order.
pub const MENU_ITEMS: [&str; 3] = ["Copy", "Paste", "Select All"];

const MENU_W: f32 = 150.0;
const ROW_H: f32 = 28.0;
const MENU_H: f32 = ROW_H * MENU_ITEMS.len() as f32;
const BORDER: f32 = 1.0;

/// Geometry and draw data for the right-click context menu.
pub struct ContextMenu {
    /// Quads in draw order: background panel, optional border, hover highlight.
    pub quads: Vec<Rect>,
    /// Text labels: (text, x, y, rgb).
    pub labels: Vec<(String, f32, f32, [u8; 3])>,
    /// Hit-test rects, one per item in MENU_ITEMS order.
    pub item_rects: Vec<Rect>,
}

/// Build the right-click context menu anchored at `(x, y)` (physical pixels).
///
/// The menu is clamped so its right and bottom edges stay within the window.
/// `hovered` is the index (0-based) of the item under the cursor, if any.
pub fn build_context_menu(
    x: f32,
    y: f32,
    win_w: u32,
    win_h: u32,
    hovered: Option<usize>,
) -> ContextMenu {
    let sw = win_w as f32;
    let sh = win_h as f32;

    // Clamp so the full menu (plus border) stays on-screen.
    let total_w = MENU_W + BORDER * 2.0;
    let total_h = MENU_H + BORDER * 2.0;
    let mx = x.min(sw - total_w).max(0.0);
    let my = y.min(sh - total_h).max(0.0);

    // Content area (inside the border).
    let cx = mx + BORDER;
    let cy = my + BORDER;

    // Build item rects first (also serve as hit-test rects).
    let mut item_rects: Vec<Rect> = Vec::with_capacity(MENU_ITEMS.len());
    for i in 0..MENU_ITEMS.len() {
        item_rects.push(Rect {
            x: cx,
            y: cy + i as f32 * ROW_H,
            w: MENU_W,
            h: ROW_H,
            color: [38, 38, 50, 255], // default row bg
        });
    }

    let mut quads: Vec<Rect> = Vec::new();

    // Outer border quad (slightly larger than content).
    quads.push(Rect {
        x: mx,
        y: my,
        w: total_w,
        h: total_h,
        color: [90, 90, 110, 255],
    });

    // Background panel.
    quads.push(Rect {
        x: cx,
        y: cy,
        w: MENU_W,
        h: MENU_H,
        color: [30, 30, 40, 245],
    });

    // Hover highlight quad (drawn on top of background, under labels).
    if let Some(idx) = hovered {
        if idx < MENU_ITEMS.len() {
            quads.push(Rect {
                x: cx,
                y: cy + idx as f32 * ROW_H,
                w: MENU_W,
                h: ROW_H,
                color: [60, 80, 120, 255],
            });
        }
    }

    // Labels.
    let mut labels: Vec<(String, f32, f32, [u8; 3])> = Vec::new();
    for (i, &name) in MENU_ITEMS.iter().enumerate() {
        let label_y = cy + i as f32 * ROW_H + 7.0; // 7px from row top
        labels.push((name.to_string(), cx + 10.0, label_y, [210, 210, 225]));
    }

    ContextMenu { quads, labels, item_rects }
}
