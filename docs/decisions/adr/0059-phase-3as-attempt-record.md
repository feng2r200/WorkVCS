# ADR-0059: Phase 3AS Attempt Record Creation

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed Record lifecycle.

## Context

Confirmed V1 semantic Record kinds include Finding, Assumption, Question,
Attempt, ordinary decision, Risk, and Handoff. The confirmed lifecycle says an
Attempt may be `running`, `succeeded`, `failed`, or `inconclusive`; terminal
Attempts are never reopened, and a later try creates a new Attempt.

The next bounded tool-development step is to let local Agents explicitly record
the start of an Attempt while preserving the existing Record state shape and
Engine facade.

## Decision

1. Phase 3AS adds `RecordKind::Attempt`.
2. Attempt creation writes `Record(kind=attempt)` with `status=running`.
3. Attempt state uses the existing Record state shape: kind, statement, scope,
   and status.
4. The CLI exposes `record attempt ...` and existing `record list` / `record
   show` support the `attempt` kind.
5. This slice does not implement terminal Attempt transitions, one-shot
   approach/result recording, Task execution runtime coupling, Handoff records,
   or typed causal Record relations.

## Consequences

- A local Agent can persist explicit running Attempts instead of leaving
  important tries only in transcript text.
- Terminal Attempt semantics remain explicit future work rather than being
  inferred through generic updates.

## Implementation Findings

- The existing Record state shape remains sufficient for a running Attempt.
