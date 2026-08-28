# ADR-0010: Phase 2 Workspace Genesis Bootstrap

- **Status:** Accepted for implementation
- **Accepted by:** current Work request to merge Phase 2 locally into `main`
  and start the next Phase 2 slice from the new `main`.

## Context

ADR-0007 confirmed that Workspace creation uses an atomic Genesis
initialization transaction. ADR-0009 delivered only Store bootstrap/open and
explicitly left Workspace Genesis out of that first Phase 2 slice.

## Decision

1. Phase 2 slice 2 implements only Workspace Genesis initialization on top of
   the existing Store/Engine boundary.
2. Public entry remains the Engine facade. The write path creates a Workspace,
   one initialization ChangeSet, one zero-parent Genesis WorkStateCommit, at
   least one initial Branch whose HEAD is Genesis, and one provenance Event in
   one SQLite transaction.
3. The Genesis ChangeSet uses operation type `workspace.genesis`, zero
   ChangeOperations, and the digest of the empty WorkState mapping.
4. The implementation uses the schema's deferred Workspace-to-Genesis Commit
   foreign key cycle and keeps per-connection foreign-key enforcement enabled.
5. The default initial Branch name is `main` as an API default, not a schema
   contract. Callers may pass a different valid Branch name.
6. This slice does not implement replay, normal entity/relation transitions,
   Branch HEAD compare-and-swap beyond the initial insert, Task, Runtime,
   Verification, Merge, Federation, Projection, Checkpoint, Bundle, Doctor, or
   migration chains.

## Consequences

- Genesis persistence is restart-visible through `Engine::open` and
  `Engine::workspace_info`.
- Store access still does not expose `rusqlite` handles to public callers.
- The CLI remains a compiling skeleton with no business commands.

## Implementation findings

- None so far.
