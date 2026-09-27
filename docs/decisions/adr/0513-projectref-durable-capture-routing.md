# ADR-0513: ProjectRef Ownership and Durable Capture Routing

Status: Accepted and delivered locally — roadmap rounds 1–11 complete with bounded live canary evidence; no release, deployment, or global Hook
Date: 2026-09-23
Last updated: 2026-09-28

## Decision boundary

This ADR records the exact design accepted after the ownership and durability
direction was confirmed. The user accepted the schema, migration, failure, and
acceptance contracts on 2026-09-23. It supersedes the project-identity and
first-use routing semantics of ADR-0497 and ADR-0511, and extends ADR-0501's
standalone-cognition entry boundary.

The implementation reads registry v1 and v2 explicitly for ordinary project
operations. Registry-v2 routing uses the unified explicit
ProjectRef/semantic/Git/CWD resolver and fails closed unless the exact
registry-ID/revision/digest read marker is active. A strict tool-neutral
locator envelope supplies verified semantic evidence; the concrete desktop
Project adapter remains outside core. Migration, rollback, activation, and
journal admission retain separate commands, locks, digests, and authority
boundaries.

Public registry-v2 `capture` now admits target-neutral `cognition_v2`, rejects
caller target guards, and optionally accepts a strict CaptureGroup whose
complete value is part of idempotency identity. Admission remains Store-free.
Separately explicit recovery records fresh target guards, delivers exactly one
canonical primary result, converges missing immutable secondary references,
and exposes read-only recall from a secondary ProjectRef. Registry-v1
value-qualified admission retains `legacy_cognition_v1`; legacy direct capture
without `--value-reason` remains a bounded compatibility route.

On 2026-09-27 the exact local package from commit
`615f946c11ea21152616dfccffe13c612a8d8059` was installed, the previously
migrated registry and both exact activation markers were revalidated, and one
bounded live CaptureGroup completed. Work-governance received the sole
canonical Record, Hernes received only a control-plane immutable association,
replay wrote nothing, and the earlier terminal legacy intent stayed
byte-identical. Exact IDs, digests, event sequence, Store proofs, and remaining
boundaries are in the
[live primary and CaptureGroup canary evidence](../../provenance/projectref-live-primary-and-capture-group-canary.md).

The companion contracts are:

- [ProjectRef Control Plane v2 Contract](../../architecture/projectref-control-plane-v2.md)
- [Registry v2 Migration and Acceptance Contract](../../architecture/projectref-registry-v2-migration-and-acceptance.md)

## Context

The current project registry identifies work through a canonical Git common
directory or canonical CWD. That is a useful locator, but it is not always the
owner of the work. A desktop Project, another tool's semantic container, a
repository, and the process CWD can all be different contexts in one task.

This mismatch has already produced the failure mode that motivated the
decision: valuable reasoning was eligible for durable capture, but an unbound
ambient mirror caused the caller to conclude that no record should be made.
ADR-0511 reduced the bootstrap cost, but it intentionally left selection of
the logical project to governance. A CWD-only resolution rule can therefore
still lose durable content or place it under the wrong project.

The design must preserve these existing boundaries:

- value, not task size or the existence of a Plan, decides whether content is
  worth recording;
- read-only work and an explicit no-record choice remain zero-write;
- every semantic Work-State mutation still targets one Store, Workspace, and
  Work Branch;
- Goal, Plan, Task, and execution graphs do not become cross-Workspace graphs;
- WorkVCS remains the durable state provider, while authorization and capture
  policy remain outside it; and
- no repository, mirror, or CWD becomes authority merely because it is
  ambient.

## Decision

WorkVCS adopts the following target contract.

### 1. Stable ProjectRef, separate from locators and targets

`ProjectRef` is the stable control-plane identity of a logical project. It is
not a path, repository, Store, Workspace, Branch, provider display name, or
external Project ID.

A ProjectRef has zero or more namespaced locators and exactly one current
target binding once it is routable. A locator explains how a context was
recognized. A target binding explains where WorkVCS state for that ProjectRef
is written. Changing or adding a locator does not move, copy, or rewrite the
ProjectRef's Store content.

A CWD-only first write may create a `provisional` ProjectRef in the central
control plane. A later authoritative or verified locator may be attached to
that same ProjectRef when the locator is unclaimed and the evidence satisfies
the attachment contract. If a stronger locator already belongs to another
ProjectRef, WorkVCS does not merge them: it records a conflict or candidate
link and requires explicit resolution.

Display names are labels only. Neither equal names nor equal basenames imply
identity.

### 2. Deterministic ownership order

Resolution evaluates candidate ownership in this order:

| Rank | Candidate | Ownership rule |
| --- | --- | --- |
| 0 | Explicit WorkVCS ProjectRef override | A valid, existing ProjectRef selected explicitly for this operation wins. |
| 1 | Verified semantic Project/container locator | A namespaced external semantic identity wins over infrastructure context. Authoritative provider metadata outranks a verified-derived locator from the same provider. |
| 2 | Repository locator | The canonical Git common directory identifies repository-owned work when no higher owner exists. |
| 3 | CWD locator | The canonical directory is the fallback. It is provisional on first durable write. |
| 4 | No eligible candidate | The capture remains pending resolution; WorkVCS must not guess a target. |

A higher-ranked owner wins. Valid lower-ranked candidates are retained as
related context, not discarded and not silently promoted to aliases. Two
different candidates at the same effective rank and assurance fail closed as
`ownership_conflict` unless an adapter contract supplies a deterministic
precedence rule.

An eligible higher-ranked locator also blocks fallback when it is not yet
bound. For example, a verified semantic Project with no ProjectRef MUST NOT be
routed into an already-bound repository merely because that repository is
available. Read-only resolution reports the semantic owner as unbound; an
eligible durable write first journals its intent and then idempotently creates
the semantic ProjectRef and binding. An invalid explicit ProjectRef override
fails closed instead of falling back.

For a ChatGPT desktop example, current task/Project metadata carrying a
Project ID outranks a Project ID verified from the canonical local mirror
shape; either outranks Git identity, which outranks CWD. A mismatch between
authoritative metadata and a derived mirror identity is surfaced as
`context_mismatch`; the authoritative Project remains primary and the
mismatched context is not silently aliased. This is an example of the generic
rule, not a ChatGPT-specific core dependency.

### 3. Tool-neutral context adapters

Provider integration lives behind `ContextLocatorProvider`. An adapter emits
namespaced locator evidence; it does not create Work-State, select a Store, or
override the core ranking rules.

Every semantic locator includes provider, provider namespace or tenant,
locator kind, normalized value, assurance, source, and an evidence digest.
External IDs without a namespace are ineligible as identity locators. Raw
tokens, credentials, transcripts, or opaque provider payloads are not retained
as locator evidence.

Codex/ChatGPT may supply one adapter. Other desktop applications, IDEs,
orchestrators, or future tools can supply equivalent adapters. Without a
specialized adapter, the resolver still accepts a caller-supplied verified
semantic locator and otherwise degrades to Git, then CWD.

### 4. Central control plane and write-ahead capture journal

The effective WorkVCS configuration selects one external control plane. It
contains the registry, per-project Store bindings, and a first-class capture
journal. It does not replace per-project Stores with one monolithic Store.

When an effective WorkVCS home exists, it is the control-plane root. When the
configuration names only a registry, the deterministic sidecar
`<canonical-registry-path>.d` is the control-plane root. Both the registry and
sidecar remain outside project boundaries. Existing direct-registry Store-root
requirements remain unchanged.

After an external value/admission decision authorizes durable capture, the
capture intent is atomically persisted to the journal before any target Store
write. If ownership is unresolved, a Store is unavailable, or a later
cross-project delivery is incomplete, the intent remains recoverable and its
state is reported explicitly. If the journal itself cannot be durably written,
the operation fails as `capture_not_persisted` and performs no target write.

The journal persists only the bounded, redacted semantic manifest needed to
replay the capture. It is not a transcript archive and must not contain
credentials, environment dumps, or unrestricted tool output. Initial delivery
does not automatically delete an applied intent; retention or compaction is a
separate future decision.

Read-only discovery, recall, resume, inspection, and an explicit no-record
decision stop before journal admission and remain zero-write.

This ordering is normative for the WorkVCS Skill and every integration:
evaluate semantic value first, stop only for read-only/no-record, then submit
eligible content to durable routing. A missing ProjectRef or binding is a
routing state, never evidence that the content lacks value.

This guarantees recoverability only after an integration admits and submits
the semantic content. Without the separately deferred global Hook, it cannot
prove that a caller recognized every valuable thought. The initial contract
therefore eliminates binding-driven silent omission and makes submission
failures visible; it does not claim perfect capture completeness.

### 5. One canonical record with explicit cross-project association

When one capture concerns more than one ProjectRef, a `CaptureGroup` records:

- `capture_group_id`;
- `primary_project_ref`;
- `canonical_record_ref` after primary delivery; and
- related project members with role, relation, and delivery references.

The primary project owns the only canonical mutable semantic record. A
secondary project receives an immutable external reference pinned to the
canonical record version/digest when that projection is needed. If a secondary
project needs its own semantic conclusion, it creates a distinct local Record
with explicit `derived_from` provenance; it is not a synchronized copy and may
evolve independently.

CaptureGroup, routing attempts, and delivery receipts are control-plane
provenance. They do not form a cross-Workspace Work Graph, do not make a second
mutable semantic authority, and do not weaken INV-002 or INV-004.

### 6. Bounded atomicity and recovery

WorkVCS makes no false claim of one transaction across the registry, journal,
and multiple Stores.

- Each registry replacement is independently locked and atomic.
- Each journal intent or event is independently locked, immutable once
  admitted, and durably installed before it is reported successful.
- Each target Store capture is one existing atomic/idempotent WorkVCS
  transition.
- Cross-Store deliveries converge through idempotency keys and receipts.

The primary delivery uses an idempotency key derived from the capture ID and
the complete ProjectRef/Store/Workspace/Branch target identity. If the Store
commit succeeds but its receipt is not recorded, replay against that exact
target observes the same committed result and appends the missing receipt.
Secondary delivery failures do not roll back the canonical primary record;
they remain visible as pending deliveries and are retried independently.

The durability guarantee is bounded by a writable and healthy control plane.
Failures before durable intent installation are visible failures, never
reported as captured or silently skipped.

### 7. Explicit registry v1 to v2 migration

Registry v2 introduces ProjectRefs, locators, bindings, links, and migration
observations. Migration follows the companion contract:

1. a read-only preview produces a deterministic normalized mapping and digest;
2. apply requires that preview digest and an unchanged v1 source digest;
3. every v1 binding becomes exactly one ProjectRef with the same Store path,
   Store ID, Workspace ID, and Branch ID;
4. no Store, Workspace, Branch, project directory, or object is moved or
   rewritten;
5. bindings that happen to share a target remain distinct ProjectRefs and are
   reported only as `possible_shared_target` observations;
6. logical linking or consolidation requires a later explicit decision;
7. no historical CaptureGroup is invented;
8. migration holds the registry lock, retains a verified v1 backup, and
   atomically replaces the registry; and
9. read-only operations never migrate implicitly.

The default migration preserves the v1 identity as an active locator. A
separately supplied ownership-repair manifest may replace that active identity
only when current semantic Project evidence proves that the v1 CWD/repository
locator represented historical execution context rather than the logical
owner. The manifest is bound to the exact source-registry digest, exact v1
binding key, exact target digest, namespaced semantic locator, and evidence
digest. The semantic locator becomes active, the v1 identity (and distinct v1
root context) remains auditable but retired, and the target tuple remains
unchanged. No path substitution, ProjectRef merge, target move, or Store
rewrite is permitted. A stale, ambiguous, weak, duplicated, or unmatched
repair fails closed.

`--cwd` remains a supported locator input. After v2 it no longer defines the
only possible identity or automatically outranks a semantic Project.

### 8. Initial implementation boundary

The first implementation, if separately authorized, covers
registry v2, resolver inputs, the adapter interface, journal recovery, and
the WorkVCS Skill/integration ordering plus focused migration/acceptance tests.
It does not enable a global per-turn Hook. Such a Hook is an optional
completeness layer that requires its own evidence and confirmation after the
core path is stable.

The first bounded implementation slice installs only the core registry v2 data
model and validation, pure ownership resolver, CaptureIntent validation, and
atomic/idempotent intent admission. It intentionally leaves the current v1 CLI
behavior unchanged.

The second bounded implementation slice adds the tool-neutral
`ContextLocatorProvider` invocation and unified resolver-input assembly, plus
the explicit `project registry-migrate --preview` command. Adapter context,
evidence, and explanations are bounded, secret-bearing fields are rejected,
and Git/CWD inputs remain separately verified by core callers. Preview parses
registry v1 strictly, validates every target Store read-only, emits a stable
one-to-one mapping and digest, reports shared-target coincidences without
aliasing, and exposes no apply path. Provider-specific adapters, registry
apply or other mutation, v2 routing activation, journal events/projections and
recovery, target delivery, Store integration, installation, and Hook
activation remain outside these slices.

The third bounded slice adds only an optional, generic ownership-repair
manifest to that read-only preview. It changes no default preview behavior and
exposes no apply path. The repair preview validates source and target digests
before relaxing only the existence check for the retired historical path;
complete Store, Workspace, Branch, content, external-target, and schema
validation remains mandatory. A real repaired preview may establish
apply-eligibility, but it is evidence for a later confirmation gate, not
authority to migrate.

The fourth bounded slice adds the digest-locked apply candidate, exact raw-byte
backup proof, atomic replacement, pre/post-rename failure classification, and
read-only rollback-readiness probe. It is tested only on isolated fixtures.
It does not install the binary, migrate the live registry, make ordinary
commands consume registry v2, or activate routing.

The fifth bounded slice makes ordinary read-only consumers explicit about
registry v1/v2. V1 reads report `registry_version=1` and
`migration_required=true`. Registry-v2 discovery, recall, resume, audit, and
other cwd-based reads resolve through explicit ProjectRef, tool-neutral
semantic evidence, verified Git common directory, and verified CWD in that
order. An eligible unbound semantic locator fails closed instead of falling
through. V2 list and activation status remain inspectable while routing is
off. Normal v2 reads require a regular activation marker bound to the exact
registry ID, revision, and canonical digest. Marker absence is the default-off
state; stale, malformed, or mismatched markers fail closed. The candidate
apply is idempotent and atomic and has been exercised only in temporary
fixtures. Its scope is `project_ref_v2_read_routing`: it neither activates
journal/Store delivery nor authorizes a live migration or activation.
Failures after the marker's atomic installation are classified as
`routing_activation_install_indeterminate`; recovery begins with read-only
status inspection, never automatic deletion, overwrite, or blind replay.

The sixth bounded slice adds the separately explicit, digest-locked rollback
candidate. It requires the exact revision-1 migration receipt, v1 backup,
installed-v2 digest, absence across both home-root and registry-sidecar
read-routing activation aliases, and empty supported journal aliases. It
preserves exact installed v2 bytes in a digest-named
snapshot, restores exact v1 bytes by same-directory atomic rename, classifies
post-rename failures as `registry_rollback_install_indeterminate`, and makes
exact repeat invocation a verified no-write result. It is validated only on
isolated fixtures and does not authorize live rollback. Apply and rollback are
currently Unix-only candidates because their replace contract depends on
same-filesystem rename over an existing destination.

The seventh bounded slice adds that shared quiescence candidate. It derives
`<canonical-registry-path>.journal-quiescence.lock` independently of the
selected home/sidecar journal alias. The registry-coupled API accepts only the
canonical registry plus that closed alias choice and derives the journal root,
so callers cannot choose an unscanned path. Routed admission must acquire the
lock and then core code must verify the expected v2 revision and canonical
digest before creating journal layout; an additional caller eligibility check
can only restrict admission, and core repeats the identity read after it.
Rollback acquires the registry lock first, then this lock, and holds both
through replacement verification. Admission-first
and rollback-first fixtures prove that both sides cannot succeed. Orphan locks
are not stolen from age or PID and require exact, controlled recovery. At the
end of that seventh slice durable writes remained disabled because the actual
adapter route and activation surface were not yet included.

The eighth bounded slice wires the actual top-level `capture` caller to the
unified locator input, durable-write resolver, immutable CaptureIntent, and
registry-coupled journal. Registry v1 can preserve an explicitly
value-qualified target-neutral intent before migration; migration apply now
shares the same quiescence lock so admission and replacement are serialized.
Registry v2 requires the exact read-routing marker plus a separate
`project_ref_v2_journal_admission` marker. The journal marker has read-only
preview/status, digest-locked apply, exact-digest disable, fail-closed
malformed/stale/wrong-scope/symlink handling, and rollback alias inspection.
Isolated end-to-end fixtures prove unbound semantic ownership is journaled,
replay is idempotent, disable blocks later admission, and registry/Store bytes
remain unchanged. No ProjectRef bootstrap, journal events/projections, target
delivery, provider-specific adapter, installation, or live activation is in
this slice.

The ninth bounded slice, roadmap round 2, adds immutable event files with
contiguous sequences, previous-event and payload digests, deterministic
projection rebuild, and a read-only recovery probe that distinguishes the
stored cache from authority derived from the intent and events. The separately
explicit `project capture-recovery --apply` requires exact current registry
and projection digests, holds registry then journal-quiescence locks, records
fresh resolution, creates or reuses one deterministic pristine Store target,
atomically advances the registry, revalidates the complete binding, records
`project_binding_ready`, and replaces only the derived projection. Safe
pre-registry faults leave a recoverable pristine Store; post-registry,
post-event, and post-projection faults require status-first forward recovery
and never auto-roll back. Isolated semantic, repository, CWD, conflict,
replay, stale-guard, corruption, and six fault-window fixtures prove no
fallback, duplicate ProjectRef/binding, or semantic Store object. This slice
does not change automatic routing, marker scope, installation, or live state.

The tenth bounded slice, roadmap round 3, extends only the separately explicit
recovery apply path. Before a target write it appends `delivery_started` with
one stable delivery ID, the exact ProjectRef/Store/Workspace/Branch, Branch
head/state guards, target idempotency key, and materialized manifest digest.
It then reuses the existing atomic `cognition.capture` engine and appends a
verified `delivery_applied` receipt containing the committed
WorkStateCommit/ChangeSet, resulting state digest, every result identity and
version digest, and whether the target result was reused. If a crash occurs
after the Store commit but before that receipt, recovery reconstructs the
manifest from the durable start event, calls the same target idempotency key,
observes `reused=true`, and installs only the missing receipt. A retained
legacy manifest whose explicit guards are absent or stale records
`delivery_failed` with `legacy_manifest_upgrade_required`; its intent remains
byte-identical and the target Store is not opened for mutation. Isolated
fixtures cover normal delivery, receipt replay, commit-before-receipt,
projection rebuild, all nine recovery fault boundaries, and legacy staleness.
Registry re-resolution preserves a completed receipt only while the resolved
ProjectRef is unchanged and a later binding receipt revalidates the exact same
Store/Workspace/Branch; a changed target clears the current delivery view and
returns to `pending_primary` without writing. Event transitions and a
CaptureGroup canonical Record are preflighted before durable event or target
installation.
The eleventh bounded slice, roadmap round 4, adds authority-derived
CaptureGroup state plus `capture_group_resolved`, immutable
`reference_applied`, and idempotent `capture_completed` events. Recovery
installs only missing secondary references, preserves the successful primary
across reference failure, and never opens a secondary semantic Store. The
explicit read-only `project capture-group-recall` candidate scans immutable
journal authority by secondary ProjectRef and returns references pinned to the
exact canonical Record version/digest. A later canonical version does not
silently retarget an existing reference. Once canonical delivery exists, a
CaptureGroup target change fails closed and requires a new capture rather than
creating a second mutable authority. At completion of that source slice, all
evidence remained fixture-only.

The twelfth bounded slice, roadmap round 5, adds the first concrete tool
adapter behind the generic interface. The CLI integration layer turns a
bounded trusted handoff of verified desktop Project metadata and/or canonical
Project-mirror evidence into generic semantic locator evidence; core remains
tool-neutral. Authoritative metadata outranks a disagreeing mirror with an
explicit mismatch diagnostic, adapter absence permits verified Git/CWD
fallback, and explicitly bad handoff data fails closed. The same slice closes
the provisional-to-established stronger-locator attachment contract, aligns
the WorkVCS Skill and operator material, accounts for every R/C/M/N acceptance
row, obtains independent adversarial review, and refreshes the real-registry
preview under a zero-write probe. At completion of that source slice,
mutating behavior remained fixture-only.

No live registry/journal/Store mutation, activation, installation, Git commit,
push, release, deployment, or global Hook was part of that source slice.

## Delivery roadmap completion

The authoritative delivery roadmap contains eleven bounded rounds. All eleven
are complete. The table preserves the boundary and purpose of each round;
ordinary implementation detail does not create another hidden round.

| Round | Boundary | Purpose |
| --- | --- | --- |
| 1 | **Source candidate complete (2026-09-26).** Actual tool-neutral routed admission plus a separate, default-off journal-admission activation contract; isolated fixtures only, with no ProjectRef bootstrap or Store delivery. | Make valuable content reach the registry-coupled write-ahead journal through a real caller, while proving marker absence or drift is zero-write. |
| 2 | **Source candidate complete (2026-09-27).** Immutable journal events, derived/rebuildable projections, recovery status, and idempotent post-intent ProjectRef/bootstrap convergence; isolated fixtures only, with no semantic Store delivery. | Make an admitted intent recoverable and turn an unbound winning owner into one routable ProjectRef without fallback or duplicate identity. |
| 3 | **Source candidate complete (2026-09-27).** Primary target Store delivery and delivery-receipt recovery, including commit-before-receipt faults and legacy-manifest staleness; fixture Stores only. | Establish one canonical primary result and idempotent recovery without duplicate Store objects. |
| 4 | **Source candidate complete (2026-09-27).** CaptureGroup state and immutable secondary-project association, including pending-reference retry, completion receipt, and recall from a secondary ProjectRef; isolated fixtures only. | Preserve one mutable authority while making cross-project work explicitly discoverable from every intended project. |
| 5 | **Source candidate complete (2026-09-27).** First concrete tool adapter behind the generic interface, WorkVCS Skill ordering, the full acceptance matrix, operator material, independent review, and a refreshed zero-write real-state preview. | Prove the motivating desktop-Project omission is closed end to end and freeze a releasable source candidate. |
| 6 | **Complete (2026-09-27).** Exact staged review and the first local delivery commit; no push or tag. | Give the reviewed source an immutable local revision for reproducible installation and recovery. |
| 7 | **Complete (2026-09-27).** The reviewed local revision was packaged and installed with source/package/runtime parity while routing remained unchanged. | Put verified code in the runtime without changing registry format or routing behavior. |
| 8 | **Complete (2026-09-27).** Digest-locked live registry v1-to-v2 migration used the accepted ownership repair and preserved exact targets, backup, and rollback evidence. | Establish the ProjectRef control plane while preserving targets, backup proof, and rollback readiness. |
| 9 | **Complete (2026-09-27).** Exact live read-routing activation and read-only ownership canaries passed. | Validate real ProjectRef ownership resolution before any journal or Store mutation is permitted. |
| 10 | **Complete (2026-09-27).** Exact live journal-admission activation and bounded Store-free intent admission passed. | Establish the live no-silent-loss-after-admission boundary before accepting Store-delivery risk. |
| 11 | **Complete (2026-09-27).** Installed public `cognition_v2` and CaptureGroup input produced Capture `01a0e33e-31e1-7b91-a698-4e06525377fd`; primary delivery, Hernes immutable-reference recall, status-first replay, and old-intent preservation all passed. | Complete the accepted initial feature in live operation and prove receipts, idempotency, and operator recovery. |

Rounds 1 through 5 were source-function rounds, round 6 established the first
local Git delivery boundary, and rounds 7 through 11 installed and canaried
the live local control plane. Global per-turn Hook activation, non-Unix atomic
replacement support, journal retention/compaction, historical CaptureGroup
backfill, push, tag, public release, and remote deployment are not hidden
follow-on rounds; each is optional or separately scoped work requiring its own
decision and authority.

## Consequences

- Valuable content can be durably queued even when the ambient directory is
  not bound or ownership cannot yet be resolved.
- A semantic Project can own work performed from a repository, mirror, or
  coordination directory without erasing those contexts.
- Project identity survives path changes and gains stronger locators without
  relocating existing WorkVCS state.
- Cross-project work remains discoverable without duplicating one mutable
  conclusion across Stores.
- Recovery becomes explicit and testable, at the cost of a central routing
  journal and eventual convergence across Stores.

## Rejected or deferred alternatives

- **CWD as the sole identity:** rejected because it can misroute or lose
  semantic Project work.
- **One global semantic Store:** rejected because it collapses project
  portability and ownership boundaries.
- **Automatic merge by name, path, or shared target:** rejected because those
  signals do not prove one logical project.
- **Best-effort direct Store write before journaling:** rejected because a
  missing binding or partial failure can again lose the record.
- **Cross-Store all-or-nothing transaction:** rejected as a false guarantee;
  idempotent convergence is the contract.
- **Full transcript capture:** rejected; capture is a bounded semantic package.
- **Global per-turn Hook in the initial release:** deferred for separate
  confirmation.

## Acceptance evidence

The user confirmed the exact schema, migration, failure semantics, and
acceptance matrix on 2026-09-23 and authorized the authority-document sync.
The user then authorized the twelve bounded implementation slices described
above, including the historical Hernes ownership-repair preview, isolated
migration apply/fault validation, v1/v2 read consumers, and the isolated
read-routing activation candidate, the isolated shared-quiescence race and
orphan-lock candidate, the actual routed journal-admission/activation
candidate, and the explicit journal recovery/ProjectRef bootstrap candidate.
They establish executable core contracts and guarded
read/migration/admission/recovery/delivery mechanics, not live feature
completion: no live registry, journal, activation marker, or target Store was
mutated. Live
migration apply, live read-routing activation, live journal admission, Store
delivery, live rollback, installation, push, release, and global Hook
activation remain separately authorized. Round 6 contributes only the local
commit containing this ADR update. The full acceptance matrix must pass before
the ADR can be reported fully implemented.
