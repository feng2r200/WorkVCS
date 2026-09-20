# ADR-0007: Schema v0.1 Assembly, Install, and Integrity Contract

- **Status:** Accepted
- **Accepted by:** explicit user confirmations for decisions 514-566
- **Confirmation turns:** `0e335454-8280-4680-a879-d204f1f6d93e`,
  `c72cff7c-c8fc-4f16-964f-581064ebdbdd`,
  `58da6f77-fc50-4c6c-8d54-29549f842295`, and
  `b6fa5191-dc3e-4a5d-b057-a90815d7e035`

## Context

[ADR-0006](0006-sqlite-physical-schema-v0.1.md) closed the bounded SQLite
Physical DDL contract through decision 513 and left executable assembly,
bootstrap/open validation, transaction templates, and compatibility/integrity
behavior for the next stage. Decisions 514-566 close that stage without
introducing new architecture capability, new table families, or performance
indexes.

The implemented artifact is [schema-v0.1.sql](../../../schema/schema-v0.1.sql).
The minimal executable harness is
[validate-schema-v0.1.sh](../../../scripts/validate-schema-v0.1.sh).

## Decision

1. `schema-v0.1.sql` is the first complete executable SQLite schema for v0.1.
   It is separate from connection bootstrap. Foreign-key enforcement is still
   explicitly enabled and verified per Engine connection.
2. The current schema contains the confirmed 68 physical tables and correctness
   indexes only. The current count includes the additive
   `context_packet_snapshot` runtime-provenance table accepted later in
   [ADR-0434](0434-phase-4ls-context-packet-persistence.md). No
   performance/query index design is part of this ADR.
3. The only deferred foreign key in v0.1 is the real Workspace/Genesis cycle:
   `workspace.genesis_commit_id` references `workstate_commit(commit_id)`
   `DEFERRABLE INITIALLY DEFERRED`. All other foreign keys remain immediate.
4. Merge attempts have immutable `merge_attempt_outcome` records for completed
   and aborted outcomes. `merge_resolution.finalization_kind` is removed; final
   outcome belongs to the attempt outcome row.
5. ExternalObjectRef object scope and version scope use separate partial
   unique correctness indexes. KnowledgeExposure transition history uses a
   partial unique correctness index for exactly one initial transition.
6. Applicability Resource Stamp supports observed, unavailable, and error
   observation states. Unavailable/error stamps may have NULL observed
   fingerprint and NULL observation reference.
7. Branch projection state treats projected Commit and projection digest as a
   pair: both are NULL for `not_materialized`, and both are present for
   materialized states that name a projected Commit.
8. Store/Manifest bootstrap is an Engine invariant over ordinary tables rather
   than a complex singleton trigger. Install/open validation proves exactly one
   Store and current StoreManifest.
9. Workspace creation uses a Genesis initialization transaction. Genesis has
   zero Commit parents, zero ChangeOperations, operation type
   `workspace.genesis`, and an empty Work State digest. A new Workspace creates
   at least one initial Branch whose HEAD is Genesis.
10. ObjectIdentity exact-family ownership, subtype companion exactness,
    KnowledgeExposure source exactness, ChangeOperation typed-child exactness,
    Commit parent-shape, Verification defining closure, relation endpoint
    kind/workspace rules, and candidate-state validation remain Engine
    validator contracts where SQLite cannot express the invariant cleanly.
11. Canonical semantic mutations and runtime coordination operations follow
    explicit transaction templates. Branch HEAD movement always uses
    expected-head compare-and-swap; SQLite writer serialization does not
    replace logical CAS.
12. Errors exposed to Agents use a stable Error Object with code, category,
    message, retryability, and structured details. Public error codes are
    semantic contracts, not raw SQLite errors. JSON and human CLI output share
    the same Engine Error Object.
13. Install/open compatibility uses fixed `application_id`, StoreManifest
    format parameters, structural introspection, and integrity/doctor checks.
    `PRAGMA user_version` is not a second schema authority, and a full DDL
    fingerprint is not Store canonical authority.
14. Newer-than-supported Store/schema versions fail closed. Older Stores need
    explicit migration. Derived corruption may be rebuilt; canonical history
    corruption fails closed and requires explicit doctor/recovery.

## Consequences

- The following stages are closed: Domain Model, Logical DDL, DDL
  Consolidation Review, Physical Encoding, Physical DDL, Schema Assembly Round
  1, FK/Ownership Assembly, Constraint/Transaction Model, and
  Compatibility/Integrity.
- ADR-0006 remains the physical schema contract; this ADR resolves its
  executable assembly and validation open items.
- Storage engine business operations are not implemented by this ADR. The
  schema and harness only prove the install/bootstrap boundary and representative
  invariant contracts.
- Performance indexes, exact canonical JSON serialization profile,
  programming language/SQLite binding, checkpoint policy, Resource
  normalization details, Bundle container/profile details, Knowledge
  access/security APIs, cross-Store live federation, and final CLI/protocol
  spelling remain outside this assembly closure.

## Implementation findings

- SQLite accepted the assembled 67-table `STRICT` schema with the Workspace to
  Genesis Commit deferred cycle.
- SQLite cannot directly enforce several exact-family and graph-shape
  invariants without expanding the schema into trigger-heavy behavior. Those
  invariants remain validator contracts and are exercised by the minimal
  bootstrap harness where representative setup can prove the boundary.
- No conflict with the frozen decisions required a semantic correction while
  assembling this first executable schema.
