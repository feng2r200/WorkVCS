# Physical Schema v0.1 Contract

This document freezes the confirmed SQLite Physical DDL decisions for WorkVCS
v0.1. It realizes
[ADR-0006](../decisions/adr/0006-sqlite-physical-schema-v0.1.md) and is bounded
by the logical authority model in
[Logical Schema Boundaries](logical-schema-boundaries.md).

This is the normative physical contract realized by the assembled executable
schema in [schema-v0.1.sql](../../schema/schema-v0.1.sql). The install and
bootstrap validation harness is
[validate-schema-v0.1.sh](../../scripts/validate-schema-v0.1.sh).

The executable schema contains the confirmed 67 v0.1 tables and correctness
indexes only. It deliberately does not add performance/query indexes, new table
families, or storage-engine business implementation.

## Predecessor closure carried forward

The Physical DDL decisions depend on the confirmed Full DDL Consolidation
closure:

- Verification Basis has distinct immutable Resource, Artifact/Input, and
  semantic-dependency components.
- AcceptanceCriterion and VerificationRequirement retain immutable,
  owner-scoped local identity; Record kind is immutable identity metadata.
- KnowledgeExposure history is an immutable linear predecessor chain, Store
  fork lineage has an infrastructure-provenance home, and full canonical
  history import is restricted to the same Store namespace.
- Workspace and Genesis Commit retain a non-null committed-state cycle that
  must support transaction-deferred construction.
- SessionEnd removes all current Session, Focus, Context Set, and owned Claim
  runtime rows while preserving occurrences, Events, and SessionDiff.

These constraints preserve the earlier logical model. They do not add a
second Work-State authority or reopen decisions 363–442.

## SQLite feature and encoding baseline

V1 physical tables use SQLite `STRICT` mode across canonical history, runtime,
projection, and infrastructure metadata. Runtime startup checks the required
SQLite feature baseline rather than assuming any system `sqlite3` is suitable.
The baseline includes strict tables, enforced and deferred foreign keys, JSON
validation functions, and WAL capability; the implementation binding and
exact minimum SQLite version remain implementation work.

Every Engine connection explicitly enables and verifies
`PRAGMA foreign_keys = ON`. Foreign keys are immediate by default. Only a real
construction cycle, including Workspace to Genesis Commit, uses
`DEFERRABLE INITIALLY DEFERRED`; ordinary operation does not rely on the
connection-wide `defer_foreign_keys` pragma.

Canonical physical encodings are:

| Domain | V1 physical form | Constraint |
|---|---|---|
| Stable logical ID | UUIDv7 in a 16-byte `BLOB` | `length(...) = 16` |
| Content, state, and Resource fingerprint | BLAKE3-256 in a 32-byte `BLOB` | `length(...) = 32` |
| Timestamp | UTC Unix epoch microseconds in `INTEGER` | Never causal or ordering authority |
| Canonical structured payload | canonical UTF-8 JSON in `TEXT` | `json_valid(...)`; semantic schema and canonical bytes are Engine responsibilities |
| Controlled vocabulary | symbolic `TEXT` | Closed structural values use SQL `CHECK`; extensible semantic values use the Engine registry |

Logical IDs and digests are separate domains and use explicit names such as
`object_id`, `state_digest`, and `content_digest`. Short Agent-facing IDs are
derived from kind plus UUID prefix and are never persisted as canonical
identity.

Canonical JSON does not use SQLite JSONB as durable history. A Store Manifest
declares the canonical JSON profile, while the WorkVCS serializer—not SQLite
`json()` output—defines key order, Unicode handling, numeric representation,
escaping, and omitted/null semantics. The exact profile rules remain part of
schema assembly and serializer specification; the storage and authority split
is closed.

WAL is the default operational journal policy, not a Store-format invariant.
Canonical mutation transactions default to explicit `BEGIN IMMEDIATE`.
WorkVCS may use multiple readers, while physical writes use controlled,
serialized SQLite writer transactions. Engine OCC resolves logical races;
SQLite writer serialization provides physical atomicity. `busy_timeout` is
operational tuning and never supplies correctness.

## Store format and schema responsibilities

StoreManifest records Store-format, schema, and object-store-format versions
plus `id_scheme`, `digest_algorithm`, and `canonical_json_profile`, so a Store
can be interpreted and migrated independently. One SQLite database contains
exactly one Store through Engine open/init validation; v0.1 does not add a
singleton surrogate key or complex trigger for that fact.

Closed structural vocabularies are enforced in DDL. Extensible semantic
vocabularies remain non-null symbolic text validated by the Engine, and v0.1
does not add persistent lookup/registry tables for them. EntityVersion and
RelationVersion do not store `created_at`; creation occurrence and time belong
to ChangeSet, Event, and Commit provenance.

V0.1 does not generate broad UPDATE/DELETE guard triggers for immutable rows.
The Engine write layer and tests enforce insert-only history, while SQLite
provides primary keys, foreign keys, nullability, uniqueness, and bounded
checks. Simple immutable guard triggers may be considered later as a hardening
migration without changing the data model.

## Identity, names, and long-lived refs

Branch and KnowledgeSpace names use exact, case-sensitive UTF-8 identity;
`main` and `Main` may coexist. Branch names are non-empty, contain no NUL or
ASCII control characters, have no leading/trailing whitespace, and may contain
ordinary UTF-8 characters including `/`; Work Branch names do not copy the
full Git-ref grammar.

Physical DDL enforces at least `length(name) > 0` and
`length(local_key) > 0` for Branch, KnowledgeSpace, AcceptanceCriterion local
keys, and VerificationRequirement local keys. The Engine applies the remaining
Unicode-safe name rules. Branch/KnowledgeSpace keep their confirmed uniqueness
scope; AC/VR keys remain owner-scoped, immutable, case-sensitive, and do not
renumber when siblings disappear or reorder.

Branch lifecycle state and similar extensible semantic vocabularies remain
Engine-validated rather than frozen as SQL enums. Canonical Branch HEAD-move
history uses immutable Events; no second canonical reflog/history table is
introduced. Generic ChangeSet causal anchors reference only local
ObjectIdentity values, never ExternalObjectRef. Branch identity rows are
long-lived; a future ref-removal operation may archive or release a current
name but does not normally physically delete the Branch and sever provenance.

## Runtime and immutable provenance

Merge item classification is the closed structural vocabulary `AUTO`,
`CONFLICT`, and `REVIEW`. Merge resolution kind is the closed structural
vocabulary `ours`, `theirs`, and `custom`. A custom resolution requires a
custom payload; `ours` and `theirs` forbid one. The same strict payload rule
applies to provisional and frozen resolution rows.

Each MergeItem has at most one immutable frozen MergeResolution after the
MergeAttempt ends. Provisional choice changes remain mutable Runtime state and
are preserved through Events. SessionEnd, Claim release, and Merge completion
perform explicit Engine cleanup rather than using cascading deletion to hide
business semantics.

Active Claim-set exclusivity and the one-active-Merge-per-target-Branch rule
are checked by the Engine in `BEGIN IMMEDIATE` transactions. Runtime rows do
not duplicate canonical owner fields only to enable partial unique indexes.
SessionRuntime v0.1 has no generation/version column; a later generation model
requires demonstrated runtime CAS/ABA need.

Pure cache/projection parent-child rows may use `ON DELETE CASCADE`, because
their children have no independent history. This exception does not apply to
Runtime business transitions or immutable provenance.

## Verification, Resource, and applicability

Persisted ResourceObservation records `adapter_kind` and a positive
`adapter_schema_version`; Verification Resource Basis additionally records a
positive `scope_schema_version`. These versions identify observation and scope
normalization contracts, not software package versions.

The Adapter normalizes the observation/scope representation. WorkVCS hashes
the canonical bytes using the Store Manifest `digest_algorithm`; V1 uses
BLAKE3-256. ResourceObservation fingerprint, Verification Resource Basis
baseline fingerprint, and Applicability Resource Stamp observed fingerprint
therefore share the 32-byte digest representation without a per-row algorithm.

Verification target and defining Evidence continue to use the canonical
`verifies` and `evidenced_by` Relation infrastructure. No duplicate
`verification_target` or `verification_evidence` canonical tables are added.
At Verification creation, the Engine validates exactly one target of kind
AcceptanceCriterion or VerificationRequirement, a complete Basis, a fixed
defining Evidence relation set, and presence of every defining relation in the
resulting Work State. Ordinary Work-State mutation cannot change this defining
closure; re-verification creates a new Verification.

Applicability uses the closed derived vocabulary `applicable`, `stale`, and
`unknown`. A Resource Stamp records Adapter and scope contract versions in
addition to its fingerprint and has a composite foreign key to the actual
`(verification_entity_id, resource_basis_ordinal)` Resource Basis. An
Applicability Cache may cascade-delete its Resource Stamps. V0.1 does not
materialize a separate AC Effective Status cache; it remains a deterministic,
workload-driven projection.

## Knowledge federation and external references

V1 ExternalObjectRef has explicit `reference_scope` of `object` or `version`.
Object scope has no version reference; version scope has one 16-byte WorkVCS
version identity. V1 does not introduce arbitrary external identifier
encoding. A KnowledgeExposure external source requires version scope and
`object_kind = knowledge`.

Every committed KnowledgeExposure has exactly one source family: local or
external. Its transition history is an immutable linear predecessor chain;
the predecessor is unique, cannot self-reference, belongs to the same
Exposure, and cannot form a cycle. Each Exposure has one initial transition
and one current linear head. Timestamps never select the head.

V1 Exposure semantic lifecycle is the closed vocabulary `active` and
`withdrawn`. Replacement uses a new Exposure plus explicit withdrawal of the
old Exposure; V1 adds no federation-level `superseded` state or relation.
Source-status cache uses the closed derived vocabulary `current`, `stale`,
`unknown`, and `unresolved`.

Record subtype identity uses immutable Record kind metadata. Canonical
`derived_from` endpoint validation permits Workspace Knowledge to derive from
a local KnowledgeExposure; no adopted-Knowledge subtype or special table is
introduced.

## Projection, Checkpoint, import, migration, and lineage

`not_materialized` Branch projection state has no projected Commit or digest.
`invalid` may retain stale Commit/digest diagnostics and stale current rows,
but readers never treat them as HEAD state. V0.1 projection rebuild uses a
temporary/shadow build, digest validation, and atomic publish rather than a
permanent generation column.

Checkpoint status is a pure derived child and may cascade-delete when a
Checkpoint is garbage-collected. ImportAttempt stores an immutable Bundle
transport-artifact digest without requiring the Bundle itself to be registered
as a ContentObject. Canonical import accepts full history only when the source
Store ID equals the local Store ID; a different Store is opened separately,
explicitly forked, or selectively adopted through Knowledge/Evidence and
ExternalObjectRef provenance.

StoreMigrationAttempt v0.1 structurally records Store-format and schema
from/to versions plus tool version. The complete manifest delta belongs to
immutable outcome/detail provenance rather than duplicated Manifest columns.
StoreLineage has independent infrastructure identity and records source Store,
derivation kind, source-root descriptor, optional source Bundle digest, and
creation time; it does not expand the Store row.

BundleManifest remains part of the Bundle transport format and is not a Store
SQLite table. Projection and cache repair may physically delete and rebuild
derived rows without a WorkStateCommit; ordinary cache rebuild does not emit
an Event unless it represents an actual corruption/repair incident.

ContextPacketSnapshot is immutable runtime provenance for an exact resolved
context packet. It records the Session, Workspace, Branch, head Commit, state
digest, profile, optional budget, optional scope, counts, packet digest,
canonical packet JSON, and creation time. It is not a WorkState, Branch, Event,
Claim, or relation authority. The Engine verifies the packet digest against the
stored canonical packet JSON when loading a snapshot and rejects rows whose
denormalized columns disagree with the packet JSON envelope or counts.

## Schema Assembly Review closure

**Physical DDL Schema Assembly Review = PASS / CLOSED.**

The review found no second Work-State authority. Canonical state remains
Branch HEAD to Commit DAG to ChangeSet to membership ChangeOperations;
projections and caches remain rebuildable; Runtime and immutable provenance
remain separate; Verification target/Evidence retain one Relation authority;
and cross-Store canonical-history import remains prohibited.

The subsequent stage assembled the full `schema-v0.1.sql`, instead of adding
another round of isolated tables. ADR-0007 closed that stage. The assembly
closure had to:

1. assemble every confirmed physical family into one SQLite schema;
2. resolve creation order and the deferred Workspace/Genesis cycle;
3. execute against an empty database with foreign keys enabled;
4. run `PRAGMA foreign_key_check`;
5. test Genesis initialization, canonical membership mutation, Runtime
   transactions, projection rebuild, and import/federation constraints; and
6. document every Engine-only invariant beside the DDL.

Performance indexes are deliberately not designed in this contract. Index
design follows executable schema validation and the core `context`, `next`,
`why`, `history`, `diff`, and `merge` query contracts.

## Executable schema assembly closure

**Schema v0.1 Assembly = PASS / CLOSED.**

Decisions 514-566 close the executable-schema assembly, install/bootstrap, and
integrity boundary. The assembled schema:

- separates durable schema DDL from per-connection bootstrap checks such as
  `PRAGMA foreign_keys = ON`;
- uses a readable creation order while relying on a single real deferred cycle:
  `Workspace.genesis_commit_id -> WorkStateCommit.commit_id`;
- adds immutable `merge_attempt_outcome` rows for completed and aborted merge
  attempts;
- removes `merge_resolution.finalization_kind`;
- enforces the ExternalObjectRef object/version partial uniqueness and the
  one-initial-Exposure-transition partial uniqueness as correctness indexes;
- permits Applicability Resource Stamps to record unavailable/error
  observations with nullable observed fingerprint and observation reference;
- requires Branch projection Commit and digest to be present or absent as a
  pair; and
- retains exactly the confirmed physical encodings: SQLite `STRICT`,
  UUIDv7/BLOB16 identities, BLAKE3-256/BLOB32 digests and Resource
  fingerprints, canonical JSON TEXT plus `json_valid`, symbolic TEXT
  vocabularies, UTC epoch microseconds, and StoreManifest format parameters.

The schema uses SQL constraints for structural facts SQLite can express. Exact
family ownership, ChangeOperation typed-child exclusivity, relation endpoint
kind/workspace contracts, merge result-shape validation, Verification defining
closure, and candidate Work-State validation remain Engine validator contracts.

## Constraint and transaction responsibility closure

**Constraint / Transaction Model = CLOSED.**

The responsibility split is:

1. SQLite enforces primary keys, foreign keys, nullability, local uniqueness,
   partial uniqueness, JSON syntax, bounded BLOB lengths, and closed structural
   vocabularies.
2. Engine preflight validates typed-family exactness, semantic relation
   endpoints, candidate Work-State invariants, Verification closure, import
   closure, and merge parent/result shape.
3. Canonical semantic mutations run in explicit versioned transactions and
   create no empty commits except Workspace Genesis.
4. Runtime coordination transactions update mutable runtime rows, immutable
   occurrence/outcome records, cleanup, and Events without creating
   WorkStateCommits unless they complete a canonical semantic mutation.
5. Branch HEAD movement always uses expected-head compare-and-swap; `BEGIN
   IMMEDIATE` supplies writer serialization but does not replace logical CAS.

Events are required provenance for state changes but are not replay authority
or semantic ordering authority. Ordinary failed transactions leave no Event
unless the failure is itself modeled as an occurrence/outcome record.

## Install, open, compatibility, and integrity closure

**Compatibility / Integrity = CLOSED.**

Schema installation sets a fixed `application_id` and does not use
`PRAGMA user_version` as a second schema authority. A Store open path separates
bootstrap validation from deep validation:

- bootstrap validation checks `application_id`, per-connection FK enforcement,
  exactly one Store and StoreManifest, supported Store/schema/object-store
  versions, ID scheme, digest algorithm, and canonical JSON profile;
- deep validation checks SQLite physical integrity, structural referential
  integrity, canonical history integrity, derived-state integrity, DAG shape,
  replay/digest behavior, exact-family contracts, projections, and doctor
  diagnostics.

Newer-than-supported Store or schema format fails closed. Older Stores require
an explicit migration operation rather than silent ordinary-open migration.
Derived projection/cache corruption may be discarded and rebuilt without a
WorkStateCommit. Canonical history corruption fails closed and requires an
explicit doctor/recovery path. A `ContentObject` with zero storage locations is
an availability condition, not by itself a canonical integrity failure.

## Still Open

- performance/query indexes and workload evidence;
- final object layout and serializer replacement policy beyond the accepted
  `workvcs-jcs-v1` Rust implementation;
- checkpoint scheduling, retention, selection, and eviction policy;
- the workload-driven set of typed current projections;
- Resource path/glob/symlink normalization details beyond the confirmed
  versioned Adapter/Scope contract;
- packaged Bundle container/compression details, Knowledge access/security and
  exchange APIs, and cross-Store live federation;
- final CLI/protocol spelling and unrelated Open product decisions.

Rust, `rusqlite`, the `workvcs-jcs-v1` canonical JSON profile, and executable
schema assembly are no longer Open implementation choices; they are closed by
ADR-0007 through ADR-0009 and
[Implementation Contract v0.1](implementation-contract-v0.1.md).
