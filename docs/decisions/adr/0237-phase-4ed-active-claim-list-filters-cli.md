# ADR-0237: Phase 4ED Active Claim List Filters CLI

Status: Accepted

Date: 2026-08-30

## Context

Claim list is a runtime query for active claims in one Session. Its current CLI
surface can show all active claims for the Session, but cannot narrow the
result by claimed task or claim mode even though both fields are already part
of each Claim snapshot.

The query intentionally remains scoped to active claims. Released claims are
available through `claim show` by ID, but this slice does not turn claim list
into a historical claim journal.

## Decision

1. Add `--task TASK_ENTITY_ID` to `workvcs claim list`.
2. Add `--mode MODE` to `workvcs claim list`.
3. Keep `--session SESSION_ID` as the required list scope.
4. Load active Claim snapshots through the existing Engine facade and apply all
   supplied filters before rendering.
5. Reuse the existing claim mode vocabulary: `exclusive` and `shared`.

## Non-Goals

- This slice does not add historical released-claim listing.
- This slice does not change claim acquisition, release, guard, or next-task
  selection semantics.
- This slice does not add lifecycle-state filtering because the list scope is
  explicitly active claims.
- This slice does not change SQLite schema or runtime claim APIs.

## Consequences

- CLI users can inspect active claim ownership for one task or one mode without
  post-processing full Session claim lists.
- Claim guard and coordination workflows get a narrower inspection command.
- The command remains a thin read-only wrapper over existing runtime snapshots.
