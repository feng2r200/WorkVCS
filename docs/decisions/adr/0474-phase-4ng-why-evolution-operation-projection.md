# ADR-0474: Phase 4NG Why Evolution Operation Projection

Status: Accepted
Date: 2026-09-02

## Context

ADR-0449 reports `evolution` as a deferred `why` relation family when a queried
Entity anchors a first-parent-reachable ChangeSet. ADR-0453 makes the concrete
anchoring commit and ChangeSet visible from `why`.

The next concrete continuation gap is one step narrower than full evolution
traversal: after `why --entity <causal-finding>` reports the anchoring
ChangeSet, an operator still has to leave `why` and run `changeset operations`
to see which direct Entity or Relation subjects the anchoring ChangeSet
changed. Phase 4NG keeps the existing causal-anchor boundary and projects only
those direct operation subjects.

## Decision

`WhyQueryResult` now includes `evolution_change_operations`, a deterministic
read-only projection of direct ChangeOperations for each already-rendered
`causal_anchor_changeset`.

Each projected item includes:

```text
commit_id
changeset_id
changeset_operation_type
changeset_operation_schema_version
operation_id
ordinal
subject
operation_payload_digest
operation_payload_size_bytes
```

The projection is built from existing ChangeSet operation query behavior. It
does not parse operation payloads, infer new semantics, write new Relation rows,
or widen the first-parent causal-anchor search.

The CLI renders stable key-value fields:

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

`--expected-evolution-change-operations COUNT` was added for dogfood scripts
and operator assertions. Existing relation filters and `--relation-limit`
continue to apply to `relation_edges`; they do not hide the causal-anchor
ChangeSet projection or its direct operation projection.

## Non-Goals

- No Store schema change.
- No new stored Relation rows.
- No new relation kind or canonical relation semantics.
- No full evolution traversal.
- No broader causal traversal.
- No ChangeSet endpoint in `relation_edges`.
- No payload parsing or semantic inference from ChangeOperation payloads.
- No transcript parsing, LLM extraction, ranking, or Agent orchestration.
- No release-candidate, release, Push, tag, deployment, remote, or V2 action.

## Evidence

- Pre-change gap proof:
  `.work-governance/runtime/logs/phase-4ng-pre-gap.4u2SjL`.
- T-002 implementation evidence:
  `50903fd71a798a6cca3ba87c9f5166c6b11569233d5dd6e5afc780987bda5cdd`.
- T-003 focused validation evidence:
  `1f137ea572d3eac0d8efb79870d79c9c408952fb895049017b9c1d6e28cb9153`.
- Post-change CLI dogfood:
  `.work-governance/runtime/logs/phase-4ng-post-dogfood.nro8Ek/summary.txt`.

Focused validation and dogfood details are recorded in
`docs/provenance/phase-4ng-why-evolution-operation-projection.md`.

## Consequences

`workvcs why` can now answer a practical continuation question in one command:
when a causal Finding anchors a Decision supersede ChangeSet, the same `why`
output shows the anchoring ChangeSet and the direct Entity/Relation operation
subjects changed by that ChangeSet.

The Context resolver, packets, and `why` explanations release gate remains
`Partial` and blocking. Full evolution traversal, broader causal traversal,
broader context/Resource resolver maturity, release-candidate validation, and
release authorization remain open.
