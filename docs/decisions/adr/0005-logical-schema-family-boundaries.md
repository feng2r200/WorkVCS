# ADR-0005: Logical Schema Family Boundaries

- **Status:** Accepted
- **Accepted by:** explicit user confirmations for decisions 189–318
- **Confirmation turns:** `ba0b8f79-229b-4ba5-883c-ec7fc2e40b2e`,
  `b22f7f25-0126-4899-bd4a-1e69d4362498`,
  `1664af5e-c02f-4387-a3d7-b7164532e889`,
  `7899ee56-190d-494a-82b2-a67d8a0c9bea`,
  `977e8720-e69e-415b-ac3b-949e43100433`,
  `94203023-df0c-4c6a-aa38-254aa0659059`, and
  `f7ee9ed5-77cb-45d5-a4ab-5cb12bc751bf`

**Subsequent resolution:**
[ADR-0006](0006-sqlite-physical-schema-v0.1.md) later closes the V1 Physical
DDL choices for ID/digest encodings, JSON/timestamps/vocabularies, SQLite
foreign-key and writer policy, and bounded runtime/projection/federation
constraints. The `Deliberately not decided` list below is historical for
ADR-0005 and remains Open only where ADR-0006 explicitly leaves it Open.

## Context

ADR-0001 through ADR-0004 confirmed WorkVCS semantic behavior, canonical
history, federation, and Store portability while deliberately leaving physical
schema choices Open. Decisions 189–318 then classified the logical families,
their ownership, immutability, membership, projection, runtime, provenance,
resource, federation, transport, and reference-closure responsibilities.

The purpose of this ADR is to record that material architecture decision
without treating the next Logical DDL stage as already decided.

## Decision

1. Work-State semantic nodes use one Entity/EntityVersion family. Relations
   use a separate Relation/RelationVersion family. `ObjectIdentity` is a lower
   Store-local addressable registry: Object is not Entity, and each committed
   ObjectIdentity has exactly one kind-matching typed owner.
2. Entity and Relation identity properties are immutable. Versions are
   complete, immutable semantic-state values independent of Branch. Relation
   logical identity uses Workspace, canonical type, source, target, and an
   immutable discriminator; remove/re-add of the same key reuses identity.
3. Work State is the Entity-to-EntityVersion and
   Relation-to-RelationVersion membership mapping selected by Branch HEAD.
   Current and typed projections are complete only under explicit projection
   validity and remain rebuildable non-authority.
4. WorkStateCommit, ordered-role parents, one ChangeSet, normalized
   ChangeOperations, and immutable Events retain distinct responsibilities.
   Change Operations reconstruct state; Events explain versioned, runtime, and
   infrastructure occurrences. Branch movement is compare-and-swap and always
   leaves old/new provenance.
5. Session, Claim, and MergeAttempt stable occurrences are separated from
   their mutable runtime projections. Evidence is separated from digest-based
   ContentObject storage. Resource identity, environment binding, immutable
   observation, and Workspace association remain separate.
6. Verification is an immutable Entity-backed judgment whose result, target,
   Basis, Evidence set, method, semantic state, and defining relation edges
   form one immutable creation closure. Applicability remains derived.
7. KnowledgeSpace and immutable KnowledgeExposure use the federation family,
   with append-only transitions and separate current/source-stale projections.
   ExternalObjectRef is restricted to allowed provenance/federation metadata;
   canonical Relations still require local ObjectIdentity endpoints.
8. Bundle export computes the required local reference closure. Store manifest,
   migration provenance, ContentObject storage location, Checkpoint,
   BundleManifest, and ImportAttempt remain distinct infrastructure families.
   Import stages and validates immutable candidates before atomic ref
   activation, and equal-identity immutable writes are idempotent only when
   content is equal.
9. Store and Workspace sit outside ObjectIdentity. Ordinary Store movement
   preserves identity; Store fork creates a new Store namespace while
   preserving internal local IDs by default. Portable full references are
   `(store_id, local_id)`. V1 adds no Workspace fork/clone operation;
   Work-State divergence continues to use Branch.

The detailed confirmed family contract is
[Logical Schema Boundaries](../../architecture/logical-schema-boundaries.md).

## Consequences

- Every mutable or immutable fact has one named authority layer; Runtime,
  Events, projections, Checkpoints, and transport artifacts do not compete with
  the Work-State DAG.
- Logical DDL can now derive key, uniqueness, nullability, and immutable-row
  requirements from a closed logical model without reopening product semantics.
- Portable compact Bundles cannot sever local canonical Relation endpoints,
  while allowed foreign provenance remains representable.
- Runtime recovery, projection rebuild, and import staging can remain safe
  because stable occurrences and canonical history outlive mutable current
  rows.

## Deliberately not decided

- the final `next` equal-candidate tie-breaker;
- persistent ID/UUID encoding, hash algorithm, and canonical serialization;
- final CLI spelling, protocol encoding, programming language, or runtime API;
- final SQLite column types, complete DDL, concrete indexes, or exact
  foreign-key enforcement mechanism;
- checkpoint strategy and the number/shape of typed projection tables;
- Knowledge exchange/access API, authorization, or cross-Store live
  federation;
- storage-layout and transaction-SQL choices beyond the confirmed logical
  atomicity and authority boundaries.
