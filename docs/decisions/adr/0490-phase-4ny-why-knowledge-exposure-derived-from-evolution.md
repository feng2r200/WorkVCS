# ADR-0490: Phase 4NY Why Knowledge Exposure Derived From Evolution

Status: Accepted
Date: 2026-09-03

## Context

ADR-0144 made adopted Knowledge provenance visible as current
`knowledge_exposure_derived_from` relation edges in `why`. ADR-0145 added the
complementary `KnowledgeExposure` query subject so operators can inspect the
incoming provenance edge from the exposure side.

Before this slice, both endpoint views could show the current
`knowledge_exposure_derived_from` relation edge, but neither endpoint exposed
the direct `knowledge.relation.create` operation that created that Relation
through existing `why` evolution fields.

The pre-change public CLI probe used current `main` commit
`7eb9fb5798dbf4b80bafd0ea490bff2b875b8323`, a temporary Store, one source
Knowledge, one workspace-local adopted Knowledge, one local KnowledgeExposure,
one explicit `store knowledge-exposure-derived-from-link`, and the current
public CLI:

```text
log_dir=/tmp/workvcs-4ny-knowledge-exposure-derived-from-probe-20260903T065440Z
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

## Decision

`why --entity <Knowledge> --relation-kind knowledge_exposure_derived_from` and
`why --exposure <KnowledgeExposure> --relation-kind
knowledge_exposure_derived_from` now project the direct
`knowledge.relation.create` operation for current
`knowledge_exposure_derived_from` Relations attached to the queried endpoint
through existing `evolution_change_operations` fields.

The operation subject detail includes:

- relation kind `knowledge_exposure_derived_from`;
- relation version id;
- source Knowledge endpoint;
- target KnowledgeExposure endpoint;
- relation state digest.

The implementation lets KnowledgeExposure subjects enter the existing direct
relation evolution scan while keeping direct Entity membership evolution limited
to Entity subjects. It resolves operation subject detail only for the current
`knowledge_exposure_derived_from` Relation whose source or target endpoint
matches the query subject.

## Non-Goals

- No `why --relation` command or subject support.
- No Store schema change.
- No Knowledge, KnowledgeExposure, or exposure-adoption semantics change.
- No CLI flag or public output field addition.
- No ContextPacket behavior change.
- No full relation-subject traversal beyond this direct endpoint slice.
- No multi-hop/full evolution traversal or broader causal traversal.
- No broader context/Resource resolver behavior change.
- No release-candidate, release, Push, tag, deployment, remote, cloud, V2,
  GUI/TUI, distributed collaboration, or Agent orchestration action.

## Evidence

Focused validation passed after implementation:

```text
log_dir=/tmp/workvcs-4ny-focused-validation-20260903T063248Z
cargo_fmt_apply=PASS
cargo_fmt_check=PASS
core_knowledge_exposure=PASS
cli_knowledge_exposure=PASS
FOCUSED_VALIDATION=PASS
```

The public CLI dogfood run used a temporary Store and the built
`target/debug/workvcs` binary:

```text
log_dir=/tmp/workvcs-4ny-why-knowledge-exposure-derived-from-evolution-20260903T063526Z/dogfood
phase4ny_dogfood=PASS
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

Final validation passed:

```text
log_dir=/tmp/workvcs-4ny-why-knowledge-exposure-derived-from-evolution-20260903T063552Z/final-validation
validation_status=PASS
status_0_count=18
```

Detailed evidence is recorded in
`docs/provenance/phase-4ny-why-knowledge-exposure-derived-from-evolution.md`.

## Consequences

Operators can now use the same `why` surface that shows the current
`knowledge_exposure_derived_from` edge to inspect the direct
`knowledge.relation.create` operation behind that edge from either the source
Knowledge endpoint or the target KnowledgeExposure endpoint.

The Context resolver, packets, and `why` explanations release gate remains
`Partial`. Phase 4NY closes this concrete KnowledgeExposure derived-from direct
endpoint evolution gap, but full relation-subject traversal beyond direct
endpoint slices, multi-hop/full evolution traversal, broader causal traversal,
broader context/Resource resolver maturity, and release-candidate validation
remain open.
