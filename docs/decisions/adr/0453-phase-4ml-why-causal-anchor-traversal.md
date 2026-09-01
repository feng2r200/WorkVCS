# ADR-0453: Phase 4ML Why Causal Anchor Traversal

Status: Accepted
Date: 2026-09-01

## Context

ADR-0449 disclosed `evolution` as a deferred `why` relation family when the
queried Entity appears as a causal anchor on a first-parent-reachable
ChangeSet. Phase 4ML dogfood proved the next concrete blockage: `changeset
anchors` could show that a Finding Record anchored a Decision supersede
ChangeSet, but `why --entity <finding>` still could not report the anchoring
commit or ChangeSet. A continuation Agent had to leave `why`, scan history, and
join back through `changeset anchors` to understand which mutation the Finding
caused.

## Decision

`WhyQueryResult` now includes `causal_anchor_changesets`, a read-only projection
of first-parent-reachable ChangeSets where the queried Entity is recorded in
`changeset_causal_anchor`.

The CLI renders the projection as stable key-value fields:

```text
causal_anchor_changesets=<N>
causal_anchor_changeset.<i>.commit_id=<COMMIT_ID>
causal_anchor_changeset.<i>.changeset_id=<CHANGESET_ID>
causal_anchor_changeset.<i>.operation_type=<OPERATION_TYPE>
causal_anchor_changeset.<i>.operation_schema_version=<VERSION>
causal_anchor_changeset.<i>.committed_at_us=<UTC_MICROS>
causal_anchor_changeset.<i>.changeset_created_at_us=<UTC_MICROS>
causal_anchor_changeset.<i>.anchor_object_id=<ENTITY_ID>
causal_anchor_changeset.<i>.anchor_object_kind=entity
```

Existing `relation_edges`, Handoff `scope_links`, and
`deferred_relation_families` remain compatible. A non-empty causal-anchor list
continues to imply `deferred_relation_family=evolution`, because full evolution
traversal is still not implemented.

## Non-Goals

- No schema change.
- No new stored Relation rows.
- No generic causal anchor write API.
- No ChangeSet endpoint in relation edges.
- No full evolution traversal or epistemic traversal.
- No automatic transcript parsing, semantic inference, or LLM extraction.
- No new `why` filters or limits for causal-anchor ChangeSets.

## Evidence

- Pre-change gap dogfood:
  `/tmp/workvcs-4ml-why-gap-dogfood-20260901T092321Z`.
- Focused validation:
  `/tmp/workvcs-4ml-focused-validation-rerun-20260901T093023Z`.
- Post-change CLI dogfood:
  `/tmp/workvcs-4ml-why-causal-anchor-dogfood-20260901T093115Z`.
- Final validation and independent review are recorded in
  `docs/provenance/phase-4ml-why-causal-anchor-traversal.md`.

## Consequences

`why` can now identify the concrete commit and ChangeSet that a causal Record
anchored in the first-parent history. This makes the existing causal-anchor
provenance directly usable in continuation dogfood while preserving the V1
boundary around deeper evolution and epistemic traversal.
