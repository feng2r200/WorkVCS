# ADR-0071: Phase 3BE Decision Record Lifecycle

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  confirmed Decision lifecycle contract.

## Context

The confirmed semantic state machine defines Decision states as `active`,
`superseded`, and `withdrawn`. ADR-0055 introduced ordinary
`Record(kind=decision)` entries as active Records and explicitly deferred
Decision supersession. Later Record relation slices added support and
contradiction edges against active Decision Records.

The next minimal tool capability is to let an ordinary Decision Record leave the
active set without opening promoted Decision entities or full supersession
resolution.

## Decision

1. Phase 3BE extends `RecordStatus` with `superseded` and `withdrawn`.
2. Ordinary Decision Records may transition only:

   ```text
   active -> superseded
   active -> withdrawn
   ```

3. Terminal Decision Records cannot transition again. An old choice returning
   still requires a new Decision Record in a later operation.
4. Decision transitions reuse the existing Record entity transition kernel and
   require non-empty rationale text/object.
5. The CLI exposes `record decision-status ... --status superseded|withdrawn`.
6. This slice does not implement promoted Decision entities, `supersedes`
   relations, automatic replacement, decision conflict review, relation updates,
   or relation removal.

## Consequences

- A local Agent can retire an ordinary Decision Record as versioned WorkState.
- Existing `supports` and `contradicts` relation creation continues to require
  an active Decision target.
- Explicit supersession links remain a separate small slice.

## Implementation Findings

- The existing Record transition machinery was sufficient after dispatching by
  `RecordKind::Decision`; no schema or replay change was required.
