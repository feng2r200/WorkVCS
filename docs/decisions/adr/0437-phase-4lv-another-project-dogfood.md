# ADR-0437: Phase 4LV Another Project Read-Only Dogfood

Status: Accepted
Date: 2026-09-01

## Context

The V1 readiness ledger intentionally shifted implementation focus away from
narrow smoke expectations, count fields, and display-only details. After Phase
4LU, the most important remaining product-risk question was whether the current
CLI could manage a realistic WorkVCS loop for another real local project, not
only WorkVCS's own throwaway smoke Stores.

The target for this slice was the local `agent_soul` project:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul
```

The target repository already had unrelated local changes, so Phase 4LV treated
it as read-only. All WorkVCS Store and log writes stayed under the 4LV
WorkVCS worktree.

## Decision

Accept a read-only another-project dogfood loop as the next readiness slice.
The loop must exercise the current CLI as an operator would use it:

- create a Workspace, Goal, Plan, Task, Acceptance Criterion, Verification
  Requirement, Resource, Session, and Claim in a local WorkVCS Store;
- request scoped context during `claim next`;
- persist and inspect a scoped ContextPacket snapshot;
- run the top-level `verify` wrapper with evidence content, Resource
  observation, Resource basis, and applicability cache output;
- prove the AC becomes `verified` and the Task can transition to `done`;
- complete the Plan and achieve the Goal with rationale;
- prove the Goal transition rationale appears in a saved ContextPacket;
- end the Session and author/show a focused Handoff tied to the SessionDiff;
- pass Store integrity checks; and
- compare target-project status snapshots before and after the loop.

This slice is evidence-only unless the dogfood run exposes a concrete
implementation blocker. It does not change the external project, infer records
from transcripts, automate adapters, add a TUI/GUI, or widen V2 scope.

## Evidence

Local dogfood passed through the real CLI:

```text
phase4lv_dogfood_result=PASS
dogfood_store=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lv-another-project-dogfood/.work-governance/runtime/dogfood/phase-4lv-20260901T013513Z.sqlite
log_dir=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lv-another-project-dogfood/.work-governance/runtime/logs/phase-4lv/another-project-dogfood.20260901T013513Z
target_project=/Users/example/Projects/HeXun/Hernes/agent_soul
task_entity_id=01a05a9b-5735-7f12-93cb-345a4b7cb4ee
acceptance_criterion_entity_id=01a05a9b-5764-7d61-8416-d8355b90d3e2
verification_requirement_entity_id=01a05a9b-577f-7033-a2c9-5a3eca10db9c
context_packet_id=01a05a9b-580c-7661-890d-e96cb2f0551a
transition_context_packet_id=01a05a9b-596c-75a0-a42e-21b427229e3a
verification_entity_id=01a05a9b-5867-78a1-99fe-cada9a0f0cb9
resource_id=01a05a9b-56d2-7630-b328-8a5098b53af6
session_diff_id=01a05a9b-5993-7a53-baba-b47256c606b7
handoff_record_id=01a05a9b-59b3-7ff1-a480-99fc768a7e3a
target_status_unchanged=PASS
```

The run found no implementation blocker requiring code changes.

## Consequences

The ledger's first next-queue item is now satisfied for one bounded real
external project. This is not a claim of broad release maturity: the proof is
single-project, local, read-only, and operator-scripted.

Next slices should continue to prefer dogfood friction over display polish:
Claim transfer/takeover in realistic continuation, Resource path/glob
normalization or adapter-backed re-observation when a workflow needs it,
broader `why` explanation paths, and larger or more varied Store validation.
