# ProjectRef Control Plane v2 Contract

Status: Accepted mixed contract — ADR-0513/0516/0517/0518 implemented; ADR-0519 sections implementation pending
Date: 2026-09-23
Last updated: 2026-10-08
Parents: [ADR-0513](../decisions/adr/0513-projectref-durable-capture-routing.md), [ADR-0516](../decisions/adr/0516-projectref-plan-durable-operation-routing.md), [ADR-0517](../decisions/adr/0517-shared-project-binding-isolation.md), [ADR-0518](../decisions/adr/0518-operator-control-plane-health-and-recovery-contract.md), [ADR-0519](../decisions/adr/0519-authorized-existing-binding-delivery-and-operation-inventory.md)

## Contract language and scope

`MUST`, `MUST NOT`, `SHOULD`, and `MAY` are normative within this contract.
This document specifies the logical and serialized control-plane contract; it
does not change the current SQLite Store schema. ADR-0513 accepts this target
design. ADR-0519 separately accepts only the sections explicitly marked
implementation-pending; those sections are not part of current source or
installed-runtime claims. The core now contains the strict registry v2 model,
pure ranked resolver, CaptureIntent model, and atomic/idempotent intent
admission foundation. The core also contains the generic `ContextLocatorProvider`
invocation and deterministic unified locator-input assembly. The CLI contains
digest-locked v1-to-v2 apply and v2-to-v1 rollback paths plus
a read-only rollback state probe in addition to preview. Core and CLI now also
contain the registry-derived journal-quiescence candidate: routed admission can
revalidate eligibility while holding the same lock that rollback holds from
its first journal inspection through replacement verification. It also
contains versioned ordinary read consumers, a strict tool-neutral
semantic-locator envelope, and a digest-bound v2 read-routing activation
candidate. The actual `capture` caller now assembles durable-write locator
input and admits a target-neutral v1 intent or a v2 registry-coupled intent.
Registry-v2 `plan admit --cwd` and `plan evolve --cwd` use the same physical
journal and delivery state machine with distinct Plan payload kinds; Plan
never enters a cognition manifest.
V2 admission is guarded by its own exact-snapshot marker and requires the
read-routing marker first. Its apply and disable operations share the registry
then quiescence lock order with rollback. V2 list/status can inspect an
inactive registry; normal reads and journal admission fail closed until their
respective exact markers are present. All mutating migration, activation,
rollback, admission, recovery/bootstrap, and concurrency fault verification
has run against isolated fixtures; separately authorized bounded live local
migration, activation, admission, and delivery canaries are also complete. The
implementation persists immutable event
chains, rebuilds deterministic projections, reports recovery status without
writing, and exposes a separately explicit registry/projection-digest-locked
recovery apply. It can establish one pristine
Store/Workspace/Branch binding and then, on the same explicit operator path,
materialize one guarded primary manifest through the existing atomic capture
engine. `delivery_started`, `delivery_applied`, and bounded
`delivery_failed` events make the target commit and missing-receipt window
recoverable without duplicate objects. The CaptureGroup implementation also
derives CaptureGroup state, installs immutable secondary-project references,
retries only missing references, records an idempotent group-completion
receipt, and recalls associations by secondary ProjectRef without opening a
semantic Store. The concrete adapter
`codex-app-project-metadata/v1`, in the CLI integration layer behind the
generic provider interface. It consumes only a bounded, command-local trusted
handoff, keeps authoritative Project metadata distinct from mirror-derived
evidence, and leaves the core provider-neutral. It also closes the explicit
stronger-locator attachment contract: an eligible unclaimed semantic or
repository locator can promote one selected provisional ProjectRef without
moving its target, while an already claimed key returns
`locator_already_claimed` without reassignment.

On 2026-09-27 the configured local registry was already migrated and both
exact activation markers were active. The installed public `capture` route
admitted a resolved `cognition_v2` CaptureGroup, explicit recovery committed
one canonical Record to work-governance, control-plane recall exposed one
immutable association from Hernes without opening its Store, and fresh-digest
replay wrote nothing. The prior legacy intent and Hernes Store remained
byte-identical. See the
[live primary and CaptureGroup canary evidence](../provenance/projectref-live-primary-and-capture-group-canary.md).
Live rollback was neither required nor performed.

For a standard registry filename, rollback treats the home-root and
registry-sidecar activation/journal paths as aliases of the same control plane
and checks both regardless of the caller's selection input. Apply and rollback
are currently Unix-only. The implementation derives one
quiescence lock from the canonical registry path for every journal alias and
passes both admission-first and rollback-first isolated concurrency fixtures.
This closes the race contract; it does not by itself activate admission or
authorize live rollback.

The v2 control plane has three distinct authorities:

1. the registry maps stable ProjectRefs and locators to Store targets;
2. the durable-operation journal preserves typed routing intent and delivery
   provenance; and
3. each target Store remains the only semantic authority for the Work-State
   objects committed to that Store.

Neither the registry nor the journal is a second Work-State database.

## Placement

The existing configuration precedence remains:

1. command-local explicit registry;
2. `WORKVCS_HOME`;
3. XDG configuration containing `home` or `registry`.

The effective registry path remains the configured registry path, including
the current default `project-bindings.json`. The control-plane root is:

| Effective configuration | Control-plane root |
| --- | --- |
| A WorkVCS home is known | The canonical WorkVCS home |
| Only a registry path is known | `<canonical-registry-path>.d` |

The durable-operation journal retains its compatible physical location,
`<control-plane-root>/capture-journal/v1`. The registry, journal, and default
Store roots MUST remain outside every resolved project boundary. A registry
sidecar does not infer a Store root; the existing explicit Store-root rule for
registry-only bootstrap remains.

The admission/rollback quiescence identity is
`<canonical-registry-path>.journal-quiescence.lock`. It is deliberately beside
the registry rather than below either capture-journal root, so configured-home
and registry-sidecar journal aliases cannot acquire different locks for the
same registry. Callers MUST canonicalize an existing registry before deriving
this path; a noncanonical, missing, or relative registry path is ineligible.
The registry-coupled journal constructor accepts only that canonical registry
plus a closed alias choice: standard-registry home or registry sidecar. It
derives the journal root itself, and callers cannot supply an arbitrary root
that rollback would not inspect.

The v2 read-routing activation marker is
`<control-plane-root>/routing-activation-v1.json`. Absence means disabled. The
strict marker carries activation version, scope
`project_ref_v2_read_routing`, registry ID, registry revision, and canonical
registry digest. It is an exact-snapshot gate, not a second registry or
identity authority. A stale, malformed, symlinked, or mismatched marker fails
closed. This scope enables only ordinary ProjectRef-v2 reads; it does not
authorize capture-journal delivery or Store mutation.

The separate v2 journal-admission marker is
`<control-plane-root>/journal-admission-activation-v1.json`. Its strict scope
is `project_ref_v2_journal_admission`, and it carries the same exact registry
ID/revision/digest lock. Absence is off. Apply requires active exact
read-routing first; admission rechecks both markers while holding the shared
quiescence lock. Disable requires the exact installed marker digest and holds
the registry lock followed by the shared quiescence lock through durable
removal verification. A stale, malformed, wrong-scope, mismatched, or
symlinked marker admits nothing.

Marker version 2 also carries the strictly sorted capabilities
`cognition_capture`, `plan_admit`, and `plan_evolve`. A historical version-1
marker remains valid only for `cognition_capture`; it never silently gains
Plan authority. The existing path name is retained so there cannot be two
marker authorities. An exact version-1 marker can be replaced for the same
registry ID, revision, and digest only through apply with its exact installed
digest and a strict capability-superset candidate. This marker authorizes
typed intent persistence only; the caller's explicit mutation command or a
separate recovery apply supplies target-delivery authority. Marker presence
alone never authorizes ProjectRef bootstrap, journal-event processing, or a
target Store write.

The canonical `project operation-recovery` surface recovers all typed
durable-operation intents and is separate from both markers. The visible
`project capture-recovery` spelling is a compatibility alias over the same
command and persistent state. `--status`
reports `payload_kind`, reconstructs authority from one immutable intent and
its events, and performs no write. `--apply` requires the exact current
registry digest and reconstructed projection digest, then acquires the
registry lock, the registry-derived quiescence lock, and per-capture event
locks in that order. It is an explicit operator route, not an automatically
activated route and not authority to operate on another live control plane. A
registry-only configuration must also supply `--store-root` for unbound
cognition bootstrap. Public Plan commands synchronously drive this same state
machine after their typed intent is durable; a failure still resumes through
the same recovery surface. Before any first Plan Store mutation, the
recovery route performs pure typed-manifest validation, compares explicit
target guards against one replayed snapshot, and materializes the typed
receipt in a fixed maximum-timestamp event envelope. Only those proven
manifest/guard failures and an envelope above the exact byte limit become
durable terminal results. A terminal manifest or target disposition is
persisted before receipt materialization is attempted; an entity that exists
under the wrong Goal/Plan kind is an explicit target conflict rather than a
retryable lookup error. An existing idempotent target result is recognized
before a now-advanced head is treated as conflict. Storage, integrity, engine
execution, transaction commit, and uncertain post-commit failures remain
recoverable or indeterminate instead of being classified by generic error
code. Journal reconstruction also validates receipt/failure families against
the intent payload kind and exact manifest-derived result shape. The timestamp
scalar permits at most nine fractional digits, so the fixed nanosecond
preflight envelope is a type-level maximum for the later append.

The composite `project health` inspection loads the selected registry once,
performs one complete validation of every binding, and then reports registry,
read-routing, journal-admission, all marker capabilities, and optional CWD
resolution state. Its `healthy`, `degraded`, and `blocked` classifications are
observations only. `--require-healthy` turns a non-healthy observation into a
stable failure, and `--timings` adds diagnostic observations without creating
a performance acceptance threshold. Health never migrates, activates,
refreshes, admits, recovers, opens a Store writable, or changes control-plane
or Store state. Full integrity validation is retained; a selected CWD binding
is not reopened after the one complete registry pass.

## Common scalar types

| Type | Contract |
| --- | --- |
| `Id` | Canonical UUID string. New ProjectRef, locator, link, capture, group, event, and delivery IDs use UUIDv7. |
| `Timestamp` | RFC 3339 UTC with an explicit `Z` and at most nine fractional digits. |
| `Digest` | Lowercase `blake3-256:<64 hex>` over the field-defined bytes; semantic objects use canonical bytes, while `backup_digest` deliberately uses exact raw file bytes. |
| `CanonicalPath` | Absolute, filesystem-canonical path at the time it was verified. It is a locator or target attribute, never a ProjectRef identity. |
| `NonEmptyString` | UTF-8 string containing at least one non-whitespace character. |

Canonical JSON uses UTF-8, sorted object keys, no insignificant whitespace,
and a terminating newline when stored as a file. Digests exclude the
terminating newline.

## Registry v2 document

The registry is one JSON object with exactly these top-level fields:

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `version` | integer, exactly `2` | yes | Registry format discriminator. |
| `registry_id` | `Id` | yes | Stable identity created at migration or first v2 initialization. |
| `revision` | non-negative integer | yes | Increments by exactly one for each accepted replacement. |
| `projects` | array of `ProjectRef` | yes | Stable logical project identities. |
| `locators` | array of `ProjectLocator` | yes | Namespaced identity and context locators. |
| `bindings` | array of `ProjectBinding` | yes | ProjectRef-to-Store routing targets. |
| `links` | array of `ProjectLink` | yes | Explicit or candidate project associations. |
| `observations` | array of `RegistryObservation` | yes | Non-authoritative audit observations. |
| `migration` | `MigrationReceipt` or `null` | yes | v1 provenance, if any. |

Unknown top-level or object fields MUST fail validation. Arrays are serialized
in stable ID order; their order has no semantic meaning.

### ProjectRef

| Field | Type | Required | Rules |
| --- | --- | --- | --- |
| `project_ref_id` | `Id` | yes | Stable identity; never derived at read time from a path or display name. |
| `maturity` | `provisional` or `established` | yes | `provisional` means CWD is the strongest active identity locator. |
| `display_name` | string or `null` | yes | Human label only; not unique and never used for resolution. |
| `created_at` | `Timestamp` | yes | First durable registry creation. |
| `created_by` | `explicit`, `migration`, `semantic_locator`, `repository_first_write`, or `cwd_first_write` | yes | Auditable creation route. |

V2 does not define automatic ProjectRef deletion, merge, or retirement. A
ProjectRef remains addressable even when locators are later retired. Any such
lifecycle needs a separate accepted decision.

`maturity` is `established` when at least one active identity locator has
authority `semantic_project` or `repository`, or creation was an explicit
ProjectRef operation. Attaching an eligible stronger locator may promote
`provisional` to `established`; it never changes `project_ref_id` or binding.

### ProjectLocator

| Field | Type | Required | Rules |
| --- | --- | --- | --- |
| `locator_id` | `Id` | yes | Stable locator-record identity. |
| `project_ref_id` | `Id` | yes | Existing ProjectRef. |
| `role` | `identity` or `context` | yes | Only `identity` participates in owner selection. |
| `authority` | `semantic_project`, `repository`, or `cwd` | yes | Core resolution rank. |
| `provider` | `NonEmptyString` | yes | Tool-neutral provider family, for example `chatgpt`, `git`, or `filesystem`. |
| `namespace` | `NonEmptyString` | yes | Tenant, workspace, account, provider instance, or stable local-installation namespace. |
| `kind` | `NonEmptyString` | yes | Provider-defined locator kind, for example `project_id` or `git_common_dir`. |
| `normalized_value` | `NonEmptyString` | yes | Provider-normalized value. Display names are forbidden here. |
| `assurance` | `authoritative`, `verified_derived`, or `observed` | yes | Evidence strength. `observed` is context-only. |
| `source_adapter` | `NonEmptyString` | yes | Adapter ID and version that produced or verified the locator. |
| `evidence_digest` | `Digest` | yes | Digest of bounded verification evidence; raw opaque provider payload is not stored. |
| `observed_at` | `Timestamp` | yes | Verification time. |
| `state` | `active` or `retired` | yes | Retired locators do not resolve ownership but remain auditable. |

An active identity locator key is the tuple `(provider, namespace, kind,
normalized_value)`. It MUST belong to at most one ProjectRef. The same key MAY
appear as context for more than one ProjectRef because a repository or
directory may participate in multiple semantic Projects.

Additional rules:

- `semantic_project` identity locators require `authoritative` or
  `verified_derived` assurance.
- `repository` identity locators require a verified canonical Git common
  directory.
- `cwd` identity locators require a verified canonical directory and always
  yield a provisional ProjectRef at creation.
- Core `repository` and `cwd` path locators use
  `namespace=registry:<registry_id>` so migrated and newly observed local-path
  identities resolve under the same stable control plane.
- `observed` locators MUST use role `context`.
- A semantic external ID without a non-empty provider namespace is invalid.
- Attaching an unclaimed stronger identity locator to an existing provisional
  ProjectRef requires either explicit user selection of that ProjectRef or a
  deterministic adapter proof whose evidence digest is retained.
- If the key is already claimed by a different ProjectRef, attachment fails as
  `locator_already_claimed`; no merge or reassignment occurs.
- A migration ownership repair may install one verified semantic locator as
  the active identity while retaining the exact v1 path locator and any
  distinct root locator in `retired` state. This exception requires the strict
  source/key/target/evidence-bound manifest defined by the migration contract;
  it does not permit deletion, reassignment, target changes, or path inference.

### ProjectBinding

| Field | Type | Required | Rules |
| --- | --- | --- | --- |
| `project_ref_id` | `Id` | yes | Exactly one binding per ProjectRef. |
| `store_path` | `CanonicalPath` | yes | Existing external Store path. |
| `store_id` | canonical Store ID | yes | Must match the opened Store. |
| `workspace_id` | canonical Workspace ID | yes | Must exist in the Store. |
| `branch_id` | canonical Branch ID | yes | Must belong to the Workspace. |
| `bound_at` | `Timestamp` | yes | Initial or migrated binding time. |
| `binding_source` | `explicit`, `migration`, `first_write`, or `isolation` | yes | How the target was selected. |

Every routable ProjectRef MUST have one binding. A binding is accepted for use
only after the current complete Store identity, format, schema, Workspace,
Branch, and integrity checks pass.

More than one ProjectRef MAY point to the same target tuple. That coincidence
does not make the ProjectRefs aliases and MUST be exposed by audit. The
registry MUST NOT silently deduplicate such bindings.

A newly created ProjectRef receives a dedicated Store, Workspace, and initial
Branch under the configured external Store root by default. Sharing an
existing target requires an explicit bind operation; migration may preserve a
pre-existing shared target without endorsing it as the new-project default.

An exact shared target may be separated only through the ADR-0517
preview/digest-locked isolation operation. It preserves the ProjectRef and old
Store, creates or reuses a deterministic pristine dedicated Store, changes one
binding, advances the registry revision, and records `binding_source=isolation`.
It never copies the old Store or removes the historical shared-target
observation. Ordinary activation markers become stale and require separate
exact-digest refresh after the registry change.

### ProjectLink

| Field | Type | Required | Rules |
| --- | --- | --- | --- |
| `project_link_id` | `Id` | yes | Stable link identity. |
| `from_project_ref` | `Id` | yes | Existing ProjectRef. |
| `to_project_ref` | `Id` | yes | Different existing ProjectRef. |
| `relation` | `artifact_repository`, `execution_context`, `related_work`, or `same_logical_project` | yes | Control-plane relationship only. |
| `status` | `candidate` or `confirmed` | yes | Candidate links never affect routing. |
| `basis` | `explicit_user`, `adapter_evidence`, or `capture_group` | yes | Why the link exists. |
| `evidence_digest` | `Digest` | yes | Bounded basis digest. |
| `created_at` | `Timestamp` | yes | Audit timestamp. |

Only a link with `relation=same_logical_project`, `status=confirmed`, and
`basis=explicit_user` may support a future consolidation workflow. V2 defines
no consolidation and never moves records because such a link exists.

Project links are not Work-State relations. They cannot connect Goal, Plan,
Task, Record, Knowledge, or execution graphs across Workspaces.
No ProjectLink, including confirmed `same_logical_project`, changes owner
selection in v2.

### RegistryObservation

V2 initially defines only `possible_shared_target`:

| Field | Type | Required | Rules |
| --- | --- | --- | --- |
| `observation_id` | `Id` | yes | Stable audit identity. |
| `kind` | exactly `possible_shared_target` | yes | Non-authoritative classification. |
| `project_refs` | array of at least two `Id` values | yes | Distinct ProjectRefs sorted by ID. |
| `target_digest` | `Digest` | yes | Digest of the equal Store/Workspace/Branch tuple. |
| `observed_at` | `Timestamp` | yes | Detection time. |
| `source` | `migration` or `audit` | yes | Observation source. |

An observation never changes resolution, creates a ProjectLink, or proves one
logical project.

### MigrationReceipt

| Field | Type | Required | Rules |
| --- | --- | --- | --- |
| `from_version` | integer, exactly `1` | yes | Source registry version. |
| `source_digest` | `Digest` | yes | Canonical v1 registry digest. |
| `preview_digest` | `Digest` | yes | Digest confirmed at apply. |
| `applied_at` | `Timestamp` | yes | Replacement time. |
| `backup_path` | `CanonicalPath` | yes | Verified v1 backup outside project boundaries. |
| `backup_digest` | `Digest` | yes | Digest of the exact raw v1 backup file bytes; unlike `source_digest`, this detects byte-level reformatting or replacement. |
| `mappings` | array of `MigrationMappingReceipt` | yes | Exactly one durable v1-key-to-ProjectRef receipt per migrated binding. |

`MigrationMappingReceipt` contains `v1_identity_kind`, canonical
`v1_identity`, and the created `project_ref_id`. Rows are unique and sorted by
the v1 key, cover every migrated ProjectRef exactly once, and may use only the
supported v1 kinds `git-common-dir` and `cwd`.

For a repair-aware candidate, `preview_digest` transitively binds the exact
repair-manifest digest. The installed active/retired locator states and their
evidence digests provide the durable semantic result; the receipt does not
turn the external manifest into a second registry authority. The raw backup
digest and canonical source digest intentionally cover different properties:
exact recovery bytes and equivalent v1 registry meaning, respectively.

## Resolver contract

### Input

The resolver consumes a `ResolutionContext` containing:

- optional explicit `project_ref_id`;
- zero or more adapter `LocatorEvidence` values;
- optional verified Git common directory;
- optional canonical CWD; and
- an operation mode of `read_only` or `durable_write`.

`LocatorEvidence` has the same namespace, kind, value, assurance, adapter, and
evidence-digest fields as `ProjectLocator`, but is not persisted merely by
being observed.

### Output

`ResolutionResult` contains:

| Field | Values | Meaning |
| --- | --- | --- |
| `status` | `resolved`, `unbound`, `unresolved`, or `conflict` | Whether a primary owner is safe and already represented. |
| `primary_project_ref` | `Id` or `null` | Present only when resolved. |
| `primary_basis` | explicit rank and locator evidence or `null` | Auditable winner or unbound highest owner. |
| `related_project_refs` | array | Lower-ranked mapped contexts with relation hints. |
| `unmapped_locators` | array | Valid context with no current ProjectRef. |
| `diagnostics` | array of stable codes | Mismatch, conflict, or fallback information. |

### Algorithm

1. A valid explicit ProjectRef wins. Other evidence is retained as context. An
   explicitly named missing or invalid ProjectRef fails closed.
2. Otherwise, eligible semantic identity evidence is considered whether or
   not it is mapped. Within one provider/namespace, `authoritative` outranks
   `verified_derived`.
3. Otherwise, eligible verified Git common-directory evidence is considered
   whether or not it is mapped.
4. Otherwise, eligible canonical-CWD evidence is considered whether or not it
   is mapped.
5. An eligible but unclaimed higher-rank locator yields `unbound` and blocks
   fallback to every lower rank.
6. In `durable_write`, exactly one unclaimed winning semantic or Git locator
   MAY create an established ProjectRef, and exactly one unclaimed winning CWD
   MAY create a provisional ProjectRef, but only after the capture intent is
   durable.
7. In `read_only`, an unclaimed winning locator remains `unbound` and creates
   nothing.
8. After provider-local assurance precedence, distinct winners at the same
   effective rank return `ownership_conflict`. Multiple unmapped winners are
   distinct; null ProjectRef values do not make them one owner. No target Store
   is selected.
9. Authoritative semantic metadata that disagrees with a verified-derived
   semantic locator wins and emits `context_mismatch`; the losing locator is
   retained only as context/candidate evidence.

This algorithm selects the primary owner. It does not decide whether the
content deserves capture.

## ContextLocatorProvider contract

An adapter implements this logical interface:

```text
provider_id() -> provider-and-version
locate(bounded_context) -> zero-or-more LocatorEvidence values
explain(evidence_digest) -> bounded non-secret verification summary
```

An adapter MUST be read-only, deterministic for the same bounded context, and
explicit about normalization and assurance. It MUST NOT create a ProjectRef,
binding, Store, CaptureGroup, or semantic object. Core WorkVCS validates its
output and applies the resolver algorithm.

The generic core interface is implemented. Adapter context is limited to 64
KiB, explanation summaries to 4 KiB each, and one collection to 128 evidence
items. Context and summaries must be JSON objects without secret-bearing field
names. Core verifies provider/source identity, semantic-project authority,
namespace eligibility, evidence-digest/explanation pairing, and canonical
ordering before producing `ResolutionContext`. Repository and CWD evidence are
accepted only through their separate verified path inputs, not through an
adapter claiming those authorities.

The source-tree CLI accepts the same verified semantic evidence through
`--locator-context FILE` on discovery, recall, resume, and currentness audit,
with optional `--project-ref ID`. The strict envelope is bounded by the same
64 KiB non-secret object gate and has this tool-neutral shape:

```json
{
  "schema_version": 1,
  "semantic_locator_evidence": [
    {
      "authority": "semantic_project",
      "provider": "provider-name",
      "namespace": "tenant-or-installation",
      "kind": "project_id",
      "normalized_value": "project-value",
      "assurance": "authoritative",
      "source_adapter": "provider-adapter/v1",
      "evidence_digest": "blake3-256:<64 hex>"
    }
  ]
}
```

This is an integration input for evidence an adapter has already verified; it
does not turn arbitrary caller text into authoritative evidence. Registry v1
rejects these higher-order inputs because it cannot safely map them. Under v2,
the core validates the evidence and applies the same resolver used by direct
provider invocation.

The CLI integration layer now includes
`codex-app-project-metadata/v1`. It accepts a bounded trusted handoff containing
verified task Project metadata and/or a canonical Project-mirror path,
normalizes both into `LocatorEvidence`, and invokes the same generic
`ContextLocatorProvider` path as any future adapter. Another tool can implement
the interface with its own authoritative container metadata. The core schema,
invocation, resolver, and persisted registry do not mention Codex or ChatGPT.
Adapter absence produces no semantic evidence and permits verified Git/CWD
fallback; an explicitly supplied malformed or unverifiable handoff fails
closed. Raw handoff context, thread IDs, and verification summaries are not
persisted in the registry, journal, or semantic Store.

Every integration, including the WorkVCS Skill, MUST run its value gate before
binding resolution. Once content is admitted as valuable and the operation is
not read-only/no-record, the integration submits it to the journal-backed
routing path even when discovery reports an unbound project. It MUST NOT
translate `project_binding_not_found`, `unbound`, or adapter absence into a
no-record decision.

The journal guarantees submitted intent, not unobserved cognition. Because the
initial scope excludes a global Hook, the system MUST describe this as
“no silent loss after admission” rather than “every valuable thought is always
captured.”

## Capture journal

### Filesystem layout

```text
capture-journal/v1/
  intents/<capture-id>.json
  events/<capture-id>/<sequence>-<event-id>.json
  projections/<capture-id>.json
  locks/<capture-id>.lock

<canonical-registry-path>.journal-quiescence.lock
```

An intent and each event are immutable after atomic installation. A projection
is a replaceable, reconstructable cache and has no independent authority.
The registry-derived quiescence lock serializes the unique idempotency-key
check and atomic intent installation across different capture IDs and also
excludes registry rollback. The core's standalone journal constructor retains
a journal-local `locks/intent-admission.lock` for isolated foundation use, but
that constructor is not eligible for ProjectRef-v2 routed admission; routed
callers MUST use the registry-coupled constructor. Per-capture locking
serializes event sequence allocation and projection replacement. Successful
writes sync the file and containing directory before success is reported.

### Admission and rollback quiescence protocol

The protocol is exclusive and fail-closed:

1. Routed admission derives the lock from the canonical registry, acquires it,
   and only then performs a mandatory core check that the registry is still v2
   with the expected revision and canonical digest. An optional caller check
   may further restrict routing/activation eligibility but cannot bypass that
   identity check; core repeats the exact identity read after the caller check
   before persistence. A failed post-lock check creates no journal layout or
   intent.
2. Rollback acquires the existing registry mutation lock first and the shared
   quiescence lock second. It holds both from the first activation/journal scan
   through the final durable replacement verification. Code that ever needs
   both locks MUST preserve this `registry -> quiescence` order.
3. If admission wins, it installs the immutable intent before releasing the
   lock; rollback then observes a non-empty journal and fails before registry
   replacement. If rollback wins, waiting admission either times out
   fail-closed or rechecks after the v1 restore and fails before persistence.
   Both sides MUST NOT succeed.
4. The lock file contains owner metadata. A guard removes it only when its
   bytes still match that guard, so it does not delete a replaced successor.
   A crash may leave an orphan. WorkVCS does not infer staleness from age or
   PID and never steals it automatically; admission and rollback fail closed,
   while `--rollback-check` reports `journal_quiescence_lock_state=present`.
   Recovery requires proving that no owner is live and removing that exact
   lock under a separately controlled operator procedure. No live recovery
   command is authorized or implemented in this slice.

### CaptureIntent compatibility envelope

| Field | Type | Required | Rules |
| --- | --- | --- | --- |
| `journal_version` | integer, exactly `1` | yes | Journal format. |
| `capture_id` | `Id` | yes | Stable routing identity. |
| `idempotency_key` | `NonEmptyString` | yes | Unique in this journal for the payload digest. |
| `created_at` | `Timestamp` | yes | Admission time. |
| `value_reason` | bounded `NonEmptyString` | yes | Why the typed operation is durable. |
| `payload_kind` | `cognition_v2`, `legacy_cognition_v1`, `plan_admit_v1`, or `plan_evolve_v1` | yes | Typed semantic contract carried by the shared durable-operation protocol. |
| `semantic_payload` | JSON object | yes | Bounded semantic manifest; never a full transcript. |
| `payload_digest` | `Digest` | yes | Digest of canonical `semantic_payload`. |
| `resolution_context` | `ResolutionContext` snapshot | yes | Bounded locator evidence used at admission. |
| `initial_resolution` | `ResolutionResult` | yes | May be unresolved or conflict. |
| `capture_group` | `CaptureGroupIntent` or `null` | yes | Required when more than one project context is intentionally in scope, including an unbound primary locator. |

The tuple `(idempotency_key, payload_digest, capture_group)` is replay-safe.
Here `capture_group` means the complete canonical CaptureGroup value or
`null`, not merely its ID. Reusing an idempotency key with a different payload
digest, adding or removing a group, or changing any group field fails as
`capture_idempotency_conflict`. This prevents a replay from silently changing
the canonical owner or secondary association set while retaining the same
semantic payload.

`cognition_v2` is target-neutral: it contains semantic items and local
relations but no target Branch head or state digest. The delivery attempt
reads and validates the selected Branch head, constructs exact target guards,
and records them in the delivery event.

`legacy_cognition_v1` preserves an existing v1 capture manifest exactly. It may
be delivered immediately when its target guards match the resolved binding.
If it remains pending and its guards become stale or were derived from a
different target, the intent stays durable with
`legacy_manifest_upgrade_required`; WorkVCS never silently drops, rebases, or
rewrites the original guarded manifest.

`plan_admit_v1` and `plan_evolve_v1` preserve the complete canonical Plan
manifest, including its target head/state and Plan/Goal/relation compare-and-
swap guards. Their journal idempotency keys are namespaced by payload kind;
their target Store idempotency keys remain the caller's manifest keys. Plan
payloads MUST NOT carry a CaptureGroup and MUST NOT be converted to cognition.
The existing Plan engines remain the only authority for Goal, Plan, Task,
Acceptance Criterion, Verification Requirement, Record, Evidence, and typed
relation semantics.

For registry v2, a public cwd-based Plan command admits this intent before
opening the target Store for mutation, then synchronously drives the common
delivery state machine. It reports command success only after a matching
`delivery_applied` receipt is durable. Repeating the command reuses the same
intent and target operation. A crash after the Store commit but before the
receipt converges by Store idempotency and appends only the missing receipt.
After a durable receipt exists, the command reconstructs its user-facing
result through the read-only idempotency lookup; it does not call the mutating
Plan engine a second time.

The semantic payload MUST be size-bounded by implementation policy, validated
before admission, and rejected if it contains known secret-bearing fields.
Callers remain responsible for not placing credentials or unbounded raw output
inside free-text semantic fields.

Validation before admission includes the complete target-neutral cognition
contract: Record, Knowledge, and Evidence construction; unique local IDs; and
relation type, endpoint-kind, label, and duplicate checks. Deterministic
semantic failure is `record_invalid` and occurs before journal layout or intent
installation. Target Branch guards remain a delivery concern.

### Explicit existing-binding continuation (accepted; implementation pending)

Default public cognition capture stops after durable admission. ADR-0519 adds
the optional `--deliver-existing-binding` continuation for `cognition_v2` only
when `capture_group=null`. The option is an explicit per-invocation authority
signal; neither binding health nor activation-marker presence implies it.

The intent is installed first. Continuation is eligible only for one resolved
ProjectRef whose complete binding passes full validation and whose exact
Store/Workspace/Branch target is not shared by another ProjectRef. The CLI then
drives the existing recovery apply path with the just-observed registry and
projection digests; recovery reacquires its normal locks and revalidates all
guards. Success requires a current delivery receipt and `recovery_action=none`.
Post-admission ineligibility or failure reports
`capture_delivery_incomplete` with the durable CaptureId and underlying cause.
It never bootstraps, isolates, changes a target, delivers a CaptureGroup, or
creates a second queue.

Marker/capability revalidation is scoped only to this same-command fast path
and occurs under the recovery lock window. It does not alter explicit recovery:
an already admitted operation remains manually recoverable under its exact
registry/projection guards when a routing or admission marker later becomes
inactive or stale.

When the idempotency key reuses an older intent, the fast path reads every
existing resolution, target-bearing event, and receipt before mutation. Every
prior resolved ProjectRef and complete target tuple must match the current
binding. A ProjectRef-only resolution recorded under a different registry
digest requires later same-chain authority proving the complete current tuple;
without that proof, the continuation fails before a new event or writable Store
open and requires a new Capture. The more general manual recovery contract is
unchanged; its ability to re-resolve some pre-delivery non-CaptureGroup intents
does not widen standing existing-binding authority.

The same accepted increment adds read-only
`project operation-recovery --list-open`. It loads one registry snapshot,
fully validates exactly once every distinct referenced binding needed for
classification, reconstructs operation state from immutable intent/event
authority across distinct journal aliases, and returns only operations whose
current recovery action is not `none`. Projections remain replaceable caches
and are not written by inventory. Filters, deterministic oldest-first ordering,
bounded rows, complete pre-limit counts, and explicit truncation keep the result
both actionable and bounded. An inventory digest over the complete matching
set locks later `--after-capture-id` pages; mismatch or an absent cursor fails
closed, so every row remains reachable without hiding concurrent change.
Rows expose only bounded machine metadata and digests. They never render raw
idempotency keys, semantic payloads, value reasons, locator/provider context,
target paths, free-text diagnostics, credentials, environment data, or raw tool
output. Listing supplies no recovery authority.

### CaptureGroupIntent

| Field | Type | Required | Rules |
| --- | --- | --- | --- |
| `capture_group_id` | `Id` | yes | Stable cross-project association. |
| `primary_project_ref` | `Id` or `null` | yes | Null only while ownership is unresolved. |
| `primary_locator_evidence_digest` | `Digest` or `null` | yes | Required exactly when the primary ProjectRef is not yet created. |
| `canonical_record_local_id` | `NonEmptyString` | yes | Exactly one Record item in the semantic payload that will anchor the group. |
| `members` | array of `CaptureGroupMember` | yes | Distinct ProjectRefs known at admission; may be empty while all project contexts are unbound. |

Each member contains `project_ref_id`, one role (`primary`,
`artifact_repository`, `execution_context`, or `related`), one relation label,
and requested delivery mode (`canonical`, `immutable_reference`, or
`none`). When `primary_project_ref` is known, exactly that member requests
`canonical`. When it is null, no member requests `canonical` until a
`capture_group_resolved` event installs the newly created primary member. A
cross-project group MUST NOT request more than one canonical delivery. The
canonical delivery MUST resolve `canonical_record_local_id` to a Record result;
other primary Records, Knowledge, Evidence, and relations stay
primary-project local.

Lower-ranked mapped ProjectRefs MAY become group members. Unmapped lower-ranked
locators remain in the resolution context and are not lost, but WorkVCS MUST
NOT create a secondary ProjectRef, Store, or reference solely to mirror ambient
context unless the capture intent explicitly requests that project delivery.

### CaptureEvent

Every event contains `journal_version`, `capture_id`, monotonically increasing
`sequence`, `event_id`, `occurred_at`, `previous_event_digest`,
`payload_digest`, `event_kind`, and an event-specific payload. Sequence starts
at one. The previous-event digest is null only for sequence one; every later
event chains to the canonical bytes of its predecessor. Filename sequence and
event ID must match the envelope, gaps and reordering fail closed, and the
payload digest detects uncoordinated mutation. The initial event vocabulary
is:

| Event | Meaning |
| --- | --- |
| `resolution_recorded` | A fresh resolver result, including conflicts and diagnostics. |
| `project_binding_ready` | The independently recoverable ProjectRef, locator, Store, Workspace, Branch, and registry bootstrap converged and its complete binding was revalidated. |
| `capture_group_resolved` | An unbound primary context was mapped to one ProjectRef and the group gained its single canonical member. |
| `delivery_started` | Target, exact Branch guards, delivery mode, and derived idempotency key were fixed. |
| `delivery_applied` | A Store returned an idempotent committed result. |
| `delivery_failed` | A bounded error and recovery action were recorded. Legacy upgrade remains recoverable; a deterministic invalid semantic manifest is terminal for that immutable intent. |
| `reference_applied` | An immutable secondary reference was installed. |
| `capture_completed` | All required deliveries have receipts. |

Events never contain credentials or raw Store files. A recoverable failed
event may be followed by a later retry. A `semantic_manifest_invalid` failure
is terminal for that immutable intent and requires a corrected new Capture.

### DeliveryReceipt and canonical record reference

A successful primary `delivery_applied` event records:

- `delivery_id` and `delivery_mode=canonical`;
- ProjectRef, Store ID, Workspace ID, and Branch ID;
- WorkStateCommit ID, ChangeSet ID, and resulting state digest;
- `operation_payload_kind=plan_admit_v1|plan_evolve_v1` for a Plan receipt,
  omitted for cognition;
- canonical object kind, logical object ID, immutable version ID, and version
  digest for each primary result; and
- `reused=true|false` from the target idempotency result.

For a v2 semantic payload, the target idempotency key binds the Capture ID and
the complete ProjectRef/Store/Workspace/Branch tuple. It therefore remains
stable for commit-before-receipt recovery on one target but cannot collide
with a later delivery after an explicit target change. A CaptureGroup also
preflights that `canonical_record_local_id` names a Record in the materialized
target manifest before the Store is opened for mutation.

For Entity and Relation results, the receipt uses the Store-native logical ID,
immutable version ID, and state/version digest. Evidence is already immutable:
its Evidence ID is both logical and immutable-version identity, while the
receipt digest is domain-separated over its canonical kind/metadata
descriptor. The receipt therefore names every capture result without turning
the journal into a second semantic database.

A Plan receipt's operation discriminator MUST equal the intent payload kind,
and its complete `local_id`/object-kind set MUST equal the result shape derived
from that exact manifest. Admission create-Goal versus existing-Goal and
evolution in-place versus supersede therefore have distinct receipt shapes;
partial or cross-family receipts cannot project `completed`. The discriminator
is absent for cognition receipts so their existing serialized contract remains
unchanged; version-1 activation never authorized Plan intents.
Domain aliases are canonicalized through the Plan engine's parser before both
preflight materialization and expected-shape derivation. In particular,
`record.kind=unknown` is receipted as `record:question`, matching the committed
Store result and its exact event-size cost.

`canonical_record_ref` is the tuple of ProjectRef, Store ID, Workspace ID,
logical Record ID, immutable Record version ID, and version digest. It is set
only from a verified primary receipt.

A fresh resolution event makes the prior binding receipt non-current. If the
new result still resolves to the same ProjectRef, delivery history is retained
but the recovery state is `pending_project` until binding is revalidated. An
exact Store/Workspace/Branch match restores the existing receipt without a
target call; any target mismatch clears the current delivery view and returns
to `pending_primary` for a capture without a canonical CaptureGroup result.
Once a CaptureGroup canonical receipt exists, changing its resolved primary or
target fails closed instead of creating a second mutable authority; a new
canonical delivery requires a new capture. Invalid state-machine transitions
are rejected before an immutable event file is installed.

An immutable secondary reference stores `capture_group_id`, secondary
ProjectRef, `canonical_record_ref`, relation, and observation time. It contains
no independently mutable copy of the canonical statement or lifecycle. If the
canonical Record later changes, the old reference remains pinned; a new
reference is required to expose the new version.

A project-local derived Record is a separate semantic delivery with a separate
Record ID and explicit `derived_from` provenance to the canonical version. It
is not represented as `immutable_reference` and never follows canonical
updates automatically.

### CaptureGroupProjection and secondary lookup

The current `CaptureGroupProjection` is reconstructed from the immutable
intent and events. It contains:

- `capture_group_id` and `capture_id`;
- resolved `primary_project_ref`;
- verified `canonical_record_ref`;
- every known member ProjectRef, role, relation, requested delivery mode, and
  current delivery receipt or pending diagnostic; and
- unresolved related locator evidence retained from the resolution context.

An `immutable_reference` is a control-plane index entry derived from the group
events and keyed by the secondary ProjectRef. The explicit read-only
`project capture-group-recall --project-ref-id ID` scans immutable
intent/event authority across the registry-derived journal aliases and exposes
that association without opening or mutating any semantic Store. The entry is
not a Store-local Record and is not a Work-State relation. If a secondary
Store needs semantic local content, it uses the distinct project-local derived
Record path described above.

The projection can be rebuilt and replaced; the intent and events are its
authority. It never becomes a second mutable copy of the canonical Record.

### Derived capture states

The current projection is reconstructed from intent plus events:

| State | Condition |
| --- | --- |
| `pending_resolution` | No safe primary ProjectRef exists. |
| `pending_project` | A highest-rank owner is known but its ProjectRef or valid binding is not ready. |
| `pending_primary` | Intent is durable but primary delivery has no receipt. |
| `pending_references` | Primary is applied and at least one required secondary reference lacks a receipt. |
| `legacy_manifest_upgrade_required` | A retained v1 manifest cannot safely target the current Branch guards. |
| `semantic_manifest_invalid` | A historical immutable intent fails deterministic cognition semantics and must be replaced by a corrected new Capture. |
| `plan_target_conflict` | The retained Plan manifest no longer matches current target guards; start a new operation with fresh guards. |
| `plan_manifest_rejected` | The retained Plan manifest deterministically failed validation before target mutation; correct it in a new operation. |
| `plan_receipt_too_large` | The exact next Plan receipt would exceed the journal event limit; split the manifest into a smaller operation. |
| `completed` | Primary and every required secondary delivery have verified receipts. |

Errors such as an unavailable Store, registry conflict, or failed reference
attempt are diagnostics on one of these recoverable states. There is no
generic state that means “silently skipped.”

The implementation materializes `resolution_recorded`,
`project_binding_ready`, `capture_group_resolved`, `delivery_started`,
`delivery_applied`, secondary `reference_applied`, `capture_completed`, and
the bounded legacy-staleness and semantic-invalid forms of `delivery_failed`.
It derives `pending_resolution`, `pending_project`, `pending_primary`,
`legacy_manifest_upgrade_required`, `semantic_manifest_invalid`,
`plan_target_conflict`, `plan_manifest_rejected`, `plan_receipt_too_large`,
`pending_references`, and `completed`.
`completed` is derived as soon as the primary and every required reference
receipt are authoritative; `capture_completed` is the idempotent summary
receipt. A crash between the last reference and that summary therefore reports
`completed` with next action `apply_capture_completion`, not a missing
delivery. A stored projection is only a cache:
absence, stale bytes, or malformed regular-file bytes are reported and can be
replaced from authority. A malformed event, identity/filename mismatch,
sequence gap, digest-chain break, payload-digest mismatch, symlink, or
non-regular authority entry fails closed and is never repaired by projection
rewrite.

### Post-intent ProjectRef/bootstrap convergence

Recovery always reruns the resolver against the exact locked registry. An
unresolved or conflicting result records fresh resolution and stays
`pending_resolution`; it creates no Store and never falls back. A resolved
ProjectRef must have its exact binding revalidated. One unbound winning
locator may create a ProjectRef and deterministic pristine Store target:

- semantic and repository first writes create `established` ProjectRefs;
- CWD-only first write creates a `provisional` ProjectRef;
- the target path is derived from the complete namespaced locator key, never
  from display name alone;
- a pre-existing target is adopted only when its marker, single Workspace,
  single initial Branch, and Genesis head are exactly pristine; and
- an already claimed locator is reused only with the same complete target;
  target substitution fails closed.

The registry replacement is one revision, locked and atomic. Failure after a
pristine Store exists but before registry replacement is safe forward
recovery: retry revalidates and reuses that Store. Failure after registry
rename, after `project_binding_ready`, or after projection replacement is
indeterminate and begins with read-only status. Recovery proceeds forward and
never automatically rolls the registry back. Once the binding event is
durable the derived state is `pending_primary`. Recovery records exact target
guards before calling the Store. If a Store commit becomes durable before its
receipt, the next apply uses the recorded manifest and idempotency key, reuses
the committed target result, and appends the missing receipt. A legacy
manifest with absent or stale guards records
`legacy_manifest_upgrade_required` without target mutation or intent rewrite.
A historical manifest that fails target-neutral cognition semantics records
`semantic_manifest_invalid` before the Store is opened for mutation. Its
intent is preserved, replay is idempotent, and recovery directs the operator
to a corrected new Capture.

## Cross-surface invariants

1. A registry-v2 project-routed durable target write MUST have a prior typed
   durable intent. Explicit `STORE --branch` and registry-v1 compatibility
   commands remain outside this routing boundary.
2. Journal failure before intent installation MUST prevent target writes.
3. ProjectRef resolution and semantic value admission are independent.
4. Read-only mode MUST NOT persist locator observations, ProjectRefs, journal
   intents, projections, or Stores.
5. One CaptureGroup has exactly one canonical semantic delivery.
6. Registry links and capture references MUST NOT become cross-Workspace
   Work-State relations.
7. Store commit without a journal receipt is recoverable by target
   idempotency; it is not treated as absent.
8. Equal binding targets, paths, names, or payload text MUST NOT automatically
   merge ProjectRefs.
9. A resolver diagnostic MUST expose the winning rank and the discarded or
   related evidence without leaking raw provider payloads.
10. Global Hooks and automatic per-turn capture are outside this schema.
11. An unbound higher-ranked locator MUST NOT fall through to a lower-ranked
    bound target.
12. A missing binding MUST NOT be interpreted as a failed value gate.
13. Historical ownership repair MUST preserve the exact target and retired v1
    locator; it MUST NOT behave as a path rewrite, merge, or Store move.
14. A stored projection is disposable and recoverable; immutable intent/event
    corruption is authoritative failure and MUST NOT be hidden by rebuilding.
15. Recovery/bootstrap MUST NOT broaden the journal-admission marker or make
    default `capture` continue past intent admission. The only accepted
    continuation is the explicit ADR-0519 existing-binding mode, and it MUST
    satisfy INV-111 without bootstrap, target substitution, or CaptureGroup
    delivery.
16. A version-1 journal marker authorizes cognition admission only. Plan
    capabilities require an explicit exact-digest strict-superset refresh;
    recovery MUST NOT perform that refresh implicitly.
17. A typed Plan operation MUST prove that its typed receipt in the fixed
    maximum-length timestamp envelope fits before the first Store write. Only
    pure manifest-validation failures and explicit current-snapshot guard
    mismatches become durable terminal events, and that terminal disposition
    MUST be recorded before any fallible receipt construction;
    engine/storage/integrity, transaction, or post-commit failures MUST remain
    eligible for status-first recovery.
18. A journal event MUST match its intent family. Cognition and Plan receipts,
    Plan admission/evolution result families, CaptureGroup events, and
    failure-code/recovery-action pairs cannot be interchanged. Plan receipts
    MUST also match the exact admitted manifest variant and appended-object
    shape.
19. A `delivery_failed` event is terminal for its current
    `delivery_started` identity. A later failure or receipt MUST NOT replace it;
    only an authorized re-resolution/rebinding that clears the complete
    delivery attempt may establish a new target attempt.

## Stable failure codes

The contract exposes machine-routable top-level errors and journal/status
conditions. Not every row is a top-level `ErrorCode`; the recovery projection
states and resolver conditions remain successful status fields where their
surface says so. In particular, historical `ownership_unbound` names the
routing condition, while ADR-0519 `project_owner_unbound` is the dedicated
top-level registry-v2 read error. Default capture/recovery status continues to
report `resolution_status=unbound` instead of either top-level error.

The future implementation MUST expose at least these machine-routable codes
and conditions:

| Code | Recovery |
| --- | --- |
| `capture_not_persisted` | Repair or select a writable control plane, then retry; no target write occurred. |
| `capture_delivery_incomplete` | The intent is durable but explicitly requested existing-binding delivery did not complete. Inspect the named CaptureId and underlying cause before any separately authorized recovery. |
| `ownership_unresolved` | Add/verify a locator or explicitly select a ProjectRef. |
| `ownership_unbound` (routing condition; not a top-level ErrorCode) | Admit a durable write to bootstrap the winning locator, or explicitly bind its ProjectRef. |
| `ownership_conflict` | Resolve same-rank candidates explicitly. |
| `context_mismatch` | Inspect authoritative and derived semantic context; primary may still be resolved. |
| `project_ref_not_found` | Correct the explicit ProjectRef; lower-ranked fallback was not attempted. |
| `project_owner_unbound` (top-level v2 read ErrorCode) | The winning owner is valid but has no ProjectRef binding. Admit durable work first, then separately authorize exact ProjectRef/bootstrap convergence; do not fall back or call a read path a delivery. |
| `locator_already_claimed` | Do not reassign automatically; explicitly link or resolve the ProjectRefs. |
| `project_binding_invalid` | Repair the exact target binding before delivery. |
| `registry_migration_required` | Run and inspect the read-only v1-to-v2 preview before separately authorizing apply. |
| `registry_migration_apply_failed` | The atomic replacement did not occur; inspect and correct the reported precondition or artifact conflict, then recompute both digest locks before retrying. |
| `registry_migration_install_indeterminate` | The atomic rename occurred but a later durability or verification step failed; inspect the installed registry and run the read-only rollback probe. Never infer or perform rollback automatically. |
| `registry_rollback_failed` | The rollback replacement did not occur. Correct the reported digest, receipt, activation, journal, backup, snapshot, lock, or temp conflict; the exact v2 snapshot may remain reusable. |
| `registry_rollback_install_indeterminate` | The v1 restore rename occurred but a later durability or verification step failed; run the read-only rollback probe with the same digests before retrying or choosing forward recovery. |
| `routing_activation_inactive` | The exact read-routing marker is absent. Inspect health/status and activate only through a separately authorized digest-locked apply. |
| `journal_admission_activation_inactive` | The exact journal-admission marker is absent. Inspect health/status and activate only through a separately authorized digest-locked apply. |
| `journal_admission_capability_inactive` | The active journal marker does not authorize the required capability. Inspect its version and explicitly refresh only through the exact installed-marker digest contract. |
| `routing_activation_install_indeterminate` | The marker installation completed or may have completed before later cleanup, directory sync, or verification failed; inspect the exact marker with `project routing-activation --status` before retrying or recovering. |
| `routing_activation_disable_indeterminate` | A journal-admission marker was removed or may have been removed before durable verification completed; inspect `project journal-admission-activation --status` before retry or recovery. |
| `capture_idempotency_conflict` | Use the original payload or a new idempotency key. |
| `capture_recovery_install_indeterminate` | Registry, event, or projection installation may already be durable; run `project operation-recovery --status` with the same capture and continue forward using the newly reported digests. The historical error name remains stable. Never infer rollback. |
| `shared_binding_isolation_install_indeterminate` | The registry replacement may already be durable; rerun `project isolate-shared-binding --preview` and continue only from `eligible`, `not_shared`, or verified `already_isolated`. Never copy the source Store or replay apply blindly. |
| `legacy_manifest_upgrade_required` | Convert the durable legacy intent to a separately confirmed target-neutral delivery; do not rewrite it implicitly. |
| `semantic_manifest_invalid` | Preserve the immutable intent, inspect the invalid semantic input, and start a corrected new Capture; do not replay or rewrite the old payload. |
| `plan_target_conflict` | Preserve the immutable operation and start a new Plan operation with current target guards. |
| `plan_manifest_rejected` | Preserve the rejected operation, correct the manifest, and admit a new Plan operation. |
| `plan_receipt_too_large` | Split the Plan manifest so its detailed receipt fits the journal event limit, then admit a new operation. |
| `capture_pending_references` | Primary is safe; retry missing secondary deliveries. |

## Illustrative resolution example

Suppose one task exposes:

- authoritative semantic locator
  `(chatgpt, account-A, project_id, g-p-123)`;
- verified mirror-derived locator
  `(chatgpt, account-A, project_id, g-p-123)`;
- Git common directory `/repos/workvcs/.git`; and
- CWD `/mirrors/g-p-123`.

The semantic ProjectRef is primary. A repository ProjectRef, if one already
exists, is retained as `artifact_repository`; the CWD is context. If the mirror
instead derives `g-p-999`, authoritative `g-p-123` remains primary and the
result carries `context_mismatch`. Neither Project ID is inferred from its
display name, and no locator is silently reassigned.
