use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    CanonicalValue, Engine, ErrorCategory, ErrorCode, ResourceBindOptions, ResourceCreateOptions,
    ResourceId, ResourceObservationCreateOptions, ResourceObservationDetailInput, SessionId,
    SessionStartOptions, StoreInitOptions, WorkspaceId, WorkspaceInitOptions,
    WorkspaceResourceAssociationOptions, content_object_digest,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase3x-resource-runtime-foundation-store").expect("store options"),
    )
    .expect("init engine")
}

fn create_workspace(path: &Path) -> (Engine, workvcs_core::WorkspaceInfo) {
    let mut engine = init_engine(path);
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn raw_connection(path: &Path) -> Connection {
    let connection = Connection::open(path).expect("raw connection");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("enable raw foreign keys");
    connection
}

fn object(entries: Vec<(&str, CanonicalValue)>) -> CanonicalValue {
    CanonicalValue::object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
    .expect("canonical object")
}

fn string(value: &str) -> CanonicalValue {
    CanonicalValue::String(value.to_owned())
}

fn count_rows(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count rows")
}

fn object_kind(connection: &Connection, object_id: [u8; 16]) -> String {
    connection
        .query_row(
            "SELECT object_kind FROM object_identity WHERE object_id = ?1",
            params![&object_id[..]],
            |row| row.get(0),
        )
        .expect("object kind")
}

#[test]
fn resource_create_bind_and_workspace_association_are_store_level_state() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let connection = raw_connection(&path);
    let before_workstate_commits = count_rows(&connection, "workstate_commit");
    let before_entities = count_rows(&connection, "entity");

    let resource = engine
        .create_resource(ResourceCreateOptions::new("git").expect("resource options"))
        .expect("create resource");

    assert_eq!(resource.resource_kind, "git");
    assert_eq!(resource.state.resource_id, resource.resource_id);
    assert_eq!(resource.state.resource_kind, "git");
    assert!(resource.state.binding.is_none());
    assert!(resource.state.workspace_associations.is_empty());
    assert_eq!(
        object_kind(&connection, resource.resource_id.raw_bytes()),
        "resource"
    );
    assert_eq!(count_rows(&connection, "entity"), before_entities);
    assert_eq!(
        count_rows(&connection, "workstate_commit"),
        before_workstate_commits
    );

    let binding = engine
        .bind_resource(
            ResourceBindOptions::new(resource.resource_id, "git", "/tmp/workvcs")
                .expect("bind options")
                .with_binding_config(object(vec![("include_untracked", string("true"))]))
                .expect("binding config"),
        )
        .expect("bind resource");
    assert_eq!(binding.state.adapter_kind, "git");
    assert_eq!(binding.state.locator, "/tmp/workvcs");
    assert_eq!(
        binding.state.binding_config,
        object(vec![("include_untracked", string("true"))])
    );

    let association = engine
        .associate_workspace_resource(
            WorkspaceResourceAssociationOptions::new(workspace.workspace_id, resource.resource_id)
                .expect("association options")
                .with_association_metadata(object(vec![("role", string("primary"))]))
                .expect("association metadata"),
        )
        .expect("associate resource");
    assert_eq!(association.workspace_id, workspace.workspace_id);
    assert_eq!(association.resource_id, resource.resource_id);
    assert_eq!(
        association.state.association_metadata,
        object(vec![("role", string("primary"))])
    );

    let snapshot = engine.resource(resource.resource_id).expect("resource");
    assert_eq!(snapshot.binding.as_ref(), Some(&binding.state));
    assert_eq!(snapshot.workspace_associations, vec![association.state]);
    assert_eq!(count_rows(&connection, "resource_binding"), 1);
    assert_eq!(count_rows(&connection, "workspace_resource"), 1);
    assert_eq!(
        count_rows(&connection, "workstate_commit"),
        before_workstate_commits
    );

    let rebound = engine
        .bind_resource(
            ResourceBindOptions::new(resource.resource_id, "filesystem", "/tmp/workvcs-copy")
                .expect("rebind options"),
        )
        .expect("rebind resource");
    let reassociated = engine
        .associate_workspace_resource(
            WorkspaceResourceAssociationOptions::new(workspace.workspace_id, resource.resource_id)
                .expect("reassociation options")
                .with_association_metadata(object(vec![("role", string("secondary"))]))
                .expect("reassociation metadata"),
        )
        .expect("reassociate resource");

    let snapshot = engine.resource(resource.resource_id).expect("resource");
    assert_eq!(snapshot.binding.as_ref(), Some(&rebound.state));
    assert_eq!(snapshot.workspace_associations, vec![reassociated.state]);
    assert_eq!(count_rows(&connection, "resource_binding"), 1);
    assert_eq!(count_rows(&connection, "workspace_resource"), 1);
    assert_eq!(
        count_rows(&connection, "workstate_commit"),
        before_workstate_commits
    );
}

#[test]
fn resource_observation_records_adapter_fingerprint_and_optional_detail_content() {
    let (_tempdir, path) = store_path();
    let (mut engine, _workspace) = create_workspace(&path);
    let resource = engine
        .create_resource(ResourceCreateOptions::new("directory").expect("resource options"))
        .expect("create resource");
    let connection = raw_connection(&path);
    let before_workstate_commits = count_rows(&connection, "workstate_commit");
    let detail_bytes = b"normalized file manifest\n";
    let detail = ResourceObservationDetailInput::from_raw_bytes(detail_bytes)
        .expect("detail bytes")
        .with_media_type("text/plain")
        .expect("detail media")
        .with_format_metadata(object(vec![("encoding", string("utf-8"))]))
        .expect("detail format metadata");
    let fingerprint = content_object_digest(b"normalized resource fingerprint");

    let observation = engine
        .record_resource_observation(
            ResourceObservationCreateOptions::new(
                resource.resource_id,
                "filesystem",
                1,
                fingerprint,
                object(vec![(
                    "paths",
                    CanonicalValue::safe_integer(3).expect("integer"),
                )]),
            )
            .expect("observation options")
            .with_detail_content(detail),
        )
        .expect("record observation");
    let snapshot = engine
        .resource_observation(observation.observation_id)
        .expect("observation");

    assert_eq!(snapshot.resource_id, resource.resource_id);
    assert_eq!(snapshot.adapter_kind, "filesystem");
    assert_eq!(snapshot.adapter_schema_version, 1);
    assert_eq!(snapshot.fingerprint, fingerprint);
    assert_eq!(
        object_kind(&connection, observation.observation_id.raw_bytes()),
        "resource_observation"
    );
    let detail_snapshot = snapshot.detail_content.expect("detail content");
    assert_eq!(
        detail_snapshot.content_digest,
        content_object_digest(detail_bytes)
    );
    assert_eq!(detail_snapshot.size_bytes, detail_bytes.len() as i64);
    assert_eq!(detail_snapshot.media_type.as_deref(), Some("text/plain"));
    assert_eq!(
        detail_snapshot.format_metadata,
        object(vec![("encoding", string("utf-8"))])
    );
    assert_eq!(count_rows(&connection, "content_object"), 1);
    assert_eq!(count_rows(&connection, "content_storage_location"), 0);
    assert_eq!(
        count_rows(&connection, "workstate_commit"),
        before_workstate_commits
    );

    engine
        .record_resource_observation(
            ResourceObservationCreateOptions::new(
                resource.resource_id,
                "filesystem",
                1,
                fingerprint,
                object(vec![(
                    "paths",
                    CanonicalValue::safe_integer(3).expect("integer"),
                )]),
            )
            .expect("second observation options")
            .with_detail_content(
                ResourceObservationDetailInput::from_raw_bytes(detail_bytes)
                    .expect("same detail bytes")
                    .with_media_type("text/plain")
                    .expect("same detail media")
                    .with_format_metadata(object(vec![("encoding", string("utf-8"))]))
                    .expect("same detail format metadata"),
            ),
        )
        .expect("second observation reuses content metadata");
    assert_eq!(count_rows(&connection, "content_object"), 1);

    let conflicting_detail =
        ResourceObservationDetailInput::from_digest(content_object_digest(detail_bytes), 25)
            .expect("conflicting detail")
            .with_media_type("application/json")
            .expect("conflicting media");
    let before_observations = count_rows(&connection, "resource_observation");
    let error = engine
        .record_resource_observation(
            ResourceObservationCreateOptions::new(
                resource.resource_id,
                "filesystem",
                1,
                fingerprint,
                object(vec![(
                    "paths",
                    CanonicalValue::safe_integer(3).expect("integer"),
                )]),
            )
            .expect("conflict options")
            .with_detail_content(conflicting_detail),
        )
        .expect_err("conflicting content metadata should be rejected");

    assert_eq!(error.code(), ErrorCode::ResourceInvalid);
    assert_eq!(error.category(), ErrorCategory::Resource);
    assert_eq!(
        count_rows(&connection, "resource_observation"),
        before_observations
    );
}

#[test]
fn resource_observation_records_and_validates_optional_source_session() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let resource = engine
        .create_resource(ResourceCreateOptions::new("git").expect("resource options"))
        .expect("create resource");
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    let fingerprint = content_object_digest(b"session scoped observation");

    let observation = engine
        .record_resource_observation(
            ResourceObservationCreateOptions::new(
                resource.resource_id,
                "git",
                1,
                fingerprint,
                object(vec![("head", string("abc123"))]),
            )
            .expect("observation options")
            .with_source_session_id(session.session_id),
        )
        .expect("record observation with source session");

    let snapshot = engine
        .resource_observation(observation.observation_id)
        .expect("observation");
    assert_eq!(snapshot.source_session_id, Some(session.session_id));
    assert_eq!(snapshot.fingerprint, fingerprint);

    let connection = raw_connection(&path);
    let before_observations = count_rows(&connection, "resource_observation");
    let missing_session = SessionId::new_v7();
    let error = engine
        .record_resource_observation(
            ResourceObservationCreateOptions::new(
                resource.resource_id,
                "git",
                1,
                content_object_digest(b"missing source session"),
                object(Vec::new()),
            )
            .expect("missing session observation options")
            .with_source_session_id(missing_session),
        )
        .expect_err("missing source session should be rejected");

    assert_eq!(error.code(), ErrorCode::SessionNotFound);
    assert_eq!(error.category(), ErrorCategory::Runtime);
    assert_eq!(
        count_rows(&connection, "resource_observation"),
        before_observations
    );

    connection
        .pragma_update(None, "foreign_keys", "OFF")
        .expect("disable foreign keys for corruption");
    connection
        .execute(
            "UPDATE resource_observation
             SET source_session_id = ?1
             WHERE observation_id = ?2",
            params![
                &missing_session.raw_bytes()[..],
                &observation.observation_id.raw_bytes()[..]
            ],
        )
        .expect("corrupt source session");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("enable foreign keys after corruption");

    let readback_error = engine
        .resource_observation(observation.observation_id)
        .expect_err("readback should reject missing source session");
    assert_eq!(readback_error.code(), ErrorCode::SessionNotFound);
    assert_eq!(readback_error.category(), ErrorCategory::Runtime);
}

#[test]
fn resource_runtime_foundation_rejects_missing_or_invalid_inputs() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let missing_resource = ResourceId::new_v7();

    let invalid_kind = ResourceCreateOptions::new(" git").expect_err("kind has leading space");
    assert_eq!(invalid_kind.code(), ErrorCode::ResourceInvalid);
    assert_eq!(invalid_kind.category(), ErrorCategory::Resource);

    let bind_error = engine
        .bind_resource(
            ResourceBindOptions::new(missing_resource, "git", "/tmp/missing")
                .expect("missing bind options"),
        )
        .expect_err("missing resource should be rejected");
    assert_eq!(bind_error.code(), ErrorCode::ResourceNotFound);
    assert_eq!(bind_error.category(), ErrorCategory::Resource);

    let resource = engine
        .create_resource(ResourceCreateOptions::new("git").expect("resource options"))
        .expect("create resource");
    let missing_workspace = WorkspaceId::new_v7();
    let association_error = engine
        .associate_workspace_resource(
            WorkspaceResourceAssociationOptions::new(missing_workspace, resource.resource_id)
                .expect("missing workspace association options"),
        )
        .expect_err("missing workspace should be rejected");
    assert_eq!(association_error.code(), ErrorCode::WorkspaceNotFound);
    assert_eq!(association_error.category(), ErrorCategory::Workspace);

    let non_object_summary = ResourceObservationCreateOptions::new(
        resource.resource_id,
        "filesystem",
        1,
        content_object_digest(b"fingerprint"),
        CanonicalValue::String("not an object".to_owned()),
    )
    .expect_err("summary must be object");
    assert_eq!(non_object_summary.code(), ErrorCode::ResourceInvalid);

    let invalid_version = ResourceObservationCreateOptions::new(
        resource.resource_id,
        "filesystem",
        0,
        content_object_digest(b"fingerprint"),
        object(Vec::new()),
    )
    .expect_err("adapter schema version must be positive");
    assert_eq!(invalid_version.code(), ErrorCode::ResourceInvalid);

    let observation_error = engine
        .record_resource_observation(
            ResourceObservationCreateOptions::new(
                missing_resource,
                "filesystem",
                1,
                content_object_digest(b"fingerprint"),
                object(Vec::new()),
            )
            .expect("missing resource observation options"),
        )
        .expect_err("missing resource observation should be rejected");
    assert_eq!(observation_error.code(), ErrorCode::ResourceNotFound);

    assert!(
        engine
            .associate_workspace_resource(
                WorkspaceResourceAssociationOptions::new(
                    workspace.workspace_id,
                    resource.resource_id,
                )
                .expect("valid association options"),
            )
            .is_ok()
    );
}
