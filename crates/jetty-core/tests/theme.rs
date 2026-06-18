// Tests for the theme system.
use jetty_core::{Terminal, Theme};

/// Unknown JETTY_THEME name falls back to default_dark background.
#[test]
fn unknown_theme_env_falls_back_to_default_dark() {
    // We use set_theme so env ordering doesn't affect other tests.
    let mut term = Terminal::new(80, 24);
    term.set_theme(Theme::by_name("nonexistent_theme_xyz"));
    assert_eq!(term.theme().bg, [18, 18, 23, 255]);
}

/// Setting a non-default theme changes the snapshot bg_rgba.
#[test]
fn set_theme_changes_snapshot_bg_rgba() {
    use jetty_core::theme::gruvbox_dark;
    let mut term = Terminal::new(80, 24);
    term.set_theme(gruvbox_dark());
    term.feed(b"x");
    let snap = term.snapshot();
    // Gruvbox dark bg is [40, 40, 40, 255]
    assert_eq!(snap.bg_rgba, [40, 40, 40, 255]);
}
