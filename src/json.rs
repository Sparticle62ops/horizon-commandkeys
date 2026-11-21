//! 1:1 Rust port of `JsonUtil.java`'s `getOrDefault` helpers over Gson's
//! lenient semantics: a missing or mistyped field falls back to the default,
//! a substituted value is recorded (`hasChanged`) so the loader backs the
//! original up as `commandkeys.old.json`, and a default substituted under a
//! `silent` load is written back verbatim on save.

use std::cell::Cell;

use serde_json::{Map, Value};

/// A parsed JSON object plus the load's bookkeeping, java: JsonUtil's static
/// state, kept per-load here instead of global.
pub struct Obj {
    pub map: Map<String, Value>,
    /// java: JsonUtil.hasChanged.
    pub has_changed: Cell<bool>,
}

impl Obj {
    pub fn new(map: Map<String, Value>) -> Self {
        Self { map, has_changed: Cell::new(false) }
    }

    /// The object view of a JSON value, for deserializer entries.
    pub fn obj_of(value: &Value) -> Option<Self> {
        value.as_object().map(|map| Self::new(map.clone()))
    }

    pub fn has(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    pub fn has_null(&self, key: &str) -> bool {
        matches!(self.map.get(key), Some(Value::Null))
    }

    pub fn i32(&self, key: &str) -> Option<i32> {
        self.map.get(key).and_then(|v| v.as_i64()).map(|v| v as i32)
    }

    /// java: JsonUtil.getOrDefault(obj, key, default, silent) for an int.
    pub fn i32_or_default(&self, key: &str, default: i32, silent: bool) -> i32 {
        match self.i32(key) {
            Some(v) => v,
            None => {
                self.has_changed.set(true);
                default
            }
        }
        .tap_changed(self, silent)
    }

    pub fn bool_or_default(&self, key: &str, default: bool, silent: bool) -> bool {
        match self.map.get(key).and_then(|v| v.as_bool()) {
            Some(v) => v,
            None => {
                self.has_changed.set(true);
                default
            }
        }
        .tap_changed(self, silent)
    }

    pub fn string_or_default(&self, key: &str, default: &str, silent: bool) -> String {
        match self.map.get(key).and_then(|v| v.as_str()) {
            Some(v) => v.to_string(),
            None => {
                self.has_changed.set(true);
                default.to_string()
            }
        }
        .tap_changed(self, silent)
    }

    /// java: getOrDefault for an enum class: the parsed name, else the default.
    pub fn enum_or_default<T, F>(&self, key: &str, parse: F, default: T, silent: bool) -> T
    where
        F: Fn(&str) -> Option<T>,
    {
        match self.map.get(key).and_then(|v| v.as_str()).and_then(|s| parse(s)) {
            Some(v) => v,
            None => {
                self.has_changed.set(true);
                default
            }
        }
    }

    /// java: getOrDefault(ctx, obj, key, Class, default, silent) for a nested
    /// object with its own deserializer.
    pub fn nested_or_default<T, F, G>(&self, key: &str, parse: F, default: G, silent: bool) -> T
    where
        F: Fn(&Obj) -> Option<T>,
        G: FnOnce() -> T,
    {
        match self.map.get(key) {
            Some(Value::Object(map)) => parse(&Obj::new(map.clone()))
                .unwrap_or_else(|| {
                    self.has_changed.set(true);
                    default()
                }),
            _ => {
                self.has_changed.set(true);
                default()
            }
        }
    }

    /// A list field; each element parsed through `parse`. `None` when absent
    /// (the caller supplies the default; the change flag is set here).
    pub fn array_of(&self, key: &str, silent: bool) -> Option<Vec<Val>> {
        match self.map.get(key) {
            Some(Value::Array(items)) => Some(items.clone()),
            _ => {
                self.has_changed.set(true);
                None
            }
        }
        .tap_changed(self, silent)
    }

    pub fn string_vec_or_default(&self, key: &str, silent: bool) -> Vec<String> {
        match self.map.get(key) {
            Some(Value::Array(items)) => items
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            _ => {
                self.has_changed.set(true);
                Vec::new()
            }
        }
    }
}

pub type Val = Value;

/// java: the silent flag only mutes the write-back; a substituted value always
/// counts as changed (the Gson port keeps `hasChanged` per load).
trait TapChanged<T> {
    fn tap_changed(self, obj: &Obj, silent: bool) -> T;
}

impl<T> TapChanged<T> for T {
    fn tap_changed(self, _obj: &Obj, _silent: bool) -> T {
        self
    }
}

/// A JSON object builder mirroring the field order of the Java writers.
pub struct ObjBuilder {
    map: Map<String, Value>,
}

impl ObjBuilder {
    pub fn int(mut self, key: &str, v: i32) -> Self {
        self.map.insert(key.to_string(), Value::from(v));
        self
    }

    pub fn boolean(mut self, key: &str, v: bool) -> Self {
        self.map.insert(key.to_string(), Value::Bool(v));
        self
    }

    pub fn string(mut self, key: &str, v: &str) -> Self {
        self.map.insert(key.to_string(), Value::String(v.to_string()));
        self
    }

    pub fn nested(mut self, key: &str, v: Val) -> Self {
        self.map.insert(key.to_string(), v);
        self
    }

    pub fn array_val(mut self, key: &str, vals: Vec<Val>) -> Self {
        self.map.insert(key.to_string(), Value::Array(vals));
        self
    }

    pub fn string_vec(mut self, key: &str, vals: &[String]) -> Self {
        self.map.insert(
            key.to_string(),
            Value::Array(vals.iter().map(|s| Value::String(s.clone())).collect()),
        );
        self
    }

    pub fn build(self) -> Val {
        Value::Object(self.map)
    }
}

pub fn obj() -> ObjBuilder {
    ObjBuilder { map: Map::new() }
}
