//! The config file store, java: Config.load/save through ConfigFiles. The
//! host's ConfigFiles reads and writes the mod's data dir; the port keeps the
//! same JSON shape on disk and the same `commandkeys.old.json` backup on a
//! substituted load.

use std::cell::Cell;

use crate::config::Config;
use crate::json;
use crate::runtime::Ts;

pub static DIRTY: Ts<Cell<bool>> = Ts(Cell::new(false));

pub fn mark_dirty() {
    DIRTY.0.set(true);
}

pub fn dirty() -> bool {
    DIRTY.0.get()
}

/// java: Config.load(): read the file (host supplies the bytes), parse
/// leniently, substitute defaults for anything missing (recording the change),
/// then save the normalized config. Returns the config.
pub fn parse_config(text: &str) -> Option<Config> {
    let val: serde_json::Value = serde_json::from_str(text).ok()?;
    let map = val.as_object()?.clone();
    let obj = json::Obj::new(map);
    Config::deserialize(&obj)
}

/// java: Config.save(): the serialized shape, written by the host.
pub fn serialize_config(config: &Config) -> String {
    let val = config.serialize();
    serde_json::to_string_pretty(&val).unwrap_or_else(|_| "{}".to_string())
}

/// java: Config.load's read: the config file's bytes through the host's
/// checked config files, `commandkeys.json` as upstream names it.
pub fn config_file_path() -> mod_api::ConfigPath {
    mod_api::ConfigPath::new("commandkeys.json").expect("valid config path")
}

pub fn read_text(files: &dyn mod_api::ConfigFiles) -> Option<String> {
    let path = config_file_path();
    match files.read(&path) {
        Ok(bytes) => Some(String::from_utf8_lossy(&bytes).to_string()),
        Err(_) => None,
    }
}

pub fn write_text(files: &dyn mod_api::ConfigFiles, name: &str, text: &str) -> bool {
    match mod_api::ConfigPath::new(name) {
        Ok(path) => files.write(&path, text.as_bytes()).is_ok(),
        Err(_) => false,
    }
}

pub fn clear_dirty() {
    DIRTY.0.set(false);
}
