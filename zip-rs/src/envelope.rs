//! JSON envelope helpers used by the macro-generated handler stubs.
//!
//! Every handler reads a JSON payload from `(ptr, len)`, deserializes it
//! into the user's typed request, calls the inner function, and
//! serializes the result (or error) back. These helpers centralize the
//! serialization shape so it stays consistent across all examples and
//! across the AS / Rust / future-Zig fixtures.
//!
//! Success: the serialized form of the user's `Serialize` return type
//! is emitted verbatim, so users control the schema.
//!
//! Error: `{"ok":false,"error":"<message>"}` — matches the AS fixture
//! contract so the same wasmvm host can talk to either guest.

use alloc::string::String;
use alloc::vec::Vec;
use serde::{de::DeserializeOwned, Serialize};

use crate::error::{Error, Result};

/// Deserialize a JSON byte slice into `T`. Safe wrapper around the
/// raw HIP-0105 entry below; exposed for host-side tests.
pub fn read_payload_bytes<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    if bytes.is_empty() {
        return Err(Error::new("empty payload"));
    }
    serde_json::from_slice::<T>(bytes).map_err(Error::from)
}

/// Read `len` bytes at `ptr` from guest linear memory and deserialize
/// as `T` via serde_json.
///
/// # Safety
///
/// The caller must guarantee `(ptr, len)` refers to a valid byte range
/// in this module's linear memory. The host wasmvm always honors that
/// on wasm32; this is unsound to call on a 64-bit host.
pub unsafe fn read_payload<T: DeserializeOwned>(ptr: i32, len: i32) -> Result<T> {
    if ptr == 0 || len <= 0 {
        return Err(Error::new("empty payload"));
    }
    // SAFETY: caller contract — the host wrote `len` bytes to `ptr`.
    let slice = core::slice::from_raw_parts(ptr as *const u8, len as usize);
    read_payload_bytes(slice)
}

/// Serialize a successful response to JSON bytes.
pub fn write_value<T: Serialize>(value: &T) -> Vec<u8> {
    match serde_json::to_vec(value) {
        Ok(bytes) => bytes,
        Err(e) => write_error(&Error::from(e)),
    }
}

/// Serialize an error response to JSON bytes in the canonical
/// `{"ok":false,"error":"<msg>"}` shape — key order matches the
/// AssemblyScript reference fixture so wasmvm consumers see byte-for-byte
/// equivalent output regardless of guest language.
pub fn write_error(err: &Error) -> Vec<u8> {
    // We hand-build the wrapper so `"ok"` precedes `"error"` (the AS
    // reference order); serde_json's serializer would sort
    // alphabetically when serializing from a json! Value. The message
    // itself goes through `serde_json::to_string` so escape rules are
    // RFC-correct.
    let escaped = match serde_json::to_string(err.message()) {
        Ok(s) => s,
        Err(_) => String::from(r#""<unencodable>""#),
    };
    let mut buf = String::with_capacity(escaped.len() + 24);
    buf.push_str(r#"{"ok":false,"error":"#);
    buf.push_str(&escaped);
    buf.push('}');
    buf.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize, Debug, PartialEq)]
    struct Req {
        x: u32,
    }

    #[derive(Serialize)]
    struct Resp {
        ok: bool,
        x2: u32,
    }

    // Tests use the safe `read_payload_bytes` entry so they're sound
    // on the 64-bit host where `cargo test` actually runs. The raw
    // pointer-based `read_payload` is exercised end-to-end by the
    // wasmvm host integration tests (see examples/ wasm fixtures).

    #[test]
    fn read_payload_decodes_json() {
        let req: Req = read_payload_bytes(br#"{"x":7}"#).unwrap();
        assert_eq!(req, Req { x: 7 });
    }

    #[test]
    fn read_payload_rejects_empty_slice() {
        let r: Result<Req> = read_payload_bytes(b"");
        assert!(r.is_err());
    }

    #[test]
    fn read_payload_rejects_empty_ptr() {
        // SAFETY: ptr=0 short-circuits before any deref.
        let r: Result<Req> = unsafe { read_payload(0, 0) };
        assert!(r.is_err());
    }

    #[test]
    fn read_payload_rejects_garbage() {
        let r: Result<Req> = read_payload_bytes(b"not json");
        assert!(r.is_err());
    }

    #[test]
    fn write_value_emits_compact_json() {
        let bytes = write_value(&Resp { ok: true, x2: 14 });
        assert_eq!(
            core::str::from_utf8(&bytes).unwrap(),
            r#"{"ok":true,"x2":14}"#
        );
    }

    #[test]
    fn write_error_matches_as_fixture_shape() {
        let bytes = write_error(&Error::new("email shape"));
        assert_eq!(
            core::str::from_utf8(&bytes).unwrap(),
            r#"{"ok":false,"error":"email shape"}"#
        );
    }

    #[test]
    fn write_error_escapes_quotes() {
        let bytes = write_error(&Error::new(r#"bad "value""#));
        // serde_json takes care of quote-escaping for us.
        let s = core::str::from_utf8(&bytes).unwrap();
        assert!(s.contains(r#"\"value\""#));
    }
}
