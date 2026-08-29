# WorkVCS Documentation

This directory is the repository-native source of truth for WorkVCS's
confirmed product, domain, and architecture state.

## Current baseline

The confirmed baseline contains:

- [Product definition](product/product-definition.md)
- [V1 and V2 boundary](product/v1-v2-boundary.md)
- [Domain entities](domain/entities.md)
- [Typed relationships](domain/relationships.md)
- [Domain invariants](domain/invariants.md)
- [System boundaries](architecture/system-boundaries.md)
- [Versioning engine](architecture/versioning-engine.md)
- [Semantic operations and state machines](architecture/semantic-operations-and-state-machines.md)
- [Verification and resource drift](architecture/verification-and-resource-drift.md)
- [Versioned-state persistence model](architecture/persistence-model.md)
- [Knowledge federation and Store portability](architecture/knowledge-federation-and-portability.md)
- [Logical schema boundaries](architecture/logical-schema-boundaries.md)
- [Physical Schema v0.1 contract](architecture/physical-schema-v0.1.md)
- [Implementation Contract v0.1](architecture/implementation-contract-v0.1.md)
- [Executable schema v0.1](../schema/schema-v0.1.sql)

Material architecture decisions promoted after the initial baseline are
recorded as Accepted ADRs:

- [ADR-0001: Verification Applicability and Resource Drift](decisions/adr/0001-verification-applicability-and-resource-drift.md)
- [ADR-0002: Semantic Operations and Runtime State Machines](decisions/adr/0002-semantic-operations-and-runtime-state-machines.md)
- [ADR-0003: Versioned-State Persistence Model](decisions/adr/0003-versioned-state-persistence-model.md)
- [ADR-0004: Knowledge Federation and Store Portability](decisions/adr/0004-knowledge-federation-and-store-portability.md)
- [ADR-0005: Logical Schema Family Boundaries](decisions/adr/0005-logical-schema-family-boundaries.md)
- [ADR-0006: SQLite Physical Schema v0.1 Contract](decisions/adr/0006-sqlite-physical-schema-v0.1.md)
- [ADR-0007: Schema v0.1 Assembly, Install, and Integrity Contract](decisions/adr/0007-schema-v0.1-assembly-install-and-integrity.md)
- [ADR-0008: Implementation Contract and Phase 1 Canonical Core](decisions/adr/0008-implementation-contract-and-phase-1-canonical-core.md)
- [ADR-0009: Phase 2 Store Bootstrap and Open Boundary](decisions/adr/0009-phase-2-store-bootstrap-open.md)
- [ADR-0010: Phase 2 Workspace Genesis Bootstrap](decisions/adr/0010-phase-2-workspace-genesis-bootstrap.md)
- [ADR-0011: Phase 2 Genesis Replay](decisions/adr/0011-phase-2-genesis-replay.md)
- [ADR-0012: Phase 2 Entity Transition, Commit, and CAS](decisions/adr/0012-phase-2-entity-transition-commit-cas.md)
- [ADR-0013: Phase 2 Query, Show-at, and History](decisions/adr/0013-phase-2-query-show-at-history.md)
- [ADR-0014: Phase 2 Concurrency and Integrity Closure](decisions/adr/0014-phase-2-concurrency-integrity.md)
- [ADR-0015: Phase 3A Task Semantic Kernel](decisions/adr/0015-phase-3a-task-semantic-kernel.md)
- [ADR-0016: Phase 3B Task Lifecycle Transition](decisions/adr/0016-phase-3b-task-lifecycle-transition.md)
- [ADR-0017: Phase 3C Acceptance Criteria](decisions/adr/0017-phase-3c-acceptance-criteria.md)
- [ADR-0018: Phase 3D Verification Requirement Projection](decisions/adr/0018-phase-3d-verification-requirement-projection.md)
- [ADR-0019: Phase 3E Session Runtime Foundation](decisions/adr/0019-phase-3e-session-runtime-foundation.md)
- [ADR-0020: Phase 3F Claim Runtime Foundation](decisions/adr/0020-phase-3f-claim-runtime-foundation.md)
- [ADR-0021: Phase 3G Runnable Task Projection](decisions/adr/0021-phase-3g-runnable-task-projection.md)
- [ADR-0022: Phase 3H Task Scheduling Relation Foundation](decisions/adr/0022-phase-3h-task-scheduling-relation-foundation.md)
- [ADR-0023: Phase 3I Runnable Dependency Readiness](decisions/adr/0023-phase-3i-runnable-dependency-readiness.md)

These documents specify what WorkVCS currently means. They do not claim that
the described runtime has been implemented.

Detailed promotion evidence and the confirmation ledger are retained in
[Confirmed State v0.1 Provenance](provenance/confirmed-state-v0.1.md) and
[Confirmed State v0.2 Provenance](provenance/confirmed-state-v0.2.md), and
[Confirmed State v0.3 Provenance](provenance/confirmed-state-v0.3.md),
[Confirmed State v0.4 Provenance](provenance/confirmed-state-v0.4.md), and
[Confirmed State v0.5 Provenance](provenance/confirmed-state-v0.5.md), and
[Confirmed State v0.6 Provenance](provenance/confirmed-state-v0.6.md). These
documents support audit and reconstruction but are not parallel normative
specifications.

## Authority and conflict handling

For repository work, follow [`AGENTS.md`](../AGENTS.md). Its workflow and ADR
requirements are repository-governance policy; they are not retroactively
classified as product decisions confirmed in the source conversation.

Within the confirmed documentation set, later explicit user confirmation
supersedes an older decision in the affected scope. Domain invariants constrain
architecture, architecture realizes the confirmed domain, and product documents
define value and release scope. An ADR is a recording and explanation vehicle:
it becomes product authority only when the exact decision it contains has been
accepted. Writing an ADR does not promote an unconfirmed proposal by itself.

If two documents appear to conflict, classify the disputed point as Open or
Unclear, trace both interpretations to their evidence, and obtain an explicit
decision before implementation chooses between them.

## State classification

- **Confirmed:** directly stated by the user or contained in an exact option
  the user explicitly accepted.
- **Open:** a current decision is still required. Open material cannot redefine
  confirmed behavior.
- **Rejected/Superseded:** explicitly rejected or replaced by a later confirmed
  decision. It is retained only as provenance, not current state.
- **Unclear:** the available evidence cannot determine the current answer. Do
  not infer one for implementation convenience.

Generated indexes, summaries, and runtime projections are derived artifacts,
not an additional decision state or an independent authority.

## Updating the baseline

A change to confirmed design must identify its source, update every affected
document, check the invariants, and receive validation at the same impact level.
Current repository policy may require a material architecture decision to be
recorded as an ADR, but the ADR is accepted only after its exact decision is
confirmed. Conversation history is supporting evidence, not a substitute for
this repository baseline.
