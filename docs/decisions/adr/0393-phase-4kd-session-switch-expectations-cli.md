# ADR-0393: Phase 4KD Session Switch Expectations CLI

Status: Accepted
Date: 2026-08-31

## Context

`workvcs session switch` moves an active Session to another Workspace and
Branch, can set focus, and reports the previous context, active context,
released claim count, focus, and lifecycle state. Automation needs to verify
that the switch produced the intended runtime state without issuing a
separate `session show` command.

## Decision

The CLI adds optional post-switch expectations to `workvcs session switch`:

- `--expected-session SESSION_ID`
- `--expected-previous-workspace WORKSPACE_ID`
- `--expected-previous-branch BRANCH_ID`
- `--expected-workspace WORKSPACE_ID`
- `--expected-branch BRANCH_ID`
- `--expected-released-claims COUNT`
- `--expected-focus ENTITY_ID_OR_NONE`
- `--expected-lifecycle-state STATE`

The command still delegates to `Engine::switch_session`. Passing checks append
`session_match_expected=true`, `previous_workspace_match_expected=true`,
`previous_branch_match_expected=true`, `workspace_match_expected=true`,
`branch_match_expected=true`, `released_claims_match_expected=true`,
`focus_match_expected=true`, and `lifecycle_state_match_expected=true`.
Mismatches use the existing Session validation error surface.

## Consequences

Session switching can now be used directly as a script-verifiable runtime
transition. Expectations are evaluated after the Session is switched; mismatch
handling does not change Session lifecycle semantics, context membership,
claim release behavior, focus behavior, runtime Event recording, or storage
schema.
