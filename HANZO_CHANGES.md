# HANZO_CHANGES

Tracks Hanzo-internal deltas not visible in `CHANGELOG.md` (audit
traceability, version pin justifications, brand-policy decisions, etc.).

## v0.1.0 (initial release)

- First public release of zip-rs.
- Workspace layout: `zip-rs` (lib) + `zip-rs-macros` (proc macros) +
  three end-to-end examples (`hello`, `validate-email`, `policy-eval`).
- Target ABI: HIP-0105 (in-process extension runtime, wazero backend).
- Cross-language compatibility verified against the AssemblyScript
  reference (`base/plugins/extbench/fixtures/wazero-as/`) — the
  `validate-email` example emits byte-equivalent responses.
- Wasm sizes: hello 87 KB, validate-email 91 KB, policy-eval 100 KB
  (release build, debug stripped via Cargo profile).
- Deps: serde 1.0.219, serde_json 1.0.140, syn 2.0.106, quote 1.0.40,
  proc-macro2 1.0.95. No major-version bumps planned.
