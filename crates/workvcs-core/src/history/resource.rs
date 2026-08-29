use crate::canonical::{
    CanonicalValue, canonical_bytes, content_object_digest, parse_canonical_json,
};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{Digest, ResourceId, ResourceObservationId, SessionId, WorkspaceId};
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};

pub(crate) const RESOURCE_OBJECT_KIND: &str = "resource";
pub(crate) const RESOURCE_OBSERVATION_OBJECT_KIND: &str = "resource_observation";
const CONTENT_OBJECT_KIND: &str = "content object";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceCreateOptions {
    resource_kind: String,
}

impl ResourceCreateOptions {
    pub fn new(resource_kind: impl Into<String>) -> Result<Self> {
        let resource_kind = resource_kind.into();
        validate_stored_text("resource kind", &resource_kind)?;
        Ok(Self { resource_kind })
    }

    pub fn resource_kind(&self) -> &str {
        &self.resource_kind
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceBindOptions {
    resource_id: ResourceId,
    adapter_kind: String,
    locator: String,
    binding_config: CanonicalValue,
}

impl ResourceBindOptions {
    pub fn new(
        resource_id: ResourceId,
        adapter_kind: impl Into<String>,
        locator: impl Into<String>,
    ) -> Result<Self> {
        let adapter_kind = adapter_kind.into();
        let locator = locator.into();
        validate_stored_text("resource binding adapter kind", &adapter_kind)?;
        validate_stored_text("resource binding locator", &locator)?;
        Ok(Self {
            resource_id,
            adapter_kind,
            locator,
            binding_config: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_binding_config(mut self, binding_config: CanonicalValue) -> Result<Self> {
        require_object_value("resource binding config", &binding_config)?;
        self.binding_config = binding_config;
        Ok(self)
    }

    pub fn resource_id(&self) -> ResourceId {
        self.resource_id
    }

    pub fn adapter_kind(&self) -> &str {
        &self.adapter_kind
    }

    pub fn locator(&self) -> &str {
        &self.locator
    }

    pub fn binding_config(&self) -> &CanonicalValue {
        &self.binding_config
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceResourceAssociationOptions {
    workspace_id: WorkspaceId,
    resource_id: ResourceId,
    association_metadata: CanonicalValue,
}

impl WorkspaceResourceAssociationOptions {
    pub fn new(workspace_id: WorkspaceId, resource_id: ResourceId) -> Result<Self> {
        Ok(Self {
            workspace_id,
            resource_id,
            association_metadata: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_association_metadata(
        mut self,
        association_metadata: CanonicalValue,
    ) -> Result<Self> {
        require_object_value(
            "workspace resource association metadata",
            &association_metadata,
        )?;
        self.association_metadata = association_metadata;
        Ok(self)
    }

    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    pub fn resource_id(&self) -> ResourceId {
        self.resource_id
    }

    pub fn association_metadata(&self) -> &CanonicalValue {
        &self.association_metadata
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceObservationDetailInput {
    content_digest: Digest,
    size_bytes: i64,
    media_type: Option<String>,
    format_metadata: CanonicalValue,
}

impl ResourceObservationDetailInput {
    pub fn from_raw_bytes(raw_bytes: impl AsRef<[u8]>) -> Result<Self> {
        let raw_bytes = raw_bytes.as_ref();
        let size_bytes = i64::try_from(raw_bytes.len()).map_err(|_| {
            WorkVcsError::ResourceInvalid(
                "resource observation detail is too large for SQLite size_bytes".to_owned(),
            )
        })?;
        Self::from_digest(content_object_digest(raw_bytes), size_bytes)
    }

    pub fn from_digest(content_digest: Digest, size_bytes: i64) -> Result<Self> {
        if size_bytes < 0 {
            return Err(WorkVcsError::ResourceInvalid(
                "resource observation detail size_bytes must be non-negative".to_owned(),
            ));
        }
        Ok(Self {
            content_digest,
            size_bytes,
            media_type: None,
            format_metadata: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_media_type(mut self, media_type: impl Into<String>) -> Result<Self> {
        let media_type = media_type.into();
        validate_stored_text("resource observation detail media_type", &media_type)?;
        self.media_type = Some(media_type);
        Ok(self)
    }

    pub fn with_format_metadata(mut self, format_metadata: CanonicalValue) -> Result<Self> {
        require_object_value(
            "resource observation detail format metadata",
            &format_metadata,
        )?;
        self.format_metadata = format_metadata;
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceObservationCreateOptions {
    resource_id: ResourceId,
    adapter_kind: String,
    adapter_schema_version: i64,
    fingerprint: Digest,
    summary: CanonicalValue,
    detail_content: Option<ResourceObservationDetailInput>,
    source_session_id: Option<SessionId>,
}

impl ResourceObservationCreateOptions {
    pub fn new(
        resource_id: ResourceId,
        adapter_kind: impl Into<String>,
        adapter_schema_version: i64,
        fingerprint: Digest,
        summary: CanonicalValue,
    ) -> Result<Self> {
        let adapter_kind = adapter_kind.into();
        validate_stored_text("resource observation adapter kind", &adapter_kind)?;
        validate_positive_version(
            "resource observation adapter schema version",
            adapter_schema_version,
        )?;
        require_object_value("resource observation summary", &summary)?;
        Ok(Self {
            resource_id,
            adapter_kind,
            adapter_schema_version,
            fingerprint,
            summary,
            detail_content: None,
            source_session_id: None,
        })
    }

    pub fn with_detail_content(mut self, detail_content: ResourceObservationDetailInput) -> Self {
        self.detail_content = Some(detail_content);
        self
    }

    pub fn with_source_session_id(mut self, source_session_id: SessionId) -> Self {
        self.source_session_id = Some(source_session_id);
        self
    }

    pub fn resource_id(&self) -> ResourceId {
        self.resource_id
    }

    pub fn adapter_kind(&self) -> &str {
        &self.adapter_kind
    }

    pub fn adapter_schema_version(&self) -> i64 {
        self.adapter_schema_version
    }

    pub fn fingerprint(&self) -> Digest {
        self.fingerprint
    }

    pub fn summary(&self) -> &CanonicalValue {
        &self.summary
    }

    pub fn detail_content(&self) -> Option<&ResourceObservationDetailInput> {
        self.detail_content.as_ref()
    }

    pub fn source_session_id(&self) -> Option<SessionId> {
        self.source_session_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceCreateResult {
    pub resource_id: ResourceId,
    pub resource_kind: String,
    pub created_at_us: i64,
    pub state: ResourceSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceBindResult {
    pub resource_id: ResourceId,
    pub bound_at_us: i64,
    pub state: ResourceBindingSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceResourceAssociationResult {
    pub workspace_id: WorkspaceId,
    pub resource_id: ResourceId,
    pub state: WorkspaceResourceAssociationSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceObservationCreateResult {
    pub observation_id: ResourceObservationId,
    pub resource_id: ResourceId,
    pub captured_at_us: i64,
    pub state: ResourceObservationSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceSnapshot {
    pub resource_id: ResourceId,
    pub resource_kind: String,
    pub created_at_us: i64,
    pub binding: Option<ResourceBindingSnapshot>,
    pub workspace_associations: Vec<WorkspaceResourceAssociationSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceBindingSnapshot {
    pub resource_id: ResourceId,
    pub adapter_kind: String,
    pub locator: String,
    pub binding_config: CanonicalValue,
    pub bound_at_us: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceResourceAssociationSnapshot {
    pub workspace_id: WorkspaceId,
    pub resource_id: ResourceId,
    pub association_metadata: CanonicalValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceObservationSnapshot {
    pub observation_id: ResourceObservationId,
    pub resource_id: ResourceId,
    pub adapter_kind: String,
    pub adapter_schema_version: i64,
    pub captured_at_us: i64,
    pub fingerprint: Digest,
    pub summary: CanonicalValue,
    pub detail_content: Option<ResourceObservationDetailSnapshot>,
    pub source_session_id: Option<SessionId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceObservationDetailSnapshot {
    pub content_digest: Digest,
    pub size_bytes: i64,
    pub media_type: Option<String>,
    pub format_metadata: CanonicalValue,
}

pub(crate) fn create_resource(
    connection: &mut StoreConnection,
    options: &ResourceCreateOptions,
) -> Result<ResourceCreateResult> {
    connection.verify_foreign_keys()?;
    validate_stored_text("resource kind", options.resource_kind())?;

    let resource_id = ResourceId::new_v7();
    let created_at_us = current_epoch_micros()?;
    let resource_id_bytes = resource_id.raw_bytes();

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES (?1, ?2, ?3)",
            params![&resource_id_bytes[..], RESOURCE_OBJECT_KIND, created_at_us],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO resource(resource_id, resource_kind, created_at_us)
             VALUES (?1, ?2, ?3)",
            params![
                &resource_id_bytes[..],
                options.resource_kind(),
                created_at_us
            ],
        )
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)?;

    let state = resource(connection, resource_id)?;
    Ok(ResourceCreateResult {
        resource_id,
        resource_kind: state.resource_kind.clone(),
        created_at_us,
        state,
    })
}

pub(crate) fn resource(
    connection: &StoreConnection,
    resource_id: ResourceId,
) -> Result<ResourceSnapshot> {
    let row = connection
        .inner()
        .query_row(
            "SELECT object_identity.object_kind,
                    resource.resource_kind,
                    resource.created_at_us
             FROM resource
             JOIN object_identity
               ON object_identity.object_id = resource.resource_id
             WHERE resource.resource_id = ?1",
            params![&resource_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((object_kind, resource_kind, created_at_us)) = row else {
        return Err(WorkVcsError::ResourceNotFound(format!(
            "resource {resource_id} does not exist"
        )));
    };
    if object_kind != RESOURCE_OBJECT_KIND {
        return Err(WorkVcsError::ResourceInvalid(format!(
            "resource {resource_id} has object kind {object_kind:?}"
        )));
    }
    validate_stored_text("resource kind", &resource_kind)?;
    if created_at_us < 0 {
        return Err(WorkVcsError::ResourceInvalid(format!(
            "resource {resource_id} has negative created_at_us"
        )));
    }

    Ok(ResourceSnapshot {
        resource_id,
        resource_kind,
        created_at_us,
        binding: load_resource_binding(connection, resource_id)?,
        workspace_associations: load_workspace_resource_associations(connection, resource_id)?,
    })
}

pub(crate) fn bind_resource(
    connection: &mut StoreConnection,
    options: &ResourceBindOptions,
) -> Result<ResourceBindResult> {
    connection.verify_foreign_keys()?;
    validate_stored_text("resource binding adapter kind", options.adapter_kind())?;
    validate_stored_text("resource binding locator", options.locator())?;
    let binding_config_json =
        canonical_object_json("resource binding config", options.binding_config())?;
    let bound_at_us = current_epoch_micros()?;
    let resource_id_bytes = options.resource_id().raw_bytes();

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    ensure_resource_exists(&transaction, options.resource_id())?;
    transaction
        .execute(
            "INSERT INTO resource_binding(
                resource_id,
                adapter_kind,
                locator,
                binding_config_json,
                bound_at_us
             )
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(resource_id) DO UPDATE SET
                adapter_kind = excluded.adapter_kind,
                locator = excluded.locator,
                binding_config_json = excluded.binding_config_json,
                bound_at_us = excluded.bound_at_us",
            params![
                &resource_id_bytes[..],
                options.adapter_kind(),
                options.locator(),
                binding_config_json,
                bound_at_us
            ],
        )
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)?;

    let state = load_resource_binding(connection, options.resource_id())?.ok_or_else(|| {
        WorkVcsError::ResourceInvalid(format!(
            "resource {} binding was not persisted",
            options.resource_id()
        ))
    })?;
    Ok(ResourceBindResult {
        resource_id: options.resource_id(),
        bound_at_us,
        state,
    })
}

pub(crate) fn associate_workspace_resource(
    connection: &mut StoreConnection,
    options: &WorkspaceResourceAssociationOptions,
) -> Result<WorkspaceResourceAssociationResult> {
    connection.verify_foreign_keys()?;
    let association_metadata_json = canonical_object_json(
        "workspace resource association metadata",
        options.association_metadata(),
    )?;
    let workspace_id_bytes = options.workspace_id().raw_bytes();
    let resource_id_bytes = options.resource_id().raw_bytes();

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    ensure_workspace_exists(&transaction, options.workspace_id())?;
    ensure_resource_exists(&transaction, options.resource_id())?;
    transaction
        .execute(
            "INSERT INTO workspace_resource(
                workspace_id,
                resource_id,
                association_metadata_json
             )
             VALUES (?1, ?2, ?3)
             ON CONFLICT(workspace_id, resource_id) DO UPDATE SET
                association_metadata_json = excluded.association_metadata_json",
            params![
                &workspace_id_bytes[..],
                &resource_id_bytes[..],
                association_metadata_json
            ],
        )
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)?;

    let state =
        workspace_resource_association(connection, options.workspace_id(), options.resource_id())?;
    Ok(WorkspaceResourceAssociationResult {
        workspace_id: options.workspace_id(),
        resource_id: options.resource_id(),
        state,
    })
}

pub(crate) fn record_resource_observation(
    connection: &mut StoreConnection,
    options: &ResourceObservationCreateOptions,
) -> Result<ResourceObservationCreateResult> {
    connection.verify_foreign_keys()?;
    validate_stored_text("resource observation adapter kind", options.adapter_kind())?;
    validate_positive_version(
        "resource observation adapter schema version",
        options.adapter_schema_version(),
    )?;
    let summary_json = canonical_object_json("resource observation summary", options.summary())?;
    let captured_at_us = current_epoch_micros()?;
    let observation_id = ResourceObservationId::new_v7();
    let observation_id_bytes = observation_id.raw_bytes();
    let resource_id_bytes = options.resource_id().raw_bytes();
    let source_session_id_bytes = options.source_session_id().map(|id| id.raw_bytes());
    let detail_content_digest = options
        .detail_content()
        .map(|content| content.content_digest.as_bytes());

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    ensure_resource_exists(&transaction, options.resource_id())?;
    if let Some(session_id) = options.source_session_id() {
        ensure_session_exists(&transaction, session_id)?;
    }
    if let Some(detail_content) = options.detail_content() {
        ensure_content_object(&transaction, detail_content)?;
    }
    transaction
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES (?1, ?2, ?3)",
            params![
                &observation_id_bytes[..],
                RESOURCE_OBSERVATION_OBJECT_KIND,
                captured_at_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO resource_observation(
                observation_id,
                resource_id,
                adapter_kind,
                adapter_schema_version,
                captured_at_us,
                fingerprint,
                summary_json,
                detail_content_digest,
                source_session_id
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                &observation_id_bytes[..],
                &resource_id_bytes[..],
                options.adapter_kind(),
                options.adapter_schema_version(),
                captured_at_us,
                &options.fingerprint().as_bytes()[..],
                summary_json,
                detail_content_digest.as_ref().map(|bytes| &bytes[..]),
                source_session_id_bytes.as_ref().map(|bytes| &bytes[..])
            ],
        )
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)?;

    let state = resource_observation(connection, observation_id)?;
    Ok(ResourceObservationCreateResult {
        observation_id,
        resource_id: options.resource_id(),
        captured_at_us,
        state,
    })
}

pub(crate) fn resource_observation(
    connection: &StoreConnection,
    observation_id: ResourceObservationId,
) -> Result<ResourceObservationSnapshot> {
    let row = connection
        .inner()
        .query_row(
            "SELECT object_identity.object_kind,
                    resource_observation.resource_id,
                    resource_observation.adapter_kind,
                    resource_observation.adapter_schema_version,
                    resource_observation.captured_at_us,
                    resource_observation.fingerprint,
                    resource_observation.summary_json,
                    resource_observation.detail_content_digest,
                    resource_observation.source_session_id,
                    content_object.size_bytes,
                    content_object.media_type,
                    content_object.format_metadata_json
             FROM resource_observation
             JOIN object_identity
               ON object_identity.object_id = resource_observation.observation_id
             JOIN resource
               ON resource.resource_id = resource_observation.resource_id
             LEFT JOIN content_object
               ON content_object.content_digest = resource_observation.detail_content_digest
             WHERE resource_observation.observation_id = ?1",
            params![&observation_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, Vec<u8>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<Vec<u8>>>(7)?,
                    row.get::<_, Option<Vec<u8>>>(8)?,
                    row.get::<_, Option<i64>>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((
        object_kind,
        resource_id,
        adapter_kind,
        adapter_schema_version,
        captured_at_us,
        fingerprint,
        summary_json,
        detail_content_digest,
        source_session_id,
        detail_size_bytes,
        detail_media_type,
        detail_format_metadata_json,
    )) = row
    else {
        return Err(WorkVcsError::ResourceObservationNotFound(format!(
            "resource observation {observation_id} does not exist"
        )));
    };
    if object_kind != RESOURCE_OBSERVATION_OBJECT_KIND {
        return Err(WorkVcsError::ResourceInvalid(format!(
            "resource observation {observation_id} has object kind {object_kind:?}"
        )));
    }
    let resource_id = decode_resource_id("resource_observation.resource_id", resource_id)?;
    validate_stored_text("resource observation adapter kind", &adapter_kind)?;
    validate_positive_version(
        "resource observation adapter schema version",
        adapter_schema_version,
    )?;
    if captured_at_us < 0 {
        return Err(WorkVcsError::ResourceInvalid(format!(
            "resource observation {observation_id} has negative captured_at_us"
        )));
    }
    let fingerprint = decode_digest("resource_observation.fingerprint", fingerprint)?;
    let summary = parse_canonical_object_json("resource_observation.summary_json", &summary_json)?;
    let source_session_id = source_session_id
        .map(|bytes| decode_session_id("resource_observation.source_session_id", bytes))
        .transpose()?;
    if let Some(session_id) = source_session_id {
        ensure_session_exists_for_read(connection, session_id)?;
    }
    let detail_content = match detail_content_digest {
        Some(content_digest) => {
            let size_bytes = detail_size_bytes.ok_or_else(|| {
                WorkVcsError::ResourceInvalid(format!(
                    "resource observation {observation_id} references missing detail content metadata"
                ))
            })?;
            let format_metadata_json = detail_format_metadata_json.ok_or_else(|| {
                WorkVcsError::ResourceInvalid(format!(
                    "resource observation {observation_id} references missing detail content format metadata"
                ))
            })?;
            if size_bytes < 0 {
                return Err(WorkVcsError::ResourceInvalid(format!(
                    "resource observation {observation_id} detail content has negative size_bytes"
                )));
            }
            if let Some(media_type) = &detail_media_type {
                validate_stored_text("resource observation detail media_type", media_type)?;
            }
            Some(ResourceObservationDetailSnapshot {
                content_digest: decode_digest(
                    "resource_observation.detail_content_digest",
                    content_digest,
                )?,
                size_bytes,
                media_type: detail_media_type,
                format_metadata: parse_canonical_object_json(
                    "content_object.format_metadata_json",
                    &format_metadata_json,
                )?,
            })
        }
        None => None,
    };

    Ok(ResourceObservationSnapshot {
        observation_id,
        resource_id,
        adapter_kind,
        adapter_schema_version,
        captured_at_us,
        fingerprint,
        summary,
        detail_content,
        source_session_id,
    })
}

fn load_resource_binding(
    connection: &StoreConnection,
    resource_id: ResourceId,
) -> Result<Option<ResourceBindingSnapshot>> {
    let row = connection
        .inner()
        .query_row(
            "SELECT adapter_kind,
                    locator,
                    binding_config_json,
                    bound_at_us
             FROM resource_binding
             WHERE resource_id = ?1",
            params![&resource_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    row.map(
        |(adapter_kind, locator, binding_config_json, bound_at_us)| {
            validate_stored_text("resource binding adapter kind", &adapter_kind)?;
            validate_stored_text("resource binding locator", &locator)?;
            if bound_at_us < 0 {
                return Err(WorkVcsError::ResourceInvalid(format!(
                    "resource {resource_id} binding has negative bound_at_us"
                )));
            }
            Ok(ResourceBindingSnapshot {
                resource_id,
                adapter_kind,
                locator,
                binding_config: parse_canonical_object_json(
                    "resource_binding.binding_config_json",
                    &binding_config_json,
                )?,
                bound_at_us,
            })
        },
    )
    .transpose()
}

fn load_workspace_resource_associations(
    connection: &StoreConnection,
    resource_id: ResourceId,
) -> Result<Vec<WorkspaceResourceAssociationSnapshot>> {
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT workspace_id, association_metadata_json
             FROM workspace_resource
             WHERE resource_id = ?1
             ORDER BY workspace_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&resource_id.raw_bytes()[..]], |row| {
            Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(storage_error)?;

    let mut associations = Vec::new();
    for row in rows {
        let (workspace_id, association_metadata_json) = row.map_err(storage_error)?;
        associations.push(WorkspaceResourceAssociationSnapshot {
            workspace_id: decode_workspace_id("workspace_resource.workspace_id", workspace_id)?,
            resource_id,
            association_metadata: parse_canonical_object_json(
                "workspace_resource.association_metadata_json",
                &association_metadata_json,
            )?,
        });
    }
    Ok(associations)
}

fn workspace_resource_association(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    resource_id: ResourceId,
) -> Result<WorkspaceResourceAssociationSnapshot> {
    let row = connection
        .inner()
        .query_row(
            "SELECT association_metadata_json
             FROM workspace_resource
             WHERE workspace_id = ?1
               AND resource_id = ?2",
            params![&workspace_id.raw_bytes()[..], &resource_id.raw_bytes()[..]],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(storage_error)?;
    let Some(association_metadata_json) = row else {
        return Err(WorkVcsError::ResourceInvalid(format!(
            "workspace {workspace_id} is not associated with resource {resource_id}"
        )));
    };
    Ok(WorkspaceResourceAssociationSnapshot {
        workspace_id,
        resource_id,
        association_metadata: parse_canonical_object_json(
            "workspace_resource.association_metadata_json",
            &association_metadata_json,
        )?,
    })
}

fn ensure_resource_exists(transaction: &Transaction<'_>, resource_id: ResourceId) -> Result<()> {
    let object_kind = transaction
        .query_row(
            "SELECT object_identity.object_kind
             FROM resource
             JOIN object_identity
               ON object_identity.object_id = resource.resource_id
             WHERE resource.resource_id = ?1",
            params![&resource_id.raw_bytes()[..]],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(storage_error)?;
    let Some(object_kind) = object_kind else {
        return Err(WorkVcsError::ResourceNotFound(format!(
            "resource {resource_id} does not exist"
        )));
    };
    if object_kind != RESOURCE_OBJECT_KIND {
        return Err(WorkVcsError::ResourceInvalid(format!(
            "resource {resource_id} has object kind {object_kind:?}"
        )));
    }
    Ok(())
}

fn ensure_workspace_exists(transaction: &Transaction<'_>, workspace_id: WorkspaceId) -> Result<()> {
    let exists = transaction
        .query_row(
            "SELECT 1
             FROM workspace
             WHERE workspace_id = ?1",
            params![&workspace_id.raw_bytes()[..]],
            |_| Ok(()),
        )
        .optional()
        .map_err(storage_error)?;
    exists.ok_or_else(|| {
        WorkVcsError::WorkspaceNotFound(format!("workspace {workspace_id} does not exist"))
    })
}

fn ensure_session_exists(transaction: &Transaction<'_>, session_id: SessionId) -> Result<()> {
    let exists = transaction
        .query_row(
            "SELECT 1
             FROM session
             WHERE session_id = ?1",
            params![&session_id.raw_bytes()[..]],
            |_| Ok(()),
        )
        .optional()
        .map_err(storage_error)?;
    exists.ok_or_else(|| {
        WorkVcsError::SessionNotFound(format!(
            "source session {session_id} does not exist for resource observation"
        ))
    })
}

fn ensure_session_exists_for_read(
    connection: &StoreConnection,
    session_id: SessionId,
) -> Result<()> {
    let exists = connection
        .inner()
        .query_row(
            "SELECT 1
             FROM session
             WHERE session_id = ?1",
            params![&session_id.raw_bytes()[..]],
            |_| Ok(()),
        )
        .optional()
        .map_err(storage_error)?;
    exists.ok_or_else(|| {
        WorkVcsError::SessionNotFound(format!(
            "source session {session_id} does not exist for resource observation"
        ))
    })
}

fn ensure_content_object(
    transaction: &Transaction<'_>,
    content: &ResourceObservationDetailInput,
) -> Result<()> {
    let format_metadata_json = canonical_object_json(
        "resource observation detail format metadata",
        &content.format_metadata,
    )?;
    let row = transaction
        .query_row(
            "SELECT size_bytes, media_type, format_metadata_json
             FROM content_object
             WHERE content_digest = ?1",
            params![&content.content_digest.as_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    if let Some((size_bytes, media_type, stored_format_metadata_json)) = row {
        let stored_format_metadata = parse_canonical_object_json(
            "content_object.format_metadata_json",
            &stored_format_metadata_json,
        )?;
        if size_bytes != content.size_bytes
            || media_type != content.media_type
            || stored_format_metadata != content.format_metadata
        {
            return Err(WorkVcsError::ResourceInvalid(format!(
                "{CONTENT_OBJECT_KIND} {} already exists with different metadata",
                content.content_digest
            )));
        }
        return Ok(());
    }

    transaction
        .execute(
            "INSERT INTO content_object(
                content_digest,
                size_bytes,
                media_type,
                format_metadata_json
             )
             VALUES (?1, ?2, ?3, ?4)",
            params![
                &content.content_digest.as_bytes()[..],
                content.size_bytes,
                content.media_type.as_deref(),
                format_metadata_json
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn validate_stored_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(WorkVcsError::ResourceInvalid(format!(
            "{label} must not be empty"
        )));
    }
    if value.trim() != value {
        return Err(WorkVcsError::ResourceInvalid(format!(
            "{label} must not have leading or trailing whitespace"
        )));
    }
    if value
        .as_bytes()
        .iter()
        .any(|byte| *byte == b'\0' || *byte < 0x20 || *byte == 0x7f)
    {
        return Err(WorkVcsError::ResourceInvalid(format!(
            "{label} must not contain NUL or ASCII control characters"
        )));
    }
    Ok(())
}

fn validate_positive_version(label: &str, value: i64) -> Result<()> {
    if value > 0 {
        Ok(())
    } else {
        Err(WorkVcsError::ResourceInvalid(format!(
            "{label} must be positive"
        )))
    }
}

fn require_object_value(label: &str, value: &CanonicalValue) -> Result<()> {
    match value {
        CanonicalValue::Object(_) => Ok(()),
        _ => Err(WorkVcsError::ResourceInvalid(format!(
            "{label} must be a canonical JSON object"
        ))),
    }
}

fn canonical_object_json(label: &str, value: &CanonicalValue) -> Result<String> {
    require_object_value(label, value)?;
    canonical_json_string(value)
}

fn parse_canonical_object_json(label: &str, input: &str) -> Result<CanonicalValue> {
    let parsed = parse_canonical_json(input.as_bytes()).map_err(|error| {
        WorkVcsError::ResourceInvalid(format!("{label} is not valid canonical JSON: {error}"))
    })?;
    require_object_value(label, &parsed)?;
    let encoded = canonical_json_string(&parsed)?;
    if input != encoded {
        return Err(WorkVcsError::ResourceInvalid(format!(
            "{label} is not stored in WorkVCS canonical JSON form"
        )));
    }
    Ok(parsed)
}

fn canonical_json_string(value: &CanonicalValue) -> Result<String> {
    String::from_utf8(canonical_bytes(value)?).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!("canonical JSON was not UTF-8: {error}"))
    })
}

fn decode_resource_id(column: &str, bytes: Vec<u8>) -> Result<ResourceId> {
    let bytes: [u8; 16] = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::ResourceInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })?;
    ResourceId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::ResourceInvalid(format!("{column} is not a canonical UUIDv7: {error}"))
    })
}

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    let bytes: [u8; 16] = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::ResourceInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })?;
    WorkspaceId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::ResourceInvalid(format!("{column} is not a canonical UUIDv7: {error}"))
    })
}

fn decode_session_id(column: &str, bytes: Vec<u8>) -> Result<SessionId> {
    let bytes: [u8; 16] = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::ResourceInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })?;
    SessionId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::ResourceInvalid(format!("{column} is not a canonical UUIDv7: {error}"))
    })
}

fn decode_digest(column: &str, bytes: Vec<u8>) -> Result<Digest> {
    let bytes: [u8; 32] = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::ResourceInvalid(format!("{column} must be 32 bytes, found {}", bytes.len()))
    })?;
    Ok(Digest::from_bytes(bytes))
}
