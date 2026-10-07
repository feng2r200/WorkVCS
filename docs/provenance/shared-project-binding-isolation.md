# Shared Project Binding Isolation Evidence

Status: Source implemented and locally validated; delivery and live adoption pending
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

## Source validation

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

Packaging, exact CI, installed-binary verification, live isolation, activation
marker refresh, and reconstructed work-governance state remain separate gates
and will be added only after each has current evidence.

## Boundaries

This operation does not delete or mutate the old Store, copy ambiguous
history, merge ProjectRefs, refresh activation markers, admit a Plan, release,
tag, or deploy. Those are separate operations and authority boundaries.
