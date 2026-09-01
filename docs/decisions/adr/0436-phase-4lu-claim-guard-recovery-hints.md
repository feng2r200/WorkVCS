# ADR-0436: Phase 4LU Claim Guard Recovery Hints

Status: Accepted
Date: 2026-09-01

## Context

Phase 4LG dogfooded a focused Handoff continuation blocked by another active
Session's exclusive Claim. The recovery semantics were correct, but the
operator still had to identify and copy the previous owning Session id and
blocking Claim id before running the explicit stale-gated recovery commands.

`claim guard` already resolves the authoritative active Claims for the focused
Task. The lowest-risk fix is to make that existing guard output expose stable
top-level recovery hint fields for the one recovery case proven by dogfood,
without combining or automating the recovery mutation.

## Decision

Extend `workvcs claim guard` output with stable hint fields:

```text
stale_takeover_available=<true|false>
stale_takeover_claim_id=<CLAIM_ID|none>
stale_takeover_previous_session_id=<SESSION_ID|none>
stale_takeover_required_previous_session_lifecycle_state=<potentially_stale|none>
```

The hint is available only when the guard reason is
`exclusive_claim_owned_by_other_session` and the active Claim set contains the
blocking active exclusive Claim owned by another Session.

All other guard states render `stale_takeover_available=false` and `none`
values. Existing `allowed`, `reason`, `active_claims`, and indexed
`active_claim.*` output remain present.

The command remains read-only. It does not mark Sessions stale, perform
takeover, update Claim runtime, write Events, move Branch heads, or change
WorkState. Operators must still deliberately run:

```text
workvcs session mark-stale STORE --session PREVIOUS_SESSION --rationale TEXT
workvcs claim takeover STORE --session TAKING_SESSION --claim CLAIM --force --rationale TEXT
```

## Non-Goals

- No automatic stale detection.
- No automatic or combined takeover command.
- No schema change.
- No Claim runtime redesign.
- No new guard reason.
- No change to stale-gated forced takeover requirements.
- No Agent orchestration, remote lock negotiation, or distributed recovery.

## Evidence

Targeted validation passed:

```text
cargo fmt --all
cargo test -p workvcs-cli cli_shows_and_lists_session_active_claims --quiet
```

Local dogfood passed through the real CLI:

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

## Consequences

Blocked focused Handoff recovery now requires less manual scanning: the guard
output gives the exact Claim and previous Session ids needed by the next
explicit commands. The safety boundary remains unchanged because recovery is
not inferred or executed automatically.

Further CLI/operator work should target real continuation workflows that still
require repeated ID plumbing, or move to another-project dogfood before
claiming broader V1 maturity.
