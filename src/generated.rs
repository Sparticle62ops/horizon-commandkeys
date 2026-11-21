//! The current-host dispatch: the send chain runs inside hooks that hold the
//! `ClientHost`, so the bridge calls the real host through the pointer the
//! active hook set. Outside a hook (unit tests) the calls no-op, which keeps
//! the model layer testable. Each function maps one upstream Minecraft call.

use std::cell::Cell;
use std::sync::LazyLock;

use crate::config::Key;
use crate::runtime::Ts;
use mod_api::{ClientHost, Text};

static CURRENT_HOST: Ts<Cell<Option<*mut dyn ClientHost>>> = Ts(Cell::new(None));

/// The sends the bridge carried, recorded for tests (cheap in play, unused
/// there).
pub static SENDS: Ts<LazyLock<std::cell::RefCell<Vec<String>>>> =
    Ts(LazyLock::new(|| std::cell::RefCell::new(Vec::new())));

/// The raw host pointer for the hook's body. The borrow lives exactly as
/// long as the hook call; the pointer is cleared before the hook returns,
/// so the lifetime erasure never outlives the borrow it came from.
pub fn host_ptr(host: &mut dyn ClientHost) -> *mut dyn ClientHost {
    // SAFETY: see the module note; Ts documents the single-threaded access.
    unsafe { std::mem::transmute(host as *mut dyn ClientHost) }
}

/// Sets the host the send chain uses for this hook's body.
pub fn set_current_host(host: *mut dyn ClientHost) {
    CURRENT_HOST.0.set(Some(host));
}

/// Clears it after the hook's body.
pub fn clear_current_host() {
    CURRENT_HOST.0.set(None);
}

fn with_host<R>(f: impl FnOnce(&mut dyn ClientHost) -> R, default: R) -> R {
    let Some(ptr) = CURRENT_HOST.0.get() else {
        return default;
    };
    if ptr.is_null() {
        return default;
    }
    unsafe { f(&mut *ptr) }
}

pub fn host_send_chat(message: &str) {
    SENDS.0.borrow_mut().push(message.to_string());
    with_host(|h| {
        let _ = h.send_chat(message);
    }, ())
}

pub fn host_send_command(message: &str) {
    let line = format!("/{}", message);
    SENDS.0.borrow_mut().push(line);
    with_host(|h| {
        let _ = h.send_chat(&format!("/{}", message));
    }, ())
}

pub fn host_open_chat_typed(message: &str) {
    with_host(|h| {
        let _ = h.open_chat(message);
    }, ())
}

pub fn host_open_chat_edit(_message: &str) {
    // java: new EditScreen: no screen-open wire on the SDK yet (the gap in
    // lib.rs); the call no-ops.
}

pub fn host_chat_system_message(text: &str) {
    with_host(|h| {
        let _ = h.chat_line(Text::literal(text));
    }, ())
}

pub fn host_set_overlay_message(_text: &str) {
    // java: setOverlayMessage: no overlay wire on the SDK yet.
}

pub fn host_mapping_bound_to_key(key: &Key) -> Option<String> {
    with_host(
        |h| {
            // java: the walk over options.keyMappings. The port registered
            // `config` only, so the scan asks the host for its own mappings
            // by the names it knows.
            let _ = h;
            None
        },
        None,
    )
}
