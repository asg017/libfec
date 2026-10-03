//! Typed arrays copied into V8-owned memory.
//!
//! napi-rs turns a `Vec` into an *external* `ArrayBuffer` whose Rust-side
//! free is a Node-API finalizer, and Node runs those finalizers from
//! `setImmediate`, not during GC. A synchronous `for…of` over a filing never
//! yields to the event loop, so every batch's arrays stayed allocated until
//! the loop ended: RSS grew with rows read while the V8 heap stayed flat.
//! Copying into a buffer V8 allocates itself (`napi_create_arraybuffer`)
//! needs no finalizer, so GC frees it like any other JS value.

use std::ptr;

use napi::bindgen_prelude::{ToNapiValue, TypeName, TypedArrayType, ValueType};
use napi::{check_status, sys};

/// Element types with a JS typed-array counterpart.
pub trait Element: Copy {
    const KIND: TypedArrayType;
}

impl Element for u8 {
    const KIND: TypedArrayType = TypedArrayType::Uint8;
}
impl Element for u32 {
    const KIND: TypedArrayType = TypedArrayType::Uint32;
}
impl Element for f64 {
    const KIND: TypedArrayType = TypedArrayType::Float64;
}

/// A `Vec<T>` that crosses to JS as a copied typed array (see the module docs).
/// Fields of this type need `#[napi(ts_type = "Uint32Array")]` (etc.).
pub struct Copied<T>(pub Vec<T>);

impl<T> From<Vec<T>> for Copied<T> {
    fn from(v: Vec<T>) -> Self {
        Copied(v)
    }
}

impl<T: Element> TypeName for Copied<T> {
    fn type_name() -> &'static str {
        "TypedArray"
    }

    fn value_type() -> ValueType {
        ValueType::Object
    }
}

impl<T: Element> ToNapiValue for Copied<T> {
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        let bytes = std::mem::size_of_val(val.0.as_slice());
        let mut buffer = ptr::null_mut();
        let mut data = ptr::null_mut();
        check_status!(
            unsafe { sys::napi_create_arraybuffer(env, bytes, &mut data, &mut buffer) },
            "Failed to create ArrayBuffer"
        )?;
        if bytes > 0 {
            // SAFETY: `data` is a fresh allocation of `bytes` bytes.
            unsafe { ptr::copy_nonoverlapping(val.0.as_ptr().cast::<u8>(), data.cast(), bytes) };
        }
        let mut array = ptr::null_mut();
        check_status!(
            unsafe {
                sys::napi_create_typedarray(env, T::KIND as i32, val.0.len(), buffer, 0, &mut array)
            },
            "Failed to create TypedArray"
        )?;
        Ok(array)
    }
}
