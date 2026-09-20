## What changed

<!-- Explain the user-visible outcome and the reason for the change. -->

## Authority and compatibility

- [ ] Product, domain, architecture, schema, and operator docs remain consistent.
- [ ] Any material architecture decision has an accepted ADR.
- [ ] Compatibility and migration effects are described.

## Verification

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo test --workspace --all-targets`
- [ ] `scripts/validate-schema-v0.1.sh`
- [ ] `scripts/smoke-v0.1-cli-workflow.sh`

## Safety

- [ ] No credentials, private paths, proprietary content, Store data, or local
      process artifacts are included.
- [ ] The change is focused and unrelated work is preserved.
