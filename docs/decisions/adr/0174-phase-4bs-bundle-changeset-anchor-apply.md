# ADR-0174: Phase 4BS Bundle ChangeSet Causal Anchor Apply

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

ADR-0173 exports generic ChangeSet causal anchors in Bundle manifests and
validates their canonical identity shape, but it kept same-Store apply disabled
for anchor-bearing Bundles. That was a temporary apply gate, not a permanent
contract boundary.

The next portable-history requirement is to restore `changeset_causal_anchor`
rows when the same Store imports a fast-forward Bundle that carries anchors for
objects already covered by the Bundle apply closure.

## Decision

1. Same-Store Bundle apply supports non-empty `changeset_causal_anchors` when
   every anchor ChangeSet is present in the exported commit closure and every
   anchor target ObjectIdentity is represented by an object family that the
   same manifest can already apply.
2. Apply restores `changeset_causal_anchor` rows after commit closure import and
   before Event, checkpoint, and branch-head updates.
3. Anchor insertion is idempotent for the same `(changeset_id, ordinal,
   anchor_object_id)` tuple.
4. Import rejects immutable conflicts when the same `(changeset_id, ordinal)`
   points to a different object or the same `(changeset_id, anchor_object_id)`
   appears at a different ordinal.
5. Import apply result details and CLI rendering report
   `imported_changeset_causal_anchors`.

## Non-Goals

- This slice does not add a generic user-facing causal anchor write API.
- This slice does not backfill older Stores.
- This slice does not define the final Bundle container/archive profile.
- This slice does not add cross-Store import support.
- This slice does not add Bundle apply closure support for object families that
  are not already importable.

## Consequences

- Anchor-bearing fast-forward Bundles produced by the current implementation can
  now be applied back into an older copy of the same Store without dropping
  generic ChangeSet provenance.
- Preflight stays conservative: a Bundle is applyable only when its causal
  anchor targets are inside the same manifest object closure.
- Imported anchor counts are visible in Engine detail JSON and CLI output.

## Implementation Findings

- No frozen-contract contradiction was found.
- `claim` and `merge_attempt` ObjectIdentity anchor targets remain unsupported
  for same-Store Bundle apply until those object families have Bundle apply
  closure support.
