// Ported to Horizon from CommandKeys by TerminalMC (https://github.com/TerminalMC/CommandKeys).
// Changed from CommandKeys: re-implemented in Rust for the Horizon native-mod API.
// Licensed under the Apache License, Version 2.0 (see the LICENSE file in this directory).

// The mod tick: drains the ratelimiter, sends due scheduled messages and
// keeps repeating macros going, like the original's afterClientTick.
fn tick(game: &mut Game<'_>) {
    crate::game::commandkeys::on_tick(game);
}
