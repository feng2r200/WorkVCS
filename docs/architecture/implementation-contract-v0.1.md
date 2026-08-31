# Implementation Contract v0.1

This document records the frozen development-entry contract for the first
WorkVCS implementation slice. It extends the confirmed product, domain,
architecture, physical schema, and schema assembly documents. It defines the
Phase 1 starting boundary; later accepted ADRs govern later implementation
slices.

## Status

- Implementation Ready = YES.
- Development prerequisites = 0.
- This document remains the authority for the Phase 1 canonical core and first
  vertical-slice contract.
- Current implementation readiness is tracked in
  [V1 Readiness Ledger](../provenance/v1-readiness-ledger.md). Do not use this
  document as a live progress ledger for later phases.

If implementation exposes a conflict inside this contract, record it as an
implementation finding. Do not silently expand architecture or reopen closed
domain/DDL decisions.

## IC: Technology Baseline

- IC-01: WorkVCS v0.1 implementation starts in Rust.
- IC-02: The Rust workspace uses edition 2024.
- IC-03: The initial workspace contains only `workvcs-core` and `workvcs-cli`.
- IC-04: `workvcs-core` owns authoritative Engine, Store, History, identity,
  digest, canonical encoding, validation, and hashing logic.
- IC-05: `workvcs-cli` is a thin command/input/output layer over core APIs and
  must not write SQL or canonical history directly.
- IC-06: Stable logical IDs use strongly typed UUIDv7 values with canonical
  lowercase hyphenated text and raw 16-byte ordering where specified.
- IC-07: Content, state, and version digests use BLAKE3-256 as 32 bytes and
  lowercase 64-character hex at text boundaries.
- IC-08: Canonical semantic JSON uses `serde`/`serde_json`; helper crates may be
  used only inside the canonical module, and WorkVCS conformance vectors remain
  the authority.
- IC-09: First validation uses Rust unit tests, property tests, `cargo fmt`,
  `cargo clippy`, and `cargo test`.
- IC-10: Phase 1 must remain independent from SQLite Store/bootstrap/Genesis,
  Task, Runtime, Verification, Merge, Federation, Projection, and migration
  chain implementation.

## CE: Canonical Encoding Contract

- CE-01: Canonicalization is a two-stage model: typed domain state is
  semantically normalized into a WorkVCS canonical value, then serialized into
  canonical UTF-8 bytes before hashing.
- CE-02: StoreManifest canonical JSON profile is `workvcs-jcs-v1`.
- CE-03: `workvcs-jcs-v1` inherits RFC 8785/JCS object member sorting,
  recursive object canonicalization, no insignificant whitespace, array order
  preservation, JSON string escaping, and no Unicode normalization.
- CE-04: Canonical semantic JSON admits only Null, Bool, SafeInteger, String,
  Array, and Object.
- CE-05: Floating point, arbitrary precision number semantics, binary payloads,
  NaN, Infinity, and non-JSON values are invalid.
- CE-06: Integer numbers are limited to the JSON safe integer range
  `-(2^53 - 1)` through `2^53 - 1`.
- CE-07: Object keys are strings; duplicate keys are invalid before
  canonicalization.
- CE-08: Object member order is determined by the JCS UTF-16 code-unit order.
  Array element order is semantic and is never sorted by the encoder.
- CE-09: Unicode strings remain exact scalar-value data. WorkVCS does not apply
  NFC, NFD, case folding, locale transforms, or application-specific string
  normalization inside canonical serialization.
- CE-10: UUID text at canonical JSON boundaries is lowercase hyphenated form;
  digest text is lowercase hex.
- CE-11: EntityVersion and RelationVersion digests are domain-separated over
  the canonical bytes of their semantic JSON state.
- CE-12: Immutable import validates fixed points: stored canonical bytes must
  parse, re-encode to identical bytes, and match the expected domain digest.
- CE-13: WorkState `state_digest` uses a binary mapping digest over
  Entity->EntityVersion and Relation->RelationVersion mappings sorted by raw
  16-byte UUID lexicographic order. It does not serialize a Rust map or JSON map
  directly.
- CE-14: ContentObject digest is the BLAKE3-256 digest of raw content bytes,
  not canonical JSON bytes.

## SE: Storage Engine Boundary and First Vertical Slice

- SE-01: Rust workspace v0.1 has only `workvcs-core` and `workvcs-cli`; do not
  split ahead into many crates.
- SE-02: Core is organized by authority boundaries such as identity, canonical,
  store, history, and engine, not by table-by-table repository or DAO files.
- SE-03: `Engine` is the external facade; SQLite connections, transactions,
  and SQL writers do not enter public API.
- SE-04: v0.1 uses one SQLite connection per Engine and does not implement
  pooling, async DB executors, read replicas, or background writers.
- SE-05: The transaction semantics are frozen, but v0.1 does not introduce a
  generic `SemanticOperation` framework before real operations justify it.
- SE-06: Canonical history mutation kernel stays `pub(crate)` internal
  capability and must not become public generic Entity CRUD.
- SE-07: Canonical WorkState uses Entity->EntityVersion and
  Relation->RelationVersion ordered mappings; ordering matches CE-13 raw UUID
  byte order.
- SE-08: `state_at` replays canonical Commit/ChangeSet history; merge replay
  follows the primary parent plus the merge ChangeSet. Events and Projections
  are not replay truth.
- SE-09: The first vertical slice does not depend on materialized Branch
  projection; `not_materialized` projection must still allow correct
  mutation/query.
- SE-10: The first vertical slice validates Store, Genesis, EntityVersion,
  ChangeSet, Commit, CAS, Replay, history, and show-at only; Task, Runtime,
  Verification, Merge, Federation, and other upper domains are not implemented.
- SE-11: Entity create/update in Slice 1 is internal kernel validation and does
  not create a public Entity CRUD CLI/API.
- SE-12: Slice 1 Definition of Done includes Genesis, two Entity transitions,
  restart persistence, projection independence, two-connection CAS race,
  canonical determinism, and core structured errors.
- SE-13: v0.1 CLI first commands are thin `init`, `doctor`, `history`, and
  `show-at` shells; exact long-term option spelling is not frozen.
- SE-14: v0.1 implements fresh install/open/version gating only and does not
  build a complex migration chain before a real v0.2 schema exists.
- SE-15: Development order is Canonical/IDs, Store bootstrap, Genesis, Replay,
  Entity transition/Commit/CAS, queries, then concurrency/integrity tests.
  CLI-first and DAO-first development are forbidden.

## Phase 1 Scope

Phase 1 implements only:

- strongly typed UUIDv7 IDs;
- BLAKE3-256 `Digest`;
- canonical module boundary;
- `CanonicalValue`, strict validation, and `workvcs-jcs-v1` encoding;
- domain-separated EntityVersion and RelationVersion hashing;
- CE-13 WorkState mapping digest;
- ContentObject raw-byte digest;
- conformance tests for the CE obligations above;
- the thinnest compiling CLI crate skeleton.

Phase 1 explicitly does not implement:

- SQLite Store/open/bootstrap/install;
- Workspace Genesis;
- ChangeSet/Commit persistence;
- Branch HEAD CAS;
- Task, Runtime, Verification, Merge, Federation, Projection, Checkpoint,
  Bundle, Doctor, or migration workflows.
