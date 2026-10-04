# Checkpoint delivery

Use this when a continuing checkpoint has been admitted or needs reconciliation.
It explains the existing explicit recovery route; it adds no automatic delivery,
authorization, bootstrap, or background queue.

Reuse this sequence while its version, route, target, and authority remain
applicable. Each mutation still needs fresh status guards; each checkpoint does
not need another full read of this reference or the project history.

## Read the current state once

After admission, keep the capture ID, semantic scope, and exact registry route.
Run `workvcs project capture-recovery --status --capture-id <id>` using that
same route. Add `--registry <path>` only when it was the verified selected route.
Status is read-only. Read these fields together:

| Current status | Meaning and next boundary |
| --- | --- |
| `resolution_status=resolved`, `binding_state=valid`, `recovery_action=apply_binding_receipt` | An existing target is valid but this capture lacks its binding receipt. `pending_project` here does not mean the project is unbound; do not call `project ensure` or create a new binding. |
| `resolution_status=unbound`, `recovery_action=apply_project_bootstrap` | Missing ownership or binding convergence requires its own authorized recovery. Do not fall back from a known semantic owner. |
| `pending_primary` | Primary delivery or its receipt is incomplete. After an uncertain apply, inspect status and recover the same capture under fresh guards. |
| `pending_references` | Primary success does not complete the declared CaptureGroup. Only authorized immutable reference convergence remains. |
| `effective_recovery_state=completed`, `delivery_receipt=true`, `recovery_action=none` | The recorded delivery completed. Verify its exact target and semantic result once; do not apply again merely to poll. |
| Conflict, invalid binding, terminal failure, or an unrecognized result | Preserve the capture and diagnose the stated cause. Do not convert it to success, recreate its intent, or bypass a gate. |

The read-only status field `target_delivery_written=false` describes what that
status call wrote, not whether an earlier delivery exists. Use the recovery
state, receipt, and target commit together. A cached projection or a process
exit code alone is not evidence that the checkpoint is recoverable.

## Deliver within the existing authority

Explicit delivery authority must cover this capture and the verified target.
An existing user instruction that already covers the current operation is
sufficient; do not ask for the same permission again. Authority to capture or
edit source alone does not authorize Store delivery. Newly unbound ownership,
target changes, migration, activation, marker refresh, or a historical batch
still needs its own covered scope. A resolved binding receipt can be recorded
by recovery without bootstrapping or changing the registry.

When the current operation is authorized, use the same installed binary and
fresh status digests:

```sh
workvcs project capture-recovery --apply --capture-id <id> \
  --expected-registry-digest <registry_digest_from_status> \
  --expected-projection-digest <projection_digest_from_status>
```

Carry the same explicit `--registry <path>` if one was selected. Do not add a
Store root or invoke activation to make an existing-target delivery work.
Recheck only the affected state if a guard rejects a changed basis. Following
an indeterminate result, status comes first; retry only the same capture with
fresh guards inside the existing authority. Changing the idempotency identity
can duplicate a Store commit that succeeded before its receipt was lost.

Then read status and recover the exact delivered record or a focused Recall
from the verified Store/Workspace/Branch. Check its semantic scope and target
commit. A bounded Recall may omit an older item; use its stable ID instead of
cycling through broad profiles. For a CaptureGroup, also verify required
references and completion. Once that boundary is proven, resume useful work.

## If delivery is not currently authorized or possible

Report the truthful checkpoint: admitted and journal-persistent, but not yet
recoverable through the target Store. Retain the existing capture ID, unresolved
action, target when known, and the minimum next-step evidence. Several mechanical
status checks do not justify new captures. Record a new delta only for changed
meaning, authority, failure diagnosis, or outcome; continue independent safe
work without claiming Store recovery is complete.
