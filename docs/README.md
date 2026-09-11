# WorkVCS Documentation

This directory is the repository-native source of truth for WorkVCS's
confirmed product, domain, and architecture state.

## Implementation readiness

The current V1 implementation-readiness, release gate, and dogfood evidence
index is:

- [V1 Readiness Ledger](provenance/v1-readiness-ledger.md)
- [V1 Release Gate Matrix](provenance/v1-release-gate-matrix.md)
- [Context Profile Budget Dogfood Evidence](provenance/context-profile-budget-dogfood.md)
- [Verify Wrapper Dogfood Evidence](provenance/verify-wrapper-dogfood.md)
- [Verification Cache Refresh Dogfood Evidence](provenance/verification-cache-refresh-dogfood.md)
- [WorkVCS Tool Reference For Governance Plan Carriers](operator/workvcs-tool-reference.md)
- [Focused Handoff Smoke Evidence](provenance/focused-handoff-smoke-evidence.md)
- [Claim Transfer and Force Takeover Smoke Evidence](provenance/claim-transfer-force-takeover-smoke-evidence.md)
- [Local Operator Quickstart and Recovery](operator/quickstart-and-recovery.md)
- [WorkVCS Error Recovery Guide](operator/error-recovery-guide.md)
- [Session Potentially Stale Smoke Evidence](provenance/session-potentially-stale-smoke-evidence.md)
- [Phase 4LE Dogfood and Stale-Gated Takeover Evidence](provenance/phase-4le-dogfood-and-stale-gated-takeover.md)
- [Phase 4LF Handoff Consumption Dogfood Evidence](provenance/phase-4lf-handoff-consumption-dogfood.md)
- [Phase 4LG Blocked Handoff Recovery Dogfood Evidence](provenance/phase-4lg-blocked-handoff-recovery-dogfood.md)
- [Phase 4LH Handoff Focus Why Evidence](provenance/phase-4lh-handoff-focus-why.md)
- [Phase 4LI Claim Next Context Packet Evidence](provenance/phase-4li-claim-next-context-packet.md)
- [Phase 4LJ Context AC and VR Packet Evidence](provenance/phase-4lj-context-ac-vr-packet.md)
- [Phase 4LK Context Blocker Packet Evidence](provenance/phase-4lk-context-blocker-packet.md)
- [Phase 4LL Context Goal/Plan Path Packet Evidence](provenance/phase-4ll-context-goal-plan-path-packet.md)
- [Phase 4LM Context Attempt Detail Packet Evidence](provenance/phase-4lm-context-attempt-detail-packet.md)
- [Phase 4LN Merge Lifecycle Dogfood Evidence](provenance/phase-4ln-merge-lifecycle-dogfood.md)
- [Phase 4LO Bundle Portability Dogfood Evidence](provenance/phase-4lo-bundle-portability-dogfood.md)
- [Phase 4LP Bundle Profile Contract Evidence](provenance/phase-4lp-bundle-profile-contract.md)
- [Phase 4LQ Larger Store Portability Validation Evidence](provenance/phase-4lq-larger-store-portability-validation.md)
- [Phase 4LR Context Path-Sensitive Knowledge Evidence](provenance/phase-4lr-context-path-sensitive-knowledge.md)
- [Phase 4LS Context Packet Persistence Evidence](provenance/phase-4ls-context-packet-persistence.md)
- [Phase 4LT Transition Rationale Context Evidence](provenance/phase-4lt-transition-rationale-context.md)
- [Phase 4LU Claim Guard Recovery Hints Evidence](provenance/phase-4lu-claim-guard-recovery-hints.md)
- [Phase 4LV Another Project Dogfood Evidence](provenance/phase-4lv-another-project-dogfood.md)
- [Phase 4LW Claim Transfer and Takeover Dogfood Evidence](provenance/phase-4lw-claim-transfer-takeover-dogfood.md)
- [Phase 4LX Resource Scope Path Normalization Evidence](provenance/phase-4lx-resource-scope-path-normalization.md)
- [Phase 4LY Resource Scope File Observation Evidence](provenance/phase-4ly-resource-scope-file-observation.md)
- [Phase 4LZ Stable Error Code Output Evidence](provenance/phase-4lz-stable-error-code-output.md)
- [Phase 4MA Why Explanation Dogfood Evidence](provenance/phase-4ma-why-explanation-dogfood.md)
- [Phase 4MB Resource Unavailable Error Dogfood Evidence](provenance/phase-4mb-resource-unavailable-error-dogfood.md)
- [Phase 4MC Local File Cache Refresh Evidence](provenance/phase-4mc-local-file-cache-refresh.md)
- [Phase 4MD Local File Path Prefix Refresh Evidence](provenance/phase-4md-local-file-path-prefix-refresh.md)
- [Phase 4ME Local File Glob Refresh Evidence](provenance/phase-4me-local-file-glob-refresh.md)
- [Phase 4MF Git Worktree Resource Refresh Evidence](provenance/phase-4mf-git-worktree-resource-refresh.md)
- [Phase 4MG Resource Basis Cache Refresh Evidence](provenance/phase-4mg-resource-basis-cache-refresh.md)
- [Phase 4MH Why Evolution Deferred Family Evidence](provenance/phase-4mh-why-evolution-deferred-family.md)
- [Phase 4MI Shared Claim Dogfood Evidence](provenance/phase-4mi-shared-claim-dogfood.md)
- [Phase 4MJ Clap Error Normalization Evidence](provenance/phase-4mj-clap-error-normalization.md)
- [Phase 4MK Per-Code Recovery Guide Evidence](provenance/phase-4mk-per-code-recovery-guide.md)
- [Phase 4ML Why Causal Anchor Traversal Evidence](provenance/phase-4ml-why-causal-anchor-traversal.md)
- [Phase 4MM Merge Maturity Dogfood Evidence](provenance/phase-4mm-merge-maturity-dogfood.md)
- [Phase 4MN Batch Basis Refresh Evidence](provenance/phase-4mn-batch-basis-refresh.md)
- [Phase 4MO JSON Error Output Evidence](provenance/phase-4mo-json-error-output.md)
- [Phase 4MQ Branch Diff Dogfood Evidence](provenance/phase-4mq-branch-diff-dogfood.md)
- [Phase 4MR Shared Claim Write-Mode Dogfood Evidence](provenance/phase-4mr-shared-claim-write-mode-dogfood.md)
- [Phase 4MS Handoff Consumption Write-Mode Dogfood Evidence](provenance/phase-4ms-handoff-consumption-write-mode-dogfood.md)
- [Phase 4MT External Merge Write-Mode Dogfood Evidence](provenance/phase-4mt-external-merge-write-mode-dogfood.md)
- [Phase 4MU Preexisting External Merge Write-Mode Dogfood Evidence](provenance/phase-4mu-preexisting-external-merge-write-mode-dogfood.md)
- [Phase 4MV Git Rename Resource Policy Dogfood Evidence](provenance/phase-4mv-git-rename-resource-policy-dogfood.md)
- [Phase 4MW Git Symlink Resource Policy Dogfood Evidence](provenance/phase-4mw-git-symlink-resource-policy-dogfood.md)
- [Phase 4MX Git Submodule Resource Policy Dogfood Evidence](provenance/phase-4mx-git-submodule-resource-policy-dogfood.md)
- [Phase 4MY Git Sparse Checkout Resource Policy Dogfood Evidence](provenance/phase-4my-git-sparse-checkout-resource-policy-dogfood.md)
- [Phase 4MZ Resource Case-Folding Policy Dogfood Evidence](provenance/phase-4mz-resource-case-folding-policy-dogfood.md)
- [Phase 4NA Resource Re-Observation Scheduling Policy Dogfood Evidence](provenance/phase-4na-resource-reobservation-scheduling-policy-dogfood.md)
- [Phase 4NB Goal/Plan/Task and AC/VR Recovery Dogfood Evidence](provenance/phase-4nb-goal-plan-task-ac-vr-recovery-dogfood.md)
- [Phase 4NC Why Epistemic Explanation Evidence](provenance/phase-4nc-why-epistemic-explanation.md)
- [Phase 4ND Maintained Store Portability Dogfood Evidence](provenance/phase-4nd-maintained-store-portability-dogfood.md)
- [Phase 4NE Larger Maintained Store Workload Validation Evidence](provenance/phase-4ne-larger-maintained-store-workload-validation.md)
- [Phase 4NF Self-Contained Run Log Summary Evidence](provenance/phase-4nf-self-contained-run-log-summary.md)
- [Phase 4NG Why Evolution Operation Projection Evidence](provenance/phase-4ng-why-evolution-operation-projection.md)
- [Phase 4NH Why Evolution Subject Detail Evidence](provenance/phase-4nh-why-evolution-subject-detail.md)
- [Phase 4NI Operator Recovery Maturity Dogfood Evidence](provenance/phase-4ni-operator-recovery-maturity-dogfood.md)
- [Phase 4NJ Why Subject Evolution Projection Evidence](provenance/phase-4nj-why-subject-evolution-projection.md)
- [Phase 4NK Why Operation-Local Entity Detail Evidence](provenance/phase-4nk-why-operation-local-entity-detail.md)
- [Phase 4NL Why Relation-Subject Endpoint Evolution Evidence](provenance/phase-4nl-why-relation-subject-endpoint-evolution.md)
- [Phase 4NM Why Knowledge Relation Endpoint Evolution Evidence](provenance/phase-4nm-why-knowledge-relation-endpoint-evolution.md)
- [Phase 4NN Context Resource Basis Packet Recovery Evidence](provenance/phase-4nn-context-resource-basis-packet.md)
- [Phase 4NO Why Relation Create Endpoint Evolution Evidence](provenance/phase-4no-why-relation-create-endpoint-evolution.md)
- [Phase 4NP Blocked Dependency Resource Context Evidence](provenance/phase-4np-blocked-dependency-resource-context.md)
- [Phase 4NQ Why Task Scheduling Relation Evolution Evidence](provenance/phase-4nq-why-task-scheduling-relation-evolution.md)
- [Phase 4NR Why Primary Containment Relation Evolution Evidence](provenance/phase-4nr-why-primary-containment-relation-evolution.md)
- [Phase 4NU Focused Plan Peer Resource Context Evidence](provenance/phase-4nu-focused-plan-peer-resource-context.md)
- [Phase 4NV Why Verifies Relation Evolution Evidence](provenance/phase-4nv-why-verifies-relation-evolution.md)
- [Phase 4NW Why Evidenced-By Relation Evolution Evidence](provenance/phase-4nw-why-evidenced-by-relation-evolution.md)
- [Phase 4NX Why Evidence Subject Evidenced-By Evolution Evidence](provenance/phase-4nx-why-evidence-subject-evidenced-by-evolution.md)
- [Phase 4NY Why Knowledge Exposure Derived-From Evolution Evidence](provenance/phase-4ny-why-knowledge-exposure-derived-from-evolution.md)
- [Phase 4NZ Focused Same-Goal Cross-Plan Resource Context Evidence](provenance/phase-4nz-focused-same-goal-cross-plan-resource-context.md)
- [Phase 4OC Focus-Set Unsupported Kind Fail-Fast Evidence](provenance/phase-4oc-focus-set-unsupported-kind-fail-fast.md)
- [Phase 4OE Task Closeout Why Closure Chain Evidence](provenance/phase-4oe-task-closeout-why-closure-chain.md)
- [Phase 4OG Task Why Resource Basis Closure Evidence](provenance/phase-4og-task-why-resource-basis-closure.md)
- [Phase 4OH Recovery From Why Probe Evidence](provenance/phase-4oh-recovery-from-why-probe.md)
- [Phase 4OK Plan Why Direct Task Closure Evidence](provenance/phase-4ok-plan-why-direct-task-closure.md)
- [Phase 4OM Goal Plan Recovery Probe Evidence](provenance/phase-4om-goal-plan-recovery-probe.md)
- [Phase 4OP Plan-Start Context Resource Probe Evidence](provenance/phase-4op-plan-start-context-resource-probe.md)
- [Phase 4OQ Goal-Start Context Resource Probe Evidence](provenance/phase-4oq-goal-start-context-resource-probe.md)
- [Phase 4OR Goal-Start Context Multiplan Resource Probe Evidence](provenance/phase-4or-goal-start-context-multiplan-resource-probe.md)
- [Phase 4OS Nested Plan Context Resource Probe Evidence](provenance/phase-4os-nested-plan-context-resource-probe.md)
- [Phase 4OT Cross-Goal Dependency Context Resource Probe Evidence](provenance/phase-4ot-cross-goal-dependency-context-resource-probe.md)
- [Phase 4OU Structural Reference Context Resource Probe Evidence](provenance/phase-4ou-structural-reference-context-resource-probe.md)
- [Phase 4OV Structural Reference Plan Target Context Resource Probe Evidence](provenance/phase-4ov-structural-reference-plan-target-context-resource-probe.md)
- [Phase 4OW Structural Reference Nested Plan Target Context Resource Probe Evidence](provenance/phase-4ow-structural-reference-nested-plan-target-context-resource-probe.md)
- [Phase 4OY Structural Reference Two-Level Nested Plan Target Context Resource Recovery Evidence](provenance/phase-4oy-structural-reference-two-level-nested-plan-target-context-resource-recovery.md)
- [Phase 4PE Compact Read-Only Resume Query Evidence](provenance/phase-4pe-compact-read-only-resume-query.md)

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
- [Bundle Local Directory Profile v0.1](architecture/bundle-local-profile-v0.1.md)
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
- [ADR-0422: Phase 4LG Blocked Handoff Recovery Dogfood](decisions/adr/0422-phase-4lg-blocked-handoff-recovery-dogfood.md)
- [ADR-0423: Phase 4LH Handoff Focus Why](decisions/adr/0423-phase-4lh-handoff-focus-why.md)
- [ADR-0424: Phase 4LI Claim Next Context Packet](decisions/adr/0424-phase-4li-claim-next-context-packet.md)
- [ADR-0425: Phase 4LJ Context AC and VR Packet Items](decisions/adr/0425-phase-4lj-context-ac-vr-packet.md)
- [ADR-0426: Phase 4LK Context Blocker Packet Items](decisions/adr/0426-phase-4lk-context-blocker-packet.md)
- [ADR-0427: Phase 4LL Context Goal/Plan Path Packet Items](decisions/adr/0427-phase-4ll-context-goal-plan-path-packet.md)
- [ADR-0428: Phase 4LM Context Attempt Detail Packet Items](decisions/adr/0428-phase-4lm-context-attempt-detail-packet.md)
- [ADR-0429: Phase 4LN Merge Lifecycle Dogfood](decisions/adr/0429-phase-4ln-merge-lifecycle-dogfood.md)
- [ADR-0430: Phase 4LO Bundle Portability Dogfood](decisions/adr/0430-phase-4lo-bundle-portability-dogfood.md)
- [ADR-0431: Phase 4LP Bundle Profile Contract](decisions/adr/0431-phase-4lp-bundle-profile-contract.md)
- [ADR-0432: Phase 4LQ Larger Store Portability Validation](decisions/adr/0432-phase-4lq-larger-store-portability-validation.md)
- [ADR-0433: Phase 4LR Context Path-Sensitive Knowledge](decisions/adr/0433-phase-4lr-context-path-sensitive-knowledge.md)
- [ADR-0434: Phase 4LS Context Packet Persistence](decisions/adr/0434-phase-4ls-context-packet-persistence.md)
- [ADR-0435: Phase 4LT Transition Rationale Context Projection](decisions/adr/0435-phase-4lt-transition-rationale-context.md)
- [ADR-0436: Phase 4LU Claim Guard Recovery Hints](decisions/adr/0436-phase-4lu-claim-guard-recovery-hints.md)
- [ADR-0437: Phase 4LV Another Project Read-Only Dogfood](decisions/adr/0437-phase-4lv-another-project-dogfood.md)
- [ADR-0438: Phase 4LW Claim Transfer and Takeover Dogfood](decisions/adr/0438-phase-4lw-claim-transfer-takeover-dogfood.md)
- [ADR-0439: Phase 4LX Resource Scope Path Normalization](decisions/adr/0439-phase-4lx-resource-scope-path-normalization.md)
- [ADR-0440: Phase 4LY Resource Scope File Observation](decisions/adr/0440-phase-4ly-resource-scope-file-observation.md)
- [ADR-0441: Phase 4LZ Stable Error Code Output](decisions/adr/0441-phase-4lz-stable-error-code-output.md)
- [ADR-0442: Phase 4MA Why Explanation Dogfood](decisions/adr/0442-phase-4ma-why-explanation-dogfood.md)
- [ADR-0443: Phase 4MB Resource Unavailable Error Dogfood](decisions/adr/0443-phase-4mb-resource-unavailable-error-dogfood.md)
- [ADR-0444: Phase 4MC Local File Cache Refresh](decisions/adr/0444-phase-4mc-local-file-cache-refresh.md)
- [ADR-0445: Phase 4MD Local File Path Prefix Refresh](decisions/adr/0445-phase-4md-local-file-path-prefix-refresh.md)
- [ADR-0446: Phase 4ME Local File Glob Refresh](decisions/adr/0446-phase-4me-local-file-glob-refresh.md)
- [ADR-0447: Phase 4MF Git Worktree Resource Refresh](decisions/adr/0447-phase-4mf-git-worktree-resource-refresh.md)
- [ADR-0448: Phase 4MG Resource Basis Cache Refresh](decisions/adr/0448-phase-4mg-resource-basis-cache-refresh.md)
- [ADR-0449: Phase 4MH Why Evolution Deferred Family](decisions/adr/0449-phase-4mh-why-evolution-deferred-family.md)
- [ADR-0450: Phase 4MI Shared Claim Dogfood](decisions/adr/0450-phase-4mi-shared-claim-dogfood.md)
- [ADR-0451: Phase 4MJ Clap Error Normalization](decisions/adr/0451-phase-4mj-clap-error-normalization.md)
- [ADR-0452: Phase 4MK Per-Code Recovery Guide](decisions/adr/0452-phase-4mk-per-code-recovery-guide.md)
- [ADR-0453: Phase 4ML Why Causal Anchor Traversal](decisions/adr/0453-phase-4ml-why-causal-anchor-traversal.md)
- [ADR-0454: Phase 4MM Merge Maturity Dogfood](decisions/adr/0454-phase-4mm-merge-maturity-dogfood.md)
- [ADR-0455: Phase 4MN Batch Basis Refresh](decisions/adr/0455-phase-4mn-batch-basis-refresh.md)
- [ADR-0456: Phase 4MO JSON Error Output](decisions/adr/0456-phase-4mo-json-error-output.md)
- [ADR-0457: Phase 4MP V1 Release Gate Matrix](decisions/adr/0457-phase-4mp-v1-release-gate-matrix.md)
- [ADR-0458: Phase 4MQ Branch Diff Dogfood](decisions/adr/0458-phase-4mq-branch-diff-dogfood.md)
- [ADR-0459: Phase 4MR Shared Claim Write-Mode Dogfood](decisions/adr/0459-phase-4mr-shared-claim-write-mode-dogfood.md)
- [ADR-0460: Phase 4MS Handoff Consumption Write-Mode Dogfood](decisions/adr/0460-phase-4ms-handoff-consumption-write-mode-dogfood.md)
- [ADR-0461: Phase 4MT External Merge Write-Mode Dogfood](decisions/adr/0461-phase-4mt-external-merge-write-mode-dogfood.md)
- [ADR-0462: Phase 4MU Preexisting External Merge Write-Mode Dogfood](decisions/adr/0462-phase-4mu-preexisting-external-merge-write-mode-dogfood.md)
- [ADR-0463: Phase 4MV Git Rename Resource Policy Dogfood](decisions/adr/0463-phase-4mv-git-rename-resource-policy-dogfood.md)
- [ADR-0464: Phase 4MW Git Symlink Resource Policy Dogfood](decisions/adr/0464-phase-4mw-git-symlink-resource-policy-dogfood.md)
- [ADR-0465: Phase 4MX Git Submodule Resource Policy Dogfood](decisions/adr/0465-phase-4mx-git-submodule-resource-policy-dogfood.md)
- [ADR-0466: Phase 4MY Git Sparse Checkout Resource Policy Dogfood](decisions/adr/0466-phase-4my-git-sparse-checkout-resource-policy-dogfood.md)
- [ADR-0467: Phase 4MZ Resource Case-Folding Policy Dogfood](decisions/adr/0467-phase-4mz-resource-case-folding-policy-dogfood.md)
- [ADR-0468: Phase 4NA Resource Re-Observation Scheduling Policy Dogfood](decisions/adr/0468-phase-4na-resource-reobservation-scheduling-policy-dogfood.md)
- [ADR-0469: Phase 4NB Goal/Plan/Task and AC/VR Recovery Dogfood](decisions/adr/0469-phase-4nb-goal-plan-task-ac-vr-recovery-dogfood.md)
- [ADR-0470: Phase 4NC Why Epistemic Explanation](decisions/adr/0470-phase-4nc-why-epistemic-explanation.md)
- [ADR-0471: Phase 4ND Maintained Store Portability Dogfood](decisions/adr/0471-phase-4nd-maintained-store-portability-dogfood.md)
- [ADR-0472: Phase 4NE Larger Maintained Store Workload Validation](decisions/adr/0472-phase-4ne-larger-maintained-store-workload-validation.md)
- [ADR-0473: Phase 4NF Self-Contained Run Log Summary](decisions/adr/0473-phase-4nf-self-contained-run-log-summary.md)
- [ADR-0474: Phase 4NG Why Evolution Operation Projection](decisions/adr/0474-phase-4ng-why-evolution-operation-projection.md)
- [ADR-0475: Phase 4NH Why Evolution Subject Detail Projection](decisions/adr/0475-phase-4nh-why-evolution-subject-detail-projection.md)
- [ADR-0476: Phase 4NI Operator Recovery Maturity Dogfood](decisions/adr/0476-phase-4ni-operator-recovery-maturity-dogfood.md)
- [ADR-0477: Phase 4NJ Why Subject Evolution Projection](decisions/adr/0477-phase-4nj-why-subject-evolution-projection.md)
- [ADR-0478: Phase 4NK Why Operation-Local Entity Detail](decisions/adr/0478-phase-4nk-why-operation-local-entity-detail.md)
- [ADR-0479: Phase 4NL Why Relation-Subject Endpoint Evolution](decisions/adr/0479-phase-4nl-why-relation-subject-endpoint-evolution.md)
- [ADR-0480: Phase 4NM Why Knowledge Relation Endpoint Evolution](decisions/adr/0480-phase-4nm-why-knowledge-relation-endpoint-evolution.md)
- [ADR-0481: Phase 4NN Context Resource Basis Packet Recovery](decisions/adr/0481-phase-4nn-context-resource-basis-packet.md)
- [ADR-0482: Phase 4NO Why Relation Create Endpoint Evolution](decisions/adr/0482-phase-4no-why-relation-create-endpoint-evolution.md)
- [ADR-0483: Phase 4NP Blocked Dependency Resource Context](decisions/adr/0483-phase-4np-blocked-dependency-resource-context.md)
- [ADR-0484: Phase 4NQ Why Task Scheduling Relation Evolution](decisions/adr/0484-phase-4nq-why-task-scheduling-relation-evolution.md)
- [ADR-0485: Phase 4NR Why Primary Containment Relation Evolution](decisions/adr/0485-phase-4nr-why-primary-containment-relation-evolution.md)
- [ADR-0486: Phase 4NU Focused Plan Peer Resource Context](decisions/adr/0486-phase-4nu-focused-plan-peer-resource-context.md)
- [ADR-0487: Phase 4NV Why Verifies Relation Evolution](decisions/adr/0487-phase-4nv-why-verifies-relation-evolution.md)
- [ADR-0488: Phase 4NW Why Evidenced-By Relation Evolution](decisions/adr/0488-phase-4nw-why-evidenced-by-relation-evolution.md)
- [ADR-0489: Phase 4NX Why Evidence Subject Evidenced-By Evolution](decisions/adr/0489-phase-4nx-why-evidence-subject-evidenced-by-evolution.md)
- [ADR-0490: Phase 4NY Why Knowledge Exposure Derived-From Evolution](decisions/adr/0490-phase-4ny-why-knowledge-exposure-derived-from-evolution.md)
- [ADR-0491: Phase 4NZ Focused Same-Goal Cross-Plan Resource Context](decisions/adr/0491-phase-4nz-focused-same-goal-cross-plan-resource-context.md)
- [ADR-0492: Phase 4OC Focus-Set Unsupported Kind Fail-Fast](decisions/adr/0492-phase-4oc-focus-set-unsupported-kind-fail-fast.md)
- [ADR-0493: Phase 4OE Task Closeout Why Closure Chain](decisions/adr/0493-phase-4oe-task-closeout-why-closure-chain.md)
- [ADR-0494: Phase 4OG Task Why Resource Basis Closure](decisions/adr/0494-phase-4og-task-why-resource-basis-closure.md)
- [ADR-0495: Phase 4OK Plan Why Direct Task Closure](decisions/adr/0495-phase-4ok-plan-why-direct-task-closure.md)
- [ADR-0496: Phase 4PE Compact Read-Only Resume Query](decisions/adr/0496-phase-4pe-compact-read-only-resume-query.md)
- [ADR-0497: Work-Governance Cutover P0 Entrypoints](decisions/adr/0497-work-governance-cutover-p0-entrypoints.md)
- [ADR-0498: Plan Evolve Supersede](decisions/adr/0498-plan-evolve-supersede.md)
- [ADR-0499: Mechanical Authorization Receipts](decisions/adr/0499-mechanical-authorization-receipts.md)
- [ADR-0500: Read-Only Closeout Inspection Projection](decisions/adr/0500-closeout-inspect-readonly-projection.md)
- [ADR-0501: Stable Configuration, Standalone Cognition, and Skill Distribution](decisions/adr/0501-standalone-cognition-config-and-skill.md)

These documents specify what WorkVCS currently means. Implementation readiness
is tracked separately in the V1 readiness ledger and release-maturity status is
tracked in the V1 release gate matrix. The local Rust V0.1 implementation
remains local and in progress, while current evidence records bounded V1 local
release maturity as ready and dogfood-complete. Release, tag, push, deploy,
remote, production, credential, and global installation actions remain separate
authority decisions. `scripts/package-workvcs.sh` provides local binary
packaging and an explicit install/overwrite path for `workvcs`, but running a
real system install remains a separate authority decision.

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
