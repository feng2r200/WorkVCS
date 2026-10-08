# Authorized Existing-Binding Delivery Historical Reconciliation Evidence

Status: Stage D selected historical reconciliation complete; remote delivery pending
Date: 2026-10-08
Decision: [ADR-0519](../decisions/adr/0519-authorized-existing-binding-delivery-and-operation-inventory.md)
Plan: [Authorized Existing-Binding Delivery Design and Change Plan](authorized-existing-binding-delivery-plan.md)
Local adoption: [Authorized Existing-Binding Delivery Local Adoption Evidence](authorized-existing-binding-delivery-local-adoption.md)

## Scope and authority

This evidence closes the separately authorized Stage D reconciliation set. The
selection was deliberately narrower than the global open-operation inventory:
only the two `cognition_v2` operations that both resolved to ProjectRef
`01a0e2e6-f2e6-77a2-b858-7680d26d9eb0` and reported
`recovery_action=apply_binding_receipt` were eligible. Unbound owners,
bootstrap, repair, target-change, semantic-invalid, other-ProjectRef, legacy,
and terminal new-capture actions remained outside the operation.

The governing durable Plan was
`01a11afc-b1ca-76d4-b53d-d193a802099e` under Goal
`01a11afc-b1ca-76d4-b53d-d1910821013d`. Its Stage D Task was
`01a11afc-b1ca-76d4-b53d-d19dd7b77543`. The Plan admitted this exact selected
set before either recovery apply.

## Read-only selection

The initial global inventory contained 42 open operations: 41
`cognition_v2` and one legacy operation. Its action summary contained 26
`apply_binding_receipt`, 12 `apply_project_bootstrap`, two
`start_new_capture_canonical_target_changed`, and two
`start_new_capture_with_corrected_payload` rows.

Filtering that same authority view by the current WorkVCS ProjectRef returned
exactly these two valid same-binding rows:

| CaptureId | Original meaning | Selected action |
| --- | --- | --- |
| `01a1112c-51f3-759c-a6d7-de2e4bfadfcf` | Historical performance diagnosis recorded before its delivered repair | `apply_binding_receipt` |
| `01a11a6e-a137-707c-9001-e825e391ee41` | Stage B milestone that still described later validation and adoption work as pending | `apply_binding_receipt` |

No other row was selected or mutated by this stage.

## Exact delivery results

Each operation was inspected immediately before apply and recovered with the
fresh registry and projection digests returned by its own status result. Both
applies completed the existing target delivery and durable receipt without a
registry write:

| CaptureId | DeliveryId | Target WorkVCS Commit | Delivered Finding |
| --- | --- | --- | --- |
| `01a1112c-51f3-759c-a6d7-de2e4bfadfcf` | `01a11afc-fb82-71fc-9235-24e2ba462701` | `01a11afc-fc8a-7344-867b-0009a8196ef8` | `01a11afc-fc49-75f1-a083-96b8c0b36e0d` |
| `01a11a6e-a137-707c-9001-e825e391ee41` | `01a11afd-276f-7723-8cc7-46d8f1d811be` | `01a11afd-2883-70e8-993a-76841f7bb75e` | `01a11afd-283f-7705-b1ca-9907d4e16fe8` |

The delivered Findings were truthful historical checkpoints but were no
longer current. They were therefore reconciled with guarded semantic
supersession rather than edited or deleted:

| Superseded Finding | Current replacement | Reason |
| --- | --- | --- |
| `01a11afc-fc49-75f1-a083-96b8c0b36e0d` | `01a11174-57bb-76e5-9de6-a8ddf1bcde95` | The later delivered performance repair, exact remote ref reconciliation, and terminal CI evidence replace the pre-implementation diagnosis as current truth. |
| `01a11afd-283f-7705-b1ca-9907d4e16fe8` | `01a11aa4-92d7-7434-9244-4b6f658c35d4` | The validated and installed Stage C canary replaces the Stage B milestone that still described validation, review, and commit as pending. |

Readback showed both prior Findings at `status=superseded` and exactly one
`supersedes` relation from each named replacement. Both Stage D Verification
Requirements passed, both Acceptance Criteria became `verified`, and the
Stage D Task reached `done` at WorkVCS commit
`01a11aff-939e-74b5-805a-135bf810ea6c`.

## Postconditions and preserved control plane

A fresh filtered inventory for the current ProjectRef returned zero rows with
inventory digest
`blake3-256:333b0a1bf3efb7f23b7b3a24a505a1bf8fb077c66d7074a10736b1ce8414da51`.
The global cross-project inventory later contained 39 rows. That live total is
not attributed solely to this task: concurrent or independently completed
operations may change it. This stage claims only the two named recoveries and
does not claim or authorize a global sweep.

After reconciliation, `project health` remained `healthy`, all seven bindings
were valid, 49 local content objects verified, read routing and journal
admission were active, and all three journal capabilities were enabled. The
control-plane artifacts retained their pre-stage SHA-256 values:

| Artifact | SHA-256 |
| --- | --- |
| ProjectRef registry | `e30b185dfa61071b4e6083eee86a570836b112e76f3e2e2d1be07bb2b3a0455f` |
| Read-routing activation | `5c93245fbaf790bee2ad677d06ef566937448f9efcf594470512382aa2fd6a96` |
| Journal-admission activation | `84132fa9610cac33327d9d7ef3d3fbed7d902a2f00e7c6bee6f1464a078c3b66` |

## Remaining boundary

This is a selected historical reconciliation, not standing batch authority.
The remaining global inventory keeps its own ProjectRef, action, and
authorization boundaries. Tag, release, deployment, rollback, history rewrite,
and destructive cleanup remain outside ADR-0519 completion.

The repository evidence commit, fast-forward Push, exact remote ref readback,
and terminal CI result are the next and final delivery boundary for this work.
