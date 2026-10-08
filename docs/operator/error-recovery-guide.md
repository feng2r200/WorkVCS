# WorkVCS Error Recovery Guide

Status: Current V1-local and ProjectRef-v2 durable-operation recovery guidance
Last updated: 2026-10-08

This guide covers the stable error fields emitted by the current CLI. It is
intentionally an operator recovery contract, not a new recovery engine or Store
protocol.

## Read The Fields

By default, WorkVCS business errors render stderr as line-oriented key-value
fields:

```text
error_code=<CODE>
error_category=<CATEGORY>
retryable=<true|false>
message=<ESCAPED_MESSAGE>
```

Top-level CLI syntax errors render the same default key-value shape:

```text
error_code=cli_parse_error
error_category=usage
retryable=false
clap_error_kind=<lower_snake_case>
message=<ESCAPED_MESSAGE>
```

Route automation by `error_code`, `error_category`, and `retryable`.
`message` is display-only context. Do not parse it for control flow.

`mutation_postcondition_failed` adds stable fields that distinguish a completed
mutation from a rejected precondition:

```text
operation_completed=true
operation=<COMMAND_FAMILY>
operation_result=<ESCAPED_RENDERED_RESULT>
recovery_hint=inspect_operation_result_before_retry
```

Read `operation_result` and current state before deciding whether a compensating
action is required. Replaying the original command can duplicate intent.

`project_binding_not_found` identifies a read-only discovery miss and adds a
stable recovery route:

```text
recoverable=true
recovery_action=project_ensure
recovery_cwd=<RESOLVED_LOGICAL_PROJECT_ROOT>
recovery_registry=<RESOLVED_REGISTRY_PATH>
```

Confirm that `recovery_cwd` is the intended logical project, then use
`workvcs project ensure --cwd <PATH>`. A direct registry locator also needs
`--store-root PATH`. Discovery itself never performs this write. Do not turn
this error into a no-record decision or silently redirect a known semantic
Project into its repository/mirror. ProjectRef-v2 uses dedicated inactive
codes when a required marker or named capability is cleanly absent. Stale,
malformed, mismatched, symlinked, unreadable, unresolved, unbound, or
conflicting control-plane authority remains `control_plane_invalid`. The
value-qualified `capture` route can
journal admitted content without target Store delivery when its exact
installed revision and activation markers are verified. It reports routing
state in successful output rather than `registry_migration_required` error
codes.

Under registry v2, a valid resolver result whose winning owner has no binding
is `project_owner_unbound` on reads that require a Store. It adds:

```text
recoverable=true
recovery_action=admit_operation_then_authorize_project_bootstrap
registry_path=<PATH>
resolution_rank=<RANK>
unmapped_locators=<COUNT>
resolution_diagnostics=<COUNT>
```

This is a clean missing-owner state, not corrupt authority and not permission
to fall back, ensure, or bootstrap. `project health --cwd` reports it as
degraded unless `--require-healthy` turns it into the top-level error. Default
capture and recovery status retain successful `resolution_status=unbound`.
Malformed, stale, ambiguous, conflicting, or unsafe authority remains
`control_plane_invalid`.

An explicitly requested existing-binding continuation that admitted its intent
but did not reach a verified receipt returns `capture_delivery_incomplete`:

```text
capture_id=<CAPTURE_ID>
journal_persisted=true
cause_error_code=<UNDERLYING_CODE>
recovery_action=inspect_operation_recovery_status
```

The intent is durable. Start with read-only status for that CaptureId; do not
change the idempotency key, infer rollback, or treat the wrapper as permission
for the underlying bootstrap, repair, isolation, marker, or target action.

The three inactive codes add bounded recovery fields in both key-value and
JSON output:

```text
recoverable=true
activation_scope=<project_ref_v2_read_routing|project_ref_v2_journal_admission>
activation_state=<absent|active>
activation_path=<PATH>
activation_version=<VERSION>          # capability error only
required_capability=<CAPABILITY>       # capability error only
recovery_action=<EXPLICIT_ACTION>
```

These fields identify the next inspect/explicit-activation route; they do not
authorize it and `retryable` remains `false`.

For `--locator-adapter-context`, distinguish absence from invalid input. An
omitted adapter or a valid context with neither Project metadata nor mirror
evidence means the provider is unavailable, so the generic resolver may use
verified Git then CWD. A supplied file with an unsupported adapter, unknown or
secret-bearing field, malformed Project/thread ID, unverifiable mirror path,
or bad verification-source set is an explicit invalid assertion and fails
closed. Correct or regenerate the trusted handoff; do not delete it merely to
force repository fallback when semantic Project context is known. When
authoritative metadata conflicts with the canonical mirror, the authoritative
Project wins and `context_mismatch` must be investigated before treating the
mirror as related context.

`project health` is the composite read-only starting point for registry,
binding, activation, capability, and optional CWD-resolution diagnosis. It
loads one registry snapshot and performs one complete validation pass over all
bindings. `--require-healthy` is suitable for a gate; `--timings` is diagnostic
only. Health never installs or refreshes a marker and never weakens complete
Store integrity validation. Use the focused status command named by its output
before deciding whether a separately authorized activation or recovery apply
is appropriate.

`project routing-activation --status` is read-only for registry v1 or v2.
`--preview` requires v2 and reports the exact registry and candidate digests.
Activation apply must receive both expected digests and refuses to replace an
invalid or existing nonmatching marker. An ordinary apply also refuses a stale
marker. If status proves that registry bootstrap advanced the same registry
lineage, apply may additionally lock the exact installed old marker with
`--expected-activation-digest`; only an earlier revision of that same registry
ID is atomically replaced. Another lineage, an equal/newer revision, an absent
marker, or an old-marker digest mismatch remains fail-closed. Do not delete or
hand-edit a marker as error recovery: inspect the selected registry, marker
path, and reported state. A matching existing marker is an idempotent reuse. A successful
marker enables v2 reads only and never authorizes journal or Store writes.
If marker installation has completed or may have completed but temp cleanup,
directory sync, or post-install verification fails, the command returns
`routing_activation_install_indeterminate`. Do not replay apply immediately.
Run `project routing-activation --status` against the same registry, preserve
the reported marker, registry, and candidate digests, and decide recovery only
from that observed state. An exact active marker makes a later exact apply an
idempotent reuse; any other state remains fail-closed.

`project journal-admission-activation --status` and `--preview` are read-only.
Apply requires the exact registry and candidate digests plus an active exact
read-routing marker. Disable requires the exact registry and installed marker
digests. Both operations hold the registry lock before the shared
journal-quiescence lock; disable therefore excludes new admissions before
removing the marker. Never delete or hand-edit this marker. A malformed,
wrong-scope, or symlinked marker is fail-closed. A stale marker can be refreshed
only with the exact installed marker digest when it is an earlier revision of
the same registry lineage. One narrower same-snapshot refresh is allowed: a
version-1 cognition-only marker can become the version-2 strict capability
superset when the registry identity, revision, and digest are unchanged and
the exact installed marker digest is supplied. Capability removal, lateral
replacement, or implicit broadening remains fail-closed. A failed post-install
step uses `routing_activation_install_indeterminate`; inspect journal-admission
status before retrying. A failed post-removal step uses
`routing_activation_disable_indeterminate`; status must prove whether the
marker is absent before any retry or recovery. Neither successful state change
authorizes ProjectRef bootstrap or target Store delivery.

`project operation-recovery --status --capture-id ID` is the canonical
read-only authority for one admitted durable operation. The visible
`project capture-recovery` spelling remains a compatibility alias over the
same CaptureId and journal state. Status reports the exact
`payload_kind` (`cognition_v2`, `plan_admit_v1`, or `plan_evolve_v1`), validates
immutable events, rebuilds the projection in memory, compares any stored
projection, and reports the current registry digest and exact next action. A
malformed stored projection may be rebuilt from valid events; an invalid event
sequence, payload digest, or digest chain is authority corruption and fails
closed.

Recovery apply is a distinct, explicit operation requiring both the exact
status-observed registry and projection digests. A stale guard fails before
mutation. `capture_recovery_install_indeterminate` means a registry, event, or
projection install may already be durable. Do not delete an initialized Store,
restore an older registry, or blindly replay. Run `--status` first and continue
forward using fresh digests only if that observed action remains intended.
For `pending_primary`, the explicit apply path records exact target guards,
performs one idempotent typed target operation, and records its receipt. A
failure after the Store commit but before the receipt is indeterminate: status
still shows `pending_primary`, and retry must use the same CaptureId and
original manifest idempotency key so the Store returns the existing commit
with `reused=true`. Plan recovery must remain Plan recovery; it is never
converted into cognition. Do not delete or roll back that Store.
`legacy_manifest_upgrade_required` means the retained v1 manifest did
not carry guards matching the selected Branch; preserve it and make a separate
upgrade decision. `semantic_manifest_invalid` means a historical immutable
intent failed deterministic cognition semantics before target mutation;
preserve it and follow
`recovery_action=start_new_capture_with_corrected_payload`. For example, a
Handoff summarized from a Finding uses `Handoff --derived_from--> Finding`,
not `Finding --supports--> Handoff`.

Typed Plan recovery performs pure manifest validation, an explicit target-
snapshot guard comparison, and receipt-size preflight using a fixed maximum-
length timestamp envelope before its first Store write.
Timestamps are capped at nanosecond precision. A proven manifest or target
failure is written before receipt construction; an existing entity of the
wrong Goal/Plan kind is `plan_target_conflict`, not a pending lookup error.
`plan_receipt_too_large` therefore proves zero Store mutation; split the
manifest and admit a new operation. The terminal states
`plan_target_conflict` and `plan_manifest_rejected` likewise preserve the old
intent for audit, but exist only for mismatches proved by that pre-write
analysis. Do not replay those terminal captures. Engine execution, storage,
integrity, transaction, control-plane, and uncertain post-commit failures are
deliberately not terminalized from a generic error code; recover the same
Capture ID from read-only status.

A Plan `delivery_applied` receipt is valid only when its operation kind and
complete result-object shape match the admitted manifest variant. After that
receipt is durable, the public command renders the result through a read-only
Store lookup and does not call the mutating Plan operation again.
Plan record aliases are receipted under their canonical kind
(`unknown` becomes `question`). Once a terminal `delivery_failed` exists for
the attempt, neither another failure nor a receipt can replace it; preserve the
Capture ID for audit and follow its recorded recovery action.

`pending_references` preserves a
successful primary while one or more immutable secondary references are
missing. Retry from fresh
status digests: the recovery path installs only missing reference events and
must not roll back or redeliver the primary. If all references are present but
the completion summary is absent, `recovery_action=apply_capture_completion`
means only the idempotent `capture_completed` event remains. Use the read-only
`project capture-group-recall --project-ref-id ID` command to inspect installed
associations from a secondary ProjectRef; it must report `store_opened=false`
and `store_written=false`. A missing secondary ProjectRef remains pending and
is not silently recreated. After canonical delivery,
`restore_exact_canonical_owner_or_start_new_capture` means current resolution
no longer names that authority, while
`start_new_capture_canonical_target_changed` means its binding tuple changed;
neither condition permits retargeting the existing CaptureGroup. This
route has isolated-fixture fault validation; live apply still requires exact
status digests and operation authority.

Migration ownership-repair manifests fail as `control_plane_invalid` when
their schema, semantic locator, source digest, binding key, or target digest is
not exact. Do not weaken or hand-edit around the guard. Regenerate the manifest
from the current registry and current bounded ownership evidence, then rerun
preview. A repaired preview that reports a target `query_invalid` still means
the Store/Workspace/Branch/content target must be corrected; the historical
repair never bypasses target validation. Neither error authorizes apply.

The isolated apply candidate emits `registry_migration_apply_failed` only
before authoritative replacement, so v1 remains authoritative. Recompute the
source and preview digests after correcting the cause; an exact existing backup
may be reused, but no conflicting backup or temp artifact may be overwritten.
`registry_migration_install_indeterminate` means the atomic rename already
completed and a later step failed. Do not retry apply or restore the backup by
assumption. Inspect the actual registry and run the digest-locked read-only
`--rollback-check`. The source-tree `--rollback` candidate is a separate,
explicitly authorized operation and has only isolated-fixture evidence.

`registry_rollback_failed` means the v2-to-v1 authoritative replacement did
not occur. Correct the reported digest, receipt, activation, journal, backup,
snapshot, lock, or temp conflict. An exact v2 rollback snapshot installed
before the failure may remain reusable, but it is never treated as authority.
`registry_rollback_install_indeterminate` means the v1 restore rename occurred
and a later durability or verification step failed. Run `--rollback-check`
with the same digests first. A `v1_restored` result permits an explicit repeat
to return a verified no-op; do not blindly retry or infer recovery.
For a standard registry filename the probe checks both WorkVCS-home and
registry-sidecar activation/journal aliases, so switching between configured
home and explicit `--registry` is not a bypass. Apply and rollback are currently
unsupported on non-Unix platforms. The source candidate now coordinates
admission and rollback with
`<canonical-registry-path>.journal-quiescence.lock`; live durable admission is
still disabled even though the source candidate has an actual routed caller.
`--rollback-check` reports the exact path and one of `absent`,
`present`, `invalid`, or `unreadable`. A present lock may be active or may be
an orphan after a crash; WorkVCS intentionally does not steal it from its age
or PID. Confirm that no admission or rollback owner remains before removing
that exact file under a separately controlled procedure, then rerun the
read-only probe. Do not delete a broad lock directory or infer recovery from a
retry timeout.

For JSON stderr, pass `--error-format json`. WorkVCS business errors render one
JSON object:

```json
{"error_category":"task","error_code":"task_invalid","message":"task invalid: example","retryable":false}
```

Top-level CLI syntax errors include the stable clap kind:

```json
{"clap_error_kind":"unknown_argument","error_category":"usage","error_code":"cli_parse_error","message":"error: unexpected argument ...","retryable":false}
```

The default remains `--error-format key-value`.

## Retry Rule

Only `branch_head_conflict` is currently marked `retryable=true` in the core
error taxonomy. Treat every other code as not directly retryable. Recovery for
those codes starts by changing the input, selector, Store path, Resource state,
or operator intent; then rerun the command with a fresh expected head or
selector as applicable.

## Recovery Matrix Script

Run `scripts/operator-recovery-maturity-v0.1.sh` from the repository root when
you need a current local proof that this guide still matches the CLI and core
taxonomy. The script audits every current `ErrorCode::as_str()` value plus
`cli_parse_error`, checks the retryability rule above, and exercises
representative parse, branch-head, Resource, Claim, and merge recovery flows
through current CLI output.

## Per-Code Recovery

| error_code | category | retryable | Recovery action |
| --- | --- | --- | --- |
| `canonical_encoding_invalid` | `canonical` | `false` | Stop using the malformed canonical payload as authority. Regenerate it through the WorkVCS CLI or schema-backed producer, then rerun the operation with the corrected payload. |
| `control_plane_invalid` | `control_plane` | `false` | Stop using the malformed, stale, unbound, conflicting, or otherwise invalid control-plane input. For migration repair, regenerate an exact source/key/target-bound manifest. For v2 reads, run `project health` and focused status, then correct the evidence; never bypass a digest, owner rank, or marker guard. Clean absence uses one of the dedicated inactive codes below. |
| `routing_activation_inactive` | `control_plane` | `false` | Read routing is cleanly off. Inspect `project health` and `project routing-activation --status`, then use a separately authorized digest-locked activation apply if intended. Never auto-activate from this error. |
| `journal_admission_activation_inactive` | `control_plane` | `false` | Journal admission is cleanly off. Confirm read routing and inspect `project journal-admission-activation --status`, then use a separately authorized digest-locked apply if intended. Never auto-activate from this error. |
| `journal_admission_capability_inactive` | `control_plane` | `false` | The active journal marker lacks the named capability. Inspect the reported path/version/capability and explicitly refresh only with the exact installed-marker digest and strict capability-superset candidate. |
| `capture_not_persisted` | `control_plane` | `false` | The journal did not durably admit the capture. Preserve the bounded pending semantic packet, repair the journal path, permissions, lock, or storage fault, and retry with the same idempotency key before any target Store write. |
| `capture_idempotency_conflict` | `control_plane` | `false` | The idempotency key already names different capture content or identity. Inspect the existing immutable intent; reuse its exact payload or choose a genuinely new key for a distinct capture. Never overwrite the admitted intent. |
| `capture_delivery_incomplete` | `control_plane` | `false` | The capture intent is durable but the explicitly requested existing-binding continuation did not reach a verified current receipt. Preserve `capture_id`, inspect `project operation-recovery --status` first, and decide any bootstrap, repair, isolation, marker, target, or recovery apply separately from the reported underlying cause. Never replace the idempotency identity to escape an uncertain target result. |
| `registry_migration_apply_failed` | `control_plane` | `false` | The atomic registry replacement did not occur. Preserve v1, correct the reported digest, manifest, Store, lock, backup, or temp conflict, recompute both expected digests, and retry only with explicit apply authority. |
| `registry_migration_install_indeterminate` | `control_plane` | `false` | The atomic rename occurred but later durability or verification failed. Inspect the actual v2 registry and exact backup with `registry-migrate --rollback-check`; do not retry apply or infer rollback. |
| `registry_rollback_failed` | `control_plane` | `false` | The authoritative registry was not replaced. Correct the reported digest, receipt, activation, journal, backup, snapshot, lock, or temp conflict; reuse an exact retained v2 snapshot only through another explicitly authorized rollback. |
| `registry_rollback_install_indeterminate` | `control_plane` | `false` | The v1 restore rename occurred but later durability or verification failed. Run `registry-migrate --rollback-check` with the same digests; continue only from its observed `v1_restored` or `blocked` state. |
| `routing_activation_install_indeterminate` | `control_plane` | `false` | The activation marker was installed or may have been installed before a later cleanup, durability, or verification failure. Run `project routing-activation --status` against the same registry before any retry; never delete, overwrite, or infer marker state from the failed command alone. |
| `routing_activation_disable_indeterminate` | `control_plane` | `false` | The journal-admission marker was removed or may have been removed before a later durability or verification failure. Run `project journal-admission-activation --status` against the same registry before retry or recovery; never infer disabled state from the failed command alone. |
| `capture_recovery_install_indeterminate` | `control_plane` | `false` | A recovery registry, event, target commit, receipt, or projection install may already be durable. Run `project operation-recovery --status --capture-id ID` against the same registry, use its fresh registry/projection digests, and converge forward only from the observed state; never guess rollback or remove a bootstrapped or delivered Store. The historical code name is stable. |
| `shared_binding_isolation_install_indeterminate` | `control_plane` | `false` | The registry replacement completed or may have completed. Rerun `project isolate-shared-binding --preview` for the same CWD, ProjectRef, registry, and Store root. Continue only from its verified state; never replay apply blindly, edit the registry, delete the backup, or copy the source Store. |
| `digest_invalid` | `canonical` | `false` | Recompute the digest from the current canonical bytes. Do not copy a digest from another object or bypass digest checks. |
| `evidence_invalid` | `evidence` | `false` | Fix the Evidence content, kind, or referenced entities before retrying. Preserve the failed payload for audit if it came from an external artifact. |
| `evidence_not_found` | `evidence` | `false` | Confirm the Evidence id at the selected branch/head and use `evidence show` or adjacent list commands to recover the correct id. Do not recreate Evidence until the missing selector is understood. |
| `identity_invalid` | `identity` | `false` | Correct the identity id, actor, or metadata shape. This is an input-contract failure, not a concurrency retry. |
| `immutable_import_invalid` | `import` | `false` | Rebuild or re-export the import source and rerun validation. Do not partially apply an import that failed fixed-point checks. |
| `integrity_invalid` | `integrity` | `false` | Stop treating the Store as authoritative. Run `workvcs doctor "$STORE" --require-valid` and `workvcs store integrity "$STORE" --require-valid`, then repair from a known-good Store or Bundle. |
| `knowledge_invalid` | `knowledge` | `false` | Fix the Knowledge payload, kind, or support references. Avoid recording semantic knowledge that is not backed by the current evidence chain. |
| `knowledge_not_found` | `knowledge` | `false` | Re-check the Knowledge id at the selected branch/head. Use current list/show output before deciding whether new Knowledge should be created. |
| `locator_already_claimed` | `control_plane` | `false` | Do not reassign or merge automatically. Inspect both ProjectRefs and their exact locator evidence, then use a separately authorized link or conflict-resolution operation. |
| `commit_not_found` | `replay` | `false` | Confirm the commit id belongs to this Store lineage and selected branch. If it came from a Bundle or copied Store, validate the import/export boundary before retrying. |
| `branch_head_conflict` | `mutation` | `true` | Refresh the branch head, inspect the intervening history, then rerun only if the mutation still represents the operator intent against the new head. Never force a stale expected head. |
| `mutation_postcondition_failed` | `mutation` | `false` | The write completed and only a result-dependent expectation failed. Inspect `operation`, `operation_result`, and current state before any new action; never blindly replay the original mutation. Caller-known deterministic expectations fail before writing and do not use this code. |
| `branch_not_found` | `mutation` | `false` | Check the branch id and Store. Create or import the branch only if that is the intended state transition. |
| `claim_invalid` | `runtime` | `false` | Fix Claim mode, actor/session binding, expected head, or transition inputs. Use `claim guard` when the failure concerns protected mutation readiness. |
| `claim_not_found` | `runtime` | `false` | Re-check the Claim id and current branch head. If another operator released or transferred it, follow the handoff/claim recovery flow before mutating. |
| `entity_not_found` | `mutation` | `false` | Confirm the entity id, kind, and selected commit. Entity existence is version-scoped, so compare against the intended branch head before recreating anything. |
| `entity_transition_invalid` | `mutation` | `false` | Inspect the current entity status and allowed lifecycle transition. Choose the next valid transition or record a recovery Task instead of forcing the state. |
| `goal_invalid` | `goal` | `false` | Correct Goal content, status, containment, or transition input. If the issue is stale branch state, first refresh the selected head. |
| `goal_not_found` | `goal` | `false` | Re-check the Goal id at the selected branch/head. Use current Goal list/show output before creating a replacement. |
| `plan_invalid` | `plan` | `false` | Correct Plan content, status, containment, or transition input. Keep Plan changes aligned with the current Goal and Task graph. |
| `plan_not_found` | `plan` | `false` | Re-check the Plan id at the selected branch/head. If the Plan was superseded, use the current containment path. |
| `project_binding_not_found` | `query` | `false` | Current v1: preserve any valuable semantic packet, confirm the intended logical owner, then run `project ensure`; supply `--store-root` for a direct registry locator. Discovery made no changes. Never interpret the miss as no-record. |
| `project_owner_unbound` | `control_plane` | `false` | Registry v2 resolved a clean winning owner that has no usable binding. Preserve the semantic packet and owner rank, do not fall back to Git/CWD or auto-ensure, and admit first before seeking separate ProjectRef-bootstrap authority. Default capture/status may report the same state successfully as `resolution_status=unbound`; a flagged continuation wraps it in `capture_delivery_incomplete`. |
| `query_invalid` | `query` | `false` | Fix selector syntax, required ids, filter values, or mutually exclusive arguments. Use the relevant subcommand help before rerunning. |
| `query_unsupported` | `query` | `false` | Choose a supported query shape or defer the workflow. Do not treat this as a transient Store failure. |
| `record_invalid` | `record` | `false` | Before admission, fix Record kind, content, support references, or relation inputs and submit again. If status for an already-admitted historical Capture is `semantic_manifest_invalid`, preserve that immutable Capture and start a corrected new Capture; do not retry or rewrite the old payload. Keep provenance links explicit. |
| `record_not_found` | `record` | `false` | Re-check the Record id at the selected branch/head and verify whether it exists in the source Store before importing or recreating it. |
| `relation_invalid` | `relation` | `false` | Fix relation type, endpoint kind/id, or version-scoped endpoint existence. Do not create dangling semantic relations. |
| `resource_invalid` | `resource` | `false` | Correct adapter kind, adapter schema version, scope kind, scope schema version, or scope payload. For local-file and Git scopes, compare against the operator quickstart contract. |
| `resource_not_found` | `resource` | `false` | Re-check the Resource id and branch/head. If the underlying file or repo is missing, record unavailable observation state instead of pretending verification passed. |
| `resource_observation_not_found` | `resource` | `false` | Refresh or record a Resource observation for the current verification head. Use the explicit cache-refresh mode that matches the Resource basis. |
| `replay_invalid` | `replay` | `false` | Stop replay and inspect the source Store, commit lineage, and Bundle/import boundary. Retry only after the replay input is corrected. |
| `replay_unsupported` | `replay` | `false` | Use a supported replay/import path or defer the workflow. Do not rely on partial replay side effects. |
| `session_invalid` | `runtime` | `false` | Correct session actor, focus, lifecycle state, or expected head. If ownership changed, follow the Claim transfer/takeover recovery flow. |
| `session_not_found` | `runtime` | `false` | Re-check the Session id and branch/head. Start a new Session only after confirming the previous one is absent, ended, or unrecoverable. |
| `store_already_initialized` | `store` | `false` | Open the existing Store or choose an empty target path. Do not reinitialize over an existing Store. |
| `store_bootstrap_invalid` | `store` | `false` | Remove only the failed bootstrap target after confirming it is not an authoritative Store, then bootstrap again from clean inputs. |
| `store_compatibility_unsupported` | `store` | `false` | Use a compatible WorkVCS binary or migrate through an explicit supported migration. Do not edit manifest compatibility fields by hand. |
| `storage_failure` | `storage` | `false` | Preserve the Store and inspect filesystem, SQLite, permissions, disk, and path state. Run doctor/integrity only when the Store can be opened safely. |
| `task_invalid` | `task` | `false` | Correct Task content, status, parent path, dependency, AC, or transition input. Use current Task show/history before changing lifecycle state. |
| `task_not_found` | `task` | `false` | Re-check the Task id at the selected branch/head. If the Task moved through Handoff or import, follow the current containment path. |
| `time_invalid` | `time` | `false` | Correct timestamp or ordering input. Use explicit timestamps when reconstructing historical state. |
| `workspace_invalid` | `workspace` | `false` | Correct Workspace metadata or bootstrap/open inputs. Do not mutate branch state until the Workspace contract is valid. |
| `workspace_not_found` | `workspace` | `false` | Re-check the Workspace id and Store. Bootstrap or import a Workspace only if that is the intended authority path. |
| `cli_parse_error` | `usage` | `false` | Inspect `clap_error_kind`, then rerun the relevant `workvcs <subcommand> --help`. Fix stale flags, missing values, or removed subcommands before retrying. |

## Recovery Boundaries

- A non-retryable code can still be resolved, but not by blind replay.
- `mutation_postcondition_failed` is not a failed write. Preserve its rendered
  result and reconcile current state before choosing a follow-up.
- `branch_head_conflict` recovery must refresh and re-evaluate intent before
  retrying.
- Resource unavailable/error states should be recorded explicitly when they are
  the truth at the evaluated head.
- Store integrity and compatibility failures are authority problems. Preserve
  evidence first; repair or migrate only through explicit supported paths.
- JSON output is opt-in. Existing scripts that do not pass
  `--error-format json` should continue parsing the default key-value stderr
  form line by line.
