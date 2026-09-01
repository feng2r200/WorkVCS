# Phase 4NH Why Evolution Subject Detail Evidence

Status: current local evidence
Date: 2026-09-02

## Scope

Phase 4NH advances the Context resolver, packets, and `why` explanations gate
by closing one concrete continuation gap left after Phase 4NG: `why` could
show direct ChangeOperation subjects for causal-anchor ChangeSets, but the
subject UUIDs still required separate lookups to understand the changed Record
or Relation.

This slice changes Rust code, focused tests, CLI rendering, operator guidance,
and release evidence docs. It does not change Store schema, stored Relation
semantics, mutation semantics, release state, Push state, tags, remote state,
V2 scope, transcript parsing, LLM extraction, ranking, or Agent orchestration.

## Contract Inspection

The current AGENTS.md, V1 readiness ledger, release gate matrix, ADR-0474, and
current `why` implementation were inspected before the implementation.

Key boundaries confirmed:

```text
target_gate=Context resolver, packets, and why explanations
gate_status_before=Partial
blocking_gap=direct_evolution_operation_subject_ids_require_separate_lookup
projection_source=current_WorkState_entity_and_relation_projections
existing_operation_fields_preserved=true
no_go=schema_change,new_relation_semantics,mutation_semantics,full_evolution_traversal,broader_causal_traversal,context_resolver_change,resource_resolver_change,llm,remote,release,push,tag,V2
```

## Implementation

Changed implementation files:

```text
crates/workvcs-core/src/history/why.rs
crates/workvcs-core/src/history/mod.rs
crates/workvcs-core/src/lib.rs
crates/workvcs-cli/src/main.rs
```

The core result now attaches optional `subject_detail` to each
`WhyEvolutionChangeOperation`.

Entity detail is emitted only when the ChangeOperation subject Entity is
current in the target WorkState. The output includes the Entity kind and
version, plus statement JSON for current Record and Knowledge subjects.

Relation detail is emitted only when the ChangeOperation subject Relation is
current in the target WorkState and belongs to an existing relation family
already recognized by `why`. The output includes relation kind, version, label,
source endpoint, target endpoint, and state digest.

If a subject is not current or not recognized by the current `why` projection,
the detail is omitted and the existing Phase 4NG operation fields remain
unchanged.

The CLI renders:

```text
evolution_change_operation.<i>.subject_detail_kind=entity
evolution_change_operation.<i>.subject_entity_kind=<entity_kind>
evolution_change_operation.<i>.subject_entity_version_id=<entity_version_id>
evolution_change_operation.<i>.subject_statement_json=<statement_json>
evolution_change_operation.<i>.subject_detail_kind=relation
evolution_change_operation.<i>.subject_relation_kind=<relation_kind>
evolution_change_operation.<i>.subject_relation_version_id=<relation_version_id>
evolution_change_operation.<i>.subject_relation_label=<label>
evolution_change_operation.<i>.subject_relation_source_kind=<entity|evidence|knowledge_exposure>
evolution_change_operation.<i>.subject_relation_source_entity_kind=<entity_kind>
evolution_change_operation.<i>.subject_relation_source_entity_id=<entity_id>
evolution_change_operation.<i>.subject_relation_source_evidence_id=<evidence_id>
evolution_change_operation.<i>.subject_relation_source_exposure_id=<exposure_id>
evolution_change_operation.<i>.subject_relation_target_kind=<entity|evidence|knowledge_exposure>
evolution_change_operation.<i>.subject_relation_target_entity_kind=<entity_kind>
evolution_change_operation.<i>.subject_relation_target_entity_id=<entity_id>
evolution_change_operation.<i>.subject_relation_target_evidence_id=<evidence_id>
evolution_change_operation.<i>.subject_relation_target_exposure_id=<exposure_id>
evolution_change_operation.<i>.subject_relation_state_digest=<state_digest>
```

Existing `evolution_change_operations` and
`evolution_change_operation.<i>.*` operation fields from Phase 4NG are
preserved.

## Focused Validation

Focused validation passed:

```text
cargo fmt --all -- --check
cargo test -p workvcs-core --test why_evolution_operation_projection_phase4ng
cargo test -p workvcs-cli cli_supersedes_decision_record_atomically -- --nocapture
```

Coverage:

```text
core_entity_subject_detail_record_statement=pass
core_relation_subject_detail_record_supersedes=pass
core_relation_subject_detail_record_derived_from=pass
core_non_anchor_evolution_operations_empty=pass
cli_entity_subject_detail_rendering=pass
cli_relation_subject_detail_rendering=pass
cli_relation_filter_limit_preserves_subject_detail=pass
```

## Dogfood Proof

The dogfood run created a Store with the previous main binary and queried the
same Store with both the previous main binary and the Phase 4NH worktree
binary.

Summary:

```text
dogfood_tmpdir=/tmp/workvcs-4nh-dogfood.F7Wuw2
dogfood_store=/tmp/workvcs-4nh-dogfood.F7Wuw2/workvcs.sqlite
dogfood_commit_id=01a05efe-2830-7a00-8d9a-6a8380ae6d04
dogfood_finding_record_entity_id=01a05efe-2287-7c63-89bf-91a9ca593f46
old_evolution_change_operations=3
old_op0_subject_family=entity
old_op0_subject_object_id=01a05efe-1fc9-7633-9116-9fcb51308471
old_subject_statement_json_present=false
old_subject_relation_kind_present=false
new_evolution_change_operations=3
new_evolution_match=true
new_op0_subject_detail_kind=entity
new_op0_subject_entity_kind=record
new_op0_subject_statement_json="Use optimistic writes"
new_op1_subject_detail_kind=relation
new_op1_subject_relation_kind=record_supersedes
new_op1_subject_relation_source_entity_id=01a05efe-2551-73a2-9e9d-c521bc617ff4
new_op1_subject_relation_target_entity_id=01a05efe-1fc9-7633-9116-9fcb51308471
new_op2_subject_relation_kind=record_derived_from
new_op2_subject_relation_source_entity_id=01a05efe-2551-73a2-9e9d-c521bc617ff4
new_op2_subject_relation_target_entity_id=01a05efe-2287-7c63-89bf-91a9ca593f46
```

This proves the Phase 4NH binary preserves the Phase 4NG operation count on an
old-main-created Store while adding readable subject detail for the current
Record and Relation subjects in the Decision supersede workflow.

## Readiness Impact

Phase 4NH narrows the `why` evolution gap by making already-rendered direct
ChangeOperation subjects self-explanatory for current recognized subjects. It
advances the Context resolver, packets, and `why` explanations gate, but the
gate remains `Partial` and blocking.

Full evolution traversal, broader causal traversal, broader context/Resource
resolver maturity, broader operator recovery maturity, release-candidate
validation, and explicit release authorization remain open.

## Final Validation

Final validation passed on 2026-09-02 after the independent-review doc fix and
the narrowed unsupported-Entity-kind best-effort implementation fix:

```text
git diff --check
cargo fmt --all -- --check
scripts/validate-schema-v0.1.sh
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
scripts/smoke-v0.1-cli-workflow.sh
```

The full test run included 148 CLI unit tests, core unit tests, all current
core integration tests, and the updated
`why_evolution_operation_projection_phase4ng` integration test. The repository
smoke ended with `smoke_result=passed`.

## Independent Review

PASS after one medium documentation finding was corrected. The independent
read-only review found no blocker or high findings.

The medium finding was that the ADR and provenance examples used the shortened
endpoint kind value `exposure` while the CLI actually renders
`knowledge_exposure`. Both documents now use
`<entity|evidence|knowledge_exposure>`.

The review confirmed that the core projection is attached only to already
selected causal-anchor ChangeSet direct operations, checks current WorkState
membership before rendering detail, uses existing `why` relation projection
families, and keeps CLI output additive.

Residual risk: coverage is centered on the Decision supersede causal-record
scenario. Knowledge statement detail, Evidence endpoint detail, and
KnowledgeExposure endpoint detail remain candidates for future tests only if
later dogfood exposes a concrete need.
