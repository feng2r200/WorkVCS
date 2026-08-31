# Phase 4LI Claim Next Context Packet Evidence

Status: current local evidence
Date: 2026-09-01

## Scope

Phase 4LI advances the V1 Context Resolver by making `claim next` optionally
return the same bounded `ContextPacket` surface already used by `context
--profile/--budget-items`.

This evidence does not claim full Context Resolver completion, context packet
persistence, new Engine transaction semantics, another-project dogfood, or
release readiness.

## Dogfood Setup

Commands were run from:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4li-claim-next-context-packet
```

The CLI was built locally with:

```text
cargo build -q -p workvcs-cli
```

The ignored local Store and log used for the dogfood run were:

```text
.work-governance/runtime/dogfood/phase-4li-20260831195427.sqlite
.work-governance/runtime/logs/phase-4li-dogfood-20260831195427.log
```

Captured IDs:

```text
workspace_id=01a05963-30be-7010-8b41-ed76f9a1e224
branch_id=01a05963-30be-7010-8b41-edad7b3d32f8
session_id=01a05963-30ec-76f0-bd08-59fc8eea3998
task_entity_id=01a05963-30d7-7a61-af43-3dad4ad460ad
task_commit_id=01a05963-30d7-7a61-af43-3d7f59b17a32
```

## Dogfood Run

The operator ran one `claim next` command with explicit packet options:

```text
workvcs claim next STORE --session SESSION \
  --context-profile brief \
  --context-budget-items 3 \
  --expected-selected true \
  --expected-task TASK
```

The same command selected and claimed the Task, focused the Session, and
returned a bounded context packet:

```text
selected=true
claim_next_context_packet=true
claim_next_focus_entity_id=01a05963-30d7-7a61-af43-3dad4ad460ad
claim_next_context_profile=brief
claim_next_context_budget_items=3
claim_next_context_items=3
claim_next_context_omitted_items=1
```

Full summary:

```text
dogfood_result=passed
store=.work-governance/runtime/dogfood/phase-4li-20260831195427.sqlite
log=.work-governance/runtime/logs/phase-4li-dogfood-20260831195427.log
```

## Validation

Targeted validation passed before documentation closeout:

```text
cargo test -q -p workvcs-cli cli_claim_next_can_return_budgeted_context_packet
cargo test -q -p workvcs-cli cli_runs_claim_next_workflow
cargo clippy -q -p workvcs-cli --all-targets -- -D warnings
```

## Findings

- `claim next` default output remains compatible: no context packet fields are
  emitted unless packet options are provided.
- `--context-profile` and `--context-budget-items` reuse existing context packet
  validation and rendering semantics.
- Invalid context budget is rejected before the Claim write path; the regression
  test proves a valid subsequent `claim next` can still select the Task.
- Context packet output is prefixed with `claim_next_` so it does not overwrite
  existing claim-next keys in line-oriented consumers.
- Remaining Context Resolver gaps are AC packets, Goal/Plan path packets,
  richer blocker context, Attempt execution detail, path-sensitive Knowledge
  policy, and packet persistence.
