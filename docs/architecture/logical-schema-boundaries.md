# Logical Schema Boundaries

This document defines the confirmed logical schema families and authority
boundaries for WorkVCS. It realizes
[ADR-0005](../decisions/adr/0005-logical-schema-family-boundaries.md).

It is intentionally the logical layer. Names in this document name logical
roles and families; later physical choices are defined by
[Physical Schema v0.1 Contract](physical-schema-v0.1.md),
[ADR-0006](../decisions/adr/0006-sqlite-physical-schema-v0.1.md), and
[ADR-0007](../decisions/adr/0007-schema-v0.1-assembly-install-and-integrity.md).
This document does not independently redefine those physical choices.

## Authority layers

The confirmed model has one authority for each kind of state:

| Concern | Canonical authority |
|---|---|
| Workspace Work State | `Branch HEAD -> WorkStateCommit DAG -> ChangeSet -> ChangeOperation` |
| Addressable identity | Store-local `ObjectIdentity` plus exactly one matching typed family owner |
| Entity state | `Entity` plus immutable, complete `EntityVersion` values selected by Work State |
| Relation state | `Relation` plus immutable `RelationVersion` values selected by Work State |
| Branch current reads | complete, rebuildable Entity/Relation and workload-driven typed projections |
| Runtime Coordination | mutable Session, Claim, and Merge runtime projections |
| Runtime and infrastructure history | immutable occurrences and Events |
| Evidence content | logical `Evidence` plus zero or more digest-identified `ContentObject` values |
| Resource state | portable `Resource`, environment-local `ResourceBinding`, immutable `ResourceObservation` |
| Knowledge federation | `KnowledgeSpace`, immutable `KnowledgeExposure`, append-only transitions, current and source-status projections |
| Historical acceleration | optional, rebuildable Checkpoints |
| Store format | current Store manifest plus immutable migration provenance |
| Transport | Bundle manifest plus immutable import-attempt provenance |

No projection, runtime row, Event stream, Checkpoint, or Bundle file becomes a
second Work-State authority.

## Identity families

### Store and Workspace identities

`Store` has a stable identity above the Store-local ObjectIdentity registry.
`Workspace` has a stable Store-level identity and owns one Work-State versioning
boundary. Neither is an `ObjectIdentity` or an Entity.

Workspace identity is independent of mutable display name, directory, and
repository path. Store copy, move, backup/restore, and ordinary export/import
preserve Store identity. An explicit Store fork creates a new Store identity.

Inside a Store, each local object, Workspace, and Commit ID is stable in its
own confirmed namespace. A portable cross-Store reference is understood as:

```text
(store_id, local_id)
```

The model does not require the local ID alone to be universally unique. An
explicit Store fork preserves internal local IDs by default; the new Store ID
creates a new namespace and fork lineage records the source root/state.
V1 adds no separate Workspace fork/clone operation: divergence inside one
Workspace uses Work Branch, while independent duplication of the whole
portable boundary uses explicit Store fork. A later Workspace-extraction
operation remains outside this confirmed model.

### ObjectIdentity registry

`ObjectIdentity` is the Store-wide registry for addressable objects across
typed families. It records stable local object identity and a controlled
`object_kind`; it does not contain semantic state, relation endpoints, Branch
selection, or runtime status.

Confirmed ObjectIdentity-backed families are:

- Entity and Relation;
- Session and SessionDiff;
- Claim and MergeAttempt;
- Evidence;
- Resource and ResourceObservation;
- KnowledgeSpace and KnowledgeExposure.

Version-control and infrastructure identities remain typed outside this
registry: Store, Workspace, Branch, WorkStateCommit, ChangeSet,
ChangeOperation, Event, Checkpoint, ContentObject, VerificationBasis, runtime
projections, BundleManifest, and ImportAttempt.

Every committed ObjectIdentity has exactly one matching typed family owner.
Its `object_kind` and owner must agree. A transaction may construct the pair
atomically, but committed state cannot contain an orphan identity, a missing
typed owner, or two family owners for one object ID.

## Versioned Entity family

The unified Entity/EntityVersion family covers:

- Goal, Plan, Task, Decision, Knowledge, and Record;
- AcceptanceCriterion and VerificationRequirement;
- Verification.

AcceptanceCriterion and VerificationRequirement use stable internal Entity
identity even though their public display references are Task/AC scoped.
Verification uses the same Work-State infrastructure but is an immutable
judgment: normal creation yields one immutable semantic EntityVersion.

Entity and ObjectIdentity share one stable ID. Entity Workspace ownership and
Entity kind are identity properties and never change through an EntityVersion.
Moving across Workspaces or changing semantic kind creates another identity
with explicit provenance.

An EntityVersion:

- is immutable and not owned by a Branch or Commit;
- contains the complete Entity-owned canonical semantic state, not a patch;
- excludes cross-object relations, Runtime Coordination, and derived state;
- does not carry a single `previous_version_id` chain;
- may carry a digest of canonical semantic state only.

Commit DAG transitions, not an EntityVersion-local chain, describe evolution.
Different EntityVersion identities may legitimately have equal state digests;
automatic version deduplication is not semantic behavior.

## Versioned Relation family

Relation is addressable but is not an Entity. It has an independent
Relation/RelationVersion family. Relation Workspace ownership, canonical type,
source ObjectIdentity, target ObjectIdentity, and logical discriminator are
immutable identity properties. RelationVersion contains only complete
immutable relation-owned metadata state and does not redefine endpoints or
type.

The logical relation key is:

```text
workspace
+ canonical relation type
+ source object
+ target object
+ immutable discriminator
```

Ordinary canonical relations use an empty discriminator. For labeled
`related_to`, the label is an immutable discriminator. One Workspace has at
most one Relation identity for one logical relation key. Removing the Relation
changes Work-State membership; adding the same logical relation later reuses
the identity and may select or create a new RelationVersion.

Canonical relation types retain Engine-enforced source kind, target kind,
Workspace, endpoint-presence, and deterministic algorithm semantics. Generic
ObjectIdentity endpoints never authorize arbitrary connections.

## Work-State membership and projections

For one Workspace Commit, canonical Work State is two mappings:

```text
EntityID   -> EntityVersionID | absent
RelationID -> RelationVersionID | absent
```

Presence or absence belongs to these mappings, not to an `active` or `deleted`
field on an EntityVersion or RelationVersion. Semantic terminal states such as
Task `done` or `superseded` are distinct from absence. Canonical identities and
versions are retained even when absent from current Work State.

Canonical Change Operations change the mappings through explicit before/after
membership conditions. The final primitive spelling remains Open, but the
precondition distinguishes expected absence from an expected version.

A normal Commit state is its parent mapping plus the recorded mapping delta. A
merge Commit stores the primary-parent-to-merged-state delta. Branch HEAD is
the Branch's only canonical current-state pointer.

`BranchEntityCurrent` and `BranchRelationCurrent` are complete materialized
HEAD projections, not authority. A Branch projection has explicit completeness
state and projected Commit. Missing rows mean absence only when the projection
is complete and its projected Commit equals HEAD. Before it is marked
complete, its state digest is checked against the HEAD Commit state digest.

For a materialized HOT Branch, an ordinary semantic mutation atomically writes
the new immutable history, compare-and-swap moves HEAD, and advances the
complete current projection. A non-materialized Branch may reconstruct a
transient base from Checkpoint plus replay without becoming HOT. Branch
creation does not copy a source projection. Import or fast-forward ref movement
may invalidate a projection instead of rebuilding it inside the ref movement.

Typed current projections are workload-driven accelerators, not one table per
Entity kind. They identify their source EntityVersion. Relation current
projection may contain verifiable denormalization for graph traversal. Its
absence or inconsistency never changes canonical history.

## Commit, ChangeSet, ChangeOperation, Event, and Branch

A WorkStateCommit belongs to one Workspace, not to a Branch. Multiple Branches
may point to it. Originating Branch, when retained, is provenance only.

Commit parent records have explicit roles:

- a Workspace Genesis Commit has zero parents and represents empty Work State;
- a normal Commit has one primary parent;
- a V1 merge Commit has primary target and secondary source parents;
- V1 has no octopus merge.

Every Workspace has exactly one Genesis Commit. Every Commit, including
Genesis, has exactly one non-reusable ChangeSet. Genesis may use an
initialization ChangeSet with zero Change Operations.

The ChangeSet is the occurrence container for the semantic operation
descriptor and rationale. It has at most one normalized before-to-after
transition for each Entity or Relation membership key. Change Operations may
have a stable rendering/serialization ordinal, but correctness is set-based:
all before conditions are checked against the parent mapping, a candidate
result is built, and global invariants are checked before commit. Before and
after versions must belong to the named logical subject.

Events are immutable provenance, not Change Operations and not replay truth.
One ChangeSet may have one or more Events, while Runtime or infrastructure
operations may emit Events with no ChangeSet or WorkStateCommit. Historical
Event correction uses later provenance or an explicit repair path, never a
silent normal update.

Branch is a stable identity plus a movable HEAD; its name is not its identity.
HEAD movement uses expected-head compare-and-swap, points only to a Commit in
the same Workspace, and always records immutable old/new HEAD provenance.
Branch creation creates a ref and Event but no empty Commit. Canonical Commit
retention does not depend only on current Branch reachability.

MergeRuntime writes no provisional Commit or ChangeSet. Successful
`continue` validates captured inputs and atomically creates canonical merge
history before moving target HEAD. Restore is a normal single-parent Commit
whose ChangeSet maps current state to the selected historical state. A
fast-forward import moves a ref to an already imported Commit and does not
invent an import Commit.

## Runtime and provenance families

### Session

Session is a stable ObjectIdentity-backed execution occurrence. SessionRuntime
is its current mutable coordination state and can disappear after the Session
ends without deleting Session identity or history.

Current Context Set membership uses separate Workspace and KnowledgeSpace
membership families. These rows do not carry history; Context add/remove Events
do. Focus uses a structured runtime family whose Entity and ordered context
path can be validated against the active Branch, not an opaque path string.

A normal Session has zero or one final immutable SessionDiff, which may
summarize multiple Workspace Commit ranges and runtime changes. Large payloads
may use ContentObject storage.

### Claim

Each ownership/mode period is a distinct ObjectIdentity-backed Claim
occurrence. Its immutable occurrence data is separate from the active
ClaimRuntime projection. Release, takeover, or mode change ends the old period;
takeover or a changed mode creates a new Claim occurrence.

For each Workspace, Branch, and Task, the active Claim set is exactly one of:
empty, one exclusive Claim, or one-or-more shared Claims. The exact index,
transaction, or lock mechanism enforcing that invariant remains Open.

### Merge

Each MergeAttempt is a stable ObjectIdentity-backed occurrence with immutable
captured Workspace, Branch, base, source head, and target head. MergeRuntime is
its current coordination projection. MergeItem and provisional resolution are
structured runtime children rather than an opaque blob. Completed and aborted
MergeAttempts, items, resolutions, and Events remain provenance after active
runtime is removed.

## Evidence and content objects

Evidence is an ObjectIdentity-backed immutable provenance object. It is not a
content blob. One Evidence may reference zero or more digest-identified
ContentObjects, and one ContentObject may be reused by multiple Evidence
objects.

ContentObject identity is its content digest and does not use ObjectIdentity.
Canonical digest metadata is distinct from storage backend and locator.
Archiving or losing a local storage location changes an availability/storage
projection; it does not delete digest metadata or sever Evidence provenance.

## Resource families

Resource is an ObjectIdentity-backed portable logical external-resource
identity. Resource kind is an immutable identity property; a fundamentally
different kind requires another Resource identity.

ResourceBinding contains current environment-local locator/configuration and
does not use ObjectIdentity or enter Workspace Work State. V1 permits at most
one active binding for one Resource in one Store runtime environment. Portable
import does not reactivate the source environment binding; it may retain the
old locator only as diagnostic provenance.

ResourceObservation is an immutable ObjectIdentity-backed observation. Large
manifests or diffs may use ContentObject storage.

Workspace and Resource have a many-to-many infrastructure association. The
association is Workspace-level configuration, not Branch Work State; its
changes produce provenance rather than WorkStateCommit. Association metadata
or labels are allowed, but the role taxonomy is not frozen.

## Verification persistence boundary

VerificationBasis is an immutable component owned by one Verification. It is
not an Entity or ObjectIdentity. Resource-basis components and Work-State
semantic dependencies remain structurally distinguishable and queryable rather
than being only an opaque document.

Verification applicability is a derived/cache projection and never a canonical
Verification field.

The immutable defining judgment closure of a Verification includes:

- result;
- the one AcceptanceCriterion or VerificationRequirement target;
- VerificationBasis;
- Evidence set and its `evidenced_by` defining edges;
- method and other judgment-owned semantic state.

The `verifies` and `evidenced_by` edges use the common Relation infrastructure
but are immutable creation-closure edges for that Verification. Changing any
member of the defining closure creates a new Verification rather than mutating
the prior judgment. Branch-sensitive applicability remains derived and may
change without changing that closure.

## Knowledge federation families

KnowledgeSpace and KnowledgeExposure are ObjectIdentity-backed Store-local
federation objects outside Workspace Work State. Exposure source Store,
Workspace, Knowledge, and KnowledgeVersion binding is immutable.

Exposure lifecycle is represented by append-only transitions plus a current
projection rather than updating the immutable Exposure occurrence. Source
staleness is a separate derived projection and never an automatic Exposure
transition. Explicit Knowledge adoption uses the ordinary Workspace Knowledge
Entity and a `derived_from` Relation to the local KnowledgeExposure; it does not
need a special adopted-Knowledge family.

`ExternalObjectRef` represents an unresolved or foreign stable provenance
reference and is not a local ObjectIdentity. It is permitted only in explicitly
allowed federation provenance, external lineage, and imported provenance
metadata. It cannot be a canonical Relation endpoint or a cross-Store Work
Graph dependency escape hatch.

When a Bundle includes an object whose canonical Relation points to another
Store-local object, the exporter includes the Relation and endpoint object in
the required local reference closure. A compact Bundle that leaves such a
local canonical endpoint dangling is invalid. The included endpoint's own
allowed source provenance may use ExternalObjectRef when its foreign source is
not included.

## Store, Checkpoint, Bundle, and import infrastructure

The current Store manifest declares Store, schema, object-store format, and
capability information. It is current infrastructure state and may change
during migration. Each migration remains immutable auditable provenance; the
manifest does not need a second version DAG.

ContentObject metadata and storage location/backend are separate. Checkpoint
has an independent typed identity outside ObjectIdentity and references one
Workspace Commit and one state representation. A Commit may have zero or more
Checkpoints for different compatible formats or repairs. Checkpoint corruption
makes that Checkpoint unusable/rebuildable; it does not prove canonical Commit
corruption unless canonical replay also fails its integrity check.

A Bundle is a transport artifact, not automatically a long-lived Store Entity.
Its BundleManifest is distinct from StoreManifest and declares source Store,
profile, included scopes, root refs, required objects, formats, and integrity
metadata for that Bundle.

Every import attempt is immutable infrastructure provenance. Import may ingest
immutable candidate data in batches, then validates identity, content,
formats, DAG, and refs before atomically activating accepted refs. Failure may
leave unreferenced immutable candidates eligible for later policy-governed
collection, but it cannot partially move current Branch state.

Immutable import is idempotent: an existing immutable identity with equal
content is a no-op; the same identity with unequal content is an integrity
conflict and is never updated in place.

## Current implementation boundary

The logical-family model above is Confirmed. Decisions 443-566 subsequently
closed the V1 ID/digest encodings, core SQLite type and foreign-key policy,
canonical JSON storage boundary, writer-transaction baseline, bounded Physical
DDL constraints, executable schema assembly, bootstrap/open validation, and
integrity classification. ADR-0175 closes the final equal-candidate stable
tie-breaker for `next` as EntityId byte order. The following remain Open:

- final CLI spelling and protocol encoding;
- concrete performance indexes and query plans;
- serializer replacement policy beyond the accepted `workvcs-jcs-v1` Rust
  implementation;
- checkpoint creation, retention, selection, and eviction strategy;
- the exact number and shape of typed projection tables;
- Knowledge exchange/access API and authorization protocol;
- cross-Store live federation, subscriptions, and distributed synchronization;
- storage-layout tuning beyond the confirmed `BEGIN IMMEDIATE`, WAL-policy,
  and controlled-writer baseline.

Rust, `rusqlite`, and the `workvcs-jcs-v1` canonical JSON profile are closed by
ADR-0008, ADR-0009, and
[Implementation Contract v0.1](implementation-contract-v0.1.md).

The closed physical contract and its remaining boundary are authoritative in
[Physical Schema v0.1 Contract](physical-schema-v0.1.md).
