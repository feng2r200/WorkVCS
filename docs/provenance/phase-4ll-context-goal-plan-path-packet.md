# Phase 4LL Context Goal/Plan Path Packet Evidence

Status: current local evidence
Date: 2026-09-01

## Scope

Phase 4LL adds `goal_plan_path` items to bounded `ContextPacket` output. The
intent is to make Task packets expose the containing Goal/Plan hierarchy
without changing runnable selection, Claim behavior, or semantic write paths.

This evidence does not claim full Context Resolver completion, context packet
persistence, path-sensitive Knowledge ranking, another-project dogfood, or
release readiness.

## Dogfood Setup

Commands were run from:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4ll-context-plan-path-packet
```

The local dogfood Store and log were:

```text
.work-governance/runtime/dogfood/phase-4ll-20260831210338.sqlite
.work-governance/runtime/logs/phase-4ll-dogfood-20260831210338.log
```

Captured IDs:

```text
goal_id=01a059a2-8cf6-7e91-9e7e-11e1d8a7da43
plan_id=01a059a2-8d0b-73b3-8854-c24036f8a388
task_id=01a059a2-8d1f-7801-953a-ceec212bc805
goal_plan_relation_id=01a059a2-8d34-7131-a213-51355e0617a4
plan_task_relation_id=01a059a2-8d4a-7781-9470-0292d2a66888
session_id=01a059a2-8d5d-7a13-991f-1ee98d2c5c4f
```

## Dogfood Run

The operator created a Goal, Plan, Task, and primary containment path:

```text
Goal -> Plan -> Task
```

The operator then claimed the next Task and requested a brief packet:

```text
workvcs claim next STORE --session SESSION \
  --expected-task TASK \
  --context-profile brief \
  --context-budget-items 4
```

The claim packet exposed the path:

```text
claim_next_context_item.3.priority=P1
claim_next_context_item.3.category=goal_plan_path
claim_next_context_item.3.subject=goal_plan_path:01a059a2-8d1f-7801-953a-ceec212bc805
claim_next_context_item.3.summary_json="goal_plan_path task=01a059a2-8d1f-7801-953a-ceec212bc805 path=goal:01a059a2-8cf6-7e91-9e7e-11e1d8a7da43 status=active relation=01a059a2-8d34-7131-a213-51355e0617a4: Deliver Phase 4LL dogfood goal > plan:01a059a2-8d0b-73b3-8854-c24036f8a388 status=active relation=01a059a2-8d4a-7781-9470-0292d2a66888: Orient the agent from packet context > task:01a059a2-8d1f-7801-953a-ceec212bc805"
```

Direct `context` output repeated the same path item:

```text
context_item.3.priority=P1
context_item.3.category=goal_plan_path
context_item.3.subject=goal_plan_path:01a059a2-8d1f-7801-953a-ceec212bc805
```

Full summary:

```text
dogfood_result=passed
dogfood_store=.work-governance/runtime/dogfood/phase-4ll-20260831210338.sqlite
dogfood_session_id=01a059a2-8d5d-7a13-991f-1ee98d2c5c4f
dogfood_goal_id=01a059a2-8cf6-7e91-9e7e-11e1d8a7da43
dogfood_plan_id=01a059a2-8d0b-73b3-8854-c24036f8a388
dogfood_task_id=01a059a2-8d1f-7801-953a-ceec212bc805
dogfood_goal_plan_relation_id=01a059a2-8d34-7131-a213-51355e0617a4
dogfood_plan_task_relation_id=01a059a2-8d4a-7781-9470-0292d2a66888
```

## Validation

Targeted validation passed before documentation closeout:

```text
cargo fmt --all -- --check
cargo test -q -p workvcs-core --test context_profile_budget_phase4kx
scripts/smoke-v0.1-cli-workflow.sh
git diff --check
```

Targeted validation log:

```text
.work-governance/runtime/logs/phase-4ll-targeted-validation-20260831210047.log
```

Documentation validation passed after ADR, operator, README, and ledger updates:

```text
cargo fmt --all -- --check
cargo test -q -p workvcs-core --test context_profile_budget_phase4kx
scripts/smoke-v0.1-cli-workflow.sh
git diff --check
```

Documentation validation log:

```text
.work-governance/runtime/logs/phase-4ll-doc-validation-20260831210511.log
```

Final validation matrix passed:

```text
cargo fmt --all -- --check
cargo clippy --quiet --all-targets --all-features -- -D warnings
cargo test --workspace --quiet
scripts/validate-schema-v0.1.sh
scripts/smoke-v0.1-cli-workflow.sh
git diff --check
```

Final validation log:

```text
.work-governance/runtime/logs/phase-4ll-final-validation-20260831210741.log
```

## Independent Review

Independent review found no blocker, high, or medium-severity issues in the
Phase 4LL implementation, tests, smoke expectations, or docs.

The review confirmed:

- default non-packet context overview output is unchanged;
- schema, semantic write paths, Claim behavior, and verify wrapper behavior are
  unchanged;
- the P1 priority, subject format, summary content, and budget omission behavior
  are coherent for this slice;
- smoke count changes align with the added Goal, Plan, and two primary
  containment commits/events;
- docs keep Context Resolver marked Partial and preserve remaining V1 gaps.

## Findings

- `goal_plan_path` items are brief-eligible P1 packet items.
- The subject binds the current Task.
- The summary exposes root-to-leaf Goal/Plan path, ancestor status, ancestor
  descriptions, and containment relation ids.
- Packet resolution fails closed if the current containment graph is
  inconsistent or a referenced Goal/Plan snapshot is absent from the current
  Work State.
- Repository smoke now creates a real Goal -> Plan -> Task path before context
  packet inspection.
- Remaining Context Resolver gaps are Attempt execution detail,
  path-sensitive Knowledge policy, packet persistence, and broader
  multi-project dogfood.
