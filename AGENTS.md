# WorkVCS Development Rules

## Current Stage

WorkVCS has moved from design-only specification into an in-progress local
Rust V0.1 implementation. Do not implement new runtime behavior, command
behavior, schema behavior, release behavior, or V2 scope unless the current
user request and active governed Plan explicitly open that scope.

Use `docs/provenance/v1-readiness-ledger.md` for current implementation
readiness and dogfood gaps. Product, architecture, schema, and accepted ADRs
remain the authority for confirmed WorkVCS semantics. Smoke coverage does not
by itself imply release readiness or dogfood completion.

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

A statement is eligible for confirmed documentation only when the discussion
of that specific issue has reached a final, consistent conclusion and the user
has explicitly confirmed, selected, approved, or stated that final conclusion.
An interim user confirmation is necessary evidence, but it is not sufficient
when the broader discussion remains open, later clarification changes its
meaning, or no final conclusion is reached. Later explicit corrections or
superseding decisions take precedence.

Assistant recommendations, examples, consolidations, and surrounding
implementation details do not become confirmed merely because the user
accepted a nearby numbered choice. When the available context cannot establish
that a conclusion is both final and still current, classify it as Open or
Unclear rather than promoting it to Confirmed.

## Changing Confirmed Design

- Read the affected domain invariants and accepted ADRs before changing
  architecture or introducing a new entity, relation, lifecycle, or boundary.
- Keep product, domain, and architecture documents consistent in the same
  change. Do not let an implementation note silently redefine the domain.
- Record a later material architecture decision as an ADR before treating it
  as settled. Keep unresolved alternatives outside confirmed documents.
- Label deferred or unknown behavior explicitly; do not infer support from an
  example, future direction, or named concept.
- Trace every promoted statement to the narrowest user confirmation. If the
  evidence confirms a requirement but not its implementation mechanism,
  preserve the requirement and leave the mechanism outside confirmed docs.
- Any change to WorkVCS configuration discovery, environment variables, or
  locator precedence must update `config.toml.example`, operator documentation,
  and focused configuration tests in the same change.

## Delivery Boundary

- Preserve unrelated user changes and keep commits scoped to one delivery
  boundary.
- A local commit does not authorize Push, merge, release, deployment, or
  cleanup. Those actions require an explicit user request.
