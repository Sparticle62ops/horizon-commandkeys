//! Tests for the 1:1 port, model layer first: the JSON shapes, validation,
//! legacy deserializers, the rate limiter, the cycle/random/repeat flows and
//! the placeholder math. Host calls no-op outside hooks, so the tests run
//! against the real logic without a host.

use crate::config::*;

#[test]
fn config_defaults() {
    let config = Config::default();
    assert_eq!(config.sp_default, DEFAULT_INDEX_DEFAULT);
    assert_eq!(config.mp_default, DEFAULT_INDEX_DEFAULT);
    assert_eq!(config.ratelimit_count, RATELIMIT_COUNT_DEFAULT);
    assert_eq!(config.ratelimit_ticks, RATELIMIT_TICKS_DEFAULT);
    assert!(!config.ratelimit_strict);
    assert_eq!(config.length_limit_length, LENGTH_LIMIT_LENGTH_DEFAULT);
    assert_eq!(config.profiles.len(), 1);
    assert_eq!(config.profiles[0].name, "Default");
}

#[test]
fn config_roundtrip() {
    let mut config = Config::default();
    config.profiles[0].add_macro(Macro {
        name: "greet".into(),
        send_mode: SendMode::Cycle,
        conflict_strategy: ConflictStrategy::Veto,
        messages: vec![Message::new("hi", 0)],
        ..Macro::default()
    });
    config.validate();
    let text = crate::config_store::serialize_config(&config);
    let parsed = crate::config_store::parse_config(&text).expect("parse");
    assert_eq!(parsed.profiles[0].name, "Default");
    assert_eq!(parsed.profiles[0].macros[0].name, "greet");
    assert_eq!(parsed.profiles[0].macros[0].send_mode, SendMode::Cycle);
    assert_eq!(parsed.profiles[0].macros[0].conflict_strategy, ConflictStrategy::Veto);
}

#[test]
fn profile_control_semantics() {
    let mut profile = Profile::default();
    profile.add_macro(Macro { add_to_history: true, ..Macro::default() });
    // DEFER keeps the macro's own value.
    profile.set_add_to_history_all(Control::Defer);
    assert!(profile.macros[0].add_to_history_status);
    // ON forces it.
    profile.set_add_to_history_all(Control::On);
    assert!(profile.macros[0].add_to_history_status);
    // OFF forces it even when the macro's own flag is true.
    profile.set_add_to_history_macro(0, true);
    profile.set_add_to_history_all(Control::Off);
    assert!(!profile.macros[0].add_to_history_status);
    assert!(profile.macros[0].add_to_history);
}

#[test]
fn macro_validation_rules() {
    let mut m = Macro {
        space_ticks: -5,
        max_repeats: -3,
        ..Macro::default()
    };
    m.validate();
    assert_eq!(m.space_ticks, 0);
    assert_eq!(m.max_repeats, 0);
}

#[test]
fn macro_cleanup_messages() {
    let mut m = Macro {
        send_mode: SendMode::Send,
        messages: vec![
            Message::new("  leading kept", 0),
            Message::new("trailing dropped  ", 0),
            Message::new("   ", 0),
        ],
        ..Macro::default()
    };
    m.cleanup_messages();
    // Leading whitespace stripped, trailing dropped for SEND, blank dropped.
    assert_eq!(m.messages.len(), 2);
    assert_eq!(m.messages[0].string, "leading kept");
    assert_eq!(m.messages[1].string, "trailing dropped");
}

#[test]
fn macro_cleanup_keeps_blank_for_type() {
    let mut m = Macro {
        send_mode: SendMode::Type,
        messages: vec![Message::new("   ", 0)],
        ..Macro::default()
    };
    m.cleanup_messages();
    // TYPE keeps blank messages (opens chat).
    assert_eq!(m.messages.len(), 1);
}

#[test]
fn keybind_validate_unbinds_limit() {
    let mut k = Keybind::from_keys(
        Key::Unknown,
        Key::parse("key.keyboard.l"),
    );
    k.validate();
    assert_eq!(k.limit_key, Key::Unknown);
}

#[test]
fn keybind_roundtrip_names() {
    let k = Keybind::from_keys(Key::parse("key.keyboard.a"), Key::Mouse(0));
    let val = k.serialize();
    let text = serde_json::to_string(&val).unwrap();
    let obj = crate::json::Obj::new(serde_json::from_str(&text).unwrap());
    let parsed = Keybind::deserialize(&obj).unwrap();
    assert_eq!(parsed.key_name, "key.keyboard.a");
    assert_eq!(parsed.key, Key::parse("key.keyboard.a"));
    assert_eq!(parsed.limit_key, Key::Mouse(0));
}

#[test]
fn macro_deserializer_legacy_numeric_strings() {
    // A version-2 macro carries numeric-string conflictStrategy/sendMode.
    let text = serde_json::json!({
        "version": 2,
        "conflictStrategy": "ONE",
        "sendMode": "TWO",
        "keyName": "key.keyboard.h",
        "messages": [{"version": 1, "string": "hi", "delayTicks": 5}]
    })
    .to_string();
    let obj = crate::json::Obj::new(serde_json::from_str(&text).unwrap());
    let m = Macro::deserialize(&obj).unwrap();
    assert_eq!(m.conflict_strategy, ConflictStrategy::Assert);
    assert_eq!(m.send_mode, SendMode::Cycle);
    assert_eq!(m.keybind.key, Key::parse("key.keyboard.h"));
    assert_eq!(m.messages[0].delay_ticks, 5);
}

#[test]
fn profile_deserializer_legacy_addresses() {
    let text = serde_json::json!({
        "version": 2,
        "name": "Old",
        "addresses": ["localhost"],
        "macros": []
    })
    .to_string();
    let obj = crate::json::Obj::new(serde_json::from_str(&text).unwrap());
    let p = Profile::deserialize(&obj).unwrap();
    assert_eq!(p.name, "Old");
    assert_eq!(p.links, vec!["localhost".to_string()]);
}

#[test]
fn json_or_default_flags_changes() {
    let text = serde_json::json!({"version": 7}).to_string();
    let obj = crate::json::Obj::new(serde_json::from_str(&text).unwrap());
    assert_eq!(obj.i32_or_default("ratelimitCount", 4, false), 4);
    assert!(obj.has_changed.get());
}

#[test]
fn activate_profile_reorders_and_repoints() {
    let mut config = Config::default();
    let mut p2 = Profile::new_named("Second".into());
    p2.add_macro(Macro { name: "m".into(), ..Macro::default() });
    config.profiles.push(p2);
    let mut p3 = Profile::new_named("Third".into());
    config.profiles.push(p3);
    config.validate();

    config.activate_profile(2);
    assert_eq!(config.profiles[0].name, "Third");
    // spDefault pointed at index 0, so it shifts right past the removal.
    assert_eq!(config.sp_default, 1);
    assert_eq!(config.mp_default, 1);
}

#[test]
fn cycle_mode_wraps() {
    crate::tests_util::reset_runtime();
    let mut m = Macro {
        send_mode: SendMode::Cycle,
        messages: vec![
            Message::new("a", 0),
            Message::new("b", 0),
            Message::new("c", 0),
        ],
        ..Macro::default()
    };
    m.validate();
    let ((), sent) = crate::tests_util::collect_sends(|| {
        m.trigger(None, false);
        m.tick();
    });
    // First trigger sends the first message and wraps to 0 from -1+1.
    assert_eq!(sent.len(), 1);
}

#[test]
fn repeat_mode_respects_space_ticks_and_max() {
    let mut m = Macro {
        send_mode: SendMode::Repeat,
        activation_type: ActivationType::Hold,
        space_ticks: 2,
        max_repeats: 2,
        messages: vec![Message::new("r", 0)],
        ..Macro::default()
    };
    m.validate();
    m.keybind = Keybind::from_keys(Key::parse("key.keyboard.h"), Key::Unknown);
    m.use_ratelimit_status = false;
    crate::runtime::set_key_state("key.keyboard.h", true);
    let (_, sends) = crate::tests_util::collect_sends(|| {
        m.trigger(None, false);
        for _ in 0..5 {
            m.tick();
        }
    });
    // The trigger's send plus the repeats at the every-2-ticks marks, capped
    // by maxRepeats=2.
    assert_eq!(sends.len(), 2);
}

#[test]
fn scheduled_message_delays() {
    let mut m = Macro::default();
    m.schedule(2, "later".into(), false, false);
    assert_eq!(m.scheduled_messages.len(), 1);
    m.tick();
    assert_eq!(m.scheduled_messages.len(), 1);
    m.tick();
    assert!(m.scheduled_messages.is_empty());
}

#[test]
fn placeholder_offset_directions() {
    let east = (1.0, 0.0, 0.0);
    let west = (-1.0, 0.0, 0.0);
    let south = (0.0, 0.0, 1.0);
    let north = (0.0, 0.0, -1.0);
    let pos = (10.0, 5.0, 10.0);
    // East: F +x, L -z.
    assert_eq!(crate::placeholder::offset_cardinal_direction(pos, east, "F", 3), (13.0, 5.0, 10.0));
    assert_eq!(crate::placeholder::offset_cardinal_direction(pos, east, "L", 3), (10.0, 5.0, 7.0));
    // West: F -x, R -z.
    assert_eq!(crate::placeholder::offset_cardinal_direction(pos, west, "F", 3), (7.0, 5.0, 10.0));
    assert_eq!(crate::placeholder::offset_cardinal_direction(pos, west, "R", 3), (10.0, 5.0, 7.0));
    // South: F +z, L +x.
    assert_eq!(crate::placeholder::offset_cardinal_direction(pos, south, "F", 3), (10.0, 5.0, 13.0));
    assert_eq!(crate::placeholder::offset_cardinal_direction(pos, south, "L", 3), (13.0, 5.0, 10.0));
    // North: F -z, L -x.
    assert_eq!(crate::placeholder::offset_cardinal_direction(pos, north, "F", 3), (10.0, 5.0, 7.0));
    assert_eq!(crate::placeholder::offset_cardinal_direction(pos, north, "L", 3), (7.0, 5.0, 10.0));
}

#[test]
fn placeholder_simple_passthrough() {
    let (out, faults) = crate::placeholder::replace("no placeholders here");
    assert_eq!(out, "no placeholders here");
    assert_eq!(faults, 0);
}

#[test]
fn rate_limiter_blocks_at_count() {
    crate::tests_util::reset_limiter(2, 20, false, false);
    let key = Key::parse("key.keyboard.h");
    assert!(crate::runtime::can_trigger(&key, true));
    assert!(crate::runtime::can_trigger(&key, true));
    assert!(!crate::runtime::can_trigger(&key, true));
    // Strict adds an entry even on the blocked path.
    crate::tests_util::reset_limiter(1, 20, true, false);
    assert!(crate::runtime::can_trigger(&key, true));
    assert!(!crate::runtime::can_trigger(&key, true));
    assert_eq!(crate::runtime::RATE_LIMITER.0.borrow().len(), 2);
}

#[test]
fn keybind_map_rebuild_on_move() {
    let mut profile = Profile::default();
    profile.add_macro(Macro {
        name: "one".into(),
        keybind: Keybind::from_keys(Key::parse("key.keyboard.j"), Key::Unknown),
        messages: vec![Message::new("m", 0)],
        ..Macro::default()
    });
    profile.add_macro(Macro {
        name: "two".into(),
        keybind: Keybind::from_keys(Key::parse("key.keyboard.k"), Key::Unknown),
        messages: vec![Message::new("m", 0)],
        ..Macro::default()
    });
    profile.validate();
    assert!(profile.keybind_map.contains_key(&Key::parse("key.keyboard.j")));
    assert!(profile.keybind_map.contains_key(&Key::parse("key.keyboard.k")));
    profile.move_macro(1, 0);
    assert_eq!(profile.macros[0].name, "two");
    assert!(profile.keybind_map.contains_key(&Key::parse("key.keyboard.k")));
}

#[test]
fn link_map_single_owner() {
    let mut config = Config::default();
    let mut p2 = Profile::new_named("Two".into());
    p2.force_add_link("server.example.net".into());
    config.profiles.push(p2);
    config.validate();
    *crate::runtime::INSTANCE.0.borrow_mut() = Some(config);
    crate::runtime::rebuild_link_map();
    assert_eq!(crate::runtime::link_profile_index("server.example.net"), Some(1));
    // The same link on the default profile moves the owner.
    {
        let mut guard = crate::runtime::INSTANCE.0.borrow_mut();
        let config = guard.as_mut().unwrap();
        config.profiles[0].force_add_link("server.example.net".into());
        config.validate();
    }
    crate::runtime::rebuild_link_map();
    // java: forceAddLink removes the link from its previous owner and puts
    // this profile in the map; the port resolves the owner on rebuild.
    assert_eq!(crate::runtime::link_profile_index("server.example.net"), Some(0));
}

#[test]
fn display_name_fallbacks() {
    let mut p = Profile::default();
    assert_eq!(p.display_name(), "[Unnamed]");
    p.name = "Named".into();
    assert_eq!(p.display_name(), "Named");
    p.name = String::new();
    p.links.push("world-42".into());
    assert_eq!(p.display_name(), "world-42");
}

#[test]
fn send_length_guard_blocks() {
    crate::tests_util::reset_runtime();
    let long = "x".repeat(300);
    crate::runtime::do_send(&long, false, false);
    assert_eq!(crate::tests_util::send_count(), 0);
}

#[test]
fn send_command_prefix() {
    crate::tests_util::reset_runtime();
    crate::runtime::do_send("/say hi", false, false);
    assert_eq!(crate::tests_util::last_command(), Some("/say hi".to_string()));
}
