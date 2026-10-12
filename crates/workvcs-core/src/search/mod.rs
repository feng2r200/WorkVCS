mod snippet;
mod tokenizer;

use crate::canonical::canonical_bytes;
use crate::error::{Result, WorkVcsError};
use crate::history::{
    KnowledgeListOptions, KnowledgeStatus, RecordListOptions, RecordStatus, ReplayCache, commit,
    knowledges_at_with_cache, records_at_with_cache, state_at_with_cache,
    validate_integrity_with_cache,
};
use crate::identity::{
    BranchId, CommitId, Digest, EntityId, EntityVersionId, ProjectRefId, StoreId, WorkspaceId,
};
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::Serialize;
use snippet::{SNIPPET_POLICY_VERSION, snippet_spans};
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::path::{Component, Path, PathBuf};
use tiktoken_rs::o200k_base_singleton;
use tokenizer::{TOKENIZER_VERSION, fts_query, lexical_tokens};

pub const SEARCH_INDEX_FORMAT_VERSION: u32 = 2;
const TOKEN_COUNT_ENCODING: &str = "o200k_base";
const NATIVE_READ_GUIDANCE: &str =
    "refresh the derived index, or use native context and exact Record/Knowledge reads";

pub fn search_lexical_terms(text: &str) -> Vec<String> {
    lexical_tokens(text)
}

pub fn search_o200k_ordinary_token_count(text: &str) -> usize {
    o200k_base_singleton().encode_ordinary(text).len()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchTemporalScope {
    Current,
    Historical,
}

impl SearchTemporalScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Historical => "historical",
        }
    }

    fn parse(value: &str) -> std::result::Result<Self, DerivedIndexIssue> {
        match value {
            "current" => Ok(Self::Current),
            "historical" => Ok(Self::Historical),
            _ => Err(DerivedIndexIssue::new(
                "incompatible_manifest",
                format!("unknown temporal scope {value:?}"),
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchSourceType {
    Record,
    Knowledge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchRangeRole {
    QueryMatch,
    DocumentLeadContext,
    QueryMatchAndDocumentLeadContext,
}

impl SearchRangeRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::QueryMatch => "query_match",
            Self::DocumentLeadContext => "document_lead_context",
            Self::QueryMatchAndDocumentLeadContext => "query_match_and_document_lead_context",
        }
    }

    fn includes_query_match(self) -> bool {
        matches!(
            self,
            Self::QueryMatch | Self::QueryMatchAndDocumentLeadContext
        )
    }

    fn includes_document_lead(self) -> bool {
        matches!(
            self,
            Self::DocumentLeadContext | Self::QueryMatchAndDocumentLeadContext
        )
    }

    fn merged(left: Self, right: Self) -> Self {
        match (
            left.includes_query_match() || right.includes_query_match(),
            left.includes_document_lead() || right.includes_document_lead(),
        ) {
            (true, true) => Self::QueryMatchAndDocumentLeadContext,
            (true, false) => Self::QueryMatch,
            (false, true) => Self::DocumentLeadContext,
            (false, false) => unreachable!("every search range has at least one role"),
        }
    }
}

impl SearchSourceType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Record => "record",
            Self::Knowledge => "knowledge",
        }
    }

    fn parse(value: &str) -> std::result::Result<Self, DerivedIndexIssue> {
        match value {
            "record" => Ok(Self::Record),
            "knowledge" => Ok(Self::Knowledge),
            _ => Err(DerivedIndexIssue::new(
                "damaged_index",
                format!("unknown source type {value:?}"),
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SearchSourceIdentity {
    pub project_ref_id: ProjectRefId,
    pub store_id: StoreId,
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
}

impl SearchSourceIdentity {
    pub const fn new(
        project_ref_id: ProjectRefId,
        store_id: StoreId,
        workspace_id: WorkspaceId,
        branch_id: BranchId,
    ) -> Self {
        Self {
            project_ref_id,
            store_id,
            workspace_id,
            branch_id,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SearchRefreshOptions {
    identity: SearchSourceIdentity,
    index_path: PathBuf,
    commit_id: Option<CommitId>,
    protected_paths: Vec<PathBuf>,
}

impl SearchRefreshOptions {
    pub fn new(identity: SearchSourceIdentity, index_path: impl Into<PathBuf>) -> Self {
        Self {
            identity,
            index_path: index_path.into(),
            commit_id: None,
            protected_paths: Vec::new(),
        }
    }

    pub fn at_commit(mut self, commit_id: CommitId) -> Self {
        self.commit_id = Some(commit_id);
        self
    }

    pub fn with_protected_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.protected_paths.push(path.into());
        self
    }

    pub fn identity(&self) -> SearchSourceIdentity {
        self.identity
    }

    pub fn index_path(&self) -> &Path {
        &self.index_path
    }

    pub fn commit_id(&self) -> Option<CommitId> {
        self.commit_id
    }
}

#[derive(Clone, Debug)]
pub struct SearchQueryOptions {
    identity: SearchSourceIdentity,
    index_path: PathBuf,
    query: String,
    commit_id: Option<CommitId>,
    result_limit: Option<usize>,
    protected_paths: Vec<PathBuf>,
    managed_directory: bool,
}

impl SearchQueryOptions {
    pub fn new(
        identity: SearchSourceIdentity,
        index_path: impl Into<PathBuf>,
        query: impl Into<String>,
    ) -> Result<Self> {
        let query = query.into();
        validate_query(&query)?;
        Ok(Self {
            identity,
            index_path: index_path.into(),
            query,
            commit_id: None,
            result_limit: None,
            protected_paths: Vec::new(),
            managed_directory: false,
        })
    }

    /// Prepare or reuse an immutable generation in this local directory.
    /// This may write derived files; the canonical source remains read-only.
    pub fn managed(
        identity: SearchSourceIdentity,
        directory: impl Into<PathBuf>,
        query: impl Into<String>,
    ) -> Result<Self> {
        let mut options = Self::new(identity, directory, query)?;
        options.managed_directory = true;
        Ok(options)
    }

    pub fn at_commit(mut self, commit_id: CommitId) -> Self {
        self.commit_id = Some(commit_id);
        self
    }

    pub fn with_result_limit(mut self, result_limit: usize) -> Result<Self> {
        if result_limit == 0 {
            return Err(WorkVcsError::QueryInvalid(
                "search result limit must be greater than zero".to_owned(),
            ));
        }
        self.result_limit = Some(result_limit);
        Ok(self)
    }

    pub fn with_protected_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.protected_paths.push(path.into());
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SearchIndexManifest {
    pub index_format_version: u32,
    pub tokenizer_version: String,
    pub snippet_policy_version: String,
    pub token_count_encoding: String,
    pub identity: SearchSourceIdentity,
    pub temporal_scope: SearchTemporalScope,
    pub source_commit_id: CommitId,
    pub source_state_digest: Digest,
    pub object_version_basis_digest: Digest,
    pub object_count: usize,
    pub snippet_count: usize,
    pub created_at_us: i64,
    pub manifest_digest: Digest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchRefreshOutcome {
    Refreshed,
    Reused,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SearchRefreshResult {
    pub outcome: SearchRefreshOutcome,
    pub index_path: PathBuf,
    pub manifest: SearchIndexManifest,
    pub records_indexed: usize,
    pub knowledge_indexed: usize,
    pub canonical_source_written: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchDerivedStatus {
    Ready,
    RefreshRequired,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SearchItem {
    pub source_id: String,
    pub source_type: SearchSourceType,
    pub entity_id: EntityId,
    pub entity_version_id: EntityVersionId,
    pub entity_state_digest: Digest,
    pub statement_digest: Digest,
    pub status: String,
    pub record_kind: Option<String>,
    pub scope_json: String,
    pub provenance_json: Option<String>,
    pub source_commit_id: CommitId,
    pub temporal_scope: SearchTemporalScope,
    pub canonical_statement: String,
    pub ranges: Vec<SearchSnippetRange>,
    pub score: f64,
    pub readback_verified: bool,
    pub serialized_tokens: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SearchSnippetRange {
    pub role: SearchRangeRole,
    pub snippet_byte_start: usize,
    pub snippet_byte_end: usize,
    pub snippet: String,
    pub readback_verified: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SearchQueryResult {
    pub status: SearchDerivedStatus,
    pub reason_code: Option<String>,
    pub reason: Option<String>,
    pub guidance: Option<String>,
    pub index_path: PathBuf,
    pub manifest: Option<SearchIndexManifest>,
    pub source_identity: SearchSourceIdentity,
    pub temporal_scope: SearchTemporalScope,
    pub source_commit_id: CommitId,
    pub source_state_digest: Digest,
    pub query: String,
    pub result_limit: Option<usize>,
    pub token_count_encoding: String,
    pub serialized_item_tokens: usize,
    pub candidates_considered: usize,
    pub matched_objects: usize,
    pub omitted_results: usize,
    pub items: Vec<SearchItem>,
    pub canonical_readback_verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_maintenance: Option<SearchRefreshOutcome>,
}

#[derive(Clone, Debug)]
struct SourceDocument {
    source_type: SearchSourceType,
    entity_id: EntityId,
    entity_version_id: EntityVersionId,
    state_digest: Digest,
    statement_digest: Digest,
    status: String,
    record_kind: Option<String>,
    statement: String,
    scope_json: String,
    provenance_json: Option<String>,
}

#[derive(Clone, Debug)]
struct SourceSnapshot {
    identity: SearchSourceIdentity,
    temporal_scope: SearchTemporalScope,
    source_commit_id: CommitId,
    source_state_digest: Digest,
    object_version_basis_digest: Digest,
    documents: Vec<SourceDocument>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExpectedSnippetRow {
    snippet_id: usize,
    document_key: String,
    byte_start: usize,
    byte_end: usize,
    snippet: String,
    snippet_digest: Digest,
    terms: String,
}

#[derive(Clone, Debug)]
struct Candidate {
    source_type: SearchSourceType,
    entity_id: EntityId,
    entity_version_id: EntityVersionId,
    state_digest: Digest,
    statement_digest: Digest,
    status: String,
    record_kind: Option<String>,
    statement: String,
    scope_json: String,
    provenance_json: Option<String>,
    snippet_byte_start: usize,
    snippet_byte_end: usize,
    snippet: String,
    lead_byte_start: usize,
    lead_byte_end: usize,
    lead_snippet: String,
    score: f64,
}

#[derive(Clone, Debug)]
struct AggregatedCandidate {
    source_type: SearchSourceType,
    entity_id: EntityId,
    entity_version_id: EntityVersionId,
    state_digest: Digest,
    statement_digest: Digest,
    status: String,
    record_kind: Option<String>,
    scope_json: String,
    provenance_json: Option<String>,
    statement: String,
    score: f64,
    query_ranges: Vec<SearchSnippetRange>,
    lead_range: SearchSnippetRange,
}

#[derive(Clone, Debug)]
struct DerivedIndexIssue {
    code: String,
    message: String,
}

impl DerivedIndexIssue {
    fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

pub(crate) fn refresh_index(
    source: &StoreConnection,
    source_path: &Path,
    options: &SearchRefreshOptions,
) -> Result<SearchRefreshResult> {
    ensure_atomic_search_publication_supported()?;
    let destination =
        validate_destination_shape(&options.index_path, source_path, &options.protected_paths)
            .map_err(derived_issue_error)?;
    let _publication_lock = SearchPublicationLock::acquire(&destination)?;
    let snapshot = source_snapshot(source, options.identity, options.commit_id)?;
    let destination = validate_refresh_destination(
        &destination,
        source_path,
        &options.protected_paths,
        &snapshot,
    )?;

    if let Some(existing) = read_existing_manifest_for_refresh(&destination)? {
        if require_supported_manifest(&existing).is_ok()
            && manifest_matches_snapshot(&existing, &snapshot)
            && existing_index_matches_snapshot(&destination, &existing, &snapshot)
        {
            search_test_pause(SearchTestPoint::RefreshBeforeReuseFreshness);
            if !source_still_matches_snapshot(source, &snapshot)? {
                return Err(WorkVcsError::BranchHeadConflict(format!(
                    "search source branch {} changed while validating reusable snapshot {}; select a new index path and refresh again",
                    snapshot.identity.branch_id, snapshot.source_commit_id
                )));
            }
            let (records_indexed, knowledge_indexed) = source_counts(&snapshot.documents);
            return Ok(SearchRefreshResult {
                outcome: SearchRefreshOutcome::Reused,
                index_path: destination,
                manifest: existing,
                records_indexed,
                knowledge_indexed,
                canonical_source_written: false,
            });
        }
        return Err(WorkVcsError::QueryInvalid(format!(
            "refusing to replace existing search index {}; reuse requires the exact same healthy source basis, otherwise select a new index path",
            destination.display()
        )));
    }

    publish_index(
        source,
        source_path,
        &options.protected_paths,
        &snapshot,
        &destination,
        PublicationFreshness::Refresh,
    )
}

enum PublicationFreshness {
    Refresh,
    ManagedQuery,
}

fn publish_index(
    source: &StoreConnection,
    source_path: &Path,
    protected_paths: &[PathBuf],
    snapshot: &SourceSnapshot,
    destination: &Path,
    freshness: PublicationFreshness,
) -> Result<SearchRefreshResult> {
    let mut temp_file = SearchTempFile::create(destination)?;
    let manifest = build_index(temp_file.path(), snapshot)?;
    validate_built_index(temp_file.path(), &manifest)?;
    sync_file(temp_file.path())?;
    validate_refresh_destination(destination, source_path, protected_paths, snapshot)?;
    search_test_pause(SearchTestPoint::RefreshBeforePublish);
    search_test_pause(SearchTestPoint::RefreshBeforeFreshness);
    if !source_still_matches_snapshot(source, snapshot)? {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "search source branch {} changed after snapshot {}; discard the derived candidate and refresh again",
            snapshot.identity.branch_id, snapshot.source_commit_id
        )));
    }
    atomic_publish_new_index(temp_file.path(), destination)?;
    temp_file.disarm();
    sync_parent(destination)?;
    search_test_pause(SearchTestPoint::RefreshAfterPublish);
    if matches!(freshness, PublicationFreshness::Refresh)
        && !source_still_matches_snapshot(source, snapshot)?
    {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "search source branch {} changed while publishing snapshot {}; the immutable index at {} is stale and must not be reused for current search; select a new index path and refresh again",
            snapshot.identity.branch_id,
            snapshot.source_commit_id,
            destination.display()
        )));
    }

    let published = read_manifest(&open_index_readonly(destination).map_err(derived_issue_error)?)
        .map_err(derived_issue_error)?;
    if require_manifest_matches_snapshot(&published, snapshot).is_err()
        || !existing_index_matches_snapshot(destination, &published, snapshot)
    {
        return Err(WorkVcsError::StorageFailure(format!(
            "published search index {} did not read back a complete index for the selected source snapshot",
            destination.display()
        )));
    }
    if matches!(freshness, PublicationFreshness::Refresh)
        && !source_still_matches_snapshot(source, snapshot)?
    {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "search source branch {} changed while reading back published snapshot {}; the immutable index at {} is stale and must not be reused for current search; select a new index path and refresh again",
            snapshot.identity.branch_id,
            snapshot.source_commit_id,
            destination.display()
        )));
    }
    let (records_indexed, knowledge_indexed) = source_counts(&snapshot.documents);
    Ok(SearchRefreshResult {
        outcome: SearchRefreshOutcome::Refreshed,
        index_path: destination.to_path_buf(),
        manifest: published,
        records_indexed,
        knowledge_indexed,
        canonical_source_written: false,
    })
}

pub(crate) fn query_index(
    source: &StoreConnection,
    source_path: &Path,
    options: &SearchQueryOptions,
) -> Result<SearchQueryResult> {
    ensure_atomic_search_publication_supported()?;
    validate_query(&options.query)?;
    let query_expression = fts_query(&options.query).ok_or_else(|| {
        WorkVcsError::QueryInvalid("search query contains no indexable terms".to_owned())
    })?;
    let snapshot = source_snapshot(source, options.identity, options.commit_id)?;
    if options.managed_directory {
        return managed_query_index(source, source_path, options, &snapshot, &query_expression);
    }
    query_index_with_snapshot(source, source_path, options, &snapshot, &query_expression)
}

fn managed_query_index(
    source: &StoreConnection,
    source_path: &Path,
    options: &SearchQueryOptions,
    snapshot: &SourceSnapshot,
    query_expression: &str,
) -> Result<SearchQueryResult> {
    let directory =
        prepare_managed_directory(&options.index_path, source_path, &options.protected_paths)?;
    // Share the publication lock with explicit refreshes. No mutable pointer
    // file or replacement of a previously published generation is needed.
    let basis = format!(
        "{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
        SEARCH_INDEX_FORMAT_VERSION,
        TOKENIZER_VERSION,
        SNIPPET_POLICY_VERSION,
        TOKEN_COUNT_ENCODING,
        snapshot.identity.project_ref_id,
        snapshot.identity.store_id,
        snapshot.identity.workspace_id,
        snapshot.identity.branch_id,
        snapshot.temporal_scope.as_str(),
        snapshot.source_commit_id,
        snapshot.source_state_digest,
        snapshot.object_version_basis_digest,
    );
    let prefix = format!("generation-{}-", Digest::raw(basis.as_bytes()));
    if let Some(result) = try_managed_generations(
        source,
        source_path,
        options,
        snapshot,
        query_expression,
        &directory,
        &prefix,
    )? {
        return Ok(result);
    }
    let publication_lock = SearchPublicationLock::acquire(&directory.join("generation.sqlite"))?;
    // A cooperative publisher may have completed after the lock-free lookup.
    if let Some(result) = try_managed_generations(
        source,
        source_path,
        options,
        snapshot,
        query_expression,
        &directory,
        &prefix,
    )? {
        return Ok(result);
    }
    let mut selected = options.clone();
    selected.managed_directory = false;
    selected.index_path = directory.join(format!("{prefix}{}.sqlite", uuid::Uuid::now_v7()));
    let destination = validate_refresh_destination(
        &selected.index_path,
        source_path,
        &options.protected_paths,
        snapshot,
    )?;
    publish_index(
        source,
        source_path,
        &options.protected_paths,
        snapshot,
        &destination,
        PublicationFreshness::ManagedQuery,
    )?;
    drop(publication_lock);
    let mut result =
        query_index_with_snapshot(source, source_path, &selected, snapshot, query_expression)?;
    result.index_maintenance = Some(SearchRefreshOutcome::Refreshed);
    Ok(result)
}

fn try_managed_generations(
    source: &StoreConnection,
    source_path: &Path,
    options: &SearchQueryOptions,
    snapshot: &SourceSnapshot,
    query_expression: &str,
    directory: &Path,
    prefix: &str,
) -> Result<Option<SearchQueryResult>> {
    let mut generations = Vec::new();
    for entry in fs::read_dir(directory).map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "cannot inspect managed search directory {}: {error}",
            directory.display()
        ))
    })? {
        let entry = entry.map_err(|error| WorkVcsError::StorageFailure(error.to_string()))?;
        if entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with(prefix) && name.ends_with(".sqlite"))
        {
            generations.push(entry.path());
        }
    }
    generations.sort();
    let mut selected = options.clone();
    selected.managed_directory = false;
    for path in generations {
        selected.index_path = path;
        let mut result =
            query_index_with_snapshot(source, source_path, &selected, snapshot, query_expression)?;
        if result.status == SearchDerivedStatus::Ready {
            result.index_maintenance = Some(SearchRefreshOutcome::Reused);
            return Ok(Some(result));
        }
        if result.reason_code.as_deref() == Some("source_changed_during_query") {
            result.index_maintenance = Some(SearchRefreshOutcome::Reused);
            return Ok(Some(result));
        }
        // Unknown, damaged, aliased or incompatible entries are preserved.
        // Their names never establish that they belong to this source basis.
    }
    Ok(None)
}

fn prepare_managed_directory(
    path: &Path,
    source_path: &Path,
    protected_paths: &[PathBuf],
) -> Result<PathBuf> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(WorkVcsError::QueryInvalid(
            "managed search directory must be absolute without dot or parent components".to_owned(),
        ));
    }
    let source = fs::canonicalize(source_path).map_err(|error| {
        WorkVcsError::QueryInvalid(format!("canonical Store path is unavailable: {error}"))
    })?;
    let protected = protected_paths
        .iter()
        .map(fs::canonicalize)
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|error| {
            WorkVcsError::QueryInvalid(format!("protected search path is unavailable: {error}"))
        })?;
    let mut directory = PathBuf::new();
    for component in path.components() {
        directory.push(component);
        if directory == source
            || is_reserved_sqlite_sidecar_name(&directory, &source)
            || protected.contains(&directory)
        {
            return Err(WorkVcsError::QueryInvalid(format!(
                "managed search directory aliases a protected source/control path {}",
                directory.display()
            )));
        }
        match fs::symlink_metadata(&directory) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => {
                return Err(WorkVcsError::QueryInvalid(format!(
                    "managed search directory component must be a non-symlink directory: {}",
                    directory.display()
                )));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                match fs::create_dir(&directory) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(error) => {
                        return Err(WorkVcsError::StorageFailure(format!(
                            "cannot create managed search directory {}: {error}",
                            directory.display()
                        )));
                    }
                }
                let metadata = fs::symlink_metadata(&directory).map_err(|error| {
                    WorkVcsError::StorageFailure(format!(
                        "cannot verify managed directory: {error}"
                    ))
                })?;
                if !metadata.is_dir() || metadata.file_type().is_symlink() {
                    return Err(WorkVcsError::QueryInvalid(format!(
                        "managed search directory component changed: {}",
                        directory.display()
                    )));
                }
            }
            Err(error) => {
                return Err(WorkVcsError::StorageFailure(format!(
                    "cannot inspect managed search directory {}: {error}",
                    directory.display()
                )));
            }
        }
    }
    fs::canonicalize(directory).map_err(|error| WorkVcsError::StorageFailure(error.to_string()))
}

fn query_index_with_snapshot(
    source: &StoreConnection,
    source_path: &Path,
    options: &SearchQueryOptions,
    snapshot: &SourceSnapshot,
    query_expression: &str,
) -> Result<SearchQueryResult> {
    let index_path = match validate_query_destination(
        &options.index_path,
        source_path,
        &options.protected_paths,
    ) {
        Ok(path) => path,
        Err(issue) => return Ok(refresh_required_result(options, snapshot, issue, None)),
    };
    let connection = match open_index_readonly(&index_path) {
        Ok(connection) => connection,
        Err(issue) => return Ok(refresh_required_result(options, snapshot, issue, None)),
    };
    if let Err(error) = connection.execute_batch("BEGIN DEFERRED TRANSACTION") {
        return Ok(refresh_required_result(
            options,
            snapshot,
            DerivedIndexIssue::new(
                "damaged_index",
                format!("cannot hold one immutable derived-index read snapshot: {error}"),
            ),
            None,
        ));
    }
    let result = query_index_in_read_transaction(
        source,
        options,
        snapshot,
        &index_path,
        &connection,
        query_expression,
    );
    let completion = if result.is_ok() { "COMMIT" } else { "ROLLBACK" };
    if let Err(error) = connection.execute_batch(completion) {
        return Err(WorkVcsError::StorageFailure(format!(
            "cannot {completion} immutable derived-index read snapshot: {error}"
        )));
    }
    result
}

fn query_index_in_read_transaction(
    source: &StoreConnection,
    options: &SearchQueryOptions,
    snapshot: &SourceSnapshot,
    index_path: &Path,
    connection: &Connection,
    query_expression: &str,
) -> Result<SearchQueryResult> {
    let manifest = match read_manifest(connection) {
        Ok(manifest) => manifest,
        Err(issue) => return Ok(refresh_required_result(options, snapshot, issue, None)),
    };
    if let Err(issue) = require_manifest_matches_snapshot(&manifest, snapshot) {
        return Ok(refresh_required_result(
            options,
            snapshot,
            issue,
            Some(manifest),
        ));
    }
    if !index_contents_match_snapshot(connection, &manifest, snapshot) {
        return Ok(refresh_required_result(
            options,
            snapshot,
            DerivedIndexIssue::new(
                "damaged_index",
                "derived search documents, snippets, or FTS terms do not match the selected canonical snapshot",
            ),
            Some(manifest),
        ));
    }
    search_test_pause(SearchTestPoint::QueryAfterIndexValidation);
    let candidates = match query_candidates(connection, query_expression) {
        Ok(candidates) => candidates,
        Err(issue) => {
            return Ok(refresh_required_result(
                options,
                snapshot,
                issue,
                Some(manifest),
            ));
        }
    };
    let candidates_considered = candidates.len();
    let candidates = match canonical_readback(snapshot, candidates) {
        Ok(candidates) => candidates,
        Err(issue) => {
            return Ok(refresh_required_result(
                options,
                snapshot,
                issue,
                Some(manifest),
            ));
        }
    };
    let candidates = aggregate_candidates(candidates);
    let matched_objects = candidates.len();
    let (items, serialized_item_tokens, omitted_results) =
        materialize_items(snapshot, candidates, options.result_limit)?;

    search_test_pause(SearchTestPoint::QueryBeforeFreshness);
    if !source_still_matches_snapshot(source, snapshot)? {
        return Ok(refresh_required_result(
            options,
            snapshot,
            DerivedIndexIssue::new(
                "source_changed_during_query",
                "canonical source branch head or state digest changed while derived results were being verified",
            ),
            Some(manifest),
        ));
    }

    Ok(SearchQueryResult {
        status: SearchDerivedStatus::Ready,
        reason_code: None,
        reason: None,
        guidance: None,
        index_path: index_path.to_path_buf(),
        manifest: Some(manifest),
        source_identity: snapshot.identity,
        temporal_scope: snapshot.temporal_scope,
        source_commit_id: snapshot.source_commit_id,
        source_state_digest: snapshot.source_state_digest,
        query: options.query.clone(),
        result_limit: options.result_limit,
        token_count_encoding: TOKEN_COUNT_ENCODING.to_owned(),
        serialized_item_tokens,
        candidates_considered,
        matched_objects,
        omitted_results,
        items,
        canonical_readback_verified: true,
        index_maintenance: None,
    })
}

fn source_snapshot(
    connection: &StoreConnection,
    identity: SearchSourceIdentity,
    commit_id: Option<CommitId>,
) -> Result<SourceSnapshot> {
    connection
        .execute_batch("BEGIN DEFERRED TRANSACTION")
        .map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "cannot begin immutable search source snapshot: {error}"
            ))
        })?;
    let result = source_snapshot_in_read_transaction(connection, identity, commit_id);
    let completion = if result.is_ok() { "COMMIT" } else { "ROLLBACK" };
    if let Err(completion_error) = connection.execute_batch(completion) {
        return Err(WorkVcsError::StorageFailure(format!(
            "cannot {completion} immutable search source snapshot: {completion_error}"
        )));
    }
    result
}

fn source_snapshot_in_read_transaction(
    connection: &StoreConnection,
    identity: SearchSourceIdentity,
    commit_id: Option<CommitId>,
) -> Result<SourceSnapshot> {
    let mut replay_cache = ReplayCache::default();
    validate_integrity_with_cache(connection, &mut replay_cache)?;
    let store_info = crate::store::validate_bootstrap(connection)?;
    if store_info.store_id != identity.store_id {
        return Err(WorkVcsError::QueryInvalid(format!(
            "search source Store {} does not match routed Store {}",
            store_info.store_id, identity.store_id
        )));
    }
    let branch = crate::history::branch_head(connection, identity.branch_id)?;
    if branch.workspace_id != identity.workspace_id {
        return Err(WorkVcsError::QueryInvalid(format!(
            "search branch {} belongs to workspace {}, not routed workspace {}",
            identity.branch_id, branch.workspace_id, identity.workspace_id
        )));
    }

    let (temporal_scope, selected_commit_id, expected_state_digest) = match commit_id {
        None => (
            SearchTemporalScope::Current,
            branch.head_commit_id,
            branch.state_digest,
        ),
        Some(commit_id) => {
            let selected = commit(connection, commit_id)?;
            if selected.workspace_id != identity.workspace_id {
                return Err(WorkVcsError::QueryInvalid(format!(
                    "historical search commit {commit_id} belongs to workspace {}, not {}",
                    selected.workspace_id, identity.workspace_id
                )));
            }
            if !commit_is_reachable_from(
                connection,
                branch.head_commit_id,
                commit_id,
                identity.workspace_id,
            )? {
                return Err(WorkVcsError::QueryInvalid(format!(
                    "historical search commit {commit_id} is not reachable from selected branch {}",
                    identity.branch_id
                )));
            }
            (
                SearchTemporalScope::Historical,
                commit_id,
                selected.state_digest,
            )
        }
    };

    let replayed = state_at_with_cache(connection, selected_commit_id, &mut replay_cache)?;
    if replayed.workspace_id != identity.workspace_id
        || replayed.state_digest != expected_state_digest
    {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "search source snapshot {selected_commit_id} does not match routed workspace and state digest"
        )));
    }
    let records = records_at_with_cache(
        connection,
        &RecordListOptions::new(selected_commit_id),
        &mut replay_cache,
    )?;
    let knowledge = knowledges_at_with_cache(
        connection,
        &KnowledgeListOptions::new(selected_commit_id),
        &mut replay_cache,
    )?;
    if records.workspace_id != identity.workspace_id
        || knowledge.workspace_id != identity.workspace_id
        || records.commit_id != selected_commit_id
        || knowledge.commit_id != selected_commit_id
    {
        return Err(WorkVcsError::IntegrityInvalid(
            "search source lists changed snapshot identity during one read".to_owned(),
        ));
    }

    let mut documents = Vec::new();
    for record in records.records {
        if temporal_scope == SearchTemporalScope::Current
            && !matches!(
                record.state.status,
                RecordStatus::Active
                    | RecordStatus::Running
                    | RecordStatus::Unverified
                    | RecordStatus::Validated
            )
        {
            continue;
        }
        let statement_digest = statement_digest(&record.state.statement);
        documents.push(SourceDocument {
            source_type: SearchSourceType::Record,
            entity_id: record.record_entity_id,
            entity_version_id: record.record_entity_version_id,
            state_digest: record.state_digest,
            statement_digest,
            status: record.state.status.as_str().to_owned(),
            record_kind: Some(record.state.kind.as_str().to_owned()),
            statement: record.state.statement,
            scope_json: canonical_text(&record.state.scope)?,
            provenance_json: None,
        });
    }
    for item in knowledge.knowledge {
        if temporal_scope == SearchTemporalScope::Current
            && item.state.status != KnowledgeStatus::Active
        {
            continue;
        }
        let statement_digest = statement_digest(&item.state.statement);
        documents.push(SourceDocument {
            source_type: SearchSourceType::Knowledge,
            entity_id: item.knowledge_entity_id,
            entity_version_id: item.knowledge_entity_version_id,
            state_digest: item.state_digest,
            statement_digest,
            status: item.state.status.as_str().to_owned(),
            record_kind: None,
            statement: item.state.statement,
            scope_json: canonical_text(&item.state.scope)?,
            provenance_json: Some(canonical_text(&item.state.provenance)?),
        });
    }
    documents.sort_by(|left, right| {
        left.source_type
            .as_str()
            .cmp(right.source_type.as_str())
            .then_with(|| left.entity_id.cmp(&right.entity_id))
    });
    let object_version_basis_digest = object_version_basis_digest(&documents);
    Ok(SourceSnapshot {
        identity,
        temporal_scope,
        source_commit_id: selected_commit_id,
        source_state_digest: replayed.state_digest,
        object_version_basis_digest,
        documents,
    })
}

fn commit_is_reachable_from(
    connection: &StoreConnection,
    descendant_commit_id: CommitId,
    ancestor_commit_id: CommitId,
    workspace_id: WorkspaceId,
) -> Result<bool> {
    let mut pending = vec![descendant_commit_id];
    let mut visited = HashSet::new();
    while let Some(commit_id) = pending.pop() {
        if !visited.insert(commit_id) {
            continue;
        }
        let snapshot = commit(connection, commit_id)?;
        if snapshot.workspace_id != workspace_id {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "commit {commit_id} reachable from workspace {workspace_id} belongs to workspace {}",
                snapshot.workspace_id
            )));
        }
        if commit_id == ancestor_commit_id {
            return Ok(true);
        }
        pending.extend(
            snapshot
                .parents
                .into_iter()
                .map(|parent| parent.parent_commit_id),
        );
    }
    Ok(false)
}

fn source_still_matches_snapshot(
    connection: &StoreConnection,
    snapshot: &SourceSnapshot,
) -> Result<bool> {
    if snapshot.temporal_scope == SearchTemporalScope::Historical {
        return Ok(true);
    }
    let head = crate::history::branch_head(connection, snapshot.identity.branch_id)?;
    Ok(head.workspace_id == snapshot.identity.workspace_id
        && head.head_commit_id == snapshot.source_commit_id
        && head.state_digest == snapshot.source_state_digest)
}

fn canonical_text(value: &crate::CanonicalValue) -> Result<String> {
    String::from_utf8(canonical_bytes(value)?).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!(
            "canonical search metadata was not UTF-8: {error}"
        ))
    })
}

fn statement_digest(statement: &str) -> Digest {
    Digest::domain_separated("workvcs-search-statement-v1", statement.as_bytes())
}

fn object_version_basis_digest(documents: &[SourceDocument]) -> Digest {
    let mut payload = Vec::new();
    for document in documents {
        payload.extend_from_slice(document.source_type.as_str().as_bytes());
        payload.push(0);
        payload.extend_from_slice(&document.entity_id.raw_bytes());
        payload.extend_from_slice(&document.entity_version_id.raw_bytes());
        payload.extend_from_slice(document.state_digest.as_bytes());
        payload.extend_from_slice(document.statement_digest.as_bytes());
        payload.extend_from_slice(document.status.as_bytes());
        payload.push(0xff);
    }
    Digest::domain_separated("workvcs-search-object-version-basis-v1", &payload)
}

fn expected_snippet_rows(snapshot: &SourceSnapshot) -> Vec<ExpectedSnippetRow> {
    let mut rows = Vec::new();
    for document in &snapshot.documents {
        let document_key = format!("{}:{}", document.source_type.as_str(), document.entity_id);
        for span in snippet_spans(&document.statement) {
            let snippet = document.statement[span.byte_start..span.byte_end].to_owned();
            rows.push(ExpectedSnippetRow {
                snippet_id: rows.len() + 1,
                document_key: document_key.clone(),
                byte_start: span.byte_start,
                byte_end: span.byte_end,
                snippet_digest: statement_digest(&snippet),
                terms: lexical_tokens(&snippet).join(" "),
                snippet,
            });
        }
    }
    rows
}

fn build_index(path: &Path, snapshot: &SourceSnapshot) -> Result<SearchIndexManifest> {
    let mut connection = Connection::open(path).map_err(derived_storage)?;
    connection
        .pragma_update(None, "journal_mode", "DELETE")
        .map_err(derived_storage)?;
    connection
        .pragma_update(None, "synchronous", "FULL")
        .map_err(derived_storage)?;
    connection
        .execute_batch(
            "CREATE TABLE manifest(
                singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
                index_format_version INTEGER NOT NULL,
                tokenizer_version TEXT NOT NULL,
                snippet_policy_version TEXT NOT NULL,
                token_count_encoding TEXT NOT NULL,
                project_ref_id TEXT NOT NULL,
                store_id TEXT NOT NULL,
                workspace_id TEXT NOT NULL,
                branch_id TEXT NOT NULL,
                temporal_scope TEXT NOT NULL,
                source_commit_id TEXT NOT NULL,
                source_state_digest TEXT NOT NULL,
                object_version_basis_digest TEXT NOT NULL,
                object_count INTEGER NOT NULL,
                snippet_count INTEGER NOT NULL,
                created_at_us INTEGER NOT NULL,
                manifest_digest TEXT NOT NULL
             );
             CREATE TABLE document(
                document_key TEXT PRIMARY KEY,
                source_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                entity_version_id TEXT NOT NULL,
                entity_state_digest TEXT NOT NULL,
                statement_digest TEXT NOT NULL,
                status TEXT NOT NULL,
                record_kind TEXT,
                statement TEXT NOT NULL,
                scope_json TEXT NOT NULL,
                provenance_json TEXT
             );
             CREATE TABLE snippet(
                snippet_id INTEGER PRIMARY KEY,
                document_key TEXT NOT NULL REFERENCES document(document_key),
                byte_start INTEGER NOT NULL,
                byte_end INTEGER NOT NULL,
                snippet TEXT NOT NULL,
                snippet_digest TEXT NOT NULL
             );
             CREATE VIRTUAL TABLE passage_fts USING fts5(
                snippet_id UNINDEXED,
                terms,
                tokenize = 'unicode61'
             );",
        )
        .map_err(|error| {
            WorkVcsError::QueryUnsupported(format!(
                "the bundled SQLite build cannot create the required FTS5 index: {error}"
            ))
        })?;

    let expected_snippets = expected_snippet_rows(snapshot);
    let transaction = connection.transaction().map_err(derived_storage)?;
    for document in &snapshot.documents {
        let document_key = format!("{}:{}", document.source_type.as_str(), document.entity_id);
        transaction
            .execute(
                "INSERT INTO document(
                    document_key, source_type, entity_id, entity_version_id,
                    entity_state_digest, statement_digest, status, record_kind,
                    statement, scope_json, provenance_json
                 ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![
                    document_key,
                    document.source_type.as_str(),
                    document.entity_id.to_string(),
                    document.entity_version_id.to_string(),
                    document.state_digest.to_string(),
                    document.statement_digest.to_string(),
                    document.status,
                    document.record_kind,
                    document.statement,
                    document.scope_json,
                    document.provenance_json,
                ],
            )
            .map_err(derived_storage)?;
    }
    for row in &expected_snippets {
        transaction
            .execute(
                "INSERT INTO snippet(
                    snippet_id, document_key, byte_start, byte_end, snippet, snippet_digest
                 ) VALUES (?1,?2,?3,?4,?5,?6)",
                params![
                    usize_to_i64("search snippet id", row.snippet_id)?,
                    row.document_key,
                    usize_to_i64("search snippet byte start", row.byte_start)?,
                    usize_to_i64("search snippet byte end", row.byte_end)?,
                    row.snippet,
                    row.snippet_digest.to_string(),
                ],
            )
            .map_err(derived_storage)?;
        transaction
            .execute(
                "INSERT INTO passage_fts(snippet_id, terms) VALUES (?1,?2)",
                params![
                    usize_to_i64("search FTS snippet id", row.snippet_id)?,
                    row.terms
                ],
            )
            .map_err(derived_storage)?;
    }

    let created_at_us = current_epoch_micros()?;
    let mut manifest = SearchIndexManifest {
        index_format_version: SEARCH_INDEX_FORMAT_VERSION,
        tokenizer_version: TOKENIZER_VERSION.to_owned(),
        snippet_policy_version: SNIPPET_POLICY_VERSION.to_owned(),
        token_count_encoding: TOKEN_COUNT_ENCODING.to_owned(),
        identity: snapshot.identity,
        temporal_scope: snapshot.temporal_scope,
        source_commit_id: snapshot.source_commit_id,
        source_state_digest: snapshot.source_state_digest,
        object_version_basis_digest: snapshot.object_version_basis_digest,
        object_count: snapshot.documents.len(),
        snippet_count: expected_snippets.len(),
        created_at_us,
        manifest_digest: Digest::raw(&[]),
    };
    manifest.manifest_digest = manifest_digest(&manifest);
    insert_manifest(&transaction, &manifest)?;
    transaction.commit().map_err(derived_storage)?;
    connection
        .execute_batch("PRAGMA optimize;")
        .map_err(derived_storage)?;
    drop(connection);
    Ok(manifest)
}

fn insert_manifest(
    transaction: &rusqlite::Transaction<'_>,
    manifest: &SearchIndexManifest,
) -> Result<()> {
    transaction
        .execute(
            "INSERT INTO manifest VALUES (1,?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
            params![
                i64::from(manifest.index_format_version),
                manifest.tokenizer_version,
                manifest.snippet_policy_version,
                manifest.token_count_encoding,
                manifest.identity.project_ref_id.to_string(),
                manifest.identity.store_id.to_string(),
                manifest.identity.workspace_id.to_string(),
                manifest.identity.branch_id.to_string(),
                manifest.temporal_scope.as_str(),
                manifest.source_commit_id.to_string(),
                manifest.source_state_digest.to_string(),
                manifest.object_version_basis_digest.to_string(),
                usize_to_i64("search manifest object count", manifest.object_count)?,
                usize_to_i64("search manifest snippet count", manifest.snippet_count)?,
                manifest.created_at_us,
                manifest.manifest_digest.to_string(),
            ],
        )
        .map_err(derived_storage)?;
    Ok(())
}

fn manifest_digest(manifest: &SearchIndexManifest) -> Digest {
    let payload = format!(
        "{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
        manifest.index_format_version,
        manifest.tokenizer_version,
        manifest.snippet_policy_version,
        manifest.token_count_encoding,
        manifest.identity.project_ref_id,
        manifest.identity.store_id,
        manifest.identity.workspace_id,
        manifest.identity.branch_id,
        manifest.temporal_scope.as_str(),
        manifest.source_commit_id,
        manifest.source_state_digest,
        manifest.object_version_basis_digest,
        manifest.object_count,
        manifest.snippet_count,
        manifest.created_at_us,
    );
    Digest::domain_separated("workvcs-search-index-manifest-v2", payload.as_bytes())
}

fn open_index_readonly(path: &Path) -> std::result::Result<Connection, DerivedIndexIssue> {
    if !path.exists() {
        return Err(DerivedIndexIssue::new(
            "missing_index",
            format!("derived search index {} does not exist", path.display()),
        ));
    }
    let connection =
        Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|error| {
            DerivedIndexIssue::new(
                "damaged_index",
                format!(
                    "cannot open derived search index {}: {error}",
                    path.display()
                ),
            )
        })?;
    connection
        .pragma_update(None, "query_only", "ON")
        .map_err(|error| {
            DerivedIndexIssue::new(
                "damaged_index",
                format!("cannot make derived search index query-only: {error}"),
            )
        })?;
    let integrity = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
        .map_err(|error| {
            DerivedIndexIssue::new(
                "damaged_index",
                format!("cannot check derived search index integrity: {error}"),
            )
        })?;
    if integrity != "ok" {
        return Err(DerivedIndexIssue::new(
            "damaged_index",
            format!("derived search index integrity check returned {integrity:?}"),
        ));
    }
    Ok(connection)
}

fn read_manifest(
    connection: &Connection,
) -> std::result::Result<SearchIndexManifest, DerivedIndexIssue> {
    let manifest = read_manifest_unchecked(connection)?;
    require_supported_manifest(&manifest)?;
    Ok(manifest)
}

fn read_manifest_unchecked(
    connection: &Connection,
) -> std::result::Result<SearchIndexManifest, DerivedIndexIssue> {
    type ManifestRow = (
        i64,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        i64,
        i64,
        i64,
        String,
    );
    let row = connection
        .query_row(
            "SELECT index_format_version, tokenizer_version, snippet_policy_version,
                    token_count_encoding, project_ref_id, store_id, workspace_id, branch_id,
                    temporal_scope, source_commit_id, source_state_digest,
                    object_version_basis_digest, object_count, snippet_count, created_at_us,
                    manifest_digest
             FROM manifest WHERE singleton = 1",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                    row.get(12)?,
                    row.get(13)?,
                    row.get(14)?,
                    row.get(15)?,
                ))
            },
        )
        .optional()
        .map_err(|error| DerivedIndexIssue::new("damaged_index", error.to_string()))?;
    let Some(row): Option<ManifestRow> = row else {
        return Err(DerivedIndexIssue::new(
            "damaged_index",
            "derived search index has no singleton manifest",
        ));
    };
    let index_format_version = u32::try_from(row.0).map_err(|_| {
        DerivedIndexIssue::new("incompatible_manifest", "invalid index format version")
    })?;
    let object_count = usize::try_from(row.12)
        .map_err(|_| DerivedIndexIssue::new("damaged_index", "negative search object count"))?;
    let snippet_count = usize::try_from(row.13)
        .map_err(|_| DerivedIndexIssue::new("damaged_index", "negative search snippet count"))?;
    let identity = SearchSourceIdentity {
        project_ref_id: parse_id("project_ref_id", &row.4, ProjectRefId::parse_canonical)?,
        store_id: parse_id("store_id", &row.5, StoreId::parse_canonical)?,
        workspace_id: parse_id("workspace_id", &row.6, WorkspaceId::parse_canonical)?,
        branch_id: parse_id("branch_id", &row.7, BranchId::parse_canonical)?,
    };
    let mut manifest = SearchIndexManifest {
        index_format_version,
        tokenizer_version: row.1,
        snippet_policy_version: row.2,
        token_count_encoding: row.3,
        identity,
        temporal_scope: SearchTemporalScope::parse(&row.8)?,
        source_commit_id: parse_id("source_commit_id", &row.9, CommitId::parse_canonical)?,
        source_state_digest: parse_digest("source_state_digest", &row.10)?,
        object_version_basis_digest: parse_digest("object_version_basis_digest", &row.11)?,
        object_count,
        snippet_count,
        created_at_us: row.14,
        manifest_digest: parse_digest("manifest_digest", &row.15)?,
    };
    let stored_digest = manifest.manifest_digest;
    manifest.manifest_digest = Digest::raw(&[]);
    let computed = manifest_digest(&manifest);
    manifest.manifest_digest = stored_digest;
    if stored_digest != computed {
        return Err(DerivedIndexIssue::new(
            "damaged_index",
            "derived search manifest digest does not match its fields",
        ));
    }
    Ok(manifest)
}

fn require_supported_manifest(
    manifest: &SearchIndexManifest,
) -> std::result::Result<(), DerivedIndexIssue> {
    if manifest.index_format_version != SEARCH_INDEX_FORMAT_VERSION
        || manifest.tokenizer_version != TOKENIZER_VERSION
        || manifest.snippet_policy_version != SNIPPET_POLICY_VERSION
        || manifest.token_count_encoding != TOKEN_COUNT_ENCODING
    {
        return Err(DerivedIndexIssue::new(
            "incompatible_manifest",
            "derived search index format, tokenizer, snippet policy, or token-count encoding is unsupported",
        ));
    }
    Ok(())
}

fn parse_id<T>(
    label: &str,
    value: &str,
    parser: impl FnOnce(&str) -> Result<T>,
) -> std::result::Result<T, DerivedIndexIssue> {
    parser(value).map_err(|error| {
        DerivedIndexIssue::new(
            "damaged_index",
            format!("derived search {label} is invalid: {error}"),
        )
    })
}

fn parse_digest(label: &str, value: &str) -> std::result::Result<Digest, DerivedIndexIssue> {
    Digest::from_hex(value).map_err(|error| {
        DerivedIndexIssue::new(
            "damaged_index",
            format!("derived search {label} is invalid: {error}"),
        )
    })
}

fn query_candidates(
    connection: &Connection,
    expression: &str,
) -> std::result::Result<Vec<Candidate>, DerivedIndexIssue> {
    let mut statement = connection
        .prepare(
            "SELECT d.source_type, d.entity_id, d.entity_version_id,
                    d.entity_state_digest, d.statement_digest, d.status, d.record_kind,
                    d.statement, d.scope_json, d.provenance_json,
                    s.byte_start, s.byte_end, s.snippet, s.snippet_digest,
                    lead.byte_start, lead.byte_end, lead.snippet, lead.snippet_digest,
                    bm25(passage_fts) AS rank_score
             FROM passage_fts
             JOIN snippet s ON s.snippet_id = passage_fts.snippet_id
             JOIN document d ON d.document_key = s.document_key
             JOIN snippet lead ON lead.snippet_id = (
                 SELECT MIN(first_snippet.snippet_id)
                 FROM snippet first_snippet
                 WHERE first_snippet.document_key = d.document_key
             )
             WHERE passage_fts MATCH ?1
             ORDER BY rank_score, s.snippet_id",
        )
        .map_err(|error| DerivedIndexIssue::new("damaged_index", error.to_string()))?;
    let rows = statement
        .query_map(params![expression], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, i64>(10)?,
                row.get::<_, i64>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, String>(13)?,
                row.get::<_, i64>(14)?,
                row.get::<_, i64>(15)?,
                row.get::<_, String>(16)?,
                row.get::<_, String>(17)?,
                row.get::<_, f64>(18)?,
            ))
        })
        .map_err(|error| DerivedIndexIssue::new("damaged_index", error.to_string()))?;
    let mut candidates = Vec::new();
    for row in rows {
        let row =
            row.map_err(|error| DerivedIndexIssue::new("damaged_index", error.to_string()))?;
        let byte_start = usize::try_from(row.10)
            .map_err(|_| DerivedIndexIssue::new("damaged_index", "negative snippet byte start"))?;
        let byte_end = usize::try_from(row.11)
            .map_err(|_| DerivedIndexIssue::new("damaged_index", "negative snippet byte end"))?;
        if statement_digest(&row.12) != parse_digest("snippet_digest", &row.13)? {
            return Err(DerivedIndexIssue::new(
                "snippet_identity_mismatch",
                "derived snippet digest does not match snippet bytes",
            ));
        }
        let lead_byte_start = usize::try_from(row.14).map_err(|_| {
            DerivedIndexIssue::new("damaged_index", "negative lead snippet byte start")
        })?;
        let lead_byte_end = usize::try_from(row.15).map_err(|_| {
            DerivedIndexIssue::new("damaged_index", "negative lead snippet byte end")
        })?;
        if statement_digest(&row.16) != parse_digest("lead_snippet_digest", &row.17)? {
            return Err(DerivedIndexIssue::new(
                "snippet_identity_mismatch",
                "derived lead snippet digest does not match snippet bytes",
            ));
        }
        candidates.push(Candidate {
            source_type: SearchSourceType::parse(&row.0)?,
            entity_id: parse_id("entity_id", &row.1, EntityId::parse_canonical)?,
            entity_version_id: parse_id(
                "entity_version_id",
                &row.2,
                EntityVersionId::parse_canonical,
            )?,
            state_digest: parse_digest("entity_state_digest", &row.3)?,
            statement_digest: parse_digest("statement_digest", &row.4)?,
            status: row.5,
            record_kind: row.6,
            statement: row.7,
            scope_json: row.8,
            provenance_json: row.9,
            snippet_byte_start: byte_start,
            snippet_byte_end: byte_end,
            snippet: row.12,
            lead_byte_start,
            lead_byte_end,
            lead_snippet: row.16,
            score: -row.18,
        });
    }
    Ok(candidates)
}

fn canonical_readback(
    snapshot: &SourceSnapshot,
    candidates: Vec<Candidate>,
) -> std::result::Result<Vec<Candidate>, DerivedIndexIssue> {
    let by_source = snapshot
        .documents
        .iter()
        .map(|document| {
            (
                (document.source_type.as_str(), document.entity_id),
                document,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut verified = Vec::new();
    for candidate in candidates {
        let key = (candidate.source_type.as_str(), candidate.entity_id);
        let document = by_source.get(&key).ok_or_else(|| {
            DerivedIndexIssue::new(
                "canonical_readback_mismatch",
                format!(
                    "indexed {} {} is absent from the selected canonical snapshot",
                    candidate.source_type.as_str(),
                    candidate.entity_id
                ),
            )
        })?;
        if candidate.entity_version_id != document.entity_version_id
            || candidate.state_digest != document.state_digest
            || candidate.statement_digest != document.statement_digest
            || candidate.statement != document.statement
            || candidate.status != document.status
            || candidate.record_kind != document.record_kind
            || candidate.scope_json != document.scope_json
            || candidate.provenance_json != document.provenance_json
        {
            return Err(DerivedIndexIssue::new(
                "canonical_readback_mismatch",
                format!(
                    "indexed {} {} version, digest, status, or full statement does not match canonical readback",
                    candidate.source_type.as_str(),
                    candidate.entity_id
                ),
            ));
        }
        verify_canonical_range(
            document,
            candidate.snippet_byte_start,
            candidate.snippet_byte_end,
            &candidate.snippet,
            "query-match",
        )?;
        verify_canonical_range(
            document,
            candidate.lead_byte_start,
            candidate.lead_byte_end,
            &candidate.lead_snippet,
            "document-lead",
        )?;
        verified.push(candidate);
    }
    Ok(verified)
}

fn verify_canonical_range(
    document: &SourceDocument,
    byte_start: usize,
    byte_end: usize,
    snippet: &str,
    range_kind: &str,
) -> std::result::Result<(), DerivedIndexIssue> {
    if byte_start >= byte_end
        || byte_end > document.statement.len()
        || !document.statement.is_char_boundary(byte_start)
        || !document.statement.is_char_boundary(byte_end)
        || document.statement[byte_start..byte_end] != *snippet
    {
        return Err(DerivedIndexIssue::new(
            "snippet_identity_mismatch",
            format!(
                "indexed {range_kind} snippet for {} {} does not match canonical UTF-8 byte range",
                document.source_type.as_str(),
                document.entity_id
            ),
        ));
    }
    Ok(())
}

fn aggregate_candidates(candidates: Vec<Candidate>) -> Vec<AggregatedCandidate> {
    let mut positions = BTreeMap::new();
    let mut aggregated = Vec::<AggregatedCandidate>::new();
    for candidate in candidates {
        let key = (candidate.source_type.as_str(), candidate.entity_id);
        let position = match positions.get(&key).copied() {
            Some(position) => position,
            None => {
                let position = aggregated.len();
                positions.insert(key, position);
                aggregated.push(AggregatedCandidate {
                    source_type: candidate.source_type,
                    entity_id: candidate.entity_id,
                    entity_version_id: candidate.entity_version_id,
                    state_digest: candidate.state_digest,
                    statement_digest: candidate.statement_digest,
                    status: candidate.status.clone(),
                    record_kind: candidate.record_kind.clone(),
                    scope_json: candidate.scope_json.clone(),
                    provenance_json: candidate.provenance_json.clone(),
                    statement: candidate.statement.clone(),
                    score: candidate.score,
                    query_ranges: Vec::new(),
                    lead_range: SearchSnippetRange {
                        role: SearchRangeRole::DocumentLeadContext,
                        snippet_byte_start: candidate.lead_byte_start,
                        snippet_byte_end: candidate.lead_byte_end,
                        snippet: candidate.lead_snippet.clone(),
                        readback_verified: true,
                    },
                });
                position
            }
        };
        let document = &mut aggregated[position];
        add_query_range(
            document,
            candidate.snippet_byte_start,
            candidate.snippet_byte_end,
        );
    }
    for document in &mut aggregated {
        let mut ranges = std::mem::take(&mut document.query_ranges);
        ranges.push(document.lead_range.clone());
        document.query_ranges = merge_ranges(&document.statement, ranges);
    }
    aggregated
}

fn add_query_range(document: &mut AggregatedCandidate, byte_start: usize, byte_end: usize) {
    if document
        .query_ranges
        .iter()
        .any(|range| range.snippet_byte_start == byte_start && range.snippet_byte_end == byte_end)
    {
        return;
    }
    document.query_ranges.push(SearchSnippetRange {
        role: SearchRangeRole::QueryMatch,
        snippet_byte_start: byte_start,
        snippet_byte_end: byte_end,
        snippet: document.statement[byte_start..byte_end].to_owned(),
        readback_verified: true,
    });
}

fn merge_ranges(statement: &str, mut ranges: Vec<SearchSnippetRange>) -> Vec<SearchSnippetRange> {
    ranges.sort_by_key(|range| (range.snippet_byte_start, range.snippet_byte_end));
    let mut merged = Vec::<SearchSnippetRange>::new();
    for range in ranges {
        if let Some(previous) = merged.last_mut()
            && range.snippet_byte_start < previous.snippet_byte_end
        {
            previous.snippet_byte_end = previous.snippet_byte_end.max(range.snippet_byte_end);
            previous.role = SearchRangeRole::merged(previous.role, range.role);
            previous.snippet =
                statement[previous.snippet_byte_start..previous.snippet_byte_end].to_owned();
            previous.readback_verified &= range.readback_verified;
            continue;
        }
        merged.push(range);
    }
    merged
}

fn materialize_items(
    snapshot: &SourceSnapshot,
    candidates: Vec<AggregatedCandidate>,
    result_limit: Option<usize>,
) -> Result<(Vec<SearchItem>, usize, usize)> {
    let mut items = Vec::new();
    let mut used = 0usize;
    let available = candidates.len();
    let selected = result_limit.unwrap_or(available);
    for candidate in candidates.into_iter().take(selected) {
        let source_id = format!("{}:{}", candidate.source_type.as_str(), candidate.entity_id);
        let mut item = SearchItem {
            source_id,
            source_type: candidate.source_type,
            entity_id: candidate.entity_id,
            entity_version_id: candidate.entity_version_id,
            entity_state_digest: candidate.state_digest,
            statement_digest: candidate.statement_digest,
            status: candidate.status,
            record_kind: candidate.record_kind,
            scope_json: candidate.scope_json,
            provenance_json: candidate.provenance_json,
            source_commit_id: snapshot.source_commit_id,
            temporal_scope: snapshot.temporal_scope,
            canonical_statement: candidate.statement,
            ranges: candidate.query_ranges,
            score: candidate.score,
            readback_verified: true,
            serialized_tokens: 0,
        };
        let tokens = complete_item_serialized_tokens(&mut item)?;
        used = used.checked_add(tokens).ok_or_else(|| {
            WorkVcsError::CanonicalEncodingInvalid(
                "serialized search item token observation exceeds usize".to_owned(),
            )
        })?;
        items.push(item);
    }
    let omitted = available.saturating_sub(items.len());
    Ok((items, used, omitted))
}

fn complete_item_serialized_tokens(item: &mut SearchItem) -> Result<usize> {
    for _ in 0..8 {
        let serialized = search_item_context_json(item)?;
        let tokens = o200k_base_singleton().encode_ordinary(&serialized).len();
        if tokens == item.serialized_tokens {
            return Ok(tokens);
        }
        item.serialized_tokens = tokens;
    }
    Err(WorkVcsError::CanonicalEncodingInvalid(
        "complete search item serialized token count did not reach a stable fixed point".to_owned(),
    ))
}

pub fn search_item_context_json(item: &SearchItem) -> Result<String> {
    serde_json::to_string(item).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!(
            "complete search item could not be serialized for token observation: {error}"
        ))
    })
}

fn refresh_required_result(
    options: &SearchQueryOptions,
    snapshot: &SourceSnapshot,
    issue: DerivedIndexIssue,
    manifest: Option<SearchIndexManifest>,
) -> SearchQueryResult {
    SearchQueryResult {
        status: SearchDerivedStatus::RefreshRequired,
        reason_code: Some(issue.code),
        reason: Some(issue.message),
        guidance: Some(NATIVE_READ_GUIDANCE.to_owned()),
        index_path: options.index_path.clone(),
        manifest,
        source_identity: snapshot.identity,
        temporal_scope: snapshot.temporal_scope,
        source_commit_id: snapshot.source_commit_id,
        source_state_digest: snapshot.source_state_digest,
        query: options.query.clone(),
        result_limit: options.result_limit,
        token_count_encoding: TOKEN_COUNT_ENCODING.to_owned(),
        serialized_item_tokens: 0,
        candidates_considered: 0,
        matched_objects: 0,
        omitted_results: 0,
        items: Vec::new(),
        canonical_readback_verified: false,
        index_maintenance: None,
    }
}

fn require_manifest_matches_snapshot(
    manifest: &SearchIndexManifest,
    snapshot: &SourceSnapshot,
) -> std::result::Result<(), DerivedIndexIssue> {
    if manifest.identity != snapshot.identity {
        return Err(DerivedIndexIssue::new(
            "source_identity_mismatch",
            "derived search ProjectRef, Store, Workspace, or Branch does not match routed source",
        ));
    }
    if manifest.temporal_scope != snapshot.temporal_scope
        || manifest.source_commit_id != snapshot.source_commit_id
        || manifest.source_state_digest != snapshot.source_state_digest
        || manifest.object_version_basis_digest != snapshot.object_version_basis_digest
    {
        return Err(DerivedIndexIssue::new(
            "stale_index",
            "derived search source commit, state digest, temporal scope, or object versions do not match the selected canonical snapshot",
        ));
    }
    if manifest.object_count != snapshot.documents.len() {
        return Err(DerivedIndexIssue::new(
            "damaged_index",
            "derived search object count does not match canonical source basis",
        ));
    }
    Ok(())
}

fn manifest_matches_snapshot(manifest: &SearchIndexManifest, snapshot: &SourceSnapshot) -> bool {
    require_manifest_matches_snapshot(manifest, snapshot).is_ok()
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn ensure_atomic_search_publication_supported() -> Result<()> {
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn ensure_atomic_search_publication_supported() -> Result<()> {
    Err(WorkVcsError::QueryUnsupported(
        "local derived search requires an operating system with kernel atomic no-replace rename and Unix file identity checks"
            .to_owned(),
    ))
}

struct SearchPublicationLock {
    directory: File,
}

impl SearchPublicationLock {
    fn acquire(destination: &Path) -> Result<Self> {
        let parent = destination.parent().ok_or_else(|| {
            WorkVcsError::QueryInvalid("search index destination has no parent".to_owned())
        })?;
        let directory = File::open(parent).map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "cannot open search index directory {} for publication locking: {error}",
                parent.display()
            ))
        })?;
        File::lock(&directory).map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "cannot exclusively lock search index directory {}: {error}",
                parent.display()
            ))
        })?;
        Ok(Self { directory })
    }
}

impl Drop for SearchPublicationLock {
    fn drop(&mut self) {
        let _ = File::unlock(&self.directory);
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn atomic_publish_new_index(temp_path: &Path, destination: &Path) -> Result<()> {
    use rustix::fs::{CWD, RenameFlags, renameat_with};

    renameat_with(CWD, temp_path, CWD, destination, RenameFlags::NOREPLACE).map_err(|error| {
        let detail = if error == rustix::io::Errno::EXIST {
            "the destination appeared after validation and was preserved"
        } else {
            "the kernel no-replace publication failed before installing the candidate"
        };
        WorkVcsError::StorageFailure(format!(
            "cannot publish new search index {}: {detail}: {error}",
            destination.display()
        ))
    })
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn atomic_publish_new_index(_temp_path: &Path, _destination: &Path) -> Result<()> {
    ensure_atomic_search_publication_supported()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SearchTestPoint {
    RefreshAfterPublish,
    QueryAfterIndexValidation,
    QueryBeforeFreshness,
    RefreshBeforeFreshness,
    RefreshBeforePublish,
    RefreshBeforeReuseFreshness,
}

#[cfg(not(test))]
fn search_test_pause(_point: SearchTestPoint) {}

#[cfg(test)]
#[derive(Clone)]
struct SearchTestPause {
    point: SearchTestPoint,
    reached: std::sync::Arc<std::sync::Barrier>,
    resume: std::sync::Arc<std::sync::Barrier>,
}

#[cfg(test)]
static SEARCH_TEST_PAUSE: std::sync::Mutex<Option<SearchTestPause>> = std::sync::Mutex::new(None);

#[cfg(test)]
fn search_test_pause(point: SearchTestPoint) {
    let pause = {
        let mut configured = SEARCH_TEST_PAUSE.lock().expect("search test pause lock");
        if configured.as_ref().map(|pause| pause.point) == Some(point) {
            configured.take()
        } else {
            None
        }
    };
    if let Some(pause) = pause {
        pause.reached.wait();
        pause.resume.wait();
    }
}

fn validate_refresh_destination(
    path: &Path,
    source_path: &Path,
    protected_paths: &[PathBuf],
    snapshot: &SourceSnapshot,
) -> Result<PathBuf> {
    let destination = validate_destination_shape(path, source_path, protected_paths)
        .map_err(derived_issue_error)?;
    if destination.exists() {
        let manifest = read_existing_manifest_for_refresh(&destination)?.ok_or_else(|| {
            WorkVcsError::QueryInvalid(format!(
                "refusing to overwrite unknown existing search index destination {}",
                destination.display()
            ))
        })?;
        if manifest.identity != snapshot.identity {
            return Err(WorkVcsError::QueryInvalid(format!(
                "refusing to replace search index {} for a different ProjectRef, Store, Workspace, or Branch",
                destination.display()
            )));
        }
    }
    Ok(destination)
}

fn validate_query_destination(
    path: &Path,
    source_path: &Path,
    protected_paths: &[PathBuf],
) -> std::result::Result<PathBuf, DerivedIndexIssue> {
    validate_destination_shape(path, source_path, protected_paths)
}

fn validate_destination_shape(
    path: &Path,
    source_path: &Path,
    protected_paths: &[PathBuf],
) -> std::result::Result<PathBuf, DerivedIndexIssue> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(DerivedIndexIssue::new(
            "unsafe_index_path",
            "search index path must be absolute without dot or parent components",
        ));
    }
    let parent = path.parent().ok_or_else(|| {
        DerivedIndexIssue::new("unsafe_index_path", "search index path has no parent")
    })?;
    let parent = fs::canonicalize(parent).map_err(|error| {
        DerivedIndexIssue::new(
            "unsafe_index_path",
            format!(
                "search index parent {} is unavailable: {error}",
                parent.display()
            ),
        )
    })?;
    let file_name = path.file_name().ok_or_else(|| {
        DerivedIndexIssue::new("unsafe_index_path", "search index path has no file name")
    })?;
    let destination = parent.join(file_name);
    let source = fs::canonicalize(source_path).map_err(|error| {
        DerivedIndexIssue::new(
            "source_path_invalid",
            format!("canonical Store path is unavailable: {error}"),
        )
    })?;
    if destination == source {
        return Err(DerivedIndexIssue::new(
            "unsafe_index_alias",
            "search index destination is the canonical Store",
        ));
    }
    let source_sidecars = sqlite_sidecar_paths(&source);
    if is_reserved_sqlite_sidecar_name(&destination, &source) {
        return Err(DerivedIndexIssue::new(
            "unsafe_index_alias",
            "search index destination is a reserved canonical Store SQLite sidecar path",
        ));
    }
    for protected in protected_paths {
        let protected = fs::canonicalize(protected).map_err(|error| {
            DerivedIndexIssue::new(
                "protected_path_invalid",
                format!(
                    "protected path {} is unavailable: {error}",
                    protected.display()
                ),
            )
        })?;
        if destination == protected {
            return Err(DerivedIndexIssue::new(
                "unsafe_index_alias",
                format!(
                    "search index destination aliases protected file {}",
                    protected.display()
                ),
            ));
        }
    }
    if let Ok(metadata) = fs::symlink_metadata(&destination) {
        if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
            return Err(DerivedIndexIssue::new(
                "unsafe_index_alias",
                "search index destination must be a regular non-symlink file",
            ));
        }
        if file_has_multiple_links(&metadata) {
            return Err(DerivedIndexIssue::new(
                "unsafe_index_alias",
                "search index destination has multiple hard links",
            ));
        }
        if same_file(&destination, &source).unwrap_or(false) {
            return Err(DerivedIndexIssue::new(
                "unsafe_index_alias",
                "search index destination is a hard-link alias of the canonical Store",
            ));
        }
        for sidecar in &source_sidecars {
            if sidecar.exists() && same_file(&destination, sidecar).unwrap_or(false) {
                return Err(DerivedIndexIssue::new(
                    "unsafe_index_alias",
                    "search index destination is a hard-link alias of a canonical Store SQLite sidecar",
                ));
            }
        }
        for protected in protected_paths {
            if same_file(&destination, protected).unwrap_or(false) {
                return Err(DerivedIndexIssue::new(
                    "unsafe_index_alias",
                    "search index destination is a hard-link alias of a protected file",
                ));
            }
        }
    }
    Ok(destination)
}

fn sqlite_sidecar_paths(database: &Path) -> [PathBuf; 3] {
    ["-journal", "-wal", "-shm"].map(|suffix| {
        let mut name = database
            .file_name()
            .expect("canonical Store path has a file name")
            .to_os_string();
        name.push(suffix);
        database.with_file_name(name)
    })
}

fn is_reserved_sqlite_sidecar_name(destination: &Path, database: &Path) -> bool {
    if destination.parent() != database.parent() {
        return false;
    }
    let (Some(destination_name), Some(database_name)) = (
        destination.file_name().and_then(|name| name.to_str()),
        database.file_name().and_then(|name| name.to_str()),
    ) else {
        return false;
    };
    ["-journal", "-wal", "-shm"]
        .iter()
        .any(|suffix| destination_name.eq_ignore_ascii_case(&format!("{database_name}{suffix}")))
}

fn read_existing_manifest_for_refresh(path: &Path) -> Result<Option<SearchIndexManifest>> {
    if !path.exists() {
        return Ok(None);
    }
    let connection = open_index_readonly(path).map_err(|issue| {
        WorkVcsError::QueryInvalid(format!(
            "refusing to overwrite unknown or damaged existing file {}: {}",
            path.display(),
            issue.message
        ))
    })?;
    read_manifest_unchecked(&connection).map(Some).map_err(|issue| {
        WorkVcsError::QueryInvalid(format!(
            "refusing to overwrite an existing file without a valid WorkVCS search manifest {}: {}",
            path.display(),
            issue.message
        ))
    })
}

struct SearchTempFile {
    path: PathBuf,
    armed: bool,
}

impl SearchTempFile {
    fn create(destination: &Path) -> Result<Self> {
        let parent = destination.parent().ok_or_else(|| {
            WorkVcsError::QueryInvalid("search index destination has no parent".to_owned())
        })?;
        let name = destination
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                WorkVcsError::QueryInvalid("search index file name must be UTF-8".to_owned())
            })?;
        for _ in 0..8 {
            let candidate = parent.join(format!(
                ".{name}.workvcs-search-{}.tmp",
                uuid::Uuid::now_v7()
            ));
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&candidate)
            {
                Ok(file) => {
                    let guard = Self {
                        path: candidate,
                        armed: true,
                    };
                    file.sync_all().map_err(|error| {
                        WorkVcsError::StorageFailure(format!(
                            "cannot initialize temporary search index {}: {error}",
                            guard.path.display()
                        ))
                    })?;
                    return Ok(guard);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(WorkVcsError::StorageFailure(format!(
                        "cannot exclusively create temporary search index in {}: {error}",
                        parent.display()
                    )));
                }
            }
        }
        Err(WorkVcsError::StorageFailure(
            "could not allocate a unique temporary search index".to_owned(),
        ))
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for SearchTempFile {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn validate_built_index(path: &Path, expected: &SearchIndexManifest) -> Result<()> {
    let connection = open_index_readonly(path).map_err(derived_issue_error)?;
    let manifest = read_manifest(&connection).map_err(derived_issue_error)?;
    if &manifest != expected {
        return Err(WorkVcsError::IntegrityInvalid(
            "built derived search index manifest failed exact readback".to_owned(),
        ));
    }
    let documents = count_table(&connection, "document").map_err(derived_issue_error)?;
    let snippets = count_table(&connection, "snippet").map_err(derived_issue_error)?;
    let fts_rows = count_table(&connection, "passage_fts").map_err(derived_issue_error)?;
    if documents != manifest.object_count
        || snippets != manifest.snippet_count
        || fts_rows != manifest.snippet_count
    {
        return Err(WorkVcsError::IntegrityInvalid(
            "built derived search index counts do not match manifest".to_owned(),
        ));
    }
    if !fts5_index_integrity_ok(&connection) {
        return Err(WorkVcsError::IntegrityInvalid(
            "built derived search FTS5 postings failed the engine integrity check".to_owned(),
        ));
    }
    Ok(())
}

fn existing_index_matches_snapshot(
    path: &Path,
    manifest: &SearchIndexManifest,
    snapshot: &SourceSnapshot,
) -> bool {
    let Ok(connection) = open_index_readonly(path) else {
        return false;
    };
    index_contents_match_snapshot(&connection, manifest, snapshot)
}

fn index_contents_match_snapshot(
    connection: &Connection,
    manifest: &SearchIndexManifest,
    snapshot: &SourceSnapshot,
) -> bool {
    if count_table(connection, "document").ok() != Some(manifest.object_count)
        || count_table(connection, "snippet").ok() != Some(manifest.snippet_count)
        || count_table(connection, "passage_fts").ok() != Some(manifest.snippet_count)
    {
        return false;
    }

    if !fts5_index_integrity_ok(connection) {
        return false;
    }

    let expected = snapshot
        .documents
        .iter()
        .map(|document| {
            (
                format!("{}:{}", document.source_type.as_str(), document.entity_id),
                document,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let Ok(mut statement) = connection.prepare(
        "SELECT document_key, source_type, entity_id, entity_version_id,
                entity_state_digest, statement_digest, status, record_kind,
                statement, scope_json, provenance_json
         FROM document ORDER BY document_key",
    ) else {
        return false;
    };
    type DocumentRow = (
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        String,
        String,
        Option<String>,
    );
    let Ok(rows) = statement.query_map([], |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
            row.get(7)?,
            row.get(8)?,
            row.get(9)?,
            row.get(10)?,
        ))
    }) else {
        return false;
    };
    for row in rows {
        let row: DocumentRow = match row {
            Ok(row) => row,
            Err(_) => return false,
        };
        let Some(document) = expected.get(&row.0) else {
            return false;
        };
        if row.1 != document.source_type.as_str()
            || row.2 != document.entity_id.to_string()
            || row.3 != document.entity_version_id.to_string()
            || row.4 != document.state_digest.to_string()
            || row.5 != document.statement_digest.to_string()
            || row.6 != document.status
            || row.7 != document.record_kind
            || row.8 != document.statement
            || row.9 != document.scope_json
            || row.10 != document.provenance_json
        {
            return false;
        }
    }

    let expected_snippets = expected_snippet_rows(snapshot);
    let Ok(mut statement) = connection.prepare(
        "SELECT s.snippet_id, s.document_key, s.byte_start, s.byte_end, s.snippet,
                s.snippet_digest, f.terms
         FROM snippet s JOIN passage_fts f ON f.snippet_id = s.snippet_id
         ORDER BY s.snippet_id",
    ) else {
        return false;
    };
    let Ok(rows) = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
        ))
    }) else {
        return false;
    };
    let mut actual_snippets = Vec::new();
    for row in rows {
        let Ok((snippet_id, document_key, start, end, snippet, digest, terms)) = row else {
            return false;
        };
        let (Ok(snippet_id), Ok(start), Ok(end)) = (
            usize::try_from(snippet_id),
            usize::try_from(start),
            usize::try_from(end),
        ) else {
            return false;
        };
        let Ok(snippet_digest) = Digest::from_hex(&digest) else {
            return false;
        };
        actual_snippets.push(ExpectedSnippetRow {
            snippet_id,
            document_key,
            byte_start: start,
            byte_end: end,
            snippet,
            snippet_digest,
            terms,
        });
    }
    actual_snippets == expected_snippets
}

fn fts5_index_integrity_ok(connection: &Connection) -> bool {
    let Ok(mut copy) = Connection::open_in_memory() else {
        return false;
    };
    let copied = {
        let Ok(backup) = rusqlite::backup::Backup::new(connection, &mut copy) else {
            return false;
        };
        backup
            .run_to_completion(256, std::time::Duration::ZERO, None)
            .is_ok()
    };
    copied
        && copy
            .execute(
                "INSERT INTO passage_fts(passage_fts, rank) VALUES('integrity-check', 1)",
                [],
            )
            .is_ok()
}

fn count_table(
    connection: &Connection,
    table: &str,
) -> std::result::Result<usize, DerivedIndexIssue> {
    let sql = format!("SELECT count(*) FROM {table}");
    let count = connection
        .query_row(&sql, [], |row| row.get::<_, i64>(0))
        .map_err(|error| DerivedIndexIssue::new("damaged_index", error.to_string()))?;
    usize::try_from(count)
        .map_err(|_| DerivedIndexIssue::new("damaged_index", "negative table count"))
}

fn sync_file(path: &Path) -> Result<()> {
    OpenOptions::new()
        .read(true)
        .open(path)
        .and_then(|file| file.sync_all())
        .map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "cannot sync derived search index {}: {error}",
                path.display()
            ))
        })
}

fn sync_parent(path: &Path) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        WorkVcsError::StorageFailure("published search index has no parent".to_owned())
    })?;
    fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "cannot sync search index directory {}: {error}",
                parent.display()
            ))
        })
}

fn source_counts(documents: &[SourceDocument]) -> (usize, usize) {
    documents.iter().fold((0, 0), |(records, knowledge), item| {
        match item.source_type {
            SearchSourceType::Record => (records + 1, knowledge),
            SearchSourceType::Knowledge => (records, knowledge + 1),
        }
    })
}

fn validate_query(query: &str) -> Result<()> {
    if query.trim().is_empty() {
        return Err(WorkVcsError::QueryInvalid(
            "search query must contain non-whitespace text".to_owned(),
        ));
    }
    Ok(())
}

fn usize_to_i64(label: &str, value: usize) -> Result<i64> {
    i64::try_from(value)
        .map_err(|_| WorkVcsError::QueryInvalid(format!("{label} exceeds SQLite integer range")))
}

fn derived_storage(error: rusqlite::Error) -> WorkVcsError {
    WorkVcsError::StorageFailure(format!("derived search index storage failed: {error}"))
}

fn derived_issue_error(issue: DerivedIndexIssue) -> WorkVcsError {
    WorkVcsError::QueryInvalid(format!(
        "derived search index {}: {}",
        issue.code, issue.message
    ))
}

#[cfg(unix)]
fn file_has_multiple_links(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    metadata.nlink() > 1
}

#[cfg(not(unix))]
fn file_has_multiple_links(_metadata: &fs::Metadata) -> bool {
    false
}

#[cfg(unix)]
fn same_file(left: &Path, right: &Path) -> std::io::Result<bool> {
    use std::os::unix::fs::MetadataExt;
    let left = fs::metadata(left)?;
    let right = fs::metadata(right)?;
    Ok(left.dev() == right.dev() && left.ino() == right.ino())
}

#[cfg(not(unix))]
fn same_file(left: &Path, right: &Path) -> std::io::Result<bool> {
    Ok(fs::canonicalize(left)? == fs::canonicalize(right)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Engine, RecordCreateOptions, StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions,
    };
    use std::sync::{Arc, Barrier, Mutex};
    use tempfile::TempDir;

    static SEARCH_CONCURRENCY_TEST: Mutex<()> = Mutex::new(());

    struct TestFixture {
        _tempdir: TempDir,
        source_path: PathBuf,
        index_path: PathBuf,
        identity: SearchSourceIdentity,
        workspace: WorkspaceInfo,
        head_commit_id: CommitId,
    }

    fn test_fixture(name: &str) -> TestFixture {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let source_path = tempdir.path().join("workvcs.sqlite");
        let index_path = tempdir.path().join("search.sqlite");
        let mut engine = Engine::init(
            &source_path,
            StoreInitOptions::new(format!("{name}-store")).expect("store options"),
        )
        .expect("init engine");
        let store_id = engine.store_info().expect("store info").store_id;
        let workspace = engine
            .create_workspace(
                WorkspaceInitOptions::new(format!("{name}-workspace")).expect("workspace options"),
            )
            .expect("workspace");
        let first = engine
            .create_record(
                RecordCreateOptions::finding(
                    workspace.initial_branch_id,
                    workspace.genesis_commit_id,
                    "initial_search_marker canonical source",
                )
                .expect("record options"),
            )
            .expect("initial record");
        TestFixture {
            _tempdir: tempdir,
            source_path,
            index_path,
            identity: SearchSourceIdentity::new(
                ProjectRefId::new_v7(),
                store_id,
                workspace.workspace_id,
                workspace.initial_branch_id,
            ),
            workspace,
            head_commit_id: first.commit_id,
        }
    }

    fn install_pause(point: SearchTestPoint) -> (Arc<Barrier>, Arc<Barrier>) {
        let reached = Arc::new(Barrier::new(2));
        let resume = Arc::new(Barrier::new(2));
        *SEARCH_TEST_PAUSE.lock().expect("search test pause lock") = Some(SearchTestPause {
            point,
            reached: Arc::clone(&reached),
            resume: Arc::clone(&resume),
        });
        (reached, resume)
    }

    #[test]
    fn managed_query_reports_generation_published_before_head_advance() {
        let _serial = SEARCH_CONCURRENCY_TEST.lock().expect("search test lock");
        let fixture = test_fixture("managed-post-publication-head-guard");
        let directory = fs::canonicalize(fixture.source_path.parent().unwrap())
            .unwrap()
            .join("managed");
        let (reached, resume) = install_pause(SearchTestPoint::RefreshAfterPublish);
        let source = fixture.source_path.clone();
        let identity = fixture.identity;
        let query_directory = directory.clone();
        let query = std::thread::spawn(move || {
            Engine::open_readonly(&source)
                .unwrap()
                .search(
                    SearchQueryOptions::managed(identity, query_directory, "initial_search_marker")
                        .unwrap(),
                )
                .unwrap()
        });
        reached.wait();
        let published: Vec<_> = fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(published.len(), 1, "complete publication already occurred");
        let published_bytes = fs::read(&published[0]).unwrap();
        Engine::open(&fixture.source_path)
            .unwrap()
            .create_record(
                RecordCreateOptions::finding(
                    fixture.workspace.initial_branch_id,
                    fixture.head_commit_id,
                    "head_advanced_after_managed_publication",
                )
                .unwrap(),
            )
            .unwrap();
        resume.wait();
        let result = query.join().unwrap();
        assert_eq!(result.status, SearchDerivedStatus::RefreshRequired);
        assert_eq!(
            result.reason_code.as_deref(),
            Some("source_changed_during_query")
        );
        assert_eq!(
            result.index_maintenance,
            Some(SearchRefreshOutcome::Refreshed)
        );
        assert_eq!(result.index_path, published[0]);
        assert!(result.manifest.is_some());
        assert_eq!(result.source_commit_id, fixture.head_commit_id);
        assert!(result.items.is_empty());
        assert!(!result.canonical_readback_verified);
        assert_eq!(fs::read(&published[0]).unwrap(), published_bytes);
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
    }

    #[test]
    fn healthy_managed_query_does_not_wait_for_publication_lock() {
        let _serial = SEARCH_CONCURRENCY_TEST.lock().expect("search test lock");
        let fixture = test_fixture("managed-lock-free-reuse");
        let directory = fs::canonicalize(fixture.source_path.parent().unwrap())
            .unwrap()
            .join("managed");
        Engine::open_readonly(&fixture.source_path)
            .unwrap()
            .search(
                SearchQueryOptions::managed(fixture.identity, &directory, "initial_search_marker")
                    .unwrap(),
            )
            .unwrap();
        let publication_lock =
            SearchPublicationLock::acquire(&directory.join("generation.sqlite")).unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        let source = fixture.source_path.clone();
        let identity = fixture.identity;
        let query = std::thread::spawn(move || {
            let result = Engine::open_readonly(&source).unwrap().search(
                SearchQueryOptions::managed(identity, directory, "initial_search_marker").unwrap(),
            );
            sender.send(result).unwrap();
        });
        let observed = receiver.recv_timeout(std::time::Duration::from_secs(10));
        drop(publication_lock);
        query.join().unwrap();
        let result = observed
            .expect("healthy reuse must complete while the publisher lock is held")
            .unwrap();
        assert_eq!(result.status, SearchDerivedStatus::Ready);
        assert_eq!(result.index_maintenance, Some(SearchRefreshOutcome::Reused));
    }

    #[test]
    fn managed_query_rejects_head_change_and_preserves_immutable_generation() {
        let _serial = SEARCH_CONCURRENCY_TEST.lock().expect("search test lock");
        let fixture = test_fixture("managed-final-head-guard");
        let directory = fs::canonicalize(fixture.source_path.parent().unwrap())
            .unwrap()
            .join("managed");
        let first = Engine::open_readonly(&fixture.source_path)
            .unwrap()
            .search(
                SearchQueryOptions::managed(fixture.identity, &directory, "initial_search_marker")
                    .unwrap(),
            )
            .unwrap();
        let original = fs::read(&first.index_path).unwrap();
        let (reached, resume) = install_pause(SearchTestPoint::QueryBeforeFreshness);
        let source = fixture.source_path.clone();
        let identity = fixture.identity;
        let query_directory = directory.clone();
        let query = std::thread::spawn(move || {
            Engine::open_readonly(&source)
                .unwrap()
                .search(
                    SearchQueryOptions::managed(identity, query_directory, "initial_search_marker")
                        .unwrap(),
                )
                .unwrap()
        });
        reached.wait();
        Engine::open(&fixture.source_path)
            .unwrap()
            .create_record(
                RecordCreateOptions::finding(
                    fixture.workspace.initial_branch_id,
                    fixture.head_commit_id,
                    "head_advanced_during_managed_query",
                )
                .unwrap(),
            )
            .unwrap();
        resume.wait();
        let result = query.join().unwrap();
        assert_eq!(
            result.reason_code.as_deref(),
            Some("source_changed_during_query")
        );
        assert!(result.items.is_empty());
        assert!(!result.canonical_readback_verified);
        assert_eq!(fs::read(first.index_path).unwrap(), original);
        let fresh = Engine::open_readonly(&fixture.source_path)
            .unwrap()
            .search(
                SearchQueryOptions::managed(fixture.identity, directory, "initial_search_marker")
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(fresh.status, SearchDerivedStatus::Ready);
        assert_ne!(fresh.source_commit_id, result.source_commit_id);
    }

    #[test]
    fn current_query_rejects_head_change_after_result_materialization() {
        let _serial = SEARCH_CONCURRENCY_TEST
            .lock()
            .expect("search concurrency test lock");
        let fixture = test_fixture("query-final-head-guard");
        Engine::open_readonly(&fixture.source_path)
            .expect("readonly source")
            .refresh_search_index(SearchRefreshOptions::new(
                fixture.identity,
                &fixture.index_path,
            ))
            .expect("refresh index");
        let (reached, resume) = install_pause(SearchTestPoint::QueryBeforeFreshness);
        let source_path = fixture.source_path.clone();
        let index_path = fixture.index_path.clone();
        let identity = fixture.identity;
        let query_thread = std::thread::spawn(move || {
            Engine::open_readonly(&source_path)
                .expect("thread readonly source")
                .search(
                    SearchQueryOptions::new(identity, &index_path, "initial_search_marker")
                        .expect("query options"),
                )
                .expect("query result")
        });
        reached.wait();
        let mut writer = Engine::open(&fixture.source_path).expect("writer");
        writer
            .create_record(
                RecordCreateOptions::finding(
                    fixture.workspace.initial_branch_id,
                    fixture.head_commit_id,
                    "head_advanced_during_query_marker",
                )
                .expect("record options"),
            )
            .expect("advance source");
        resume.wait();
        let result = query_thread.join().expect("query thread");
        assert_eq!(result.status, SearchDerivedStatus::RefreshRequired);
        assert_eq!(
            result.reason_code.as_deref(),
            Some("source_changed_during_query")
        );
        assert!(result.items.is_empty());
    }

    #[test]
    fn derived_query_uses_one_sqlite_snapshot_across_validation_match_and_readback() {
        let _serial = SEARCH_CONCURRENCY_TEST
            .lock()
            .expect("search concurrency test lock");
        let fixture = test_fixture("query-one-derived-snapshot");
        Engine::open_readonly(&fixture.source_path)
            .expect("readonly source")
            .refresh_search_index(SearchRefreshOptions::new(
                fixture.identity,
                &fixture.index_path,
            ))
            .expect("refresh index");
        let index_writer = Connection::open(&fixture.index_path).expect("open derived writer");
        let journal_mode = index_writer
            .query_row("PRAGMA journal_mode=WAL", [], |row| row.get::<_, String>(0))
            .expect("enable WAL for controlled concurrent writer");
        assert_eq!(journal_mode, "wal");
        drop(index_writer);

        let (reached, resume) = install_pause(SearchTestPoint::QueryAfterIndexValidation);
        let source_path = fixture.source_path.clone();
        let index_path = fixture.index_path.clone();
        let identity = fixture.identity;
        let query_thread = std::thread::spawn(move || {
            Engine::open_readonly(&source_path)
                .expect("thread readonly source")
                .search(
                    SearchQueryOptions::new(identity, &index_path, "initial_search_marker")
                        .expect("query options"),
                )
                .expect("query result")
        });
        reached.wait();
        let index_writer = Connection::open(&fixture.index_path).expect("open concurrent writer");
        index_writer
            .execute(
                "UPDATE passage_fts SET terms='external_changed_marker' WHERE snippet_id=1",
                [],
            )
            .expect("advance derived database generation in WAL");
        drop(index_writer);
        resume.wait();

        let stable = query_thread.join().expect("query thread");
        assert_eq!(stable.status, SearchDerivedStatus::Ready);
        assert_eq!(stable.items.len(), 1);
        assert!(
            stable.items[0]
                .canonical_statement
                .contains("initial_search_marker")
        );

        let changed = Engine::open_readonly(&fixture.source_path)
            .expect("readonly source after mutation")
            .search(
                SearchQueryOptions::new(
                    fixture.identity,
                    &fixture.index_path,
                    "initial_search_marker",
                )
                .expect("query options"),
            )
            .expect("changed generation diagnostic");
        assert_eq!(changed.status, SearchDerivedStatus::RefreshRequired);
        assert_eq!(changed.reason_code.as_deref(), Some("damaged_index"));
    }

    #[test]
    fn reused_refresh_rejects_head_change_after_generation_validation() {
        let _serial = SEARCH_CONCURRENCY_TEST
            .lock()
            .expect("search concurrency test lock");
        let fixture = test_fixture("refresh-reuse-final-head-guard");
        Engine::open_readonly(&fixture.source_path)
            .expect("readonly source")
            .refresh_search_index(SearchRefreshOptions::new(
                fixture.identity,
                &fixture.index_path,
            ))
            .expect("initial refresh");
        let (reached, resume) = install_pause(SearchTestPoint::RefreshBeforeReuseFreshness);
        let source_path = fixture.source_path.clone();
        let index_path = fixture.index_path.clone();
        let identity = fixture.identity;
        let reuse_thread = std::thread::spawn(move || {
            Engine::open_readonly(&source_path)
                .expect("thread readonly source")
                .refresh_search_index(SearchRefreshOptions::new(identity, index_path))
        });
        reached.wait();
        let mut writer = Engine::open(&fixture.source_path).expect("writer");
        writer
            .create_record(
                RecordCreateOptions::finding(
                    fixture.workspace.initial_branch_id,
                    fixture.head_commit_id,
                    "head_advanced_during_reuse_marker",
                )
                .expect("record options"),
            )
            .expect("advance source");
        resume.wait();
        assert!(matches!(
            reuse_thread.join().expect("reuse thread"),
            Err(WorkVcsError::BranchHeadConflict(_))
        ));
    }

    #[test]
    fn old_head_refresh_cannot_publish_after_new_head_refresh() {
        let _serial = SEARCH_CONCURRENCY_TEST
            .lock()
            .expect("search concurrency test lock");
        let fixture = test_fixture("refresh-final-head-guard");
        let (reached, resume) = install_pause(SearchTestPoint::RefreshBeforeFreshness);
        let old_source = fixture.source_path.clone();
        let old_index = fixture.index_path.clone();
        let identity = fixture.identity;
        let old_refresh = std::thread::spawn(move || {
            Engine::open_readonly(&old_source)
                .expect("old readonly source")
                .refresh_search_index(SearchRefreshOptions::new(identity, old_index))
        });
        reached.wait();

        let mut writer = Engine::open(&fixture.source_path).expect("writer");
        writer
            .create_record(
                RecordCreateOptions::finding(
                    fixture.workspace.initial_branch_id,
                    fixture.head_commit_id,
                    "new_head_refresh_marker canonical source",
                )
                .expect("record options"),
            )
            .expect("advance source");
        drop(writer);

        let new_source = fixture.source_path.clone();
        let new_index = fixture.index_path.clone();
        let start_new = Arc::new(Barrier::new(2));
        let start_new_thread = Arc::clone(&start_new);
        let new_refresh = std::thread::spawn(move || {
            let readonly = Engine::open_readonly(&new_source).expect("new readonly source");
            start_new_thread.wait();
            readonly.refresh_search_index(SearchRefreshOptions::new(identity, new_index))
        });
        start_new.wait();
        resume.wait();

        assert!(matches!(
            old_refresh.join().expect("old refresh thread"),
            Err(WorkVcsError::BranchHeadConflict(_))
        ));
        let published = new_refresh
            .join()
            .expect("new refresh thread")
            .expect("new head refresh");
        assert_eq!(published.outcome, SearchRefreshOutcome::Refreshed);
        let result = Engine::open_readonly(&fixture.source_path)
            .expect("final readonly source")
            .search(
                SearchQueryOptions::new(
                    fixture.identity,
                    &fixture.index_path,
                    "new_head_refresh_marker",
                )
                .expect("query options"),
            )
            .expect("query newest generation");
        assert_eq!(result.status, SearchDerivedStatus::Ready);
        assert_eq!(result.items.len(), 1);
    }

    #[test]
    fn no_replace_publication_preserves_file_created_after_validation() {
        let _serial = SEARCH_CONCURRENCY_TEST
            .lock()
            .expect("search concurrency test lock");
        let fixture = test_fixture("refresh-no-replace");
        let (reached, resume) = install_pause(SearchTestPoint::RefreshBeforePublish);
        let source_path = fixture.source_path.clone();
        let index_path = fixture.index_path.clone();
        let identity = fixture.identity;
        let refresh = std::thread::spawn(move || {
            Engine::open_readonly(&source_path)
                .expect("readonly source")
                .refresh_search_index(SearchRefreshOptions::new(identity, index_path))
        });
        reached.wait();
        fs::write(
            &fixture.index_path,
            b"unrelated user file created during refresh",
        )
        .expect("create racing unrelated file");
        resume.wait();
        assert!(refresh.join().expect("refresh thread").is_err());
        assert_eq!(
            fs::read(&fixture.index_path).expect("read racing file"),
            b"unrelated user file created during refresh"
        );
    }

    #[test]
    fn exclusive_temp_guard_removes_only_its_own_armed_file() {
        let directory = tempfile::tempdir().expect("tempdir");
        let destination = directory.path().join("search.sqlite");
        let guarded_path = {
            let guard = SearchTempFile::create(&destination).expect("create guarded temp file");
            assert!(guard.path().exists());
            guard.path().to_path_buf()
        };
        assert!(!guarded_path.exists());

        let retained_path = {
            let mut guard =
                SearchTempFile::create(&destination).expect("create disarmed temp file");
            let path = guard.path().to_path_buf();
            guard.disarm();
            path
        };
        assert!(retained_path.exists());
        fs::remove_file(retained_path).expect("remove retained test file");
    }
}
