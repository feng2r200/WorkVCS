# Confirmed State v0.5 Provenance

This provenance file records the accepted WorkVCS decisions 514-566 and the
schema-v0.1 assembly closure. It extends
[Confirmed State v0.4](confirmed-state-v0.4.md) and does not replace the
normative documents.

## Confirmation source

- Referenced conversation: `6a847cdb-7f00-83e8-8870-a8acee174a4c`
- Conversation title: `WorkVCS需求沟通-v1`
- Confirmation turns:
  - `0e335454-8280-4680-a879-d204f1f6d93e`: user confirmed 514-522.
  - `c72cff7c-c8fc-4f16-964f-581064ebdbdd`: user confirmed 523-534.
  - `58da6f77-fc50-4c6c-8d54-29549f842295`: user confirmed 535-550.
  - `b6fa5191-dc3e-4a5d-b057-a90815d7e035`: user confirmed 551-566.

## Closed stage ledger

- Domain Model: CLOSED
- Logical DDL: CLOSED
- DDL Consolidation Review: CLOSED
- Physical Encoding: CLOSED
- Physical DDL: CLOSED
- Schema Assembly Round 1: CLOSED
- FK/Ownership Assembly: CLOSED
- Constraint/Transaction Model: CLOSED
- Compatibility/Integrity: CLOSED

## Decision ledger

### Schema assembly and concrete physical corrections

- 514: `schema-v0.1.sql` is separate from connection bootstrap; foreign keys
  are enabled and verified per connection; WAL and busy timeout are operational
  policy, not schema contract.
- 515: readable table creation order is not the FK-cycle solution;
  `workspace.genesis_commit_id` is non-null and deferred, and SQLite forward
  references are allowed.
- 516: `merge_attempt_outcome` records immutable completed/aborted merge
  outcomes and the result Commit when present.
- 517: `merge_resolution.finalization_kind` is removed.
- 518: ExternalObjectRef has object-scope and version-scope partial uniqueness.
- 519: KnowledgeExposure has a partial unique invariant for exactly one initial
  transition.
- 520: Applicability Resource Stamp can record no observed fingerprint through
  `observation_status` values `observed`, `unavailable`, and `error`.
- 521: Branch projection projected Commit and digest are paired: both present
  or both absent according to projection state.
- 522: v0.1 includes correctness indexes only; performance indexes wait for
  workload evidence.

### Bootstrap, Genesis, ownership, and exact-family contracts

- 523: Store bootstrap atomically creates exactly one Store and one current
  Manifest as an Engine invariant, without a singleton trigger.
- 524: Workspace creation includes an atomic Genesis initialization transaction.
- 525: Genesis has zero ChangeOperations, zero parents, `workspace.genesis`
  operation type, and the digest of the empty Work State.
- 526: a new Workspace creates at least one initial Branch whose HEAD is
  Genesis; default Branch name is not schema contract.
- 527: the only deferred foreign key is Workspace to Genesis Commit; all other
  foreign keys are immediate.
- 528: ObjectIdentity-backed typed owners are exactly one of entity, relation,
  session, session_diff, claim, merge_attempt, evidence, resource,
  resource_observation, knowledge_space, and knowledge_exposure.
- 529: ObjectIdentity and its typed owner are ingested atomically, and import
  batches must be closure-safe.
- 530: Entity subtype companion exactness is fixed for Record,
  AcceptanceCriterion, VerificationRequirement, and Verification.
- 531: KnowledgeExposure creation includes ObjectIdentity, KnowledgeExposure,
  exactly one local/external source, and exactly one initial transition.
- 532: each ChangeOperation has exactly one typed child: Entity membership or
  Relation membership.
- 533: Commit parent shape is fixed: Genesis zero parents, normal one primary,
  merge primary plus secondary, and merge attempt outcome matches the result.
- 534: relation endpoint workspace matching is relation-type-specific, not a
  universal same-Workspace rule.

### Constraint and transaction model

- 535: constraints are allocated across SQLite structural checks, Engine
  preflight, transaction templates, validator/doctor checks, and tests.
- 536: SQLite structural integrity is the final boundary for constraints SQL
  can express.
- 537: ordinary semantic mutations use a versioned transaction template.
- 538: `BEGIN IMMEDIATE` does not replace Branch HEAD compare-and-swap.
- 539: Branch HEAD CAS is SQL-level expected-head update with one changed row
  required; otherwise the Engine raises `WORKSTATE_HEAD_MOVED`.
- 540: candidate-state validation cannot depend on projection availability.
- 541: required Events are atomic with the state changes they document.
- 542: Events are not semantic ordering authority or replay truth.
- 543: ordinary versioned semantic operations do not create empty commits;
  Genesis is the exception.
- 544: runtime coordination uses a separate transaction template.
- 545: merge continue validates attempt/runtime and heads, inserts canonical
  history plus frozen resolutions and completed outcome, CASes target HEAD,
  cleans runtime, updates projection, and records Events.
- 546: merge abort records runtime/provenance outcome and cleanup without
  creating a Commit or moving Branch HEAD.
- 547: SessionEnd creates SessionDiff, releases ClaimRuntime, clears
  Focus/Context, deletes SessionRuntime, and records Events.
- 548: failed canonical transactions normally leave no Event unless the
  failure is modeled as an occurrence/outcome record.
- 549: import cannot use `INSERT OR REPLACE`; immutable identity collision with
  different canonical content is an integrity conflict.
- 550: projection/cache publish validates a candidate projection before
  switching the authoritative cache gate.

### Errors, install, open, compatibility, and integrity

- 551: Engine/CLI errors expose a stable Error Object: code, category, message,
  retryable flag, and structured details.
- 552: public error codes are semantic contracts, not raw SQLite error text.
- 553: stable error categories are `usage`, `not_found`, `precondition`,
  `conflict`, `validation`, `compatibility`, `integrity`, `unavailable`, and
  `internal`.
- 554: JSON and human CLI output share the same Engine Error Object; JSON uses
  canonical IDs in details.
- 555: schema installation sets a fixed SQLite `application_id`.
- 556: `PRAGMA user_version` is not a second schema authority.
- 557: Store open splits bootstrap validation from deep validation.
- 558: newer-than-supported Store or schema format fails closed.
- 559: older Store/schema format requires explicit migration, not silent open.
- 560: integrity is classified as physical SQLite integrity, structural
  referential integrity, canonical history integrity, and derived-state
  integrity.
- 561: derived corruption can be discarded/rebuilt without a WorkStateCommit.
- 562: canonical history corruption fails closed and requires explicit
  doctor/recovery.
- 563: ContentObject with zero storage locations is availability state, not an
  automatic canonical integrity failure.
- 564: Store Doctor has reserved quick/full contracts using the same diagnostic
  and structured error model.
- 565: install success requires application_id, tables/indexes, FK enforcement,
  Store/Manifest invariant, Genesis ChangeSet/Commit, zero parents, initial
  Branch HEAD, and foreign-key check.
- 566: no full DDL fingerprint becomes Store canonical authority; use
  application_id, schema_version, structural introspection, and integrity/doctor
  checks.

## Repository effects

- [ADR-0007](../decisions/adr/0007-schema-v0.1-assembly-install-and-integrity.md)
  records the accepted assembly, install, and integrity contract.
- [Physical Schema v0.1 Contract](../architecture/physical-schema-v0.1.md)
  now points to the executable schema and validation harness.
- [schema-v0.1.sql](../../schema/schema-v0.1.sql) is the first executable
  assembly of the confirmed v0.1 SQLite schema.
