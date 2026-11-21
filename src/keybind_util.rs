//! 1:1 Rust port of `KeybindUtil.java`: the mod-API key handlers
//! (`handleKey`, `handleKeys`, `getConflict`) and the `KeybindInfo` conflict
//! scanner the GUI uses.

use crate::config::{ConflictStrategy, Key, Keybind, Profile, SendMode};

/// java: handleKeys(key, limitKey), the mod-API dispatch other mods use to
/// activate macros: returns the number activated.
pub fn handle_keys(key: &Key, limit_key: &Key) -> usize {
    if *key == Key::Unknown {
        return 0;
    }
    // Read the matching keybinds and macros inside one borrow, then trigger
    // outside it (the trigger path re-enters the profile).
    let plan = crate::runtime::with_active_profile(|profile| {
        let mut plan = Vec::new();
        if let Some(keybinds) = profile.keybind_map.get(key) {
            for keybind in keybinds {
                if &keybind.limit_key != limit_key {
                    continue;
                }
                if let Some(macro_indices) = profile.macro_map.get(keybind) {
                    for index in macro_indices {
                        plan.push((*index, keybind.clone()));
                    }
                }
            }
        }
        plan
    });
    let mut i = 0;
    for (index, keybind) in plan {
        crate::runtime::trigger_macro(index, Some(keybind), false);
        i += 1;
    }
    i
}

/// java: handleKey(key). Returns 0 (nothing), 1 (cancel charTyped) or
/// 2 (cancel charTyped and KeyMapping#click).
pub fn handle_key(key: &Key) -> i32 {
    let mut cancel = 0;

    // java: only when no screen is open.
    if crate::runtime::screen_open() {
        return cancel;
    }

    // Read the match plan inside one borrow, then trigger outside it.
    let plan = crate::runtime::with_active_profile(|profile| {
        let Some(keybinds) = profile.keybind_map.get(key).cloned() else {
            return None;
        };
        let mut trigger_kb: Option<Keybind> = None;
        let mut macros: Option<Vec<usize>> = None;
        for kb in &keybinds {
            if kb.is_limit_key_down() {
                let list = profile
                    .macro_map
                    .get(kb)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|i| profile.macros[*i].conflict_strategy != ConflictStrategy::Avoid)
                    .collect::<Vec<_>>();
                if !list.is_empty() {
                    trigger_kb = Some(kb.clone());
                    macros = Some(list);
                    break;
                }
                trigger_kb = Some(kb.clone());
            } else if kb.limit_key == Key::Unknown {
                trigger_kb = Some(kb.clone());
            }
        }
        let Some(trigger_kb) = trigger_kb else { return None };
        let macros = match macros {
            Some(m) if !m.is_empty() => m,
            _ => {
                let list = profile
                    .macro_map
                    .get(&trigger_kb)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|i| profile.macros[*i].conflict_strategy != ConflictStrategy::Avoid)
                    .collect::<Vec<_>>();
                if list.is_empty() {
                    return None;
                }
                list
            }
        };
        Some((trigger_kb, macros))
    });

    let Some((trigger_kb, macros)) = plan else {
        return cancel;
    };
    let mut ratelimited = false;
    for index in macros {
        let mut send = true;
        let strategy = crate::runtime::with_active_profile(|p| p.macros[index].conflict_strategy);
        match strategy {
            // SUBMIT only allows sending if there's no conflict
            ConflictStrategy::Submit => send = get_conflict(key).is_none(),
            // VETO requires cancelling everything
            ConflictStrategy::Veto => cancel = 2,
            _ => {}
        }
        if send {
            let rl = crate::runtime::trigger_macro(index, Some(trigger_kb.clone()), ratelimited);
            // TYPE, EDIT modes require cancelling char
            let mode = crate::runtime::with_active_profile(|p| p.macros[index].send_mode);
            if !rl && cancel == 0 && matches!(mode, SendMode::Type | SendMode::Edit) {
                cancel = 1;
            }
            ratelimited |= rl;
        }
    }
    cancel
}

/// java: getConflict(key): the first non-debug key mapping bound to `key`.
/// The SDK surfaces mappings by name, so the by-key scan walks the registered
/// mapping names this port registered and reports the first whose assigned
/// key matches.
pub fn get_conflict(key: &Key) -> Option<String> {
    crate::runtime::get_mapping_conflict(key)
}

/// java: KeybindInfo: the label + conflict tooltip for one macro's keybind,
/// used by the GUI.
pub struct KeybindInfo {
    /// java: label.
    pub label: String,
    /// java: conflictLabel.
    pub conflict_label: String,
    /// java: tooltip.
    pub tooltip: String,
}

impl KeybindInfo {
    pub fn new(profile: &Profile, macro_index: usize, keybind: &Keybind) -> Self {
        let mut info = Self {
            label: String::new(),
            conflict_label: String::new(),
            tooltip: String::new(),
        };
        let label = if keybind.limit_key == Key::Unknown {
            keybind.key.name()
        } else {
            format!("{} + {}", keybind.limit_key.name(), keybind.key.name())
        };
        info.label = label;
        check_conflict(profile, macro_index, keybind.limit_key.clone(), None, &mut info);
        check_conflict(profile, macro_index, keybind.key.clone(), Some(keybind), &mut info);
        create_conflict_label(&mut info, profile, macro_index);
        info
    }
}

/// java: KeybindInfo#checkConflict.
fn check_conflict(
    profile: &Profile,
    macro_index: usize,
    key: Key,
    keybind: Option<&Keybind>,
    info: &mut KeybindInfo,
) {
    if key == Key::Unknown {
        return;
    }
    let macro_conflict_strategy = profile.macros[macro_index].conflict_strategy;
    let key_name = key.name();
    // Check internal conflict
    if profile.keybind_map.get(&key).map(|v| v.len()).unwrap_or(0) > 1 {
        if !info.tooltip.is_empty() {
            info.tooltip.push('\n');
        }
        info.tooltip.push_str(&crate::lang::localized(
            "option",
            "macro.bind.tooltip.conflict.internal",
            &[key_name.clone()],
        ));
    } else if let Some(keybind) = keybind {
        if profile
            .macro_map
            .get(keybind)
            .map(|v| v.len())
            .unwrap_or(0)
            > 1
        {
            if !info.tooltip.is_empty() {
                info.tooltip.push('\n');
            }
            info.tooltip.push_str(&crate::lang::localized(
                "option",
                "macro.bind.tooltip.conflict.internal",
                &[key_name.clone()],
            ));
        }
    }
    if macro_conflict_strategy != ConflictStrategy::Avoid {
        // Check MC conflict
        if let Some(key_mapping) = get_conflict(&key) {
            if !info.tooltip.is_empty() {
                info.tooltip.push('\n');
            }
            info.tooltip.push_str(&crate::lang::localized(
                "option",
                "macro.bind.tooltip.conflict.external",
                &[key_name, key_mapping],
            ));
        }
    }
}

/// java: KeybindInfo#createConflictLabel.
fn create_conflict_label(info: &mut KeybindInfo, profile: &Profile, macro_index: usize) {
    if get_conflict(&profile.macros[macro_index].keybind.key).is_some()
        && profile.macros[macro_index].conflict_strategy != ConflictStrategy::Avoid
    {
        info.conflict_label = format!("[ {} ]", info.label);
        info.tooltip.push_str(&crate::lang::localized(
            "option",
            "macro.bind.tooltip.conflictStrategy",
            &[],
        ));
    } else {
        // No MC conflict; the label stays plain (the GUI applies its own
        // bracket styling for internal conflicts).
        info.conflict_label = info.label.clone();
    }
}

