//! # zip-rs
//!
//! Write zip-mounted wasm handlers in idiomatic Rust. zip-rs hides the
//! HIP-0105 wasm calling convention (`__base_alloc` / `__base_free` /
//! `fn(i32, i32) -> i64`) behind a `#[handler]` attribute so user code
//! reads as ordinary, typed Rust.
//!
//! ## Quickstart
//!
//! ```ignore
//! use zip_rs::{handler, Result};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Deserialize)]
//! struct Req { email: String, age: u32 }
//!
//! #[derive(Serialize)]
//! struct Res { ok: bool, email: String, age: u32 }
//!
//! #[handler]
//! fn validate(req: Req) -> Result<Res> {
//!     if !req.email.contains('@') {
//!         return Err("email shape".into());
//!     }
//!     Ok(Res { ok: true, email: req.email.trim().to_lowercase(), age: req.age })
//! }
//! ```
//!
//! Build with `cargo build --release --target wasm32-wasip1`. Drop the
//! resulting `.wasm` next to an `extension.json` manifest with
//! `"runtime": "wazero"` and the wasmvm runtime in Hanzo Base will pick
//! it up.
//!
//! ## Wire convention (informational)
//!
//! The host writes a JSON payload at `(ptr, len)` and calls
//! `<handler>(ptr, len)` which returns an `i64` packed as
//! `(result_ptr << 32) | result_len`. Both buffers are freed by the
//! host via `__base_free`. See HIP-0105 for the full specification.

#![no_std]

extern crate alloc;

pub mod alloc_abi;
pub mod envelope;
pub mod error;

pub use error::{Error, Result};
pub use zip_rs_macros::handler;

// Re-export serde types so user code only has to depend on zip-rs.
pub use serde;
pub use serde_json;

// Internal helpers invoked from macro-expanded code. Not part of the
// stable surface; the macro is the only intended caller.
#[doc(hidden)]
pub mod __private {
    pub use crate::alloc_abi::{leak_box, pack};
    pub use crate::envelope::{read_payload, read_payload_bytes, write_error, write_value};
}
