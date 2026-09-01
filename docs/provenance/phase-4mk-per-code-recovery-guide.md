# Phase 4MK Per-Code Recovery Guide Evidence

Date: 2026-09-01

## Scope

Phase 4MK closes the per-code recovery guidance gap named by the V1 readiness
ledger. It is a docs-only slice that builds on the stable process error fields
from Phase 4LZ and the top-level clap syntax-error fields from Phase 4MJ.

## Implementation Evidence

Added file:

```text
docs/operator/error-recovery-guide.md
```

Updated files:

```text
docs/operator/quickstart-and-recovery.md
docs/provenance/v1-readiness-ledger.md
docs/README.md
docs/decisions/adr/0452-phase-4mk-per-code-recovery-guide.md
```

The guide covers every current `ErrorCode::as_str()` value from
`crates/workvcs-core/src/error.rs` and the CLI-local `cli_parse_error` from
Phase 4MJ. It preserves the current core retryability contract: only
`branch_head_conflict` is directly retryable.

## Validation

PASS: `/tmp/workvcs-4mk-doc-validation-20260901T091240Z`.

PASS after addressing the independent review finding:
`/tmp/workvcs-4mk-post-review-validation-20260901T091524Z`.

Covered checks:

```text
error-code coverage check
git diff --cached --check
cargo fmt --all -- --check
```

Key coverage result:

```text
core_error_codes=41
missing_in_guide=0
```

## Independent Review

PASS after one medium finding was addressed.

The independent review found no blocker/high issues. It found one medium
evidence-consistency issue: ADR-0452 referred to validation and review evidence
being recorded while this provenance file still had Pending placeholders. This
file now records the validation log and the review result.

## Remaining Open

- JSON error output remains outside this slice.
- Automatic repair and recovery execution remain outside this slice.

## Delivery Closeout

- Implementation/docs commit:
  `fb340b45198b765a474e9ac49d20b673010a96f0`.
- Fast-forward merged to `main`.
- Worktree cleanup proof:
  `/tmp/workvcs-4mk-cleanup-20260901T091653Z`.
- Cleanup proof fields:
  `clean=yes`, `attached=yes`, `unlocked=yes`, `covered_by_main=yes`,
  `removed=yes`, `branch_retained=yes`.
