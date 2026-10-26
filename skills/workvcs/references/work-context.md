# Session-selected work and recovery

ProjectRef selects the logical owner, Store, and Workspace. Choose current work
from the Project's conversation goals, continuation, parallelism, and shared
state. Conversations may share or separate Branches and may explicitly switch.
Work Branches are independent of Git branches and worktrees.

## Select existing work

Recover exact Sessions, Focus, Task, and Handoff identities rather than guessing
from the newest project records. A conversation ID is provenance, not another
durable work identity or a self-proving authorization receipt. Native commands:

```text
workvcs session start STORE --workspace WORKSPACE --branch BRANCH --metadata-json JSON
workvcs session focus-set STORE --session SESSION --focus ENTITY
workvcs branch fork STORE --from-branch BRANCH --name NAME
workvcs session switch STORE --session SESSION --workspace WORKSPACE --branch BRANCH
```

Create or switch only within current authority. Neither a new conversation nor
the Project's default binding determines the Branch automatically. Switching
clears Focus unless replaced and releases old-Branch Claims through the native
protocol; read back the result before treating ownership as current.

Project-facing recovery follows the selected Session's actual Branch:

```text
workvcs recall --cwd PATH --project-ref PROJECT --session SESSION --profile brief
workvcs resume --cwd PATH --project-ref PROJECT --session SESSION
workvcs recall --cwd PATH --project-ref PROJECT --branch BRANCH --task TASK --profile brief
workvcs recall --cwd PATH --project-ref PROJECT --branch BRANCH --scope-json '{"work":"named-work"}' --profile brief
```

An explicit Branch must agree with the Session and be active in the resolved
Workspace. `resume --cwd` without an exact Session requires one unambiguous
active Session across that Workspace. An ended predecessor is read through
exact Handoff/history evidence, not resumed as active.

Selectors match record scope by structural subset before ranking and output
budget. No-Plan work can retain a stable explicit scope across conversations;
a read creates neither a Task nor a Plan. Runtime Sessions and Claims must also
be relevant, so other tasks do not consume the selected work's budget.
Task/Focus identifiers in a selector must agree with the Session Focus. An
explicit No-Plan scope selects its own records and suppresses unrelated Tasks,
acceptance, readiness, dependencies, and Claims. The Session's original Focus
is reported only as a runtime anchor. A Session with no Focus and no explicit
work selector requests Project overview; select a Task/Goal/Plan Focus or a
No-Plan scope before treating recovered records as the current work.
File selectors use the native normalized path/prefix overlap rules, including
`scope_payload` nesting. Additional Task/Focus and metadata conditions remain
conjunctive. A file-only selector without Task/Focus keeps the Project overview
of Tasks and runtime while restricting Record/Knowledge evidence to that path;
it does not choose a Task. Shared project facts and knowledge without a file
condition remain explicitly labelled reference context. Empty or malformed
file selectors fail before recovery or capture admission.

## Record in the same context

```text
workvcs capture-validate --manifest FILE
workvcs capture --cwd PATH --project-ref PROJECT --session SESSION --value-reason REASON --manifest FILE --deliver-existing-binding
workvcs capture --cwd PATH --project-ref PROJECT --branch BRANCH --scope-json '{"work":"named-work"}' --value-reason REASON --manifest FILE --deliver-existing-binding
workvcs plan admit --cwd PATH --project-ref PROJECT --session SESSION --manifest FILE
workvcs plan evolve --cwd PATH --project-ref PROJECT --session SESSION --manifest FILE
```

Journal, capability, binding, and action-authority gates remain in force.
Selection is immutable in the admitted operation; the same idempotency key
with another context fails. Session Workspace/Branch/Focus are checked inside
the write transaction. A switched, ended, or refocused Session cannot deliver a
new write under old context. An already committed identical result can still
repair its original receipt. Retain its CaptureId and use status-first recovery;
never change key or target to escape uncertainty.

Explicit selectors and native Task Focus are attached to new records.
Routed `plan admit` and `plan evolve` accept the same `--session`, `--branch`,
`--task` and `--scope-json` selection and preserve it on accompanying Records
and Evidence. Their exact tagged manifest is used for delivery and readback.
Conflicting scope fields fail before admission. Preserve the complete final
goal, achieved result, remaining gap, actual blocker, current authority, next
useful action, and exact reusable artifact references as minimal semantic
deltas. A local repair, test, review, commit, or handoff does not finish an
unmet original requirement.

## Applicability and currentness

Task records use `scope.task_id`; non-Task Focus uses
`scope.focus_entity_id`. A Finding deliberately published as an evidenced shared
fact uses `scope.applicability = "project_fact"`. Knowledge without work-specific
scope can also serve as project knowledge. Keep source and conditions explicit.
Do not relabel another task's pending actions, acceptance, or authorization as
shared facts; reference evidence does not independently grant authority.
Rendered Record/Knowledge items carry explicit `applicability` and
`action_authority=none` labels. Shared facts and project knowledge are evidence
to assess against current instructions, never inherited task instructions.

Focused Context/Resume excludes other work's instructions. Unclassified legacy
records are exact references in Full context, not current task requirements.
Forking inherits the base state and does not classify mixed historical records.
Reconcile existing scope/currentness only with verified provenance and authority
for that exact change. Use the dedicated guarded operations in `semantics.md`
after user corrections; a newer independent note does not supersede an active
older Record automatically.

Retrospective Evidence is selected before the output budget. Routed operations
write the selected identity and verified origin `workspace_id`/`branch_id` to
the single structured `metadata.work_scope` field. Other metadata, including a
source Session's current Focus, is not an ownership selector. Explicit shared
Evidence uses `metadata.work_scope.applicability = "project_fact"` and remains
reference material. Contradictory scope or origin fields fail before admission.
Immutable capture and Plan commit ancestry proves inherited Evidence; a Branch
forked before introduction does not receive it. Current typed Verification
citations can also qualify Evidence through the selected Task's exact AC/VR
references and semantic versions. Unclassified standalone Evidence stays
available by its exact native id, including when a Project overview is selected.

## Incremental recovery

Recall returns `context_digest` and `head_commit_id`. Reuse a sufficient cached
packet with the same selection and profile:

```text
workvcs recall --cwd PATH --project-ref PROJECT --session SESSION --profile brief --since-commit COMMIT --expected-context-digest DIGEST
```

The response reports changed semantic versions, `removed.*` identities, and
fresh runtime context. The basis must be an ancestor of the chosen head.
Owner, Store, Branch, Session/Focus, selector, or profile changes invalidate the
digest. The digest also binds the exact cached commit, selected semantic item
versions, Evidence identities, and immutable introduction/citation basis;
another valid ancestor cannot reuse it. Unchanged Evidence is excluded from deltas.
Relevant standalone Evidence added outside semantic history invalidates the
old basis; recover a fresh packet instead of replaying old Evidence.
Each response returns the new head's basis. Delta does not recover items omitted from an earlier bounded packet;
read exact Task/Record/Evidence/Handoff objects when needed. An agreeing exact
mutation receipt/readback does not justify another broad recovery read.

## Layered health

Use `project health --format json` with the exact CWD/ProjectRef for structured
control-plane and current Store evidence. Select `--source-archive PATH` or
`--search-index PATH` only when that historical/retrieval capability matters.
Unselected evidence is `not_checked`; optional failures retain successful
results. Health never refreshes an index, repairs state, or authorizes a guarded
mutation. Product continuation still needs a trusted adapter and its separate
Project policy; local readiness cannot self-prove visible conversation creation
or user authorization.

`workvcs continuation readiness STORE --scope-id ID` returns one typed JSON
snapshot from a read-only transaction. It checks current policy, listener,
carrier promotion, Claim activity, Branch head/projection, Task, and rotation
state. A non-ready state has explicit gaps; a malformed or missing source
fails at its native boundary. Consume this aggregate directly rather than
joining independent runtime reads or refreshing currentness during inspection.
