# Physical Schema v0.1 Contract

This document freezes the confirmed SQLite Physical DDL decisions for WorkVCS
v0.1. It realizes
[ADR-0006](../decisions/adr/0006-sqlite-physical-schema-v0.1.md) and is bounded
by the logical authority model in
[Logical Schema Boundaries](logical-schema-boundaries.md).

This is the normative physical contract, not the assembled executable schema.
`schema-v0.1.sql` does not yet exist. Exact table ordering, the complete
foreign-key graph, and executable empty-database validation belong to the next
assembly stage.

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

## Schema Assembly Review closure

**Physical DDL Schema Assembly Review = PASS / CLOSED.**

The review found no second Work-State authority. Canonical state remains
Branch HEAD to Commit DAG to ChangeSet to membership ChangeOperations;
projections and caches remain rebuildable; Runtime and immutable provenance
remain separate; Verification target/Evidence retain one Relation authority;
and cross-Store canonical-history import remains prohibited.

The next stage is complete `schema-v0.1.sql` Assembly, not another round of
isolated table additions. That stage must:

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

## Still Open

- the complete executable `schema-v0.1.sql`, creation order, and full
  DDL-to-Engine invariant matrix;
- performance/query indexes and workload evidence;
- the exact canonical JSON profile rules, serializer implementation, and
  programming language/SQLite binding;
- checkpoint scheduling, retention, selection, and eviction policy;
- the workload-driven set of typed current projections;
- Resource path/glob/symlink normalization details beyond the confirmed
  versioned Adapter/Scope contract;
- Bundle container/profile/compression details, Knowledge access/security and
  exchange APIs, and cross-Store live federation;
- final CLI/protocol spelling and unrelated Open product decisions.
