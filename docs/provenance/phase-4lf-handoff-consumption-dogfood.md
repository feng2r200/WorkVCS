# Phase 4LF Handoff Consumption Dogfood Evidence

Status: current local evidence
Date: 2026-09-01

## Scope

Phase 4LF corrects the Phase 4K risk of over-focusing on narrow smoke
expectations by using WorkVCS itself to prove a continuation loop:

1. create a real source implementation Task and source Session;
2. close the source Session and author a focused Handoff;
3. start a second continuation Session;
4. consume the Handoff focus into that Session;
5. inspect Context, `next`, and `why`; and
6. claim the continuation Task.

This evidence does not claim release readiness, automatic Agent orchestration,
another-project use, or complete `why` explanation coverage.

## Dogfood Setup

Commands were run from:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lf-handoff-consumption-dogfood
```

The CLI was built locally with:

```text
cargo build -q -p workvcs-cli
```

The ignored local Store and log used for the dogfood run were:

```text
.work-governance/runtime/dogfood/phase-4lf-20260901025912.sqlite
.work-governance/runtime/logs/phase-4lf-dogfood-20260901025912.log
```

Captured IDs:

```text
workspace_id=01a05930-9a84-7d01-bea9-cb3ba10bd43f
branch_id=01a05930-9a84-7d01-bea9-cb642011dd30
source_task_id=01a05930-9a92-76c2-b381-782c1c88953c
source_session_id=01a05930-9aa8-72b2-8262-770fdaeffc95
source_claim_id=01a05930-9ac0-7830-a2e7-d8d7eb8665f7
source_done_version_id=01a05930-9aed-7a90-9ec9-6d54aed27090
session_diff_id=01a05930-9afe-7423-bd1a-9478889f5def
handoff_record_id=01a05930-9b18-72a3-b553-efc6daec43fc
handoff_commit_id=01a05930-9b18-72a3-b553-ef9c136165e8
continuation_task_id=01a05930-9ad9-7353-8460-81db49485b4a
continuation_task_version_id=01a05930-9ad9-7353-8460-81c12bffb2c1
```

## Pre-Implementation Consumption

Before adding `handoff consume`, the continuation loop succeeded by manually
copying `focus_entity_id` from:

```text
workvcs handoff show STORE --commit HANDOFF_COMMIT --handoff HANDOFF_RECORD
```

into:

```text
workvcs session focus-set STORE --session CONTINUATION_SESSION --focus FOCUS_ENTITY
```

Observed continuation result:

```text
dogfood_result=passed
continuation_session_id=01a05930-9b43-7053-972f-73d78140fa53
context_focus_entity_id=01a05930-9ad9-7353-8460-81db49485b4a
context_items=6
context_available_items=6
why_handoff_edges=0
why_focus_edges=0
continuation_claim_id=01a05931-39a9-7790-a3da-7697e30c0e49
finding_record_id=01a05931-39c3-71c1-8d55-5ea09712549f
finding_commit_id=01a05931-39c3-71c1-8d55-5e7637af7298
continuation_session_diff_id=01a05931-39d5-7953-add2-91fa5f44c82d
```

Context was sufficient for continuation: it included the continuation Task as
`current_task`, the Task readiness packet, and the Handoff record as
`relevant_handoff`.

The concrete friction was manual key-value capture between `handoff show` and
`session focus-set`.

## Implementation Response

Phase 4LF adds:

```text
workvcs handoff consume STORE --commit COMMIT --handoff HANDOFF --session SESSION
```

The command reads the focused Handoff, validates its recognized scope and
SessionDiff, then applies the Handoff focus to the target active Session through
the existing Session focus API.

After rebuilding the CLI, the same Store proved the new command and its
unambiguous source/continuation Session expectation fields:

```text
handoff_consume_precheck_result=passed
consume_session_id=01a0593a-98a3-7081-bfe9-ffcd834050cf
bad_precheck_status=1
bad_focus_entity_id=none
bad_focus_matches_expected=true
source_session_matches_expected=true
consume_session_matches_expected=true
consume_focus_entity_id=01a05930-9ad9-7353-8460-81db49485b4a
context_focus_entity_id=01a05930-9ad9-7353-8460-81db49485b4a
claim_id=01a0593a-9922-7d52-9d40-e8288e0ecada
consume_session_diff_id=01a0593a-993c-7761-ae8a-57d5a19df4ba
```

An independent read-only review found one high issue in the first
implementation: source-side expectations were checked after the focus mutation.
The fix moved source Session, SessionDiff, and focus expectations before
`session focus-set`. The focused handoff CLI regression now proves a mismatched
source expectation returns an error while the continuation Session remains
`focus_entity_id=none`.

## Findings

- Continuation from a focused Handoff is now dogfood-proven.
- `handoff consume` removes the immediate manual focus-copy step but still
  requires explicit Session creation. That is intentional for V1.
- `why` returned zero relation edges for both the Handoff record and focused
  continuation Task at the Handoff commit. The focus link is visible through
  Handoff scope and Context, but not yet modeled as a `why` relation.
- The dogfood loop still did not cover a blocked handoff recovery requiring
  stale marking plus forced Claim takeover.

## Validation

Targeted validation after the implementation:

```text
cargo test -q -p workvcs-cli cli_runs_focused_handoff_workflow
```

Result:

```text
targeted_cli_handoff_test=passed
independent_review_high_finding=fixed_by_pre_mutation_source_expectation_check
```
