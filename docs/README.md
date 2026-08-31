# WorkVCS Documentation

This directory is the repository-native source of truth for WorkVCS's
confirmed product, domain, and architecture state.

## Implementation readiness

The current V1 implementation-readiness and dogfood gap ledger is:

- [V1 Readiness Ledger](provenance/v1-readiness-ledger.md)
- [Context Profile Budget Dogfood Evidence](provenance/context-profile-budget-dogfood.md)
- [Verify Wrapper Dogfood Evidence](provenance/verify-wrapper-dogfood.md)
- [Verification Cache Refresh Dogfood Evidence](provenance/verification-cache-refresh-dogfood.md)
- [Focused Handoff Smoke Evidence](provenance/focused-handoff-smoke-evidence.md)
- [Claim Transfer and Force Takeover Smoke Evidence](provenance/claim-transfer-force-takeover-smoke-evidence.md)
- [Local Operator Quickstart and Recovery](operator/quickstart-and-recovery.md)
- [Session Potentially Stale Smoke Evidence](provenance/session-potentially-stale-smoke-evidence.md)
- [Phase 4LE Dogfood and Stale-Gated Takeover Evidence](provenance/phase-4le-dogfood-and-stale-gated-takeover.md)
- [Phase 4LF Handoff Consumption Dogfood Evidence](provenance/phase-4lf-handoff-consumption-dogfood.md)

The ledger maps current evidence. It does not replace the confirmed product,
domain, architecture, schema, or ADR authorities.

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
- [ADR-0024: Phase 3J Plan Semantic Kernel](decisions/adr/0024-phase-3j-plan-semantic-kernel.md)
- [ADR-0025: Phase 3K Primary Containment Relations](decisions/adr/0025-phase-3k-primary-containment-relations.md)
- [ADR-0026: Phase 3L Runnable Containment Scope](decisions/adr/0026-phase-3l-runnable-containment-scope.md)
- [ADR-0027: Phase 3M Goal Semantic Kernel](decisions/adr/0027-phase-3m-goal-semantic-kernel.md)
- [ADR-0028: Phase 3N Goal Containment Endpoints](decisions/adr/0028-phase-3n-goal-containment-endpoints.md)
- [ADR-0029: Phase 3O Goal Lifecycle Transition](decisions/adr/0029-phase-3o-goal-lifecycle-transition.md)
- [ADR-0030: Phase 3P Plan Lifecycle Transition](decisions/adr/0030-phase-3p-plan-lifecycle-transition.md)
- [ADR-0031: Phase 3Q Goal-Focused Runnable Projection](decisions/adr/0031-phase-3q-goal-focused-runnable-projection.md)
- [ADR-0032: Phase 3R Structural References Relations](decisions/adr/0032-phase-3r-structural-references-relations.md)
- [ADR-0033: Phase 3S WorkState Diff Query](decisions/adr/0033-phase-3s-workstate-diff-query.md)
- [ADR-0034: Phase 3T Why Structural Neighborhood](decisions/adr/0034-phase-3t-why-structural-neighborhood.md)
- [ADR-0035: Phase 3U Why Verification Neighborhood](decisions/adr/0035-phase-3u-why-verification-neighborhood.md)
- [ADR-0036: Phase 3V Verification Evidence Closure](decisions/adr/0036-phase-3v-verification-evidence-closure.md)
- [ADR-0037: Phase 3W Why Evidence Neighborhood](decisions/adr/0037-phase-3w-why-evidence-neighborhood.md)
- [ADR-0038: Phase 3X Resource Runtime Foundation](decisions/adr/0038-phase-3x-resource-runtime-foundation.md)

Recent repository-level CLI smoke gates:

- [ADR-0397: Phase 4KH CLI Smoke Workflow](decisions/adr/0397-phase-4kh-cli-smoke-workflow.md)
- [ADR-0398: Phase 4KI CLI Completion Gate Smoke](decisions/adr/0398-phase-4ki-cli-completion-gate-smoke.md)
- [ADR-0399: Phase 4KJ CLI Smoke Runtime Closeout](decisions/adr/0399-phase-4kj-cli-smoke-runtime-closeout.md)
- [ADR-0400: Phase 4KK CLI Smoke Integrity Gate](decisions/adr/0400-phase-4kk-cli-smoke-integrity-gate.md)
- [ADR-0401: Phase 4KL CLI Smoke Scheduling Claim Next Gate](decisions/adr/0401-phase-4kl-cli-smoke-scheduling-claim-next-gate.md)
- [ADR-0402: Phase 4KM Merge Action Expectations Smoke Gate](decisions/adr/0402-phase-4km-merge-action-expectations-smoke-gate.md)
- [ADR-0403: Phase 4KN Checkpoint Smoke Gate](decisions/adr/0403-phase-4kn-checkpoint-smoke-gate.md)
- [ADR-0404: Phase 4KO Bundle Smoke Gate](decisions/adr/0404-phase-4ko-bundle-smoke-gate.md)
- [ADR-0405: Phase 4KP Bundle Checkpoint Smoke Gate](decisions/adr/0405-phase-4kp-bundle-checkpoint-smoke-gate.md)
- [ADR-0406: Phase 4KQ Bundle Divergence Smoke Gate](decisions/adr/0406-phase-4kq-bundle-divergence-smoke-gate.md)
- [ADR-0407: Phase 4KR Bundle Branch Preflight Detail](decisions/adr/0407-phase-4kr-bundle-branch-preflight-detail.md)
- [ADR-0408: Phase 4KS Bundle Import Show Branch Detail](decisions/adr/0408-phase-4ks-bundle-import-show-branch-detail.md)
- [ADR-0409: Phase 4KT Bundle Import List Branch Detail](decisions/adr/0409-phase-4kt-bundle-import-list-branch-detail.md)
- [ADR-0410: Phase 4KU CLI Smoke Verification Requirement Closure](decisions/adr/0410-phase-4ku-cli-smoke-verification-requirement-closure.md)
- [ADR-0411: Phase 4KV CLI Help Summaries](decisions/adr/0411-phase-4kv-cli-help-summaries.md)
- [ADR-0412: Phase 4KW V1 Readiness Ledger](decisions/adr/0412-phase-4kw-v1-readiness-ledger.md)
- [ADR-0413: Phase 4KX Context Profile Budget](decisions/adr/0413-phase-4kx-context-profile-budget.md)
- [ADR-0414: Phase 4KY Verification Wrapper](decisions/adr/0414-phase-4ky-verification-wrapper.md)
- [ADR-0415: Phase 4KZ Verification Cache Refresh Recovery](decisions/adr/0415-phase-4kz-verification-cache-refresh.md)
- [ADR-0416: Phase 4LA Focused Handoff](decisions/adr/0416-phase-4la-focused-handoff.md)
- [ADR-0417: Phase 4LB Claim Transfer and Force Takeover](decisions/adr/0417-phase-4lb-claim-transfer-force-takeover.md)
- [ADR-0418: Phase 4LC Operator Quickstart and Recovery Docs](decisions/adr/0418-phase-4lc-operator-quickstart-recovery.md)
- [ADR-0419: Phase 4LD Explicit Potentially Stale Session State](decisions/adr/0419-phase-4ld-session-potentially-stale.md)
- [ADR-0420: Phase 4LE Stale-Gated Claim Takeover](decisions/adr/0420-phase-4le-stale-gated-claim-takeover.md)
- [ADR-0421: Phase 4LF Handoff Consumption Dogfood](decisions/adr/0421-phase-4lf-handoff-consumption-dogfood.md)

These documents specify what WorkVCS currently means. Implementation readiness
is tracked separately in the V1 readiness ledger. The local Rust V0.1
implementation is in progress and has smoke-proven coverage for several V1
areas, but it is not release-ready and is not dogfood-complete.

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
