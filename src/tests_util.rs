//! Test helpers: a send recorder around the bridge's log, the rate-limiter
//! and runtime resets the model tests need.

use crate::config::*;

/// Runs `f`, returning the chat/command lines the send chain carried.
pub fn collect_sends<T>(f: impl FnOnce() -> T) -> (T, Vec<String>) {
    crate::generated::SENDS.0.borrow_mut().clear();
    let out = f();
    let sends = crate::generated::SENDS.0.borrow().clone();
    (out, sends)
}

pub fn send_count() -> usize {
    crate::generated::SENDS.0.borrow().len()
}

pub fn last_command() -> Option<String> {
    crate::generated::SENDS.0.borrow().last().cloned()
}

/// A fresh runtime with a config carrying the limiter params the test sets.
pub fn reset_limiter(count: i32, ticks: i32, strict: bool, sp: bool) {
    let mut config = Config::default();
    config.ratelimit_count = count;
    config.ratelimit_ticks = ticks;
    config.ratelimit_strict = strict;
    config.ratelimit_sp = sp;
    config.validate();
    *crate::runtime::INSTANCE.0.borrow_mut() = Some(config);
    crate::runtime::RATE_LIMITER.0.borrow_mut().clear();
    *crate::runtime::RATELIMITED_KEY.0.borrow_mut() = None;
    crate::runtime::TICK_COUNT.0.set(1_000_000);
}

/// A fresh runtime with default config.
pub fn reset_runtime() {
    *crate::runtime::INSTANCE.0.borrow_mut() = Some(Config::default());
    crate::generated::SENDS.0.borrow_mut().clear();
}
