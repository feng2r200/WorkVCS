# ADR-0456: Phase 4MO JSON Error Output

Status: Accepted
Date: 2026-09-01

## Context

ADR-0441 added stable line-oriented key-value stderr fields for WorkVCS
business errors. ADR-0451 added the same script-readable shape for top-level
clap parse errors. ADR-0452 documented per-code recovery actions, but left JSON
error output Open.

The V1 readiness gap is not a new error taxonomy. Operators need an explicit
machine-readable encoding of the same stable fields so automation can parse
failures without line splitting, while existing scripts keep the default
key-value contract.

## Decision

Add a global CLI option:

```text
--error-format key-value|json
```

The default remains `key-value`. `json` changes only failure stderr rendering:

- WorkVCS business errors render one JSON object with `error_code`,
  `error_category`, `retryable`, and `message`.
- Top-level clap parse errors render one JSON object with `error_code`,
  `error_category`, `retryable`, `clap_error_kind`, and `message`.
- Help and version display remain clap display paths and do not render WorkVCS
  error fields.
- Successful command stdout is unchanged.
- JSON values reuse the existing `WorkVcsError` code/category/retryability
  authority and the CLI-local `clap_error_kind` labels.

Because a full `Cli` value is unavailable when parsing fails, the process entry
point pre-scans raw argv only for valid `--error-format json` and
`--error-format=json` tokens. Invalid or missing format values fall back to the
default key-value parse-error output.

## Non-Goals

- No new WorkVCS core error code, category, or retryability taxonomy.
- No schema, Store, Event, Evidence, or Recovery Guide data model change.
- No JSON success-output mode.
- No hidden command retry, automatic recovery, or semantic remediation.
- No change to existing key-value stderr fields or escaping.

## Evidence

- Implementation: `crates/workvcs-cli/src/main.rs` adds `--error-format`,
  JSON renderers, parse-failure format pre-scan, focused tests, and top-level
  help text coverage.
- Focused validation:
  `cargo fmt --check` and targeted `workvcs-cli` error-rendering tests passed.
- Process-level dogfood:
  `/tmp/workvcs-4mo-json-error-output-20260901T105030Z`.
- Final validation and independent review closeout are tracked in
  `docs/provenance/phase-4mo-json-error-output.md`.

## Consequences

Automation can opt into a stable JSON failure envelope while existing
line-oriented recovery scripts remain compatible by default. JSON output closes
the V1-local encoding gap for actionable errors, but it does not by itself
prove broader recovery maturity or release readiness.
