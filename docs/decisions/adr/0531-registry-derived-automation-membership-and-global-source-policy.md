# ADR-0531: Registry-derived Automation Membership and Global Source Policy

Status: Accepted; local implementation and validation authorized
Date: 2026-10-13
Accepted by: The user's direct approval of the recommended post-V1 automation
evolution. The implementation contract is tracked in WorkVCS Plan
`01a13511-2ed9-7487-bdd8-9b26b5247940`.

## Context

ADR-0529 and ADR-0530 made optional local automation safe by requiring one
explicit registration per ProjectRef. That protects against implicit adoption,
but it does not meet the intended per-user behavior: a coordinator should see
every current and future valid ProjectRef in the selected registry without
requiring a second enrollment write for each project.

Membership and permission to read conversation roots are separate questions.
The ProjectRef registry can answer the former, but it must not cause arbitrary
filesystem discovery or make an automation write into a ProjectRef bootstrap.
Likewise, a rollout file's claimed working directory is not enough to turn its
body into canonical knowledge or silently choose a project.

## Decision

### Membership is a read-only registry-derived snapshot

For the selected registry-v2 route, each ProjectRef with a structurally valid
binding is a member of a reconciliation round. A round fixes the registry ID,
revision, digest and ordered member target tuples
(ProjectRef, Store, Workspace and Branch). New valid ProjectRefs become members
at the next round; removed, invalid or drifted members remain visible as
round-local failures rather than being repaired, materialized or omitted.

The coordinator never creates a ProjectRef, binding, locator, Store, Workspace
or Branch. A registry member does not need a filesystem source root and does
not grant source-discovery permission.

### Source authorization is a separate owner-private policy

An owner-private, versioned global source policy declares the exact local
rollout roots that may be scanned. Each policy entry has a stable ID, one safe
non-overlapping root, an explicit role policy, enabled/revoked state, and an
optional ProjectRef exclusion set. The default role policy is `user_only`;
including assistant text requires explicit per-entry opt-in. Developer,
system, tool and non-text material remain excluded unconditionally.

Policy preview is read-only and policy apply/revoke is compare-and-swap against
the exact policy revision and digest. A policy is neither a ProjectRef
registration nor a registry mutation. The service does not infer a root from a
ProjectRef, scan outside a policy root, or activate a policy as part of package
installation.

### Attribution is verified before body import

A recognised rollout is scanned once per approved root. Its session metadata
must provide one internally consistent, currently resolvable local ownership
context that maps to exactly one member in the selected registry. The resolver
revalidates the active binding and target tuple before archive import.

Conflicting metadata, unavailable or unsafe local context, an unbound or
non-member ProjectRef, a policy exclusion, and binding drift are quarantined
with a source-level error and no body indexing. A quarantined source does not
become evidence of deletion, currentness, ownership or action authority.

### Reconciliation binds both bases and stays derived-only

Every immutable catalog round records the registry membership basis and global
source-policy basis separately. Canonical generations are attempted once per
member; historical source files are scanned once per approved root and then
grouped by verified member attribution. `canonical_current` and
`historical_reference` remain independent lanes. Any member or source failure
is visible in the catalog and yields `partial_failure` rather than a silent
smaller result.

The coordinator may create new immutable derived generations, archives,
inventories and catalog manifests. It must not write canonical Stores, the
ProjectRef registry, Exposure/source-status, source files, existing archives or
existing generations. It performs no automatic cleanup.

### Legacy registrations are an isolated compatibility mode

The existing explicit-registration format remains readable for its historical
catalogs and for an owner who has not adopted a global source policy. New
global-policy reconciliation never combines legacy registrations with the
derived membership snapshot in one round: concurrent modes fail closed rather
than creating duplicate or ambiguous source ownership. Migration is an
explicit policy-adoption action; no existing registration is bulk-converted.

Daemon activation, LaunchAgent/startup installation, remote delivery, release,
and destructive cleanup remain separate user-authorized operations.

## Rejected alternatives

- Bulk-creating one registration for every ProjectRef.
- Scanning all home, project or session directories without a policy root.
- Mapping a rollout body by filename, display name, or an unchecked historical
  path.
- Treating a policy write as ProjectRef adoption or daemon activation.
- Dropping failed members or ambiguous sources to claim a complete catalog.

## Validation obligations

Validation covers heterogeneous valid ProjectRef locators, membership digest
changes, invalid binding isolation, policy preview/apply/revoke conflicts,
safe/unsafe/overlapping roots, user-only and assistant opt-in extraction,
consistent and conflicting session metadata, unbound/non-member/excluded
attribution, one-scan-per-root accounting, member and source partial failure,
immutable catalog basis/readback, and byte-stable canonical Store and registry
readback. Local implementation evidence must distinguish this capability from
an inactive daemon or uninstalled startup service.
