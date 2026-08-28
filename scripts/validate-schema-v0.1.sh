#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(cd "$script_dir/.." && pwd -P)"
schema_path="$repo_root/schema/schema-v0.1.sql"
db_path="$(mktemp "${TMPDIR:-/tmp}/workvcs-schema-v0.1.XXXXXX.sqlite")"
trap 'rm -f "$db_path"' EXIT

# This is a schema/install/bootstrap harness, not a storage-engine
# implementation. Exact-family ownership, relation endpoint contracts,
# ChangeOperation typed-child exactness, merge result shape, Verification
# closure, and candidate Work-State validation are Engine validator contracts
# recorded in ADR-0007 and the physical schema contract.

sqlite3 "$db_path" < "$schema_path"

sqlite3 "$db_path" <<'SQL'
.bail on
PRAGMA foreign_keys = ON;

CREATE TEMP TABLE assert_true (
    label TEXT NOT NULL,
    ok INTEGER NOT NULL CHECK(ok)
) STRICT;

INSERT INTO assert_true
SELECT 'foreign_keys enabled', (PRAGMA_foreign_keys.foreign_keys = 1)
FROM pragma_foreign_keys() AS PRAGMA_foreign_keys;

INSERT INTO assert_true
SELECT 'application_id', (application_id = 1465271123)
FROM pragma_application_id();

INSERT INTO assert_true
SELECT 'table count', count(*) = 67
FROM sqlite_schema
WHERE type = 'table'
  AND name NOT LIKE 'sqlite_%';

INSERT INTO assert_true
SELECT 'only workspace genesis FK is deferred',
       count(*) = 1
FROM sqlite_schema
WHERE type = 'table'
  AND sql LIKE '%DEFERRABLE INITIALLY DEFERRED%';

INSERT INTO assert_true
SELECT 'merge attempt outcome table exists',
       count(*) = 1
FROM sqlite_schema
WHERE type = 'table'
  AND name = 'merge_attempt_outcome';

INSERT INTO assert_true
SELECT 'merge resolution has no finalization kind',
       NOT EXISTS (
           SELECT 1 FROM pragma_table_info('merge_resolution')
           WHERE name = 'finalization_kind'
       );

INSERT INTO assert_true
SELECT 'external object partial unique index exists',
       count(*) = 1
FROM sqlite_schema
WHERE type = 'index'
  AND name = 'uq_external_object_ref_object';

INSERT INTO assert_true
SELECT 'external version partial unique index exists',
       count(*) = 1
FROM sqlite_schema
WHERE type = 'index'
  AND name = 'uq_external_object_ref_version';

INSERT INTO assert_true
SELECT 'exposure initial partial unique index exists',
       count(*) = 1
FROM sqlite_schema
WHERE type = 'index'
  AND name = 'uq_exposure_initial_transition';

INSERT INTO assert_true
SELECT 'projection commit digest pair check exists',
       count(*) = 1
FROM sqlite_schema
WHERE type = 'table'
  AND name = 'branch_projection_state'
  AND sql LIKE '%projected_commit_id IS NULL AND projection_state_digest IS NULL%'
  AND sql LIKE '%projected_commit_id IS NOT NULL AND projection_state_digest IS NOT NULL%';

INSERT INTO assert_true
SELECT 'applicability stamp nullable observation states exist',
       count(*) = 1
FROM sqlite_schema
WHERE type = 'table'
  AND name = 'applicability_resource_stamp'
  AND sql LIKE '%observation_status%'
  AND sql LIKE '%unavailable%'
  AND sql LIKE '%observed_fingerprint IS NULL%';

BEGIN IMMEDIATE;

INSERT INTO store(store_id, created_at_us, display_name, metadata_json)
VALUES (x'00000000000000000000000000000001', 1000, 'default-store', '{}');

INSERT INTO store_manifest(
    store_id,
    store_format_version,
    schema_version,
    object_store_format_version,
    id_scheme,
    digest_algorithm,
    canonical_json_profile,
    created_at_us,
    manifest_json
)
VALUES (
    x'00000000000000000000000000000001',
    1,
    1,
    1,
    'uuidv7-blob16',
    'blake3-256',
    'workvcs-jcs-v1',
    1001,
    '{"canonical_json_profile":"workvcs-jcs-v1","digest_algorithm":"blake3-256","id_scheme":"uuidv7-blob16","object_store_format_version":1,"schema_version":1,"store_format_version":1}'
);

INSERT INTO workspace(workspace_id, store_id, display_name, genesis_commit_id, created_at_us)
VALUES (
    x'00000000000000000000000000000002',
    x'00000000000000000000000000000001',
    'default-workspace',
    x'00000000000000000000000000000004',
    1002
);

INSERT INTO changeset(
    changeset_id,
    workspace_id,
    operation_type,
    operation_schema_version,
    operation_payload_json,
    rationale_json,
    created_at_us
)
VALUES (
    x'00000000000000000000000000000003',
    x'00000000000000000000000000000002',
    'workspace.genesis',
    1,
    '{}',
    '{}',
    1003
);

INSERT INTO workstate_commit(
    commit_id,
    workspace_id,
    changeset_id,
    commit_kind,
    state_digest,
    committed_at_us
)
VALUES (
    x'00000000000000000000000000000004',
    x'00000000000000000000000000000002',
    x'00000000000000000000000000000003',
    'genesis',
    x'0000000000000000000000000000000000000000000000000000000000000000',
    1004
);

INSERT INTO branch(branch_id, workspace_id, name, head_commit_id, lifecycle_state, created_at_us)
VALUES (
    x'00000000000000000000000000000005',
    x'00000000000000000000000000000002',
    'main',
    x'00000000000000000000000000000004',
    'active',
    1005
);

INSERT INTO event(event_id, workspace_id, changeset_id, event_kind, occurred_at_us, payload_json)
VALUES (
    x'00000000000000000000000000000006',
    x'00000000000000000000000000000002',
    x'00000000000000000000000000000003',
    'workspace.initialized',
    1006,
    '{}'
);

COMMIT;

INSERT INTO assert_true
SELECT 'store bootstrap count', (SELECT count(*) FROM store) = 1
   AND (SELECT count(*) FROM store_manifest) = 1;

INSERT INTO assert_true
SELECT 'workspace genesis committed',
       (SELECT genesis_commit_id FROM workspace WHERE workspace_id = x'00000000000000000000000000000002')
       = x'00000000000000000000000000000004';

INSERT INTO assert_true
SELECT 'genesis has zero parents',
       NOT EXISTS (
           SELECT 1 FROM commit_parent
           WHERE commit_id = x'00000000000000000000000000000004'
       );

INSERT INTO assert_true
SELECT 'genesis has zero change operations',
       NOT EXISTS (
           SELECT 1
           FROM change_operation
           WHERE changeset_id = x'00000000000000000000000000000003'
       );

INSERT INTO assert_true
SELECT 'initial branch head is genesis',
       (SELECT head_commit_id FROM branch WHERE branch_id = x'00000000000000000000000000000005')
       = x'00000000000000000000000000000004';

BEGIN IMMEDIATE;

INSERT INTO object_identity(object_id, object_kind, created_at_us)
VALUES (x'00000000000000000000000000000007', 'entity', 2000);

INSERT INTO entity(object_id, workspace_id, entity_kind)
VALUES (
    x'00000000000000000000000000000007',
    x'00000000000000000000000000000002',
    'task'
);

INSERT INTO entity_version(
    entity_version_id,
    entity_id,
    state_schema_version,
    state_json,
    state_digest
)
VALUES (
    x'00000000000000000000000000000008',
    x'00000000000000000000000000000007',
    1,
    '{"status":"open"}',
    x'1111111111111111111111111111111111111111111111111111111111111111'
);

INSERT INTO changeset(
    changeset_id,
    workspace_id,
    operation_type,
    operation_schema_version,
    operation_payload_json,
    rationale_json,
    created_at_us
)
VALUES (
    x'00000000000000000000000000000009',
    x'00000000000000000000000000000002',
    'entity.create',
    1,
    '{}',
    '{}',
    2001
);

INSERT INTO change_operation(
    operation_id,
    changeset_id,
    ordinal,
    subject_family,
    subject_object_id,
    operation_payload_json
)
VALUES (
    x'0000000000000000000000000000000a',
    x'00000000000000000000000000000009',
    0,
    'entity',
    x'00000000000000000000000000000007',
    '{}'
);

INSERT INTO entity_membership_change(
    operation_id,
    entity_id,
    before_entity_version_id,
    after_entity_version_id,
    field_delta_json
)
VALUES (
    x'0000000000000000000000000000000a',
    x'00000000000000000000000000000007',
    NULL,
    x'00000000000000000000000000000008',
    '{}'
);

INSERT INTO workstate_commit(
    commit_id,
    workspace_id,
    changeset_id,
    commit_kind,
    state_digest,
    committed_at_us
)
VALUES (
    x'0000000000000000000000000000000b',
    x'00000000000000000000000000000002',
    x'00000000000000000000000000000009',
    'normal',
    x'2222222222222222222222222222222222222222222222222222222222222222',
    2002
);

INSERT INTO commit_parent(commit_id, parent_ordinal, parent_role, parent_commit_id)
VALUES (
    x'0000000000000000000000000000000b',
    0,
    'primary',
    x'00000000000000000000000000000004'
);

UPDATE branch
SET head_commit_id = x'0000000000000000000000000000000b'
WHERE branch_id = x'00000000000000000000000000000005'
  AND head_commit_id = x'00000000000000000000000000000004';

INSERT INTO assert_true SELECT 'branch head CAS success', changes() = 1;

UPDATE branch
SET head_commit_id = x'0000000000000000000000000000000b'
WHERE branch_id = x'00000000000000000000000000000005'
  AND head_commit_id = x'00000000000000000000000000000004';

INSERT INTO assert_true SELECT 'branch head stale CAS rejected', changes() = 0;

COMMIT;

INSERT INTO assert_true
SELECT 'foreign key check clean',
       NOT EXISTS (SELECT 1 FROM pragma_foreign_key_check());

SELECT 'schema-v0.1 validation ok';
SQL
