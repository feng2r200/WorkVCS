# ADR-0013: Phase 2 Query, Show-at, and History

- **Status:** Accepted for implementation
- **Accepted by:** current Work request to start the next Phase 2 slice from
  the new `main` after Entity transition / Commit / CAS and clear worktrees and
  local branches already covered by `main`.

## Context

ADR-0012 introduced the first normal semantic mutation kernel and replay for
linear normal commits. The frozen implementation contract orders query
broadening after Entity transition / Commit / CAS and before later
concurrency/integrity work. The first vertical slice also allows the CLI to
expose only thin shell commands over Engine APIs.

## Decision

1. This slice introduces a narrow Engine query surface for Branch HEAD,
   first-parent history, and show-at by Commit.
2. Branch HEAD query reads the Branch record and its head WorkStateCommit.
   Events, current-state cache tables, and projection tables are not
   authoritative query truth.
3. History query starts from either a Branch HEAD or a Commit and walks the
   newest-first first-parent chain through WorkStateCommit, CommitParent, and
   ChangeSet metadata.
4. This slice supports Genesis commits and linear normal commits. Merge history
   semantics, non-primary parent traversal, and graph-shaped history queries
   remain deferred.
5. `show_at` returns the replayed WorkState at a Commit. It does not mutate
   Branch HEAD, rebuild projections, or read projections as replay truth.
6. The CLI remains a thin shell over Engine APIs with only `init`, `doctor`,
   `history`, and `show-at`. It does not implement business Entity CRUD,
   Task, Runtime, Verification, Merge, Federation, Checkpoint, Bundle, remote
   behavior, or direct SQL access.
7. `why`, `context`, `diff`, `next`, projection materialization/rebuild,
   doctor migrations, and upper-domain runtime behavior remain deferred.

## Consequences

- A Workspace can now expose its current Branch HEAD, inspect the linear Commit
  history behind that head, and replay historical WorkState by Commit.
- Query behavior stays aligned with immutable history rather than derived
  runtime projections.
- The CLI can smoke test Store and query APIs without becoming an
  authoritative business UX.

## Implementation findings

- None so far.
