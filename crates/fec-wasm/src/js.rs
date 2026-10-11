//! Small helpers for building the plain JS objects the shared TypeScript
//! layer expects, and errors carrying the `code` that `js/errors.ts`
//! (`fromNative`) maps to the public classes.

use js_sys::{Array, Object, Reflect};
use wasm_bindgen::prelude::*;

/// A plain object built key by key.
pub struct Obj(Object);

impl Obj {
    pub fn new() -> Self {
        Obj(Object::new())
    }

    pub fn set(self, key: &str, value: impl Into<JsValue>) -> Self {
        // Reflect.set on a fresh plain object can't fail.
        let _ = Reflect::set(&self.0, &JsValue::from_str(key), &value.into());
        self
    }

    pub fn build(self) -> JsValue {
        self.0.into()
    }
}

/// `Option<&str>` as a string or `null` (never `undefined`: the napi build
/// uses `null` for every missing value).
pub fn opt_str(s: Option<&str>) -> JsValue {
    s.map_or(JsValue::NULL, JsValue::from_str)
}

/// Strings as a JS array.
pub fn str_array<'a>(items: impl IntoIterator<Item = &'a str>) -> Array {
    items.into_iter().map(JsValue::from_str).collect()
}

/// An `Error` whose `code` says what went wrong (the same codes as
/// `crates/fec-node/src/errors.rs`).
pub fn coded_error(code: &str, message: &str) -> JsValue {
    let e = js_sys::Error::new(message);
    let _ = Reflect::set(&e, &JsValue::from_str("code"), &JsValue::from_str(code));
    e.into()
}

/// Anything the parser rejected: `FEC_PARSE`.
pub fn parse_error(e: &dyn std::fmt::Display) -> JsValue {
    coded_error("FEC_PARSE", &format!("{e:#}"))
}

/// A path or `file:` URL source: there is no file system in the browser. A
/// real `TypeError` (the argument's type is wrong for this build).
pub fn unsupported_source(path: &str) -> JsValue {
    let e = js_sys::TypeError::new(&format!(
        "the browser build reads bytes, not paths ({path:?}); \
         pass a Uint8Array or ArrayBuffer, e.g. new Uint8Array(await file.arrayBuffer())"
    ));
    let _ = Reflect::set(
        &e,
        &JsValue::from_str("code"),
        &JsValue::from_str("ERR_UNSUPPORTED_SOURCE"),
    );
    e.into()
}
