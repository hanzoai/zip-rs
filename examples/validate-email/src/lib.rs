//! Rust port of the canonical wazero-as fixture.
//!
//! Mirrors `~/work/hanzo/base/plugins/extbench/fixtures/wazero-as/src/assembly/index.ts`
//! semantics so the same wasmvm harness can compare AS vs Rust output
//! byte-for-byte.
//!
//! Contract:
//!   in:  {"email":"Foo@Example.COM ","age":25}
//!   ok:  {"ok":true,"email":"foo@example.com","age":25}
//!   err: {"ok":false,"error":"email required"}
//!   err: {"ok":false,"error":"age out of range"}
//!   err: {"ok":false,"error":"email shape"}
//!   err: {"ok":false,"error":"email domain"}

use serde::{Deserialize, Serialize};
use zip_rs::{handler, Result};

#[derive(Deserialize)]
struct ValidateRequest {
    #[serde(default)]
    email: String,
    #[serde(default)]
    age: i64,
}

#[derive(Serialize)]
struct ValidateResponse {
    ok: bool,
    email: String,
    age: i64,
}

#[handler]
fn validate(req: ValidateRequest) -> Result<ValidateResponse> {
    if req.email.is_empty() {
        return Err("email required".into());
    }
    if !(0..=150).contains(&req.age) {
        return Err("age out of range".into());
    }

    let norm: String = req.email.trim().to_ascii_lowercase();
    let at = match norm.find('@') {
        Some(i) if i > 0 && i + 1 < norm.len() => i,
        _ => return Err("email shape".into()),
    };
    let domain = &norm[at + 1..];
    if !domain.contains('.') {
        return Err("email domain".into());
    }

    Ok(ValidateResponse {
        ok: true,
        email: norm,
        age: req.age,
    })
}
