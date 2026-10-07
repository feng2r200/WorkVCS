# WorkVCS Tool Reference For Governance Plan Carriers

Status: current-main operator reference for Codex sessions
Last updated: 2026-10-07

This reference explains what the current `workvcs` tool can carry for an
Agent-facing governance workflow. It is meant for other Codex sessions that
need a compact answer to: "Can WorkVCS hold the durable Plan state for this
task, and what should remain in work-governance?"

## Accepted ProjectRef Target

ADR-0513 accepts a tool-neutral ProjectRef control plane and journal-first
capture route. Its owner order is explicit ProjectRef, verified semantic
Project, Git common directory, CWD, then pending; an unbound higher owner blocks
fallback. Cross-project work has one canonical primary Record and immutable
control-plane references. Registry v2 migration is explicit and preview-gated.

This is project authority with a core foundation and bounded migration/read
routing mechanics. The source tree has versioned ordinary reads for discovery,
list, recall, resume, and currentness audit. Registry v1 remains readable and
reports `migration_required=true`. Registry v2 ordinary reads fail closed
unless an exact read-routing activation marker matches the current registry
ID, revision, and digest. Legacy direct Store mutation remains on the v1
compatibility path. The `capture` caller has a separate,
value-qualified journal route: on v1 it admits a target-neutral intent and
reports that migration is required; on v2 it requires both exact activation
markers and admits a registry-coupled intent. Neither path bootstraps a
ProjectRef or writes a target Store. This command previews the exact v1-to-v2
mapping without writing:

```text
workvcs project registry-migrate --preview [--registry PATH] [--repair-manifest PATH] [--format text|json]
```

The preview validates Stores read-only and reports source/preview digests and
`apply_eligible`; it does not create ProjectRefs or authorize migration.
An optional strict ownership-repair manifest may preview one or more proven
historical path locators as retired and namespaced semantic locators as active;
it pins the exact source and target digests and cannot move a Store or mutate
the live registry.

The CLI also contains `registry-migrate --apply` with mandatory
`--expected-source-digest` and `--expected-preview-digest`, plus read-only
`--rollback-check` and mutating `--rollback`, each with mandatory
installed/backup digests. Apply has bounded configured-local evidence;
rollback and mutation fault windows retain isolated-fixture evidence. Rollback
requires absent read-routing activation and empty supported
journal layouts, retains an exact v2 snapshot, atomically restores the exact v1
backup, and treats post-rename failures as indeterminate. For a standard
registry filename, the probe enumerates both WorkVCS-home and registry-sidecar
activation/journal aliases, independent of whether the caller used home
configuration or explicit `--registry`. `rollback_apply_safe` identifies
`v2_ready`; `rollback_reentry_safe` is true only for exact `v1_restored`.
Apply/rollback replacement is currently enabled only on Unix-family platforms.
The implementation combines the journal-emptiness guard with one
canonical-registry-derived admission/rollback quiescence lock and exposes its
path/state in `--rollback-check`; both race orders are fixture-tested. The
actual `capture` route uses that closed alias and mandatory pre/post-caller
registry identity check. V2 admission is still default-off and requires its
own exact activation marker.

The exact v2 read-routing gate is inspected and prepared separately:

```text
workvcs project routing-activation --status [--registry PATH]
workvcs project routing-activation --preview [--registry PATH]
workvcs project routing-activation --apply --expected-registry-digest DIGEST --expected-candidate-digest DIGEST [--registry PATH]
workvcs project routing-activation --apply --expected-registry-digest DIGEST --expected-candidate-digest DIGEST --expected-activation-digest OLD_DIGEST [--registry PATH]
```

Absence is off. Malformed, symlinked, or mismatched markers fail closed. A stale
marker also fails closed unless apply supplies its exact installed digest and
the marker names an earlier revision of the same registry ID. That refresh
locks and rechecks the current registry, new candidate, and old marker before
atomically replacing it. It never accepts another lineage, an equal/newer
revision, or manual deletion as recovery.
The marker activates only v2 reads; it does not activate journal admission or
Store writes. Presence in source or an installed package does not prove that
migration or activation happened. Inspect the exact installed revision and
the read-only activation status for the selected registry; every live
mutation still requires authority covering that exact operation.
After an activation post-install failure,
`routing_activation_install_indeterminate` requires a read-only `--status`
check against the same registry before retry or recovery. The command never
deletes or overwrites an uncertain marker.

The separate journal-admission gate is:

```text
workvcs project journal-admission-activation --status [--registry PATH]
workvcs project journal-admission-activation --preview [--registry PATH]
workvcs project journal-admission-activation --apply --expected-registry-digest DIGEST --expected-candidate-digest DIGEST [--registry PATH]
workvcs project journal-admission-activation --apply --expected-registry-digest DIGEST --expected-candidate-digest DIGEST --expected-activation-digest OLD_DIGEST [--registry PATH]
workvcs project journal-admission-activation --disable --expected-registry-digest DIGEST --expected-activation-digest DIGEST [--registry PATH]
```

Apply requires the exact read-routing marker to be active and holds the
registry lock before the shared journal-quiescence lock. Disable uses the same
order, requires the exact current registry and installed-marker digests, and
excludes new admissions before removal. Absence is off; malformed, stale,
wrong-scope, symlinked, or uncertain state fails closed. The only stale-marker
exception is an apply that locks the exact old marker digest and proves an
earlier revision of the same registry lineage before atomic replacement.
Marker version 2 exposes `cognition_capture`, `plan_admit`, and `plan_evolve`.
A version-1 marker remains cognition-only and may be replaced at the same
registry revision only through an exact-old-digest strict capability-superset
refresh. This marker permits only immutable typed-intent admission. It does
not by itself authorize ProjectRef bootstrap, journal-event processing, target
Store delivery, or any live operation.

Shared-binding isolation is a separate registry repair surface:

When registry v2 preserves an unintentional exact shared target, use the
separate repair surface before writing new logical-project state:

```text
workvcs project isolate-shared-binding --preview --cwd PATH [--project-ref ID] [--registry PATH] [--store-root PATH]
workvcs project isolate-shared-binding --apply --cwd PATH [--project-ref ID] [--registry PATH] [--store-root PATH] --expected-registry-digest DIGEST --expected-candidate-digest DIGEST
```

Preview resolves and verifies one ProjectRef without requiring active ordinary
read routing and performs zero writes. Apply is Unix-only, digest-locked, and
serialized against journal admission. It preserves exact old registry bytes,
creates or reuses only the deterministic pristine Store, changes exactly one
binding to `binding_source=isolation`, and copies no source Store history.
Other bindings and historical `possible_shared_target` observations remain
unchanged. Both activation markers become stale and require their own exact
old-digest refresh. After
`shared_binding_isolation_install_indeterminate`, run preview before any retry;
never hand-edit the registry or infer rollback.

Start composite diagnosis with the strictly read-only health surface:

```text
workvcs project health [--cwd PATH [--project-ref ID]] [--registry PATH] [--require-healthy] [--timings]
```

It loads one registry snapshot, performs one complete validation of every
binding, and reports registry, routing, journal, capability, and optional CWD
resolution health. It never activates, admits, recovers, or writes. Use
`--require-healthy` for a gate and `--timings` only for diagnostics.

The source tree also contains a separately explicit recovery surface:

```text
workvcs project operation-recovery --status --capture-id ID [--registry PATH]
workvcs project operation-recovery --apply --capture-id ID --expected-registry-digest DIGEST --expected-projection-digest DIGEST [--registry PATH] [--store-root PATH]
```

`capture-recovery` remains a visible compatibility alias. `--status` is
read-only. It reports the intent `payload_kind`, validates the immutable event chain, derives the
authoritative projection, compares any stored projection, re-resolves current
ownership, and reports the exact next action. `--apply` is not implied by
either activation marker: it is a distinct Unix-only, digest-locked operator
action. For an unbound semantic or repository owner it may converge exactly
one established ProjectRef/binding; CWD-only ownership remains provisional.
Conflict and unresolved ownership never fall through to a lower owner.

Recovery first converges or revalidates the exact binding. When
the projection is `pending_primary`, it records `delivery_started` with the
exact Branch head/state guards and one target idempotency key before invoking
the existing atomic cognition-capture engine. A successful result records a
complete `delivery_applied` receipt. If the target commit completed before the
receipt, retry reuses the same target result and writes only the missing
receipt; it never creates a second Record, Knowledge, Evidence, relation, or
commit. A legacy v1 manifest is delivered only when both of its original
guards match; absent or stale guards produce
`legacy_manifest_upgrade_required` without rewriting the intent or mutating
the target Store. A historical intent whose materialized cognition manifest
is semantically invalid records the terminal state `semantic_manifest_invalid`
before opening the target Store for mutation and directs the operator to start
a corrected new Capture. A primary receipt with requested secondary references
stops at `pending_references`. Recovery validates the member ProjectRefs, appends
only missing immutable-reference receipts without opening a secondary Store,
and installs one idempotent group-completion summary. The read-only command
`workvcs project capture-group-recall --project-ref-id ID [--registry PATH]`
scans immutable intent/event authority and returns references pinned to the
exact canonical Record version.
Registry re-resolution reuses that receipt only after the same ProjectRef and
exact Store/Workspace/Branch are revalidated; a changed target clears the
current receipt view for a non-group capture and does not write automatically.
After a CaptureGroup canonical receipt exists, retargeting fails closed and a
new capture is required.

Post-install or post-target uncertainty returns
`capture_recovery_install_indeterminate` and requires `--status` before
forward recovery; the command never guesses a rollback. The ordinary
`capture` route remains admission-only, neither activation marker implicitly
runs recovery. The recovery route and its fault boundaries are
fixture-tested; current installed revision, registry identity, marker digests,
and operation authority must be verified before use on a configured registry.

Unless an installed route and both exact activation markers are verified,
`project_binding_not_found` must
preserve a valuable pending semantic packet and trigger deliberate owner
selection; it must never be translated into “no record.” That active-context
packet is not a current WorkVCS durability guarantee.

The accepted route guarantee is **no silent loss after admission**. It is
not a claim that every valuable thought is observed or captured while the
global per-turn Hook remains deferred.

## P0 Cutover Entrypoint

The implemented entry surface includes configuration inspection, project
bind/discover/list audit, bounded read-only `recall`, read-only `resume --cwd`,
legacy atomic/idempotent standalone `capture`, journal-first ProjectRef-v2
`capture`, atomic/idempotent `plan admit`, and
`plan evolve` with manifest mode `in_place|supersede`. Binding uses the Git
common-directory identity and discovers an external Store registry through
explicit `--registry PATH`, `WORKVCS_HOME`, or the XDG config file. The
registry and Store are forced outside the project/repository; complete Store
integrity validation is required before use.

On an activated v2 fixture, `project discover`, `recall`, cwd-based `resume`,
and cwd-based `record currentness-audit` accept `--project-ref ID` and
`--locator-context FILE`; they also accept
`--locator-adapter-context FILE` for a strict adapter-dispatch envelope from a
trusted integration. `--locator-context` carries already-verified,
tool-neutral semantic locator evidence. The adapter path invokes the selected
provider behind `ContextLocatorProvider` and may be combined with that verified
evidence. Resolution order is explicit ProjectRef, semantic Project, Git
common directory, then CWD; an unbound higher rank blocks fallback. These
options fail closed on registry v1 rather than silently changing its ownership
semantics.

The first concrete dispatch target is
`codex-app-project-metadata/v1`. Its context is bounded by the generic 64 KiB
non-secret gate:

```json
{
  "schema_version": 1,
  "adapter_id": "codex-app-project-metadata/v1",
  "context": {
    "codex_home": "/canonical/codex/home",
    "project_metadata": {
      "host_id": "local",
      "project_id": "g-p-0123456789abcdef0123456789abcdef",
      "project_kind": "chatgpt",
      "thread_id": "01234567-89ab-cdef-0123-456789abcdef",
      "verification_sources": [
        "codex_app.list_projects",
        "codex_app.read_thread"
      ]
    },
    "mirror_path": "/canonical/codex/home/.chatgpt-projects/g-p-0123456789abcdef0123456789abcdef"
  }
}
```

`project_metadata` is authoritative only when the integration verified both
named application sources. `mirror_path` is separately canonicalized and is
only verified-derived evidence. Either field may be omitted; omitting both is
an explicit provider-unavailable result and permits normal Git/CWD fallback.
If both disagree, metadata wins and `context_mismatch` remains visible. Unknown
fields, secret-bearing keys, malformed IDs, a noncanonical/out-of-root mirror,
or unsupported adapters fail closed. WorkVCS persists only locator fields and
evidence digests in an admitted intent, not the raw adapter context or its
explanation. This file is an integration handoff, not a user-authored claim;
do not synthesize it from labels, memory, or arbitrary text.

The legacy direct-Store capture form is:

```text
workvcs capture --cwd PATH --manifest FILE [--registry PATH]
```

The journal-first form adds `--value-reason TEXT` and may also add
`--project-ref ID`, `--locator-context FILE`,
`--locator-adapter-context FILE`, and `--capture-group FILE`. Registry v1
admits a `legacy_cognition_v1` target-neutral intent and reports migration
required; it rejects `--capture-group`. Registry v2 requires both exact
activation markers and admits `cognition_v2`: the CLI removes manifest
transport fields and rejects caller-supplied target head/state guards so that
recovery derives them from the selected target after admission.

CaptureGroup input is strict JSON and requires `--value-reason`. A resolved
primary plus one related secondary has this shape:

```text
workvcs id new --kind capture-group
```

Use the returned UUIDv7 as `capture_group_id`; do not borrow another typed ID
or generate an unvalidated UUID. The group file then has this shape:

```json
{
  "capture_group_id": "01a00000-0000-7000-8000-000000000001",
  "primary_project_ref": "01a00000-0000-7000-8000-000000000002",
  "primary_locator_evidence_digest": null,
  "canonical_record_local_id": "canonical-finding",
  "members": [
    {
      "project_ref_id": "01a00000-0000-7000-8000-000000000002",
      "role": "primary",
      "relation": "canonical_owner",
      "delivery_mode": "canonical"
    },
    {
      "project_ref_id": "01a00000-0000-7000-8000-000000000003",
      "role": "related",
      "relation": "related_context",
      "delivery_mode": "immutable_reference"
    }
  ]
}
```

The canonical local ID must name exactly one Record in the semantic payload.
Reusing the manifest idempotency key reuses an intent only when both the
semantic payload and complete CaptureGroup (including `null`) are identical;
any drift fails as `capture_idempotency_conflict`. Admission does not deliver
the payload into a target Store. Delivery is a separate, explicit
`project operation-recovery --apply` action using fresh status digests.

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

With registry v2, the cwd form requires the exact `plan_admit` capability,
persists a `plan_admit_v1` intent in the shared durable-operation journal, and
drives delivery synchronously. Success output includes
`durable_route=projectref_journal`, the recovery-compatible `capture_id`, and
`delivery_receipt=true`. Explicit `STORE --branch` and registry-v1 cwd retain
their direct compatibility behavior.

The live evolve syntax is:

```text
workvcs plan evolve [OPTIONS] --manifest <PATH> <STORE|--cwd <PATH>>
```

Its manifest `mode` is `in_place` or `supersede`. In-place evolution updates
only explicitly supplied Plan fields, preserves omitted fields, atomically appends
Tasks with AC/VR, Records, and Evidence, and does not implicitly delete or
replace omitted state. Expected guards, target Plan identity/version/digest,
and idempotency are manifest fields.

With registry v2, the cwd form requires `plan_evolve`, persists
`plan_evolve_v1`, and uses the same receipt recovery. Repeating an identical
manifest is idempotent. A process failure after the Store commit but before
the receipt is repaired by repeating the exact command or by
`project operation-recovery` (`capture-recovery` compatibility alias); neither
route creates another Plan commit.

The routed Plan path performs pure typed-manifest validation, explicit
current-snapshot guard comparison, and a fixed maximum-timestamp receipt-
envelope size check before any first Store write. Proven oversize, guard
conflict, or manifest rejection records `plan_receipt_too_large`,
`plan_target_conflict`, or `plan_manifest_rejected` with no Store mutation.
Terminal manifest/target disposition precedes receipt construction; wrong-kind
Goal/Plan references are target conflicts. Timestamp precision is capped at
nine fractional digits, making the preflight envelope an append-safe maximum.
These captures are preserved for audit but are not replay candidates; start a
new operation with a smaller manifest, current guards, or corrected content.
Engine, storage, integrity, transaction, control-plane, or post-commit
uncertainty remains nonterminal and requires status-first recovery of the same
operation. Receipt and failure event families are validated against their
admitted payload kind during append and reconstruction. Plan receipts also
carry the exact operation kind and must match the complete result shape of the
admitted create/existing-Goal or in-place/supersede manifest. Once a receipt
is durable, command output is reconstructed through a read-only Store lookup.
Receipt preflight canonicalizes Plan record aliases exactly as the Store does
(`unknown` to `question`). A terminal failure is immutable for the current
delivery attempt: a later failure or receipt is rejected before event install.

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
