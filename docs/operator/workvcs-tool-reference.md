# WorkVCS Tool Reference For Governance Plan Carriers

Status: current-main operator reference for Codex sessions
Last updated: 2026-09-12

This reference explains what the current `workvcs` tool can carry for an
Agent-facing governance workflow. It is meant for other Codex sessions that
need a compact answer to: "Can WorkVCS hold the durable Plan state for this
task, and what should remain in work-governance?"

## P0 Cutover Entrypoint

The implemented entry surface includes configuration inspection, project
bind/discover/list audit, bounded read-only `recall`, read-only `resume --cwd`,
atomic/idempotent standalone `capture`, atomic/idempotent `plan admit`, and
`plan evolve` with manifest mode `in_place|supersede`. Binding uses the Git
common-directory identity and discovers an external Store registry through
explicit `--registry PATH`, `WORKVCS_HOME`, or the XDG config file. The
registry and Store are forced outside the project/repository; complete Store
integrity validation is required before use.

The live admit syntax is:

```text
workvcs plan admit [OPTIONS] --manifest <PATH> <STORE|--cwd <PATH>>
```

Options are `--cwd PATH`, `--registry PATH`, `--branch BRANCH`, and
`--manifest PATH`. Explicit `STORE` requires `--branch`; `--cwd` uses the
bound branch and cannot combine with `--branch`. Expected head/state and
idempotency are manifest fields; there is no `--expected-head` option. The
manifest can carry prior findings, decisions, questions, constraints, and
evidence. Admission is one atomic transition and replaying the same
idempotency key reuses the prior result.

The live evolve syntax is:

```text
workvcs plan evolve [OPTIONS] --manifest <PATH> <STORE|--cwd <PATH>>
```

Its manifest `mode` is `in_place` or `supersede`. In-place evolution updates
only explicitly supplied Plan fields, preserves omitted fields, atomically appends
Tasks with AC/VR, Records, and Evidence, and does not implicitly delete or
replace omitted state. Expected guards, target Plan identity/version/digest,
and idempotency are manifest fields.

`mode=supersede` is current and performs the guarded old→superseded/new→active
transition with same-Goal dual `contains` relations and a machine
`new_plan→old_plan` `supersedes` relation. Constraints require explicit
`carry_all` or `replace`; old Tasks, Records, and Evidence are not migrated.
`receipt issue`, `receipt show`, `receipt list`, and `receipt consume` are
current P0-3a/P0-3b commands.
They expose only mechanical binding plus authority-ref type/digest and a
redacted marker. The structured `authority_ref.ref` input is automatically
redacted and is not persisted or emitted in scope, payload, CLI/show/list, or
debug output; this is not a full-manifest secret scan. Receipt `rationale` is
persisted, so callers must not put credentials, tokens, or other secrets in it.
Consume is branch-scoped single-use; idempotent
replay may reuse only a committed `workstate_commit` result, never an orphan
ChangeSet, and no Store-global lock across restore histories is promised. It is
not atomic with an external action. `revoke`, plus receipt projection into
`context`/`why`, remain deferred and are not current capabilities; they do not
block the current P0 surface.
No-Plan means no Plan is invented. Discovery, audit, recall, and resume are
no-write entrypoints. `project ensure` is the explicit idempotent recovery for
an unbound logical project; it creates only the default external
Store/Workspace/Branch binding. A caller may still explicitly persist standalone
cognition with `capture`; this creates no Goal, Plan, Task, Session, or Claim.
Finding currentness is explicit: `record supersede-finding` and
`record invalidate-finding` atomically transition an active target and add its
typed causal relation. Brief Recall excludes terminal Findings, handoff keeps
terminal Attempts as anti-repetition context, and retrospective preserves
terminal cognition after current Records and Knowledge.

`record currentness-audit` is the bounded read-only review entrypoint for
semantic debt. By default it returns explicit open obligations; add
`--include-current-claims` to review validated Assumptions and active Decisions
and Findings. It supports kind, exact scope, and statement filters, defaults to
50 candidates, rejects budgets above 200, and reports full statement/scope plus
stable IDs and omitted counts. It never infers staleness, mutates Records, or
adds Plan gaps. Branch output is current-head and potentially actionable;
Commit output is historical and inspection-only.

`workvcs closeout inspect` is current. It requires explicit
`--target-kind goal|plan|task` and `--target`, using either `--cwd PATH` or
`STORE` with `--branch BRANCH`/`--commit COMMIT`; it never implicitly selects a
Session. The read is OS/query-only and bounded: default budget 50, maximum 200,
stable ordering, `truncated`/omitted counts, direct target expansion,
exact-target runtime aggregates, and before/after branch/source/target/Store
main-WAL-SHM proof. It emits mechanical state only, not policy,
authorization, quality, ready, complete, push, or deploy conclusions.

Registry updates use cross-process mutual exclusion and atomic replacement;
Store use requires complete integrity validation. WorkVCS does not
decide authorization policy.

This cutover has no `workctl`, schema-v3/v4/v5, or `.work-governance`
compatibility surface. WorkVCS does not decide authorization policy; policy,
confirmation gates, and completion judgment remain with work-governance.

Use the live command help and current repository documents as authority before
running a real workflow. Do not assume commands or flags that are absent from
`workvcs --help` in the current checkout or installed binary.

## Bottom Line

`workvcs` is the local CLI for a durable WorkVCS Store. Use it to record and
query explicit work state: Goals, Plans, Tasks, acceptance and verification
records, evidence, Resources, Sessions, Claims, Handoffs, context packets,
history, diffs, restores, Bundles, Checkpoints, and Merges.

WorkVCS carries durable cognition and execution state. A governance layer, when
present, still owns policy and judgment: goal discovery, Plan admission,
demand contracts, confirmation gates, high-impact boundaries, Git change
governance, validation strength, completion claims, and user-facing reporting.
WorkVCS does not require that governance layer and the governance layer must
remain usable without WorkVCS.

WorkVCS must not add routine process friction. If a task is small or
single-step, do not force a WorkVCS Plan. Still capture a valuable finding,
decision, risk, evidence item, or reusable Knowledge when it would improve
later work or review.

## Current Authority

When sources disagree, use this order:

1. Current user instruction.
2. Current project `AGENTS.md`.
3. Accepted ADRs under `docs/decisions/adr/`.
4. Confirmed domain, architecture, and product documents.
5. Current CLI help and fresh command output.
6. Readiness ledger and release-gate matrix as implementation evidence.
7. Historical conversations, old Plans, logs, and memory only as leads.

Useful current entrypoints:

- `workvcs --help`: live command-family surface.
- `scripts/package-workvcs.sh`: local package helper and explicit
  install/overwrite entrypoint for the `workvcs` binary.
- `docs/operator/quickstart-and-recovery.md`: runnable local workflow and
  recovery examples.
- `docs/operator/error-recovery-guide.md`: stable error fields and per-code
  recovery actions.
- `docs/product/product-definition.md`: product role, boundary, and principles.
- `docs/product/v1-v2-boundary.md`: confirmed V1 scope and deferred V2 scope.
- `docs/provenance/v1-readiness-ledger.md`: current evidence ledger.
- `docs/provenance/v1-release-gate-matrix.md`: current local release-maturity
  gate state.

As of this reference, current project evidence records local V1 release
maturity as ready. That is not release, tag, push, deploy, remote, production,
credential, or global-install authorization.

## Binary Packaging And Installation

Use `scripts/package-workvcs.sh` when another session needs a reproducible local
binary artifact or a governed path to make `workvcs` available as a system
command.

Default package-only mode:

```bash
scripts/package-workvcs.sh
```

This builds the current checkout's `workvcs` binary, packages it with
`skills/workvcs`, writes a manifest containing binary, Skill-entry, and complete
Skill-tree digests, creates a `.tar.gz` archive, and validates the packaged
binary plus every regular file in the Skill tree.

User-global install or overwrite is explicit:

```bash
scripts/package-workvcs.sh --dry-run --install --bin-dir "$HOME/.local/bin"
scripts/package-workvcs.sh --install --bin-dir "$HOME/.local/bin"
command -v workvcs
workvcs --help
```

The dry-run command is the safe first check: it does not build or write, and it
reports the planned binary and Skill destinations. Select `/usr/local/bin`
explicitly instead when a system-wide destination is intended, or use `--dest`
for an exact path whose basename is `workvcs`. The install command creates the
target directory if needed, overwrites through a temporary file, verifies the
installed command with `workvcs --help`, checks that the installed digest
matches the packaged binary, and atomically installs the Skill under
`$HOME/.agents/skills/workvcs` by default. Use `--skills-dir` to select another
Agent Skills root or `--no-install-skill` for a binary-only installation. If
the binary destination directory is not writable, the script uses `sudo` for
that binary step; the selected Skill directory must be writable.
The packaged `skill-tree.sha256` is deterministic and catches missing, modified,
or extra regular files. It also fails closed for unreadable files, unsupported
special entries, and paths containing control characters.
`scripts/workvcs-skill-tree.sh verify SKILL_DIR MANIFEST` provides the same
check independently.

For validation without touching a system path:

```bash
tmp_bin="$(mktemp -d "${TMPDIR:-/tmp}/workvcs-bin.XXXXXX")"
scripts/package-workvcs.sh --install --bin-dir "$tmp_bin" --profile debug
"$tmp_bin/workvcs" --help
```

The script is an availability helper, not an authority grant. A Codex session
still needs current user authorization before running a real global
installation, overwrite, release, tag, push, deploy, remote, production, or
credential operation.

## Responsibility Split With Policy Skills

WorkVCS is responsible for durable work memory:

- the user's Goal and the Plan strategy or decomposition;
- concrete Tasks, dependencies, ordering, containment, and priorities;
- Acceptance Criteria and Verification Requirements;
- Evidence and Verification judgments;
- Resource observations, applicability, and drift;
- Session focus, Claims, Handoffs, and continuation context;
- historical state, diffs, why explanations, and recovery packets.

An optional policy layer, including work-governance when installed, remains
responsible for judgment:

- discovering the goal and deciding whether a Plan adds value;
- defining scope, exclusions, stop or revision conditions, and authority;
- choosing validation strength and interpreting whether evidence proves a claim;
- deciding when cognition should be promoted into project authority;
- reporting completion, residual risk, and useful next work.

WorkVCS and work-governance are independently usable. When combined, the
WorkVCS Skill owns configuration and command mechanics while policy Skills own
meaning and judgment. WorkVCS records, including receipts, do not authorize the
underlying external or high-impact action.

## Current Capability Surface

The current top-level CLI exposes these command families:

| Area | Command families | What they carry |
| --- | --- | --- |
| Store and integrity | `init`, `doctor`, `store`, `canonical`, `id` | Store bootstrap, metadata, schema/integrity checks, lineage, canonical bytes, typed IDs, and digests. |
| Versioned work graph | `workspace`, `branch`, `goal`, `plan`, `task`, `reference`, `entity` | Workspace and Work Branch state, Goals, Plans, Tasks, structural references, lifecycle transitions, containment, dependency, ordering, and general entity inspection. |
| Acceptance and evidence | `ac`, `vr`, `verify`, `verification`, `evidence` | Acceptance Criteria, Verification Requirements, single-target verification wrapper output, Verification judgments, and Evidence records. |
| Resources and drift | `resource`, `projection`, `verification cache-refresh` | Resource registration, observations, applicability, stale/drift/unavailable/error projections, and explicit foreground refresh. |
| Runtime coordination | `session`, `claim`, `handoff`, `next`, `runnable` | Agent Sessions, focus, exclusive/shared Claims, Claim transfer/takeover, focused Handoffs, runnable Task projection, and next-work selection. |
| Authorization receipts | `receipt issue`, `receipt show`, `receipt list`, `receipt consume` | Current P0-3a/P0-3b mechanical AuthorizationReceipt issue, redacted inspection/listing, and branch-scoped single-use consume; revoke is not current. |
| Entry, capture, and recall | `config`, `project`, `capture`, `record currentness-audit`, `record supersede-finding`, `record invalidate-finding`, `recall`, `resume` | Stable registry discovery, explicit idempotent first-use binding, binding audit, standalone cognition, bounded read-only semantic-currentness review, guarded Finding correction, profile-prioritized bounded project context, and Session-aware recovery. |
| Query and explanation | `closeout inspect`, `context`, `context-packet`, `why`, `history`, `show-at`, `diff`, `changeset`, `commit`, `event` | Closeout summaries, bounded mechanical inspection, saved context snapshots, causal and structural explanations, historical inspection, WorkState diffs, ChangeSets, commit metadata, and events. |
| Portability and branching | `checkpoint`, `bundle`, `restore`, `merge` | Checkpoints, local Bundle export/validate/apply flows, restore-as-new-commit semantics, and three-way Work Branch merge lifecycle. |
| Error handling | `--error-format key-value|json` plus command stderr | Script-readable error code, category, retryability, optional JSON output, and actionable recovery boundaries. |

## Minimal Use Pattern

For a new local Store:

```bash
STORE=.workvcs/local.sqlite
workvcs init "$STORE" --display-name local-work
workvcs workspace create "$STORE" --display-name local-workspace
```

Capture emitted IDs such as `workspace_id`, `branch_id`, and
`genesis_commit_id`. Treat the emitted `commit_id` from each versioned mutation
as the next expected head for later mutations.

For a governed task:

```bash
workvcs goal create "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --description "Preserve the user's explicit objective"

workvcs plan create "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --description "Execute the smallest validated slice" \
  --strategy "Keep the next action aligned with the active goal"

workvcs task create "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --description "Implement or analyze the next bounded step"
```

After capturing the emitted Goal, Plan, Task, and `commit_id` values, establish
the current Plan path explicitly:

```bash
workvcs task contain "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --parent "$GOAL_ENTITY_ID" \
  --child "$PLAN_ENTITY_ID"

workvcs task contain "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --parent "$PLAN_ENTITY_ID" \
  --child "$TASK_ENTITY_ID"
```

Add Acceptance Criteria and Verification Requirements when a task has a real
completion condition:

```bash
workvcs ac create "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --task "$TASK_ENTITY_ID" \
  --task-version "$TASK_ENTITY_VERSION_ID" \
  --local-key ac-validation \
  --statement "The bounded slice passes its validation command"

workvcs vr create "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --criterion "$AC_ENTITY_ID" \
  --criterion-version "$AC_ENTITY_VERSION_ID" \
  --local-key vr-validation \
  --statement "Run the validation command and record its result"
```

Record verification evidence through the high-level wrapper when possible:

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

Start or continue an Agent session:

```bash
workvcs session start "$STORE" \
  --workspace "$WORKSPACE_ID" \
  --branch "$BRANCH_ID"

workvcs claim next "$STORE" \
  --session "$SESSION_ID" \
  --context-profile brief \
  --context-budget-items 12
```

Use `claim next` when selecting and claiming work atomically is intended. Use
`context` or `next` when you only need to inspect state.

## Low-Token Recovery Pattern

Start with the smallest deterministic query that can answer the continuation
question:

```bash
workvcs resume "$STORE" \
  --session "$SESSION_ID" \
  --budget-items 12

workvcs context "$STORE" \
  --session "$SESSION_ID" \
  --profile brief \
  --budget-items 12

workvcs next "$STORE" --session "$SESSION_ID"

workvcs why "$STORE" \
  --commit "$HEAD_COMMIT_ID" \
  --entity "$TASK_OR_PLAN_ID"
```

Use `resume` first when the continuation question is "what is the goal, current
work, blocker, evidence or Resource-basis recovery hint, and next action?" It
is read-only and reuses brief context data without claiming work or saving a
packet.

Use `context` when you need the generic packet shape, `next` when you only need
the scheduler's next-work selection, and `why` when the question is causal or
structural. Use `--scope-path`, `--scope-path-prefix`, or `--scope-json` when
the current task is tied to a file, directory, or resource scope. Prefer
`resume` or `brief` first, then `normal`, and only use `full` when the smaller
packet omits a fact that changes the next action.

Save the packet when another session, reviewer, or future continuation must be
able to verify exactly what context was used:

```bash
workvcs context-packet save "$STORE" \
  --session "$SESSION_ID" \
  --profile brief \
  --budget-items 12
```

For historical questions, use the query that matches the need:

- `history`: what commits happened.
- `diff`: what changed between two WorkState targets.
- `show-at`: what state existed at a branch or commit.
- `why`: why a current entity or relation is explainable from current
  structural, verification, knowledge, or selected evolution evidence.

## Handoff Pattern

When handing work to another session, do not rely on transcript memory alone.
End or summarize the current Session, create a focused Handoff, and save or
show the relevant context:

```bash
workvcs session end "$STORE" \
  --session "$SESSION_ID" \
  --summary-json '{"outcome":"completed bounded slice"}'

workvcs handoff create "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --session "$SESSION_ID" \
  --session-diff "$SESSION_DIFF_ID" \
  --focus "$TASK_ENTITY_ID" \
  --statement "Continue from this task and inspect context before claiming work"
```

The next session should run Store validation, inspect the Handoff, consume it
only if it is the intended focus, then query `context` and `next` before
claiming work.

## Resource And Drift Pattern

Use Resources when a verification depends on source files, directory prefixes,
globs, Git worktree state, or another explicit observable basis.

Supported local V1 refresh modes include:

- exact local-file path;
- local-file path prefix;
- local-file glob;
- Git worktree;
- basis-aware refresh for supported current-head Resource-backed Verifications;
- batch foreground refresh for current-head Resource-backed Verifications.

Refresh is explicit foreground work. Current V1 does not run background
watchers, daemons, automatic polling, implicit refresh, or Agent orchestration.

## Error And Recovery Pattern

By default, WorkVCS business errors and top-level CLI syntax errors emit
line-oriented fields:

```text
error_code=<CODE>
error_category=<CATEGORY>
retryable=<true|false>
message=<ESCAPED_MESSAGE>
```

For script-readable JSON stderr:

```bash
workvcs --error-format json <command> ...
```

Branch on `error_code`, `error_category`, and `retryable`; treat `message` as
display context. See `docs/operator/error-recovery-guide.md` before retrying a
failed workflow.

## Current Non-Capabilities

Do not use WorkVCS as if it currently provided:

- transcript parsing or automatic extraction of Findings, Decisions, Risks, or
  Knowledge;
- LLM-generated semantic records without explicit Agent confirmation;
- embeddings, vector search, semantic retrieval, LLM merge, or natural-language
  conflict detection;
- automatic knowledge distillation;
- hooks that infer and prompt for semantic records;
- Agent launching, scheduling, orchestration, or automatic execution;
- remote/cloud synchronization, distributed collaboration, or live cross-Store
  federation;
- background Resource watchers, daemons, or automatic re-observation;
- destructive compaction of core Decision, Finding, Knowledge, ChangeSet, or
  WorkStateCommit history;
- a required GUI, TUI, or human-first storage format;
- release, tag, push, deploy, production, credential, or global installation
  authority;
- automatic replacement of work-governance's confirmation, risk, validation,
  Git, or closeout responsibilities.

The repository includes `scripts/package-workvcs.sh` to package and, when
separately authorized, overwrite a system `workvcs` binary. Until that
operation is explicitly performed and verified, a session should discover the
available binary through the current environment and confirm it with
`workvcs --help`.

## Practical Rule For Other Sessions

Use WorkVCS when the question is about durable work state:

- What is the explicit goal?
- What Plan or Task is current?
- What evidence proves or blocks it?
- What Resource or source state was verified?
- Why is this state believed?
- What should a continuation session inspect or claim next?

Use work-governance when the question is about process authority:

- Is this request No-Plan or Plan-controlled?
- What is the demand contract?
- Does the action need user confirmation?
- Is the validation strong enough?
- Can the local slice, route, commit, release, installation, or remote action be
  claimed complete?

The best integration keeps both tools small: WorkVCS stores the facts and
relationships; work-governance decides whether acting on those facts is
authorized, validated, and ready to report.
