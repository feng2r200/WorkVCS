# Phase 4MO JSON Error Output Evidence

Date: 2026-09-01

## Scope

Phase 4MO adds explicit JSON stderr rendering for existing WorkVCS failure
metadata:

```bash
workvcs --error-format json <command>
workvcs <command> --error-format json
```

The default remains line-oriented key-value stderr. JSON mode changes only
failure output. It does not add success-output JSON, new recovery behavior,
schema changes, Store changes, background remediation, or semantic extraction.

## Implementation Evidence

- `crates/workvcs-cli/src/main.rs` adds global
  `--error-format key-value|json`.
- WorkVCS business errors can render one JSON object with `error_code`,
  `error_category`, `retryable`, and `message`.
- Top-level clap parse errors can render one JSON object with `error_code`,
  `error_category`, `retryable`, `clap_error_kind`, and `message`.
- The parse-error path pre-scans raw argv for valid JSON format tokens because
  no parsed `Cli` exists on clap failure.
- Help/version display continues to use clap display handling.
- Default key-value rendering remains the default and keeps the previous field
  names.

## Focused Validation

PASS: focused validation before documentation closeout.

Covered checks:

```text
cargo fmt --check
cargo test -q -p workvcs-cli cli_renders_workvcs_error_json
cargo test -q -p workvcs-cli cli_renders_clap_parse_error_json
cargo test -q -p workvcs-cli cli_detects_json_error_format_before_parse_completion
cargo test -q -p workvcs-cli cli_renders_stable_workvcs_error_fields
cargo test -q -p workvcs-cli cli_renders_stable_clap_parse_error_fields
cargo test -q -p workvcs-cli cli_renders_retryable_workvcs_error_fields
```

The new focused tests cover:

```text
business-error JSON object parsing
clap-parse JSON object parsing
boolean retryable JSON value
single-object stderr shape
raw argv format detection for parse failures
invalid/missing format fallback to key-value
unchanged default business-error key-value fields
unchanged default clap-parse key-value fields
top-level help text includes --error-format
```

An additional real success-output probe ran `canonical digest-domains` with no
error-format option, with `--error-format json` before the command, between the
command family and subcommand, and after the subcommand. All four runs returned
the same stdout:

```text
domains=2
domain.0=entity-version
domain.1=relation-version
```

## Dogfood Evidence

PASS: `/tmp/workvcs-4mo-json-error-output-20260901T105030Z`.

The process-level run executed one WorkVCS business failure, one top-level clap
parse failure, and one default-format business failure. The two JSON stderr
files were parsed with `jq`; the default stderr retained key-value fields and
was rejected by `jq` as non-JSON.

Key observations:

```text
business_status=1
business_json.error_code=store_compatibility_unsupported
business_json.error_category=store
business_json.retryable=false
clap_status=2
clap_json.error_code=cli_parse_error
clap_json.error_category=usage
clap_json.retryable=false
clap_json.clap_error_kind=invalid_subcommand
default_status=1
default_key_value_error_code=store_compatibility_unsupported
default_json=not_json
```

## Remaining Open

- JSON error output does not prove broader recovery maturity.
- Error recovery still requires operator or script choice based on
  `error_code`, `error_category`, and `retryable`.
- Release readiness still requires a final V1 release gate matrix with current
  evidence across the remaining ledger gaps.

## Independent Review

PASS: independent review found no blocker/high/medium issues for the scoped
CLI JSON error output implementation, tests, documentation, and evidence
summary.

Residual risk noted by review was that success stdout had not yet been covered
by a separate process-level sample. The additional success-output probe above
addresses that gap for one stable no-Store command.

## Final Validation

PASS: `/tmp/workvcs-4mo-final-validation-20260901T105801Z`.

Covered checks:

```text
git diff --check
cargo fmt --all -- --check
scripts/validate-schema-v0.1.sh
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
scripts/smoke-v0.1-cli-workflow.sh
```
