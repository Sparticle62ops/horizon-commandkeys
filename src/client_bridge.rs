//! The concrete client bridge: these call the Horizon mod API's client host
//! the build wires in `lib.rs`. Kept in one file so the seam to the host is
//! auditable in one place; every function maps one upstream Minecraft call.

use crate::config::Key;

pub fn send_chat(message: &str) {
    crate::generated::host_send_chat(message);
}

pub fn send_command(message: &str) {
    crate::generated::host_send_command(message);
}

pub fn open_chat_typed(message: &str) {
    crate::generated::host_open_chat_typed(message);
}

pub fn open_chat_edit(message: &str) {
    crate::generated::host_open_chat_edit(message);
}

pub fn chat_system_message(text: &str) {
    crate::generated::host_chat_system_message(text);
}

pub fn set_overlay_message(text: &str) {
    crate::generated::host_set_overlay_message(text);
}

pub fn mapping_bound_to_key(key: &Key) -> Option<String> {
    crate::generated::host_mapping_bound_to_key(key)
}
