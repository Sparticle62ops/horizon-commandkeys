//! 1:1 Rust port of `CommandKeys.java`'s statics and event wiring, plus the
//! host glue the SDK hooks read. The class semantics (CONFIG_KEY tick, rate
//! limiter, macro tick, key dispatch, message send) are ported verbatim; the
//! platform seam is where upstream reads Minecraft state directly, the port
//! reads the same state through the SDK's host events and keeps a per-tick
//! snapshot.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::sync::LazyLock;

use crate::config::{Config, Key, Profile};

/// The runtime state lives in `static`s the hooks reach from one client
/// thread; the wrapper is the honest mark that access is single-threaded.
pub struct Ts<T>(pub T);
// The mod hooks dispatch on the client thread only; no shared access exists.
unsafe impl<T> Sync for Ts<T> {}

/// The client tick counter, java: TickCounter's global clock.
pub static TICK_COUNT: Ts<Cell<i64>> = Ts(Cell::new(0));

/// The one config instance, java: Config.instance.
pub static INSTANCE: Ts<RefCell<Option<Config>>> = Ts(RefCell::new(None));

/// java: CommandKeys.lastConnection.
pub static LAST_CONNECTION: Ts<RefCell<Option<String>>> = Ts(RefCell::new(None));

/// java: CommandKeys.hasResetConfig.
pub static HAS_RESET_CONFIG: Ts<RefCell<bool>> = Ts(RefCell::new(false));

/// java: CommandKeys.rateLimiter: a VecDeque of tick stamps.
pub static RATE_LIMITER: Ts<RefCell<VecDeque<i64>>> = Ts(RefCell::new(VecDeque::new()));

/// java: CommandKeys.ratelimitedKey.
pub static RATELIMITED_KEY: Ts<RefCell<Option<Key>>> = Ts(RefCell::new(None));

/// Input state the hooks track, the same observable state upstream polls from
/// SDL directly (the port cannot query SDL; it tracks the input events it
/// receives).
static KEY_STATE: Ts<LazyLock<RefCell<HashMap<String, bool>>>> =
    Ts(LazyLock::new(|| RefCell::new(HashMap::new())));
static MOUSE_STATE: Ts<LazyLock<RefCell<HashMap<u32, bool>>>> =
    Ts(LazyLock::new(|| RefCell::new(HashMap::new())));

/// The per-tick host snapshot, java's direct Minecraft reads.
#[derive(Default, Clone)]
pub struct HostState {
    pub in_game: bool,
    pub in_singleplayer: bool,
    pub accepting_messages: bool,
    pub screen_open: bool,
    pub player_name: Option<String>,
    pub player_block_pos: Option<(i32, i32, i32)>,
    pub look_angle: Option<(f64, f64, f64)>,
    pub look_block_pos: Option<(i32, i32, i32)>,
    pub clipboard: String,
    /// java: ChatComponent#getAllMessages (newest first, (translationKey, text)).
    pub all_messages: VecDeque<(Option<String>, String)>,
    /// java: ChatComponent#getRecentChat.
    pub recent_chat: VecDeque<String>,
    /// The command history the port tracks from its own sends; upstream reads
    /// the vanilla chat command history, which the SDK does not surface.
    pub command_history: VecDeque<String>,
}

pub static HOST: Ts<LazyLock<RefCell<HostState>>> =
    Ts(LazyLock::new(|| RefCell::new(HostState::default())));

/// Placeholder state, java: PlaceholderUtil's statics.
static PLACEHOLDER_FAULTS: Ts<RefCell<i32>> = Ts(RefCell::new(0));
static PLACEHOLDER_PM_SENDER: Ts<RefCell<Option<String>>> = Ts(RefCell::new(None));

/// The link map, java: Config.LINK_PROFILE_MAP (link -> profile), kept as
/// link -> profile position and rebuilt after every profile-list change.
pub static LINK_MAP: Ts<RefCell<Option<HashMap<String, usize>>>> = Ts(RefCell::new(None));

/// java: Config.profile(): the active profile.
pub fn profile() -> &'static Profile {
    // Safe: the instance is always present in-game (validated on load).
    unsafe {
        match &*INSTANCE.0.as_ptr() {
            Some(c) => c.active_profile(),
            None => unreachable!("Config accessed before load"),
        }
    }
}

/// A scoped mutable borrow of the active profile.
pub fn with_active_profile<T>(f: impl FnOnce(&mut Profile) -> T) -> T {
    let mut instance = INSTANCE.0.borrow_mut();
    let config = instance.as_mut().expect("Config accessed before load");
    f(config.active_profile_mut())
}

/// java: Config.getAndSave() without a filesystem (the host writes the file;
/// the port validates and marks the config dirty).
pub fn get_and_save() -> &'static Config {
    crate::config_store::mark_dirty();
    unsafe {
        match &*INSTANCE.0.as_ptr() {
            Some(c) => c,
            None => unreachable!("Config accessed before load"),
        }
    }
}

/// java: CommandKeys.profile() used by place_call-style helpers is not
/// relevant here; the rate limiter is.
///
/// java: TickCounter/rateLimiter: canTrigger(key, shouldAddEntry) -> true when
/// the key may trigger.
pub fn can_trigger(key: &Key, should_add_entry: bool) -> bool {
    let now = crate::tick_clock();
    let config = get_and_save();
    // java: the count cap applies everywhere unless ratelimitSp, which caps
    // only outside singleplayer.
    let count_cap = if config.ratelimit_sp && crate::runtime::in_singleplayer() {
        i32::MAX
    } else {
        config.ratelimit_count
    };
    let ticks = config.ratelimit_ticks.max(1) as i64;
    let mut limiter = RATE_LIMITER.0.borrow_mut();
    limiter.retain(|t| now - t < ticks);
    if limiter.len() as i32 >= count_cap {
        if should_add_entry && RATELIMITED_KEY.0.borrow().is_none() {
            crate::lang::chat_system_message(&crate::lang::localized(
                "message",
                "ratelimit.block",
                &[
                    config.ratelimit_count.to_string(),
                    config.ratelimit_ticks.to_string(),
                ],
            ));
            *RATELIMITED_KEY.0.borrow_mut() = Some(key.clone());
        }
        if config.ratelimit_strict {
            limiter.push_back(now);
        }
        return false;
    }
    limiter.push_back(now);
    true
}

/// java: Minecraft.isSingleplayer, through the host snapshot.
pub fn in_singleplayer() -> bool {
    HOST.0.borrow().in_singleplayer
}

/// java: CommandKeys.tickRateLimiter().
pub fn tick_rate_limiter() {
    let now = crate::tick_clock();
    RATE_LIMITER.0.borrow_mut().retain(|t| now - t < crate::tick_window());
}

/// java: send(message, addToHistory, showHudMsg).
pub fn send(message: &str, show_hud_message: bool, add_to_history: bool) {
    let (out, faults) = crate::placeholder::replace(message);
    if faults == 0 {
        do_send(&out, show_hud_message, add_to_history);
    } else {
        crate::lang::chat_system_message(&crate::lang::localized(
            "message",
            "placeholderFault",
            &[out],
        ));
    }
}

/// java: the private send(type, message, ...): length-limit guard, then the
/// chat send, history, or overlay message.
pub fn do_send(message: &str, show_hud_message: bool, add_to_history: bool) {
    let message = filter_text(message);
    let limit = crate::config::LENGTH_LIMIT_LENGTH_DEFAULT.max(get_and_save().length_limit_length);
    if message.len() as i32 > limit {
        crate::lang::chat_system_message(&crate::lang::localized(
            "message",
            "blocked.lengthlimit",
            &[
                message.len().to_string(),
                limit.to_string(),
            ],
        ));
        return;
    }
    // java: message.startsWith("/") -> sendCommand, else sendChat.
    if message.starts_with('/') {
        crate::host_bridge::send_command(&message[1..]);
    } else {
        crate::host_bridge::send_chat(&message);
    }
    if add_to_history {
        let mut host = HOST.0.borrow_mut();
        host.recent_chat.push_back(message.clone());
        if host.recent_chat.len() > 100 {
            host.recent_chat.pop_front();
        }
        host.command_history.push_back(message.clone());
        if host.command_history.len() > 50 {
            host.command_history.pop_front();
        }
    }
    if show_hud_message {
        crate::lang::set_overlay_message(&message);
    }
}

/// java: CommandKeys.type(message).
pub fn r#type(message: &str) {
    let (out, faults) = crate::placeholder::replace(message);
    if faults == 0 {
        crate::host_bridge::open_chat_typed(&out);
    } else {
        crate::lang::chat_system_message(&crate::lang::localized(
            "message",
            "placeholderFault",
            &[out],
        ));
    }
}

/// java: CommandKeys.edit(message).
pub fn edit(message: &str) {
    let (out, faults) = crate::placeholder::replace(message);
    if faults == 0 {
        crate::host_bridge::open_chat_edit(&out);
    } else {
        crate::lang::chat_system_message(&crate::lang::localized(
            "message",
            "placeholderFault",
            &[out],
        ));
    }
}

/// java: StringUtil.filterText: strips the chat-forbidden glyphs. The SDK
/// has no filter utility yet, so the text passes through (the seam is noted
/// in lib.rs).
pub fn filter_text(message: &str) -> String {
    message.to_string()
}

/// java: PlaceholderUtil's incoming PM sender scan: upstream matches the
/// `commands.message.display.incoming` translation key. Chat lines arrive
/// without their keys on the SDK, so the scan cannot match: None, faulting
/// the placeholder (seam noted in lib.rs).
pub fn incoming_pm_sender(_text: &str) -> Option<String> {
    None
}

/// java: Macro.trigger is dispatched here so the port can own the mutable
/// config while handling key events. `index` indexes the active profile's
/// macro list; `ratelimited` is the in-flight suppress flag from handleKey.
pub fn trigger_macro(index: usize, keybind: Option<crate::config::Keybind>, ratelimited: bool) -> bool {
    with_active_profile(|profile| {
        if let Some(m) = profile.macros.get_mut(index) {
            m.trigger(keybind, ratelimited)
        } else {
            false
        }
    })
}

/// java: Macro.tick, dispatched the same way.
pub fn tick_macro(index: usize) {
    with_active_profile(|profile| {
        if let Some(m) = profile.macros.get_mut(index) {
            m.tick();
        }
    })
}

// --- Host state accessors the placeholders and util read. ---

pub fn placeholder_state_reset() {
    *PLACEHOLDER_FAULTS.0.borrow_mut() = 0;
    *PLACEHOLDER_PM_SENDER.0.borrow_mut() = None;
}

pub fn placeholder_faults() -> i32 {
    *PLACEHOLDER_FAULTS.0.borrow()
}

pub fn placeholder_fault() {
    *PLACEHOLDER_FAULTS.0.borrow_mut() += 1;
}

pub fn set_pm_sender_name(name: String) {
    *PLACEHOLDER_PM_SENDER.0.borrow_mut() = Some(name);
}

pub fn pm_sender_name() -> Option<String> {
    PLACEHOLDER_PM_SENDER.0.borrow().clone()
}

pub fn is_key_down(key: &Key) -> bool {
    match key {
        Key::Unknown => false,
        Key::Keyboard(name) => KEY_STATE.0.borrow().get(name).copied().unwrap_or(false),
        Key::Mouse(v) => MOUSE_STATE.0.borrow().get(v).copied().unwrap_or(false),
    }
}

pub fn set_key_state(name: &str, down: bool) {
    KEY_STATE.0.borrow_mut().insert(name.to_string(), down);
}

pub fn set_mouse_state(button: u32, down: bool) {
    MOUSE_STATE.0.borrow_mut().insert(button, down);
}

pub fn get_player_name() -> String {
    HOST.0.borrow().player_name.clone().unwrap_or_default()
}

pub fn get_clipboard() -> String {
    HOST.0.borrow().clipboard.clone()
}

pub fn all_messages() -> VecDeque<(Option<String>, String)> {
    HOST.0.borrow().all_messages.clone()
}

pub fn get_last_message() -> String {
    HOST.0.borrow()
        .recent_chat
        .back()
        .cloned()
        .unwrap_or_else(|| {
            crate::log_warn("Recent chat placeholder failed: No message found");
            "?".to_string()
        })
}

pub fn get_last_command() -> String {
    HOST.0.borrow()
        .command_history
        .back()
        .cloned()
        .unwrap_or_else(|| {
            crate::log_error("Command history not available");
            "?".to_string()
        })
}

pub fn player_block_pos() -> Option<(i32, i32, i32)> {
    HOST.0.borrow().player_block_pos
}

pub fn look_block_pos() -> Option<(i32, i32, i32)> {
    HOST.0.borrow().look_block_pos
}

pub fn look_angle() -> Option<(f64, f64, f64)> {
    HOST.0.borrow().look_angle
}

pub fn screen_open() -> bool {
    HOST.0.borrow().screen_open
}

/// java: link_profile_index(link): Config.LINK_PROFILE_MAP.getOrDefault,
/// resolved through the rebuilt link map.
pub fn link_profile_index(link: &str) -> Option<usize> {
    LINK_MAP.0.borrow().as_ref().and_then(|m| m.get(link).copied())
}

/// java: rebuildLinkMap(): link -> profile position, rebuilt after every
/// profile-list change.
pub fn rebuild_link_map() {
    let mut map = HashMap::new();
    let mut instance = INSTANCE.0.borrow_mut();
    if let Some(config) = instance.as_mut() {
        for (i, profile) in config.profiles.iter().enumerate() {
            for link in &profile.links {
                map.entry(link.clone()).or_insert(i);
            }
        }
    }
    *LINK_MAP.0.borrow_mut() = Some(map);
}

/// java: getConflict's by-key mapping scan, fed by the mappings this port
/// registered (the SDK's mapping list is name-keyed).
pub fn get_mapping_conflict(key: &Key) -> Option<String> {
    crate::host_bridge::mapping_bound_to_key(key)
}

pub fn log_warn(msg: &str) {
    crate::log_warn(msg)
}

pub fn log_error(msg: &str) {
    crate::log_error(msg)
}

// A small thread-local RNG, java: Mth.randomBetween / ThreadLocalRandom.
pub fn random_int(len: i32) -> i32 {
    use std::cell::Cell;
    thread_local! {
        static STATE: Cell<u64> = const { Cell::new(0x2545F4914F6CDD1D) };
    }
    STATE.with(|s| {
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s.set(x);
        ((x >> 33) as i32).rem_euclid(len)
    })
}
