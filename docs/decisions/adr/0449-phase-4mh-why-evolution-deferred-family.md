# ADR-0449: Phase 4MH Why Evolution Deferred Family

Status: Accepted
Date: 2026-09-01

## Context

`why` already explains current structural, verification, evidence, Record,
Knowledge, Knowledge exposure, and focused Handoff scope neighborhoods. Phase
4MA proved those implemented neighborhoods in a real external-project review.

The remaining gap is that some WorkVCS operations already record causal anchors
on ChangeSets, but `why` does not yet traverse ChangeSet evolution. A
continuation Agent can therefore ask `why` about the Record that caused a later
decision change and receive only current stored relations, with no indication
that a relevant evolution family exists but is not expanded.

## Decision

When `why` is queried for a current Entity, inspect the first-parent history
reachable from the target commit. If that Entity is recorded as an Entity causal
anchor on any first-parent-reachable ChangeSet, include:

```text
deferred_relation_families=1
deferred_relation_family.0=evolution
```

This is a disclosure of a known deferred family, not a relation edge. It does
not describe the ChangeSet graph, changed Entity versions, or causal direction
between records. Existing stored `relation_edges` and Handoff `scope_links`
remain unchanged.

If the queried Entity is merely superseded, changed, or related to an anchored
ChangeSet but is not itself the anchor, `why` does not report the `evolution`
deferred family.

## Non-Goals

- No full evolution traversal.
- No ChangeSet endpoint in `why`.
- No new stored Relation rows.
- No epistemic deferred-family implementation.
- No relation filters, direction filters, or relation limits for deferred
  families.
- No target-project mutation during dogfood.

## Evidence

- Focused validation:
  `/tmp/workvcs-4mh-focused-validation-rerun3-20260901T073543Z`.
- Read-only real-project dogfood:
  `/tmp/workvcs-4mh-why-evolution-dogfood-rerun-20260901T074712Z`.
- Final validation matrix:
  `/tmp/workvcs-4mh-final-validation-rerun-20260901T075035Z`.
- Independent review:
  two medium evidence-boundary issues were fixed; re-review found no remaining
  blocker/high/medium findings.

## Consequences

`why` now distinguishes "no explanation relation exists" from "a relevant
evolution family exists but is not implemented yet" for causal-anchor records.
This helps continuation Agents avoid silent under-reading while preserving the
V1 boundary around full evolution semantics.
