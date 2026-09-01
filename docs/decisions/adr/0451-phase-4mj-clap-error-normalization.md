# ADR-0451: Phase 4MJ Clap Error Normalization

Status: Accepted
Date: 2026-09-01

## Context

ADR-0441 made `WorkVcsError` process failures script-readable, but deliberately
left top-level clap syntax errors outside that slice. Phase 4MI then exposed the
remaining cost: stale command spellings such as old arguments or removed
subcommands failed before `run()` and emitted raw clap prose, so dogfood scripts
could not branch on stable fields.

## Decision

The CLI process entrypoint now uses `Cli::try_parse()` instead of
`Cli::parse()`. For non-display top-level clap errors, stderr is rendered as
stable key-value fields:

```text
error_code=cli_parse_error
error_category=usage
retryable=false
clap_error_kind=<lower_snake_case_kind>
message=<escaped_clap_message>
```

`message` uses the existing key-value escaping helper, so multi-line clap
output remains parseable as one line. Help and version display errors keep clap
display behavior and exit codes; `workvcs --help` still exits 0, writes usage
to stdout, leaves stderr empty, and does not emit `error_code`.

## Non-Goals

- No core `WorkVcsError` taxonomy change.
- No JSON error output.
- No per-code recovery guide.
- No command spelling redesign.
- No normalization redesign for nested parser helpers beyond the current
  `WorkVcsError::QueryInvalid` path they already use.
- No successful command output change.

## Evidence

- Focused formatting:
  `/tmp/workvcs-4mj-focused-validation-20260901T084328Z`.
- Focused tests:
  `/tmp/workvcs-4mj-focused-validation-tests-rerun-20260901T084348Z`.
- Process dogfood:
  `/tmp/workvcs-4mj-clap-error-dogfood-rerun-20260901T084552Z`.
- Independent review found no blocker/high/medium issues.
- Final validation matrix:
  `/tmp/workvcs-4mj-final-validation-20260901T085444Z`.

## Consequences

Operator scripts can now treat both WorkVCS business errors and top-level CLI
usage failures as stable key-value stderr. This reduces the recovery and
debugging cost surfaced by Phase 4MI without broadening V1 into a JSON error
protocol or a complete recovery catalog.
