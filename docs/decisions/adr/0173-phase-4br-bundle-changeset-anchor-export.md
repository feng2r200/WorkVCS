# ADR-0173: Phase 4BR Bundle ChangeSet Causal Anchor Export

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

ADR-0170 exposed generic ChangeSet causal anchors through the Engine and CLI.
ADR-0171 writes the decision-supersede causal Record into
`changeset_causal_anchor`, and ADR-0172 validates those rows through `doctor`.
ADR-0004 requires portable Bundles not to omit data needed to validate canonical
history.

Bundle export already carries ChangeSet summaries, operations, membership
changes, Event provenance, and other closure metadata, but it did not include
generic ChangeSet causal anchors.

## Decision

1. Phase 4BR extends `BundleExportManifest` with
   `changeset_causal_anchors`.
2. Each exported ref records:
   - `changeset_id`;
   - `ordinal`;
   - canonical lowercase UUIDv7 `anchor_object_id`;
   - `anchor_object_kind` from ObjectIdentity.
3. Bundle manifest parsing validates that anchor refs use canonical UUIDv7
   object ids, supported ObjectIdentity kinds, non-negative ordinals, and point
   to ChangeSets present in the exported commit closure.
4. Bundle payload validation accepts exact exported directories containing the
   new manifest field.
5. Until Bundle apply restores `changeset_causal_anchor` rows, any Bundle with
   non-empty `changeset_causal_anchors` is not marked same-Store apply
   supported.
6. CLI `bundle export` reports the new anchor count and anchor identifiers.

## Non-Goals

- This slice does not add a generic user-facing causal anchor write API.
- This slice does not backfill older Stores.
- This slice does not define the final Bundle container/archive profile.
- This slice does not apply or restore `changeset_causal_anchor` rows during
  Bundle import.
- This slice does not change SQLite schema, replay semantics, or ChangeSet
  causal anchor write semantics.

## Consequences

- Exported Bundle manifests preserve generic ChangeSet causal anchor provenance
  instead of omitting it from portable history metadata.
- Manifest and payload validation now cover the new anchor refs.
- Same-Store apply remains conservative for anchor-bearing Bundles until the
  next apply slice implements restoration.

## Implementation Findings

- No new contract ambiguity was found. The only boundary decision is the
  conservative apply gate: export and validation support anchors now, while
  import apply support remains a separate follow-up slice.
