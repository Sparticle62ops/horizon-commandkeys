# CommandKeys (Horizon Rust port)

Exact 1:1 Rust port of [CommandKeys](https://github.com/TerminalMC/CommandKeys)
(upstream Java, Apache-2.0) by TerminalMC, for the Horizon client. The port
carries the upstream licence and credit.

Bind keys to macros: profiles of macros, each macro a list of messages with a
primary and optional alternate keybind, per-message delays (`delayTicks`),
send modes (SEND, TYPE, EDIT, CYCLE, RANDOM, REPEAT with `spaceTicks` and
`maxRepeats`), activation types (HOLD, VANILLA, RELEASE), dual-key
activation (`keybind.limitKeyName` must be held), conflict strategies
(SUBMIT, ASSERT, VETO, AVOID), per-macro placeholder substitution, a global
ratelimit (`ratelimitCount` triggers per `ratelimitTicks`), the chat length
limit and automatic per-world/per-server profile switching via `links`.

## Modules (one per upstream class)

| Upstream | Port |
| --- | --- |
| `CommandKeys` (entry, send/type/edit paths) | `src/lib.rs`, `src/runtime.rs` |
| `Config` (config + profiles) | `src/config.rs`, `src/config_store.rs` |
| `Profile` / `Macro` / `Message` / `Keybind` | `src/config.rs` |
| `JsonUtil` (lenient JSON + `hasChanged` bookkeeping) | `src/json.rs` |
| `KeybindUtil` (key dispatch, conflicts) | `src/keybind_util.rs` |
| `PlaceholderUtil` (placeholder substitution) | `src/placeholder.rs` |
| `Commands` (`/commandkeys` command tree) | `src/commands.rs` |
| `Localization` (en_us lang) | `src/lang.rs`, `assets/commandkeys/lang/en_us.json` |
| Key / tick / mouse / chat / connect-screen mixins | `src/client_bridge.rs`, `src/host_bridge.rs`, `src/generated.rs` |
| Config + macro editor screens | `src/gui.rs` |

## Mod API wiring

- The config key (`K`) and the macros' Minecraft activator keys register as
  key mappings with the upstream conflict contexts; presses and releases
  track held state on every input event, and the dispatch path gates itself
  on the open screen like upstream's `handleKey`.
- The trigger path runs through the input, mouse and client-tick hooks; chat
  sends go through the network host (`SEND`) or the chat opener (`TYPE` /
  `EDIT`); the `/commandkeys` command tree registers as client commands.
- The config lives at the mod data dir's `commandkeys.json`, schema-compatible
  with the original's `config/commandkeys.json`, written back when a load
  substituted defaults (the `JsonUtil.hasChanged` bookkeeping).

## Known seams (engine-stage; tracked)

- Chat lines arrive without their translation keys on the SDK, so the
  incoming-PM placeholder scan cannot match (`PlaceholderUtil.pmSenderName`
  faults until one lands).
- A chat component's resolved text is not readable through the API surface
  (`TextContent` is not exported), so recorded lines carry no content.
- Config reload outside a host hook keeps the loaded config in memory.

Pinned to Horizon f00125f and horizon-mixins e59da909; exact 1:1 Rust port
tested against those revisions (23 tests green).

Licence: Apache-2.0 (LICENSE carries the upstream text).
Source: https://github.com/Sparticle62ops/horizon-commandkeys
