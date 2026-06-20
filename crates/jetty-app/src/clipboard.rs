/// Thin wrapper around `arboard::Clipboard` with lazy construction and
/// graceful error handling: a missing clipboard never panics.
use arboard::Clipboard;

/// Write `text` to the system clipboard. Errors are silently discarded.
pub fn set(text: &str) {
    match Clipboard::new() {
        Ok(mut cb) => { let _ = cb.set_text(text); }
        Err(_) => {}
    }
}

/// Read a `String` from the system clipboard. Returns `None` on error or when
/// the clipboard contains no text.
pub fn get() -> Option<String> {
    let mut cb = Clipboard::new().ok()?;
    cb.get_text().ok()
}
