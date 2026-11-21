//! 1:1 Rust port of `PlaceholderUtil.java`: the same simple and regex
//! placeholder tables, the same `?` fault and fault counting, the same
//! cardinal-direction offset math. The world reads (player block position,
//! look angle, clipboard, chat history) come from the per-tick host snapshot
//! `runtime` fills; upstream reads the same state from Minecraft directly.

use std::collections::VecDeque;

use regex::Regex;

use crate::runtime;

/// java: replace(String) -> (String, faults).
pub fn replace(message: &str) -> (String, i32) {
    if !message.contains('%') {
        return (message.to_string(), 0);
    }
    runtime::placeholder_state_reset();
    let mut message = message.to_string();
    for p in SIMPLE_PLACEHOLDERS {
        if message.contains(p.0) {
            let replacement = (p.1)();
            message = replace_literal(&message, p.0, &replacement);
        }
    }
    for p in REGEX_PLACEHOLDERS {
        match Regex::new(p.0) {
            Ok(re) => {
                while let Some(caps) = re.captures(&message) {
                    let mut args: Vec<String> = Vec::with_capacity(p.1);
                    for i in 1..=p.1 {
                        if let Some(m) = caps.get(i) {
                            args.push(m.as_str().to_string());
                        }
                    }
                    let replacement = (p.2)(&args);
                    // java: matcher.replaceFirst, $N group references live.
                    message = re.replace(&message, replacement.as_str()).to_string();
                }
            }
            // java: PatternSyntaxException leaves the message as is.
            Err(_) => continue,
        }
    }
    (message, runtime::placeholder_faults())
}

/// java: String.replaceAll with a regex replacement string ($ group refs live).
fn replace_literal(message: &str, pattern: &str, replacement: &str) -> String {
    match Regex::new(&regex::escape(pattern)) {
        Ok(re) => re.replace_all(message, replacement).to_string(),
        Err(_) => message.to_string(),
    }
}

/// java: SimplePlaceholder.
type SimpleSupplier = fn() -> String;
const SIMPLE_PLACEHOLDERS: [(&str, SimpleSupplier); 13] = [
    ("%lastsent%", || runtime::get_last_message()),
    ("%lastcmd%", || runtime::get_last_command()),
    ("%clipboard%", || get_clipboard(None)),
    ("%myname%", || runtime::get_player_name()),
    ("%pmsender%", || get_pm_sender_name()),
    ("%pos%", || get_player_block_pos(&["0".to_string(), "0".to_string()])),
    ("%x%", || get_player_block_x(&["0".to_string()])),
    ("%y%", || get_player_block_y(&["0".to_string()])),
    ("%z%", || get_player_block_z(&["0".to_string()])),
    ("%lpos%", || get_look_block_pos(&["0".to_string(), "0".to_string()])),
    ("%lx%", || get_look_block_x(&["0".to_string()])),
    ("%ly%", || get_look_block_y(&["0".to_string()])),
    ("%lz%", || get_look_block_z(&["0".to_string()])),
];

/// java: Placeholder (pattern, groups, operator).
type RegexOperator = fn(&[String]) -> String;
const REGEX_PLACEHOLDERS: [(&str, usize, RegexOperator); 10] = [
    ("%#(.*)%", 1, |args| get_recent_chat(&args)),
    ("%clipboard#(.*)%", 1, |args| get_clipboard(Some(&args))),
    ("%pos([FBLR])(\\d+)%", 2, |args| get_player_block_pos(&args)),
    ("%x([+-]\\d+)%", 1, |args| get_player_block_x(&args)),
    ("%y([+-]\\d+)%", 1, |args| get_player_block_y(&args)),
    ("%z([+-]\\d+)%", 1, |args| get_player_block_z(&args)),
    ("%lpos([FBLR])(\\d+)%", 2, |args| get_look_block_pos(&args)),
    ("%lx([+-]\\d+)%", 1, |args| get_look_block_x(&args)),
    ("%ly([+-]\\d+)%", 1, |args| get_look_block_y(&args)),
    ("%lz([+-]\\d+)%", 1, |args| get_look_block_z(&args)),
];

/// java: getRecentChat(pattern), the newest 50 received lines scanned for the
/// first match, group 1 returned.
fn get_recent_chat(args: &[String]) -> String {
    match Regex::new(&args[0]) {
        Ok(re) => {
            let mut i = 0;
            for (_, text) in runtime::all_messages() {
                i += 1;
                if i > 50 {
                    break;
                }
                if let Some(caps) = re.captures(&text) {
                    if let Some(g) = caps.get(1) {
                        return g.as_str().to_string();
                    }
                    // java: IndexOutOfBoundsException -> fault
                    runtime::log_error("Recent chat placeholder failed: Group 1 not available");
                    return fault();
                }
            }
            runtime::log_warn(&format!(
                "Recent chat placeholder failed: No message found: Checked {}",
                i
            ));
        }
        Err(e) => {
            runtime::log_error(&format!(
                "Recent chat placeholder failed: Invalid regex: {}",
                e
            ));
        }
    }
    fault()
}

/// java: getClipboard(pattern), the clipboard text, regex-checked when a
/// pattern is given.
fn get_clipboard(pattern: Option<&[String]>) -> String {
    let clipboard = runtime::get_clipboard();
    if clipboard.is_empty() {
        runtime::log_warn("Clipboard placeholder failed: No data");
        return fault();
    }
    if let Some(pattern) = pattern {
        match Regex::new(&pattern[0]) {
            Ok(re) => {
                if re.find(&clipboard).is_none() {
                    runtime::log_warn("Clipboard placeholder failed: Non-matching regex");
                    return fault();
                }
            }
            Err(e) => {
                runtime::log_warn(&format!("Clipboard placeholder failed: Invalid regex: {}", e));
                return fault();
            }
        }
    }
    clipboard
}

/// java: getMyName.
fn get_player_name() -> String {
    runtime::get_player_name()
}

/// java: getPmSenderName(): the newest incoming private message's sender,
/// found by the `commands.message.display.incoming` translation key in the
/// newest 50 chat lines.
fn get_pm_sender_name() -> String {
    if let Some(name) = runtime::pm_sender_name() {
        return name;
    }
    let mut i = 0;
    for (key, text) in runtime::all_messages() {
        i += 1;
        if i > 50 {
            break;
        }
        if let Some(key) = key {
            if key.contains("commands.message.display.incoming") {
                if let Some(name) = runtime::incoming_pm_sender(&text) {
                    runtime::set_pm_sender_name(name.clone());
                    return name;
                }
            }
        }
    }
    runtime::log_warn(&format!(
        "PmSenderName placeholder failed: No message found: Checked {}",
        i
    ));
    fault()
}

// Player position. java: updatePlayerBlockPos / updateLookBlockPos cache
// their results for the whole replace() call; the runtime snapshot carries
// them.

fn get_player_block_pos(args: &[String]) -> String {
    let (pos, angle) = (runtime::player_block_pos(), runtime::look_angle());
    let (Some((x, y, z)), Some(angle)) = (pos, angle) else {
        return fault();
    };
    let offset: i32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let player_pos = (x as f64, y as f64, z as f64);
    let player_pos = if offset != 0 {
        offset_cardinal_direction(player_pos, angle, &args[0], offset)
    } else {
        player_pos
    };
    format!(
        "{} {} {}",
        player_pos.0.floor() as i64,
        player_pos.1.floor() as i64,
        player_pos.2.floor() as i64
    )
}

fn get_player_block_x(args: &[String]) -> String {
    match runtime::player_block_pos() {
        Some((x, _, _)) => format!(
            "{}",
            x + args
                .first()
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0)
        ),
        None => fault(),
    }
}

fn get_player_block_y(args: &[String]) -> String {
    match runtime::player_block_pos() {
        Some((_, y, _)) => format!(
            "{}",
            y + args
                .first()
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0)
        ),
        None => fault(),
    }
}

fn get_player_block_z(args: &[String]) -> String {
    match runtime::player_block_pos() {
        Some((_, _, z)) => format!(
            "{}",
            z + args
                .first()
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0)
        ),
        None => fault(),
    }
}

fn get_look_block_pos(args: &[String]) -> String {
    let (pos, angle) = (runtime::look_block_pos(), runtime::look_angle());
    let (Some((x, y, z)), Some(angle)) = (pos, angle) else {
        return fault();
    };
    let offset: i32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let look_pos = (x as f64, y as f64, z as f64);
    let look_pos = if offset != 0 {
        offset_cardinal_direction(look_pos, angle, &args[0], offset)
    } else {
        look_pos
    };
    format!(
        "{} {} {}",
        look_pos.0.floor() as i64,
        look_pos.1.floor() as i64,
        look_pos.2.floor() as i64
    )
}

fn get_look_block_x(args: &[String]) -> String {
    match runtime::look_block_pos() {
        Some((x, _, _)) => format!(
            "{}",
            x + args
                .first()
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0)
        ),
        None => fault(),
    }
}

fn get_look_block_y(args: &[String]) -> String {
    match runtime::look_block_pos() {
        Some((_, y, _)) => format!(
            "{}",
            y + args
                .first()
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0)
        ),
        None => fault(),
    }
}

fn get_look_block_z(args: &[String]) -> String {
    match runtime::look_block_pos() {
        Some((_, _, z)) => format!(
            "{}",
            z + args
                .first()
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0)
        ),
        None => fault(),
    }
}

/// java: offsetCardinalDirection(pos, facingAngle, offsetDir, offset): the
/// same four-way facing buckets (east/west by x, then south/north by z), the
/// same F/B/L/R offsets.
pub fn offset_cardinal_direction(
    pos: (f64, f64, f64),
    facing_angle: (f64, f64, f64),
    offset_dir: &str,
    offset: i32,
) -> (f64, f64, f64) {
    let o = offset as f64;
    if facing_angle.0.abs() >= facing_angle.2.abs() {
        if facing_angle.0 >= 0.0 {
            // East
            match offset_dir {
                "F" => (pos.0 + o, pos.1, pos.2),
                "B" => (pos.0 - o, pos.1, pos.2),
                "L" => (pos.0, pos.1, pos.2 - o),
                "R" => (pos.0, pos.1, pos.2 + o),
                _ => (pos.0, pos.1, pos.2),
            }
        } else {
            // West
            match offset_dir {
                "F" => (pos.0 - o, pos.1, pos.2),
                "B" => (pos.0 + o, pos.1, pos.2),
                "L" => (pos.0, pos.1, pos.2 + o),
                "R" => (pos.0, pos.1, pos.2 - o),
                _ => (pos.0, pos.1, pos.2),
            }
        }
    } else if facing_angle.2 >= 0.0 {
        // South
        match offset_dir {
            "F" => (pos.0, pos.1, pos.2 + o),
            "B" => (pos.0, pos.1, pos.2 - o),
            "L" => (pos.0 + o, pos.1, pos.2),
            "R" => (pos.0 - o, pos.1, pos.2),
            _ => (pos.0, pos.1, pos.2),
        }
    } else {
        // North
        match offset_dir {
            "F" => (pos.0, pos.1, pos.2 - o),
            "B" => (pos.0, pos.1, pos.2 + o),
            "L" => (pos.0 - o, pos.1, pos.2),
            "R" => (pos.0 + o, pos.1, pos.2),
            _ => (pos.0, pos.1, pos.2),
        }
    }
}

/// java: fault(): `?` plus a fault count.
fn fault() -> String {
    runtime::placeholder_fault();
    "?".to_string()
}

// Keep the deque import referenced for the runtime's types.
#[allow(dead_code)]
type _KeepDeque = VecDeque<()>;
