# ADR-0003: Versioned-State Persistence Model

- **Status:** Accepted
- **Accepted by:** explicit user confirmations for decisions 131–169
- **Confirmation turns:** `2d4839ad-2177-4806-a641-6a6935477a5b`,
  `ab97fe13-6ca6-40a4-8949-69ca6065d119`, and
  `52a93d6f-26b7-4bfb-940c-11dde28cd526`

## Context

The baseline selected SQLite metadata plus content-addressed immutable objects
as the V1 storage direction, but it did not define the canonical history,
projection, replay, or logical-version representation needed for branch,
merge, restore, repair, and portable verification.

## Decision

1. Store data is separated into logical identity, immutable semantic-state
   versions, canonical immutable history, rebuildable projections, Runtime
   Coordination, and immutable content objects. Fields used by deterministic
   algorithms and invariants must be structurally queryable; non-core
   extensions may use a structured extension document.
2. One WorkStateCommit owns exactly one ChangeSet. A ChangeSet contains
   deterministic machine-applicable Change Operations and separately associated
   semantic Events. Commit + ChangeSet + Change Operations are the canonical
   Work-State reconstruction source; Events are provenance, not replay truth.
3. Change Operations use a stable, schema-versioned, deterministic patch
   vocabulary. Update/destructive transitions bind expected-before and after
   state or equivalent structural preconditions.
4. A merge commit identifies primary parent=target and secondary parent=source.
   Its ChangeSet transforms the primary-parent state into the merged state.
   Historical reconstruction replays the recorded merge ChangeSet along the
   primary-parent chain; it never reruns the merge algorithm.
5. Checkpoints are digest-validated, format-versioned, rebuildable acceleration
   objects associated with a WorkStateCommit. They are not commits or a second
   history authority. Checkpoint scheduling is implementation policy.
6. Branch creation is an O(1) ref creation at a WorkStateCommit. Current
   materialized projections are disposable caches. Only HOT Branches need
   remain materialized; WARM/COLD Branches may reconstruct lazily.
7. Each WorkStateCommit carries a digest of its canonical resulting state for
   replay, checkpoint, corruption, and transport verification. Commit identity
   and state digest are distinct: different histories may reach the same state.
8. Entity identity is separated from immutable EntityVersion state. Relation
   identity is similarly separated from immutable RelationVersion state.
   Versions are not owned by Branches; Branches select active versions through
   their heads and projections. Removal from Work State is absence/retirement,
   not physical deletion of canonical history.
9. HOT current projections map Entity and Relation identities to their active
   versions at one Branch head and may include typed query projections. They do
   not store history. Canonical versions contain complete semantic state and a
   state-schema version; readers upcast older immutable versions without
   rewriting them.
10. Canonical relation types use controlled stable identifiers. `related_to`
    remains the labeled extension. Primary containment has at most one parent
    per object and is acyclic; multi-Plan reuse uses `references`. Task
    dependency is acyclic. Work Graph relation endpoints and ownership remain
    inside one Workspace.
11. Immutable Evidence, large outputs, detailed ResourceObservation payloads,
    and checkpoints use content-addressed object storage; SQLite carries the
    necessary logical and metadata records. Runtime tables hold current Session,
    Claim, and Merge coordination while immutable provenance records what
    happened.
12. A Store is self-describing through format, schema, capability, identity,
    and object-format metadata sufficient to determine how it can be read or
    migrated.

## Consequences

- The architecture is commit/delta-based versioned state with provenance
  Events and materialized projections, not pure Event Sourcing and not a
  full-Workspace snapshot per commit.
- Branches can share immutable versions and start without copying current
  state.
- Projection loss is repairable from checkpoint plus canonical Change
  Operations.
- Runtime and canonical history retain different recovery and portability
  rules.

## Deliberately not decided

- complete SQLite schema, DDL, table/column names, indexes, or transaction SQL;
- implementation language;
- persistent ID format, state-digest algorithm, canonical encoding, or object
  layout;
- JSON versus another canonical version payload encoding;
- physical representation of explicit sibling order;
- checkpoint frequency, size thresholds, and eviction policy.
