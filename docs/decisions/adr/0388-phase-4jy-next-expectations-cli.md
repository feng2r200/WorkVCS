# ADR-0388: Phase 4JY Next Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs next` is the combined operator command that selects work through the
same claim-next path and returns the updated Session context. Phase 4JW made the
lower-level `claim next` command script-verifiable. The combined command needs
the same selection assertions so operators can use one command as an action and
acceptance gate.

## Decision

The CLI adds optional post-selection expectations to `workvcs next`:

- `--expected-selected true|false`
- `--expected-head COMMIT_ID`
- `--expected-inspected-candidates COUNT`
- `--expected-task TASK_ENTITY_ID`
- `--expected-mode exclusive|shared`
- `--expected-lifecycle-state STATE`

The command still delegates to `Engine::next_work`. It reuses the same internal
claim-next expectation checks as `workvcs claim next`. Passing checks append
field-specific `*_match_expected=true` markers.

## Consequences

The main next-work operator entrypoint can now be used directly in local
automation and smoke workflows. Expectations are evaluated after the next-work
selection result is observed; mismatch handling does not alter Claim creation,
Session context, runnable ordering, dependency readiness, or storage schema.
