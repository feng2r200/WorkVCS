# Phase 4MI Shared Claim Dogfood Evidence

Date: 2026-09-01

## Scope

Phase 4MI dogfoods shared-Claim collaboration against a real external project
without changing Rust code or schema.

Target project:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul
```

Target file:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack/SYSTEM.md
```

## Dogfood Evidence

PASS:
`/tmp/workvcs-4mi-shared-claim-dogfood-rerun9-20260901T082157Z`.

Key observations from `summary.txt`:

```text
second_context_runnable_candidates=1
second_context_shared_claim=yes
shared_guard_reason=non_unique_shared_claim_set
shared_guard_allowed=false
active_claims=2
blocked_transition_contains_non_unique_shared_claim_set=yes
after_release_guard_reason=unique_shared_claimant
after_release_guard_allowed=true
verification_result=passed
verification_applicability=applicable
verification_reason_code=all_basis_applicable
ac_status_at_verification_head=verified
task_status_at_final_head=done
ac_status_after_task_done=stale
doctor_rc=0
target_status_unchanged=yes
target_hash_unchanged=yes
target_stat_unchanged=yes
```

Supplemental current doctor output in `doctor-current.out` returned exit code
0 and reported:

```text
checked_branches=1
checked_commits=10
checked_changesets=10
checked_change_operations=13
checked_events=21
invalid_checkpoints=0
```

The target file before/after checks are retained in:

```text
target-status-before.txt
target-status-after.txt
target-hash-before.txt
target-hash-after.txt
target-stat-before.txt
target-stat-after.txt
```

## What This Proves

- `claim task --mode shared` and `claim next --mode shared` can establish two
  active shared Claims over the same real-project Task.
- `context` makes the shared Claim set visible to the second Session through
  the runnable candidate.
- `claim guard --action structural-task` surfaces the current non-unique shared
  coordination state as `allowed=false` with
  `reason=non_unique_shared_claim_set`.
- A protected terminal Task transition is blocked while two shared Claims are
  active, then allowed after the second Claim is released.
- `verify` records Resource-backed evidence from the real target file with an
  applicable cache stamp.
- The external target file is not modified.

## Dogfood Findings

- The current `claim guard` action vocabulary is `terminal-task` and
  `structural-task`; guessed action names fail with process-level business
  errors.
- `workspace create`, `ac create`, `vr create`, `session focus-set`, and
  `doctor` should be driven from current `--help` output in dogfood scripts.
  Several superseded attempts failed on stale command spellings before the final
  run.
- Broad target-project status capture is noisy for real projects. The final run
  uses target-file-only status, hash, and stat snapshots.
- AC status is version-scoped: the criterion was `verified` at the verification
  commit and became `stale` after Task closeout advanced the Task version.

## Validation

No Rust code changed in this slice. Validation is limited to focused dogfood,
documentation checks, and independent review.

PASS: `/tmp/workvcs-4mi-doc-validation-clean-20260901T082728Z`.

Covered check:

```text
git diff --check
```

PASS: `/tmp/workvcs-4mi-fmt-validation-clean-20260901T082728Z`.

Covered check:

```text
cargo fmt --all -- --check
```

The full repository matrix was not repeated for this docs-only/dogfood-only
slice because recent full-matrix evidence was already green and this slice did
not change Rust code, schema, tests, or the smoke script.

## Independent Review

PASS: the independent review found no blocker/high/medium issues.

The review checked the six Phase 4MI claims against the scoped document diff,
the saved dogfood log directory, target-file before/after snapshots, and a
read-only query of `store.sqlite`. It confirmed that the docs do not describe
shared Claim as permission for simultaneous protected mutation.

## Remaining Open

- Broader write-mode or read/write shared-Claim collaboration remains Open.
- Automatic ownership arbitration between shared claimants remains outside V1.
- Release guidance should explain the AC verified-at-commit versus
  stale-after-closeout behavior.

## Delivery Closeout

- Documentation commit:
  `2ae04ec1d6c2afa9bb2b881f0aa4b4a51584bfd8`.
- Fast-forward merged to `main`.
- Worktree cleanup proof:
  `/tmp/workvcs-4mi-cleanup-20260901T083722Z`.
- Cleanup proof fields:
  `clean=yes`, `attached=yes`, `unlocked=yes`, `covered_by_main=yes`,
  `removed=yes`, `branch_retained=yes`.
