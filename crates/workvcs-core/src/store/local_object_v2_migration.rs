use crate::canonical::{CanonicalValue, canonical_bytes};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::history::LOCAL_CONTENT_STORAGE_BACKEND;
use crate::identity::{Digest, MigrationId, StoreId};
use crate::store::bootstrap::{
    OBJECT_STORE_FORMAT_VERSION, SCHEMA_VERSION, STORE_FORMAT_VERSION, StoreInfo, StoreManifest,
    current_epoch_micros, load_store_info_for_local_object_v2_migration, validate_application_id,
    validate_bootstrap,
};
use crate::store::connection::StoreConnection;
use crate::store::open::{
    Store, local_content_relative_path, persist_local_content_object, relative_path_to_locator,
    verify_local_content_file,
};
use crate::store::schema;
use rusqlite::{TransactionBehavior, params};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const LEGACY_OBJECT_STORE_FORMAT_VERSION: i64 = 1;
const LEGACY_STORAGE_BACKEND: &str = "workvcs.local-object-v1";
const LEGACY_PATH_SEGMENT: &str = "sha256";
const MIGRATION_TOOL_VERSION: &str = "workvcs-local-object-v2-cutover-v1";
const LOCAL_CONTENT_OBJECT_DIR: &str = ".workvcs-objects";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalObjectV2MigrationMode {
    Preflight,
    Applied,
    AlreadyCurrent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalObjectV2MigrationOptions {
    apply: bool,
    expected_local_objects: Option<usize>,
}

impl LocalObjectV2MigrationOptions {
    pub fn preflight() -> Self {
        Self {
            apply: false,
            expected_local_objects: None,
        }
    }

    pub fn apply() -> Self {
        Self {
            apply: true,
            expected_local_objects: None,
        }
    }

    pub fn with_expected_local_objects(mut self, expected: usize) -> Self {
        self.expected_local_objects = Some(expected);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalObjectV2MigrationResult {
    pub mode: LocalObjectV2MigrationMode,
    pub store_info: StoreInfo,
    pub local_objects: usize,
    pub preexisting_target_objects: usize,
    pub legacy_root: PathBuf,
    pub target_root: PathBuf,
    pub migration_id: Option<MigrationId>,
}

#[derive(Clone, Debug)]
struct LegacyObject {
    digest: Digest,
    size_bytes: i64,
    old_locator: String,
    new_locator: String,
    old_path: PathBuf,
    raw_bytes: Vec<u8>,
    target_preexisting: bool,
}

#[derive(Debug)]
struct Preflight {
    store_info: StoreInfo,
    legacy_root: PathBuf,
    target_root: PathBuf,
    objects: Vec<LegacyObject>,
}

pub fn migrate_local_object_store_v2(
    store_path: &Path,
    options: LocalObjectV2MigrationOptions,
) -> Result<LocalObjectV2MigrationResult> {
    let preflight = preflight(store_path)?;
    if preflight.store_info.manifest.object_store_format_version == OBJECT_STORE_FORMAT_VERSION {
        let store = Store::open(store_path)?;
        let local_objects = store.validate_local_content_storage()?;
        if let Some(expected) = options.expected_local_objects
            && local_objects != expected
        {
            return Err(WorkVcsError::StoreBootstrapInvalid(format!(
                "local object validation found {local_objects} object(s), expected {expected}"
            )));
        }
        return Ok(LocalObjectV2MigrationResult {
            mode: LocalObjectV2MigrationMode::AlreadyCurrent,
            store_info: preflight.store_info,
            local_objects,
            preexisting_target_objects: local_objects,
            legacy_root: preflight.legacy_root,
            target_root: preflight.target_root,
            migration_id: None,
        });
    }

    if let Some(expected) = options.expected_local_objects
        && preflight.objects.len() != expected
    {
        return Err(WorkVcsError::StoreBootstrapInvalid(format!(
            "local object migration found {} object(s), expected {expected}",
            preflight.objects.len()
        )));
    }

    let preexisting_target_objects = preflight
        .objects
        .iter()
        .filter(|object| object.target_preexisting)
        .count();
    if !options.apply {
        return Ok(LocalObjectV2MigrationResult {
            mode: LocalObjectV2MigrationMode::Preflight,
            store_info: preflight.store_info,
            local_objects: preflight.objects.len(),
            preexisting_target_objects,
            legacy_root: preflight.legacy_root,
            target_root: preflight.target_root,
            migration_id: None,
        });
    }

    for object in &preflight.objects {
        let locator = persist_local_content_object(
            store_path,
            preflight.store_info.store_id,
            object.digest,
            object.size_bytes,
            &object.raw_bytes,
        )?;
        if locator != object.new_locator {
            return Err(WorkVcsError::StoreBootstrapInvalid(format!(
                "prepared v2 locator {locator} does not match preflight locator {}",
                object.new_locator
            )));
        }
    }

    let migration_id = apply_database_cutover(store_path, &preflight)?;
    let connection = StoreConnection::open(store_path)?;
    let store_info = validate_bootstrap(&connection)?;
    drop(connection);
    let store = Store::open(store_path)?;
    let local_objects = store.validate_local_content_storage()?;
    if local_objects != preflight.objects.len() {
        return Err(WorkVcsError::StoreBootstrapInvalid(format!(
            "post-migration validation found {local_objects} local object(s), expected {}",
            preflight.objects.len()
        )));
    }

    Ok(LocalObjectV2MigrationResult {
        mode: LocalObjectV2MigrationMode::Applied,
        store_info,
        local_objects,
        preexisting_target_objects,
        legacy_root: preflight.legacy_root,
        target_root: preflight.target_root,
        migration_id: Some(migration_id),
    })
}

fn preflight(store_path: &Path) -> Result<Preflight> {
    let connection = StoreConnection::open(store_path)?;
    validate_application_id(&connection)?;
    schema::validate_installed_schema(&connection)?;
    let store_info = load_store_info_for_local_object_v2_migration(&connection)?;
    let store_parent = store_path.parent().ok_or_else(|| {
        WorkVcsError::StoreBootstrapInvalid(format!(
            "Store path {} has no parent for local object migration",
            store_path.display()
        ))
    })?;
    let legacy_root = store_parent
        .join(LOCAL_CONTENT_OBJECT_DIR)
        .join(store_info.store_id.to_string())
        .join(LEGACY_PATH_SEGMENT);
    let target_root = store_parent
        .join(LOCAL_CONTENT_OBJECT_DIR)
        .join(store_info.store_id.to_string())
        .join("blake3-256");

    if store_info.manifest.object_store_format_version == OBJECT_STORE_FORMAT_VERSION {
        return Ok(Preflight {
            store_info,
            legacy_root,
            target_root,
            objects: Vec::new(),
        });
    }
    if store_info.manifest.object_store_format_version != LEGACY_OBJECT_STORE_FORMAT_VERSION {
        return Err(WorkVcsError::StoreCompatibilityUnsupported(format!(
            "object_store_format_version {} is not the v1 migration source",
            store_info.manifest.object_store_format_version
        )));
    }

    let current_backend_rows: i64 = connection
        .inner()
        .query_row(
            "SELECT COUNT(*) FROM content_storage_location WHERE storage_backend = ?1",
            [LOCAL_CONTENT_STORAGE_BACKEND],
            |row| row.get(0),
        )
        .map_err(storage_error)?;
    if current_backend_rows != 0 {
        return Err(WorkVcsError::StoreBootstrapInvalid(format!(
            "v1 Store already contains {current_backend_rows} v2 local object location(s)"
        )));
    }

    let mut statement = connection
        .inner()
        .prepare(
            "SELECT l.content_digest,
                    o.size_bytes,
                    l.locator,
                    l.availability_state
             FROM content_storage_location AS l
             JOIN content_object AS o ON o.content_digest = l.content_digest
             WHERE l.storage_backend = ?1
             ORDER BY l.content_digest, l.locator",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map([LEGACY_STORAGE_BACKEND], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(storage_error)?;

    let mut objects = Vec::new();
    let mut seen_digests = HashSet::new();
    for row in rows {
        let (digest_bytes, size_bytes, old_locator, availability_state) =
            row.map_err(storage_error)?;
        let digest_bytes: [u8; Digest::BYTE_LEN] =
            digest_bytes.try_into().map_err(|bytes: Vec<u8>| {
                WorkVcsError::StoreBootstrapInvalid(format!(
                    "content digest must be {} bytes, found {}",
                    Digest::BYTE_LEN,
                    bytes.len()
                ))
            })?;
        let digest = Digest::from_bytes(digest_bytes);
        if !seen_digests.insert(digest) {
            return Err(WorkVcsError::StoreBootstrapInvalid(format!(
                "v1 Store contains more than one local object locator for digest {digest}"
            )));
        }
        if availability_state != "available" {
            return Err(WorkVcsError::StoreBootstrapInvalid(format!(
                "v1 local object {digest} has unsupported availability state {availability_state}"
            )));
        }
        let expected_old_relative = legacy_content_relative_path(store_info.store_id, digest);
        let expected_old_locator = relative_path_to_locator(&expected_old_relative)?;
        if old_locator != expected_old_locator {
            return Err(WorkVcsError::StoreBootstrapInvalid(format!(
                "v1 local object {digest} locator {old_locator} is not canonical {expected_old_locator}"
            )));
        }
        let old_path = store_parent.join(&expected_old_relative);
        let raw_bytes = verify_local_content_file(&old_path, digest, size_bytes)?;
        let new_relative = local_content_relative_path(store_info.store_id, digest);
        let new_locator = relative_path_to_locator(&new_relative)?;
        let new_path = store_parent.join(&new_relative);
        let target_preexisting = new_path.is_file();
        if target_preexisting {
            verify_local_content_file(&new_path, digest, size_bytes)?;
        } else if new_path.exists() {
            return Err(WorkVcsError::StoreBootstrapInvalid(format!(
                "v2 local object target {} exists but is not a regular file",
                new_path.display()
            )));
        }
        objects.push(LegacyObject {
            digest,
            size_bytes,
            old_locator,
            new_locator,
            old_path,
            raw_bytes,
            target_preexisting,
        });
    }
    drop(statement);
    drop(connection);

    validate_legacy_inventory(&legacy_root, &objects)?;
    Ok(Preflight {
        store_info,
        legacy_root,
        target_root,
        objects,
    })
}

fn apply_database_cutover(store_path: &Path, preflight: &Preflight) -> Result<MigrationId> {
    let mut connection = StoreConnection::open(store_path)?;
    let migration_id = MigrationId::new_v7();
    let now_us = current_epoch_micros()?;
    let target_manifest = StoreManifest::current();
    let target_manifest_json = target_manifest.canonical_manifest_json()?;
    let detail = CanonicalValue::object(vec![
        (
            "from_object_store_format_version".to_owned(),
            CanonicalValue::safe_integer(LEGACY_OBJECT_STORE_FORMAT_VERSION)?,
        ),
        (
            "local_objects".to_owned(),
            CanonicalValue::safe_integer(usize_to_i64(preflight.objects.len())?)?,
        ),
        (
            "source_objects_retained".to_owned(),
            CanonicalValue::Bool(true),
        ),
        (
            "to_object_store_format_version".to_owned(),
            CanonicalValue::safe_integer(OBJECT_STORE_FORMAT_VERSION)?,
        ),
    ])?;
    let detail_json = String::from_utf8(canonical_bytes(&detail)?).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!(
            "migration detail canonical JSON was not UTF-8: {error}"
        ))
    })?;
    let migration_id_bytes = migration_id.raw_bytes();
    let store_id_bytes = preflight.store_info.store_id.raw_bytes();

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let manifest_version: i64 = transaction
        .query_row(
            "SELECT object_store_format_version FROM store_manifest WHERE store_id = ?1",
            params![&store_id_bytes[..]],
            |row| row.get(0),
        )
        .map_err(storage_error)?;
    if manifest_version != LEGACY_OBJECT_STORE_FORMAT_VERSION {
        return Err(WorkVcsError::StoreBootstrapInvalid(format!(
            "object store changed after preflight: found version {manifest_version}"
        )));
    }
    let legacy_rows: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM content_storage_location WHERE storage_backend = ?1",
            [LEGACY_STORAGE_BACKEND],
            |row| row.get(0),
        )
        .map_err(storage_error)?;
    if legacy_rows != usize_to_i64(preflight.objects.len())? {
        return Err(WorkVcsError::StoreBootstrapInvalid(format!(
            "v1 local object locations changed after preflight: found {legacy_rows}, expected {}",
            preflight.objects.len()
        )));
    }
    let current_rows: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM content_storage_location WHERE storage_backend = ?1",
            [LOCAL_CONTENT_STORAGE_BACKEND],
            |row| row.get(0),
        )
        .map_err(storage_error)?;
    if current_rows != 0 {
        return Err(WorkVcsError::StoreBootstrapInvalid(format!(
            "v2 local object locations changed after preflight: found {current_rows}"
        )));
    }

    for object in &preflight.objects {
        let updated = transaction
            .execute(
                "UPDATE content_storage_location
                 SET storage_backend = ?4, locator = ?5
                 WHERE content_digest = ?1
                   AND storage_backend = ?2
                   AND locator = ?3",
                params![
                    &object.digest.as_bytes()[..],
                    LEGACY_STORAGE_BACKEND,
                    object.old_locator,
                    LOCAL_CONTENT_STORAGE_BACKEND,
                    object.new_locator,
                ],
            )
            .map_err(storage_error)?;
        if updated != 1 {
            return Err(WorkVcsError::StoreBootstrapInvalid(format!(
                "v1 local object location for digest {} changed after preflight",
                object.digest
            )));
        }
    }
    let updated_manifest = transaction
        .execute(
            "UPDATE store_manifest
             SET object_store_format_version = ?2, manifest_json = ?3
             WHERE store_id = ?1 AND object_store_format_version = ?4",
            params![
                &store_id_bytes[..],
                OBJECT_STORE_FORMAT_VERSION,
                target_manifest_json,
                LEGACY_OBJECT_STORE_FORMAT_VERSION,
            ],
        )
        .map_err(storage_error)?;
    if updated_manifest != 1 {
        return Err(WorkVcsError::StoreBootstrapInvalid(
            "StoreManifest changed after local object migration preflight".to_owned(),
        ));
    }
    transaction
        .execute(
            "INSERT INTO store_migration_attempt(
                migration_id,
                from_store_format_version,
                to_store_format_version,
                from_schema_version,
                to_schema_version,
                tool_version,
                started_at_us
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &migration_id_bytes[..],
                STORE_FORMAT_VERSION,
                STORE_FORMAT_VERSION,
                SCHEMA_VERSION,
                SCHEMA_VERSION,
                MIGRATION_TOOL_VERSION,
                now_us,
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO store_migration_outcome(
                migration_id, outcome, completed_at_us, detail_json
             ) VALUES (?1, ?2, ?3, ?4)",
            params![&migration_id_bytes[..], "applied", now_us, detail_json],
        )
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)?;
    Ok(migration_id)
}

fn validate_legacy_inventory(legacy_root: &Path, objects: &[LegacyObject]) -> Result<()> {
    let expected = objects
        .iter()
        .map(|object| object.old_path.clone())
        .collect::<HashSet<_>>();
    let actual = collect_regular_files(legacy_root)?;
    if actual != expected {
        let unexpected = actual.difference(&expected).next();
        let missing = expected.difference(&actual).next();
        return Err(WorkVcsError::StoreBootstrapInvalid(format!(
            "legacy object inventory mismatch: unexpected={}, missing={}",
            unexpected
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "none".to_owned()),
            missing
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "none".to_owned())
        )));
    }
    Ok(())
}

fn collect_regular_files(root: &Path) -> Result<HashSet<PathBuf>> {
    let mut files = HashSet::new();
    if !root.exists() {
        return Ok(files);
    }
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory).map_err(|error| {
            WorkVcsError::StoreBootstrapInvalid(format!(
                "cannot inspect legacy object directory {}: {error}",
                directory.display()
            ))
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| {
                WorkVcsError::StoreBootstrapInvalid(format!(
                    "cannot inspect legacy object directory entry: {error}"
                ))
            })?;
            let path = entry.path();
            let file_type = entry.file_type().map_err(|error| {
                WorkVcsError::StoreBootstrapInvalid(format!(
                    "cannot inspect legacy object path {}: {error}",
                    path.display()
                ))
            })?;
            if file_type.is_dir() {
                pending.push(path);
            } else if file_type.is_file() {
                files.insert(path);
            } else {
                return Err(WorkVcsError::StoreBootstrapInvalid(format!(
                    "legacy object path {} is not a regular file or directory",
                    path.display()
                )));
            }
        }
    }
    Ok(files)
}

fn legacy_content_relative_path(store_id: StoreId, digest: Digest) -> PathBuf {
    let digest_text = digest.to_string();
    PathBuf::from(LOCAL_CONTENT_OBJECT_DIR)
        .join(store_id.to_string())
        .join(LEGACY_PATH_SEGMENT)
        .join(&digest_text[..2])
        .join(digest_text)
}

fn usize_to_i64(value: usize) -> Result<i64> {
    i64::try_from(value).map_err(|_| {
        WorkVcsError::StoreBootstrapInvalid("local object count does not fit i64".to_owned())
    })
}
