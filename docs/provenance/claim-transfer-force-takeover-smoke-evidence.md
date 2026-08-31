# Claim Transfer and Force Takeover Smoke Evidence

Status: Phase 4LB local smoke and early implementation evidence
Recorded: 2026-09-01

This file records evidence for the Claim replacement operations added by
ADR-0417.

The implemented path is intentionally narrow:

1. `claim transfer` replaces an active Claim occurrence owned by one active
   Session with a new active Claim occurrence for another active Session on the
   same Workspace and Branch;
2. `claim takeover --force` replaces another Session's active Claim occurrence
   only when the caller supplies a non-empty rationale;
3. both commands preserve the replaced Claim as a released occurrence, keep the
   current active-set invariant in `claim_runtime`, and record immutable Event
   payloads with prior claimant and replacement details; and
4. the post-takeover guard recognizes the taking Session as the single active
   exclusive claimant; and
5. neither command creates a WorkStateCommit or changes Task lifecycle.

Validated command surface:

```text
workvcs claim transfer STORE \
  --from-session SOURCE_SESSION_ID \
  --to-session TARGET_SESSION_ID \
  --claim CLAIM_ID \
  --expected-previous-claim CLAIM_ID \
  --expected-from-session SOURCE_SESSION_ID \
  --expected-to-session TARGET_SESSION_ID \
  --expected-mode exclusive \
  --expected-lifecycle-state active

workvcs claim takeover STORE \
  --session TAKING_SESSION_ID \
  --claim CLAIM_ID \
  --force \
  --rationale "operator recovery smoke" \
  --expected-previous-claim CLAIM_ID \
  --expected-previous-session PREVIOUS_SESSION_ID \
  --expected-session TAKING_SESSION_ID \
  --expected-mode exclusive \
  --expected-lifecycle-state active

workvcs claim guard STORE \
  --session TAKING_SESSION_ID \
  --task TASK_ENTITY_ID \
  --expected-allowed true \
  --expected-reason owned_exclusive_claim \
  --expected-active-claims 1
```

Targeted validation:

```text
cargo test -q -p workvcs-core --test claim_runtime_phase3f
result: passed

cargo test -q -p workvcs-cli cli_runs_claim_transfer_and_force_takeover_workflow
result: passed
```

Repository smoke validation:

```text
scripts/smoke-v0.1-cli-workflow.sh
smoke_result=passed
store_id=01a058d9-6368-7be3-b746-2c0cccb33bb2
workspace_id=01a058d9-6ac3-7782-9b44-e40fe71eab01
branch_id=01a058d9-6ac3-7782-9b44-e43a644fb0bf
task_entity_id=01a058d9-6f2e-72c1-a065-33d7f20887b7
claim_transfer_id=01a058db-417f-7db0-9448-ee170230f561
claim_takeover_id=01a058db-487b-73d3-adab-b988b26aea79
```

Residual gaps:

- Stale takeover is not implemented because the runtime does not yet have a
  concrete `potentially_stale` Session state or policy.
- The forced takeover command does not bypass semantic verification gates or
  terminal Task guard enforcement by itself.
- This remains smoke-level evidence; durable dogfood should use these commands
  to recover a real blocked implementation handoff.
