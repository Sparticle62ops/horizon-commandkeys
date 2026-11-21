//! Integration tests for the 1:1 port over the crate's public surface. A
//! process-wide lock serializes the tests that touch the crate's global state
//! (the live config cell, key state and the bridge), mirroring the JVM's
//! single-instance statics.

use commandkeys::*;
use std::sync::{Mutex, OnceLock};

fn state_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

// java: the Config constructor's default profile, and the serializer
// round-trip the loader does.
#[test]
fn config_holder_roundtrip() {
    let _g = state_lock();
    let mut config = config::Config::default();
    assert_eq!(config.profiles.len(), 1);
    assert_eq!(config.profiles[0].name, "Default");
    config.profiles[0].add_macro(config::Macro {
        name: "greet".into(),
        send_mode: config::SendMode::Cycle,
        messages: vec![config::Message::new("hi", 0)],
        ..config::Macro::default()
    });
    config.validate();
    let text = config_store::serialize_config(&config);
    let parsed = config_store::parse_config(&text).expect("parse");
    assert_eq!(parsed.profiles[0].macros[0].name, "greet");
    assert_eq!(parsed.profiles[0].macros[0].send_mode, config::SendMode::Cycle);
}

// java: PlaceholderUtil.replace passes plain text through with no faults.
#[test]
fn placeholder_passthrough() {
    let _g = state_lock();
    let (out, faults) = placeholder::replace("hello world");
    assert_eq!(out, "hello world");
    assert_eq!(faults, 0);
}

// java: Localization resolves the registered en_us keys.
#[test]
fn lang_resolves_registered_keys() {
    let _g = state_lock();
    let s = lang::localized("message", "ratelimit.block", &["3".into(), "20".into()]);
    assert!(s.contains("3") && s.contains("20"));
}
