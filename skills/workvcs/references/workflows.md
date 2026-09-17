# Plan and capture workflows

## First durable write in an unbound project

Keep discovery read-only. If it returns `project_binding_not_found`, first
confirm the logical project from the user's target and the work's primary
artifact or operation boundary. Do not bind an ambient mirror or temporary
working directory merely because it is the process cwd.

Run `project ensure` only when the first durable write is useful. For planned
work, ensure and then admit the Plan, carrying forward the useful pre-Plan
findings, decisions, unknowns, constraints, and evidence. For No-Plan work,
ensure only when a standalone semantic item is worth capturing. A failed
ensure pauses persistence, not otherwise safe work; keep a bounded pending
packet and retry after the locator/bootstrap issue is fixed.

## Standalone cognition

Use one `capture` manifest for a coherent set of new Records/Knowledge and
their relations:

```json
{
  "schema_version": 1,
  "idempotency_key": "project-specific-stable-key",
  "records": [
    {
      "local_id": "finding-1",
      "kind": "finding",
      "statement": "Observed behavior",
      "scope": {"source": "focused-probe"}
    }
  ],
  "knowledge": [
    {
      "local_id": "knowledge-1",
      "statement": "Reusable conclusion",
      "scope": {"project": "example"},
      "provenance": {"source": "finding-1"}
    }
  ],
  "evidence": [],
  "relations": [
    {
      "local_id": "validates-1",
      "type": "validates",
      "source_local_id": "finding-1",
      "target_local_id": "knowledge-1",
      "rationale": "The focused probe confirms the conclusion"
    }
  ],
  "rationale": {"reason": "preserve reusable cognition"}
}
```

Run `workvcs capture --cwd <project> --manifest <file>`. Optional expected
head/state fields add CAS guards; the CLI uses the currently verified bound
head when they are omitted. Reusing the same key with identical content returns
the prior result; a conflicting payload fails.

## No-Plan to Plan

Admit a Plan only after governance concludes that durable planning now adds
value. The first `plan admit` manifest should include:

- an explicit upgrade reason in rationale;
- the Goal and selected strategy;
- only useful Tasks and acceptance/verification requirements;
- earlier Findings, Decisions, Questions, constraints, and Evidence that the
  Plan still depends on.

Use `plan evolve mode=in_place` for additive or non-contract-breaking changes.
Use `mode=supersede` when the confirmed Plan contract is being replaced and its
ancestry must remain visible. Do not maintain parallel Plan versions merely as
a testing ritual.

## Recall and retrospective

Every Recall profile first reserves one representative from each non-empty
active Goal, active Plan, live Session, active Claim, and non-terminal Task
category before any category can consume the remaining budget. Live Sessions
include both `active` and recoverable `potentially_stale` states; a large Task
queue therefore cannot hide all current ownership detail. `brief` is for immediate
active context. `handoff` then adds the newest semantic context needed by
another Agent. `retrospective` samples the newest Records, Knowledge, semantic
relations, and Store-wide Evidence metadata across categories before remaining
semantic history and terminal Goal/Plan/Task summaries. A bounded projection
therefore cannot be consumed by one large category or old work merely because
it was recorded first.

Brief Recall includes only current Record states. Handoff Recall additionally
keeps terminal Attempts because they prevent duplicate work. Retrospective
Recall preserves terminal Findings and other history, but prioritizes current
Records and Knowledge so historical statements do not masquerade as live
project truth.

Bounded Recall does not guarantee that a particular terminal Task appears. If
continuation depends on an exact predecessor outcome, pass the stable Task or
Evidence id in the delegation contract, or create a focused Handoff and query
that object directly.

Use each item's `temporal_scope`, version/state digest,
`snapshot_commit_id`, Record scope, and Knowledge scope/provenance to
distinguish current Work State, live Runtime Coordination, and historical Store
inventory. `snapshot_commit_id` is the evaluation point, not the object's
creation time; Recall uses current version identifiers for newest-first semantic
ordering. For closeout, a branch source has
`runtime_temporal_scope=live_read_time`; an explicit commit has
`runtime_temporal_scope=historical_commit`. If evidence body matters, inspect
the item and extract its persisted content separately; do not place arbitrary
large bodies into every recall response.

Recall answers “what context should I recover now?”; currentness audit answers
“which explicit open obligations or current claims merit deliberate review?”
Use `record currentness-audit` only when that review has value. It is bounded
and read-only, and it neither replaces retrospective Recall nor turns Plan
closeout into a Record-lifecycle gate.

When a mutating command returns `mutation_postcondition_failed`, the mutation
already completed but a result-dependent `--expected-*` assertion did not.
Inspect `operation_result`, recover the current state, and decide whether any
new action is still needed. Do not blindly replay the mutation.
