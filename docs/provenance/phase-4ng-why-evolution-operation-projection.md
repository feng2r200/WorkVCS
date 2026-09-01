# Phase 4NG Why Evolution Operation Projection Evidence

Status: current local evidence
Date: 2026-09-02

## Scope

Phase 4NG advances the Context resolver, packets, and `why` explanations gate
by closing one concrete continuation gap: `why` could already report that a
queried Entity anchored a first-parent-reachable ChangeSet, but it did not show
the direct ChangeOperation subjects for that ChangeSet.

This slice changes Rust code, focused tests, CLI rendering, operator guidance,
and release evidence docs. It does not change Store schema, stored Relation
semantics, mutation semantics, release state, Push state, tags, remote state,
V2 scope, transcript parsing, LLM extraction, ranking, or Agent orchestration.

## Contract Inspection

The current AGENTS.md, V1 readiness ledger, release gate matrix, ADR-0449,
ADR-0453, ADR-0470, and current `why` implementation were inspected before the
implementation.

Key boundaries confirmed:

```text
target_gate=Context resolver, packets, and why explanations
gate_status_before=Partial
blocking_gap=causal_anchor_changeset_direct_operation_subjects_require_changeset_operations_lookup
projection_source=existing_changeset_operations_for_already_rendered_causal_anchor_changesets
relation_filters_apply_to=relation_edges
relation_filters_hide_evolution_change_operations=false
no_go=schema_change,new_relation_semantics,full_evolution_traversal,broader_causal_traversal,llm,remote,release,push,tag,V2
```

## Pre-Change Gap Proof

The pre-change gap run is recorded in:

```text
.work-governance/runtime/logs/phase-4ng-pre-gap.4u2SjL
```

Summary:

```text
phase_4ng_pre_gap_result=PASS
why_causal_anchor_changesets=1
why_deferred_relation_families=1
changeset_operations=3
pre_gap_evolution_operation_fields_absent=true
store=.work-governance/runtime/logs/phase-4ng-pre-gap.4u2SjL/workvcs.sqlite
branch_id=01a05ece-ed9f-7fd1-a259-98c2f34234c9
supersede_commit_id=01a05ece-edf1-7c73-8eef-510fcfd3ccbf
supersede_changeset_id=01a05ece-edf1-7c73-8eef-50f9d4091234
finding_record_entity_id=01a05ece-edc9-7ed2-a81f-5da5a5fc785f
```

The first harness attempt failed before the semantic scenario because the
runtime log parent directory was missing. The successful run created the parent
directory and proved the actual pre-change gap.

## Implementation

Changed implementation files:

```text
crates/workvcs-core/src/history/why.rs
crates/workvcs-core/src/history/mod.rs
crates/workvcs-core/src/lib.rs
crates/workvcs-cli/src/main.rs
```

The core result now includes `evolution_change_operations`. The projection is
populated by reading direct ChangeOperations for each causal-anchor ChangeSet
already selected by `why`.

The CLI renders:

```text
evolution_change_operations=<N>
evolution_change_operation.<i>.commit_id=<COMMIT_ID>
evolution_change_operation.<i>.changeset_id=<CHANGESET_ID>
evolution_change_operation.<i>.changeset_operation_type=<OPERATION_TYPE>
evolution_change_operation.<i>.changeset_operation_schema_version=<VERSION>
evolution_change_operation.<i>.operation_id=<OPERATION_ID>
evolution_change_operation.<i>.ordinal=<ORDINAL>
evolution_change_operation.<i>.subject_family=<entity|relation>
evolution_change_operation.<i>.subject_object_id=<ENTITY_ID|RELATION_ID>
evolution_change_operation.<i>.operation_payload_digest=<DIGEST>
evolution_change_operation.<i>.operation_payload_size_bytes=<BYTES>
```

`--expected-evolution-change-operations COUNT` was added for CLI dogfood
assertions.

## Focused Validation

Focused validation passed:

```text
cargo fmt --all -- --check
cargo test -p workvcs-core --test why_evolution_operation_projection_phase4ng
cargo test -p workvcs-core --test record_decision_supersede_causal_phase3bh
cargo test -p workvcs-cli cli_supersedes_decision_record_atomically
```

Coverage:

```text
core_causal_anchor_projects_direct_evolution_operations=pass
core_non_anchor_evolution_operations_empty=pass
core_supersede_causal_regression=pass
cli_evolution_operation_rendering=pass
cli_expected_evolution_change_operations_assertion=pass
cli_relation_filter_limit_preserves_evolution_projection=pass
```

## Post-Change Dogfood

The post-change dogfood run is recorded in:

```text
.work-governance/runtime/logs/phase-4ng-post-dogfood.nro8Ek/summary.txt
```

Summary:

```text
phase=4NG-post-change-dogfood
status=pass
store=.work-governance/runtime/logs/phase-4ng-post-dogfood.nro8Ek/workvcs.sqlite
branch_id=01a05edd-b5d7-7180-9388-a47f45240166
supersede_commit_id=01a05edd-b62e-7ce0-9ce0-a0daf7f3f7c0
supersede_changeset_id=01a05edd-b62e-7ce0-9ce0-a0c89d0fe460
prior_record_entity_id=01a05edd-b5ef-78e3-8d3b-0501dd34e66a
finding_record_entity_id=01a05edd-b606-72e1-a47b-f7a92079ecd8
changeset_operations=3
changeset_operations_match_expected=true
why_prior_causal_anchor_changesets=0
why_prior_evolution_change_operations=0
why_prior_expected_match=true
why_finding_causal_anchor_changesets=1
why_finding_evolution_change_operations=3
why_finding_operation_0_subject_family=entity
why_finding_operation_0_subject_object_id=01a05edd-b5ef-78e3-8d3b-0501dd34e66a
why_finding_operation_1_subject_family=relation
why_finding_operation_2_subject_family=relation
why_finding_deferred_relation_family_0=evolution
why_filtered_relation_edges=1
why_filtered_evolution_change_operations=3
why_filtered_expected_match=true
single_why_command_operation_subject_lookup=pass
release_state_changed=false
```

This proves a single `workvcs why` command can now return the causal-anchor
ChangeSet and its direct operation subjects for the Decision supersede
workflow.

## Readiness Impact

Phase 4NG narrows the `why` evolution gap by making direct ChangeOperation
subjects visible for already-detected causal-anchor ChangeSets. It advances
the Context resolver, packets, and `why` explanations gate, but the gate
remains `Partial` and blocking.

Full evolution traversal, broader causal traversal, broader context/Resource
resolver maturity, broader operator recovery maturity, release-candidate
validation, and explicit release authorization remain open.

## Final Validation

Final validation passed on 2026-09-02:

```text
git diff --check
cargo fmt --all -- --check
scripts/validate-schema-v0.1.sh
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
scripts/smoke-v0.1-cli-workflow.sh
```

The full test run included 148 CLI unit tests, core unit tests, all current
core integration tests, and the new
`why_evolution_operation_projection_phase4ng` integration test. The repository
smoke ended with `smoke_result=passed`.

## Independent Review

PASS: independent read-only review found no blocker, high, or medium issues
for the scoped implementation, CLI output, tests, ADR, provenance, operator
guide, readiness ledger, and release gate matrix.

The review confirmed that the core projection is derived only from already
selected causal-anchor ChangeSets, the CLI fields and expected-count assertion
are stable and covered, release false markers remain in the matrix, the
Context/why gate remains `Partial` and blocking, and full evolution traversal
remains outside this slice.

Residual risk: coverage is centered on the Decision supersede causal-record
scenario. Multiple causal-anchor ChangeSet aggregation and explicit
expected-count mismatch error text remain candidates for future tests only if
later dogfood exposes a concrete need.
