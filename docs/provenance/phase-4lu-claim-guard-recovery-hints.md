# Phase 4LU Claim Guard Recovery Hints Evidence

Status: current local evidence
Date: 2026-09-01

## Scope

Phase 4LU reduces the blocked-handoff recovery key-value capture friction
recorded in Phase 4LG. It adds stable top-level recovery hint fields to
read-only `claim guard` output.

This evidence does not claim automatic stale detection, automatic takeover,
Claim runtime redesign, another-project dogfood, or release readiness.

## Behavior

`workvcs claim guard` now renders:

```text
stale_takeover_available=<true|false>
stale_takeover_claim_id=<CLAIM_ID|none>
stale_takeover_previous_session_id=<SESSION_ID|none>
stale_takeover_required_previous_session_lifecycle_state=<potentially_stale|none>
```

The fields are populated only when the guard reason is
`exclusive_claim_owned_by_other_session`. They identify the blocking Claim and
previous owning Session needed for the existing stale-gated recovery chain.

All other guard states render `stale_takeover_available=false` and `none`
values. Existing indexed `active_claim.*` output remains available for full
inspection.

The guard remains read-only. It does not mark the previous Session stale and
does not perform Claim takeover.

## Dogfood Run

Commands were run from:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lu-claim-guard-recovery-hints
```

The local dogfood Store and log were:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lu-claim-guard-recovery-hints/.work-governance/runtime/dogfood/phase-4lu-20260901T011735Z.sqlite
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lu-claim-guard-recovery-hints/.work-governance/runtime/logs/phase-4lu/claim-guard-recovery-hints.20260901T011735Z/run.log
```

The dogfood Store contained one Task, one owner Session, and one continuation
Session. The run checked three guard states: unclaimed, owned exclusive, and
blocked by another Session's exclusive Claim.

The run proved:

- unclaimed guard output renders `stale_takeover_available=false` and `none`
  ids;
- owned exclusive guard output renders `stale_takeover_available=false` and
  `none` ids;
- blocked guard output renders `allowed=false` and
  `reason=exclusive_claim_owned_by_other_session`;
- blocked guard output renders `stale_takeover_available=true`;
- blocked `stale_takeover_claim_id` matched the owner Claim id;
- blocked `stale_takeover_previous_session_id` matched the owner Session id;
  and
- blocked
  `stale_takeover_required_previous_session_lifecycle_state=potentially_stale`.

Summary:

```text
phase4lu_dogfood_result=PASS
dogfood_store=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lu-claim-guard-recovery-hints/.work-governance/runtime/dogfood/phase-4lu-20260901T011735Z.sqlite
log_file=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lu-claim-guard-recovery-hints/.work-governance/runtime/logs/phase-4lu/claim-guard-recovery-hints.20260901T011735Z/run.log
task_entity_id=01a05a8b-0c86-7283-a91d-8eff8f4a9ba1
owner_session_id=01a05a8b-0c9b-7561-8319-47b5906280cc
owner_claim_id=01a05a8b-0cc6-70c3-bd05-c69d996f5a25
continuation_session_id=01a05a8b-0cef-76a2-9e77-d1edfe7e2db8
unclaimed_stale_takeover_available=false
owned_stale_takeover_available=false
blocked_stale_takeover_available=true
blocked_stale_takeover_claim_id=01a05a8b-0cc6-70c3-bd05-c69d996f5a25
blocked_stale_takeover_previous_session_id=01a05a8b-0c9b-7561-8319-47b5906280cc
```

## Validation

Targeted validation passed:

```text
cargo fmt --all
cargo test -p workvcs-cli cli_shows_and_lists_session_active_claims --quiet
```

The CLI test proves:

- unclaimed guard states render no stale-takeover hint;
- owned exclusive guard states render no stale-takeover hint;
- blocked exclusive-other-session guard states render the owner Claim id,
  owner Session id, and required stale state; and
- existing `active_claim.*` output and expectation fields remain present.

## Findings

- The improvement is a recovery hint, not a recovery mutation.
- Operators can now read the next required IDs from stable top-level keys
  instead of scanning indexed active-claim rows.
- The next recovery steps remain explicit and auditable:
  `session mark-stale`, then `claim takeover --force`.
