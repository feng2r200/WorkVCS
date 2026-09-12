# Local Operator Quickstart and Recovery

Status: Phase 4LC local V0.1 operator guide
Last updated: 2026-09-08

This guide is for a local operator or Agent using the current WorkVCS CLI from
this repository. It describes runnable local commands and the local
package/install helper, not a public release.

## P0 Cutover Entry

The implemented P0 entrypoint binds and discovers a project from the working
directory, exposes read-only `resume --cwd`, and provides atomic/idempotent
`workvcs plan admit` plus `plan evolve mode=in_place` and `mode=supersede`.
Use `--registry PATH` to select the external registry explicitly. Otherwise
WorkVCS checks `WORKVCS_HOME`, then
`$XDG_CONFIG_HOME/workvcs/config.toml` (or
`$HOME/.config/workvcs/config.toml`). Run `workvcs config show` to inspect the
effective locator and `workvcs project list --require-valid` to audit every
binding. The registry and Store are forced outside the project/repository. Git
identity is derived from the repository common directory, and the discovered
Store receives complete integrity validation before use.

Useful discovery from before the first admission is retained. If active
Session selection is zero, multiple, or otherwise ambiguous, stop and recover
explicitly; the entrypoint fails closed. `workvcs plan admit` takes a required
manifest and exactly one target: `STORE` plus `--branch`, or `--cwd` plus its
bound branch. `--registry` is valid with `--cwd`. Expected head/state and
idempotency are manifest fields, not CLI flags. The manifest may carry prior
findings, decisions, questions, constraints, and evidence; the admission is
one atomic transition and same-key replay is idempotent.

`workvcs plan evolve` requires a manifest with `mode=in_place` or
`mode=supersede`. In-place mode updates only explicitly supplied Plan fields,
preserves omitted fields, and atomically
appends Tasks with AC/VR, Records, and Evidence without implicit deletion or
replacement. Expected guards, target Plan identity/version/digest, and
idempotency are manifest fields. `mode=supersede` is current: it transitions
old active→superseded and creates a new active Plan under the same Goal,
retaining the old `contains`, adding the new `contains`, and creating the
`new_plan→old_plan` `supersedes` relation. Constraints require explicit
`carry_all` or `replace`; old Tasks, Records, and Evidence are not migrated.
`workvcs receipt issue`, `receipt show`, `receipt list`, and `receipt consume`
are current P0-3a/P0-3b commands. They expose only mechanical binding plus
authority-ref type/digest and a redacted marker. The structured
`authority_ref.ref` input is automatically redacted and is not persisted or
emitted in scope, payload, CLI/show/list, or debug output; this is not a
full-manifest secret scan. Receipt `rationale` is persisted, so callers must
not put credentials, tokens, or other secrets in it. Consume is branch-scoped single-use;
idempotent replay may reuse only a committed `workstate_commit` result, never
an orphan ChangeSet, and no Store-global lock across restore histories is
promised. It is not atomic with an external action. `revoke`, plus receipt
projection into `context`/`why`, remain deferred and are not current commands;
they do not block the current P0 surface.

`workvcs closeout inspect` is current and requires explicit
`--target-kind goal|plan|task` plus `--target`, using either `--cwd PATH` or
`STORE` with `--branch BRANCH`/`--commit COMMIT`. It is fully read-only,
defaults to 50 items with a hard maximum of 200, reports stable ordering and
truncation/omitted counts, expands only the documented direct target scope,
and emits before/after branch/source/target/Store main-WAL-SHM proof. It reports
mechanical state only; it does not conclude authorization, quality, readiness,
completion, push, or deployment.

No-Plan means no Plan is invented. Discovery, audit, recall, and resume remain
read-only, while `workvcs capture` may explicitly persist valuable standalone
Records, Knowledge, Evidence, and relations without a Goal, Plan, Task,
Session, or Claim.

Use `workvcs recall --cwd <path> --profile brief|handoff|retrospective` for a
bounded project projection that does not require an active Session. Use
`workvcs capture --cwd <path> --manifest <file>` for one atomic, idempotent
standalone cognition change. Raw Evidence content supplied by `--content` or
`--content-file` is stored in the local content-addressed object area; inspect
it with `evidence show` and recover it with `evidence extract`.

This cutover does not provide compatibility for `workctl`, schema-v3/v4/v5,
or `.work-governance`. WorkVCS reports mechanical state; it does not decide
authorization policy.

## Current Boundary

WorkVCS currently runs as a local Rust CLI over a local SQLite Store. The Store
is the durable WorkVCS state; Git branches, Codex worktrees, remote pushes,
deployment, and plugin activation remain separate workflows.

Current V0.1 does not include a daemon, GUI, TUI, cloud sync, remote
collaboration, automatic transcript parsing, LLM extraction, or Agent
orchestration. A Session can be explicitly marked `potentially_stale`; Claim
takeover is stale-gated, explicit, and still requires `--force` plus a
rationale. Automatic stale detection remains Open.

## Build, Package, Or Install

From the repository root:

```bash
RUST_MIN_STACK=33554432 cargo test --workspace --quiet
scripts/package-workvcs.sh --install --bin-dir /usr/local/bin
workvcs --help
```

The larger test-thread stack is required by the current monolithic CLI test
binary. Focused tests for the new entrypoints run with the default stack; a
future CLI dispatcher split should remove this full-suite requirement.

For local packaging without installing:

```bash
scripts/package-workvcs.sh
```

The script builds `workvcs`, copies it into a unique package directory under
`target/package/`, writes `manifest.txt` plus a deterministic source-generated
SHA-256 manifest for every regular file in `skills/workvcs`, rejects symlinks or
special entries, creates a `.tar.gz`
archive, and validates both the packaged binary and complete Skill tree.
Package mode is the default and does not write to `/usr/local/bin`.

To preview the system-level install/overwrite action without writing:

```bash
scripts/package-workvcs.sh --dry-run --install --bin-dir /usr/local/bin
```

To validate install/overwrite behavior without touching a system path:

```bash
tmp_bin="$(mktemp -d "${TMPDIR:-/tmp}/workvcs-bin.XXXXXX")"
scripts/package-workvcs.sh --install --bin-dir "$tmp_bin" --profile debug
"$tmp_bin/workvcs" --help
```

Only after a separate explicit authorization to install or overwrite the system
binary:

```bash
scripts/package-workvcs.sh --install --bin-dir /usr/local/bin
workvcs --help
```

If `/usr/local/bin` is not writable, the script uses `sudo` for the directory
creation or final overwrite step. The destination basename must be `workvcs`,
the installed binary digest must match the packaged binary, and the installed
Skill must match the package's full-tree manifest with no missing, modified, or
extra regular files. Unreadable files, unsupported special entries, and paths
containing control characters fail closed. `scripts/workvcs-skill-tree.sh
verify` can independently recheck a Skill directory against a packaged
`skill-tree.sha256`.

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
`workvcs verify`. If a current-head Resource-backed Verification targets one
of those Verification Requirements, the `verification_requirement` summary also
names the Resource basis count, `verification_id`, `resource_id`,
`adapter=<kind>@<version>`, `scope=<kind>@<version>`,
`baseline_observation_id`, and a `refresh_hint` for basis-aware
`verification cache-refresh`. When a Task is blocked by an unsatisfied
dependency, brief packets also include `blocked_dependency` items that name the
blocking Task and summarize its current status. If the blocking Task has
current-head Resource-backed Verification Requirements, the
`blocked_dependency` summary also names `dependency_resource_requirements`,
`dependency_acceptance_criterion`, `dependency_verification_requirement`,
`dependency_vr_local_key`, Resource basis details, `baseline_observation_id`,
and a basis-aware `refresh_hint`. Failed Attempts appear as
`failed_attempt` items in brief packets; normal packets also include running,
succeeded, and inconclusive `attempt` items. Attempt summaries expose status,
terminality, current Record version and digest, canonical scope, and nearby
Record relation counts.

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
`deferred_relation_family.0=evolution`, the anchoring
`causal_anchor_changeset.<i>.*` fields, and the direct
`evolution_change_operation.<i>.*` fields for that ChangeSet. The operation
fields include `subject_family` and `subject_object_id`, so an operator can see
which Entity or Relation the causal anchor changed without running a separate
`changeset operations` command.

When querying an Entity that was directly changed by a first-parent-reachable
ChangeOperation, `why` also reports that direct Entity-subject evolution
operation even if the Entity was not the causal anchor. For example, a
superseded prior Decision can report `causal_anchor_changesets=0` while still
showing the direct `record.decision.supersede` operation that changed it.
For these direct queried-Entity operations, recognized Entity detail is
operation-local when the operation has an after Entity version:
`subject_entity_version_id` identifies the version produced by that
ChangeOperation, and `subject_statement_json` is looked up at that operation's
commit. Initial Entity creation operations are not counted as this direct
evolution slice.

When querying a Task endpoint of a current scheduling relation, `why` reports
the scheduling relation as a normal relation edge. `depends_on` renders as
`task_depends_on`, `ordered_before` renders as `task_ordered_before`, and both
source and target endpoints use `entity_kind=task`. The source endpoint
reports `direction=outgoing`; the target endpoint reports
`direction=incoming`. Direct `task.scheduling_relation.create` operations are
also reported as endpoint evolution for the source or target Task, with
relation subject detail for the relation kind, relation version, Task
endpoints, and state digest.

When querying a Goal, Plan, or Task endpoint of a current primary containment
relation, `why` reports the containment relation as a normal
`primary_containment` relation edge. Direct `primary_containment.create`
operations are also reported as endpoint evolution for the source or target
endpoint, with relation subject detail for the relation kind, relation version,
source endpoint, target endpoint, and state digest.

When querying a Record or Knowledge Entity that is the source or target
endpoint of a first-parent-reachable direct relation create, removal, or
restore, `why` also reports that direct Relation-subject evolution operation
for currently recognized Record-to-Record, Record-to-Knowledge, and
Knowledge-to-Knowledge relation shapes. Removed relations can therefore have
operation-local relation `subject_detail` even when `relation_edges=0` at the
queried commit. Use `--expected-evolution-change-operations` when a script
needs to assert the projected operation count. Full ChangeSet evolution
traversal, full relation-subject traversal beyond these direct
create/remove/restore endpoint slices, and multi-hop evolution traversal are
still not implemented, and relation filters and limits continue to apply only
to `relation_edges`.

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

For repeated maintained-Store portability beyond the one-shot larger Store
run, use the opt-in maintained Store dogfood:

```bash
WORKVCS_MAINTAINED_STORE_OUTPUT_ROOT=.work-governance/runtime/logs/phase-4nd \
  ./scripts/maintained-store-portability-v0.1.sh
```

The default run initializes one source Store, copies one same-Store target
after a seed baseline, then performs three maintenance cycles against the same
source and target Stores. Each cycle reopens the source, adds current V1
semantic state, creates a Checkpoint, exports and validates a Bundle directory,
preflights and applies the Bundle to the target, checks target Branch
head/state-digest convergence, runs source and target integrity plus doctor,
and confirms same-Store copied-target lineage lists zero cross-Store lineage
records.

The script also proves target-local post-apply maintenance by forking a
target-local Branch, adding local work, restoring that Branch to the imported
Bundle head, and checking that the target main Branch remains available for
later fast-forward applies. Use `WORKVCS_MAINTAINED_STORE_CYCLES`,
`WORKVCS_MAINTAINED_STORE_SEED_TASKS`,
`WORKVCS_MAINTAINED_STORE_TASKS_PER_CYCLE`,
`WORKVCS_MAINTAINED_STORE_VERIFICATIONS_PER_CYCLE`, and
`WORKVCS_MAINTAINED_STORE_RELATION_PAIRS_PER_CYCLE` to scale the run. Keep it
opt-in until default smoke expansion is explicitly justified.

The Phase 4NE larger maintained Store validation used the same script with:

```bash
WORKVCS_MAINTAINED_STORE_CYCLES=5 \
WORKVCS_MAINTAINED_STORE_SEED_TASKS=12 \
WORKVCS_MAINTAINED_STORE_TASKS_PER_CYCLE=20 \
WORKVCS_MAINTAINED_STORE_VERIFICATIONS_PER_CYCLE=4 \
WORKVCS_MAINTAINED_STORE_RELATION_PAIRS_PER_CYCLE=8 \
WORKVCS_MAINTAINED_STORE_OUTPUT_ROOT=.work-governance/runtime/logs/phase-4ne \
  ./scripts/maintained-store-portability-v0.1.sh
```

That run produced 112 final Tasks, 20 Verifications, 80 script-counted
scheduling relation versions, five same-target applies, 647 final Bundle
payload files, 1,817 final payload references, source/target head and digest
convergence, and required-valid integrity/doctor proof in 331 seconds. Treat
this as bounded V1-local evidence, not as a general benchmark or index-tuning
basis.

For successful preserved runs, the script now appends the final key-value
summary to the tail of the same `run.log` path that records per-command output.
The Phase 4NF proof ran the script with a small two-cycle opt-in workload and
confirmed that the stdout summary and final same-length `run.log` tail were
byte-identical. Use this when handing off validation evidence: the named
`log_file` now contains both the command trace and the final compact result
summary.

## Common Recovery Actions

When a WorkVCS business command fails, read stderr as line-oriented key-value
metadata before choosing the recovery path. This is the default output:

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

For machine-readable stderr, pass `--error-format json`. JSON mode emits one
object for the same fields and changes only failure output; successful command
stdout is unchanged.

For code-specific operator actions, use the current
[WorkVCS Error Recovery Guide](error-recovery-guide.md). It covers every
current WorkVCS business `error_code` and the top-level `cli_parse_error`
syntax failure shape.

To prove the current recovery surface end to end, run:

```bash
scripts/operator-recovery-maturity-v0.1.sh
```

The Phase 4NI script audits guide coverage against the current core error
taxonomy and exercises representative CLI recovery for parse errors, branch
head conflicts, Resource drift/unavailable/error states, stale-gated Claim
takeover, and merge unresolved freeze guards.

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
If you are recovering from `context --profile brief`, use the
`verification_id` and `refresh_hint` shown on the relevant
`verification_requirement` item. For a focused Task blocked by a prerequisite,
use the `dependency_verification_requirement` and `refresh_hint` shown on the
`blocked_dependency` item for the blocking Task.

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

Top-level syntax failures use stable key-value stderr by default and the same
field names in opt-in JSON mode. Branch on `error_code=cli_parse_error` and
`clap_error_kind` for stale flags or removed subcommands; do not parse the
human `message` except for display:

```text
error_code=cli_parse_error
error_category=usage
retryable=false
clap_error_kind=unknown_argument
message=error: unexpected argument ...
```

## Still Open For V1

- The documented loop has read-only another-project and bounded write-mode
  repository dogfood evidence. Direct mutation of original external projects,
  remote operation, and broad multi-project maturity remain outside V1 unless
  separately authorized.
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
  Explicit unavailable/error applicability stamps are dogfood-proven. Bounded
  rename, symlink, submodule, sparse-checkout, and path case policies are
  dogfood-proven. Batch Resource-basis refresh is the V1-local explicit
  foreground re-observation scheduling policy; background re-observation,
  daemons, watchers, automatic polling, and implicit refresh remain disabled.
- Context packet persistence, transition-rationale projection, current-task
  Resource-backed Verification Requirement recovery hints, and focused
  blocked-dependency Resource-backed Verification Requirement recovery hints
  are implemented; broader Context/Resource resolver dogfood remains open.
- `why` exposes focused Handoff scope links, anchored evolution as a deferred
  family, causal anchor ChangeSet projections, direct evolution operation
  subjects for those ChangeSets, current recognized detail for those operation
  subjects, direct changed-Entity evolution operations, operation-local Entity
  detail for direct queried-Entity evolution operations, direct relation
  create/remove/restore endpoint evolution operations for recognized
  Record-to-Record, Record-to-Knowledge, and Knowledge-to-Knowledge shapes, and
  current Task scheduling relation edges plus direct scheduling create
  endpoint evolution for `depends_on` and `ordered_before`, current primary
  containment relation edges plus direct containment create endpoint evolution
  for Goal, Plan, and Task endpoints, and direct
  epistemic statement explanations. Full relation-subject traversal beyond
  those direct endpoint slices, multi-hop/full evolution traversal, broader
  causal traversal, and broader context/Resource resolver maturity remain
  open.
- Shared-Claim collaboration has read-only real-project and bounded
  write-mode/read-write dogfood evidence. Automatic ownership arbitration,
  distributed collaboration, and remote multi-operator coordination remain
  outside V1.
- Automatic stale detection remains open.
- Merge lifecycle is dogfood-proven for the bounded V1-local scope, including
  larger local Stores and pre-existing external-project write-mode merge loops.
  Semantic/LLM, remote, distributed, cross-Store, Agent-orchestrated, and direct
  original-repository mutation merge flows remain outside V1.
- Bundle portability is locally dogfood-proven for copied-target same-Store
  operation. The V1-local directory profile is defined, bounded larger Store
  portability is proven, and maintained Store repeated reopen/apply/doctor
  portability is proven. Packaged archives, exchange APIs, and external Store
  canonical DAG activation remain open.
- Larger Store validation now includes the Phase 4LQ one-shot portability run,
  Phase 4MM larger merge-path run, and Phase 4NE larger maintained Store run.
  These are bounded V1-local evidence, not general benchmarks or index-tuning
  justification.
- WorkVCS business errors and top-level clap syntax errors now emit stable
  key-value and JSON fields, and current per-code recovery guidance is
  documented in the operator error recovery guide. Maintained Store validator
  logs now include the final compact success summary at the preserved `run.log`
  tail. Phase 4NI adds a local operator recovery maturity script that proves
  guide coverage, parse-error JSON recovery, branch-head retry, Resource
  recovery, Claim stale takeover, and merge unresolved recovery. Further
  command-friction reduction remains demand-driven.
