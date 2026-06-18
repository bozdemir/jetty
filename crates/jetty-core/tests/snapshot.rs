use jetty_core::Terminal;

#[test]
fn plain_text_lands_in_the_grid() {
    let mut term = Terminal::new(80, 24);
    term.feed(b"hello");
    let snap = term.snapshot();
    assert_eq!(snap.cols, 80);
    assert_eq!(snap.rows, 24);
    assert!(snap.row_text(0).starts_with("hello"), "row0 = {:?}", snap.row_text(0));
}

#[test]
fn newline_moves_to_next_row() {
    let mut term = Terminal::new(80, 24);
    term.feed(b"a\r\nb");
    let snap = term.snapshot();
    assert!(snap.row_text(0).starts_with('a'));
    assert!(snap.row_text(1).starts_with('b'));
}
