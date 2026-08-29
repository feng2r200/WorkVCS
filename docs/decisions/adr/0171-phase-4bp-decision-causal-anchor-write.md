# ADR-0171: Phase 4BP Decision Causal Anchor Write

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

ADR-0074 added `record supersede-decision --because-record` as a semantic
Record-to-Record causal edge. ADR-0170 exposed the generic
`changeset_causal_anchor` table through Engine and CLI queries, but the existing
decision supersede writer did not populate that generic provenance surface.

## Decision

When `DecisionRecordSupersedeOptions::with_causal_record` is provided, the same
atomic ChangeSet now writes one `changeset_causal_anchor` row:

- `changeset_id`: the supersede ChangeSet;
- `ordinal`: `0`;
- `anchor_object_id`: the causal Record entity id.

The semantic `derived_from` relation remains unchanged and remains the
authoritative traversable domain relation.

## Non-Goals

- This slice does not add a generic user-facing causal anchor write API.
- This slice does not infer anchors from rationale text.
- This slice does not backfill older stores or migrate existing ChangeSets.
- This slice does not change replay semantics or schema.

## Consequences

- `changeset show` and `changeset anchors` now expose a concrete generic anchor
  for decision supersede operations that use `--because-record`.
- Generic ChangeSet anchors and semantic `derived_from` relations remain separate
  but are now co-written for this confirmed workflow.
