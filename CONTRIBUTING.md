# Contributing to WorkVCS

Thank you for taking an interest in WorkVCS.

## Contribution license

WorkVCS is licensed under Apache-2.0. Unless you explicitly state otherwise,
any contribution intentionally submitted for inclusion in WorkVCS is provided
under the same license, as described by section 5 of [LICENSE](LICENSE). Only
submit work that you have the right to contribute.

## Good issue reports

Before opening an issue:

1. Check the existing issues and the [support policy](SUPPORT.md).
2. Reproduce the behavior against the latest revision you are permitted to use.
3. Include the WorkVCS version or commit, operating system, Rust version, exact
   command, expected behavior, actual behavior, and a minimal reproduction.
4. Remove credentials, private paths, proprietary project content, and Store
   data that you are not authorized to disclose.

Security vulnerabilities must not be filed as public issues; follow
[SECURITY.md](SECURITY.md).

## Design discipline

WorkVCS treats its domain and architecture documents as product authority.
Changes to entities, relations, lifecycle, persistence, configuration, or
public behavior must keep the relevant product, domain, architecture, and
operator documentation consistent. Material architecture changes require an
accepted ADR before they become confirmed behavior.

## Local verification

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
scripts/validate-schema-v0.1.sh
scripts/smoke-v0.1-cli-workflow.sh
```

Keep changes focused. Do not commit generated build output, real WorkVCS
stores, credentials, private project records, or local process artifacts.
