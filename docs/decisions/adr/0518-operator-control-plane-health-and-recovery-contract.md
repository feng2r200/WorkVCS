# ADR-0518: Operator Control-Plane Health and Recovery Contract

Status: Accepted and implemented
Date: 2026-10-07

## Context

ProjectRef v2 deliberately separates registry binding, read-routing activation,
journal-admission activation, typed capabilities, and durable-operation
recovery. That separation keeps authority explicit, but operators previously
had to combine several commands to answer one basic question: whether the
selected control plane is ready for normal reads and all currently supported
durable operations.

The existing status surfaces also used the broad `control_plane_invalid` code
for both a cleanly absent activation and a malformed or stale marker. Those
states require different responses. Absence is a recoverable, explicitly
inactive configuration; corruption or stale authority must continue to fail
closed without being mistaken for permission to activate.

Finally, `project capture-recovery` had become the recovery route for cognition
and typed Plan operations. Its name described the historical persistence
schema rather than the operator concept and could imply that it was
cognition-only. Renaming persistent CaptureIds, journal paths, projections, or
stable historical errors would create a second authority and is not acceptable.

## Decision

WorkVCS adds a composite, read-only operator inspection:

```text
workvcs project health [--cwd PATH [--project-ref ID]] [--registry PATH]
                       [--require-healthy] [--timings]
```

One invocation loads the selected registry once and performs exactly one complete validation of every binding.
Complete validation still opens each
Store read-only, validates full integrity, and verifies Store, Workspace,
Branch, and local content identity. Health then inspects both exact-snapshot
activation markers, all three journal capabilities, and optional CWD owner
resolution without reopening the selected Store. It reports:

- `healthy` when registry v2, every binding, both activation markers, all
  declared capabilities, and any requested resolution are valid;
- `degraded` when authority is valid but registry migration, explicit marker
  activation, or capability refresh is still required; and
- `blocked` when a binding, marker, capability inspection, or requested
  resolution is invalid, stale, ambiguous, or otherwise unsafe.

The default command always reports an inspectable state when its inputs can be
read. `--require-healthy` converts non-healthy state into a machine-routable
failure. `--timings` adds diagnostic microsecond observations only; timings are
not acceptance thresholds and do not weaken validation.

The command is strictly read-only. It never migrates a registry, installs or
refreshes a marker, admits a journal operation, opens a Store writable, or
changes registry, journal, projection, or Store bytes.

Cleanly absent authority uses dedicated stable errors:

- `routing_activation_inactive`;
- `journal_admission_activation_inactive`; and
- `journal_admission_capability_inactive`.

Their key-value and JSON forms include bounded activation scope, state, path,
required capability/version when applicable, `recoverable=true`, and one
explicit recovery action. They do not authorize that action. Stale, malformed,
wrong-scope, mismatched, symlinked, unreadable, or racing marker state remains
`control_plane_invalid`.

The canonical operator command becomes:

```text
workvcs project operation-recovery ...
```

`project capture-recovery` remains a visible compatibility alias without a
removal deadline. The Rust variant, CaptureId, journal directory, projection
format, historical output fields, and
`capture_recovery_install_indeterminate` error code remain unchanged. This is
one command surface over one existing state machine, not a parallel recovery
protocol.

## Consequences

- Operators and automation can prove readiness with one auditable read-only
  command instead of composing partially overlapping status calls.
- A missing marker or capability is distinguishable from damaged authority
  without making either state retryable or self-activating.
- Existing scripts and admitted durable operations continue to work with the
  compatibility spelling and persistent Capture terminology.
- Complete integrity validation and the separately fresh writable-open
  validation used by recovery apply remain intact.
- Health does not replace focused migration, activation, rollback, recovery,
  or Store diagnostics; it provides their bounded control-plane summary.

## Verification

CLI regressions prove one verifier call per binding, full healthy/degraded/
blocked classification, optional CWD resolution, diagnostic timings, dedicated
absent/capability errors in key-value and JSON, generic failure for stale
markers, canonical and compatibility command parsing, and byte-stable registry,
marker, journal, and Store state across health inspection. The existing full
workspace suite, schema validation, operator recovery audit, ProjectRef
acceptance matrix, formatting, lint, and source-tree checks remain required for
delivery.
