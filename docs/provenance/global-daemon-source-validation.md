# Global Daemon Source Validation

Observed: 2026-10-13
Scope: WorkVCS 1.4.2 source acceptance under ADR-0532.
Execution Plan: `01a135cc-272b-75f4-b93a-477d966a84bd`.

## Recovered remaining work

The original post-V1 delivery stopped at a disabled per-user service. Commit
`546ce33e94a158dc6e3ddf64749f848d318ad147` added registry-derived membership
and a separate source policy, but the live policy and registrations were
empty and no LaunchAgent or daemon was active. Its watcher still observed only
legacy registrations, so global-mode startup did not observe real sources.

The user subsequently authorized completing daemon startup, login startup,
installation and delivery. Feature-branch installation is allowed for
verification; final main integration remains specific to this project.

## Real falsifying evidence

An explicitly authorized user-only policy adopted exactly
`/Users/ld/.codex/sessions` and `/Users/ld/.codex/archived_sessions`. The first
real foreground reconciliation took 306 seconds, exceeding the former fixed
300-second daemon worker cutoff. It classified 1,099 files, prepared all seven
canonical generations, made 281 history archives available and retained 818
failed historical sources. Seven hundred were unbound; others had removed
working directories, inconsistent metadata, absent selected text or missing
stable message IDs. These failures remain visible in an incomplete catalog.
No historical ownership repair or ProjectRef bootstrap was inferred.

The exact report and catalog are persisted through WorkVCS Capture
`01a135d6-f809-7681-8270-89c2518188cd`. The installed `546ce33` HXAP brief Recall
also returned successfully; final installed validation is a separate check.

## Corrections and local checks

- Registry-derived watcher targets now include exact registry/routing/policy
  controls, member Store families and approved roots, with target refresh after
  complete rounds and derived-output feedback suppression.
- One asynchronous worker permits responsive status and queued/coalesced
  triggers. Default worker timeout is absent; an explicit positive value is
  supported. SIGTERM/SIGINT and IPC shutdown cancel, reap, retain evidence and
  clean the socket. Natural process exit at timeout/cancel boundaries is not
  falsely classified as termination.
- The macOS adoption script renders, installs, starts and inspects the exact
  stable executable/registry/root job. It refuses conflicting definitions and
  unsafe file/directory permissions, preserves identical installation and
  confirms the actual loaded job before reporting running or startup state.

The complete workspace suite passed 1,086 tests across 179 targets. After the
signal/cancellation corrections, all 22 automation-module tests and six daemon
E2E tests passed, including slow active-worker response, queued manual work,
SIGTERM/SIGINT cleanup/restart, registry/policy/member changes and no feedback.
All three current installer tests passed, covering zero-write XML rendering,
invalid input, idempotency, unrelated/symlinked plists and permission drift.
Formatting, full-workspace/all-target/all-feature Clippy, schema, CLI smoke and
the unchanged 98-row ProjectRef acceptance matrix passed.

## Independent challenge

Separate reviewers challenged the installation and runtime slices. Installation
review exposed false startup claims outside login discovery directories,
mutable configuration/directory acceptance and missing running confirmation;
all were corrected and rechecked. Runtime review exposed missing signal
cleanup and process-exit races; both were corrected, with independent focused
unit and real slow-worker E2E verification. No blocking/high finding remained.

This source-stage snapshot does not itself prove live startup, login/restart
adoption, main push, CI or installed-build parity. Those exact observations and
the final independent Validator result must be recovered from the execution
Plan and its evidence before overall completion is claimed. Catalog partial
failure and concurrent canonical writes by other tasks must retain their
actual scope throughout that validation.
