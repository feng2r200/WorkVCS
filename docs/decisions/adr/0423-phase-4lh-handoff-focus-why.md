# ADR-0423: Phase 4LH Handoff Focus Why

Status: Accepted
Date: 2026-09-01

## Context

Phase 4LF dogfooded focused Handoff consumption and found a reviewability gap:
`why` returned zero relation edges for both the Handoff Record and the focused
Task. That was accurate under ADR-0034 and ADR-0061 because focused Handoff
state lives in Record scope, and no typed Handoff Relation is currently stored.

The V1 readiness ledger required a decision before changing explanation
semantics. The safe path is to make focused Handoff scope visible without
pretending that it is a stored Relation.

## Decision

Add read-only `why` scope links for recognized focused Handoff scope:

```text
Record(kind=handoff) --handoff_focus--> focused Entity
```

The link is emitted only when all of these are true:

1. the Record is current at the queried commit;
2. the Record kind is `handoff`;
3. the scope has `handoff_scope_schema_version=1`;
4. the scope has a non-null `focus_entity_id`; and
5. the focused Entity is current at the queried commit.

Malformed, incomplete, or future-version Handoff scope is treated as
unrecognized for this auxiliary `why` output and does not make unrelated
`why` queries fail. Strict focused Handoff validation remains on the
`handoff show` and `handoff consume` command paths.

The output is separate from stored relation edges:

```text
relation_edges=0
scope_links=1
scope_link.0.link_kind=handoff_focus
```

CLI `why` now supports `--expected-scope-links` so smoke and dogfood runs can
assert this reviewability contract. Existing relation filters and
`--relation-limit` still apply only to stored `relation_edges`; scope links are
reported and counted separately.

## Non-Goals

- No stored Relation rows for Handoff focus.
- No schema migration or Handoff scope schema change.
- No arbitrary Record scope traversal.
- No causal/evolution graph expansion.
- No relation-style filtering or limiting for scope links in this slice.
- No implicit Handoff consumption, Claim, Task transition, or context mutation.

## Consequences

Operators can now use `why` to review both sides of a focused Handoff:

1. querying the Handoff Record reports an outgoing `handoff_focus` scope link;
2. querying the focused Task reports the same link as incoming; and
3. relation-edge output remains reserved for real stored relations.

This closes the Phase 4LF zero-edge review gap while keeping the V1 boundary
intact. Full `why` maturity still needs broader causal, evolution, and
real-project dogfood evidence.
