# Phase 5F User-Global Install Path Evidence

Status: current local evidence
Date: 2026-09-12

## Question

Does WorkVCS need another installation-selection mechanism after the
higher-priority truth, portability, multi-Agent, and default-stack work, or is
the existing explicit destination surface already reliable for this host?

## Evidence

The exact committed Phase 5E source
`40afdb7c3be7601e987f4d7ca7e5e53047c3f882` was packaged and installed with:

```bash
scripts/package-workvcs.sh --install --bin-dir "$HOME/.local/bin"
```

The package manifest recorded a clean source tree and that full Git commit.
The packaged and installed binary digests both equal
`e77058c0e4ba82b72082f6c5de836de832e2fb0cab59783b8bd6457b06b23a88`.
The packaged and installed five-file Skill-tree manifest digest both equal
`62fef0e23510cb207d01935d7751294ef85bf64ca8a11c457112c731286857e4`.

Two no-write previews selected the same destination, one through
`--bin-dir "$HOME/.local/bin"` and one through exact
`--dest "$HOME/.local/bin/workvcs"`. Both reported the binary and Skill
overwrites before execution. Independent checks then established:

- `$HOME/.local/bin/workvcs` exists and is executable;
- `command -v workvcs` resolves exactly that path;
- no `/usr/local/bin/workvcs` exists on this host;
- the user binary and Agent Skill parent directories are writable; and
- independent Skill-tree verification passes for all five files.

## Decision

Keep installation behavior unchanged. `--bin-dir`, `--prefix`, and `--dest`
already provide deterministic selection, dry-run exposes overwrite and system
path intent, installation is atomic, and post-install digest checks fail
closed. Adding another selector would duplicate working behavior without
removing a demonstrated failure.

The proportionate correction is documentation: make the user-global path the
primary non-elevated example, retain `/usr/local/bin` as the explicit
system-wide choice, and require `command -v workvcs` to prove discoverability.

## Boundary

This evidence applies to the current macOS host and the selected user-global
path. It does not claim that `$HOME/.local/bin` is already on every user's
`PATH`, choose a destination automatically, or authorize installation for
another host.
