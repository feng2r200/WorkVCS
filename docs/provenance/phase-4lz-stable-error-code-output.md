# Phase 4LZ Stable Error Code Output Evidence

Date: 2026-09-01

## Scope

Phase 4LZ addresses the V1 readiness ledger row for actionable errors and
recovery by adding stable key-value stderr fields for WorkVCS business errors.
It does not add a recovery catalog, a JSON output mode, a shell execution
wrapper, or normalization for every top-level clap syntax error.

## Implementation Evidence

Implemented files:

```text
crates/workvcs-core/src/error.rs
crates/workvcs-cli/src/main.rs
```

The core error module now exposes explicit stable lower-snake-case strings for
`ErrorCode` and `ErrorCategory`. The CLI process entrypoint renders
`WorkVcsError` failures as:

```text
error_code=<code>
error_category=<category>
retryable=<true|false>
message=<escaped_message>
```

## Focused Tests

Focused tests passed:

```text
cargo test -q -p workvcs-core error_codes_render_stable_lower_snake_case
cargo test -q -p workvcs-cli cli_renders_
```

These tests prove stable code/category strings, retryability rendering, and
single-line escaping for newline-containing error messages.

## Dogfood Evidence

The dogfood used a real CLI process, not only function-level tests:

```text
phase4lz_dogfood_result=PASS
log_dir=/tmp/workvcs-4lz-error-output-dogfood-20260901T033213Z
exit_status=1
error_code_field=PASS
error_category_field=PASS
retryable_field=PASS
message_field=PASS
```

The failing command initialized a temporary Store and then invoked `verify` with
valid generated IDs and `--resource-content-from-scope-path` but without the
required `--scope-path`. The expected stderr fields were present:

```text
error_code=task_invalid
error_category=task
retryable=false
message=task invalid: --scope-path is required
```

## Independent Review

Independent review reported no blocker/high/medium findings. The review checked
the scoped diff and confirmed:

```text
workvcs --help exits 0, prints Usage on stdout, and leaves stderr empty
top-level clap unknown subcommand exits 2 without error_code=
```

This matches the documented boundary: Phase 4LZ covers WorkVCS business errors
returned through `run()`, not every clap parser failure.

## Validation Note

An initial final-validation command used the wrong smoke script path
(`scripts/smoke-cli-workflow.sh`) and failed before exercising the product. The
correct smoke command passed:

```text
cli_smoke_corrected=PASS
log_dir=/tmp/workvcs-4lz-smoke-corrected-20260901T033927Z
```

The corrected final validation matrix passed:

```text
git_diff_check=PASS
cargo_fmt=PASS
schema=PASS
cargo_clippy=PASS
cargo_test=PASS
cli_smoke=PASS
summary=/tmp/workvcs-4lz-final-validation-corrected-20260901T034234Z/summary.log
```

## Boundary

The result is intentionally small but operator-facing. It closes the broad
"no stable business error code on process failure" gap without pretending that
every recovery path is documented or that clap usage failures have been
normalized.
