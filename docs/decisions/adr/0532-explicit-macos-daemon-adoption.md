# ADR-0532: Explicit macOS Daemon Adoption

Status: Accepted; source implemented and locally validated
Date: 2026-10-13
Accepted by: The user's explicit instruction to complete the remaining global
daemon startup task, including installation, verification and delivery.
Execution Plan: `01a135cc-272b-75f4-b93a-477d966a84bd`.

## Decision

The per-user coordinator remains disabled by default. macOS operators may
explicitly install and start `io.workvcs.automation` through
`scripts/workvcs-launchagent.sh`. Ordinary binary or Skill installation does
not invoke this script. Rendering is zero-write; plist installation and
launchd loading are separate actions.

The exact stable installed executable, selected ProjectRef registry, derived
automation root and anti-entropy interval are fixed in the owner-controlled
plist. RunAtLoad enables login startup, KeepAlive provides process recovery,
and Umask preserves private runtime files. The service runs as the current
user with an owner-private Unix socket, without a TCP port or privilege change.
Different existing plists and symlinks are preserved and refused. Identical
installation is a no-op. Disable unloads the job while retaining its plist;
start explicitly re-enables it.

In registry-derived mode, watcher targets include the selected registry,
source-policy control file, member Stores and explicitly allowed rollout
roots. Control-plane changes refresh the targets. Watchers remain dirty hints;
full reconciliation determines membership, attribution and catalog currentness.
Derived outputs must not create a watcher feedback loop.

The coordinator cannot attest that an OS startup item is installed. Its ledger
reports `launch_agent_installed: null`; the installer inspects the exact plist
and launchctl state to report installed, loaded, running and disabled state.
Process health, worker outcome, catalog completeness and startup adoption each
have their own observation. Historical quarantines remain visible and do not
become successful observations merely because the process is running.

Start acceptance binds the launchctl process PID to the IPC ledger PID as well
as the exact plist, executable and full argument vector. Enabled-state output
is normalised across macOS representations; unknown state cannot establish
auto-start. Protocol I/O has its own finite response boundary, independent of
the duration or amount of reconciliation work.

The first real global reconciliation on the adoption machine took 306 seconds,
exceeding the earlier fixed 300-second worker cutoff. Workers therefore have
no implicit wall-clock cutoff. An operator may explicitly select a deadline
with `--worker-timeout-seconds`; zero means none. The coordinator polls one
asynchronous worker, serves status during startup and long scans, coalesces
pending triggers, and cancels and reaps an active worker on an explicit stop.
Each worker has a dedicated process group so cancellation also terminates its
descendant probes before stream cleanup.
Accepted/queued requests are distinct from validated completion. These rules
supersede ADR-0529's fixed worker-runtime boundary while preserving its socket,
stream-evidence and immutable-publication contracts.

## Adoption acceptance

Validate a finite real reconciliation before resident adoption. Observe the
exact installed process, allowed-source changes, catalog publication and
recovery after an explicit service restart. Inspect startup configuration
without rebooting the user's machine. Preserve full worker/catalog evidence
and explicitly describe quarantines, failures and remaining scope.

Canonical Store/registry semantics, role-policy restrictions, source files,
immutable artifacts and automatic-cleanup prohibitions remain governed by
ADR-0529 through ADR-0531.

Source acceptance and independent review are recorded in
[Global Daemon Source Validation](../../provenance/global-daemon-source-validation.md).
Actual installed/runtime acceptance is tracked by the execution Plan above.
