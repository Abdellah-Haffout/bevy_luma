//! Thin system clipboard wrapper used by text inputs (copy / cut / paste).

use std::sync::{Mutex, OnceLock};

fn store() -> &'static Mutex<Option<arboard::Clipboard>> {
    static STORE: OnceLock<Mutex<Option<arboard::Clipboard>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(arboard::Clipboard::new().ok()))
}

/// Writes text to the system clipboard. Returns `true` on success.
pub fn set_clipboard_text(text: &str) -> bool {
    store()
        .lock()
        .ok()
        .and_then(|mut guard| {
            guard
                .as_mut()
                .map(|cb| cb.set_text(text.to_owned()).is_ok())
        })
        .unwrap_or(false)
}

/// Reads text from the system clipboard, if any is available.
pub fn get_clipboard_text() -> Option<String> {
    store()
        .lock()
        .ok()
        .and_then(|mut guard| guard.as_mut().and_then(|cb| cb.get_text().ok()))
}
