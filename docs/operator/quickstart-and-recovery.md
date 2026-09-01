# Local Operator Quickstart and Recovery

Status: Phase 4LC local V0.1 operator guide
Last updated: 2026-09-01

This guide is for a local operator or Agent using the current WorkVCS CLI from
this repository. It describes runnable local commands, not a packaged release.

## Current Boundary

WorkVCS currently runs as a local Rust CLI over a local SQLite Store. The Store
is the durable WorkVCS state; Git branches, Codex worktrees, remote pushes,
deployment, and plugin activation remain separate workflows.

Current V0.1 does not include a daemon, GUI, TUI, cloud sync, remote
collaboration, automatic transcript parsing, LLM extraction, or Agent
orchestration. A Session can be explicitly marked `potentially_stale`; Claim
takeover is stale-gated, explicit, and still requires `--force` plus a
rationale. Automatic stale detection remains Open.

## Build Or Install

From the repository root:

```bash
cargo test --workspace --quiet
cargo install --path crates/workvcs-cli --locked
workvcs --help
```

For one-off local use without installing:

```bash
cargo run -q -p workvcs-cli -- --help
```

The smoke script uses the one-off form internally, so it can validate the
workspace even when no `workvcs` binary has been installed.

## Baseline Validation

Run the schema validator and repository smoke before trusting a new local build:

```bash
scripts/validate-schema-v0.1.sh
scripts/smoke-v0.1-cli-workflow.sh
```

Run integrity checks against any Store before and after risky local operations:

```bash
workvcs doctor "$STORE" --require-valid
workvcs store integrity "$STORE" --require-valid
```

If a local Store was created before Phase 4LS and ordinary open reports that
the frozen schema object set is missing only context packet snapshot objects,
run the explicit additive migration:

```bash
workvcs store migrate-context-packet-snapshot "$STORE"
```

The command is intentionally narrow. It only adds the context packet snapshot
table and indexes for a recognized pre-4LS Store and records migration
provenance. Other schema drift remains a stop condition.

## Minimal Work Loop

Create a Store and Workspace:

```bash
STORE=.workvcs/local.sqlite
workvcs init "$STORE" --display-name local-work
workvcs workspace create "$STORE" --display-name local-workspace
```

The CLI prints `key=value` lines. Capture the emitted `workspace_id`,
`branch_id`, and `genesis_commit_id` for subsequent commands.

Create work, start a Session, and claim it:

```bash
workvcs task create "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --description "Implement the next bounded slice"
```

Capture the emitted `task_entity_id`, `task_entity_version_id`, and `commit_id`.
Use that `commit_id` as the next `HEAD_COMMIT_ID`.

```bash

workvcs session start "$STORE" \
  --workspace "$WORKSPACE_ID" \
  --branch "$BRANCH_ID"

workvcs claim task "$STORE" \
  --session "$SESSION_ID" \
  --task "$TASK_ENTITY_ID"
```

To claim the next runnable Task and immediately receive bounded post-claim
context:

```bash
workvcs claim next "$STORE" \
  --session "$SESSION_ID" \
  --context-profile normal \
  --context-budget-items 20 \
  --context-scope-path crates/workvcs-core/src/runtime/context.rs
```

When context options are present, packet fields are emitted with
`claim_next_` prefixes, for example `claim_next_context_profile` and
`claim_next_context_item.0.category`. Use `--context-scope-path` or
`--context-scope-path-prefix` with normal or full packets when the current work
has an explicit file or resource path; these shorthands build the same scope
objects as `--context-scope-json`. They filter path-scoped Knowledge while
leaving global and non-path-scoped Knowledge visible. Brief packets echo the
scope but do not include
`scoped_knowledge` items. If the selected Task is contained by a Goal or Plan,
brief packets include a `goal_plan_path` item that summarizes the current
hierarchy. If the selected Task has Acceptance Criteria or Verification
Requirements, brief packets include `acceptance_criterion` and
`verification_requirement` context items whose subjects can be reused with
`workvcs verify`. When a Task is blocked by an unsatisfied dependency, brief
packets also include `blocked_dependency` items that name the blocking Task and
summarize its current status. Failed Attempts appear as `failed_attempt` items
in brief packets; normal packets also include running, succeeded, and
inconclusive `attempt` items. Attempt summaries expose status, terminality,
current Record version and digest, canonical scope, and nearby Record relation
counts.

Inspect continuation context:

```bash
workvcs context "$STORE" --session "$SESSION_ID"
workvcs context "$STORE" --session "$SESSION_ID" --profile normal --budget-items 20
workvcs context "$STORE" --session "$SESSION_ID" \
  --scope-path crates/workvcs-core/src/runtime/context.rs
workvcs context "$STORE" --session "$SESSION_ID" \
  --scope-json '{"path":"crates/workvcs-core/src/runtime/context.rs"}'
workvcs next "$STORE" --session "$SESSION_ID"
```

Persist the exact packet used for continuation when an Agent handoff or review
needs durable context evidence:

```bash
workvcs context-packet save "$STORE" \
  --session "$SESSION_ID" \
  --profile normal \
  --budget-items 20 \
  --scope-path-prefix crates/workvcs-core/src/runtime
```

Capture the emitted `context_packet_id` and `packet_digest`. The snapshot is
append-only provenance; it does not move a Branch head, create a WorkState
commit, create an Event, or create a Claim. `show` and `list` load paths verify
the packet digest and reject metadata that no longer agrees with the canonical
packet JSON. `--scope-path` and `--scope-path-prefix` normalize common lexical
variants such as repeated separators, `.`, `..`, and trailing separators for
matching; they do not resolve symlinks, check file existence, expand globs, or
ask an adapter to observe the Resource. Relative paths stay relative; pass an
absolute path when cross-working-directory scope identity is required.

```bash
workvcs context-packet show "$STORE" \
  --packet "$CONTEXT_PACKET_ID"

workvcs context-packet list "$STORE" \
  --session "$SESSION_ID"
```

Record acceptance and verification when a slice has an explicit check:

```bash
workvcs ac create "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --task "$TASK_ENTITY_ID" \
  --task-version "$TASK_ENTITY_VERSION_ID" \
  --local-key ac-validation \
  --statement "The bounded slice passes its validation command"
```

Capture the emitted `acceptance_criterion_entity_id`,
`acceptance_criterion_entity_version_id`, and `commit_id`. Use that `commit_id`
as the next `HEAD_COMMIT_ID`.

```bash
workvcs vr create "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --criterion "$AC_ENTITY_ID" \
  --criterion-version "$AC_ENTITY_VERSION_ID" \
  --local-key vr-validation \
  --statement "Run the validation command and record its result"
```

Capture the emitted `verification_requirement_entity_id` and `commit_id`. Use
that `commit_id` as the next `HEAD_COMMIT_ID`.

```bash
workvcs verify "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --verification-requirement "$VR_ENTITY_ID" \
  --result passed \
  --method cli \
  --evidence-kind command_output \
  --evidence-content-role log \
  --evidence-content "validation command passed"
```

For Resource-backed evidence scoped to a local path, prefer the shorthand:

```bash
workvcs verify "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --verification-requirement "$VR_ENTITY_ID" \
  --result passed \
  --method cli \
  --evidence-kind command_output \
  --evidence-content-role log \
  --evidence-content "validation command passed" \
  --resource "$RESOURCE_ID" \
  --adapter-kind local-file \
  --adapter-schema-version 1 \
  --scope-path crates/workvcs-core/src/runtime/context.rs \
  --resource-content-from-scope-path
```

For Git worktree-backed evidence, use the Git shorthand:

```bash
workvcs verify "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --verification-requirement "$VR_ENTITY_ID" \
  --result passed \
  --method cli \
  --evidence-kind command_output \
  --evidence-content-role log \
  --evidence-content "validation command passed" \
  --resource "$RESOURCE_ID" \
  --adapter-kind git \
  --adapter-schema-version 1 \
  --scope-git-worktree "$REPO_PATH" \
  --resource-content-from-scope-git-worktree
```

`verify --scope-path`, `verify --scope-path-prefix`, and `verify --scope-glob`
default to `scope_kind=path` and `scope_schema_version=1`.
`verify --scope-git-worktree` defaults to `scope_kind=git-worktree` and
`scope_schema_version=1`. Keep using `--scope-payload-json` with explicit
`--scope-kind` and `--scope-schema-version` for advanced payloads. The opt-in
`--resource-content-from-scope-path` reads the file named by `--scope-path` and
uses its content digest as the ResourceObservation fingerprint. For a local-file
directory prefix, use `--scope-path-prefix` with
`--resource-content-from-scope-path-prefix`; it fingerprints a deterministic
manifest of regular files under the prefix. For a local-file glob, use
`--scope-glob` with `--resource-content-from-scope-glob`; it fingerprints a
deterministic manifest of regular matched files under the glob's fixed root. For
a Git repository worktree, use `--scope-git-worktree` with
`--resource-content-from-scope-git-worktree`; it fingerprints a deterministic
manifest of HEAD, index listing, porcelain status, staged diff, unstaged diff,
and untracked regular-file content fingerprints. Use the older
`--resource-fingerprint`, `--resource-content`, or `--resource-content-file`
inputs when the observed content is not exactly one of these scoped manifests.

Capture the emitted `verification_entity_id` and `commit_id`. Use that
`commit_id` as the next `HEAD_COMMIT_ID`. Before marking the Task done, inspect
the current Task version at the new head:

```bash
workvcs task show "$STORE" \
  --branch "$BRANCH_ID" \
  --task "$TASK_ENTITY_ID"
```

Capture the current `task_entity_version_id`, then close the Task:

```bash
workvcs task transition "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --task "$TASK_ENTITY_ID" \
  --task-version "$CURRENT_TASK_ENTITY_VERSION_ID" \
  --status done \
  --outcome completed \
  --session "$SESSION_ID"
```

End a Session with a summary and create a focused handoff:

```bash
workvcs session end "$STORE" \
  --session "$SESSION_ID" \
  --summary-json '{"outcome":"completed bounded slice"}'
```

Capture the emitted `session_diff_id`.

```bash
workvcs handoff create "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --session "$SESSION_ID" \
  --session-diff "$SESSION_DIFF_ID" \
  --focus "$TASK_ENTITY_ID" \
  --statement "Continue from this task and inspect context before claiming work"
```

Capture the emitted `record_entity_id` as `HANDOFF_RECORD_ID` and the emitted
`commit_id` as `HANDOFF_COMMIT_ID`.

Show the handoff before using it:

```bash
workvcs handoff show "$STORE" \
  --commit "$HANDOFF_COMMIT_ID" \
  --handoff "$HANDOFF_RECORD_ID"
```

Review the Handoff focus link from either endpoint:

```bash
workvcs why "$STORE" \
  --commit "$HANDOFF_COMMIT_ID" \
  --entity "$HANDOFF_RECORD_ID"

workvcs why "$STORE" \
  --commit "$HANDOFF_COMMIT_ID" \
  --entity "$TASK_ENTITY_ID"
```

For a focused Handoff, the Handoff Record reports
`scope_link.0.direction=outgoing` and the focused Task reports
`scope_link.0.direction=incoming`. Stored `relation_edges` remain separate from
these read-only scope links; existing relation filters and limits apply to
`relation_edges`, not to scope links.

When querying a Record or other Entity that was used as a causal anchor for a
ChangeSet reachable through first-parent history, `why` reports
`deferred_relation_family.0=evolution`. This means WorkVCS can see that the
Entity participates in an evolution explanation family, but full ChangeSet
evolution traversal is still not implemented.

Start a continuation Session, then consume the Handoff into that Session's
focus:

```bash
workvcs session start "$STORE" \
  --workspace "$WORKSPACE_ID" \
  --branch "$BRANCH_ID"
```

Capture the emitted `session_id` as `CONTINUATION_SESSION_ID`.

```bash
workvcs handoff consume "$STORE" \
  --commit "$HANDOFF_COMMIT_ID" \
  --handoff "$HANDOFF_RECORD_ID" \
  --session "$CONTINUATION_SESSION_ID"
```

Inspect Context and `next` after consuming the Handoff. The Session focus should
match the Handoff focus before work is claimed.

## Bundle Portability And Restore

For local copied-target portability, create and validate a Checkpoint for the
source commit before exporting the Bundle directory:

```bash
workvcs checkpoint create "$SOURCE_STORE" \
  --commit "$EXPORT_HEAD_COMMIT_ID"

workvcs checkpoint validate "$SOURCE_STORE" \
  --checkpoint "$CHECKPOINT_ID" \
  --require-valid

workvcs bundle export-dir "$SOURCE_STORE" \
  --commit "$EXPORT_HEAD_COMMIT_ID" \
  --output-dir "$BUNDLE_DIR"

workvcs bundle validate-dir "$SOURCE_STORE" \
  --commit "$EXPORT_HEAD_COMMIT_ID" \
  --input-dir "$BUNDLE_DIR" \
  --require-valid
```

The current V1-local directory profile emits `manifest.json`,
`payload-index.json`, and content-addressed payload files. The payload index
profile is `workvcs-local-payload-index-v1` with version `1`; packaged archive
or exchange containers remain Open.

Before applying to a target Store, preflight and require that the target can
apply the Bundle:

```bash
workvcs bundle preflight-dir "$TARGET_STORE" \
  --input-dir "$BUNDLE_DIR" \
  --require-valid \
  --require-can-apply

workvcs bundle apply-dir "$TARGET_STORE" \
  --input-dir "$BUNDLE_DIR" \
  --require-applied
```

After apply, validate the imported Checkpoint and inspect the target Branch
head:

```bash
workvcs branch head "$TARGET_STORE" \
  --branch "$BRANCH_ID"

workvcs checkpoint latest "$TARGET_STORE" \
  --commit "$EXPORT_HEAD_COMMIT_ID" \
  --require-found

workvcs checkpoint show "$TARGET_STORE" \
  --checkpoint "$CHECKPOINT_ID"

workvcs checkpoint validate "$TARGET_STORE" \
  --checkpoint "$CHECKPOINT_ID" \
  --require-valid
```

If target-local work must be rolled back to the imported Bundle head, restore
the Branch and inspect the restored Work State:

```bash
workvcs restore "$TARGET_STORE" \
  --branch "$BRANCH_ID" \
  --head "$CURRENT_HEAD_COMMIT_ID" \
  --target-commit "$EXPORT_HEAD_COMMIT_ID" \
  --rationale-json '{"reason":"return to imported bundle head"}'

workvcs show-at "$TARGET_STORE" \
  --commit "$RESTORE_COMMIT_ID"
```

`checkpoint latest` is a commit-anchored selector. Query it against the commit
that owns the Checkpoint candidate, such as the imported Bundle head. Do not
use it as a state-digest lookup for a later restore commit that happens to have
the same Work State.

For a representative opt-in local portability validation beyond the default
smoke Store, run:

```bash
WORKVCS_LARGER_STORE_OUTPUT_ROOT=.work-governance/runtime/logs/phase-4lq \
  ./scripts/larger-store-portability-v0.1.sh
```

The default workload creates 48 Tasks, copies the target Store after 24 Tasks,
then adds AC/VR/Verification records, scheduling relations, Checkpoint, Bundle
export/validate/preflight/apply, target restore, integrity, and doctor checks.
Use `WORKVCS_LARGER_STORE_TASKS`,
`WORKVCS_LARGER_STORE_BASELINE_TASKS`,
`WORKVCS_LARGER_STORE_VERIFICATIONS`, and
`WORKVCS_LARGER_STORE_RELATION_PAIRS` to scale the run. Keep it opt-in until a
larger default smoke matrix is explicitly justified.

## Common Recovery Actions

When a WorkVCS business command fails, read stderr as line-oriented key-value
metadata before choosing the recovery path:

```text
error_code=<CODE>
error_category=<CATEGORY>
retryable=<true|false>
message=<ESCAPED_MESSAGE>
```

Use `retryable=true` as a signal to refresh current state and retry the
operation only after confirming the relevant branch, session, claim, or merge
head. `message` remains human-facing context; recovery scripts should branch on
`error_code` and `error_category`.

For code-specific operator actions, use the current
[WorkVCS Error Recovery Guide](error-recovery-guide.md). It covers every
current WorkVCS business `error_code` and the top-level `cli_parse_error`
syntax failure shape.

When a Store fails integrity or doctor checks, stop using it as an authority
until the failure is understood:

```bash
workvcs doctor "$STORE" --require-valid
workvcs store integrity "$STORE" --require-valid
```

When verification applicability is stale after a branch head advances, refresh
the cache for the new head instead of manually editing observations:

```bash
workvcs verification cache-refresh "$STORE" \
  --branch "$BRANCH_ID" \
  --verification "$VERIFICATION_ID"
```

For an exact local-file path Resource basis, use the explicit adapter-backed
refresh mode to re-read the current file and record a new ResourceObservation:

```bash
workvcs verification cache-refresh "$STORE" \
  --branch "$BRANCH_ID" \
  --verification "$VERIFICATION_ID" \
  --resource-content-from-scope-path \
  --expected-evaluated-commit "$HEAD_COMMIT_ID"
```

This mode supports only `adapter_kind=local-file`, `scope_kind=path`,
`scope_schema_version=1`, and `scope_payload={"path":"..."}`. If the file is
unchanged, the cache remains `applicability=applicable`. If content changed, it
becomes `reason_code=resource_drift`. A missing file becomes
`reason_code=resource_unavailable`; a failed read becomes
`reason_code=resource_error`. Relative stored paths are read relative to the
current working directory of the command.

For a local-file path-prefix Resource basis, use the explicit path-prefix
refresh mode:

```bash
workvcs verification cache-refresh "$STORE" \
  --branch "$BRANCH_ID" \
  --verification "$VERIFICATION_ID" \
  --resource-content-from-scope-path-prefix \
  --expected-evaluated-commit "$HEAD_COMMIT_ID"
```

This mode supports only `adapter_kind=local-file`, `scope_kind=path`,
`scope_schema_version=1`, and `scope_payload={"path_prefix":"..."}`. It
recursively fingerprints regular files under the prefix using the
`local-file-path-prefix-manifest-v1` profile. Unchanged content remains
`applicability=applicable`; changed files become `reason_code=resource_drift`;
a missing prefix becomes `reason_code=resource_unavailable`; a non-directory,
symlink, special file, or traversal/read failure becomes
`reason_code=resource_error`.

For a local-file glob Resource basis, use the explicit glob refresh mode:

```bash
workvcs verification cache-refresh "$STORE" \
  --branch "$BRANCH_ID" \
  --verification "$VERIFICATION_ID" \
  --resource-content-from-scope-glob \
  --expected-evaluated-commit "$HEAD_COMMIT_ID"
```

This mode supports only `adapter_kind=local-file`, `scope_kind=path`,
`scope_schema_version=1`, and `scope_payload={"glob":"..."}`. The glob must
have a fixed non-wildcard root before the first wildcard segment; absolute
filesystem-root scans, patterns such as `*.md` or `**/*.rs`, and fixed roots
that retain parent-directory traversal are rejected. It fingerprints sorted
regular-file matches using the `local-file-glob-manifest-v1` profile. Unchanged
matches remain
`applicability=applicable`; changed matched files, added matches, removed
matches, or an existing root with no matches become
`reason_code=resource_drift`; a missing fixed root becomes
`reason_code=resource_unavailable`; matched directories, symlinks, special
files, pattern errors, or traversal/read failures become
`reason_code=resource_error`.

For a Git worktree Resource basis, use the explicit Git refresh mode:

```bash
workvcs verification cache-refresh "$STORE" \
  --branch "$BRANCH_ID" \
  --verification "$VERIFICATION_ID" \
  --resource-content-from-scope-git-worktree \
  --expected-evaluated-commit "$HEAD_COMMIT_ID"
```

This mode supports only `adapter_kind=git`, `adapter_schema_version=1`,
`scope_kind=git-worktree`, `scope_schema_version=1`, and
`scope_payload={"repo":"..."}`. It invokes `git` with `GIT_OPTIONAL_LOCKS=0`
and fingerprints a deterministic `git-worktree-manifest-v1` manifest. Unchanged
worktree state remains `applicability=applicable`; changed tracked, staged,
unstaged, or untracked state becomes `reason_code=resource_drift`; a missing
repo path becomes `reason_code=resource_unavailable`; a non-Git directory or
Git command failure becomes `reason_code=resource_error`.

When the Verification already has supported Resource basis entries and you want
the CLI to choose the matching implemented adapter path, use basis-aware refresh:

```bash
workvcs verification cache-refresh "$STORE" \
  --branch "$BRANCH_ID" \
  --verification "$VERIFICATION_ID" \
  --resource-content-from-basis \
  --expected-evaluated-commit "$HEAD_COMMIT_ID"
```

This explicit mode supports the current exact local-file path, local-file
path-prefix, local-file glob, and Git worktree contracts. It validates every
Resource basis before observing any Resource. Unsupported or malformed basis
entries fail the command instead of producing a partial refresh.

When a Resource-backed Verification cannot be re-observed because the Resource
is temporarily unavailable, record that state explicitly and keep the AC stale:

```bash
workvcs verification cache-record "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --verification "$VERIFICATION_ID" \
  --adapter-kind "$ADAPTER_KIND" \
  --adapter-schema-version "$ADAPTER_SCHEMA_VERSION" \
  --scope-schema-version "$SCOPE_SCHEMA_VERSION" \
  --observation-status unavailable \
  --expected-applicability unknown \
  --expected-reason-code resource_unavailable

workvcs ac status "$STORE" \
  --branch "$BRANCH_ID" \
  --criterion "$AC_ENTITY_ID"
```

When the adapter attempted re-observation but failed, use `error` instead:

```bash
workvcs verification cache-record "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --verification "$VERIFICATION_ID" \
  --adapter-kind "$ADAPTER_KIND" \
  --adapter-schema-version "$ADAPTER_SCHEMA_VERSION" \
  --scope-schema-version "$SCOPE_SCHEMA_VERSION" \
  --observation-status error \
  --expected-applicability unknown \
  --expected-reason-code resource_error
```

Do not pass `--observed-fingerprint` or `--observation` with `unavailable` or
`error` stamps. Those states intentionally mean the current content was not
observed. `workvcs verification cache-show` should report
`observed_fingerprint=none` and `observation_id=none`; the related AC remains
`status=stale` until an applicable observation is recorded or refreshed.

When a merge cannot be frozen, inspect every merge item and resolve each item
explicitly before freezing. This includes `AUTO` items:

```bash
workvcs merge show "$STORE" \
  --merge "$MERGE_ID"

workvcs merge resolve "$STORE" \
  --item "$MERGE_ITEM_ID" \
  --kind theirs \
  --session "$SESSION_ID" \
  --rationale-json '{"reason":"accept source item"}'

workvcs merge freeze "$STORE" \
  --merge "$MERGE_ID"
```

When `merge continue` reports that the target or source Branch moved, do not
try to force the stale attempt forward. Abort the stale attempt, start a new
merge from the current Branch heads, then resolve/freeze/continue the new
attempt. Repeat `merge resolve` for every item shown by `merge show`:

```bash
workvcs merge abort "$STORE" \
  --merge "$STALE_MERGE_ID" \
  --session "$SESSION_ID" \
  --detail-json '{"reason":"restart after branch head moved"}'

workvcs merge start "$STORE" \
  --target-branch "$TARGET_BRANCH_ID" \
  --source-branch "$SOURCE_BRANCH_ID" \
  --session "$SESSION_ID"

workvcs merge show "$STORE" \
  --merge "$NEW_MERGE_ID"

workvcs merge resolve "$STORE" \
  --item "$NEW_MERGE_ITEM_ID" \
  --kind theirs \
  --session "$SESSION_ID" \
  --rationale-json '{"reason":"accept item after restart"}'

workvcs merge freeze "$STORE" \
  --merge "$NEW_MERGE_ID"

workvcs merge continue "$STORE" \
  --merge "$NEW_MERGE_ID" \
  --session "$SESSION_ID"
```

When `bundle preflight-dir --require-can-apply` reports
`same_store_divergence_detected`, do not force the Bundle onto the target
Branch. Inspect the reported source head, target head, and merge base, then
resolve the divergence through the merge workflow or export a new Bundle from a
compatible head. `bundle apply-dir` without `--require-applied` records the
non-applied outcome and leaves the target Branch head unchanged; with
`--require-applied`, it fails.

When an active Claim blocks another active Session and the claimant can hand
work over, transfer the Claim. The receiving Session should inspect the guard
before doing terminal work:

```bash
workvcs claim transfer "$STORE" \
  --from-session "$SOURCE_SESSION_ID" \
  --to-session "$TARGET_SESSION_ID" \
  --claim "$CLAIM_ID"

workvcs claim guard "$STORE" \
  --session "$TARGET_SESSION_ID" \
  --task "$TASK_ENTITY_ID"
```

When two operators intentionally coordinate on the same Task, use shared Claims
and inspect `context` before any protected mutation:

```bash
workvcs claim task "$STORE" \
  --session "$FIRST_SESSION_ID" \
  --task "$TASK_ENTITY_ID" \
  --mode shared

workvcs claim next "$STORE" \
  --session "$SECOND_SESSION_ID" \
  --mode shared

workvcs context "$STORE" \
  --session "$SECOND_SESSION_ID"

workvcs claim guard "$STORE" \
  --session "$FIRST_SESSION_ID" \
  --task "$TASK_ENTITY_ID" \
  --action structural-task
```

If the guard reports `reason=non_unique_shared_claim_set`, protected mutation
is intentionally blocked. Release or transfer Claims until one responsible
Session remains, then inspect the guard again before terminal work:

```bash
workvcs claim release "$STORE" \
  --session "$SECOND_SESSION_ID" \
  --claim "$SECOND_SHARED_CLAIM_ID"

workvcs claim guard "$STORE" \
  --session "$FIRST_SESSION_ID" \
  --task "$TASK_ENTITY_ID" \
  --action terminal-task
```

For version-scoped ACs, capture or cite the verification commit when the AC is
`verified`. A later Task closeout commit can advance the Task version and make
that same AC project as `stale`.

If a focused Handoff continuation is blocked by another active Session's Claim,
first consume the Handoff, inspect the guard, then recover with the same
stale-gated takeover chain:

```bash
workvcs handoff consume "$STORE" \
  --commit "$HANDOFF_COMMIT_ID" \
  --handoff "$HANDOFF_RECORD_ID" \
  --session "$CONTINUATION_SESSION_ID"

workvcs claim guard "$STORE" \
  --session "$CONTINUATION_SESSION_ID" \
  --task "$TASK_ENTITY_ID"
```

For the `exclusive_claim_owned_by_other_session` case, `claim guard` emits the
stable top-level fields needed by the next explicit recovery steps:

```text
stale_takeover_available=true
stale_takeover_claim_id=<CLAIM_ID>
stale_takeover_previous_session_id=<PREVIOUS_OWNER_SESSION_ID>
stale_takeover_required_previous_session_lifecycle_state=potentially_stale
```

These fields are hints only. They do not mark the previous Session stale and do
not perform takeover.

```bash

workvcs session mark-stale "$STORE" \
  --session "$PREVIOUS_OWNER_SESSION_ID" \
  --rationale "previous owner cannot continue"

workvcs claim takeover "$STORE" \
  --session "$CONTINUATION_SESSION_ID" \
  --claim "$CLAIM_ID" \
  --force \
  --rationale "handoff recovery"
```

When an operator must override a blocked active Claim, first mark the previous
owning Session as `potentially_stale`, then use forced takeover with a rationale
and inspect the guard afterward:

```bash
workvcs session mark-stale "$STORE" \
  --session "$PREVIOUS_SESSION_ID" \
  --rationale "operator recovery: previous session cannot continue"

workvcs claim takeover "$STORE" \
  --session "$TAKING_SESSION_ID" \
  --claim "$CLAIM_ID" \
  --force \
  --rationale "operator recovery: previous session cannot continue"

workvcs claim guard "$STORE" \
  --session "$TAKING_SESSION_ID" \
  --task "$TASK_ENTITY_ID"
```

When a Session is ended, start a new Session rather than attempting to mutate
the ended runtime row or resolve context through it:

```bash
workvcs session start "$STORE" \
  --workspace "$WORKSPACE_ID" \
  --branch "$BRANCH_ID"
```

When the repository smoke fails, keep the temporary Store directory reported by
the script, rerun the failing command with `workvcs ... --help` open for that
subcommand, and only update expectations after the command output proves the
intended state transition.

Top-level syntax failures use stable key-value stderr. Branch on
`error_code=cli_parse_error` and `clap_error_kind` for stale flags or removed
subcommands; do not parse the human `message` except for display:

```text
error_code=cli_parse_error
error_category=usage
retryable=false
clap_error_kind=unknown_argument
message=error: unexpected argument ...
```

## Still Open For V1

- The documented loop has been repeated once against another real local
  project in read-only mode. It is not yet broad write-mode or multi-project
  maturity evidence.
- Explicit path-scope lexical normalization and opt-in local-file observation
  from `verify --scope-path` and `verify --scope-path-prefix` are implemented
  for the common CLI shorthands.
  Exact local-file path cache refresh is implemented behind
  `verification cache-refresh --resource-content-from-scope-path`; local-file
  path-prefix cache refresh is implemented behind
  `verification cache-refresh --resource-content-from-scope-path-prefix`.
  Local-file glob cache refresh is implemented behind
  `verification cache-refresh --resource-content-from-scope-glob`.
  Git worktree cache refresh is implemented behind
  `verification cache-refresh --resource-content-from-scope-git-worktree`.
  Basis-aware cache refresh is implemented behind
  `verification cache-refresh --resource-content-from-basis`.
  Explicit unavailable/error applicability stamps are dogfood-proven. Broader
  symlink/case/rename, broader Git adapter policy, and background
  re-observation scheduling remain open.
- Context packet persistence and transition-rationale projection are
  implemented; broader Context Resolver dogfood remains open.
- `why` exposes focused Handoff scope links and anchored evolution as a deferred
  family, but full evolution traversal and epistemic explanation remain open.
- Shared-Claim collaboration has read-only real-project dogfood evidence.
  Broader write-mode or read/write multi-operator maturity remains open.
- Automatic stale detection remains open.
- Merge lifecycle is locally dogfood-proven, but not yet another-project or
  larger-Store proven.
- Bundle portability is locally dogfood-proven for copied-target same-Store
  operation. The V1-local directory profile is defined, but packaged archives,
  exchange APIs, and external Store canonical DAG activation remain open.
- Larger Store validation has one bounded local run; broader and more varied
  performance evidence remains open.
- WorkVCS business errors and top-level clap syntax errors now emit stable
  key-value fields, and current per-code recovery guidance is documented in the
  operator error recovery guide. JSON error output remains open.
