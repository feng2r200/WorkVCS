# ADR-0014: Phase 2 Concurrency and Integrity Closure

- **Status:** Accepted for implementation
- **Accepted by:** current Work request to start the subsequent Phase 2 slice
  from the new `main` after Query / Show-at / History.

## Context

SE-12 requires the first vertical slice to prove canonical determinism,
restart persistence, projection independence, two-connection CAS behavior, and
structured errors. SE-15 places later concurrency and integrity tests after the
Query / Show-at / History slice. ADR-0013 deliberately deferred that work.

The existing mutation kernel already routes semantic writes through Engine APIs,
uses optimistic Branch HEAD comparison for same-branch concurrency, and replays
authoritative history rather than projections. This slice closes the remaining
Phase 2 first-vertical-slice validation gap without introducing upper-domain
runtime behavior.

## Decision

1. This slice closes the Phase 2 first vertical slice with optimistic
   same-branch CAS race tests and authoritative history integrity validation.
2. Core exposes a narrow `Engine::validate_integrity` API returning an
   `IntegrityReport`. SQLite handles and transactions remain internal Store
   implementation details.
3. `validate_integrity` checks SQLite integrity, foreign-key consistency,
   Branch HEAD replay, and every stored WorkStateCommit replay/digest. Events,
   current-state cache tables, and projections remain non-authoritative.
4. CLI `doctor` calls `Engine::validate_integrity` but remains a thin Store
   smoke-check command. It does not gain business Entity CRUD or direct SQL
   behavior.
5. Mutation tests prove that a two-connection stale expected-HEAD race rejects
   the loser with a structured retryable conflict and leaves authoritative
   history rows unchanged.
6. Mutation tests also prove invalid transition input and corrupted expected
   head replay failures leave no partial authoritative history rows.
7. This slice does not change schema v0.1.
8. Merge, Task, Runtime, Verification, Federation, Resource adapters,
   Checkpoint, Bundle, migrations, performance indexes, public Entity CRUD, and
   generic SemanticOperation execution remain deferred.

## Consequences

- Phase 2 now has an Engine-owned integrity check that can be reused by tests
  and by the thin CLI doctor command.
- First vertical slice completion evidence no longer relies on ad hoc SQL
  probes alone.
- Derived projections and event rows continue to be treated as non-authority
  for replay and integrity validation.
- Later slices can add richer repair, migration, or domain-runtime behavior
  without changing this slice's minimal Store boundary.

## Implementation findings

- None so far.
