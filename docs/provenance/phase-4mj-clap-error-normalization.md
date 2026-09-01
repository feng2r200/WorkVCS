# Phase 4MJ Clap Error Normalization Evidence

Date: 2026-09-01

## Scope

Phase 4MJ closes the top-level clap syntax-error normalization gap named by the
V1 readiness ledger and observed during Phase 4MI. It changes only the CLI
process entrypoint and tests.

## Implementation Evidence

Implemented file:

```text
crates/workvcs-cli/src/main.rs
```

Key behavior:

```text
error_code=cli_parse_error
error_category=usage
retryable=false
clap_error_kind=<lower_snake_case_kind>
message=<escaped_clap_message>
```

`WorkVcsError` rendering remains unchanged. Help and version display remain on
the clap display path.

## Focused Validation

PASS: `/tmp/workvcs-4mj-focused-validation-20260901T084328Z`.

Covered check:

```text
cargo fmt --all -- --check
```

PASS: `/tmp/workvcs-4mj-focused-validation-tests-rerun-20260901T084348Z`.

Covered checks:

```text
cargo test -q -p workvcs-cli cli_renders_stable_clap_parse_error_fields
cargo test -q -p workvcs-cli cli_keeps_help_and_version_as_clap_display
```

A superseded focused-test attempt used two Cargo filters in one command and is
retained only as an ineffective-command example:
`/tmp/workvcs-4mj-focused-validation-test-20260901T084328Z`.

## Dogfood Evidence

PASS: `/tmp/workvcs-4mj-clap-error-dogfood-rerun-20260901T084552Z`.

Key observations from `summary.txt`:

```text
unknown_arg_rc=2
unknown_arg_error_code=cli_parse_error
unknown_arg_error_category=usage
unknown_arg_retryable=false
unknown_arg_clap_error_kind=unknown_argument
invalid_subcommand_rc=2
invalid_subcommand_error_code=cli_parse_error
invalid_subcommand_error_category=usage
invalid_subcommand_retryable=false
invalid_subcommand_clap_error_kind=invalid_subcommand
help_rc=0
help_stdout_usage=yes
help_stderr_empty=yes
help_no_error_code=yes
```

The process dogfood used the same failure classes surfaced in Phase 4MI:

- stale argument: `workspace show ... --expected-head ...`;
- removed subcommand: `goal contain`; and
- normal help display: `workvcs --help`.

## Remaining Open

- Per-code recovery guidance remains Open.
- JSON error output remains outside the current V1 slice.
- Nested parser helpers still use their existing `query_invalid` business-error
  path.

## Independent Review

PASS: the independent review found no blocker/high/medium issues.

The review checked `crates/workvcs-cli/src/main.rs`, the scoped 4MJ docs, the
focused test logs, process dogfood logs, and validation logs. It confirmed that
the docs preserve the top-level-only scope and do not claim JSON output,
per-code recovery, command spelling redesign, or nested-parser unification.

## Final Validation

PASS: `/tmp/workvcs-4mj-final-validation-20260901T085444Z`.

Covered checks:

```text
git diff --check
cargo fmt --all -- --check
scripts/validate-schema-v0.1.sh
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
scripts/smoke-v0.1-cli-workflow.sh
```

## Delivery Closeout

- Implementation commit:
  `122e01804211dc0510ddb5414717d06d5128fb5e`.
- Fast-forward merged to `main`.
- Worktree cleanup proof:
  `/tmp/workvcs-4mj-cleanup-20260901T090157Z`.
- Cleanup proof fields:
  `clean=yes`, `attached=yes`, `unlocked=yes`, `covered_by_main=yes`,
  `removed=yes`, `branch_retained=yes`.
