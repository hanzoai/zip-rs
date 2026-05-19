//! Smallest possible zip-rs handler. Reads `{"name":"..."}` and returns
//! `{"greeting":"hello, <name>!"}`.

use serde::{Deserialize, Serialize};
use zip_rs::{handler, Result};

#[derive(Deserialize)]
struct HelloRequest {
    name: String,
}

#[derive(Serialize)]
struct HelloResponse {
    greeting: String,
}

#[handler]
fn hello(req: HelloRequest) -> Result<HelloResponse> {
    if req.name.is_empty() {
        return Err("name required".into());
    }
    Ok(HelloResponse {
        greeting: format!("hello, {}!", req.name),
    })
}
