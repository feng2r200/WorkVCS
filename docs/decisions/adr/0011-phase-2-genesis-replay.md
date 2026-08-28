# ADR-0011: Phase 2 Genesis Replay

- **Status:** Accepted for implementation
- **Accepted by:** current Work request to merge Genesis locally into `main`
  and start the Phase 2 Replay slice from the new `main`.

## Context

ADR-0010 added Workspace Genesis persistence. The frozen implementation
contract orders Replay after Genesis and before Entity transition, Commit/CAS,
and broader query work.

## Decision

1. Phase 2 Replay starts with a minimal `state_at` boundary that can reconstruct
   a Genesis WorkStateCommit as the empty WorkState.
2. Replay truth is WorkStateCommit plus ChangeSet plus ChangeOperations.
   Events and projections are not replay authority.
3. Genesis replay validates workspace linkage, `workspace.genesis`, zero Commit
   parents, zero ChangeOperations, and the resulting empty WorkState digest.
4. Normal and merge commits remain explicitly unsupported in this slice until
   the ChangeOperation apply vocabulary and merge replay are implemented.
5. Public entry remains the Engine facade. SQLite handles remain internal to
   `workvcs-core`.
6. This slice does not implement Entity transitions, Branch HEAD CAS, CLI
   business commands, Runtime, Verification, Merge, Federation, Projection,
   Checkpoint, Bundle, Doctor, or migration chains.

## Consequences

- `Engine::state_at` can prove Genesis replay and corruption detection before
  normal semantic mutations exist.
- Event or projection corruption must not redefine reconstructed WorkState.

## Implementation findings

- None so far.
