# ADR-0522: Runtime Entry Compatibility and Artifact Lifecycle

Status: Accepted; source implemented and locally validated
Date: 2026-10-10

## Context

The registry-v2 control plane may contain the `binding_source=isolation` value.
An older WorkVCS binary that predates shared-binding isolation rejects that
valid current registry with `unknown variant isolation`. The failure is a
binary/schema compatibility mismatch, not evidence that the current registry
is corrupt. The local machine also retained several dated package and adoption
copies, so an absolute path could silently select an old binary even when the
shell's `workvcs` command resolved to the stable installation.

The existing v1-to-v2 migration is deliberately preview- and digest-locked,
but operators need a read-only compatibility result before choosing that route.
They also need an auditable way to distinguish the stable installed CLI from a
development candidate or a historical package artifact.

## Decision

### 1. Stable CLI is the non-development entrypoint

All non-WorkVCS-development operations use the configured stable installation.
Candidate binaries under a source `target` tree are valid only for WorkVCS
development and validation. Dated `adoption` and package artifacts are not
live entrypoints.

The CLI exposes the read-only command:

```text
workvcs runtime status [--require-stable] [--format text|json]
```

It reports the resolved executable path, build commit, dirty-source marker,
binary digest, and an execution role. `--require-stable` fails closed when the
binary is not the configured stable installation path. The optional
`WORKVCS_STABLE_PATH` variable supplies an exact non-default stable path;
`$HOME/.local/bin/workvcs` and `/usr/local/bin/workvcs` remain the defaults.

### 2. Compatibility is explicit and read-only before migration

The registry command gains:

```text
workvcs project registry-migrate --compatibility [--registry PATH]
```

The command reads one exact registry and reports whether the current CLI can
read it, whether v1-to-v2 migration is required, the raw and canonical
digests when available, and the next safe action. A readable v1 registry points
to `--preview` followed by digest-locked `--apply`. A readable v2 registry,
including `binding_source=isolation`, is used as-is by the current stable CLI.
Unknown versions, unknown v2 enum values, malformed JSON, and invalid shapes
remain fail-closed: the command never rewrites, coerces, or discards unknown
semantics and instead preserves the raw bytes and directs the operator to a
newer compatible CLI.

### 3. Historical package artifacts have an explicit bounded cleanup path

The repository provides:

```text
scripts/prune-workvcs-artifacts.sh --root DIR [--root DIR ...]
```

It removes only explicitly named, direct `workvcs-*` package directories and
matching archives whose manifest identifies WorkVCS and whose binary digest
matches that manifest. Unrecognized, symlinked, or tampered entries are
preserved. `--keep-artifact DIR` retains the current package. Cleanup is run
after implementation, installation, Push, stable-entry readback, and target
session reconciliation; it never removes source, registry, Store, WorkVCS
records, or the retained current package.

## Consequences

- A stable-entry check makes accidental use of an absolute historical binary
  visible before control-plane reads or writes.
- A new CLI can distinguish legacy migration from an unsupported future shape
  without turning a read-only diagnostic into a mutation.
- The existing digest-locked v1-to-v2 migration and fail-closed unknown-schema
  behavior remain unchanged.
- Package history is no longer an implicit long-lived executable inventory;
  cleanup is exact, explicit, and independently auditable.

## Validation

1. The runtime status command identifies the installed stable binary and
   rejects an executable under a development or historical artifact path when
   `--require-stable` is supplied.
2. Compatibility status reports v1 as migration-required, current v2 with
   `isolation` as readable, and unknown v2 values as unsupported without
   writing bytes.
3. Cleanup removes only manifest- and digest-verified package artifacts and
   preserves the exact current package plus unrecognized entries.
4. The installed stable binary reads the live registry and the target session
   is processed only after source validation, installation, Push, and exact
   stable-entry readback.
