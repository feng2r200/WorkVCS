# Durable-operation delivery

Use this when continuing cognition or a typed Plan operation has been admitted
or needs reconciliation. It explains the existing explicit recovery route; it
adds no automatic delivery, authorization, bootstrap, or background queue.

Reuse this sequence while its version, route, target, and authority remain
applicable. Each mutation still needs fresh status guards; each checkpoint does
not need another full read of this reference or the project history.

## Read the current state once

After admission, keep the compatibility CaptureId, `payload_kind`, semantic
scope, and exact registry route.
Run `workvcs project operation-recovery --status --capture-id <id>` using that
same route. Add `--registry <path>` only when it was the verified selected route.
Status is read-only. `payload_kind` identifies `cognition_v2`,
`plan_admit_v1`, or `plan_evolve_v1`; it is not permission to reinterpret one
domain payload as another. Read these fields together:

| Current status | Meaning and next boundary |
| --- | --- |
| `resolution_status=resolved`, `binding_state=valid`, `recovery_action=apply_binding_receipt` | An existing target is valid but this capture lacks its binding receipt. `pending_project` here does not mean the project is unbound; do not call `project ensure` or create a new binding. |
| `resolution_status=unbound`, `recovery_action=apply_project_bootstrap` | Missing ownership or binding convergence requires its own authorized recovery. Do not fall back from a known semantic owner. |
| `pending_primary` | Primary delivery or its receipt is incomplete. After an uncertain apply, inspect status and recover the same operation under fresh guards. Plan recovery replays the original manifest idempotently. |
| `plan_target_conflict`, `plan_manifest_rejected`, or `plan_receipt_too_large` | The typed Plan operation ended deterministically before a Store write. Preserve it for audit and start a new operation with current guards, corrected content, or a smaller manifest; do not replay the terminal Capture ID. |
| `pending_references` | Primary success does not complete the declared CaptureGroup. Only authorized immutable reference convergence remains. |
| `effective_recovery_state=completed`, `delivery_receipt=true`, `recovery_action=none` | The recorded delivery completed. Verify its exact target and semantic result once; do not apply again merely to poll. |
| Conflict, invalid binding, terminal failure, or an unrecognized result | Preserve the capture and diagnose the stated cause. Do not convert it to success, recreate its intent, or bypass a gate. |

The read-only status field `target_delivery_written=false` describes what that
status call wrote, not whether an earlier delivery exists. Use the recovery
state, receipt, and target commit together. A cached projection or a process
exit code alone is not evidence that the checkpoint is recoverable.

If the CaptureId was lost after admission, inspect the backlog without
mutating it:

```sh
workvcs project operation-recovery --list-open [--registry <path>]
```

Filter by ProjectRef, payload kind, or recovery action when useful. The command
derives state from immutable authority, treats stored projections only as
cache, and renders allowlisted metadata rather than raw keys or payload text.
For a later page, supply both the returned `next_after_capture_id` and the same
`inventory_digest`; a mismatch fails closed. A listed action is not authority
to perform it.

## Keep ownership with the responsible task

The task that admits or starts a durable operation owns its ordinary follow-
through. Keep its CaptureId and, when existing authority already covers the
same operation and exact target, complete status, delivery, receipt repair,
and exact target readback without asking the user to monitor routine pending
state. A user decision is needed only when the next step introduces new
authority, a changed owner or target, a material semantic-currentness choice,
or another separately guarded mutation.

Start with the current task's known CaptureIds. If an older open inventory is
relevant, classify each operation before applying it and state the
counterfactual effect of acting now. Read the projected and effective recovery
states together with any delivery failure, receipt, and target commit:

- a completed projection with a stale binding receipt is receipt-reconciliation
  work and must not create the semantic objects again;
- a deterministic terminal projection remains terminal even if a later
  registry change makes the effective state look pending;
- superseded undelivered intent must not be written into the Store merely to
  clear the inventory; and
- an immutable target change requires a new capture only when the semantics
  are still current and separately authorized for that target.

The provider has no authority to infer these semantic decisions from age or
queue position. If currentness cannot be established from the responsible
task's context and verified project truth, preserve the CaptureId and do not
apply it.

## Deliver within the existing authority

Explicit delivery authority must cover this operation and the verified target.
An existing user instruction that already covers the current operation is
sufficient; do not ask for the same permission again. Authority to capture or
edit source alone does not authorize Store delivery. Newly unbound ownership,
target changes, migration, activation, marker refresh, or a historical batch
still needs its own covered scope. A resolved binding receipt can be recorded
by recovery without bootstrapping or changing the registry.

When the current operation is authorized, use the same installed binary and
fresh status digests:

```sh
workvcs project operation-recovery --apply --capture-id <id> \
  --expected-registry-digest <registry_digest_from_status> \
  --expected-projection-digest <projection_digest_from_status>
```

Carry the same explicit `--registry <path>` if one was selected. Do not add a
Store root or invoke activation to make an existing-target delivery work.
Recheck only the affected state if a guard rejects a changed basis. Following
an indeterminate result, status comes first; retry only the same operation with
fresh guards inside the existing authority. Changing the idempotency identity
can duplicate a Store commit that succeeded before its receipt was lost.

Then read status and recover the exact delivered record or a focused Recall
from the verified Store/Workspace/Branch. Check its semantic scope and target
commit. A bounded Recall may omit an older item; use its stable ID instead of
cycling through broad profiles. For a CaptureGroup, also verify required
references and completion. Once that boundary is proven, resume useful work.

For a newly submitted, non-group `cognition_v2` operation whose authority
already covers delivery to the exact existing binding, the shorter equivalent
is:

```sh
workvcs capture --cwd <context> --manifest <file> --value-reason <reason> \
  --deliver-existing-binding [--registry <path>] [--project-ref <id>]
```

The default without the option remains admission-only. The option never
bootstraps an unbound owner, changes or repairs a target, writes a shared
binding, refreshes markers, or processes a historical batch. It admits or
reuses the intent first and then calls the same recovery engine with fresh
guards. Success requires a current verified receipt; completed replay is
zero-write. `capture_delivery_incomplete` means admission is durable but the
requested continuation did not complete. Preserve its CaptureId and inspect
status before deciding any separately authorized recovery. A clean underlying
`project_owner_unbound` is a missing-binding classification, not corruption or
fallback authority.

## If delivery is not currently authorized or possible

Report the truthful checkpoint: admitted and journal-persistent, but not yet
recoverable through the target Store. Retain the existing capture ID, unresolved
action, target when known, and the minimum next-step evidence. Several mechanical
status checks do not justify new captures. Record a new delta only for changed
meaning, authority, failure diagnosis, or outcome; continue independent safe
work without claiming Store recovery is complete.
