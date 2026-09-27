# ProjectRef CaptureGroup Secondary Reference Candidate Evidence

Status: Implemented and validated on isolated fixtures; no live registry,
journal, semantic Store, activation, installation, or Git commit change
Date: 2026-09-27
Authority: [ADR-0513](../decisions/adr/0513-projectref-durable-capture-routing.md)
Contract: [ProjectRef Control Plane v2](../architecture/projectref-control-plane-v2.md)
Acceptance: [Registry v2 Migration and Acceptance](../architecture/projectref-registry-v2-migration-and-acceptance.md)
Roadmap position: round 4 of 11 complete; seven mainline rounds remain

## Authorized boundary

This round extends only the source-tree, separately explicit ProjectRef
recovery candidate. It materializes CaptureGroup state, immutable
secondary-project associations, missing-only reference retry, a completion
receipt, and read-only recall from a secondary ProjectRef. Every registry,
journal, projection, semantic Store, injected failure, and recovery mutation
used a disposable fixture root.

The WorkVCS Goal, Plan, Tasks, and later completion records in the repository's
already-bound governance Store are ordinary governance state for this round.
They are not ProjectRef registry, capture-journal, activation, or semantic
target-Store writes.

This round did not run the source binary against the configured live control
plane, activate either routing marker, install a binary, change a real
ProjectRef registry or journal, create a Git commit, push, release, deploy, or
install a global Hook.

## One canonical authority and rebuildable group state

The immutable CaptureIntent and event chain remain authority. The projection
now derives:

- one resolved primary ProjectRef and exactly one canonical primary-role
  member;
- the exact canonical Record reference from the verified primary receipt;
- every required, applied, and pending immutable secondary reference;
- unresolved related locator evidence retained from current resolution; and
- the optional `capture_completed` summary receipt.

An initially unbound group pins the admitted primary locator evidence digest.
Only a later resolution whose primary basis carries that exact digest may
install `capture_group_resolved`; it adds one canonical member without
rewriting the intent. A second primary role, second canonical delivery,
non-member reference, wrong relation, wrong canonical version, or duplicate
receipt fails before an immutable event file is installed.

After the canonical receipt exists, a resolution or binding event that would
retarget the CaptureGroup fails closed. A different canonical target requires
a new capture. This is stricter than the non-group recovery path and prevents
recovery from creating a second mutable authority.

## Immutable reference convergence

After primary delivery, `pending_references` lists only CaptureGroup members
whose requested delivery is `immutable_reference` and whose receipt is absent.
Explicit recovery validates that each secondary ProjectRef still exists in the
current registry, then appends only missing `reference_applied` events. It does
not open or write the secondary semantic Store and never rolls the primary
back.

Each reference stores the CaptureGroup ID, secondary ProjectRef, relation,
observation time, and the complete canonical Record reference, including its
logical Record ID, immutable version ID, and version digest. Later evolution
of the canonical Record does not rewrite or silently advance an existing
reference.

When primary plus all required references are authoritative, semantic state is
`completed`. `capture_completed` is an idempotent summary receipt rather than
the source of completion truth. A crash after the last reference but before
the summary therefore reports `completed` with the bounded next action
`apply_capture_completion`.

## Read-only recall from a secondary ProjectRef

The new explicit candidate is:

```text
workvcs project capture-group-recall --project-ref-id ID [--registry PATH]
```

It verifies that the ProjectRef exists in the selected v2 registry, enumerates
the registry-derived journal aliases, validates every encountered immutable
intent and event chain, rebuilds projections in memory, and returns matching
secondary associations in deterministic order. Duplicate association identity
across journal aliases fails closed. The query does not consult a stored
projection cache and reports `store_opened=false`, `store_written=false`, and
all control-plane write flags false.

This command is an uninstalled operator candidate. Its ability to inspect an
inactive fixture registry does not activate ordinary v2 read routing.

## Fault and fixture evidence

The round-4 CLI matrix injects these boundaries:

| Fault boundary | Observed safe state | Recovery behavior |
| --- | --- | --- |
| after `capture_group_resolved` install | primary fixed, no primary receipt | continue the existing primary delivery once |
| before secondary-reference install | canonical primary retained, `pending_references` | install only missing references |
| after secondary-reference install | reference may already be authoritative, completion summary absent | status first, then install only the summary |
| after `capture_completed` install | completed receipt authoritative | exact replay performs no new step |

The focused core tests prove:

- missing-only retry and exact replay;
- recall from both secondary ProjectRefs using immutable authority rather than
  projection cache;
- pinning to the original canonical version after a later Store-local
  correction;
- invalid reference transition rejection before event installation;
- exactly one event-derived primary from matching locator evidence; and
- rejection of a second primary role before intent admission.

The focused CLI tests prove one end-to-end grouped recovery/recall path and all
four fault windows. The secondary Store's SQLite file snapshots remain
byte-identical, completed replay writes zero new events or projection bytes,
and the original primary delivery remains exactly one commit.

## Acceptance mapping

The isolated source candidate closes the implemented portions of C-06, C-10,
C-14, C-16, C-17, C-18, C-23, C-25, N-05, and N-07. It does not claim a live
canary, installed behavior, provider integration, or cross-Store transaction.
N-10's formally independent review remains intentionally assigned to roadmap
round 5; this round used an adversarial self-review and added the second-primary
role regression after that review found the gap.

## Validation evidence

Validation against the final source candidate passed with the standalone
Command Line Tools LLVM toolchain selected explicitly because the host's
default Xcode driver is blocked by an unaccepted Xcode license:

- four focused core CaptureGroup tests and two focused CLI recovery/recall
  tests;
- `cargo test --workspace --all-targets --quiet`, including all 255 CLI tests
  and every core unit/integration target;
- strict workspace/all-target/all-feature Clippy with warnings denied;
- schema v0.1 validation;
- the full CLI smoke workflow with `smoke_result=passed`;
- operator recovery maturity with `PASS`, 53 core error codes, 54 guide
  entries, zero missing/extra coverage, and retryability matching the core
  rule;
- Rust formatting and documentation-link checks; and
- `git diff --check`.

No system toolchain setting or license state was changed.

## Live zero-write proof

Before implementation, the configured registry was v1 at
`/Users/example/.codex/workvcs/project-bindings.json`, with SHA-256
`c57ca8aa9b1f4a7d3ff0b1db9a35384b7e3776f809b7fb44737498a0ee323ed4`, size
`2862`, mtime `1789695294`, inode `107118324`, and mode `0644`.

The final post-validation probe reproduced that exact tuple and confirmed
`version=1`. Both home and registry-sidecar read-routing markers,
journal-admission markers, capture-journal aliases, and the canonical
journal-quiescence lock remained absent. The source binary was never pointed
at those paths.

## Governance closeout

The installed WorkVCS binary recorded six passed Verification objects, then
closed all three round tasks, completed Plan
`01a0e12b-cc3e-7481-b1f8-57428f61f137`, and achieved Goal
`01a0e12b-cc3e-7481-b1f8-5725cdcf6c78` in the dedicated governance Store.
The final governance Branch head is
`01a0e153-6bf3-7352-ad68-bb3c0a1c2b20`, with state digest
`435d23a692a723595c4e1355a2a846dfbcee6e6baa1d9447dc0b659821a00831`.
These are the only non-fixture durable writes performed in this round.

## Next confirmation gate

Round 5 may implement the first concrete tool adapter behind the generic
interface, update WorkVCS Skill ordering around that adapter, execute the full
acceptance matrix, finish operator material, obtain independent adversarial
review, and refresh the zero-write real-state preview. It must still exclude
real registry/journal/semantic Store mutation, activation, installation, Git
commit, push, release, deployment, and global Hook work.
