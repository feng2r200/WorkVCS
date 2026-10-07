# ADR-0517: Shared Project Binding Isolation

Status: Accepted and implemented; live adoption evidence pending
Date: 2026-10-07

## Context

Registry-v1 migration intentionally preserves each binding one-to-one. When
two historical identities already name the same Store, Workspace, and Branch,
registry v2 retains both ProjectRefs and emits a non-authoritative
`possible_shared_target` observation. This is lossless, but it does not prove
that the two logical projects intentionally share one unpartitioned Work State.

The control plane previously had no supported way to separate one such
ProjectRef after migration. `project bind` is a registry-v1 compatibility
operation and correctly refuses registry v2. Hand-editing registry JSON would
bypass revision, digest, locking, activation-marker, backup, and post-install
verification contracts. Copying the old Store would also copy state whose
logical ownership cannot be proven after an accidental shared binding.

## Decision

WorkVCS provides one explicit operator operation:

```text
workvcs project isolate-shared-binding --preview --cwd PATH [--project-ref ID] [--registry PATH] [--store-root PATH]
workvcs project isolate-shared-binding --apply --cwd PATH [--project-ref ID] [--registry PATH] [--store-root PATH] --expected-registry-digest DIGEST --expected-candidate-digest DIGEST
```

Preview is read-only and bypasses ordinary activation only for this bounded
repair inspection. It resolves one exact ProjectRef from explicit identity,
Git, or CWD evidence, verifies its current Store target, and is eligible only
when another ProjectRef has the identical Store/Workspace/Branch tuple. Its
candidate digest binds the exact registry snapshot, selected ProjectRef, old
target tuple, and deterministic dedicated Store path. An absent, empty
recoverable, or pristine deterministic Store is acceptable; a foreign,
aliased, bound, or non-pristine candidate fails closed.

Apply is Unix-only and requires both preview digests. It acquires the registry
lock and then the registry-derived journal-quiescence lock, recomputes the
candidate, preserves the exact old registry bytes in a digest-named backup,
and creates or reuses only the deterministic pristine Store and Genesis
Workspace. The core registry transition changes exactly one binding, records
`binding_source=isolation`, and advances the registry revision once. It then
atomically replaces and rereads the registry and revalidates every binding and
the backup.

The old Store is never mutated, deleted, copied, or replayed. Existing
`possible_shared_target` observations remain immutable historical evidence.
The ProjectRef remains stable; only its routing target changes. The operation
does not migrate Work State or journal history and does not infer which old
Store objects belong to which project.

Both activation markers become stale when the registry revision changes.
Isolation never refreshes them implicitly. The operator must inspect and
explicitly refresh read routing and journal admission against the new exact
registry snapshot before ordinary reads or writes resume.

If failure occurs after the authoritative registry replacement, WorkVCS
returns `shared_binding_isolation_install_indeterminate`. Recovery begins with
another read-only isolation preview. `already_isolated` is a verified terminal
inspection state, not permission to replay apply.

## Consequences

- An accidentally shared unpartitioned target can be separated without
  rewriting the registry by hand or cloning ambiguous history.
- The original ProjectRef and old Store remain auditable.
- The new dedicated Store starts at Genesis; current project state must be
  reconstructed only from independently verified authority and admitted
  through normal durable-operation routes.
- A shared Store with distinct Workspaces or Branches is not an exact shared
  target and is outside this operation.
- Isolation is not consolidation, rollback, release, deployment, or
  authorization for later Store writes.

## Verification

Core tests prove that only the selected exact shared binding moves, the peer
is preserved, the source changes to `isolation`, revision advances once, and a
dedicated binding cannot be isolated again. CLI fixtures prove zero-write
preview, pre-write digest rejection, exact backup, byte-stable source Store,
dedicated pristine Store bootstrap, atomic registry replacement,
`already_isolated` inspection, stale activation markers, explicit marker
refresh, and independent discovery of the isolated ProjectRef and preserved
peer. Fault-injection tests stop after registry rename, directory sync, and
immediately before installed-state verification; each returns the dedicated
indeterminate error and converges through read-only preview without mutating
the source Store or leaving a registry temp artifact.
