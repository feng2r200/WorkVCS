# Versioned-State Persistence Model

This document defines the confirmed logical persistence architecture without
freezing a complete SQLite schema. It realizes
[ADR-0003](../decisions/adr/0003-versioned-state-persistence-model.md).
The later confirmed logical-family split, ownership rules, and projection
contracts are defined in
[Logical Schema Boundaries](logical-schema-boundaries.md), which realizes
[ADR-0005](../decisions/adr/0005-logical-schema-family-boundaries.md).

## Data layers

```text
Logical identity
  Store, Workspace, Branch, ObjectIdentity and matching typed families

Immutable semantic-state versions
  EntityVersion, RelationVersion

Canonical version history
  WorkStateCommit, Commit parents, ChangeSet, Change Operations

Immutable semantic provenance
  Events, Session timeline, Evidence, ResourceObservations

Current coordination
  SessionRuntime, ClaimRuntime, MergeRuntime and their structured children

Derived/rebuildable acceleration
  Branch current projections, typed projections, indexes, checkpoints
```

Fields used by deterministic algorithms or invariants must be structurally
queryable. Non-core extension semantics may use a structured extension
document. This requirement does not select vertical tables, JSON, a binary
format, or specific columns.

## Canonical history and replay

Each WorkStateCommit belongs to a Workspace rather than a Branch and references
exactly one non-reusable ChangeSet. Each Workspace has one zero-parent Genesis
Commit for empty Work State; normal and V1 merge Commits have one and two
explicitly role-identified parents respectively. A ChangeSet contains:

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

Entity and Relation are ObjectIdentity-backed typed families; ObjectIdentity
itself is only the Store-local addressable registry. Every committed registry
entry has exactly one kind-matching family owner. Store and Workspace identities
remain above/outside the registry.

EntityVersion does not carry a single previous-version chain. Relation
identity uses immutable Workspace/type/source/target/discriminator properties;
remove/re-add of the same logical key reuses the Relation identity.

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
not canonical truth. Each complete projection binds its projected Commit and
validates its state digest against HEAD; without that proof, a missing row does
not establish absence. Projections may be rebuilt from checkpoint plus Change
Operations.

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

V1 uses BLAKE3-256 for state/content digests and canonical Resource
fingerprints. Canonical structured payload is UTF-8 JSON text whose canonical
bytes are produced by the WorkVCS serializer under the Store Manifest profile.
The exact profile rules and serializer implementation remain part of the next
schema/implementation specification.

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
metadata, or both. The current manifest may change through migration while each
migration remains immutable provenance.

## Current implementation boundary

Logical table-family responsibilities are confirmed in
[Logical Schema Boundaries](logical-schema-boundaries.md). The later
[Physical Schema v0.1 Contract](physical-schema-v0.1.md) closes UUIDv7/BLOB ID
encoding, BLAKE3-256 digests, JSON/timestamp/vocabulary storage, core SQLite
foreign-key policy, and the writer-transaction baseline. ADR-0007 through
ADR-0009 and [Implementation Contract v0.1](implementation-contract-v0.1.md)
close the executable schema assembly, Rust implementation language, `rusqlite`,
and the `workvcs-jcs-v1` canonical JSON profile. Performance indexes, final
object layout, ordering representation beyond current behavior, checkpoint
policy, and typed-projection count remain Open.
