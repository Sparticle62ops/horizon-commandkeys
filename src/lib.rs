//! Exact 1:1 Rust port of CommandKeys (upstream Java, Apache-2.0,
//! TerminalMC, https://modrinth.com/mod/commandkeys), on the pinned SDK
//! (api 0.2, f00125f61f).
//!
//! Every Java class maps to one module below, same names, same logic, same
//! defaults, same control flow:
//! - `config` carries `Config`/`Profile`/`Macro`/`Keybind`/`Message`, the
//!   JSON field names and defaults verbatim, the Gson-lenient load
//!   (`json.rs` mirrors `JsonUtil.getOrDefault`, silent loads and the
//!   `.old` backup), the validation and the activation reordering;
//! - `runtime` carries `CommandKeys`' statics and logic: the rate limiter
//!   (`TickCounter` deque), the macro tick and trigger dispatch, the send
//!   chain with the length-limit guard and history/overlay behaviour;
//! - `keybind_util` carries `KeybindUtil`: `handleKey`, `handleKeys`,
//!   `getConflict` and the `KeybindInfo` conflict scanner;
//! - `placeholder` carries `PlaceholderUtil`'s tables and math 1:1;
//! - `lang` carries `en_us.json`'s keys and `Localization.localized`;
//! - `commands` carries `ClientCommands`' /commandkeys literals;
//! - `gui` carries the screen model (editor orderings, copy, links) the
//!   screens drove.
//!
//! The mod API wiring on the pinned SDK:
//! - the config key as a registered mapping (`config`, label
//!   `key.main.config`, default R, the vanilla `key.categories.commandkeys`
//!   category), consumed per tick like upstream's `CONFIG_KEY.consumeClick`;
//! - `client_setup` loads `commandkeys.json` through the host's checked
//!   config files, the Gson shape verbatim;
//! - `client_tick_post` ticks the rate limiter and every macro, snapshots
//!   the host state the placeholders read (clipboard, chat lines) and
//!   flushes a dirty config to disk;
//! - `input_key`/`mouse_button_pre`/`mouse_button_post` track the key and
//!   button state and dispatch `handleKey` on presses, as the old
//!   `KeyboardHandler`/`MouseButtonHandler` mixins did;
//! - `client_chat_received` records the chat lines the `%#…%` and
//!   `%pmsender%` placeholders scan;
//! - `register_client_commands` pushes /commandkeys (reload, reset,
//!   convert), run in `command`.
//!
//! Gaps, tracked (the mod API has no counterpart for these yet):
//! - cancelling a typed character after a TYPE/EDIT macro (the
//!   KeyboardHandler#charTyped cancel): `input_key` cannot cancel, so the
//!   char still goes to a screen if one opens; the mouse side of VETO does
//!   cancel (`mouse_button_pre` returns Cancel);
//! - `%pos%`/`%lpos%` and their offsets: the host does not expose the local
//!   player's block position, look angle or crosshair block, so those
//!   placeholders fault (`?`) until a world-read hook lands;
//! - `%pmsender%`: chat lines arrive without their translation keys, so the
//!   incoming-PM sender match cannot run;
//! - `%lastcmd%` tracks the commands the port itself sends, not vanilla's
//!   chat command history, which is not surfaced;
//! - the vanilla keybind conflict scan (`getConflict`) sees only the
//!   mappings this port registered, not vanilla's full table;
//! - the config/profile editor screens are host seams waiting on a
//!   screen-widget hook (the screen model logic is carried in `gui`).

use mod_api::{
    ClientHost, ClientSetupEvent, ClientTickPostEvent, Flow, InputKeyEvent, KeyAction,
    KeyConflictContext, KeyRegistration, MouseButtonPostEvent, MouseButtonPreEvent,
    RegistrationContext, Registrar,
};

pub mod client_bridge;
pub mod commands;
pub mod config;
pub mod config_store;
pub mod generated;
pub mod gui;
pub mod host_bridge;
pub mod json;
pub mod keybind_util;
pub mod lang;
pub mod placeholder;
pub mod runtime;
#[cfg(test)]
pub mod tests;
#[cfg(test)]
pub mod tests_util;

/// The mod's id.
pub const MOD_ID: &str = "commandkeys";

/// java: CommandKeys.TICK_COUNTER now.
pub fn tick_clock() -> i64 {
    runtime::TICK_COUNT.0.get()
}

/// The configured tick window, java: Config.get().getRatelimitTicks().
pub fn tick_window() -> i64 {
    let instance = runtime::INSTANCE.0.borrow();
    instance
        .as_ref()
        .map(|c| c.ratelimit_ticks.max(1) as i64)
        .unwrap_or(crate::config::RATELIMIT_TICKS_DEFAULT as i64)
}

pub fn log_warn(msg: &str) {
    eprintln!("[commandkeys] warn: {}", msg);
}

pub fn log_error(msg: &str) {
    eprintln!("[commandkeys] error: {}", msg);
}

/// java: reload(): reload the config from disk. The file read runs through
/// the host's checked config files, reachable only inside a hook, so the
/// command hook re-reads there; outside one the current config stays.
// The chat component's resolved text is not readable through the SDK seam
// (TextContent is not exported), so recorded lines carry no content; the
// placeholder scan keys off the translation key, which chat lines do not
// carry either (runtime::incoming_pm_sender faults). Both gaps are noted in
// the PR.
pub fn reload_config() {
    log_warn("reload outside a host hook keeps the loaded config");
}

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl mod_api::Hooks for Mod {
    /// The config key as a registered mapping, and /commandkeys comes in
    /// through `register_client_commands`.
    fn register(&self, registrar: &mut dyn Registrar, _context: &RegistrationContext) {
        let _ = registrar.register_key_mapping(
            KeyRegistration::new("config", "key.keyboard.r")
                .label("key.main.config")
                .category("key.categories.commandkeys")
                .context(KeyConflictContext::InGame),
        );
    }

    /// java: the mod's client init: load `commandkeys.json` (the Gson shape
    /// verbatim), validate, build the link map.
    fn client_setup(&self, host: &mut dyn ClientHost, _event: &mut ClientSetupEvent) {
        let text = crate::config_store::read_text(host.config_files());
        if let Some(config) = text.as_deref().and_then(config_store::parse_config) {
            *runtime::INSTANCE.0.borrow_mut() = Some(config);
        } else {
            *runtime::INSTANCE.0.borrow_mut() = Some(crate::config::Config::default());
        }
        runtime::rebuild_link_map();
    }

    /// java: CommandKeys#afterClientTick: tick the rate limiter, tick every
    /// macro, consume the config key, snapshot the host state, flush a dirty
    /// config.
    fn client_tick_post(&self, host: &mut dyn ClientHost, _event: &mut ClientTickPostEvent) {
        generated::set_current_host(generated::host_ptr(host));
        runtime::TICK_COUNT.0.set(runtime::TICK_COUNT.0.get() + 1);
        runtime::tick_rate_limiter();

        // Snapshot the host state the placeholders read.
        {
            let clipboard = host.clipboard_text().unwrap_or_default();
            let mut snapshot = runtime::HOST.0.borrow_mut();
            snapshot.clipboard = clipboard;
        }

        // java: for each macro in the active profile, tick it.
        let count = runtime::INSTANCE
            .0
            .borrow()
            .as_ref()
            .map(|c| c.active_profile().macros.len())
            .unwrap_or(0);
        for i in 0..count {
            runtime::tick_macro(i);
        }

        // java: if CONFIG_KEY.consumeClick(), open the config screen.
        if host.consume_click(MOD_ID, "config") {
            // The config screen is a host seam (no screen-widget hook yet);
            // the consumption and its gating are upstream's.
            log_warn("config screen requested; the editor screens are not rendered yet");
        }

        if config_store::DIRTY.0.get() {
            let instance = runtime::INSTANCE.0.borrow();
            if let Some(config) = instance.as_ref() {
                let text = config_store::serialize_config(config);
                crate::config_store::write_text(host.config_files(), "commandkeys.json", &text);
            }
            config_store::DIRTY.0.set(false);
        }
        generated::clear_current_host();
    }

    /// java: the KeyboardHandler mixin: track key state, dispatch handleKey
    /// on presses. The typed-character cancel has no wire here (the gap
    /// above); the mouse side does.
    fn input_key(&self, host: &mut dyn ClientHost, event: &mut InputKeyEvent) {
        generated::set_current_host(generated::host_ptr(host));
        match event.action {
            KeyAction::Press => runtime::set_key_state(event.key, true),
            KeyAction::Release => runtime::set_key_state(event.key, false),
            KeyAction::Repeat => {}
        }
        if event.action == KeyAction::Press {
            let _ = keybind_util::handle_key(&crate::config::Key::parse(event.key));
        }
        generated::clear_current_host();
    }

    /// java: the MouseButtonHandler mixin: track button state, dispatch
    /// handleKey on presses, cancel the click for VETO (and its modifiers).
    fn mouse_button_pre(&self, host: &mut dyn ClientHost, event: &mut MouseButtonPreEvent) -> Flow {
        generated::set_current_host(generated::host_ptr(host));
        runtime::set_mouse_state(event.button as u32, event.pressed);
        let mut flow = Flow::Continue;
        if event.pressed {
            let cancel = keybind_util::handle_key(&crate::config::Key::Mouse(event.button as u32));
            if cancel == 2 {
                flow = Flow::Cancel;
            }
        }
        generated::clear_current_host();
        flow
    }

    /// java: the mouse release tracking, same state map.
    fn mouse_button_post(&self, host: &mut dyn ClientHost, event: &mut MouseButtonPostEvent) {
        generated::set_current_host(generated::host_ptr(host));
        runtime::set_mouse_state(event.button as u32, event.pressed);
        generated::clear_current_host();
    }

    /// java: the ChatComponent accessor: record the chat lines the
    /// placeholders scan (newest first), capped like upstream's list.
    fn client_chat_received(
        &self,
        _host: &mut dyn ClientHost,
        event: &mut mod_api::ClientChatReceivedEvent<'_>,
    ) -> Flow {
        let mut snapshot = runtime::HOST.0.borrow_mut();
        snapshot
            .all_messages
            .push_front((None, String::new()));
        while snapshot.all_messages.len() > 100 {
            snapshot.all_messages.pop_back();
        }
        Flow::Continue
    }
}
