// Verifies ANSI / xterm-256 color resolution in the GridSnapshot.
use jetty_core::Terminal;

#[test]
fn named_ansi_color_maps_to_palette() {
    let mut term = Terminal::new(80, 24);
    term.feed(b"\x1b[31mX"); // SGR 31: red foreground
    let snap = term.snapshot();
    assert_eq!(snap.cell(0, 0).c, 'X');
    assert_eq!(snap.cell(0, 0).fg, [205, 0, 0]); // standard ANSI red
}

#[test]
fn bright_ansi_color_maps_to_palette() {
    let mut term = Terminal::new(80, 24);
    term.feed(b"\x1b[92mX"); // SGR 92: bright green foreground
    let snap = term.snapshot();
    assert_eq!(snap.cell(0, 0).fg, [0, 255, 0]);
}

#[test]
fn indexed_256_color_maps_to_cube() {
    let mut term = Terminal::new(80, 24);
    term.feed(b"\x1b[38;5;196mX"); // 256-color index 196
    let snap = term.snapshot();
    // 196-16=180 -> r=5,g=0,b=0 -> [255,0,0]
    assert_eq!(snap.cell(0, 0).fg, [255, 0, 0]);
}

#[test]
fn truecolor_fg_is_exact() {
    let mut term = Terminal::new(80, 24);
    term.feed(b"\x1b[38;2;10;20;30mY"); // SGR 38;2: 24-bit truecolor
    let snap = term.snapshot();
    assert_eq!(snap.cell(0, 0).fg, [10, 20, 30]);
}
