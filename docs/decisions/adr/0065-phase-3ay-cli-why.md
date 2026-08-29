# ADR-0065: Phase 3AY CLI Why

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  existing core `why` projection.

## Context

The core Engine already exposes a `why` projection for relation neighborhoods,
including the Phase 3AX Record `invalidates` edge. The CLI still lacked a direct
tool surface for this query, which forced users or scripts to call library APIs
or inspect lower-level state.

## Decision

1. Phase 3AY adds a top-level `why` CLI command.
2. The command accepts exactly one target: `--branch` or `--commit`.
3. The command accepts exactly one subject: `--entity` or `--evidence`.
4. The command calls `Engine::why` and renders the returned target, subject,
   relation edges, endpoints, state digests, and deferred relation families as
   stable key-value lines.
5. This slice does not add new why semantics, new relation families, or storage
   reads outside the Engine facade.

## Consequences

- Local scripts can query WorkVCS semantic neighborhoods through the CLI.
- Record `invalidates` edges are now visible through both `record relation-list`
  and `why`.

## Implementation Findings

- The existing `WhyQueryOptions` target/subject model mapped directly to a thin
  CLI command; no core API changes were required.
