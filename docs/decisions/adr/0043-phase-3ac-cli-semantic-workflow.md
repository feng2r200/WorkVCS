# ADR-0043: Phase 3AC CLI Semantic Workflow

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed thin CLI over Engine boundary.

## Context

The CLI originally exposed only Store bootstrap and read-only history commands:
`init`, `doctor`, `history`, and `show-at`. The Engine now has enough semantic
operations to create a minimal Task/Acceptance Criterion/Verification workflow,
but those operations were not reachable from the command line.

## Decision

1. Phase 3AC adds a thin command shell over existing Engine APIs for:
   workspace creation, Task creation, Task transition, Acceptance Criterion
   creation, branch-aware Acceptance Criterion status, Verification Requirement
   creation, and work-state-only Verification recording.
2. The CLI remains an input/output layer. It parses typed UUIDs and closed
   vocabulary strings, then delegates to Engine option constructors and Engine
   facade methods.
3. CLI output remains concise line-oriented `key=value` text, matching the
   existing human-debuggable shell style.
4. The command spelling is a practical V0.1 shell and is not a final long-term
   protocol commitment.
5. Phase 3AC does not expose Resource Basis, Resource Observation, Evidence
   attachment, Resource applicability cache stamping, runtime sessions/claims,
   merge/restore/federation, or remote operations through the CLI.

## Consequences

- A local user or Agent can now run a minimal semantic workflow entirely through
  the CLI: initialize a Store, create a Workspace, create a Task and Acceptance
  Criterion, record a Verification, query branch-effective status, and complete
  the Task.
- The CLI still preserves the authoritative Engine boundary and does not
  duplicate semantic write logic.

## Implementation Findings

- No core API change was needed for this slice beyond consuming the existing
  Engine facade.
