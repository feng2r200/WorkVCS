# ADR-0070: Phase 3BD Record Related-To Relation

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  confirmed generic relation escape hatch.

## Context

The confirmed relation vocabulary reserves `related_to` for custom meaning. It
requires a label, preserves the relationship, and must not affect readiness,
merge, context ranking, or other deterministic algorithms until that meaning
is promoted to a confirmed canonical relation.

## Decision

1. Phase 3BD adds `RecordRelationType::RelatedTo`.
2. `RecordRelationCreateOptions::related_to` writes a `related_to` relation
   between two distinct Record entities.
3. The custom label is stored as `relation_discriminator`, matching the
   confirmed logical key rule.
4. Public Record relation create/list/show results expose the discriminator as
   `relation_label`.
5. Relation labels must be non-empty, have no leading/trailing whitespace, and
   contain no control characters.
6. `record relation-list` accepts `--type related_to` and optional `--label`.
7. The CLI exposes `record link-related-to ... --label ...`.
8. Core `why` exposes the edge as `WhyRelationKind::RecordRelatedTo` and
   carries the label; the CLI renders it as `record_related_to` with
   `relation_label`.
9. This slice does not promote custom labels into deterministic core
   algorithms, does not add relation updates/removal, and does not change the
   schema.

## Consequences

- Local Agents can preserve custom Record-to-Record relationships without
  inventing new canonical relation types.
- Multiple `related_to` edges between the same Records may coexist when their
  labels differ, because the label is the immutable discriminator.

## Implementation Findings

- The existing `relation_discriminator` column matched the confirmed
  `related_to + label` model; no schema change was required.
