//! 1:1 Rust port of CommandKeys' config model:
//! `Config.java`, `Profile.java`, `Macro.java`, `Keybind.java`, `Message.java`.
//! Same field names in JSON, same defaults, same validation, same activation
//! reordering. Gson's lenient load (`JsonUtil.getOrDefault`, silent when the
//! version differs, the `.old` backup when anything was substituted) is
//! mirrored by `json.rs`'s helpers.

use std::collections::HashMap;

use crate::json;

/// java: Config.VERSION / Profile.VERSION / Macro.VERSION.
pub const CONFIG_VERSION: i32 = 7;
pub const PROFILE_VERSION: i32 = 4;
pub const MACRO_VERSION: i32 = 7;

/// java: Config.defaultIndexDefault.
pub const DEFAULT_INDEX_DEFAULT: i32 = 0;
/// java: Config.ratelimitCountDefault.
pub const RATELIMIT_COUNT_DEFAULT: i32 = 4;
/// java: Config.ratelimitTicksDefault.
pub const RATELIMIT_TICKS_DEFAULT: i32 = 20;
/// java: Config.ratelimitStrictDefault.
pub const RATELIMIT_STRICT_DEFAULT: bool = false;
/// java: Config.ratelimitSpDefault.
pub const RATELIMIT_SP_DEFAULT: bool = false;
/// java: Config.lengthLimitLengthDefault (SharedConstants.MAX_CHAT_LENGTH).
pub const LENGTH_LIMIT_LENGTH_DEFAULT: i32 = 256;
/// java: Config.macroMcKeyCountDefault.
pub const MACRO_MC_KEY_COUNT_DEFAULT: i32 = 0;

/// java: Macro.conflictStrategyDefault.
pub const CONFLICT_STRATEGY_DEFAULT: ConflictStrategy = ConflictStrategy::Assert;
/// java: Macro.sendModeDefault.
pub const SEND_MODE_DEFAULT: SendMode = SendMode::Send;
/// java: Macro.activationTypeDefault.
pub const ACTIVATION_TYPE_DEFAULT: ActivationType = ActivationType::Hold;
/// java: Macro.spaceTicksDefault.
pub const SPACE_TICKS_DEFAULT: i32 = 0;
/// java: Macro.maxRepeatsDefault.
pub const MAX_REPEATS_DEFAULT: i32 = 0;
/// java: Macro.addToHistoryDefault.
pub const ADD_TO_HISTORY_DEFAULT: bool = false;
/// java: Macro.showHudMessageDefault.
pub const SHOW_HUD_MESSAGE_DEFAULT: bool = false;
/// java: Macro.resumeRepeatingDefault.
pub const RESUME_REPEATING_DEFAULT: bool = false;
/// java: Macro.useRatelimitDefault.
pub const USE_RATELIMIT_DEFAULT: bool = true;

// ---------------------------------------------------------------------------
// Enums. Serialized by their Java enum names, exactly as Gson writes them.
// ---------------------------------------------------------------------------

/// java: Profile.Control. A tri-state control: ON forces the behaviour on,
/// OFF forces it off, DEFER lets each macro keep its own setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    On,
    Off,
    Defer,
}

impl Control {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "ON" => Some(Self::On),
            "OFF" => Some(Self::Off),
            "DEFER" => Some(Self::Defer),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::On => "ON",
            Self::Off => "OFF",
            Self::Defer => "DEFER",
        }
    }
}

/// java: Macro.ConflictStrategy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConflictStrategy {
    Submit,
    Assert,
    Veto,
    Avoid,
}

impl ConflictStrategy {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "SUBMIT" => Some(Self::Submit),
            "ASSERT" => Some(Self::Assert),
            "VETO" => Some(Self::Veto),
            "AVOID" => Some(Self::Avoid),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Submit => "SUBMIT",
            Self::Assert => "ASSERT",
            Self::Veto => "VETO",
            Self::Avoid => "AVOID",
        }
    }
}

/// java: Macro.SendMode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SendMode {
    Send,
    Type,
    Edit,
    Cycle,
    Random,
    Repeat,
}

impl SendMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "SEND" => Some(Self::Send),
            "TYPE" => Some(Self::Type),
            "EDIT" => Some(Self::Edit),
            "CYCLE" => Some(Self::Cycle),
            "RANDOM" => Some(Self::Random),
            "REPEAT" => Some(Self::Repeat),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Send => "SEND",
            Self::Type => "TYPE",
            Self::Edit => "EDIT",
            Self::Cycle => "CYCLE",
            Self::Random => "RANDOM",
            Self::Repeat => "REPEAT",
        }
    }
}

/// java: Macro.ActivationType.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivationType {
    Hold,
    Vanilla,
    Release,
}

impl ActivationType {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "HOLD" => Some(Self::Hold),
            "VANILLA" => Some(Self::Vanilla),
            "RELEASE" => Some(Self::Release),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Hold => "HOLD",
            Self::Vanilla => "VANILLA",
            Self::Release => "RELEASE",
        }
    }
}

/// java: getConflictStrategy(str), the legacy numeric-string mapping.
pub fn legacy_conflict_strategy(s: &str) -> ConflictStrategy {
    match s {
        "ZERO" => ConflictStrategy::Submit,
        "ONE" => ConflictStrategy::Assert,
        "TWO" => ConflictStrategy::Veto,
        "THREE" => ConflictStrategy::Avoid,
        _ => CONFLICT_STRATEGY_DEFAULT,
    }
}

/// java: getSendMode(str), the legacy numeric-string mapping.
pub fn legacy_send_mode(s: &str) -> SendMode {
    match s {
        "ZERO" => SendMode::Send,
        "ONE" => SendMode::Type,
        "TWO" => SendMode::Cycle,
        _ => SEND_MODE_DEFAULT,
    }
}

// ---------------------------------------------------------------------------
// Key. java: InputConstants.Key, carried by its name and parsed back the same
// way (`InputConstants.getKey(String)`; unknown names fall back to UNKNOWN).
// ---------------------------------------------------------------------------

/// The name of the unbound key, java: InputConstants.UNKNOWN.
pub const UNKNOWN_KEY_NAME: &str = "unknown";

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    /// The unbound key, java: InputConstants.UNKNOWN.
    Unknown,
    /// A keyboard key, by its vanilla name (`key.keyboard.a`), the value the
    /// input hook names (`InputKeyEvent.key`).
    Keyboard(String),
    /// A mouse button, by its vanilla name (`mouse.left`...`mouse.button5`).
    Mouse(u32),
}

impl Key {
    /// java: InputConstants.getKey(String) for the names this port stores,
    /// with the same UNKNOWN fallback.
    pub fn parse(name: &str) -> Self {
        if name.is_empty() || name == UNKNOWN_KEY_NAME {
            return Self::Unknown;
        }
        if let Some(rest) = name.strip_prefix("key.keyboard.") {
            if rest.is_empty() {
                return Self::Unknown;
            }
            return Self::Keyboard(name.to_string());
        }
        if let Some(rest) = name.strip_prefix("mouse.") {
            return match rest {
                "left" => Self::Mouse(0),
                "right" => Self::Mouse(1),
                "middle" => Self::Mouse(2),
                "button4" => Self::Mouse(3),
                "button5" => Self::Mouse(4),
                _ => Self::Unknown,
            };
        }
        Self::Unknown
    }

    pub fn name(&self) -> String {
        match self {
            Self::Unknown => UNKNOWN_KEY_NAME.to_string(),
            Self::Keyboard(n) => n.clone(),
            Self::Mouse(v) => match v {
                0 => "mouse.left".to_string(),
                1 => "mouse.right".to_string(),
                2 => "mouse.middle".to_string(),
                3 => "mouse.button4".to_string(),
                _ => "mouse.button5".to_string(),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Message. java: Message.java.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub string: String,
    pub delay_ticks: i32,
}

impl Default for Message {
    fn default() -> Self {
        Self { string: String::new(), delay_ticks: 0 }
    }
}

impl Message {
    pub fn new(string: impl Into<String>, delay_ticks: i32) -> Self {
        Self { string: string.into(), delay_ticks }
    }

    /// java: Message.validate().
    pub fn validate(&mut self) {
        if self.delay_ticks < 0 {
            self.delay_ticks = 0;
        }
    }

    /// java: the Gson Deserializer, field for field.
    pub fn deserialize(obj: &json::Obj) -> Option<Self> {
        let version = obj.i32("version").unwrap_or(0);
        let silent = version != 1;
        let mut m = Self {
            string: obj.string_or_default("string", "", silent),
            delay_ticks: obj.i32_or_default("delayTicks", 0, silent),
        };
        // java: string == null -> ""
        if obj.has_null("string") {
            m.string = String::new();
        }
        m.validate();
        Some(m)
    }

    pub fn serialize(&self) -> json::Val {
        json::obj()
            .int("version", 1)
            .string("string", &self.string)
            .int("delayTicks", self.delay_ticks)
            .build()
    }
}

// ---------------------------------------------------------------------------
// Keybind. java: Keybind.java.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Keybind {
    /// java: keyName.
    pub key_name: String,
    /// java: limitKeyName.
    pub limit_key_name: String,
    /// java: key, the parsed Key.
    pub key: Key,
    /// java: limitKey, the parsed Key.
    pub limit_key: Key,
}

impl Default for Keybind {
    fn default() -> Self {
        Self {
            key_name: UNKNOWN_KEY_NAME.to_string(),
            limit_key_name: UNKNOWN_KEY_NAME.to_string(),
            key: Key::Unknown,
            limit_key: Key::Unknown,
        }
    }
}

impl Keybind {
    /// java: Keybind(Key, Key), not validated.
    pub fn from_keys(key: Key, limit_key: Key) -> Self {
        Self {
            key_name: key.name(),
            limit_key_name: limit_key.name(),
            key,
            limit_key,
        }
    }

    /// java: Keybind.isKeyDown(), polled through the input state the hooks
    /// track (upstream polls SDL directly; same observable state).
    pub fn is_key_down(&self) -> bool {
        crate::runtime::is_key_down(&self.key)
    }

    /// java: Keybind.isLimitKeyDown().
    pub fn is_limit_key_down(&self) -> bool {
        crate::runtime::is_key_down(&self.limit_key)
    }

    /// java: Keybind.validate().
    pub fn validate(&mut self) {
        // If main key is unbound, limit key cannot be bound
        if self.key == Key::Unknown && self.limit_key != Key::Unknown {
            self.limit_key = Key::Unknown;
        }
        // Update names just because
        self.key_name = self.key.name();
        self.limit_key_name = self.limit_key.name();
    }

    /// java: the Gson Deserializer, field for field.
    pub fn deserialize(obj: &json::Obj) -> Option<Self> {
        let version = obj.i32("version").unwrap_or(0);
        let silent = version != 0;
        let key = Key::parse(&obj.string_or_default("keyName", UNKNOWN_KEY_NAME, silent));
        let limit_key =
            Key::parse(&obj.string_or_default("limitKeyName", UNKNOWN_KEY_NAME, silent));
        let mut k = Self::from_keys(key, limit_key);
        k.validate();
        Some(k)
    }

    pub fn serialize(&self) -> json::Val {
        json::obj()
            .int("version", 0)
            .string("keyName", &self.key_name)
            .string("limitKeyName", &self.limit_key_name)
            .build()
    }
}

// ---------------------------------------------------------------------------
// ScheduledMessage. java: Macro.ScheduledMessage.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct ScheduledMessage {
    pub delay: i32,
    pub message: String,
    pub show_hud_message: bool,
    pub add_to_history: bool,
}

impl ScheduledMessage {
    /// java: ScheduledMessage.tick(). `false` when this message has finished
    /// ticking, `true` to keep it.
    pub fn tick(&mut self) -> bool {
        self.delay -= 1;
        if self.delay <= 0 {
            crate::runtime::send(
                &self.message,
                self.show_hud_message,
                self.add_to_history,
            );
            true
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Macro. java: Macro.java, state and tick logic together.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Macro {
    // Serialized fields, java field names in JSON.
    pub add_to_history: bool,
    pub show_hud_message: bool,
    pub resume_repeating: bool,
    pub use_ratelimit: bool,
    pub conflict_strategy: ConflictStrategy,
    pub send_mode: SendMode,
    pub activation_type: ActivationType,
    pub space_ticks: i32,
    pub max_repeats: i32,
    /// java: cycleIndex, serialized since version 5 (kept here as state; the
    /// writer emits it when the cycle index is not the default).
    pub cycle_index: i32,
    pub name: String,
    pub mc_activator_key: i32,
    pub keybind: Keybind,
    pub alt_keybind: Keybind,
    pub messages: Vec<Message>,

    // Transient state, java fields of the same names.
    /// java: addToHistoryStatus, the effective value after the profile's control.
    pub add_to_history_status: bool,
    pub show_hud_message_status: bool,
    pub resume_repeating_status: bool,
    pub use_ratelimit_status: bool,
    pub active: bool,
    pub waiting: bool,
    /// java: activationKeybind, `None` when activation came with no keybind.
    pub activation_keybind: Option<Keybind>,
    pub active_ticks: i32,
    pub repetitions: i32,
    pub scheduled_messages: Vec<ScheduledMessage>,
}

impl Default for Macro {
    fn default() -> Self {
        Self::new(
            ADD_TO_HISTORY_DEFAULT,
            SHOW_HUD_MESSAGE_DEFAULT,
            RESUME_REPEATING_DEFAULT,
            USE_RATELIMIT_DEFAULT,
            CONFLICT_STRATEGY_DEFAULT,
            SEND_MODE_DEFAULT,
            ACTIVATION_TYPE_DEFAULT,
            SPACE_TICKS_DEFAULT,
            MAX_REPEATS_DEFAULT,
            0,
            String::new(),
            0,
            Keybind::default(),
            Keybind::default(),
            Vec::new(),
        )
    }
}

impl Macro {
    /// java: the full constructor, field order as upstream.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        add_to_history: bool,
        show_hud_message: bool,
        resume_repeating: bool,
        use_ratelimit: bool,
        conflict_strategy: ConflictStrategy,
        send_mode: SendMode,
        activation_type: ActivationType,
        space_ticks: i32,
        max_repeats: i32,
        cycle_index: i32,
        name: String,
        mc_activator_key: i32,
        keybind: Keybind,
        alt_keybind: Keybind,
        messages: Vec<Message>,
    ) -> Self {
        let mut m = Self {
            add_to_history,
            show_hud_message,
            resume_repeating,
            use_ratelimit,
            conflict_strategy,
            send_mode,
            activation_type,
            space_ticks,
            max_repeats,
            cycle_index,
            name,
            mc_activator_key,
            keybind,
            alt_keybind,
            messages,
            add_to_history_status: false,
            show_hud_message_status: false,
            resume_repeating_status: false,
            use_ratelimit_status: false,
            active: false,
            waiting: false,
            activation_keybind: None,
            active_ticks: 0,
            repetitions: 0,
            scheduled_messages: Vec::new(),
        };
        m.validate();
        m
    }

    /// java: ownsKeybind(Keybind): identity of this macro's primary or alt keybind.
    pub fn owns_keybind(&self, keybind: &Keybind) -> bool {
        &self.keybind == keybind || &self.alt_keybind == keybind
    }

    /// java: usesAltKeybind().
    pub fn uses_alt_keybind(&self) -> bool {
        self.send_mode == SendMode::Cycle
    }

    // Message management, java: addMessage / setMessage / removeMessage / moveMessage.

    pub fn set_message(&mut self, index: usize, string: impl Into<String>) {
        if let Some(m) = self.messages.get_mut(index) {
            m.string = string.into();
        }
    }

    /// java: moveMessage(sourceIndex, destIndex).
    pub fn move_message(&mut self, source_index: usize, dest_index: usize) -> bool {
        if source_index != dest_index && source_index < self.messages.len()
            && dest_index < self.messages.len()
        {
            let msg = self.messages.remove(source_index);
            self.messages.insert(dest_index, msg);
            return true;
        }
        false
    }

    // Activation. java: tick().

    pub fn tick(&mut self) {
        // Tick scheduled messages, removing those that have finished
        // (java: removeIf(ScheduledMessage::tick), which removes on true).
        self.scheduled_messages.retain_mut(|sm| !sm.tick());

        if self.active {
            // Deactivate if key has been released
            if self.activation_type == ActivationType::Hold && !self.keybind.is_key_down() {
                self.deactivate();
            } else if self.activation_type == ActivationType::Release && self.waiting {
                if !self.keybind.is_key_down() {
                    self.waiting = false;
                    let kb = self.activation_keybind.clone();
                    self.do_action(kb);
                }
            }
            // Tick ongoing actions
            else {
                if self.send_mode == SendMode::Repeat
                    && (self.space_ticks == 0
                        || (self.active_ticks > 0 && self.active_ticks % self.space_ticks == 0))
                {
                    self.repetitions += 1;
                    if self.max_repeats == 0 || self.repetitions <= self.max_repeats {
                        self.schedule_all(false);
                    }
                }
                self.active_ticks += 1;
            }
        }
    }

    /// java: trigger(keybind, ratelimited). `true` if ratelimited.
    pub fn trigger(&mut self, keybind: Option<Keybind>, ratelimited: bool) -> bool {
        if self.active {
            if self.activation_type != ActivationType::Release || !self.waiting {
                self.single_action_complete();
            }
        } else {
            // Only check ratelimiter if we've actually got something to do
            if let Some(kb) = &keybind {
                if self.use_ratelimit_status
                    && !crate::runtime::can_trigger(&kb.key, !ratelimited)
                {
                    return true;
                }
            }
            self.activate(keybind);
        }
        false
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }

    pub fn deactivate_and_cancel(&mut self) {
        self.deactivate();
        self.clear_scheduled();
    }

    /// java: singleActionComplete().
    fn single_action_complete(&mut self) {
        if self.activation_type != ActivationType::Hold {
            self.deactivate();
        }
    }

    fn activate(&mut self, keybind: Option<Keybind>) {
        self.active = true;
        self.active_ticks = -1; // trigger is processed prior to tick
        self.repetitions = 1;

        if self.activation_type == ActivationType::Release {
            self.waiting = true;
            self.activation_keybind = keybind;
        } else {
            self.do_action(keybind);
        }
    }

    /// java: doAction(keybind).
    fn do_action(&mut self, keybind: Option<Keybind>) {
        match self.send_mode {
            SendMode::Send => {
                let spaced = self.space_ticks != 0;
                self.schedule_all(spaced);
                self.single_action_complete();
            }
            SendMode::Repeat => self.schedule_all(false),
            SendMode::Edit => {
                // Edit the first message
                if let Some(first) = self.messages.first() {
                    crate::runtime::edit(&first.string);
                }
                self.single_action_complete();
            }
            SendMode::Type => {
                // Type the first message
                if let Some(first) = self.messages.first() {
                    crate::runtime::r#type(&first.string);
                }
                self.single_action_complete();
            }
            SendMode::Cycle => {
                let alt = matches!(&keybind, Some(kb) if *kb == self.alt_keybind);
                if alt {
                    // Alt keybind cycles backwards
                    self.cycle_index -= 1;
                    if self.cycle_index < 0 {
                        self.cycle_index = self.messages.len() as i32 - 1;
                    }
                } else {
                    // Main keybind cycles forwards
                    self.cycle_index += 1;
                    if self.cycle_index >= self.messages.len() as i32 {
                        self.cycle_index = 0;
                    }
                }
                // Split to allow multiple messages per press
                let idx = self.cycle_index.clamp(0, self.messages.len() as i32 - 1) as usize;
                let combined = self.messages[idx].string.clone();
                let show = self.show_hud_message_status;
                let history = self.add_to_history_status;
                for str in combined.split(",,") {
                    // Blank messages are treated as spacers
                    if !str.trim().is_empty() {
                        self.schedule(
                            self.messages[idx].delay_ticks,
                            str.to_string(),
                            history,
                            show,
                        );
                    }
                }
                self.single_action_complete();
            }
            SendMode::Random => {
                // Pick a random message and send it
                if !self.messages.is_empty() {
                    let idx = crate::runtime::random_int(self.messages.len() as i32) as usize;
                    let msg = self.messages[idx].clone();
                    if !msg.string.trim().is_empty() {
                        self.schedule(
                            msg.delay_ticks,
                            msg.string,
                            self.add_to_history_status,
                            self.show_hud_message_status,
                        );
                    }
                }
                self.single_action_complete();
            }
        }
    }

    /// java: schedule(delay, message, addToHistory, showHudMessage).
    pub(crate) fn schedule(&mut self, delay: i32, message: String, add_to_history: bool, show_hud_message: bool) {
        if delay > 0 {
            self.scheduled_messages.push(ScheduledMessage {
                delay,
                message,
                show_hud_message,
                add_to_history,
            });
        } else {
            crate::runtime::send(&message, show_hud_message, add_to_history);
        }
    }

    /// java: scheduleAll(standardDelay).
    pub(crate) fn schedule_all(&mut self, standard_delay: bool) {
        let mut total_delay = if standard_delay { -self.space_ticks } else { 0 };
        let history = self.add_to_history_status;
        let show = self.show_hud_message_status;
        for msg in self.messages.clone() {
            total_delay += if standard_delay { self.space_ticks } else { msg.delay_ticks };
            if !msg.string.trim().is_empty() {
                self.schedule(total_delay, msg.string, history, show);
            }
        }
    }

    /// java: clearScheduled().
    pub fn clear_scheduled(&mut self) {
        self.scheduled_messages.clear();
    }

    // Validation. java: validate() and cleanupMessages().

    pub fn validate(&mut self) {
        if self.space_ticks < 0 {
            self.space_ticks = 0;
        }
        if self.max_repeats < 0 {
            self.max_repeats = 0;
        }
        self.keybind.validate();
        self.alt_keybind.validate();
        for msg in &mut self.messages {
            msg.validate();
        }
        self.cleanup_messages();
    }

    /// java: cleanupMessages().
    pub fn cleanup_messages(&mut self) {
        self.messages.retain_mut(|msg| {
            // Never allow leading whitespace
            msg.string = msg.string.trim_start().to_string();
            // Only allow trailing whitespace for TYPE mode
            if self.send_mode != SendMode::Type && self.send_mode != SendMode::Edit {
                msg.string = msg.string.trim_end().to_string();
            }
            // Only allow blank messages for CYCLE mode (as spacers) and TYPE
            // mode (to open chat)
            !(msg.string.trim().is_empty()
                && self.send_mode != SendMode::Cycle
                && self.send_mode != SendMode::Type)
        });
    }

    /// java: the Gson Deserializer, field for field, legacy shapes included.
    pub fn deserialize(obj: &json::Obj) -> Option<Self> {
        let version = obj.i32("version").unwrap_or(0);
        let silent = version != MACRO_VERSION;

        let add_to_history = obj.bool_or_default("addToHistory", ADD_TO_HISTORY_DEFAULT, silent);
        let show_hud_message =
            obj.bool_or_default("showHudMessage", SHOW_HUD_MESSAGE_DEFAULT, silent);
        let resume_repeating =
            obj.bool_or_default("resumeRepeating", RESUME_REPEATING_DEFAULT, silent);
        let use_ratelimit = obj.bool_or_default("useRatelimit", USE_RATELIMIT_DEFAULT, silent);

        let conflict_strategy = if version >= 3 {
            obj.enum_or_default("conflictStrategy", ConflictStrategy::parse, CONFLICT_STRATEGY_DEFAULT, silent)
        } else {
            legacy_conflict_strategy(&obj.string_or_default("conflictStrategy", "", true))
        };

        let send_mode = if version >= 3 {
            obj.enum_or_default("sendMode", SendMode::parse, SEND_MODE_DEFAULT, silent)
        } else {
            legacy_send_mode(&obj.string_or_default("sendMode", "", true))
        };

        let activation_type = obj.enum_or_default(
            "activationType",
            ActivationType::parse,
            ACTIVATION_TYPE_DEFAULT,
            silent,
        );

        let space_ticks = obj.i32_or_default("spaceTicks", SPACE_TICKS_DEFAULT, silent);
        let max_repeats = obj.i32_or_default("maxRepeats", MAX_REPEATS_DEFAULT, silent);
        let name = obj.string_or_default("name", "", silent);
        let mc_activator_key =
            obj.i32_or_default("mcActivatorKey", 0, silent);

        let keybind = if version >= 4 {
            obj.nested_or_default("keybind", Keybind::deserialize, Keybind::default, silent)
        } else {
            // Legacy flat keyName/limitKeyName fields.
            Keybind::from_keys(
                Key::parse(&obj.string_or_default("keyName", UNKNOWN_KEY_NAME, true)),
                Key::parse(&obj.string_or_default("limitKeyName", UNKNOWN_KEY_NAME, true)),
            )
        };

        let alt_keybind = obj.nested_or_default("altKeybind", Keybind::deserialize, Keybind::default, silent);

        let messages = obj
            .array_of("messages", silent)
            .map(|vals| vals.iter().filter_map(|v| json::Obj::obj_of(v).and_then(|o| Message::deserialize(&o))).collect::<Vec<_>>())
            .unwrap_or_default();

        let mut m = Self::new(
            add_to_history,
            show_hud_message,
            resume_repeating,
            use_ratelimit,
            conflict_strategy,
            send_mode,
            activation_type,
            space_ticks,
            max_repeats,
            0,
            name,
            mc_activator_key,
            keybind,
            alt_keybind,
            messages,
        );
        m.validate();
        Some(m)
    }

    pub fn serialize(&self) -> json::Val {
        json::obj()
            .int("version", MACRO_VERSION)
            .boolean("addToHistory", self.add_to_history)
            .boolean("showHudMessage", self.show_hud_message)
            .boolean("resumeRepeating", self.resume_repeating)
            .boolean("useRatelimit", self.use_ratelimit)
            .string("conflictStrategy", self.conflict_strategy.name())
            .string("sendMode", self.send_mode.name())
            .string("activationType", self.activation_type.name())
            .int("spaceTicks", self.space_ticks)
            .int("maxRepeats", self.max_repeats)
            .string("name", &self.name)
            .int("mcActivatorKey", self.mc_activator_key)
            .nested("keybind", self.keybind.serialize())
            .nested("altKeybind", self.alt_keybind.serialize())
            .array_val(
                "messages",
                self.messages.iter().map(Message::serialize).collect(),
            )
            .build()
    }
}

// ---------------------------------------------------------------------------
// Profile. java: Profile.java.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Profile {
    pub name: String,
    pub links: Vec<String>,
    pub add_to_history: Control,
    pub show_hud_message: Control,
    pub resume_repeating: Control,
    pub use_ratelimit: Control,
    pub macros: Vec<Macro>,

    /// java: keybindMap: key -> keybinds using it (insertion order kept).
    pub keybind_map: HashMap<Key, Vec<Keybind>>,
    /// java: macroMap: keybind -> macros using it (insertion order kept).
    pub macro_map: HashMap<Keybind, Vec<usize>>,
}

pub const PROFILE_NAME_DEFAULT: &str = "";
/// java: linksDefault.
pub const LINKS_DEFAULT: () = ();
/// java: Control defaults.
pub const ADD_TO_HISTORY_CONTROL_DEFAULT: Control = Control::Off;
pub const SHOW_HUD_MESSAGE_CONTROL_DEFAULT: Control = Control::Off;
pub const RESUME_REPEATING_CONTROL_DEFAULT: Control = Control::Off;
pub const USE_RATELIMIT_CONTROL_DEFAULT: Control = Control::On;

impl Default for Profile {
    fn default() -> Self {
        Self::new_named(String::new())
    }
}

impl Profile {
    pub fn new_named(name: String) -> Self {
        Self {
            name,
            links: Vec::new(),
            add_to_history: ADD_TO_HISTORY_CONTROL_DEFAULT,
            show_hud_message: SHOW_HUD_MESSAGE_CONTROL_DEFAULT,
            resume_repeating: RESUME_REPEATING_CONTROL_DEFAULT,
            use_ratelimit: USE_RATELIMIT_CONTROL_DEFAULT,
            macros: Vec::new(),
            keybind_map: HashMap::new(),
            macro_map: HashMap::new(),
        }
    }

    /// java: getDisplayName().
    pub fn display_name(&self) -> String {
        let mut name = self.name.clone();
        if name.trim().is_empty() {
            name = self.links.first().cloned().unwrap_or_default();
        }
        if name.trim().is_empty() {
            name = "[Unnamed]".to_string();
        }
        name
    }

    // Link management, java: forceAddLink / removeLink.

    /// java: forceAddLink(link): adds `link` to this profile, removing it from
    /// whichever profile held it (the map keeps one profile per link).
    pub fn force_add_link(&mut self, link: String) {
        if !self.links.contains(&link) {
            self.links.push(link);
        }
    }

    pub fn remove_link(&mut self, link: &str) {
        self.links.retain(|l| l != link);
    }

    // Control getters/setters with the macro-override semantics.

    pub fn set_add_to_history_all(&mut self, control: Control) {
        self.add_to_history = control;
        let own: Vec<bool> = self.macros.iter().map(|m| m.add_to_history).collect();
        for (i, value) in own.iter().enumerate() {
            self.set_add_to_history_macro(i, *value);
        }
    }

    /// java: setAddToHistory(Macro, boolean): the macro keeps its own value
    /// and the status follows the profile control.
    pub fn set_add_to_history_macro(&mut self, index: usize, value: bool) {
        let control = self.add_to_history;
        if let Some(m) = self.macros.get_mut(index) {
            m.add_to_history = value;
            m.add_to_history_status = match control {
                Control::On => true,
                Control::Off => false,
                Control::Defer => m.add_to_history,
            };
        }
    }

    pub fn set_show_hud_message_all(&mut self, control: Control) {
        self.show_hud_message = control;
        let own: Vec<bool> = self.macros.iter().map(|m| m.show_hud_message).collect();
        for (i, value) in own.iter().enumerate() {
            self.set_show_hud_message_macro(i, *value);
        }
    }

    pub fn set_show_hud_message_macro(&mut self, index: usize, value: bool) {
        let control = self.show_hud_message;
        if let Some(m) = self.macros.get_mut(index) {
            m.show_hud_message = value;
            m.show_hud_message_status = match control {
                Control::On => true,
                Control::Off => false,
                Control::Defer => m.show_hud_message,
            };
        }
    }

    pub fn set_resume_repeating_all(&mut self, control: Control) {
        self.resume_repeating = control;
        let own: Vec<bool> = self.macros.iter().map(|m| m.resume_repeating).collect();
        for (i, value) in own.iter().enumerate() {
            self.set_resume_repeating_macro(i, *value);
        }
    }

    pub fn set_resume_repeating_macro(&mut self, index: usize, value: bool) {
        let control = self.resume_repeating;
        if let Some(m) = self.macros.get_mut(index) {
            m.resume_repeating = value;
            m.resume_repeating_status = match control {
                Control::On => true,
                Control::Off => false,
                Control::Defer => m.resume_repeating,
            };
        }
    }

    pub fn set_use_ratelimit_all(&mut self, control: Control) {
        self.use_ratelimit = control;
        let own: Vec<bool> = self.macros.iter().map(|m| m.use_ratelimit).collect();
        for (i, value) in own.iter().enumerate() {
            self.set_use_ratelimit_macro(i, *value);
        }
    }

    pub fn set_use_ratelimit_macro(&mut self, index: usize, value: bool) {
        let control = self.use_ratelimit;
        if let Some(m) = self.macros.get_mut(index) {
            m.use_ratelimit = value;
            m.use_ratelimit_status = match control {
                Control::On => true,
                Control::Off => false,
                Control::Defer => m.use_ratelimit,
            };
        }
    }

    // Macro management, java: addMacro / removeMacro / moveMacro.

    pub fn add_macro(&mut self, macro_: Macro) {
        self.macros.push(macro_);
    }

    /// java: moveMacro(sourceIndex, destIndex).
    pub fn move_macro(&mut self, source_index: usize, dest_index: usize) -> bool {
        if source_index != dest_index
            && source_index < self.macros.len()
            && dest_index < self.macros.len()
        {
            let m = self.macros.remove(source_index);
            self.macros.insert(dest_index, m);
            self.rebuild_maps();
            return true;
        }
        false
    }

    /// java: addToMaps(Macro): registers the macro's keybinds.
    pub fn add_to_maps(&mut self, macro_index: usize) {
        let Some(m) = self.macros.get(macro_index) else { return };
        let kb = m.keybind.clone();
        let alt = m.alt_keybind.clone();
        self.keybind_map.entry(kb.key.clone()).or_default().push(kb.clone());
        self.macro_map.entry(kb.clone()).or_default().push(macro_index);
        if alt != kb {
            self.keybind_map.entry(alt.key.clone()).or_default().push(alt.clone());
            self.macro_map.entry(alt.clone()).or_default().push(macro_index);
        }
    }

    /// java: rebuildMaps(): clears both maps and re-adds every macro.
    pub fn rebuild_maps(&mut self) {
        self.keybind_map.clear();
        self.macro_map.clear();
        for i in 0..self.macros.len() {
            self.add_to_maps(i);
        }
    }

    /// java: setKey(Macro, Keybind, Key).
    pub fn set_key(&mut self, macro_index: usize, primary: bool, key: Key) {
        let same = {
            let m = &self.macros[macro_index];
            if primary {
                m.keybind.key == key
            } else {
                m.alt_keybind.key == key
            }
        };
        if same {
            return;
        }
        self.macros[macro_index].deactivate_and_cancel();
        if primary {
            self.macros[macro_index].keybind.key = key.clone();
        } else {
            self.macros[macro_index].alt_keybind.key = key.clone();
        }
        self.rebuild_maps();
    }

    /// java: setLimitKey(Macro, Keybind, Key).
    pub fn set_limit_key(&mut self, macro_index: usize, primary: bool, limit_key: Key) {
        let same = {
            let m = &self.macros[macro_index];
            if primary {
                m.keybind.limit_key == limit_key
            } else {
                m.alt_keybind.limit_key == limit_key
            }
        };
        if same {
            return;
        }
        self.macros[macro_index].deactivate_and_cancel();
        if primary {
            self.macros[macro_index].keybind.limit_key = limit_key;
        } else {
            self.macros[macro_index].alt_keybind.limit_key = limit_key;
        }
        self.rebuild_maps();
    }

    // Validation. java: validate().

    pub fn validate(&mut self) {
        for m in &mut self.macros {
            m.validate();
        }
        self.macros.retain(|m| !m.messages.is_empty());

        // Update transients in macros
        self.set_add_to_history_all(self.add_to_history);
        self.set_show_hud_message_all(self.show_hud_message);
        self.set_resume_repeating_all(self.resume_repeating);
        self.set_use_ratelimit_all(self.use_ratelimit);

        // Possibly not required?
        self.rebuild_maps();
    }

    /// java: the Gson Deserializer, field for field, legacy shapes included.
    pub fn deserialize(obj: &json::Obj) -> Option<Self> {
        let version = obj.i32("version").unwrap_or(0);
        let silent = version != PROFILE_VERSION;

        let name = obj.string_or_default("name", PROFILE_NAME_DEFAULT, silent);

        let links = if version >= 3 {
            obj.string_vec_or_default("links", silent)
        } else {
            obj.string_vec_or_default("addresses", true)
        };

        let add_to_history = obj.enum_or_default(
            "addToHistory",
            Control::parse,
            ADD_TO_HISTORY_CONTROL_DEFAULT,
            silent,
        );
        let show_hud_message = obj.enum_or_default(
            "showHudMessage",
            Control::parse,
            SHOW_HUD_MESSAGE_CONTROL_DEFAULT,
            silent,
        );
        let resume_repeating = obj.enum_or_default(
            "resumeRepeating",
            Control::parse,
            RESUME_REPEATING_CONTROL_DEFAULT,
            silent,
        );
        let use_ratelimit = obj.enum_or_default(
            "useRatelimit",
            Control::parse,
            USE_RATELIMIT_CONTROL_DEFAULT,
            silent,
        );

        let macros = if version >= 2 {
            obj.array_of("macros", silent)
                .map(|vals| vals.iter().filter_map(|v| json::Obj::obj_of(v).and_then(|o| Macro::deserialize(&o))).collect())
                .unwrap_or_default()
        } else {
            obj.array_of("commandKeys", true)
                .map(|vals| vals.iter().filter_map(|v| json::Obj::obj_of(v).and_then(|o| Macro::deserialize(&o))).collect())
                .unwrap_or_default()
        };

        let mut p = Self {
            name,
            links,
            add_to_history,
            show_hud_message,
            resume_repeating,
            use_ratelimit,
            macros,
            keybind_map: HashMap::new(),
            macro_map: HashMap::new(),
        };
        p.validate();
        Some(p)
    }

    pub fn serialize(&self) -> json::Val {
        json::obj()
            .int("version", PROFILE_VERSION)
            .string("name", &self.name)
            .string_vec("links", &self.links)
            .string("addToHistory", self.add_to_history.name())
            .string("showHudMessage", self.show_hud_message.name())
            .string("resumeRepeating", self.resume_repeating.name())
            .string("useRatelimit", self.use_ratelimit.name())
            .array_val(
                "macros",
                self.macros.iter().map(Macro::serialize).collect(),
            )
            .build()
    }
}

// ---------------------------------------------------------------------------
// Config. java: Config.java.
// ---------------------------------------------------------------------------

/// java: Config.profilesDefault: the default profile list.
pub fn profiles_default() -> Vec<Profile> {
    vec![Profile::new_named("Default".to_string())]
}

#[derive(Clone, Debug)]
pub struct Config {
    pub profiles: Vec<Profile>,
    pub sp_default: i32,
    pub mp_default: i32,
    pub default_conflict_strategy: ConflictStrategy,
    pub default_send_mode: SendMode,
    pub default_activation_type: ActivationType,
    pub ratelimit_count: i32,
    pub ratelimit_ticks: i32,
    pub ratelimit_strict: bool,
    pub ratelimit_sp: bool,
    pub length_limit_length: i32,
    pub macro_mc_key_count: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            profiles: profiles_default(),
            sp_default: DEFAULT_INDEX_DEFAULT,
            mp_default: DEFAULT_INDEX_DEFAULT,
            default_conflict_strategy: CONFLICT_STRATEGY_DEFAULT,
            default_send_mode: SEND_MODE_DEFAULT,
            default_activation_type: ACTIVATION_TYPE_DEFAULT,
            ratelimit_count: RATELIMIT_COUNT_DEFAULT,
            ratelimit_ticks: RATELIMIT_TICKS_DEFAULT,
            ratelimit_strict: RATELIMIT_STRICT_DEFAULT,
            ratelimit_sp: RATELIMIT_SP_DEFAULT,
            length_limit_length: LENGTH_LIMIT_LENGTH_DEFAULT,
            macro_mc_key_count: MACRO_MC_KEY_COUNT_DEFAULT,
        }
    }
}

impl Config {
    /// java: activeProfile().
    pub fn active_profile(&self) -> &Profile {
        &self.profiles[0]
    }

    pub fn active_profile_mut(&mut self) -> &mut Profile {
        &mut self.profiles[0]
    }

    /// java: activateProfile(index): reorders the list so the profile is
    /// first, adjusting the default pointers exactly as upstream.
    pub fn activate_profile(&mut self, index: i32) {
        // Deactivate active profile if set to do so
        for m in &mut self.profiles[0].macros {
            if !m.resume_repeating_status {
                m.deactivate();
            }
        }
        if index != 0 {
            let i = index as usize;
            let profile = self.profiles.remove(i);
            self.profiles.insert(0, profile);
            // Update default pointers
            if index == self.sp_default {
                self.sp_default = 0;
            } else if index > self.sp_default {
                self.sp_default += 1;
            }
            if index == self.mp_default {
                self.mp_default = 0;
            } else if index > self.mp_default {
                self.mp_default += 1;
            }
        }
    }

    /// java: activateSpProfile(levelId): the linked profile if one exists,
    /// else the singleplayer default. Links resolve through the link map the
    /// runtime rebuilds after every profile-list change.
    pub fn activate_sp_profile(&mut self, level_id: &str) {
        if let Some(index) = crate::runtime::link_profile_index(level_id) {
            self.activate_profile(index as i32);
        } else {
            self.activate_profile(self.sp_default);
        }
    }

    /// java: activateMpProfile(address).
    pub fn activate_mp_profile(&mut self, address: &str) {
        if let Some(index) = crate::runtime::link_profile_index(address) {
            self.activate_profile(index as i32);
        } else {
            self.activate_profile(self.mp_default);
        }
    }

    /// java: addCopyProfile(profile): a custom copy, no links, at the end.
    pub fn add_copy_profile(&mut self, source_index: usize) {
        let mut copy = self.profiles[source_index].clone();
        copy.name = format!("{} (Copy)", copy.display_name());
        copy.links = Vec::new();
        copy.rebuild_maps();
        self.profiles.push(copy);
    }

    /// java: addNewProfile(): a new default-named profile at the end.
    pub fn add_new_profile(&mut self) {
        let mut p = Profile::new_named(String::new());
        p.add_macro(Macro::default());
        self.profiles.push(p);
    }

    /// java: removeProfile(index).
    pub fn remove_profile(&mut self, index: usize) {
        if index < self.profiles.len() {
            self.profiles.remove(index);
            self.validate();
        }
    }

    // Validation. java: validate().

    pub fn validate(&mut self) {
        // Validate each profile
        for p in &mut self.profiles {
            p.validate();
        }

        // Validate profile list
        if self.profiles.is_empty() {
            self.profiles = profiles_default();
        }

        // Validate default pointers
        if self.sp_default < 0 || self.sp_default >= self.profiles.len() as i32 {
            self.sp_default = DEFAULT_INDEX_DEFAULT;
        }
        if self.mp_default < 0 || self.mp_default >= self.profiles.len() as i32 {
            self.mp_default = DEFAULT_INDEX_DEFAULT;
        }

        // Validate ratelimit
        if self.ratelimit_count < 1 {
            self.ratelimit_count = RATELIMIT_COUNT_DEFAULT;
        }
        if self.ratelimit_ticks < 1 {
            self.ratelimit_ticks = RATELIMIT_TICKS_DEFAULT;
        }

        // Validate length limit
        if self.length_limit_length < 1 {
            self.length_limit_length = LENGTH_LIMIT_LENGTH_DEFAULT;
        }
    }

    /// java: the Gson Deserializer, field for field.
    pub fn deserialize(obj: &json::Obj) -> Option<Self> {
        let version = obj.i32("version").unwrap_or(0);
        let silent = version != CONFIG_VERSION;

        let profiles = obj
            .array_of("profiles", silent)
            .map(|vals| vals.iter().filter_map(|v| json::Obj::obj_of(v).and_then(|o| Profile::deserialize(&o))).collect::<Vec<_>>())
            .unwrap_or_else(profiles_default);

        let mut c = Self {
            profiles,
            sp_default: obj.i32_or_default("spDefault", DEFAULT_INDEX_DEFAULT, silent),
            mp_default: obj.i32_or_default("mpDefault", DEFAULT_INDEX_DEFAULT, silent),
            default_conflict_strategy: obj.enum_or_default(
                "defaultConflictStrategy",
                ConflictStrategy::parse,
                CONFLICT_STRATEGY_DEFAULT,
                silent,
            ),
            default_send_mode: obj.enum_or_default(
                "defaultSendMode",
                SendMode::parse,
                SEND_MODE_DEFAULT,
                silent,
            ),
            default_activation_type: obj.enum_or_default(
                "defaultActivationType",
                ActivationType::parse,
                ACTIVATION_TYPE_DEFAULT,
                silent,
            ),
            ratelimit_count: obj.i32_or_default("ratelimitCount", RATELIMIT_COUNT_DEFAULT, silent),
            ratelimit_ticks: obj.i32_or_default("ratelimitTicks", RATELIMIT_TICKS_DEFAULT, silent),
            ratelimit_strict:
                obj.bool_or_default("ratelimitStrict", RATELIMIT_STRICT_DEFAULT, silent),
            ratelimit_sp: obj.bool_or_default("ratelimitSp", RATELIMIT_SP_DEFAULT, silent),
            length_limit_length: obj.i32_or_default(
                "lengthLimitLength",
                LENGTH_LIMIT_LENGTH_DEFAULT,
                silent,
            ),
            macro_mc_key_count: obj.i32_or_default(
                "macroMcKeyCount",
                MACRO_MC_KEY_COUNT_DEFAULT,
                silent,
            ),
        };
        c.validate();
        Some(c)
    }

    pub fn serialize(&self) -> json::Val {
        json::obj()
            .int("version", CONFIG_VERSION)
            .array_val(
                "profiles",
                self.profiles.iter().map(Profile::serialize).collect(),
            )
            .int("spDefault", self.sp_default)
            .int("mpDefault", self.mp_default)
            .string("defaultConflictStrategy", self.default_conflict_strategy.name())
            .string("defaultSendMode", self.default_send_mode.name())
            .string("defaultActivationType", self.default_activation_type.name())
            .int("ratelimitCount", self.ratelimit_count)
            .int("ratelimitTicks", self.ratelimit_ticks)
            .boolean("ratelimitStrict", self.ratelimit_strict)
            .boolean("ratelimitSp", self.ratelimit_sp)
            .int("lengthLimitLength", self.length_limit_length)
            .int("macroMcKeyCount", self.macro_mc_key_count)
            .build()
    }
}
