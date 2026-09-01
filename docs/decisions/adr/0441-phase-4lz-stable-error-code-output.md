# ADR-0441: Phase 4LZ Stable Error Code Output

Status: Accepted
Date: 2026-09-01

## Context

The V1 readiness ledger still named stable key-value `error_code` output as an
Open recovery gap. Core already had structured error authority through
`WorkVcsError::code()`, `category()`, and `retryable()`, but the CLI process
entrypoint only printed the human display message on failure.

This made real operator recovery harder than necessary. Scripts could inspect
some successful command fields, such as Claim guard recovery hints, but could
not reliably distinguish a retryable branch-head conflict from a query or task
input error without parsing prose.

## Decision

Render every `WorkVcsError` returned by the CLI `run()` path as stable key-value
stderr:

```text
error_code=<lower_snake_case_code>
error_category=<lower_snake_case_category>
retryable=<true|false>
message=<escaped_display_message>
```

The code and category strings are implemented in `workvcs-core` as explicit
lower-snake-case `as_str()` values and `Display` output. The CLI does not invent
a separate taxonomy; it uses existing core error authority.

The `message` value is a single key-value line. Backslash, newline, carriage
return, and tab are escaped so a line-oriented script can parse the output
without losing the original human message.

## Non-Goals

- No Store schema change.
- No new error variants or recovery taxonomy.
- No successful command output change.
- No JSON error output.
- No full per-code recovery catalog.
- No normalization of every top-level clap syntax error. This slice covers
  WorkVCS business errors returned through `run()`.
- No shell command execution wrapper.

## Evidence

Focused regression coverage:

```text
cargo test -q -p workvcs-core error_codes_render_stable_lower_snake_case
cargo test -q -p workvcs-cli cli_renders_
```

Real CLI failure dogfood:

```text
phase4lz_dogfood_result=PASS
log_dir=/tmp/workvcs-4lz-error-output-dogfood-20260901T033213Z
exit_status=1
error_code_field=PASS
error_category_field=PASS
retryable_field=PASS
message_field=PASS
```

The dogfood initialized a temporary local Store, generated valid IDs, ran
`workvcs verify` with `--resource-content-from-scope-path` but without
`--scope-path`, and confirmed stderr contained:

```text
error_code=task_invalid
error_category=task
retryable=false
message=task invalid: --scope-path is required
```

## Consequences

Operators and future Agents can now branch recovery scripts on stable error
metadata for WorkVCS business failures instead of scraping prose.

The remaining recovery gap is narrower: per-code recovery guidance and
top-level syntax-error normalization remain useful, but V1 no longer lacks a
stable process-level business error signal.
