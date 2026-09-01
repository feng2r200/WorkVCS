# ADR-0470: Phase 4NC Why Epistemic Explanation

Status: Accepted
Date: 2026-09-02

## Context

The V1 release gate matrix still marks the Context resolver, packets, and
`why` explanations gate as `Partial`. The current concrete dogfood gap is
epistemic readability: `workvcs why` can expose direct `record_supports`,
`record_contradicts`, `record_validates`, and `record_invalidates` relation
edges, but a continuation Agent must run separate `record show` or
`knowledge show` commands to read the statements connected by those edges.

ADR-0177 removed unconditional coarse `Epistemic` deferred-family reporting and
kept relation edges as the current coverage signal. ADR-0453 added a read-only
causal-anchor projection without changing stored relations or adding traversal.
Phase 4NC follows the same boundary: make already-rendered direct epistemic
edges more actionable without broadening graph semantics.

## Decision

`WhyQueryResult` now includes `epistemic_explanations`, a deterministic
read-only projection for direct epistemic relation edges.

The projection is emitted only for these existing `WhyRelationKind` values:

- `RecordSupports`
- `RecordContradicts`
- `RecordValidates`
- `RecordInvalidates`

Each explanation mirrors one relation edge and includes:

```text
relation_kind
direction
relation_id
relation_version_id
source
target
source_statement
target_statement
state_digest
```

Statements are loaded at the resolved `why` commit through the existing
Record and Knowledge snapshot readers. Record-to-Record and Record-to-Knowledge
epistemic edges are covered. Other endpoint kinds are ignored.

The CLI renders stable key-value fields:

```text
epistemic_explanations=<N>
epistemic_explanation.<i>.relation_kind=<RELATION_KIND>
epistemic_explanation.<i>.direction=<incoming|outgoing>
epistemic_explanation.<i>.relation_id=<RELATION_ID>
epistemic_explanation.<i>.relation_version_id=<RELATION_VERSION_ID>
epistemic_explanation.<i>.<source|target>_kind=entity
epistemic_explanation.<i>.<source|target>_entity_kind=<record|knowledge>
epistemic_explanation.<i>.<source|target>_entity_id=<ENTITY_ID>
epistemic_explanation.<i>.<source|target>_statement_json=<JSON_STRING>
epistemic_explanation.<i>.relation_state_digest=<DIGEST>
```

Existing relation filters and `--relation-limit` continue to apply to
`relation_edges`; CLI rendering retains only explanations whose relation
identity remains in the rendered edge set. The CLI also adds
`--expected-epistemic-explanations COUNT` for dogfood and script assertions.

## Non-Goals

- No Store schema change.
- No new stored Relation rows.
- No new relation kinds or canonical relation semantics.
- No transitive causal, evolution, or epistemic traversal.
- No broad `Epistemic` deferred-family marker.
- No LLM extraction, transcript parsing, ranking, or semantic inference.
- No release-candidate, release, Push, tag, deployment, remote, or V2 action.

## Evidence

- Pre-change gap dogfood:
  `/tmp/workvcs-4nc-why-epistemic-gap-20260901T185036Z/summary.txt`.
- Focused core validation:
  `cargo test -p workvcs-core --test why_epistemic_explanation_phase4nc`.
- Focused CLI validation:
  `cargo test -p workvcs-cli cli_links_record_support_to_knowledge`.
- Focused CLI filter/limit validation:
  `cargo test -p workvcs-cli cli_why_epistemic_explanations_follow_relation_filters_and_limit`.
- Focused non-epistemic CLI validation:
  `cargo test -p workvcs-cli cli_supersedes_knowledge`.
- Post-change CLI dogfood:
  `/tmp/workvcs-4nc-why-epistemic-post-20260901T190415Z/summary.txt`.

Final validation and independent review are recorded in
`docs/provenance/phase-4nc-why-epistemic-explanation.md`.

## Consequences

`workvcs why` can now make direct epistemic edges self-explanatory for
continuation Agents: the same command output carries both the relation identity
and the Record or Knowledge statements being supported, contradicted,
validated, or invalidated.

The broader Context resolver, packets, and `why` explanations release gate
remains `Partial` because full evolution traversal, broader causal traversal,
and broader context/Resource resolver maturity remain open.
