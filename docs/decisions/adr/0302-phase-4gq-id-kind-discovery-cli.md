# ADR-0302: Phase 4GQ ID Kind Discovery CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs id new` and `workvcs id validate` require callers to provide a typed id
kind. Without a discovery command, manual users and scripts must rely on docs or
source inspection to learn the supported vocabulary.

## Decision

`workvcs id kinds` lists the typed id kinds supported by the CLI.

The output is store-independent and deterministic. The command uses the same
supported-kind list exercised by the id generation and validation tests.

## Consequences

Manual tooling can discover supported typed id kinds without opening a Store or
duplicating source-code knowledge. This does not add new id kinds, reserve ids,
or change the typed UUIDv7 identity scheme.
