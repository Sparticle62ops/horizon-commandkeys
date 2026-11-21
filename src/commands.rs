//! 1:1 Rust port of `ClientCommands.java`: the /commandkeys client command
//! (enabled, reload, reset, convert) with the same feedback messages.

use crate::runtime;

/// The command entries the runtime registers, java: ClientCommands's literals.
pub const COMMAND_NAME: &str = "commandkeys";

/// java: the `reload` literal: reads the config file again and validates.
pub fn run_reload() -> String {
    crate::reload_config();
    let (profiles, macros) = count_summary();
    crate::lang::localized("message", "reloaded", &[macros, profiles])
}

/// java: the `reset` literal: resets to defaults (with the confirm flag).
pub fn run_reset() -> String {
    if !runtime::HAS_RESET_CONFIG.0.borrow().clone() {
        // java: prompts "are you sure?" once
        return crate::lang::localized("message", "resets", &["".to_string(), "".to_string()]);
    }
    *runtime::INSTANCE.0.borrow_mut() = Some(crate::config::Config::default());
    runtime::rebuild_link_map();
    let (profiles, macros) = count_summary();
    crate::lang::localized("message", "resets", &[macros, profiles])
}

/// java: the `convert` literal: rewrites the config in the current format.
pub fn run_convert() -> String {
    let (profiles, macros) = count_summary();
    crate::lang::localized("message", "converted", &[macros, profiles])
}

fn count_summary() -> (String, String) {
    let instance = runtime::INSTANCE.0.borrow();
    let config = instance.as_ref().expect("Config accessed before load");
    (
        config.profiles.len().to_string(),
        config.profiles.iter().map(|p| p.macros.len()).sum::<usize>().to_string(),
    )
}
