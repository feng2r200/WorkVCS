# Phase 4NC Why Epistemic Explanation Evidence

Date: 2026-09-02

## Scope

Phase 4NC advances the release-maturity gate for Context resolver, packets, and
`why` explanations by closing one concrete dogfood gap: direct epistemic
relation edges were visible, but the Record or Knowledge statements connected
by those edges were not visible in the same `why` output.

This slice changes Rust code, focused tests, CLI rendering, and documentation.
It does not change the Store schema, stored Relation semantics, relation
creation commands, deferred-family semantics, traversal depth, release state,
Push state, tags, remote state, V2 scope, transcript parsing, LLM extraction,
ranking, or Agent orchestration.

## Contract Inspection

The current readiness ledger, release gate matrix, typed relationship domain
rules, ADR-0177, and ADR-0453 were inspected before implementation.

Key boundaries confirmed:

```text
target_gate=Context resolver, packets, and why explanations
gate_status_before=Partial
blocking_gap=direct epistemic explanation requires statement lookup outside why
canonical_epistemic_relations=supports,contradicts,validates,invalidates
adr_0177_boundary=no broad Epistemic deferred-family marker
adr_0453_pattern=read-only projection without stored relation/schema change
no_go=schema_change,new_relation_semantics,traversal,llm,remote,release,push,tag,V2
```

## Pre-Change Gap Dogfood

The pre-change gap run is recorded in:

```text
/tmp/workvcs-4nc-why-epistemic-gap-20260901T185036Z/summary.txt
```

Summary:

```text
phase=4NC-pre-change-gap
store=/tmp/workvcs-4nc-why-epistemic-gap-20260901T185036Z/store.workvcs
record_id=01a05e4f-1753-7eb0-877e-09c3f5a5c05a
knowledge_id=01a05e4f-1740-7e52-8740-1aa82212cc6e
commit_id=01a05e4f-1767-7f10-840a-e17eb2202244
why_relation_edges=1
why_epistemic_statement_fields=absent
record_show_statement_field=present
knowledge_show_statement_field=present
requires_cross_command_statement_lookup=yes
status=pass
```

The first script attempt failed before the scenario was constructed because it
used obsolete Store initialization and workspace-head field assumptions. Those
failures were harness issues and did not change the tested semantics. The
successful run used current `workvcs init` and the real `genesis_commit_id`
field.

## Implementation

Changed implementation files:

```text
crates/workvcs-core/src/history/why.rs
crates/workvcs-core/src/history/mod.rs
crates/workvcs-core/src/lib.rs
crates/workvcs-cli/src/main.rs
```

The core result now includes `epistemic_explanations`. Explanations are created
only for `RecordSupports`, `RecordContradicts`, `RecordValidates`, and
`RecordInvalidates` relation edges. Statement projection uses `record_at` and
`knowledge_at` at the resolved `why` commit.

The CLI renders:

```text
epistemic_explanations=<N>
epistemic_explanation.<i>.relation_kind=<RELATION_KIND>
epistemic_explanation.<i>.direction=<incoming|outgoing>
epistemic_explanation.<i>.relation_id=<RELATION_ID>
epistemic_explanation.<i>.relation_version_id=<RELATION_VERSION_ID>
epistemic_explanation.<i>.<source|target>_entity_kind=<record|knowledge>
epistemic_explanation.<i>.<source|target>_entity_id=<ENTITY_ID>
epistemic_explanation.<i>.<source|target>_statement_json=<JSON_STRING>
epistemic_explanation.<i>.relation_state_digest=<DIGEST>
```

`--expected-epistemic-explanations COUNT` was added for CLI dogfood assertions.
After existing relation filters and `--relation-limit` are applied, the CLI
retains only explanations whose relation id and relation version id remain in
the rendered edge set.

## Focused Validation

Focused validation passed:

```text
cargo check -p workvcs-cli
cargo test -p workvcs-core --test why_epistemic_explanation_phase4nc
cargo test -p workvcs-cli cli_links_record_support_to_knowledge
cargo test -p workvcs-cli cli_why_epistemic_explanations_follow_relation_filters_and_limit
cargo test -p workvcs-cli cli_supersedes_knowledge
```

Coverage:

```text
core_record_supports_knowledge_statement_projection=pass
core_record_contradicts_knowledge_statement_projection=pass
core_record_validates_record_statement_projection=pass
core_record_invalidates_knowledge_statement_projection=pass
core_non_epistemic_knowledge_supersedes_absence=pass
cli_statement_rendering_and_expected_count=pass
cli_filter_and_relation_limit_explanation_sync=pass
cli_non_epistemic_knowledge_supersedes_absence=pass
```

## Post-Change Dogfood

The post-change dogfood run is recorded in:

```text
/tmp/workvcs-4nc-why-epistemic-post-20260901T190415Z/summary.txt
```

Summary:

```text
phase=4NC-post-change-dogfood
store=/tmp/workvcs-4nc-why-epistemic-post-20260901T190415Z/store.workvcs
record_id=01a05e5b-9ca9-7120-bde7-095ef5cd5051
knowledge_id=01a05e5b-9c94-76e2-93c8-442dd43eee21
commit_id=01a05e5b-9cbc-7013-a27e-9f84094631fe
why_relation_edges=1
why_epistemic_explanations=1
why_source_statement_json=present
why_target_statement_json=present
single_why_command_statement_lookup=pass
status=pass
```

This proves a single `workvcs why` command can now return the direct epistemic
edge and both connected statements.

## Readiness Impact

Phase 4NC closes the direct epistemic statement lookup subgap for current
Record-to-Record and Record-to-Knowledge epistemic edges. It gives
continuation Agents a directly readable explanation for why a current Record or
Knowledge item is supported, contradicted, validated, or invalidated.

The Context resolver, packets, and `why` explanations release gate remains
`Partial` and blocking. Full evolution traversal, broader causal traversal,
and broader context/Resource resolver maturity remain open and must be driven
by concrete future dogfood gaps.

## Validation

Full pre-merge validation passed on 2026-09-02:

```text
git diff --check
cargo fmt --all -- --check
scripts/validate-schema-v0.1.sh
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/smoke-v0.1-cli-workflow.sh
workctl plan validate
README Phase 4NC link checks
release-state literal checks
release-gate matrix status check
```

Independent read-only review found no blocker, high, or medium issues. Its one
low issue noted that `RecordContradicts` and `RecordInvalidates` were supported
by the same implementation path but lacked direct statement-projection test
assertions. That low issue is resolved by the current focused core test, which
now covers `RecordSupports`, `RecordContradicts`, `RecordValidates`,
`RecordInvalidates`, and non-epistemic `KnowledgeSupersedes` absence.

The formal WorkVCS `plan independent-review record` path was not used because
this lightweight Plan has no `independent_validation` object or trusted
attestation structure. The review is therefore treated as read-only subagent
evidence, not as a formal attested review record.

The validation and review did not authorize V1 release readiness, dogfood
completion, a release candidate, release, tag, Push, deployment, remote/cloud
Handoff, cross-Store synchronization, distributed collaboration, automatic
takeover, daemon, watcher, Agent orchestration, schema change, stored relation
semantic change, traversal expansion, or V2 feature.
