# Session Potentially Stale Smoke Evidence

Status: Phase 4LD local smoke and early implementation evidence
Recorded: 2026-09-01

This file records evidence for explicit Session stale marking added by
ADR-0419.

The implemented path is intentionally narrow:

1. `session mark-stale` changes an active Session runtime to
   `potentially_stale` only when the caller supplies a non-empty rationale;
2. the Session keeps its active Workspace, Branch, focus, and Claim recovery
   context;
3. active-only operations reject the potentially stale Session; and
4. `session end` can still close the stale Session and release active Claims.

Validated command surface:

```text
workvcs session mark-stale STORE \
  --session SESSION_ID \
  --rationale "operator observed no heartbeat" \
  --expected-session SESSION_ID \
  --expected-lifecycle-state potentially_stale \
  --expected-active-workspace WORKSPACE_ID \
  --expected-active-branch BRANCH_ID

workvcs session show STORE \
  --session SESSION_ID \
  --expected-lifecycle-state potentially_stale \
  --expected-active-workspace WORKSPACE_ID \
  --expected-active-branch BRANCH_ID

workvcs session list STORE \
  --lifecycle potentially_stale \
  --workspace WORKSPACE_ID \
  --branch BRANCH_ID \
  --expected-sessions 1
```

Targeted validation:

```text
cargo test -q -p workvcs-core --test session_runtime_phase3e mark_session_potentially_stale
result: passed

cargo test -q -p workvcs-cli cli_marks_session_potentially_stale_and_preserves_recovery_path
result: passed
```

Repository smoke validation:

```text
scripts/smoke-v0.1-cli-workflow.sh
smoke_result=passed
store_id=01a058fc-f7ec-7d90-88de-15138d3deedc
workspace_id=01a058fc-fe5c-75b2-b790-a0868a97d69b
branch_id=01a058fc-fe5c-75b2-b790-a0b15d3e4bbb
task_entity_id=01a058fd-016b-7ab2-97ff-68cccfa3419b
stale_session_id=01a058fe-a9f6-7d33-b585-2c190085eb22
```

Residual gaps:

- Automatic stale detection and heartbeat policy remain Open.
- Stale-gated Claim takeover policy remains Open.
- This remains smoke-level evidence; durable dogfood should use stale marking
  in a real interrupted implementation recovery path.
