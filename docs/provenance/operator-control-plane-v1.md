# Operator Control-Plane V1 Implementation Evidence

Status: Source implemented and locally accepted
Date: 2026-10-07
Decision: [ADR-0518](../decisions/adr/0518-operator-control-plane-health-and-recovery-contract.md)
Invariant: [INV-110](../domain/invariants.md#inv-110--operator-health-is-complete-single-pass-and-non-activating)

## Accepted scope

`WORKVCS-OPERATOR-CONTROL-PLANE-V1` adds one bounded operator contract without
weakening any existing integrity, routing, journal, or recovery boundary:

- `workvcs project health` is a composite, strictly read-only inspection of
  registry validity, binding integrity, read-routing activation,
  journal-admission activation and capabilities, and optional CWD ownership;
- cleanly absent activation or capability authority has dedicated structured
  error codes, while stale, malformed, mismatched, unreadable, or racing
  authority continues to fail closed as `control_plane_invalid`;
- `workvcs project operation-recovery` is the canonical operator spelling for
  the existing recovery state machine, and `project capture-recovery` remains
  a visible compatibility alias;
- persistent CaptureIds, journal paths and schemas, Rust recovery variants,
  historical output fields, and stable historical errors are unchanged.

The accepted scope does not install or refresh an activation marker, migrate a
registry, admit a durable operation, write a Store, install an artifact,
release, tag, or deploy.

## Health contract implemented

One health invocation loads one registry snapshot and calls the complete
read-only binding verifier exactly once for every binding. That verifier still
performs full Store integrity validation plus Store, Workspace, Branch, and
local-content identity checks. Optional CWD resolution reuses that inspected
snapshot and does not reopen the selected Store.

The report classifies the control plane as:

| State | Meaning |
| --- | --- |
| `healthy` | Registry v2, every binding, both activation markers, all supported durable-operation capabilities, and optional CWD resolution are valid. |
| `degraded` | Authority is valid, but registry migration, explicit activation, or capability refresh remains required. |
| `blocked` | Binding, activation, capability, or requested resolution evidence is invalid, stale, ambiguous, or otherwise unsafe. |

`--require-healthy` converts a non-healthy report into a stable machine-routable
error. `--timings` adds observations only; it is not an acceptance threshold.
Regression fixtures compare registry, marker, journal, and Store bytes before
and after inspection to enforce the zero-write contract.

## Error and recovery contract implemented

The public key-value and JSON error envelopes now preserve bounded recovery
context for these cleanly inactive states:

| Error code | Clean state represented | Recovery direction |
| --- | --- | --- |
| `routing_activation_inactive` | Read-routing marker is absent. | Inspect, then explicitly activate read routing. |
| `journal_admission_activation_inactive` | Journal-admission marker is absent. | Inspect, then explicitly activate journal admission. |
| `journal_admission_capability_inactive` | Marker is valid but omits the required typed capability. | Inspect, then explicitly refresh the same-snapshot capability set. |

All three are non-retryable and recoverable only through a separate authorized
control-plane action. The error envelopes do not perform or authorize that
action.

## Acceptance evidence

The current source tree passed:

- `cargo test --workspace --all-features`; the complete CLI, core integration,
  and documentation suite passed. After the final CLI-only error-priority
  review correction, `cargo test -p workvcs-cli` reported 283 passed and zero
  failed;
- `cargo check --workspace --all-targets --all-features`;
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
- `cargo fmt --all -- --check` and `git diff --check`;
- schema v0.1 validation and the full CLI smoke workflow;
- operator-recovery maturity, including exact operator-guide coverage for all
  58 core error codes plus the CLI parse error; and
- the 98-row ProjectRef acceptance matrix, including its operator-control-plane
  source, documentation, invariant, and provenance probes.

A source-built, read-only canary against the configured registry also returned
`healthy`: registry v2 revision 3, seven valid and zero invalid bindings, all
three durable-operation capabilities active, the WorkVCS checkout resolved to
its expected ProjectRef, and every reported write flag was `false`. This is
inspection evidence, not installation or activation evidence.

The feature regressions specifically prove one verifier call per binding,
healthy/degraded/blocked classification, optional CWD resolution, diagnostic
timings, dedicated absent and missing-capability errors in both public output
formats, blocked-evidence priority over cleanly inactive states, generic
failure for stale authority, and canonical/compatibility recovery command
equivalence.

The first full-suite run also exposed an existing concurrency weakness in the
first-use `project ensure` path: under sustained test load, the second of two
simultaneous bootstraps could exhaust the generic short registry-lock window.
The repair gives only `project ensure` a longer but still bounded lock wait;
all other registry mutations keep the existing shorter timeout, no lock is
stolen, and no validation is skipped. The concurrent-first-use regression
passed repeatedly in isolation and again inside the complete workspace suite.

## Delivery boundary

This record proves source implementation and local acceptance only. Git
commit, Push, exact remote CI, installation, activation, and live adoption are
separate facts and must not be inferred from this document.
