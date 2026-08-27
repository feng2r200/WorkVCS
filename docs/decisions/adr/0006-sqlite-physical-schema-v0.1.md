# ADR-0006: SQLite Physical Schema v0.1 Contract

- **Status:** Accepted
- **Accepted by:** explicit user confirmations for decisions 443–513
- **Confirmation turns:** `e64d57e6-aedc-4b30-8b74-112034009560`,
  `4e6d5b38-0331-4397-99f5-ce50e8a1d0fd`,
  `596e1544-ff51-4e7f-8fab-8cfd66f30a7f`,
  `00d13235-4a2b-4361-81db-5a64a2ab36b6`,
  `46084618-f6e8-4178-beb7-8fb4cb5826d0`,
  `9d15b983-97c2-4e34-88b2-75979b0e30ca`,
  `7e3df5ef-a05a-441b-94d9-451f9041ce37`, and
  `9fe613e6-773f-4452-a597-68fcebef79bc`

## Context

ADR-0005 closed the logical family and single-authority model while leaving
physical encodings, SQLite integrity allocation, runtime transaction defaults,
and physical schema details Open. The subsequent Physical DDL rounds translated
that model into a bounded v0.1 SQLite contract and closed the Schema Assembly
Review without assembling an executable schema or implementing the engine.

## Decision

1. V1 uses SQLite `STRICT` tables, explicit per-connection foreign-key
   enforcement, immediate foreign keys by default, and schema-declared deferred
   foreign keys only for real construction cycles. Canonical mutation uses
   controlled `BEGIN IMMEDIATE` writer transactions; WAL and `busy_timeout`
   remain operational policies rather than Store-format correctness.
2. Stable logical IDs are UUIDv7 stored as 16-byte BLOBs. Content/state digests
   and canonical Resource fingerprints are BLAKE3-256 stored as 32-byte BLOBs.
   Timestamps are UTC epoch microseconds and never causal authority. Short IDs
   are derived display aliases.
3. Canonical structured payloads are UTF-8 JSON text validated for JSON syntax
   in SQLite. The WorkVCS serializer and Store Manifest canonical JSON profile
   define canonical bytes and semantic validity. Closed structural vocabularies
   use SQL checks; extensible semantic vocabularies remain Engine-validated
   symbolic text without v0.1 registry tables.
4. StoreManifest carries the identity, digest, and canonical JSON profile
   parameters needed to interpret the Store. Engine open/init enforces one
   Store per SQLite database. Immutable semantic-state rows omit occurrence
   timestamps, and v0.1 uses the Engine write layer plus tests rather than a
   large trigger set for insert-only enforcement.
5. Human-addressable names and scoped local keys are exact, case-sensitive
   UTF-8 identities with a DDL non-empty check and Engine validation for NUL,
   ASCII controls, and surrounding whitespace. Branch HEAD history remains
   immutable Event provenance; Branch identity rows remain long-lived.
6. Runtime cleanup is explicit Engine behavior. Merge classifications and
   resolution kinds are closed structural vocabularies; one MergeItem has at
   most one frozen final resolution and custom payload nullability is exact.
   Active Claim and active-Merge uniqueness remain Engine invariants in
   `BEGIN IMMEDIATE` transactions without duplicated canonical owner fields.
7. Resource observations and Verification Resource Basis record Adapter and
   scope contract versions. The Adapter normalizes canonical bytes and WorkVCS
   hashes them with the Store digest algorithm. Applicability stamps reference
   real Resource Basis rows. Applicability is a closed derived vocabulary, and
   pure cache children may cascade-delete.
8. Verification target and defining Evidence continue to use the single
   canonical Relation infrastructure. The Engine validates the exactly-one
   target and complete immutable defining closure at creation; no duplicate
   target/evidence tables are introduced.
9. KnowledgeExposure has exactly one local or external source and a linear,
   non-forking transition history. V1 lifecycle is `active`/`withdrawn`, source
   status is `current`/`stale`/`unknown`/`unresolved`, and replacement is a new
   Exposure plus withdrawal of the old. External version references remain
   WorkVCS 16-byte version identities with explicit object/version scope.
10. Projection, Checkpoint, import, migration, and lineage remain non-competing
    layers. Projection rebuild uses shadow construction plus atomic publish;
    caches may be physically rebuilt; full canonical import is same-Store
    only; StoreLineage is separate infrastructure provenance; BundleManifest
    remains transport metadata rather than a Store table.
11. The Physical DDL Schema Assembly Review is **PASS / CLOSED**. The next
    stage is complete `schema-v0.1.sql` Assembly and executable validation, not
    continued isolated table design.

The detailed normative contract is
[Physical Schema v0.1 Contract](../../architecture/physical-schema-v0.1.md).

## Consequences

- Prior documents that described UUID encoding, BLAKE3 selection, JSON storage,
  SQLite type/FK strategy, or Exposure lifecycle/status as Open are superseded
  in those exact scopes by this ADR.
- Canonical Work State still has one authority: Branch HEAD, Commit DAG,
  ChangeSet, and membership ChangeOperations. DDL convenience cannot introduce
  competing Verification, projection, runtime, or federation truth.
- The executable schema must prove these decisions at the real SQLite boundary
  and identify every remaining Engine-only invariant.

## Deliberately not decided

- the assembled `schema-v0.1.sql`, table-creation order, and complete executable
  foreign-key/invariant matrix;
- performance indexes or query plans;
- the exact canonical JSON profile rules and implementation language/binding;
- checkpoint policy and the workload-driven typed-projection set;
- Resource normalization details, Bundle container/profile details, Knowledge
  access/security APIs, and cross-Store live federation;
- final CLI/protocol spelling and unrelated Open product behavior.
