# ADR-0074: Phase 3BH Decision Supersede Causal Anchor

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  confirmed relationship model for causal traversal.

## Context

The confirmed relationship examples map:

```text
supersede T-18 with T-21 because F-17

=> T-21 supersedes T-18
=> T-21 derived_from F-17
```

ADR-0072 implemented atomic ordinary Decision Record supersession. ADR-0073
added explicit Record-to-Record `derived_from` relations. The next minimal tool
step is to let the supersede operation attach a causal Record anchor without
opening generic Entity relation APIs.

## Decision

1. `DecisionRecordSupersedeOptions` accepts an optional causal Record through
   `with_causal_record`.
2. `record supersede-decision` accepts optional `--because-record`.
3. When provided, the same atomic ChangeSet writes:
   - the prior Decision Record status update to `superseded`;
   - the `replacement supersedes prior` relation;
   - the `replacement derived_from because_record` relation.
4. The causal anchor must be a Record in the same Workspace and cannot be the
   replacement Record itself.
5. This slice does not make causal anchors mandatory, does not implement
   generic non-Record `derived_from`, and does not infer causal anchors from
   rationale text.

## Consequences

- A local Agent can preserve both the replacement edge and the cause behind the
  replacement in one versioned operation.
- `why` can traverse both supersession and source provenance for the replacement
  Decision Record.

## Implementation Findings

- Replay required no further change because the Phase 3BF operation type already
  allows multi-operation ChangeSets.
