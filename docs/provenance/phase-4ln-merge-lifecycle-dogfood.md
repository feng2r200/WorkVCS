# Phase 4LN Merge Lifecycle Dogfood Evidence

Status: current local evidence
Date: 2026-09-01

## Scope

Phase 4LN closes the V1 readiness ledger's merge lifecycle dogfood gap by using
the implemented CLI against a durable local Store. The run proves divergent
Work Branch merge classification, explicit operator resolution, unresolved-item
guard behavior, target-head and source-head moved guards, abort recovery, restart
recovery, and a completed two-parent merge commit.

This evidence does not claim semantic/LLM merge, custom resolution application,
GUI/TUI support, another-project use, release readiness, or broader scale
maturity.

## Dogfood Setup

Commands were run from:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4ln-merge-lifecycle-dogfood
```

The CLI was built locally with:

```text
cargo build -q -p workvcs-cli
```

The ignored local Store and log used for the dogfood run were:

```text
.work-governance/runtime/dogfood/phase-4ln-20260831T220754Z/workvcs.sqlite
.work-governance/runtime/logs/phase-4ln-merge-lifecycle-dogfood-20260831T220754Z.log
```

## Divergent Merge And Target-Head Recovery

The first workspace created a shared Task at the merge base, then forked a
source Branch. The target Branch transitioned the shared Task to `in_progress`;
the source Branch transitioned the same base Task to `blocked` and added a
source-only Task.

Captured IDs:

```text
workspace_id=01a059dd-5be0-7fb3-b466-d88fcf34faf0
target_branch_id=01a059dd-5be0-7fb3-b466-d8b51a611a8f
source_branch_id=01a059dd-5c13-7940-949f-01650c45b75a
first_merge_id=01a059dd-5c58-7013-a41c-1c56a946beb3
restart_merge_id=01a059dd-5d03-7a50-bfd7-426dc553e6da
result_commit_id=01a059dd-5d6e-7472-b1a2-f5432e60436f
```

`merge show` reported two items:

```text
items=2
merge1_conflict_item_id=01a059dd-5c58-7013-a41c-1c364c908e71
merge1_auto_item_id=01a059dd-5c58-7013-a41c-1c4fba079701
```

The unresolved guard blocked freezing until every item had an explicit
resolution:

```text
workspace invalid: merge 01a059dd-5c58-7013-a41c-1c56a946beb3 has 2 unresolved item(s)
unresolved_guard_failure_seen=true
```

After resolving and freezing both items, the target Branch advanced before
`merge continue`. Continue rejected the stale merge input:

```text
branch head conflict: merge target branch 01a059dd-5be0-7fb3-b466-d8b51a611a8f moved from 01a059dd-5c24-7961-a901-2cbbaff5da54 to 01a059dd-5cc3-7a90-8cbb-8393a6d44ff3
moved_head_guard_failure_seen=true
```

Operator recovery was explicit: abort the stale attempt, start a new attempt
against the moved target head, resolve/freeze the new items, and continue.
The completed commit was a merge commit:

```text
runtime_state=aborted
runtime_state=completed
head_operation_type=merge.continue
commit=01a059dd-5d6e-7472-b1a2-f5432e60436f operation=merge.continue
```

The final completed merge used the moved target head as primary parent and the
source head as secondary parent.

## Source-Head Recovery Supplement

A second workspace in the same Store proved the symmetric source-head moved
guard. The source Branch advanced after freeze and before continue.

Captured IDs:

```text
source_moved_workspace_id=01a059df-83a2-7e90-b668-0fcd72002303
source_moved_first_merge_id=01a059df-83f8-75a3-a961-b535b8700a88
source_moved_restart_merge_id=01a059df-846b-7010-b6b4-4f70a51facee
source_moved_result_commit_id=01a059df-84c0-7ed3-adb8-e3c718e9c8f5
```

Continue rejected the stale source input:

```text
branch head conflict: merge source branch 01a059df-83d6-70f2-b0db-8536062b7f16 moved from 01a059df-83e6-76c0-be20-1250a8320a8c to 01a059df-843a-7551-8a1d-e17156641b37
source_moved_head_guard_failure_seen=true
```

The same recovery pattern worked: abort the stale attempt, start a new attempt
from the current heads, resolve all items, freeze, and continue.

```text
source_moved_status=PASS
```

## Validation

Both workspaces ended with required-valid checks:

```text
workvcs store integrity STORE --require-valid
workvcs doctor STORE --require-valid
valid_required=true
```

## Findings

- The existing merge lifecycle is sufficient for real local operator recovery:
  unresolved items block freeze, moved heads block continue, abort preserves
  provenance, and restart from current heads completes.
- `AUTO` merge items still require an explicit resolution before freeze. This is
  conservative but should be documented for operators.
- The CLI error text was actionable enough for recovery, but failure output is
  not yet emitted as stable key-value `error_code` fields. That remains part of
  the broader actionable-error and CLI discoverability backlog.
- Merge lifecycle dogfood is now local-proven, but not yet another-project or
  larger-Store proven.
