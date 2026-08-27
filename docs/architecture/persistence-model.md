# Versioned-State Persistence Model

This document defines the confirmed logical persistence architecture without
freezing a complete SQLite schema. It realizes
[ADR-0003](../decisions/adr/0003-versioned-state-persistence-model.md).

## Data layers

```text
Logical identity
  Entity, Relation, Resource, Store, Workspace, Branch

Immutable semantic-state versions
  EntityVersion, RelationVersion

Canonical version history
  WorkStateCommit, Commit parents, ChangeSet, Change Operations

Immutable semantic provenance
  Events, Session timeline, Evidence, ResourceObservations

Current coordination
  Session, Claim, Merge Runtime projections

Derived/rebuildable acceleration
  Branch current projections, typed projections, indexes, checkpoints
```

Fields used by deterministic algorithms or invariants must be structurally
queryable. Non-core extension semantics may use a structured extension
document. This requirement does not select vertical tables, JSON, a binary
format, or specific columns.

## Canonical history and replay

Each WorkStateCommit references exactly one ChangeSet. A ChangeSet contains:

- schema-versioned Change Operations that deterministically transform state;
- semantic Events that explain the accepted operation and its provenance.

Canonical state reconstruction uses:

```text
WorkStateCommit DAG + ChangeSets + Change Operations
```

It does not replay Events. Event schemas may evolve without redefining the
state transition. Given the same parent state and ChangeSet, application must
produce the same resulting state.

Update and destructive operations bind expected-before and after immutable
versions or equivalent structural conditions. This supports replay validation,
OCC, corruption detection, explanatory diff, and merge comparison. The final
low-level patch vocabulary remains to be specified.

### Merge replay

A merge commit records target as primary parent and source as secondary parent.
Its ChangeSet transforms the primary-parent state into the stored merged state.
Reconstruction follows the primary-parent chain and applies the recorded
ChangeSet. The merge algorithm runs only when creating the merge commit; replay
never reruns it.

## Logical identity and immutable versions

Entity identity answers who the object is. EntityVersion answers what its
complete semantic state was after a mutation. Relation uses the same split:

```text
Entity E1
  -> EntityVersion EV10
  -> EntityVersion EV27

Relation R1
  -> RelationVersion RV3
  -> RelationVersion RV9
```

EntityVersion and RelationVersion are not Branch-owned. Multiple Branches may
select the same immutable version until they diverge. Change Operations bind
the before/after versions and may also carry a structured field delta.

Removing an Entity or Relation from current Work State is logical absence or a
retired state; canonical identity and history are not physically deleted.
Physical deletion is limited to safely regenerable or policy-eligible data.

Canonical EntityVersion state is complete and schema-versioned. Readers may
upcast historical state into the current in-memory representation, but must not
rewrite old immutable versions. Change Operation payloads are independently
schema-versioned.

## Branch refs and current projections

Creating a Branch at a WorkStateCommit is an O(1) ref operation. It does not
copy every Entity or Relation.

A materialized current projection maps one Branch head's active identities to
their versions:

```text
Branch + Entity   -> EntityVersion
Branch + Relation -> RelationVersion
```

Typed current projections may expose frequently queried deterministic fields
such as Task status or priority. Current projections carry no history and are
not canonical truth. They may be rebuilt from checkpoint plus Change Operations.

Only HOT Branches must remain materialized. WARM/COLD Branch projections may be
evicted and reconstructed lazily. The eviction and checkpoint schedule remain
implementation policy.

## Checkpoints and integrity

A checkpoint is a format-versioned, digest-validated snapshot of the canonical
state at one WorkStateCommit. It accelerates historical and cold-Branch reads,
but is not a new commit or authority. Losing a checkpoint must not lose history.

Each WorkStateCommit records a digest of the canonical resulting state. Replay
and checkpoint validation compare against it. Commit identity remains distinct:
two commits with different histories may legitimately have the same state
digest.

The exact digest algorithm and canonical encoding remain Open.

## Relation graph constraints

- Canonical relation types use stable controlled identifiers. Custom meaning
  uses `related_to` plus a label.
- Primary containment is tree/forest-like in one Work State: an object has at
  most one primary parent. Multi-Plan use is expressed by `references`.
- Primary containment is acyclic.
- Task dependency is acyclic.
- A Work Graph Relation belongs to the same Workspace as both endpoints.
- Explicit sibling order remains canonical domain semantics, but its physical
  storage need not be one `ordered_before` row for every pair.

## SQLite and object-store responsibilities

The V1 direction remains SQLite metadata plus content-addressed immutable
objects:

- SQLite carries logical identities, version metadata, canonical history
  metadata, graph state, current projections, Runtime Coordination, object
  metadata, and Store description;
- the object store carries immutable Evidence blobs, large command outputs,
  detailed ResourceObservation manifests/diffs, checkpoints, and future large
  artifacts;
- ResourceObservation metadata remains queryable without requiring large
  payloads in SQLite.

A Store describes its own Store identity, format version, schema version,
capabilities, and object-format version sufficiently to determine how it is
read or migrated. This does not decide whether the manifest is a file, SQLite
metadata, or both.

## Open implementation boundary

No table family, column, index, DDL, foreign-key mechanism, transaction SQL,
programming language, ID scheme, payload encoding, hash algorithm, object
layout, ordering representation, checkpoint frequency, or eviction algorithm
is confirmed by this document.
