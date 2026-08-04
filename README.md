# zip-rs

Write zip-mounted wasm extensions in idiomatic Rust.

`zip-rs` hides the HIP-0105 wasm calling convention (`__base_alloc` /
`__base_free` / `fn(ptr, len) -> i64`) behind a `#[handler]` proc macro
so your code reads as ordinary, typed Rust:

```rust
use zip_rs::{handler, Result};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct ValidateRequest {
    email: String,
    age: u32,
}

#[derive(Serialize)]
struct ValidateResponse {
    ok: bool,
    email: String,
    age: u32,
}

#[handler]
fn validate(req: ValidateRequest) -> Result<ValidateResponse> {
    if !req.email.contains('@') {
        return Err("email shape".into());
    }
    Ok(ValidateResponse {
        ok: true,
        email: req.email.trim().to_lowercase(),
        age: req.age,
    })
}
```

The macro expands to a `#[no_mangle] extern "C" fn validate(i32, i32) ->
i64` wrapper that reads JSON from `(ptr, len)`, deserializes it, calls
your function, serializes the result, and packs `(ptr, len)` back into
an `i64`. The `__base_alloc` / `__base_free` allocator exports come for
free — they live in the `zip-rs` crate itself.

## Quickstart

```bash
cargo new --lib my-handler && cd my-handler
```

Edit `Cargo.toml`:

```toml
[package]
name = "my-handler"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
zip-rs = "0.1"
serde = { version = "1", features = ["derive"] }
```

Edit `src/lib.rs` with a `#[handler]` function (see example above), then
build:

```bash
rustup target add wasm32-wasip1   # one-time, if not already added
cargo build --release --target wasm32-wasip1
```

The resulting `target/wasm32-wasip1/release/my_handler.wasm` is a
drop-in HIP-0105 wasm module. Pair it with an `extension.json` and the
wasmvm runtime in Hanzo Base will load it:

```json
{
  "name": "my-handler",
  "version": "0.1.0",
  "runtime": "wazero",
  "module": "my_handler.wasm",
  "exports": ["validate"]
}
```

## Examples

Three end-to-end examples in `examples/`. Each one is a complete
extension directory — `cargo build` it, copy the `.wasm` next to the
`extension.json`, and the wasmvm host accepts it.

| Example | Purpose | Size |
|---------|---------|------|
| `hello` | minimal `name -> greeting` handler | 87 KB |
| `validate-email` | port of the canonical AS fixture (byte-equivalent output) | 91 KB |
| `policy-eval` | realistic IAM/gateway authz decision handler | 100 KB |

Build all three:

```bash
cargo build --release --target wasm32-wasip1
./examples/hello/build.sh
./examples/validate-email/build.sh
./examples/policy-eval/build.sh
```

Verify against the production wasmvm host (requires
`~/work/hanzo/base`):

```bash
cd ~/work/hanzo/base
go run ./cmd/zip-rs-integration ~/work/hanzo/zip-rs/examples/validate-email \
    validate '{"email":"Foo@Example.COM ","age":25}'
# -> validate {...} -> {"ok":true,"email":"foo@example.com","age":25}
```

(The integration harness source is `tools/wasmvm-run/main.go` in this
repo; copy it into the base tree to execute.)

## Wire convention (informational)

The host:

1. Calls `__base_alloc(payload_len)` to allocate guest memory.
2. Writes JSON payload to the returned pointer.
3. Calls `<handler>(ptr, len)` and receives an `i64` packed as
   `(result_ptr << 32) | result_len`.
4. Reads the result, then calls `__base_free` on both buffers.

zip-rs's job is to hide all of that. You write a typed function. See
[HIP-0105](https://github.com/hanzoai/hips/blob/main/HIPs/hip-0105-in-process-extension-runtime-standard.md)
for the full ABI specification.

## Constraints on `#[handler]`

| Rule | Reason |
|------|--------|
| Exactly one parameter | The wasm host calls with `(ptr, len)`; there is only one payload. |
| Return type `zip_rs::Result<T>` where `T: Serialize` | The wrapper serializes successes to JSON and errors to `{"ok":false,"error":"..."}`. |
| Not `async` | Wasm host calls are synchronous. |
| Not generic | The wasm export must be a concrete `extern "C"` function. |
| Not `unsafe` | Same. |
| One handler per function name in the crate | The `#[no_mangle]` export must be unique. |

You can have multiple handlers per crate — they share the
single-instance `__base_alloc` / `__base_free` allocator. Just list each
function name in your `extension.json` `exports` array.

## Sibling crates

`zip-rs` is one of a family of language SDKs targeting the HIP-0105
ABI:

- `zip-as` (AssemblyScript) — reference implementation, ~13 KB wasm
- `zip-py` (Python via pyodide, future) — for the `wazero-cpython-wasi` runtime
- `zip-zig` (Zig, future) — smallest possible wasm output

All four produce `wasm32-wasi[p1]` modules that any of the wasmvm /
pyvm host runtimes can load.

## License

Licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option — per HIP-0137.
