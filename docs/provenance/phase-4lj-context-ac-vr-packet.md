# Phase 4LJ Context AC and VR Packet Evidence

Status: current local evidence
Date: 2026-09-01

## Scope

Phase 4LJ adds current-task Acceptance Criterion and Verification Requirement
items to bounded `ContextPacket` output. The intent is to make `claim next
--context-*` useful immediately before a `workvcs verify` wrapper call.

This evidence does not claim full Context Resolver completion, packet
persistence, Resource adapter-backed observation, another-project dogfood, or
release readiness.

## Dogfood Setup

Commands were run from:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lj-context-ac-vr-packet
```

The local dogfood Store and log were:

```text
.work-governance/runtime/dogfood/phase-4lj-20260831201410.sqlite
.work-governance/runtime/logs/phase-4lj-dogfood-20260831201410.log
```

Captured IDs:

```text
workspace_id=01a05975-6836-76b1-83e8-582220ad2193
branch_id=01a05975-6836-76b1-83e8-585db068b707
task_entity_id=01a05975-6851-7f10-b280-98cf1fcb3eb6
acceptance_criterion_entity_id=01a05975-6868-7a20-9f1a-15084963b714
verification_requirement_entity_id=01a05975-6880-7390-9522-09265cb1736e
verification_entity_id=01a05975-68d8-7d33-8169-887bcb44a39f
verified_head=01a05975-68d8-7d33-8169-88cd57f07544
```

## Dogfood Run

The operator created one Task, one required Acceptance Criterion, and one
Verification Requirement, then claimed the next Task with a brief packet budget:

```text
workvcs claim next STORE --session SESSION \
  --context-profile brief \
  --context-budget-items 5 \
  --expected-selected true \
  --expected-task TASK
```

The packet exposed the verification obligations before the verification command:

```text
claim_next_context_item.3.category=acceptance_criterion
claim_next_context_item.3.subject=acceptance_criterion:01a05975-6868-7a20-9f1a-15084963b714
claim_next_context_item.3.summary_json="acceptance criterion task=01a05975-6851-7f10-b280-98cf1fcb3eb6 local_key=ac-context classification=required status=unverified requirements=1: Claim packet shows the validation obligation"
claim_next_context_item.4.category=verification_requirement
claim_next_context_item.4.subject=verification_requirement:01a05975-6880-7390-9522-09265cb1736e
claim_next_context_item.4.summary_json="verification requirement criterion=01a05975-6868-7a20-9f1a-15084963b714 local_key=vr-wrapper: Run workvcs verify with captured evidence"
```

The operator then used the surfaced Verification Requirement id with
`workvcs verify`, and `ac status` returned:

```text
status=verified
```

Full summary:

```text
dogfood_result=passed
dogfood_store=.work-governance/runtime/dogfood/phase-4lj-20260831201410.sqlite
dogfood_session_id=01a05975-6894-75f0-9658-c09f2bf03db0
dogfood_task_id=01a05975-6851-7f10-b280-98cf1fcb3eb6
dogfood_acceptance_criterion_id=01a05975-6868-7a20-9f1a-15084963b714
dogfood_verification_requirement_id=01a05975-6880-7390-9522-09265cb1736e
dogfood_verified_head=01a05975-68d8-7d33-8169-88cd57f07544
```

## Validation

Targeted validation passed before documentation closeout:

```text
cargo fmt --all -- --check
cargo test -q -p workvcs-core --test context_profile_budget_phase4kx
```

Post-review validation passed after adding the Acceptance Criterion identity
guard and updating the repository smoke packet expectations:

```text
cargo fmt --all -- --check
cargo test -q -p workvcs-core --test context_profile_budget_phase4kx
scripts/smoke-v0.1-cli-workflow.sh
git diff --check
```

Final validation passed before local commit:

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
.work-governance/runtime/logs/phase-4lj-final-validation-20260831202243.log
```

Independent review found no blocker issues. It raised one high closeout risk
for committing an active governance Plan to `main`, to be resolved by
`workctl goal close` before staging, and one medium AC identity guard issue,
resolved by failing closed when the Task AC reference does not match the stored
Criterion task id or local key.

## Findings

- `ContextPacket` now includes brief-eligible `acceptance_criterion` and
  `verification_requirement` items for current Task candidates.
- The items are read-only and derive from existing Store snapshots at the active
  Branch head.
- AC items include local key, classification, branch-scoped effective status,
  requirement count, and statement.
- VR items include the owning Acceptance Criterion id, local key, and statement.
- The tight budget dogfood confirms AC/VR obligations survive before lower
  priority readiness detail.
- Context resolution fails closed if a Task Acceptance Criterion reference does
  not match the stored Criterion task id or local key, matching the existing VR
  owner/local-key guard.
- The repository smoke workflow now verifies an `acceptance_criterion` packet
  item under `workvcs context --profile brief --budget-items 5`.
- Remaining Context Resolver gaps are Goal/Plan path packets, richer blocker
  context, Attempt execution detail, path-sensitive Knowledge policy, packet
  persistence, and broader multi-project dogfood.
