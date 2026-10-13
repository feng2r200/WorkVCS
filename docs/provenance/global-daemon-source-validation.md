# Global Daemon Source Validation

Observed: 2026-10-14
Scope: WorkVCS 1.4.5 source acceptance under ADR-0532.
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

The final frozen source passed 1,100 workspace tests across 179 targets using
Rust/Cargo 1.98.1 in an isolated build directory. All 32 automation-module tests,
seven daemon E2E tests and four installer tests passed, including slow active
worker response, queued manual work, SIGTERM/SIGINT cleanup/restart,
registry/policy/member changes, no feedback, PID-bound IPC readiness and
installation permission drift.
Formatting, full-workspace/all-target/all-feature Clippy, schema, CLI smoke and
the unchanged 98-row ProjectRef acceptance matrix passed.

The raw 1.4.5 full-suite log is immutable WorkVCS Evidence
`01a136a6-0baa-715d-bed3-5043e2bf9957`. Its independently reviewed Rust hash is
`38c56bd9fe4d47d06748bb65db4bb80b38b1334884854f8b857068ff18d8034f`;
the daemon E2E file is unchanged from accepted 1.4.4. The earlier 1.4.4 frozen
full-suite Evidence is `01a13672-bedf-7528-be2e-7261435ba56e`.
The earlier 1.4.3 runtime/installer
source-review evidence is `01a13637-0c5f-75dc-8602-f88d51d8b4ec`; the bounded
alias review and final test-only delta are
`01a13667-08db-71b7-95fa-ded724f9815b` and
`01a1366e-a4b3-7101-a209-8c05251bee28`. Earlier mixed-source/shared-target
failures and the pre-lint 1.4.4 run are retained separately, not counted as the
final frozen proof. Six test clone lints and one equivalent move-to-borrow
consistency rewrite preceded the final freeze; product logic did not change.

## Independent challenge

Separate reviewers challenged the installation and runtime slices. Installation
review exposed false startup claims outside login discovery directories,
mutable configuration/directory acceptance and missing running confirmation;
all were corrected and rechecked. Runtime review exposed missing signal
cleanup and process-exit races; both were corrected, with independent focused
unit and real slow-worker E2E verification. No blocking/high finding remained.

Actual 1.4.2 LaunchAgent adoption additionally falsified foreground-only
validation: an unnecessary Git subprocess at an unknown historical Documents
cwd waited for TCC permission. Automation now matches native cwd/Git metadata
to active registry identities without opening unknown directory bodies,
verifies standard external worktree backlinks and rejects symlink escapes.
Cached local identity is revalidated before publication; ordinary foreground
project discovery is unchanged. No system privacy grant or historical owner
repair was made.

Independent review then closed early-leader-exit pipe retention, Darwin
zombie-only group EPERM, cache-time symlink replacement and slow-drip protocol
waiting. A worker's PID remains reserved until its remaining group is cleaned;
actual exit status is preserved. Client/server frames have absolute monotonic
deadlines, separate from the unbounded-by-default reconciliation duration.
Independent slow-drip validation failed in 2.008 seconds, while a valid newline
frame returned in 0.009 seconds without waiting for peer EOF. The CI oracle also
now preserves the explicit stale-routing failure instead of parsing its empty
stdout as a successful report.

Actual 1.4.3 background adoption then exposed a coverage regression: 69
previously ready StockLens sources used the existing absolute
`/Users/ld/Workspaces` alias to `/Users/ld/Projects`. A strict lexical pre-gate
rejected them despite an existing exact active ProjectRef identity. The final
source restores bounded single-link alias discovery through stable metadata,
proves lexical expansion into an active physical boundary before target access,
and then applies full-path no-symlink, native Git and exact owner checks.
Cached observations bind the original logical cwd and are reselected before
reuse and publication. Relative/external/direct-suffix aliases, inode
replacement and retargeting to another member are exercised; the real daemon
E2E proves ready archive/query ownership and no external Git invocation.
Chained and multiple-resolution ambiguous cases have independent static
fail-closed review, not separate constructed tests. Metadata checks are not
atomic dir-fd proofs; before/after fingerprints and final revalidation retain
the existing same-user local threat model. The separate installed 1.4.4
Validator confirmed restoration of the exact 69-source cohort; that historical
observation does not substitute for final 1.4.5 installed acceptance.

Actual 1.4.4 startup, watcher and periodic rounds then repeatedly failed with
empty stdout and no new catalog while a normally appended historical source
changed between the coordinator's pre-fingerprint N and a valid Engine import
of stable N+1. The CLI snapshot mismatch incorrectly aborted the whole round,
unlike the existing per-source import-error path. The separate final runtime
Validator returned FAIL, preserved in Evidence
`01a13697-6d15-76aa-a74c-e912001d1d52`; intervening successful rounds did not
close that lane-isolation blocker.

The 1.4.5 correction keeps the mismatch fail-closed but local to that source.
N+1 is never published under the earlier owner or adopted into inventory.
Only a same-member, readback-validated prior archive may be retained; without
one, the source is failed without a fabricated Ready entry. Subsequent sources,
canonical generations, final control-plane fences, inventory and partial
catalog publication continue. The next ordinary round re-resolves a stable
source; no retry, quota, deadline, permission or legacy expansion is added.
Metadata-only diagnostics expose the compared byte counts/hashes and import
outcome, not message text.

Three deterministic tests use the real Engine through a private per-call
barrier seam, covering append with prior, append without prior plus a later
source, owner rewrite A to B without adopting N+1 under A, and stable next-round
convergence. The old implementation was falsified by the prior-append test's
global IntegrityInvalid; all three new tests passed in the final full suite
and an independent focused run. The worker's old-red and focused/static check
outputs were direct tool results, not saved raw transcripts. Independent
Review Evidence `01a136a4-45da-7437-aea1-ad8b0813c0e0` reports PASS with no
blocker/high finding. The diagnostic infers source change without an additional
post-import fingerprint to distinguish an Engine contract bug; either case
still rejects N+1. Combined mismatch/control-plane drift was statically traced
with existing fence tests, not a newly constructed combined test.

This source-stage snapshot does not itself prove live startup, login/restart
adoption, main push, CI or installed-build parity. Those exact observations and
the final independent Validator result must be recovered from the execution
Plan and its evidence before overall completion is claimed. Catalog partial
failure and concurrent canonical writes by other tasks must retain their
actual scope throughout that validation.
