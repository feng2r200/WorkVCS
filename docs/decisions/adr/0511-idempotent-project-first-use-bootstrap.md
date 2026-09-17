# ADR-0511: Idempotent Project First-Use Bootstrap

Status: Accepted
Date: 2026-09-17

## Context

Project discovery intentionally performs no writes. Before this decision, an
unbound logical project therefore required a caller to initialize a Store,
create a Workspace, recover their identifiers, and bind them into the external
registry before any Plan or standalone cognition could be recorded. In normal
Codex work this surfaced as “the current project has no WorkVCS binding, so no
work record will be created.” The underlying work could continue, but durable
planning and useful cognition were silently skipped at exactly the point where
the provider was expected to reduce friction.

Making discovery mutate would destroy its audit contract. Treating every
ambient working directory as a project would also bind temporary mirrors or
coordination directories instead of the logical project that owns the work.
The write boundary therefore needs a separate, explicit, idempotent first-use
operation.

## Decision

1. Add `workvcs project ensure --cwd PATH`. It resolves the same logical
   identity as project discovery. Git repositories use the canonical Git
   common directory; non-Git projects use the canonical directory.
2. An already bound project is verified and returned unchanged. No Store,
   Workspace, registry entry, Plan, Record, Session, or Claim is created.
3. An unbound project receives one dedicated Store, one Workspace, and its
   initial Work Branch, followed by one external registry binding. This
   bootstrap creates no semantic work objects.
4. The default Store root is `<configured-home>/stores/projects`. When the
   selected locator names only a registry and therefore supplies no WorkVCS
   home, `--store-root PATH` is required. Registry and Store locations remain
   outside all project boundaries.
5. Store filenames combine a readable project slug with a digest of the full
   identity kind and value. Equal basenames cannot alias distinct projects,
   while linked Git worktrees resolve to the same Store.
6. The registry lock covers rechecking the binding, creating or recovering the
   deterministic Store and Workspace, and atomically replacing the registry.
   Concurrent first-use calls therefore converge on one binding.
7. An interrupted bootstrap is recoverable only when the deterministic Store
   carries the exact identity marker and is either Store-only or has exactly
   one pristine Workspace at its Genesis. Any foreign, ambiguous, or
   non-pristine Store fails closed and is never overwritten or silently
   adopted.
8. `project discover` remains zero-write. A missing binding returns the stable
   `project_binding_not_found` code with `recoverable=true`,
   `recovery_action=project_ensure`, and the resolved project and registry
   paths. Callers route on these fields rather than parsing message text.
9. One Store per logical project is the default topology. Intentional
   cross-repository coordination still uses explicit `project bind` to share a
   chosen Store/Workspace/Branch; bootstrap does not infer that policy.

## Consequences

- A caller can recover an unbound project with one idempotent command and then
  admit a Plan or capture standalone cognition without abandoning the current
  work.
- Read-only audits retain their zero-write guarantee.
- Deterministic placement plus strict marker/pristine checks makes process
  interruption recoverable without turning an arbitrary SQLite file into
  project authority.
- The provider remains mechanical: governance still decides the logical
  project, whether a Plan is warranted, and what cognition deserves recording.

## Non-goals

- Mutating `project discover`, `recall`, or `resume`.
- Automatically selecting an ambient temporary directory as the logical
  project.
- Creating a Plan, Goal, Task, Record, Evidence item, Session, or Claim during
  bootstrap.
- Inferring multi-repository sharing, organization policy, authorization, or
  retention policy.
- Overwriting, deleting, or adopting an existing Store whose bootstrap marker
  or pristine state does not match.
