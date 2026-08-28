use crate::canonical::{CanonicalValue, canonical_bytes, parse_canonical_json};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::StoreId;
use crate::store::connection::StoreConnection;
use crate::store::schema;
use rusqlite::{OptionalExtension, params};
use std::time::{SystemTime, UNIX_EPOCH};

pub const APPLICATION_ID: i64 = 1_465_271_123;
pub const STORE_FORMAT_VERSION: i64 = 1;
pub const SCHEMA_VERSION: i64 = 1;
pub const OBJECT_STORE_FORMAT_VERSION: i64 = 1;
pub const ID_SCHEME: &str = "uuidv7-blob16";
pub const DIGEST_ALGORITHM: &str = "blake3-256";
pub const CANONICAL_JSON_PROFILE: &str = "workvcs-jcs-v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreInitOptions {
    display_name: String,
}

impl StoreInitOptions {
    pub fn new(display_name: impl Into<String>) -> Result<Self> {
        let display_name = display_name.into();
        if display_name.is_empty() {
            return Err(WorkVcsError::StoreBootstrapInvalid(
                "store display name must not be empty".to_owned(),
            ));
        }
        Ok(Self { display_name })
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreManifest {
    pub store_format_version: i64,
    pub schema_version: i64,
    pub object_store_format_version: i64,
    pub id_scheme: String,
    pub digest_algorithm: String,
    pub canonical_json_profile: String,
}

impl StoreManifest {
    pub fn current() -> Self {
        Self {
            store_format_version: STORE_FORMAT_VERSION,
            schema_version: SCHEMA_VERSION,
            object_store_format_version: OBJECT_STORE_FORMAT_VERSION,
            id_scheme: ID_SCHEME.to_owned(),
            digest_algorithm: DIGEST_ALGORITHM.to_owned(),
            canonical_json_profile: CANONICAL_JSON_PROFILE.to_owned(),
        }
    }

    pub fn validate_supported(&self) -> Result<()> {
        validate_version(
            "store_format_version",
            self.store_format_version,
            STORE_FORMAT_VERSION,
        )?;
        validate_version("schema_version", self.schema_version, SCHEMA_VERSION)?;
        validate_version(
            "object_store_format_version",
            self.object_store_format_version,
            OBJECT_STORE_FORMAT_VERSION,
        )?;
        validate_exact("id_scheme", &self.id_scheme, ID_SCHEME)?;
        validate_exact("digest_algorithm", &self.digest_algorithm, DIGEST_ALGORITHM)?;
        validate_exact(
            "canonical_json_profile",
            &self.canonical_json_profile,
            CANONICAL_JSON_PROFILE,
        )?;
        Ok(())
    }

    pub fn canonical_manifest_json(&self) -> Result<String> {
        let value = CanonicalValue::object(vec![
            (
                "canonical_json_profile".to_owned(),
                CanonicalValue::String(self.canonical_json_profile.clone()),
            ),
            (
                "digest_algorithm".to_owned(),
                CanonicalValue::String(self.digest_algorithm.clone()),
            ),
            (
                "id_scheme".to_owned(),
                CanonicalValue::String(self.id_scheme.clone()),
            ),
            (
                "object_store_format_version".to_owned(),
                CanonicalValue::safe_integer(self.object_store_format_version)?,
            ),
            (
                "schema_version".to_owned(),
                CanonicalValue::safe_integer(self.schema_version)?,
            ),
            (
                "store_format_version".to_owned(),
                CanonicalValue::safe_integer(self.store_format_version)?,
            ),
        ])?;
        String::from_utf8(canonical_bytes(&value)?).map_err(|error| {
            WorkVcsError::StoreBootstrapInvalid(format!(
                "manifest canonical JSON was not UTF-8: {error}"
            ))
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreInfo {
    pub store_id: StoreId,
    pub created_at_us: i64,
    pub display_name: String,
    pub manifest: StoreManifest,
}

pub(crate) fn ensure_empty_database(connection: &StoreConnection) -> Result<()> {
    let count = user_schema_object_count(connection)?;
    if count == 0 {
        Ok(())
    } else {
        Err(WorkVcsError::StoreAlreadyInitialized(format!(
            "database already contains {count} schema objects"
        )))
    }
}

pub(crate) fn initialize_manifest(
    connection: &mut StoreConnection,
    options: &StoreInitOptions,
) -> Result<StoreInfo> {
    let store_id = StoreId::new_v7();
    let created_at_us = current_epoch_micros()?;
    let manifest = StoreManifest::current();
    let manifest_json = manifest.canonical_manifest_json()?;
    let store_id_bytes = store_id.raw_bytes();

    let transaction = connection
        .inner_mut()
        .transaction()
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO store(store_id, created_at_us, display_name, metadata_json)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                &store_id_bytes[..],
                created_at_us,
                options.display_name(),
                "{}"
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO store_manifest(
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
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                &store_id_bytes[..],
                manifest.store_format_version,
                manifest.schema_version,
                manifest.object_store_format_version,
                manifest.id_scheme,
                manifest.digest_algorithm,
                manifest.canonical_json_profile,
                created_at_us,
                manifest_json
            ],
        )
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)?;

    load_store_info(connection)
}

pub(crate) fn validate_application_id(connection: &StoreConnection) -> Result<()> {
    let application_id = connection
        .inner()
        .pragma_query_value(None, "application_id", |row| row.get::<_, i64>(0))
        .map_err(storage_error)?;
    if application_id == APPLICATION_ID {
        Ok(())
    } else {
        Err(WorkVcsError::StoreCompatibilityUnsupported(format!(
            "application_id {application_id} is not the supported WorkVCS application_id {APPLICATION_ID}"
        )))
    }
}

pub(crate) fn validate_bootstrap(connection: &StoreConnection) -> Result<StoreInfo> {
    validate_application_id(connection)?;
    schema::validate_installed_schema(connection)?;
    load_store_info(connection)
}

pub(crate) fn load_store_info(connection: &StoreConnection) -> Result<StoreInfo> {
    let store_count = count_table_rows(connection, "store")?;
    let manifest_count = count_table_rows(connection, "store_manifest")?;
    if store_count != 1 || manifest_count != 1 {
        return Err(WorkVcsError::StoreBootstrapInvalid(format!(
            "expected exactly one Store and one StoreManifest, found store={store_count}, manifest={manifest_count}"
        )));
    }

    let row = connection
        .inner()
        .query_row(
            "SELECT
                s.store_id,
                s.created_at_us,
                s.display_name,
                m.store_format_version,
                m.schema_version,
                m.object_store_format_version,
                m.id_scheme,
                m.digest_algorithm,
                m.canonical_json_profile,
                m.manifest_json
             FROM store AS s
             JOIN store_manifest AS m ON m.store_id = s.store_id",
            [],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    StoreManifest {
                        store_format_version: row.get(3)?,
                        schema_version: row.get(4)?,
                        object_store_format_version: row.get(5)?,
                        id_scheme: row.get(6)?,
                        digest_algorithm: row.get(7)?,
                        canonical_json_profile: row.get(8)?,
                    },
                    row.get::<_, String>(9)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((store_id_bytes, created_at_us, display_name, manifest, manifest_json)) = row else {
        return Err(WorkVcsError::StoreBootstrapInvalid(
            "Store and StoreManifest rows did not join".to_owned(),
        ));
    };

    let store_id = decode_store_id(store_id_bytes)?;
    manifest.validate_supported()?;
    validate_manifest_json(&manifest, manifest_json.as_bytes())?;

    Ok(StoreInfo {
        store_id,
        created_at_us,
        display_name,
        manifest,
    })
}

fn validate_manifest_json(manifest: &StoreManifest, bytes: &[u8]) -> Result<()> {
    let value = parse_canonical_json(bytes).map_err(|error| {
        WorkVcsError::StoreBootstrapInvalid(format!("manifest_json is not canonical JSON: {error}"))
    })?;
    let reencoded = canonical_bytes(&value).map_err(|error| {
        WorkVcsError::StoreBootstrapInvalid(format!("manifest_json cannot be re-encoded: {error}"))
    })?;
    if reencoded != bytes {
        return Err(WorkVcsError::StoreBootstrapInvalid(
            "manifest_json is not a fixed-point canonical representation".to_owned(),
        ));
    }
    let expected = manifest.canonical_manifest_json()?;
    if bytes != expected.as_bytes() {
        return Err(WorkVcsError::StoreBootstrapInvalid(
            "manifest_json does not match the StoreManifest columns".to_owned(),
        ));
    }
    Ok(())
}

fn validate_version(name: &str, actual: i64, supported: i64) -> Result<()> {
    if actual == supported {
        Ok(())
    } else if actual > supported {
        Err(WorkVcsError::StoreCompatibilityUnsupported(format!(
            "{name} {actual} is newer than supported version {supported}"
        )))
    } else {
        Err(WorkVcsError::StoreCompatibilityUnsupported(format!(
            "{name} {actual} requires an explicit migration to supported version {supported}"
        )))
    }
}

fn validate_exact(name: &str, actual: &str, expected: &str) -> Result<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(WorkVcsError::StoreCompatibilityUnsupported(format!(
            "{name} {actual:?} is not supported; expected {expected:?}"
        )))
    }
}

fn count_table_rows(connection: &StoreConnection, table: &str) -> Result<i64> {
    let sql = match table {
        "store" => "SELECT count(*) FROM store",
        "store_manifest" => "SELECT count(*) FROM store_manifest",
        _ => {
            return Err(WorkVcsError::StoreBootstrapInvalid(format!(
                "unsupported bootstrap table count target {table:?}"
            )));
        }
    };
    connection
        .inner()
        .query_row(sql, [], |row| row.get(0))
        .map_err(|error| {
            WorkVcsError::StoreBootstrapInvalid(format!(
                "failed to count bootstrap table {table}: {error}"
            ))
        })
}

fn user_schema_object_count(connection: &StoreConnection) -> Result<i64> {
    connection
        .inner()
        .query_row(
            "SELECT count(*)
             FROM sqlite_schema
             WHERE name NOT LIKE 'sqlite_%'
               AND type IN ('table', 'index', 'view', 'trigger')",
            [],
            |row| row.get(0),
        )
        .map_err(storage_error)
}

fn decode_store_id(bytes: Vec<u8>) -> Result<StoreId> {
    let bytes: [u8; 16] = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::StoreBootstrapInvalid(format!(
            "store_id must be 16 bytes, found {}",
            bytes.len()
        ))
    })?;
    StoreId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::StoreBootstrapInvalid(format!("store_id is not a UUIDv7 value: {error}"))
    })
}

fn current_epoch_micros() -> Result<i64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            WorkVcsError::TimeInvalid(format!("system clock is before Unix epoch: {error}"))
        })?;
    let micros = duration
        .as_secs()
        .checked_mul(1_000_000)
        .and_then(|seconds| seconds.checked_add(u64::from(duration.subsec_micros())))
        .ok_or_else(|| {
            WorkVcsError::TimeInvalid("Unix epoch microseconds overflowed".to_owned())
        })?;
    i64::try_from(micros)
        .map_err(|_| WorkVcsError::TimeInvalid("Unix epoch microseconds do not fit i64".to_owned()))
}

#[cfg(test)]
mod tests {
    use super::{
        DIGEST_ALGORITHM, ID_SCHEME, SCHEMA_VERSION, StoreManifest, validate_manifest_json,
    };
    use crate::error::ErrorCode;

    #[test]
    fn manifest_validation_rejects_unsupported_frozen_parameters() {
        let mut manifest = StoreManifest::current();
        manifest.schema_version = SCHEMA_VERSION + 1;
        assert_eq!(
            manifest.validate_supported().unwrap_err().code(),
            ErrorCode::StoreCompatibilityUnsupported
        );

        let mut manifest = StoreManifest::current();
        manifest.id_scheme = "uuidv4".to_owned();
        assert_eq!(
            manifest.validate_supported().unwrap_err().code(),
            ErrorCode::StoreCompatibilityUnsupported
        );

        let mut manifest = StoreManifest::current();
        manifest.digest_algorithm = "sha256".to_owned();
        assert_eq!(
            manifest.validate_supported().unwrap_err().code(),
            ErrorCode::StoreCompatibilityUnsupported
        );

        let manifest = StoreManifest::current();
        assert_eq!(manifest.id_scheme, ID_SCHEME);
        assert_eq!(manifest.digest_algorithm, DIGEST_ALGORITHM);
    }

    #[test]
    fn manifest_json_must_be_fixed_point_and_match_columns() {
        let manifest = StoreManifest::current();
        let valid = manifest.canonical_manifest_json().expect("manifest json");
        validate_manifest_json(&manifest, valid.as_bytes()).expect("valid manifest");

        let pretty = br#"{
          "schema_version": 1
        }"#;
        assert_eq!(
            validate_manifest_json(&manifest, pretty)
                .unwrap_err()
                .code(),
            ErrorCode::StoreBootstrapInvalid
        );

        assert_eq!(
            validate_manifest_json(&manifest, br#"{}"#)
                .unwrap_err()
                .code(),
            ErrorCode::StoreBootstrapInvalid
        );
    }
}
