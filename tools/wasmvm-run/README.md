# wasmvm-run

Standalone integration harness for zip-rs handlers. Loads a HIP-0105
extension directory and invokes one or more functions against the
production wasmvm host.

The source depends on `github.com/hanzoai/base/plugins/wasmvm` so it
isn't a standalone Go program — copy it into a module that already
imports base (the most convenient is base itself):

```bash
cp tools/wasmvm-run/main.go ~/work/hanzo/base/cmd/zip-rs-integration/main.go
cd ~/work/hanzo/base
go run ./cmd/zip-rs-integration \
    ~/work/hanzo/zip-rs/examples/validate-email \
    validate '{"email":"Foo@Example.COM ","age":25}'
# validate {"email":"Foo@Example.COM ","age":25} -> {"ok":true,"email":"foo@example.com","age":25}
```

CI in `.github/workflows/ci.yml` automates this against a checked-out
base tree.
