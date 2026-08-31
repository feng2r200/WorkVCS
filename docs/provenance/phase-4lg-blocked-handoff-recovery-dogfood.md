# Phase 4LG Blocked Handoff Recovery Dogfood Evidence

Status: current local evidence
Date: 2026-09-01

## Scope

Phase 4LG closes the next dogfood gap from the V1 readiness ledger: a focused
Handoff continuation blocked by another active Session's exclusive Claim can be
recovered through explicit stale marking and stale-gated forced takeover.

This evidence does not claim automatic stale detection, another-project use,
release readiness, or reduced ID capture beyond the existing `handoff consume`
command.

## Dogfood Setup

Commands were run from:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lg-blocked-handoff-recovery-dogfood
```

The CLI was built locally with:

```text
cargo build -q -p workvcs-cli
```

The ignored local Store and log used for the dogfood run were:

```text
.work-governance/runtime/dogfood/phase-4lg-20260901031927.sqlite
.work-governance/runtime/logs/phase-4lg-dogfood-20260901031927.log
```

Captured IDs:

```text
workspace_id=01a05943-28c1-7a10-b433-62fa59b47fa9
branch_id=01a05943-28c1-7a10-b433-632204e346af
source_task_id=01a05943-28d6-7dd3-a6d6-a10d24bbe445
blocked_task_id=01a05943-28ea-7c81-8759-130df9ac1594
blocked_task_version_id=01a05943-28ea-7c81-8759-12f8c13e751b
source_session_id=01a05943-28fc-7401-aecb-4c331d62f8b4
source_claim_id=01a05943-291d-7cf0-b40e-8638b72a715c
handoff_record_id=01a05943-2932-7730-b364-83e9df462fff
handoff_commit_id=01a05943-2932-7730-b364-83b0986b79bf
continuation_session_id=01a05943-2944-75c2-86c7-2ee728e034aa
```

## Recovery Run

The source Session stayed active and held an exclusive Claim on the Handoff
focus Task. The continuation Session then consumed the Handoff:

```text
consume_focus_entity_id=01a05943-28ea-7c81-8759-130df9ac1594
context_focus_entity_id=01a05943-28ea-7c81-8759-130df9ac1594
```

Before recovery, guard and `next` proved the block:

```text
blocked_allowed=false
blocked_reason=exclusive_claim_owned_by_other_session
next_selected_before_recovery=false
```

The recovery sequence explicitly marked the previous owning Session stale, then
performed forced takeover:

```text
source_lifecycle_after_mark=potentially_stale
takeover_claim_id=01a05943-29ba-7c51-b55c-51e4cca303b1
previous_session_lifecycle_state=potentially_stale
```

After takeover, guard allowed the continuation Session to continue:

```text
recovered_allowed=true
recovered_reason=owned_exclusive_claim
continuation_session_diff_id=01a05943-29dd-7912-ba80-38065d88b8f9
```

Full summary:

```text
dogfood_result=passed
store=.work-governance/runtime/dogfood/phase-4lg-20260901031927.sqlite
log=.work-governance/runtime/logs/phase-4lg-dogfood-20260901031927.log
```

## Findings

- The existing `handoff consume`, `claim guard`, `session mark-stale`, and
  `claim takeover` commands are sufficient for a real blocked focused Handoff
  recovery loop.
- The recovery path correctly refuses to proceed through `next` while another
  active Session owns the exclusive Claim.
- The stale-gated takeover evidence chain is explicit and local: mark the
  previous owner `potentially_stale`, then force-take over with rationale.
- Remaining friction is ID capture for the previous owning Session and Claim.
  This is acceptable for V1 unless repeated dogfood shows it blocks operators.
