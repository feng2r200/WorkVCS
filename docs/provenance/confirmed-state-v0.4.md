# Confirmed State v0.4 Provenance

This document records promotion evidence for WorkVCS decisions 443–513: the
SQLite Physical DDL baseline, bounded physical constraints, and the closing
Schema Assembly Review. It is an audit ledger, not a parallel normative
specification. Normative authority is
[ADR-0006](../decisions/adr/0006-sqlite-physical-schema-v0.1.md) and
[Physical Schema v0.1 Contract](../architecture/physical-schema-v0.1.md).

## Source and promotion authority

The sequence was discussed and explicitly confirmed in the ChatGPT
conversation `WorkVCS需求沟通-v1`
(`6a847cdb-7f00-83e8-8870-a8acee174a4c`). The current user request authorizes
promotion of final confirmed decisions 443–513, requires compatibility with
the earlier 363–442/398–442/435–442 conclusions, forbids new architecture
decisions, and makes complete `schema-v0.1.sql` Assembly the next stage.

Conversation data is supporting provenance. Only the numbered conclusion
explicitly accepted by the user is promoted; surrounding examples and draft
`CREATE TABLE` text are not independently promoted by proximity.

## Repository-baseline correction

The repository baseline before this update contained numbered provenance
through decision 318. It did not contain a separate numbered ledger for
363–442. This update therefore does not claim that such a ledger already
existed. It carries forward the 435–442 Full DDL Consolidation closure because
those decisions are direct prerequisites of 443–513, preserves the prior
single-authority logical model, and records the discrepancy here rather than
silently fabricating earlier repository history.

Predecessor confirmation turns are:

| Decision range | User confirmation turn | Role in this promotion |
|---|---|---|
| 363–397 | `7adcfd49-945a-4d2e-b674-ed780597a38f` | Prior Logical DDL baseline; not separately re-promoted here |
| 398–434 | `fd36ce7d-ec6a-4fcb-86aa-eed0c5b69a0f` | Prior projection/import/consolidation baseline; not separately re-promoted here |
| 435–442 | `0fd4e955-2a45-45d9-a130-08f95a414e96` | Direct predecessor closure carried into the Physical contract |

## Confirmation turns

| Decision range | User confirmation turn | Response |
|---|---|---|
| 443–462 | `e64d57e6-aedc-4b30-8b74-112034009560` | `443是 ... 462是` |
| 463–468 | `4e6d5b38-0331-4397-99f5-ce50e8a1d0fd` | `463是 ... 468是` |
| 469–474 | `596e1544-ff51-4e7f-8fab-8cfd66f30a7f` | `469是 ... 474是` |
| 475–480 | `00d13235-4a2b-4361-81db-5a64a2ab36b6` | `475是 ... 480是` |
| 481–488 | `46084618-f6e8-4178-beb7-8fb4cb5826d0` | `481是 ... 488是` |
| 489–498 | `9d15b983-97c2-4e34-88b2-75979b0e30ca` | `489是 ... 498是` |
| 499–506 | `7e3df5ef-a05a-441b-94d9-451f9041ce37` | `499是 ... 506是` |
| 507–513 | `9fe613e6-773f-4452-a597-68fcebef79bc` | `507是 ... 513是`; review closed |

All 71 integers from 443 through 513 are represented below exactly once.

## Decision-to-authority map

### SQLite baseline and encodings

| ID | Confirmed conclusion | Normative authority |
|---:|---|---|
| 443 | V1 physical tables use SQLite `STRICT` mode. | [SQLite feature and encoding baseline](../architecture/physical-schema-v0.1.md#sqlite-feature-and-encoding-baseline) |
| 444 | Runtime validates a SQLite feature baseline; it does not assume any system SQLite or yet freeze an exact version. | Same |
| 445 | Every Engine connection explicitly enables and verifies foreign keys. | Same |
| 446 | Foreign keys are immediate by default and deferred only for real construction cycles. | Same |
| 447 | Ordinary operation does not depend on global `defer_foreign_keys`. | Same |
| 448 | Canonical structured payload uses UTF-8 JSON text, not SQLite JSONB. | Same |
| 449 | WorkVCS serializer, not SQLite JSON output, defines canonical bytes. | Same |
| 450 | SQLite validates JSON syntax; the Engine validates schema semantics. | Same |
| 451 | Controlled vocabulary is stored as symbolic text rather than integer ordinals. | Same |
| 452 | Canonical timestamps are UTC Unix epoch microseconds in integers. | Same |
| 453 | Timestamps never determine causal or logical order. | Same |
| 454 | Stable logical IDs are UUIDv7 stored as 16-byte BLOBs. | Same |
| 455 | Logical IDs and digests are distinct physical domains. | Same |
| 456 | Content/state digests use BLAKE3-256 in 32-byte BLOBs and the Store format declares the algorithm. | Same |
| 457 | ID and digest BLOBs carry length checks. | Same |
| 458 | Short Agent-facing IDs are derived display values, not persisted canonical identity. | Same |
| 459 | WAL is default operational policy, not a Store-format invariant. | Same |
| 460 | Canonical mutations default to explicit `BEGIN IMMEDIATE`. | Same |
| 461 | Many readers and controlled writer transactions coexist; Engine OCC and SQLite serialization have different duties. | Same |
| 462 | `busy_timeout` is tuning and does not supply correctness. | Same |

### Store format, vocabulary, and names

| ID | Confirmed conclusion | Normative authority |
|---:|---|---|
| 463 | StoreManifest records identity, digest, and canonical JSON profile parameters in addition to format versions. | [Store format and schema responsibilities](../architecture/physical-schema-v0.1.md#store-format-and-schema-responsibilities) |
| 464 | Closed structural vocabularies use SQL checks; extensible semantic vocabularies use Engine validation. | Same |
| 465 | Exactly one Store per SQLite database is an Engine open/init invariant rather than a singleton-key/trigger design. | Same |
| 466 | Canonical JSON columns use non-null text plus JSON validity checks; canonical bytes and semantic validity remain Engine duties. | Same |
| 467 | EntityVersion and RelationVersion omit creation time; occurrence time belongs to provenance. | Same |
| 468 | V0.1 adds no persistent registry tables for extensible semantic vocabularies. | Same |
| 469 | Branch/KnowledgeSpace names use exact, case-sensitive UTF-8 identity. | [Identity, names, and long-lived refs](../architecture/physical-schema-v0.1.md#identity-names-and-long-lived-refs) |
| 470 | Branch names have the confirmed minimal syntax and do not copy full Git-ref rules. | Same |
| 471 | Extensible lifecycle vocabularies remain Engine-validated rather than SQL-enumerated. | Same |
| 472 | Immutable Events are the sole canonical Branch HEAD-move provenance; no second reflog table is added. | Same |
| 473 | Generic ChangeSet causal anchors reference local ObjectIdentity only. | Same |
| 474 | Branch identity rows remain long-lived and are not normally physically deleted. | Same |

### Runtime and merge

| ID | Confirmed conclusion | Normative authority |
|---:|---|---|
| 475 | MergeItem classification is the closed structural vocabulary `AUTO`/`CONFLICT`/`REVIEW`. | [Runtime and immutable provenance](../architecture/physical-schema-v0.1.md#runtime-and-immutable-provenance) |
| 476 | Merge resolution kind is the closed structural vocabulary `ours`/`theirs`/`custom`. | Same |
| 477 | One MergeItem has at most one immutable frozen final resolution; provisional history remains in Events. | Same |
| 478 | Runtime cleanup is explicit Engine behavior, not hidden business cascade. | Same |
| 479 | Active Claim and active-Merge uniqueness remain `BEGIN IMMEDIATE` Engine invariants without duplicated owner fields. | Same |
| 480 | SessionRuntime v0.1 has no generation/version field. | Same |

### Verification, Resource, and applicability

| ID | Confirmed conclusion | Normative authority |
|---:|---|---|
| 481 | Persisted ResourceObservation records Adapter schema-contract version. | [Verification, Resource, and applicability](../architecture/physical-schema-v0.1.md#verification-resource-and-applicability) |
| 482 | Verification Resource Basis records scope-normalization contract version. | Same |
| 483 | Verification target/Evidence reuse canonical Relations; no duplicate canonical tables are introduced. | Same |
| 484 | Applicability `applicable`/`stale`/`unknown` is a closed derived vocabulary. | Same |
| 485 | Pure cache/projection owner-child rows may use cascade deletion. | Same |
| 486 | Applicability Resource Stamps record Adapter and scope contract versions as well as fingerprints. | Same |
| 487 | V0.1 does not materialize a separate AC Effective Status cache. | Same |
| 488 | AC/VR local keys are immutable, case-sensitive, owner-scoped identities and do not renumber. | Same |

### Knowledge federation and external references

| ID | Confirmed conclusion | Normative authority |
|---:|---|---|
| 489 | V1 external version references are limited to 16-byte WorkVCS version identity. | [Knowledge federation and external references](../architecture/physical-schema-v0.1.md#knowledge-federation-and-external-references) |
| 490 | ExternalObjectRef distinguishes object and version scope with matching version-reference nullability. | Same |
| 491 | KnowledgeExposure external source requires version scope and Knowledge kind. | Same |
| 492 | Each committed KnowledgeExposure has exactly one local or external source family. | Same |
| 493 | Exposure transitions form an immutable linear predecessor chain and do not use time for ordering. | Same |
| 494 | V1 Exposure lifecycle is `active`/`withdrawn`; replacement is new Exposure plus old withdrawal. | Same |
| 495 | Exposure current semantic state uses a closed `active`/`withdrawn` check. | Same |
| 496 | Exposure source-status cache uses `current`/`stale`/`unknown`/`unresolved`. | Same |
| 497 | Record subtype identity stores immutable Record kind metadata. | Same |
| 498 | `derived_from` permits Knowledge derived from KnowledgeExposure without a special adopted-Knowledge family. | Same |

### Projection, Checkpoint, import, migration, and lineage

| ID | Confirmed conclusion | Normative authority |
|---:|---|---|
| 499 | `not_materialized` projection has no Commit/digest; `invalid` may retain unreadable stale diagnostics. | [Projection, Checkpoint, import, migration, and lineage](../architecture/physical-schema-v0.1.md#projection-checkpoint-import-migration-and-lineage) |
| 500 | V0.1 projection rebuild uses shadow build, digest validation, and atomic publish rather than a permanent generation field. | Same |
| 501 | Checkpoint status is a derived child that may cascade-delete with Checkpoint GC. | Same |
| 502 | ImportAttempt Bundle digest is transport-artifact identity and does not require a ContentObject row. | Same |
| 503 | StoreMigrationAttempt structures format/schema from/to plus tool version; complete manifest delta stays in immutable outcome detail. | Same |
| 504 | StoreLineage has separate infrastructure identity and source-root context rather than expanding Store. | Same |
| 505 | BundleManifest is transport metadata, not a Store SQLite table. | Same |
| 506 | Projection/cache repair may physically rebuild without WorkStateCommit or ordinary rebuild Events. | Same |

### Assembly review refinements and closure

| ID | Confirmed conclusion | Normative authority |
|---:|---|---|
| 507 | Resource fingerprints use the Store digest algorithm; Adapter/scope versions define normalization. | [Verification, Resource, and applicability](../architecture/physical-schema-v0.1.md#verification-resource-and-applicability) |
| 508 | Exposure predecessor is physically non-forking and non-self-referential; remaining linear-head rules are Engine invariants. | [Knowledge federation and external references](../architecture/physical-schema-v0.1.md#knowledge-federation-and-external-references) |
| 509 | Frozen resolution is unique per MergeItem and custom-payload nullability is exact in runtime and frozen rows. | [Runtime and immutable provenance](../architecture/physical-schema-v0.1.md#runtime-and-immutable-provenance) |
| 510 | Applicability Resource Stamp has a composite foreign key to the actual Verification Resource Basis. | [Verification, Resource, and applicability](../architecture/physical-schema-v0.1.md#verification-resource-and-applicability) |
| 511 | Human-addressable names/local keys have a DDL non-empty check plus Engine-level full syntax validation. | [Identity, names, and long-lived refs](../architecture/physical-schema-v0.1.md#identity-names-and-long-lived-refs) |
| 512 | V0.1 immutable rows use Engine write-layer/tests rather than a large UPDATE/DELETE trigger set. | [Store format and schema responsibilities](../architecture/physical-schema-v0.1.md#store-format-and-schema-responsibilities) |
| 513 | Verification single-target and immutable closure remain Engine invariants over one Relation authority. | [Verification, Resource, and applicability](../architecture/physical-schema-v0.1.md#verification-resource-and-applicability) |

## Supersession and conflict resolution

- Decision 507 narrows the earlier wording around decision 481: Adapter and
  scope contracts define normalization, but the persisted fingerprint algorithm
  is the Store Manifest digest algorithm, BLAKE3-256 in V1.
- ADR-0006 supersedes older Open-list statements only for the physical choices
  explicitly closed by 443–513. Historical ADRs remain evidence of what was
  Open when accepted.
- No executable schema, storage-engine code, performance index, or unnumbered
  draft SQL is promoted by this ledger.

## Review closure and next stage

**Physical DDL Schema Assembly Review = PASS / CLOSED.**

The next stage is complete `schema-v0.1.sql` Assembly followed by empty-DB
execution, foreign-key checking, Genesis, canonical mutation, Runtime,
projection rebuild, and import/federation validation. Performance index design
follows executable schema and core query-contract evidence.
