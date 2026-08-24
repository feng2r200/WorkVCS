# WorkVCS Development Rules

## Current Stage

WorkVCS is in the confirmed-design specification stage. Do not implement
runtime code, choose a programming language, or freeze a database schema unless
the current user request explicitly opens that scope.

## Source of Truth

Use this order when sources disagree:

1. The current user's explicit instruction.
2. This `AGENTS.md`.
3. Accepted Architecture Decision Records under `docs/decisions/adr/`.
4. Confirmed domain rules under `docs/domain/`.
5. Confirmed architecture boundaries under `docs/architecture/`.
6. Confirmed product scope under `docs/product/`.

The 2026-08-24 baseline is the initial user-confirmed design promotion and
therefore predates repository ADRs. `docs/decisions/adr/` may not exist until
the first later material decision. From this baseline forward, a material
architecture change requires an accepted ADR before it becomes confirmed.

Chat transcripts, exploration notes, implementation sketches, historical Plans,
and generated summaries are evidence or proposals, not project truth by
themselves.

## Changing Confirmed Design

- Read the affected domain invariants and accepted ADRs before changing
  architecture or introducing a new entity, relation, lifecycle, or boundary.
- Keep product, domain, and architecture documents consistent in the same
  change. Do not let an implementation note silently redefine the domain.
- Record a later material architecture decision as an ADR before treating it
  as settled. Keep unresolved alternatives outside confirmed documents.
- Label deferred or unknown behavior explicitly; do not infer support from an
  example, future direction, or named concept.

## Delivery Boundary

- Preserve unrelated user changes and keep commits scoped to one delivery
  boundary.
- A local commit does not authorize Push, merge, release, deployment, or
  cleanup. Those actions require an explicit user request.
