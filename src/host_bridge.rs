//! The SDK host glue: everything upstream does through Minecraft's client
//! classes directly (send chat/command, screens, mappings, world reads) the
//! port does through the mod API host. Where the SDK has no equivalent read,
//! the port keeps its own equivalent state and says so in the seam docs.

use crate::config::Key;

/// java: mc.player.connection.sendChat(message).
pub fn send_chat(message: &str) {
    crate::client_bridge::send_chat(message);
}

/// java: mc.player.connection.sendCommand(message).
pub fn send_command(message: &str) {
    crate::client_bridge::send_command(message);
}

/// java: new ChatScreen(message); the SDK's chat-open hook (types the text).
pub fn open_chat_typed(message: &str) {
    crate::client_bridge::open_chat_typed(message);
}

/// java: new EditScreen: opens the chat edit box with the text.
pub fn open_chat_edit(message: &str) {
    crate::client_bridge::open_chat_edit(message);
}

/// java: addClientSystemMessage.
pub fn chat_system_message(text: &str) {
    crate::client_bridge::chat_system_message(text);
}

/// java: setOverlayMessage.
pub fn set_overlay_message(text: &str) {
    crate::client_bridge::set_overlay_message(text);
}

/// java: getConflict(key) over options.keyMappings. The SDK surfaces the mod
/// API's own registered mappings by name and reports the assigned key of a
/// mapping when asked; the port walks the names it registered and reports the
/// first bound to `key`. Vanilla's own bindings are not readable, which is
/// also true upstream when the API cannot see them.
pub fn mapping_bound_to_key(key: &Key) -> Option<String> {
    crate::client_bridge::mapping_bound_to_key(key)
}
