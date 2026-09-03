# Phase 4NY Why Knowledge Exposure Derived From Evolution Evidence

Status: current local evidence
Date: 2026-09-03

## Scope

Phase 4NY advances the Context resolver, packets, and `why` explanations
release gate by making direct `knowledge_exposure_derived_from` Relation
creation visible as endpoint evolution from both:

```text
workvcs why --entity <Knowledge> --relation-kind knowledge_exposure_derived_from
workvcs why --exposure <KnowledgeExposure> --relation-kind knowledge_exposure_derived_from
```

The slice is intentionally narrow. It adds direct `knowledge.relation.create`
endpoint evolution only for current `knowledge_exposure_derived_from` Relations
when the queried subject is the source Knowledge endpoint or target
KnowledgeExposure endpoint. It does not add `why --relation`, full
relation-subject traversal, Store schema changes, CLI flags, ContextPacket
behavior, release, Push, tag, deployment, remote/cloud/V2 scope, GUI/TUI
behavior, distributed collaboration, or Agent orchestration.

## Gap Proof

The pre-change gap used current `main`, a temporary local Store, and public CLI
commands.

Scenario:

```text
create Workspace
create source Knowledge
create workspace-local adopted Knowledge
create local KnowledgeExposure from the source Knowledge
create explicit store knowledge-exposure-derived-from-link from adopted Knowledge to exposure
run workvcs why for the adopted Knowledge and exposure endpoints
```

Observed before implementation:

```text
log_dir=/tmp/workvcs-4ny-knowledge-exposure-derived-from-probe-20260903T065440Z
head_commit_id=7eb9fb5798dbf4b80bafd0ea490bff2b875b8323
source_actual_relation_edges=1
source_actual_relation_kind_0=knowledge_exposure_derived_from
source_actual_relation_direction_0=outgoing
source_actual_evolution_change_operations=0
source_expected_error_code=query_invalid
source_expected_message=query invalid: why evolution change operations 0 does not match expected 1
exposure_actual_relation_edges=1
exposure_actual_relation_kind_0=knowledge_exposure_derived_from
exposure_actual_relation_direction_0=incoming
exposure_actual_evolution_change_operations=0
exposure_expected_error_code=query_invalid
exposure_expected_message=query invalid: why evolution change operations 0 does not match expected 1
probe_status=PASS
```

The current relation edge was visible from both endpoints, but the direct
`knowledge.relation.create` operation that created that relation was not visible
through existing `why` evolution output.

## Implementation

Changed:

```text
crates/workvcs-core/src/history/why.rs
crates/workvcs-core/tests/why_knowledge_exposure_provenance_phase4ao.rs
crates/workvcs-cli/src/main.rs
docs/decisions/adr/0490-phase-4ny-why-knowledge-exposure-derived-from-evolution.md
docs/provenance/phase-4ny-why-knowledge-exposure-derived-from-evolution.md
docs/provenance/v1-readiness-ledger.md
docs/provenance/v1-release-gate-matrix.md
```

The implementation lets KnowledgeExposure subjects enter the existing direct
relation evolution scan. It keeps Entity-only direct Entity membership
evolution unchanged, and it resolves direct relation subject detail only for the
current `knowledge_exposure_derived_from` Relation whose source Knowledge or
target KnowledgeExposure endpoint matches the query subject.

No Store schema, CLI flag, public output field, ContextPacket behavior,
Knowledge semantics, KnowledgeExposure semantics, or adoption semantics changed.

## Focused Validation

Focused tests passed after implementation:

```text
log_dir=/tmp/workvcs-4ny-focused-validation-20260903T063248Z
cargo_fmt_apply=PASS
cargo_fmt_check=PASS
core_knowledge_exposure=PASS
cli_knowledge_exposure=PASS
FOCUSED_VALIDATION=PASS
```

## Dogfood Proof

The public CLI dogfood uses a temporary local Store and the built
`target/debug/workvcs` binary.

```text
log_dir=/tmp/workvcs-4ny-why-knowledge-exposure-derived-from-evolution-20260903T063526Z/dogfood
phase4ny_dogfood=PASS
workspace_id=01a065fa-ed80-7a33-a7f3-f2bb848e6191
branch_id=01a065fa-ed80-7a33-a7f3-f2e2b96bb476
source_knowledge_id=01a065fa-ed9a-7b00-bda5-5f262b53fd30
adopted_knowledge_id=01a065fa-edb1-7623-84e4-dd77e3a63144
exposure_id=01a065fa-edd8-7710-bd82-f5515d02955f
relation_id=01a065fa-edeb-7c33-8965-a5efb50f6511
relation_commit_id=01a065fa-edec-7450-9a91-a77f429c01e8
source_relation_edges=1
source_relation_kind=knowledge_exposure_derived_from
source_relation_direction=outgoing
source_evolution_change_operations=1
source_evolution_match_expected=true
source_evolution_operation_type=knowledge.relation.create
source_evolution_subject_relation_kind=knowledge_exposure_derived_from
source_evolution_source_matches_knowledge=true
source_evolution_target_matches_exposure=true
exposure_relation_edges=1
exposure_relation_kind=knowledge_exposure_derived_from
exposure_relation_direction=incoming
exposure_evolution_change_operations=1
exposure_evolution_match_expected=true
exposure_evolution_operation_type=knowledge.relation.create
exposure_evolution_subject_relation_kind=knowledge_exposure_derived_from
exposure_evolution_source_matches_knowledge=true
exposure_evolution_target_matches_exposure=true
```

## Final Validation

Final validation passed:

```text
log_dir=/tmp/workvcs-4ny-why-knowledge-exposure-derived-from-evolution-20260903T063552Z/final-validation
validation_status=PASS
status_0_count=18
```

## Interpretation

Phase 4NY closes the concrete KnowledgeExposure derived-from direct endpoint
evolution gap shown by the current CLI probe. Operators can now inspect the
direct `knowledge.relation.create` operation behind the visible
`knowledge_exposure_derived_from` relation from both the source Knowledge
endpoint and the target KnowledgeExposure endpoint.

The proof is still bounded. It does not show `why --relation`, full
relation-subject traversal, multi-hop/full evolution traversal, broader causal
traversal, broader context/Resource resolver maturity, release-candidate
validation, or release authorization.

## Release Gate Impact

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4NY advances the gate by closing direct
`knowledge_exposure_derived_from` creation endpoint evolution for queried
Knowledge and KnowledgeExposure endpoints.

The overall release decision remains:

```text
V1_RELEASE_READY=false
V0_1_DOGFOOD_COMPLETE=false
RELEASE_CANDIDATE_ALLOWED=false
```
