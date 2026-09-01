# WorkVCS Error Recovery Guide

Status: Phase 4MO current V1-local operator guidance.

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

## Per-Code Recovery

| error_code | category | retryable | Recovery action |
| --- | --- | --- | --- |
| `canonical_encoding_invalid` | `canonical` | `false` | Stop using the malformed canonical payload as authority. Regenerate it through the WorkVCS CLI or schema-backed producer, then rerun the operation with the corrected payload. |
| `digest_invalid` | `canonical` | `false` | Recompute the digest from the current canonical bytes. Do not copy a digest from another object or bypass digest checks. |
| `evidence_invalid` | `evidence` | `false` | Fix the Evidence content, kind, or referenced entities before retrying. Preserve the failed payload for audit if it came from an external artifact. |
| `evidence_not_found` | `evidence` | `false` | Confirm the Evidence id at the selected branch/head and use `evidence show` or adjacent list commands to recover the correct id. Do not recreate Evidence until the missing selector is understood. |
| `identity_invalid` | `identity` | `false` | Correct the identity id, actor, or metadata shape. This is an input-contract failure, not a concurrency retry. |
| `immutable_import_invalid` | `import` | `false` | Rebuild or re-export the import source and rerun validation. Do not partially apply an import that failed fixed-point checks. |
| `integrity_invalid` | `integrity` | `false` | Stop treating the Store as authoritative. Run `workvcs doctor "$STORE" --require-valid` and `workvcs store integrity "$STORE" --require-valid`, then repair from a known-good Store or Bundle. |
| `knowledge_invalid` | `knowledge` | `false` | Fix the Knowledge payload, kind, or support references. Avoid recording semantic knowledge that is not backed by the current evidence chain. |
| `knowledge_not_found` | `knowledge` | `false` | Re-check the Knowledge id at the selected branch/head. Use current list/show output before deciding whether new Knowledge should be created. |
| `commit_not_found` | `replay` | `false` | Confirm the commit id belongs to this Store lineage and selected branch. If it came from a Bundle or copied Store, validate the import/export boundary before retrying. |
| `branch_head_conflict` | `mutation` | `true` | Refresh the branch head, inspect the intervening history, then rerun only if the mutation still represents the operator intent against the new head. Never force a stale expected head. |
| `branch_not_found` | `mutation` | `false` | Check the branch id and Store. Create or import the branch only if that is the intended state transition. |
| `claim_invalid` | `runtime` | `false` | Fix Claim mode, actor/session binding, expected head, or transition inputs. Use `claim guard` when the failure concerns protected mutation readiness. |
| `claim_not_found` | `runtime` | `false` | Re-check the Claim id and current branch head. If another operator released or transferred it, follow the handoff/claim recovery flow before mutating. |
| `entity_not_found` | `mutation` | `false` | Confirm the entity id, kind, and selected commit. Entity existence is version-scoped, so compare against the intended branch head before recreating anything. |
| `entity_transition_invalid` | `mutation` | `false` | Inspect the current entity status and allowed lifecycle transition. Choose the next valid transition or record a recovery Task instead of forcing the state. |
| `goal_invalid` | `goal` | `false` | Correct Goal content, status, containment, or transition input. If the issue is stale branch state, first refresh the selected head. |
| `goal_not_found` | `goal` | `false` | Re-check the Goal id at the selected branch/head. Use current Goal list/show output before creating a replacement. |
| `plan_invalid` | `plan` | `false` | Correct Plan content, status, containment, or transition input. Keep Plan changes aligned with the current Goal and Task graph. |
| `plan_not_found` | `plan` | `false` | Re-check the Plan id at the selected branch/head. If the Plan was superseded, use the current containment path. |
| `query_invalid` | `query` | `false` | Fix selector syntax, required ids, filter values, or mutually exclusive arguments. Use the relevant subcommand help before rerunning. |
| `query_unsupported` | `query` | `false` | Choose a supported query shape or defer the workflow. Do not treat this as a transient Store failure. |
| `record_invalid` | `record` | `false` | Fix Record kind, content, support references, or relation inputs. Keep provenance links explicit. |
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
- `branch_head_conflict` recovery must refresh and re-evaluate intent before
  retrying.
- Resource unavailable/error states should be recorded explicitly when they are
  the truth at the evaluated head.
- Store integrity and compatibility failures are authority problems. Preserve
  evidence first; repair or migrate only through explicit supported paths.
- JSON output is opt-in. Existing scripts that do not pass
  `--error-format json` should continue parsing the default key-value stderr
  form line by line.
