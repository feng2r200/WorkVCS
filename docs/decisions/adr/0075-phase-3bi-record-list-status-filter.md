# ADR-0075: Phase 3BI Record List Status Filter

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  implemented Record lifecycle slices.

## Context

Record lifecycle slices now create multiple current statuses across
Assumptions, Attempts, ordinary Decisions, and other Record kinds. The existing
Record list projection supports a kind filter only, which makes it awkward for
local tools to inspect active or terminal Records after supersession,
withdrawal, validation, or attempt completion.

## Decision

1. `RecordListOptions` gains an optional `RecordStatus` filter.
2. `records_at` applies both optional filters: kind and status.
3. The CLI exposes `record list --status <status>`.
4. The status vocabulary is the current semantic Record status vocabulary:
   `active`, `failed`, `inconclusive`, `invalidated`, `running`, `succeeded`,
   `superseded`, `unverified`, `validated`, and `withdrawn`.
5. This slice does not change the default list behavior; unfiltered list still
   returns all current Record entities at the selected commit.

## Consequences

- A local Agent can query active, superseded, withdrawn, validated, invalidated,
  or terminal attempt Records without client-side filtering.
- Future context ranking/default exclusion behavior can build on the same
  projection without changing this API.

## Implementation Findings

- The existing Record list projection was already state-aware; only options,
  filtering, and CLI parsing were required.
