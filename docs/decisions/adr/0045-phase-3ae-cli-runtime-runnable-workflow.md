# ADR-0045: Phase 3AE CLI Runtime Runnable Workflow

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed runtime coordination / runnable projection boundary.

## Context

Core already supports Session runtime, Claim coordination, and deterministic
Runnable Task projection. The CLI did not expose those capabilities, leaving
the Agent-facing workflow unable to start a runtime session, inspect runnable
work, or coordinate claims from the command line.

## Decision

1. Phase 3AE adds thin CLI commands for Session start/end, Task claim/release,
   and Runnable Task projection.
2. The commands delegate to existing Engine runtime APIs:
   `start_session`, `end_session`, `claim_task`, `release_claim`, and
   `runnable_tasks`.
3. Runtime JSON inputs are canonical object values parsed by the core
   canonical JSON boundary.
4. Runnable output is line-oriented `key=value` text with candidate IDs,
   lifecycle readiness, dependency readiness, claim coordination, and blocked
   reasons.
5. This slice does not implement automatic `next` selection, focus-path CLI UX,
   claim takeover, session switching, adapter execution, merge coordination,
   remote operations, or a finalized command protocol.

## Consequences

- A local Agent can now create semantic work, start a Session, inspect runnable
  Tasks, claim/release a Task, and end the Session through the CLI.
- Runtime operations remain non-WorkState coordination state and continue to use
  the existing Core validation and provenance paths.

## Implementation Findings

- No Core API or schema change was required. This was a CLI exposure slice over
  already implemented runtime semantics.
