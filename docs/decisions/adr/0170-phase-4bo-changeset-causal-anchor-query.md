# ADR-0170: Phase 4BO ChangeSet Causal Anchor Query

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

The frozen physical schema includes `changeset_causal_anchor` for generic local
ObjectIdentity causal anchors. Current ChangeSet summary and operation drill-down
queries expose provenance metadata, but the generic causal anchor table had no
Engine or CLI read surface.

## Decision

1. Add `changeset_causal_anchors` to the Engine facade.
2. Return anchors as ordered `ordinal`, canonical UUIDv7 object id text, and
   ObjectIdentity kind.
3. Add `changeset anchors` to the CLI.
4. Add `causal_anchors` count to `changeset show`.

## Non-Goals

- This slice does not add or alter any generic causal anchor write path.
- This slice does not infer anchors from rationale text or semantic
  `derived_from` relations.
- This slice does not change the schema or bundle format.

## Consequences

- Existing and imported generic ChangeSet causal anchors are inspectable through
  the same Engine/CLI boundary as ChangeSet summaries and operations.
- The remaining writer-side gap is explicit: semantic causal relations and
  generic ChangeSet causal anchors are still separate persistence surfaces.
