// wasmvm-run is a standalone integration harness for zip-rs.
//
// It loads a HIP-0105 wasm extension directory from disk and runs a
// sequence of (fn, json) invocations against it via the production
// wasmvm host. Used by CI and by hand-verification.
//
// Build: this file depends on github.com/hanzoai/base. Copy it into
// the `base/cmd/zip-rs-integration/` tree (or any module that already
// imports `base/plugins/wasmvm`) and run with `go run`.
//
// Usage:
//   go run ./cmd/zip-rs-integration \
//     ~/work/hanzo/zip-rs/examples/validate-email \
//     validate '{"email":"a@b.com","age":25}'
package main

import (
	"context"
	"encoding/json"
	"fmt"
	"os"

	"github.com/hanzoai/base/plugins/wasmvm"
)

func main() {
	if len(os.Args) < 4 || (len(os.Args)-2)%2 != 0 {
		fmt.Fprintln(os.Stderr, "usage: wasmvm-run <extension-dir> <fn> <json> [<fn> <json>]...")
		os.Exit(64)
	}
	rt := wasmvm.NewRuntime()
	defer rt.Close()

	dir := os.Args[1]
	ctx := context.Background()
	mod, err := rt.Load(ctx, dir)
	if err != nil {
		fmt.Fprintf(os.Stderr, "load: %v\n", err)
		os.Exit(1)
	}
	defer mod.Close()

	for i := 2; i < len(os.Args); i += 2 {
		fn, payload := os.Args[i], os.Args[i+1]
		out, err := mod.Invoke(ctx, fn, []byte(payload))
		if err != nil {
			fmt.Fprintf(os.Stderr, "invoke %s: %v\n", fn, err)
			os.Exit(2)
		}
		var v any
		if jerr := json.Unmarshal(out, &v); jerr != nil {
			fmt.Fprintf(os.Stderr, "invalid JSON output: %s (%v)\n", out, jerr)
			os.Exit(3)
		}
		fmt.Printf("%s %s -> %s\n", fn, payload, out)
	}
}
