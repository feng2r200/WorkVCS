# ADR-0072: Phase 3BF Decision Record Supersedes

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  confirmed canonical relationship vocabulary.

## Context

The confirmed relationship model defines `supersedes` as an Evolution relation
with canonical direction `replacement -> prior`. It also requires semantic
operations such as supersede to create or update their required canonical edges
automatically, and to commit all entity and relation changes in one atomic
ChangeSet.

ADR-0071 added the ordinary Decision Record lifecycle states `superseded` and
`withdrawn`, but did not create supersession links.

## Decision

1. Phase 3BF adds `RecordRelationType::Supersedes`.
2. The public Engine API exposes `supersede_decision_record`.
3. The operation requires:
   - an active replacement `Record(kind=decision)`;
   - an active prior `Record(kind=decision)`;
   - the expected current version of the prior Decision Record;
   - non-empty rationale.
4. A successful operation writes one normal WorkState commit with one ChangeSet:
   - an entity operation changing the prior Decision Record to `superseded`;
   - a relation operation creating `replacement supersedes prior`.
5. `record relation-list --type supersedes` and `why` expose the resulting edge.
6. This slice does not implement promoted Decision entities, automatic conflict
   review, `derived_from` causal anchors, relation updates, relation removal, or
   reactivation of old Decisions.

## Consequences

- A local Agent can now replace an ordinary Decision Record without rewriting
  the prior choice.
- Supersession is visible in `history`, `show`, `relation-list`, and `why`.
- Later promoted Decision work can reuse the same canonical direction.

## Implementation Findings

- Existing replay already supports multiple `ChangeOperation` rows in one
  ChangeSet, so the atomic supersede operation only needed a new operation type
  whitelist entry.
- Relation creation-time status checks must not be reused as projection-time
  checks. A relation created against an active Decision must remain queryable
  after that Decision is later `superseded` or `withdrawn`. Phase 3BF therefore
  separates create validation from projection validation.
