# Task Disposition and Currentness Local Adoption Evidence

Status: Local installation and bounded stable adoption verified
Date: 2026-10-11
Decision: [ADR-0524](../decisions/adr/0524-task-disposition-and-execution-currentness.md)

## Scope

This record covers the local installation and bounded stable-fixture adoption
of WorkVCS 1.0.0 after ADR-0524 was accepted. It ties the installed binary and
Skill to the verified clean source build, confirms that the existing local
ProjectRef control plane is healthy without a refresh or migration, and
exercises the new typed Plan-evolution route only against an isolated fixture.

The retained execution evidence is under
`execution/adoption-task-disposition-20261011` in the coordinating workspace.
The stable-fixture summary SHA-256 is
`f7c0dd97bf915153e29f630de311aec580521a462daeabfff82661011b7591c0`.
Its 96-file raw evidence index SHA-256 is
`af40448ca3c267de89f6c8991be4c8fa88f25f56951218c453eb1dd7f3c01c48`.

## Source, package, and installation parity

The installed stable entrypoint is `$HOME/.local/bin/workvcs`:

- version `1.0.0`;
- clean build commit `f66f34b3572ed800bdafc3c88a8da64d108d15ef`;
- BLAKE3 binary digest
  `d118e65a44a6f1cdd4074bf8288d9d58ca1a435c80e620c0f302cf34f6af7c45`;
- package and installed binary SHA-256
  `ae935210866dd57be8f0b1f7f4cfd13dcf81f0f5e9c5e89b79a74e981a622e5d`; and
- seven-file installed WorkVCS Skill tree identical to source and package.

`workvcs runtime status --require-stable` reported the installed role, clean
build, matching digest, and no registry, journal, or Store write.

The installed work-governance 2.2.0 cache has source parity across 28 files,
is enabled at commit `69b1f3ac52b90d01e57c55c6eed9772c8e82d8f0`, and a fresh
ephemeral process explicitly read its complete `plan-governance` Skill before a
bounded retain-original assessment. The initial long Skill injection was
truncated by context limits, and this does not establish hot reload in an
older running conversation.

## Control plane and typed operation evidence

The installed read-only health check reported registry v2 revision 3, seven of
seven valid bindings, 74 local content objects verified, active read routing,
and active journal-admission version 2 with cognition capture, Plan admit, and
Plan evolve capabilities. These read-only probes wrote no registry, marker,
journal, or Store state; no migration or activation refresh occurred.
Separately authorized source-project semantic checkpoints did write the shared
journal and their bound Stores, with exact delivery receipts and readback.

An independent stable-fixture validation passed with the installed binary. A
mixed schema-2 `plan_evolve_v2` operation continued two Tasks and retained two
Tasks while every original Task identity, status, version, and digest remained
unchanged. The read-only Task-disposition audit and exact typed retry left their
authoritative fixture byte snapshots unchanged; the retry reused a completed
receipt. An all-`retain_original` operation preserved the Work-State digest,
passed doctor, and exactly reused on retry.

## Independent review and limits

Independent installation review found no material findings. It verified the
clean source ancestry, installed/package parity, WorkVCS Skill parity,
work-governance cache parity, and the bounded fresh-process Skill read.

This evidence is local and fixture-scoped. It does not prove concurrent-writer
or crash behavior, real HXAP task quality, HXAP reads or writes, remote Push,
CI for this source change, release, cleanup, production behavior, or authority
for any external execution. Local tracking refs remained at older remote heads
when this evidence was recorded.
