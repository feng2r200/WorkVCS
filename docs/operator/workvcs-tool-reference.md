# WorkVCS Tool Reference For Governance Plan Carriers

Status: current-main operator reference for Codex sessions
Last updated: 2026-09-07

This reference explains what the current `workvcs` tool can carry for an
Agent-facing governance workflow. It is meant for other Codex sessions that
need a compact answer to: "Can WorkVCS hold the durable Plan state for this
task, and what should remain in work-governance?"

Use the live command help and current repository documents as authority before
running a real workflow. Do not assume commands or flags that are absent from
`workvcs --help` in the current checkout or installed binary.

## Bottom Line

`workvcs` is the local CLI for a durable WorkVCS Store. Use it to record and
query explicit work state: Goals, Plans, Tasks, acceptance and verification
records, evidence, Resources, Sessions, Claims, Handoffs, context packets,
history, diffs, restores, Bundles, Checkpoints, and Merges.

For work-governance integration, WorkVCS should carry the durable execution
state. work-governance should still own policy and judgment: intake, Plan
admission, demand contracts, confirmation gates, high-impact boundaries, Git
change governance, validation strength, completion claims, and user-facing
handoff wording.

WorkVCS must not add routine process friction. If a task is truly small,
single-step, and has no handoff or recovery value, do not force a WorkVCS Plan.
Use WorkVCS when durable state reduces drift, preserves evidence, or makes a
future continuation cheaper.

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

## Responsibility Split With work-governance

WorkVCS is responsible for durable work memory:

- the user's Goal and the Plan strategy or decomposition;
- concrete Tasks, dependencies, ordering, containment, and priorities;
- Acceptance Criteria and Verification Requirements;
- Evidence and Verification judgments;
- Resource observations, applicability, and drift;
- Session focus, Claims, Handoffs, and continuation context;
- historical state, diffs, why explanations, and recovery packets.

work-governance remains responsible for process control:

- deciding whether a request needs governed Plan control or can stay No-Plan;
- defining the demand contract, scope, exclusions, and stop triggers;
- asking for user confirmation when authority is insufficient;
- guarding high-impact actions, remote state, production, credentials, release,
  global installation, destructive cleanup, and substantive rollback;
- selecting validation strength and reporting residual risk;
- making completion, commit-ready, release-ready, or route-complete claims.

The intended integration pattern is to keep long execution memory in WorkVCS
IDs, digests, and context packets. work-governance should keep only the
governance receipt needed to explain why the task was admitted, what authority
exists, what gates remain, and which WorkVCS Store/Workspace/Branch/Goal/Plan
or Task is authoritative.

## Current Capability Surface

The current top-level CLI exposes these command families:

| Area | Command families | What they carry |
| --- | --- | --- |
| Store and integrity | `init`, `doctor`, `store`, `canonical`, `id` | Store bootstrap, metadata, schema/integrity checks, lineage, canonical bytes, typed IDs, and digests. |
| Versioned work graph | `workspace`, `branch`, `goal`, `plan`, `task`, `reference`, `entity` | Workspace and Work Branch state, Goals, Plans, Tasks, structural references, lifecycle transitions, containment, dependency, ordering, and general entity inspection. |
| Acceptance and evidence | `ac`, `vr`, `verify`, `verification`, `evidence` | Acceptance Criteria, Verification Requirements, single-target verification wrapper output, Verification judgments, Evidence records, and closeout support. |
| Resources and drift | `resource`, `projection`, `verification cache-refresh` | Resource registration, observations, applicability, stale/drift/unavailable/error projections, and explicit foreground refresh. |
| Runtime coordination | `session`, `claim`, `handoff`, `next`, `runnable` | Agent Sessions, focus, exclusive/shared Claims, Claim transfer/takeover, focused Handoffs, runnable Task projection, and next-work selection. |
| Query and explanation | `context`, `context-packet`, `why`, `history`, `show-at`, `diff`, `changeset`, `commit`, `event` | Low-token recovery packets, saved context snapshots, causal and structural explanations, historical inspection, WorkState diffs, ChangeSets, commit metadata, and events. |
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
workvcs context "$STORE" \
  --session "$SESSION_ID" \
  --profile brief \
  --budget-items 12

workvcs next "$STORE" --session "$SESSION_ID"

workvcs why "$STORE" \
  --commit "$HEAD_COMMIT_ID" \
  --entity "$TASK_OR_PLAN_ID"
```

Use `--scope-path`, `--scope-path-prefix`, or `--scope-json` when the current
task is tied to a file, directory, or resource scope. Prefer `brief` first,
then `normal`, and only use `full` when the smaller packet omits a fact that
changes the next action.

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

Installing or overwriting `/usr/local/bin/workvcs` is a separate later
operation. Until that operation is explicitly performed and verified, a session
should discover the available binary through the current environment and
confirm it with `workvcs --help`.

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
