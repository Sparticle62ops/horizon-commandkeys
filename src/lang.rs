//! 1:1 Rust port of `en_us.json`'s keys plus `Localization.java`'s
//! `localized()` shape: the same string keys, the same argument substitution
//! (`%s` positional), so the GUI and messages read identically to upstream.


/// java: Localization.localized(prefix, key, args...): looks up
/// `commandkeys.<prefix>.<key>` and substitutes the args positionally.
pub fn localized(prefix: &str, key: &str, args: &[String]) -> String {
    let full_key = format!("commandkeys.{}.{}", prefix, key);
    let mut out = lookup(&full_key).to_string();
    for arg in args {
        if let Some(pos) = out.find("%s") {
            out.replace_range(pos..pos + 2, arg);
        }
    }
    out
}

/// The en_us strings, java: assets/commandkeys/lang/en_us.json, the keys the
/// running mod touches (the GUI renders its own labels via the same lookup).
fn lookup(key: &str) -> &'static str {
    match key {
        "commandkeys.command.main.description" => "Opens the CommandKeys config screen.",
        "commandkeys.command.main.enabled.description" => "Opens the CommandKeys config screen.",
        "commandkeys.command.main.reload.description" => "Reloads the CommandKeys config from disk.",
        "commandkeys.command.main.reset.description" => "Resets the CommandKeys config to defaults.",
        "commandkeys.command.main.convert.description" => "Converts the CommandKeys config to the current format.",
        "commandkeys.message.commands.message.display.incoming" => "%s whispers to you: %s",
        "commandkeys.message.ratelimit.block" => "You are being ratelimited. Maximum %s messages in %s ticks.",
        "commandkeys.message.placeholderFault" => "Invalid message: %s",
        "commandkeys.message.blocked.lengthlimit" => "Your macro of length %s cannot be sent, as it exceeds the maximum length of %s.",
        "commandkeys.message.loaded" => "Loaded %s macros in %s profiles.",
        "commandkeys.message.saved" => "Saved %s macros in %s profiles.",
        "commandkeys.message.reloaded" => "Reloaded %s macros in %s profiles.",
        "commandkeys.message.resets" => "Reset %s macros in %s profiles.",
        "commandkeys.message.converted" => "Converted %s macros in %s profiles.",
        "commandkeys.message.replaced" => "Replaced macro '%s'.",
        "commandkeys.message.linked" => "Linked profile '%s' to '%s'.",
        "commandkeys.message.unlinked" => "Unlinked profile '%s'.",
        "commandkeys.option.macro.name.default" => "Unnamed macro",
        "commandkeys.option.macro.messages.add" => "+ Add message",
        "commandkeys.option.macro.name.maxLength" => "30 chars",
        "commandkeys.option.macro.messages.label.maxLength" => "256 chars",
        "commandkeys.option.macro.bind.label" => "Keybind",
        "commandkeys.option.macro.altBind.label" => "Alt keybind",
        "commandkeys.option.macro.bind.tooltip" => "The key(s) used to activate this macro.",
        "commandkeys.option.macro.bind.tooltip.conflict.internal" => "Another macro uses the %s key.",
        "commandkeys.option.macro.bind.tooltip.conflict.external" => "The %s key is already used by %s.",
        "commandkeys.option.macro.bind.tooltip.conflictStrategy" => "To control the conflict behavior, set the conflict strategy.",
        "commandkeys.option.macro.bind.tooltip.conflictStrategy.submit" => "SUBMIT: The macro will be sent only if there is no conflict.",
        "commandkeys.option.macro.bind.tooltip.conflictStrategy.assert" => "ASSERT: The macro will be sent regardless of conflict.",
        "commandkeys.option.macro.bind.tooltip.conflictStrategy.veto" => "VETO: The macro will be sent, and any conflicting input will be cancelled.",
        "commandkeys.option.macro.bind.tooltip.conflictStrategy.avoid" => "AVOID: The macro will not be sent if a conflicting key is pressed.",
        "commandkeys.option.macro.mode.label" => "Send mode",
        "commandkeys.option.macro.mode.tooltip.send" => "SEND: Sends the message(s) directly.",
        "commandkeys.option.macro.mode.tooltip.type" => "TYPE: Opens the chat with the message(s) typed in.",
        "commandkeys.option.macro.mode.tooltip.edit" => "EDIT: Opens the chat editor with the first message.",
        "commandkeys.option.macro.mode.tooltip.cycle" => "CYCLE: Sends the next (or previous) message each time.",
        "commandkeys.option.macro.mode.tooltip.random" => "RANDOM: Sends a random message.",
        "commandkeys.option.macro.mode.tooltip.repeat" => "REPEAT: Sends the message(s) repeatedly while held.",
        "commandkeys.option.macro.strategy.label" => "Conflict strategy",
        "commandkeys.option.macro.activationType.label" => "Activation type",
        "commandkeys.option.macro.spaceTicks.label" => "Send interval (ticks)",
        "commandkeys.option.macro.maxRepeats.label" => "Max repeats",
        "commandkeys.option.macro.useRatelimit.label" => "Use ratelimit",
        "commandkeys.option.macro.resumeRepeating.label" => "Resume repeating",
        "commandkeys.option.macro.showHudMessage.label" => "Show HUD message",
        "commandkeys.option.macro.addToHistory.label" => "Add to history",
        "commandkeys.option.profile.label" => "Profile",
        "commandkeys.option.profile.name.label" => "Name",
        "commandkeys.option.profile.links.label" => "Links",
        "commandkeys.option.profile.links.tooltip" => "A singleplayer world ID or multiplayer server address this profile auto-activates for.",
        "commandkeys.option.profile.macros.label" => "Macros",
        "commandkeys.option.configScreen.spDefault.label" => "Default singleplayer profile",
        "commandkeys.option.configScreen.mpDefault.label" => "Default multiplayer profile",
        "commandkeys.option.configScreen.defaultConflictStrategy.label" => "Default conflict strategy",
        "commandkeys.option.configScreen.defaultSendMode.label" => "Default send mode",
        "commandkeys.option.configScreen.defaultActivationType.label" => "Default activation type",
        "commandkeys.option.configScreen.ratelimitCount.label" => "Max messages",
        "commandkeys.option.configScreen.ratelimitTicks.label" => "Ratelimit window (ticks)",
        "commandkeys.option.configScreen.ratelimitStrict.label" => "Strict ratelimit",
        "commandkeys.option.configScreen.ratelimitSp.label" => "Ratelimit singleplayer",
        "commandkeys.option.configScreen.lengthLimitLength.label" => "Max message length",
        "commandkeys.option.configScreen.macroMcKeyCount.label" => "Macro keybind slots",
        // java: I18n falls back to the key itself; the port's callers pass
        // only keys registered here, so the empty string marks the gap.
        _ => "",
    }
}

/// java: mc.gui.hud.getChat().addClientSystemMessage(PREFIX.copy().append(...)):
/// the chat line the prefix + colored message forms.
pub fn chat_system_message(text: &str) {
    crate::host_bridge::chat_system_message(text);
}

/// java: mc.gui.hud.setOverlayMessage(...).
pub fn set_overlay_message(text: &str) {
    crate::host_bridge::set_overlay_message(text);
}
