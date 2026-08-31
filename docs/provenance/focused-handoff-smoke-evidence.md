# Focused Handoff Smoke Evidence

Status: Phase 4LA local smoke and early implementation evidence
Recorded: 2026-09-01

This file records the evidence for the focused Handoff wrapper added by
ADR-0416.

The implemented path is intentionally narrow:

1. a Session can be ended with a canonical SessionDiff summary;
2. a caller explicitly creates a `Record(kind=handoff)` through `handoff create`;
3. the Handoff scope records the source Session id, source lifecycle state,
   optional SessionDiff id, and optional focus Entity id; and
4. a receiver can run `handoff show` at a branch head or commit to inspect the
   Handoff and linked SessionDiff summary without direct SQLite access.

Validated command surface:

```text
workvcs handoff create STORE \
  --branch BRANCH_ID \
  --head CURRENT_HEAD \
  --session SESSION_ID \
  --statement "Continue from smoke session" \
  --session-diff SESSION_DIFF_ID \
  --focus TASK_ENTITY_ID \
  --expected-session-diff SESSION_DIFF_ID \
  --expected-focus TASK_ENTITY_ID

workvcs handoff show STORE \
  --commit HANDOFF_COMMIT_ID \
  --handoff HANDOFF_RECORD_ENTITY_ID \
  --expected-session SESSION_ID \
  --expected-session-diff SESSION_DIFF_ID \
  --expected-focus TASK_ENTITY_ID
```

Targeted validation:

```text
cargo test -q -p workvcs-core --test session_runtime_phase3e
result: passed

cargo test -q -p workvcs-cli cli_runs_focused_handoff_workflow
result: passed
```

Repository smoke validation:

```text
scripts/smoke-v0.1-cli-workflow.sh
smoke_result=passed
store_id=01a058c0-4d24-70e0-9bb0-b60962cc5dec
workspace_id=01a058c0-5393-7f82-97fd-1a38407c529d
branch_id=01a058c0-5393-7f82-97fd-1a62384fbd50
task_entity_id=01a058c0-56c7-71e3-9c5a-6fc2dad85c8c
session_diff_id=01a058c1-da47-7b90-8a1a-e9851839cb40
handoff_record_id=01a058c1-dd88-7523-9493-bf67e650eb5a
```

Residual gaps:

- This is not yet a durable WorkVCS-to-WorkVCS implementation handoff.
- `handoff show` renders linked SessionDiff context but does not yet feed the
  Context resolver as a first-class packet.
- There is no typed Handoff relation, Handoff table, remote handoff protocol, or
  automatic transcript extraction in this slice.
