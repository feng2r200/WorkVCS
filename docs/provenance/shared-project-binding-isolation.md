# Shared Project Binding Isolation Evidence

Status: Source, exact CI, installation, live isolation, activation, and isolated Plan admission complete
Date: 2026-10-07
Decision: [ADR-0517](../decisions/adr/0517-shared-project-binding-isolation.md)

## Problem

The configured registry contains two distinct ProjectRefs whose bindings name
the same Store, Workspace, and Branch. Registry migration preserved that fact
correctly, but the target is unpartitioned and cannot prove logical-project
ownership for new governance state. Registry v2 had no supported rebind or
isolation operation, so a safe repair could not be completed through the
public control plane.

## Implemented contract

- `project isolate-shared-binding --preview` performs a zero-write, exact-owner
  inspection and reports whether the target is `eligible`, `not_shared`, or
  `already_isolated`.
- The candidate digest binds the registry snapshot, ProjectRef, exact old
  target tuple, and deterministic dedicated Store path.
- Apply rechecks both digests under registry and journal-quiescence locks,
  retains an exact digest-named registry backup, and initializes or reuses only
  a missing, empty-recoverable, or pristine deterministic Store.
- The core transition changes one binding, advances revision once, and records
  `binding_source=isolation`.
- No old Store bytes or Work State are copied. The original Store and migration
  observation remain unchanged as historical evidence.
- Registry replacement is atomic and every binding plus the backup is verified
  after installation. Post-replacement uncertainty has the dedicated stable
  error `shared_binding_isolation_install_indeterminate`.
- Read-routing and journal-admission markers are not changed by isolation and
  therefore become stale until separately refreshed against the new registry.

## Validation

Current local evidence:

- `cargo test --workspace --all-targets --locked` exits zero; the CLI unit
  target includes 276 passing tests, including post-replacement fault injection
  and preview-first recovery for all three isolation installation fault points.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
  and `cargo fmt --all -- --check` pass.
- Schema validation, the full CLI smoke workflow, the 98-row ProjectRef
  acceptance matrix, and operator recovery maturity all pass. The acceptance
  matrix includes the shared-binding isolation documentation and policy probe.
- `git diff --check` passes.

## Delivery and live adoption evidence

| Gate | Exact evidence |
| --- | --- |
| Source delivery | Commit `fa2597636410280b5049fc789468bc37c93602ed` was pushed normally to `main`. |
| Remote verification | Exact [CI run 37553941190](https://github.com/feng2r200/WorkVCS/actions/runs/37553941190) succeeded with formatting, lint, full tests, schema, CLI smoke, and the 98-row ProjectRef matrix all passing. |
| Installed binary | The official clean-source package installed `/Users/ld/.local/bin/workvcs` with SHA-256 `d20dc980139b40328338e1a97e7e6f109ec2121bb4acc4ec38076de6d7e199b7`. |
| Installed Skill | The installed WorkVCS Skill entry has SHA-256 `026eec4abc5ccbe8d9e8b148a6951509664e07eee21428370f7fc86ae6e1f02f`; its full tree has SHA-256 `a8829c7956085384d10920eaf34209d7e6fc53c1c46d497a78d7d3e7b2897687` and matches the source tree exactly. |
| Registry transition | Registry revision 2 digest `blake3-256:608fbd17ca0469019062653c546bf8c8b56b66c9e5c0f1bad8b39a33fb87da64` advanced exactly once to revision 3 digest `blake3-256:b5ec12377cbe536bce8cb53e5378c95d019a5c8dde1584843b843e68868aa6c9`. |
| Selected binding | ProjectRef `01a0e2e6-f2e6-77a2-b858-7690808a81a7` now names dedicated Store `/Users/ld/.codex/workvcs/stores/projects/work-governance-76995749aeb47b240ac15702cf64bbbe1f3dd4bc74ee571b703ab53a4f3d69fd.sqlite`, Store `01a113dd-b621-77b9-8034-f838133213d5`, Workspace `01a113dd-b62a-750c-8d02-1beada96c911`, and Branch `01a113dd-b62a-750c-8d02-1bedd4a7728c`. |
| Preserved authority | The digest-named registry backup has SHA-256 `ae4202e09ae66c6e0188dc4a4e4d6e6b9e219907cfa6fd324121e2ca3cf03ac9`, exactly matching the pre-isolation registry bytes. The old shared Store remained at SHA-256 `9d71c02601946d3756c9ea16ff2e6b552b155664b276b740e93c127d13a83367`; no old Work State was copied. |
| Reactivation | Read-routing digest `blake3-256:d4c659d59c8ff1aa56d8f15365d87c831c67ce9ecd4def1db8b5e58807a02e3b` and journal-admission digest `blake3-256:91279a43e0b0913a78f625f2c5bb20b66ff7790c0f3af5faed9f94cc20dae622` are active for registry revision 3. All seven bindings validate, with zero invalid bindings. |
| Isolated durable work | Journal capture `01a113df-4b8a-75f8-9abc-b48956997039` committed at `01a113df-4c77-7423-b47c-f2e2b28c6c98`, admitting Goal `01a113df-4c77-7423-b47c-f2d2871cb496`, Plan `01a113df-4c77-7423-b47c-f2d4deb591d9`, and Task `01a113df-4c77-7423-b47c-f2d6d49e85a7` into the dedicated Store with a durable receipt. |

The WorkVCS peer ProjectRef remains on the original Store. A post-transition
`project list --require-valid` reported seven valid and zero invalid bindings.
This is direct live proof that logical discovery, physical state isolation,
capability reactivation, and subsequent durable Plan admission all converge.

## Boundaries

The isolation operation itself does not delete or mutate the old Store, copy
ambiguous history, merge ProjectRefs, refresh activation markers, admit a
Plan, release, tag, or deploy. Marker refresh and Plan admission above were
separate, explicit post-isolation operations. No release, tag, or deployment
was performed.
