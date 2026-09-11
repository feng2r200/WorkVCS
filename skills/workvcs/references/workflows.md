# Plan and capture workflows

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

`brief` is for immediate active context. `handoff` starts from active work and
includes semantic relations needed by another Agent. `retrospective` samples
the newest Records, Knowledge, semantic relations, and Store-wide Evidence
metadata across categories before remaining semantic history and terminal
Goal/Plan/Task summaries. A bounded projection therefore cannot be consumed by
one large category alone. If evidence body matters, inspect the item and extract
its persisted content separately; do not place arbitrary large bodies into
every recall response.
