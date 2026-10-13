use super::{SearchSourceIdentity, search_lexical_terms, search_o200k_ordinary_token_count};
use crate::error::{Result, WorkVcsError};
use serde::{Deserialize, Serialize};
use sha2::{Digest as ShaDigest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};

const ARCHIVE_FORMAT_VERSION: u32 = 1;
const EXTRACTION_POLICY_VERSION: &str = "codex-rollout-user-assistant-text-v1";
const MANIFEST_FILE: &str = "manifest.json";
const DOCUMENTS_FILE: &str = "documents.json";
const AUTHORITY_CLASS: &str = "historical_reference_only";
const CURRENTNESS_CLASS: &str = "unknown_historical";
const ACTION_AUTHORITY: &str = "none";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceArchiveFormat {
    CodexRolloutJsonl,
}

#[derive(Clone, Debug)]
pub struct SourceArchiveImportOptions {
    identity: SearchSourceIdentity,
    archive_root: PathBuf,
    sources: Vec<PathBuf>,
    include_assistant: bool,
}

impl SourceArchiveImportOptions {
    pub fn new(
        identity: SearchSourceIdentity,
        archive_root: impl Into<PathBuf>,
        sources: Vec<PathBuf>,
    ) -> Result<Self> {
        if sources.is_empty() {
            return Err(WorkVcsError::QueryInvalid(
                "source archive import requires at least one explicit source".to_owned(),
            ));
        }
        Ok(Self {
            identity,
            archive_root: archive_root.into(),
            sources,
            include_assistant: true,
        })
    }

    pub fn include_assistant(mut self, include: bool) -> Self {
        self.include_assistant = include;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceArchiveImportOutcome {
    Imported,
    Reused,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceArchiveFileManifest {
    pub source_path: PathBuf,
    pub source_sha256: String,
    pub source_bytes: u64,
    pub imported_messages: usize,
    pub ignored_roles: BTreeMap<String, usize>,
    pub ignored_content_blocks: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceArchiveDocument {
    pub document_id: String,
    pub source_path: PathBuf,
    pub source_sha256: String,
    pub source_line: usize,
    pub source_thread_id: String,
    pub source_message_id: String,
    pub source_timestamp: Option<String>,
    pub source_role: String,
    pub text: String,
    pub text_sha256: String,
    pub authority_class: String,
    pub currentness_class: String,
    pub action_authority: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceArchiveManifest {
    pub format_version: u32,
    pub source_format: SourceArchiveFormat,
    pub extraction_policy_version: String,
    pub identity: SearchSourceIdentity,
    pub include_assistant: bool,
    pub authority_class: String,
    pub currentness_class: String,
    pub action_authority: String,
    pub files: Vec<SourceArchiveFileManifest>,
    pub document_count: usize,
    pub documents_sha256: String,
    pub basis_sha256: String,
    pub created_at_us: i64,
    pub manifest_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceArchiveImportResult {
    pub outcome: SourceArchiveImportOutcome,
    pub archive_path: PathBuf,
    pub manifest: SourceArchiveManifest,
    pub canonical_source_written: bool,
    pub raw_source_copied: bool,
}

#[derive(Clone, Debug)]
pub struct SourceArchiveInspectOptions {
    identity: SearchSourceIdentity,
    archive_path: PathBuf,
}

impl SourceArchiveInspectOptions {
    pub fn new(identity: SearchSourceIdentity, archive_path: impl Into<PathBuf>) -> Self {
        Self {
            identity,
            archive_path: archive_path.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceArchiveFileObservation {
    pub source_path: PathBuf,
    pub state: SourceArchiveFileState,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceArchiveInspectResult {
    pub archive_path: PathBuf,
    pub manifest: SourceArchiveManifest,
    pub file_observations: Vec<SourceArchiveFileObservation>,
    pub archive_readback_verified: bool,
    pub canonical_source_written: bool,
}

#[derive(Clone, Debug)]
pub struct SourceArchiveQueryOptions {
    identity: SearchSourceIdentity,
    archive_path: PathBuf,
    query: String,
    result_limit: Option<usize>,
}

impl SourceArchiveQueryOptions {
    pub fn new(
        identity: SearchSourceIdentity,
        archive_path: impl Into<PathBuf>,
        query: impl Into<String>,
    ) -> Result<Self> {
        let query = query.into();
        super::validate_query(&query)?;
        Ok(Self {
            identity,
            archive_path: archive_path.into(),
            query,
            result_limit: None,
        })
    }

    pub fn with_result_limit(mut self, limit: usize) -> Result<Self> {
        if limit == 0 {
            return Err(WorkVcsError::QueryInvalid(
                "source archive result limit must be greater than zero".to_owned(),
            ));
        }
        self.result_limit = Some(limit);
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceArchiveFileState {
    Unchanged,
    Changed,
    Missing,
    Unreadable,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SourceArchiveSearchItem {
    pub document: SourceArchiveDocument,
    pub score: f64,
    pub matched_terms: Vec<String>,
    pub source_file_state: SourceArchiveFileState,
    pub archive_readback_verified: bool,
    pub serialized_tokens: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SourceArchiveQueryResult {
    pub status: String,
    pub query: String,
    pub archive_path: PathBuf,
    pub manifest: SourceArchiveManifest,
    pub result_limit: Option<usize>,
    pub candidates_considered: usize,
    pub matched_documents: usize,
    pub omitted_results: usize,
    pub serialized_item_tokens: usize,
    pub items: Vec<SourceArchiveSearchItem>,
    pub authority_class: String,
    pub currentness_class: String,
    pub action_authority: String,
    pub archive_readback_verified: bool,
    pub canonical_source_written: bool,
}

#[derive(Serialize)]
struct ArchiveBasis<'a> {
    format_version: u32,
    source_format: SourceArchiveFormat,
    extraction_policy_version: &'a str,
    identity: SearchSourceIdentity,
    include_assistant: bool,
    authority_class: &'a str,
    currentness_class: &'a str,
    action_authority: &'a str,
    files: &'a [SourceArchiveFileManifest],
}

pub(crate) fn import_source_archive(
    options: &SourceArchiveImportOptions,
) -> Result<SourceArchiveImportResult> {
    let paths = collect_sources(&options.sources)?;
    let root = prepare_archive_root(&options.archive_root, &paths)?;
    let mut files = Vec::with_capacity(paths.len());
    let mut documents = Vec::new();
    for source in paths {
        let (file, mut extracted) = extract_codex_rollout(&source, options.include_assistant)?;
        files.push(file);
        documents.append(&mut extracted);
    }
    files.sort_by(|left, right| left.source_path.cmp(&right.source_path));
    documents.sort_by(|left, right| left.document_id.cmp(&right.document_id));
    require_unique_documents(&documents)?;
    if documents.is_empty() {
        return Err(WorkVcsError::QueryInvalid(
            "source archive import found no user or selected assistant text messages".to_owned(),
        ));
    }

    let basis_sha256 = sha256_json(&ArchiveBasis {
        format_version: ARCHIVE_FORMAT_VERSION,
        source_format: SourceArchiveFormat::CodexRolloutJsonl,
        extraction_policy_version: EXTRACTION_POLICY_VERSION,
        identity: options.identity,
        include_assistant: options.include_assistant,
        authority_class: AUTHORITY_CLASS,
        currentness_class: CURRENTNESS_CLASS,
        action_authority: ACTION_AUTHORITY,
        files: &files,
    })?;
    let destination = root.join(&basis_sha256);
    let _publication_lock = super::SearchPublicationLock::acquire(&destination)?;
    if destination.exists() {
        let (manifest, _) = validate_archive(&destination, Some(options.identity))?;
        return Ok(SourceArchiveImportResult {
            outcome: SourceArchiveImportOutcome::Reused,
            archive_path: destination,
            manifest,
            canonical_source_written: false,
            raw_source_copied: false,
        });
    }

    let stage = root.join(format!(".workvcs-source-{}.tmp", uuid::Uuid::now_v7()));
    create_private_directory(&stage)?;
    let result = (|| {
        let document_bytes = serde_json::to_vec_pretty(&documents).map_err(|error| {
            WorkVcsError::CanonicalEncodingInvalid(format!(
                "cannot encode source archive documents: {error}"
            ))
        })?;
        let documents_sha256 = sha256_bytes(&document_bytes);
        write_new(&stage.join(DOCUMENTS_FILE), &document_bytes)?;
        let mut manifest = SourceArchiveManifest {
            format_version: ARCHIVE_FORMAT_VERSION,
            source_format: SourceArchiveFormat::CodexRolloutJsonl,
            extraction_policy_version: EXTRACTION_POLICY_VERSION.to_owned(),
            identity: options.identity,
            include_assistant: options.include_assistant,
            authority_class: AUTHORITY_CLASS.to_owned(),
            currentness_class: CURRENTNESS_CLASS.to_owned(),
            action_authority: ACTION_AUTHORITY.to_owned(),
            files,
            document_count: documents.len(),
            documents_sha256,
            basis_sha256,
            created_at_us: crate::store::current_epoch_micros()?,
            manifest_sha256: String::new(),
        };
        manifest.manifest_sha256 = manifest_sha256(&manifest)?;
        write_json_new(&stage.join(MANIFEST_FILE), &manifest)?;
        sync_directory(&stage)?;
        validate_archive(&stage, Some(options.identity))?;
        super::atomic_publish_new_index(&stage, &destination)?;
        super::sync_parent(&destination)?;
        Ok(manifest)
    })();
    match result {
        Ok(manifest) => Ok(SourceArchiveImportResult {
            outcome: SourceArchiveImportOutcome::Imported,
            archive_path: destination,
            manifest,
            canonical_source_written: false,
            raw_source_copied: false,
        }),
        Err(error) => {
            if stage.exists() {
                let _ = fs::remove_dir_all(&stage);
            }
            Err(error)
        }
    }
}

pub(crate) fn inspect_source_archive(
    options: &SourceArchiveInspectOptions,
) -> Result<SourceArchiveInspectResult> {
    let (manifest, _) = validate_archive(&options.archive_path, Some(options.identity))?;
    let file_observations = manifest
        .files
        .iter()
        .map(|file| SourceArchiveFileObservation {
            source_path: file.source_path.clone(),
            state: observe_source_file(file),
        })
        .collect();
    Ok(SourceArchiveInspectResult {
        archive_path: options.archive_path.clone(),
        manifest,
        file_observations,
        archive_readback_verified: true,
        canonical_source_written: false,
    })
}

pub(crate) fn query_source_archive(
    options: &SourceArchiveQueryOptions,
) -> Result<SourceArchiveQueryResult> {
    let (manifest, documents) = validate_archive(&options.archive_path, Some(options.identity))?;
    let candidates_considered = documents.len();
    let query_terms = search_lexical_terms(&options.query);
    let mut document_tokens = Vec::with_capacity(documents.len());
    let mut document_frequency = HashMap::<String, usize>::new();
    for document in &documents {
        let tokens = search_lexical_terms(&document.text);
        let unique = tokens.iter().cloned().collect::<BTreeSet<_>>();
        for term in unique {
            *document_frequency.entry(term).or_default() += 1;
        }
        document_tokens.push(tokens);
    }
    let average_length = if document_tokens.is_empty() {
        0.0
    } else {
        document_tokens.iter().map(Vec::len).sum::<usize>() as f64 / document_tokens.len() as f64
    };
    let file_states = manifest
        .files
        .iter()
        .map(|file| (file.source_path.clone(), observe_source_file(file)))
        .collect::<BTreeMap<_, _>>();

    let mut items = Vec::new();
    for (document, tokens) in documents.into_iter().zip(document_tokens) {
        let frequencies =
            tokens
                .into_iter()
                .fold(HashMap::<String, usize>::new(), |mut map, token| {
                    *map.entry(token).or_default() += 1;
                    map
                });
        let matched_terms = query_terms
            .iter()
            .filter(|term| frequencies.contains_key(*term))
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if matched_terms.is_empty() {
            continue;
        }
        let length = frequencies.values().sum::<usize>();
        let score = bm25_score(
            &matched_terms,
            &frequencies,
            &document_frequency,
            manifest.document_count,
            length,
            average_length,
        );
        let source_file_state = *file_states
            .get(&document.source_path)
            .unwrap_or(&SourceArchiveFileState::Unreadable);
        let mut item = SourceArchiveSearchItem {
            document,
            score,
            matched_terms,
            source_file_state,
            archive_readback_verified: true,
            serialized_tokens: 0,
        };
        item.serialized_tokens = serialized_tokens(&item)?;
        items.push(item);
    }
    items.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.document.document_id.cmp(&right.document.document_id))
    });
    let matched_documents = items.len();
    let omitted_results = options
        .result_limit
        .map(|limit| items.len().saturating_sub(limit))
        .unwrap_or_default();
    if let Some(limit) = options.result_limit {
        items.truncate(limit);
    }
    let serialized_item_tokens = items.iter().map(|item| item.serialized_tokens).sum();
    Ok(SourceArchiveQueryResult {
        status: "ready".to_owned(),
        query: options.query.clone(),
        archive_path: options.archive_path.clone(),
        manifest,
        result_limit: options.result_limit,
        candidates_considered,
        matched_documents,
        omitted_results,
        serialized_item_tokens,
        items,
        authority_class: AUTHORITY_CLASS.to_owned(),
        currentness_class: CURRENTNESS_CLASS.to_owned(),
        action_authority: ACTION_AUTHORITY.to_owned(),
        archive_readback_verified: true,
        canonical_source_written: false,
    })
}

fn extract_codex_rollout(
    source: &Path,
    include_assistant: bool,
) -> Result<(SourceArchiveFileManifest, Vec<SourceArchiveDocument>)> {
    require_regular_file(source)?;
    let before = fs::read(source).map_err(storage_error("read source rollout", source))?;
    let source_sha256 = sha256_bytes(&before);
    let source_bytes = u64::try_from(before.len()).map_err(|_| {
        WorkVcsError::QueryInvalid("source rollout size does not fit u64".to_owned())
    })?;
    let mut parsed = Vec::new();
    let mut source_thread_id = None;
    for (offset, line) in before.split(|byte| *byte == b'\n').enumerate() {
        if line.is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_slice(line).map_err(|error| {
            WorkVcsError::QueryInvalid(format!(
                "source rollout {} line {} is invalid JSON: {error}",
                source.display(),
                offset + 1
            ))
        })?;
        if value.get("type").and_then(serde_json::Value::as_str) == Some("session_meta")
            && let Some(id) = value
                .get("payload")
                .and_then(|payload| payload.get("id"))
                .and_then(serde_json::Value::as_str)
        {
            source_thread_id.get_or_insert_with(|| id.to_owned());
        }
        parsed.push((offset + 1, value));
    }
    let source_thread_id = source_thread_id.ok_or_else(|| {
        WorkVcsError::QueryInvalid(format!(
            "source rollout {} has no session_meta thread identity",
            source.display()
        ))
    })?;
    let mut ignored_roles = BTreeMap::new();
    let mut ignored_content_blocks = BTreeMap::new();
    let mut documents = Vec::new();
    for (source_line, value) in parsed {
        if value.get("type").and_then(serde_json::Value::as_str) != Some("response_item") {
            continue;
        }
        let Some(payload) = value.get("payload") else {
            continue;
        };
        if payload.get("type").and_then(serde_json::Value::as_str) != Some("message") {
            continue;
        }
        let role = payload
            .get("role")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        if role != "user" && !(include_assistant && role == "assistant") {
            *ignored_roles.entry(role.to_owned()).or_default() += 1;
            continue;
        }
        let message_id = payload
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                WorkVcsError::QueryInvalid(format!(
                    "source rollout {} line {} message has no stable id",
                    source.display(),
                    source_line
                ))
            })?;
        let mut text_parts = Vec::new();
        if let Some(blocks) = payload.get("content").and_then(serde_json::Value::as_array) {
            for block in blocks {
                let block_type = block
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown");
                if matches!(block_type, "input_text" | "output_text") {
                    if let Some(text) = block.get("text").and_then(serde_json::Value::as_str)
                        && !text.is_empty()
                    {
                        text_parts.push(text);
                    }
                } else {
                    *ignored_content_blocks
                        .entry(block_type.to_owned())
                        .or_default() += 1;
                }
            }
        }
        if text_parts.is_empty() {
            continue;
        }
        let text = text_parts.join("\n");
        let text_sha256 = sha256_bytes(text.as_bytes());
        let document_id =
            source_document_id(&source_sha256, source_line, message_id, role, &text_sha256);
        documents.push(SourceArchiveDocument {
            document_id,
            source_path: source.to_path_buf(),
            source_sha256: source_sha256.clone(),
            source_line,
            source_thread_id: source_thread_id.clone(),
            source_message_id: message_id.to_owned(),
            source_timestamp: value
                .get("timestamp")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            source_role: role.to_owned(),
            text,
            text_sha256,
            authority_class: AUTHORITY_CLASS.to_owned(),
            currentness_class: CURRENTNESS_CLASS.to_owned(),
            action_authority: ACTION_AUTHORITY.to_owned(),
        });
    }
    let after = fs::read(source).map_err(storage_error("re-read source rollout", source))?;
    if sha256_bytes(&after) != source_sha256 {
        return Err(WorkVcsError::StorageFailure(format!(
            "source rollout {} changed during import",
            source.display()
        )));
    }
    Ok((
        SourceArchiveFileManifest {
            source_path: source.to_path_buf(),
            source_sha256,
            source_bytes,
            imported_messages: documents.len(),
            ignored_roles,
            ignored_content_blocks,
        },
        documents,
    ))
}

fn validate_archive(
    path: &Path,
    expected_identity: Option<SearchSourceIdentity>,
) -> Result<(SourceArchiveManifest, Vec<SourceArchiveDocument>)> {
    require_real_directory(path)?;
    require_private_archive_permissions(path)?;
    let members = fs::read_dir(path)
        .map_err(storage_error("scan source archive", path))?
        .map(|entry| {
            entry
                .map_err(|error| WorkVcsError::StorageFailure(error.to_string()))
                .and_then(|entry| {
                    let metadata = fs::symlink_metadata(entry.path()).map_err(storage_error(
                        "inspect source archive member",
                        &entry.path(),
                    ))?;
                    if metadata.file_type().is_symlink() || !metadata.is_file() {
                        return Err(WorkVcsError::StorageFailure(format!(
                            "source archive member {} is not a regular file",
                            entry.path().display()
                        )));
                    }
                    entry.file_name().into_string().map_err(|_| {
                        WorkVcsError::StorageFailure(
                            "source archive member name is not UTF-8".to_owned(),
                        )
                    })
                })
        })
        .collect::<Result<BTreeSet<_>>>()?;
    if members != BTreeSet::from([MANIFEST_FILE.to_owned(), DOCUMENTS_FILE.to_owned()]) {
        return Err(WorkVcsError::StorageFailure(format!(
            "source archive {} contains missing or unexpected members",
            path.display()
        )));
    }
    let manifest_bytes = fs::read(path.join(MANIFEST_FILE)).map_err(storage_error(
        "read source archive manifest",
        &path.join(MANIFEST_FILE),
    ))?;
    let manifest: SourceArchiveManifest =
        serde_json::from_slice(&manifest_bytes).map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "source archive manifest {} is invalid: {error}",
                path.display()
            ))
        })?;
    if manifest.format_version != ARCHIVE_FORMAT_VERSION
        || manifest.source_format != SourceArchiveFormat::CodexRolloutJsonl
        || manifest.extraction_policy_version != EXTRACTION_POLICY_VERSION
        || manifest.authority_class != AUTHORITY_CLASS
        || manifest.currentness_class != CURRENTNESS_CLASS
        || manifest.action_authority != ACTION_AUTHORITY
        || expected_identity.is_some_and(|identity| identity != manifest.identity)
        || manifest.manifest_sha256 != manifest_sha256(&manifest)?
        || manifest.basis_sha256
            != sha256_json(&ArchiveBasis {
                format_version: manifest.format_version,
                source_format: manifest.source_format,
                extraction_policy_version: &manifest.extraction_policy_version,
                identity: manifest.identity,
                include_assistant: manifest.include_assistant,
                authority_class: &manifest.authority_class,
                currentness_class: &manifest.currentness_class,
                action_authority: &manifest.action_authority,
                files: &manifest.files,
            })?
    {
        return Err(WorkVcsError::StorageFailure(format!(
            "source archive {} manifest identity, policy, or digest mismatch",
            path.display()
        )));
    }
    let document_bytes = fs::read(path.join(DOCUMENTS_FILE)).map_err(storage_error(
        "read source archive documents",
        &path.join(DOCUMENTS_FILE),
    ))?;
    if sha256_bytes(&document_bytes) != manifest.documents_sha256 {
        return Err(WorkVcsError::StorageFailure(format!(
            "source archive {} document digest mismatch",
            path.display()
        )));
    }
    let documents: Vec<SourceArchiveDocument> =
        serde_json::from_slice(&document_bytes).map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "source archive documents {} are invalid: {error}",
                path.display()
            ))
        })?;
    if documents.len() != manifest.document_count {
        return Err(WorkVcsError::StorageFailure(
            "source archive document count mismatch".to_owned(),
        ));
    }
    require_unique_documents(&documents)?;
    if manifest.files.is_empty() {
        return Err(WorkVcsError::StorageFailure(
            "source archive manifest contains no source files".to_owned(),
        ));
    }
    let files = manifest
        .files
        .iter()
        .map(|file| (file.source_path.as_path(), file))
        .collect::<BTreeMap<_, _>>();
    if files.len() != manifest.files.len() {
        return Err(WorkVcsError::StorageFailure(
            "source archive manifest contains duplicate source paths".to_owned(),
        ));
    }
    let mut observed_file_counts = BTreeMap::<&Path, usize>::new();
    for document in &documents {
        let Some(file) = files.get(document.source_path.as_path()) else {
            return Err(WorkVcsError::StorageFailure(format!(
                "source archive document {} names an undeclared source file",
                document.document_id
            )));
        };
        *observed_file_counts
            .entry(document.source_path.as_path())
            .or_default() += 1;
        if document.source_sha256 != file.source_sha256
            || document.text.is_empty()
            || sha256_bytes(document.text.as_bytes()) != document.text_sha256
            || document.document_id
                != source_document_id(
                    &document.source_sha256,
                    document.source_line,
                    &document.source_message_id,
                    &document.source_role,
                    &document.text_sha256,
                )
            || document.source_line == 0
            || document.source_thread_id.is_empty()
            || document.source_message_id.is_empty()
            || document.authority_class != AUTHORITY_CLASS
            || document.currentness_class != CURRENTNESS_CLASS
            || document.action_authority != ACTION_AUTHORITY
            || !matches!(document.source_role.as_str(), "user" | "assistant")
        {
            return Err(WorkVcsError::StorageFailure(format!(
                "source archive document {} failed provenance or authority readback",
                document.document_id
            )));
        }
    }
    for file in &manifest.files {
        require_absolute_clean_path(&file.source_path, "archived source path")?;
        let observed = observed_file_counts
            .get(file.source_path.as_path())
            .copied()
            .unwrap_or_default();
        if observed != file.imported_messages {
            return Err(WorkVcsError::StorageFailure(format!(
                "source archive file {} message count mismatch",
                file.source_path.display()
            )));
        }
    }
    Ok((manifest, documents))
}

fn collect_sources(inputs: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut paths = BTreeSet::new();
    for input in inputs {
        require_absolute_clean_path(input, "source path")?;
        let metadata =
            fs::symlink_metadata(input).map_err(storage_error("inspect explicit source", input))?;
        if metadata.file_type().is_symlink() {
            return Err(WorkVcsError::QueryInvalid(format!(
                "explicit source {} is a symlink",
                input.display()
            )));
        }
        let canonical = fs::canonicalize(input)
            .map_err(storage_error("canonicalize explicit source", input))?;
        if metadata.is_file() {
            require_jsonl(&canonical)?;
            paths.insert(canonical);
        } else if metadata.is_dir() {
            collect_directory(&canonical, &mut paths)?;
        } else {
            return Err(WorkVcsError::QueryInvalid(format!(
                "source {} is neither a regular file nor directory",
                input.display()
            )));
        }
    }
    if paths.is_empty() {
        return Err(WorkVcsError::QueryInvalid(
            "source archive import found no .jsonl files".to_owned(),
        ));
    }
    Ok(paths.into_iter().collect())
}

fn collect_directory(directory: &Path, paths: &mut BTreeSet<PathBuf>) -> Result<()> {
    let mut entries = fs::read_dir(directory)
        .map_err(storage_error("scan source directory", directory))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|error| WorkVcsError::StorageFailure(error.to_string()))?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(storage_error("inspect source directory member", &path))?;
        if metadata.file_type().is_symlink() {
            return Err(WorkVcsError::QueryInvalid(format!(
                "source tree contains symlink {}",
                path.display()
            )));
        }
        if metadata.is_dir() {
            collect_directory(&path, paths)?;
        } else if metadata.is_file() && path.extension().is_some_and(|value| value == "jsonl") {
            paths.insert(path);
        }
    }
    Ok(())
}

fn prepare_archive_root(root: &Path, sources: &[PathBuf]) -> Result<PathBuf> {
    require_absolute_clean_path(root, "source archive root")?;
    let parent = root.parent().ok_or_else(|| {
        WorkVcsError::QueryInvalid("source archive root has no parent".to_owned())
    })?;
    fs::create_dir_all(parent).map_err(storage_error("create source archive parent", parent))?;
    let parent = fs::canonicalize(parent)
        .map_err(storage_error("canonicalize source archive parent", parent))?;
    let root = parent.join(root.file_name().ok_or_else(|| {
        WorkVcsError::QueryInvalid("source archive root has no final name".to_owned())
    })?);
    for source in sources {
        if root == *source || root.starts_with(source) || source.starts_with(&root) {
            return Err(WorkVcsError::QueryInvalid(
                "source archive root must not contain or be contained by an input source"
                    .to_owned(),
            ));
        }
    }
    fs::create_dir_all(&root).map_err(storage_error("create source archive root", &root))?;
    require_real_directory(&root)?;
    Ok(root)
}

fn require_absolute_clean_path(path: &Path, label: &str) -> Result<()> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} must be absolute without dot or parent components"
        )));
    }
    Ok(())
}

fn require_regular_file(path: &Path) -> Result<()> {
    let metadata =
        fs::symlink_metadata(path).map_err(storage_error("inspect source file", path))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(WorkVcsError::QueryInvalid(format!(
            "source {} must be a regular file",
            path.display()
        )));
    }
    Ok(())
}

fn require_real_directory(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .map_err(storage_error("inspect source archive directory", path))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(WorkVcsError::QueryInvalid(format!(
            "source archive {} must be a real directory",
            path.display()
        )));
    }
    Ok(())
}

fn require_jsonl(path: &Path) -> Result<()> {
    if path.extension().is_none_or(|value| value != "jsonl") {
        return Err(WorkVcsError::QueryInvalid(format!(
            "Codex rollout source {} must have a .jsonl extension",
            path.display()
        )));
    }
    Ok(())
}

fn require_unique_documents(documents: &[SourceArchiveDocument]) -> Result<()> {
    let unique = documents
        .iter()
        .map(|document| document.document_id.as_str())
        .collect::<BTreeSet<_>>();
    if unique.len() != documents.len() {
        return Err(WorkVcsError::StorageFailure(
            "source archive contains duplicate document identities".to_owned(),
        ));
    }
    Ok(())
}

fn observe_source_file(file: &SourceArchiveFileManifest) -> SourceArchiveFileState {
    match fs::read(&file.source_path) {
        Ok(bytes) if sha256_bytes(&bytes) == file.source_sha256 => {
            SourceArchiveFileState::Unchanged
        }
        Ok(_) => SourceArchiveFileState::Changed,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            SourceArchiveFileState::Missing
        }
        Err(_) => SourceArchiveFileState::Unreadable,
    }
}

fn bm25_score(
    matched_terms: &[String],
    frequencies: &HashMap<String, usize>,
    document_frequency: &HashMap<String, usize>,
    document_count: usize,
    document_length: usize,
    average_length: f64,
) -> f64 {
    const K1: f64 = 1.2;
    const B: f64 = 0.75;
    matched_terms
        .iter()
        .map(|term| {
            let tf = *frequencies.get(term).unwrap_or(&0) as f64;
            let df = *document_frequency.get(term).unwrap_or(&0) as f64;
            let count = document_count as f64;
            let idf = ((count - df + 0.5) / (df + 0.5) + 1.0).ln();
            let length_ratio = if average_length > 0.0 {
                document_length as f64 / average_length
            } else {
                0.0
            };
            idf * (tf * (K1 + 1.0)) / (tf + K1 * (1.0 - B + B * length_ratio))
        })
        .sum()
}

fn serialized_tokens(item: &SourceArchiveSearchItem) -> Result<usize> {
    let bytes = serde_json::to_vec(item).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!(
            "cannot encode source archive search item: {error}"
        ))
    })?;
    let text = std::str::from_utf8(&bytes).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!(
            "source archive search item JSON is not UTF-8: {error}"
        ))
    })?;
    Ok(search_o200k_ordinary_token_count(text))
}

fn manifest_sha256(manifest: &SourceArchiveManifest) -> Result<String> {
    let mut value = serde_json::to_value(manifest).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!(
            "cannot encode source archive manifest for digest: {error}"
        ))
    })?;
    let object = value.as_object_mut().ok_or_else(|| {
        WorkVcsError::CanonicalEncodingInvalid(
            "source archive manifest did not encode as an object".to_owned(),
        )
    })?;
    object.remove("manifest_sha256");
    sha256_json(&value)
}

fn sha256_json(value: &impl Serialize) -> Result<String> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!(
            "cannot encode source archive digest input: {error}"
        ))
    })?;
    Ok(sha256_bytes(&bytes))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn source_document_id(
    source_sha256: &str,
    source_line: usize,
    message_id: &str,
    role: &str,
    text_sha256: &str,
) -> String {
    sha256_bytes(
        format!(
            "workvcs-source-document-v1\0{source_sha256}\0{source_line}\0{message_id}\0{role}\0{text_sha256}"
        )
        .as_bytes(),
    )
}

fn write_json_new(path: &Path, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!(
            "cannot encode source archive JSON: {error}"
        ))
    })?;
    write_new(path, &bytes)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(path)
        .map_err(storage_error("create source archive file", path))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(storage_error("write source archive file", path))
}

fn create_private_directory(path: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    builder.mode(0o700);
    builder.create(path).map_err(storage_error(
        "create private source archive directory",
        path,
    ))
}

#[cfg(unix)]
fn require_private_archive_permissions(path: &Path) -> Result<()> {
    let directory_mode = fs::symlink_metadata(path)
        .map_err(storage_error("inspect source archive permissions", path))?
        .permissions()
        .mode();
    if directory_mode & 0o077 != 0 {
        return Err(WorkVcsError::StorageFailure(format!(
            "source archive {} exposes group or other permissions",
            path.display()
        )));
    }
    for name in [MANIFEST_FILE, DOCUMENTS_FILE] {
        let member = path.join(name);
        let mode = fs::symlink_metadata(&member)
            .map_err(storage_error(
                "inspect source archive file permissions",
                &member,
            ))?
            .permissions()
            .mode();
        if mode & 0o077 != 0 {
            return Err(WorkVcsError::StorageFailure(format!(
                "source archive file {} exposes group or other permissions",
                member.display()
            )));
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn require_private_archive_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

fn sync_directory(path: &Path) -> Result<()> {
    let directory = fs::File::open(path).map_err(storage_error("open source archive", path))?;
    directory
        .sync_all()
        .map_err(storage_error("sync source archive", path))
}

fn storage_error<'a>(
    action: &'static str,
    path: &'a Path,
) -> impl FnOnce(std::io::Error) -> WorkVcsError + 'a {
    move |error| {
        WorkVcsError::StorageFailure(format!("cannot {action} {}: {error}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> SearchSourceIdentity {
        SearchSourceIdentity::new(
            crate::identity::ProjectRefId::new_v7(),
            crate::identity::StoreId::new_v7(),
            crate::identity::WorkspaceId::new_v7(),
            crate::identity::BranchId::new_v7(),
        )
    }

    fn write_rollout(path: &Path) {
        let thread = uuid::Uuid::now_v7();
        let user = uuid::Uuid::now_v7();
        let assistant = uuid::Uuid::now_v7();
        let lines = [
            serde_json::json!({"type":"session_meta","timestamp":"2026-01-01T00:00:00Z","payload":{"id":thread}}),
            serde_json::json!({"type":"response_item","timestamp":"2026-01-01T00:00:01Z","payload":{"type":"message","role":"developer","id":uuid::Uuid::now_v7(),"content":[{"type":"input_text","text":"secret developer contract"}]}}),
            serde_json::json!({"type":"response_item","timestamp":"2026-01-01T00:00:02Z","secret":"must-not-copy","payload":{"type":"message","role":"user","id":user,"content":[{"type":"input_text","text":"如何恢复历史检索证据"},{"type":"image_url","url":"https://example.invalid/token"}]}}),
            serde_json::json!({"type":"response_item","timestamp":"2026-01-01T00:00:03Z","payload":{"type":"message","role":"assistant","id":assistant,"content":[{"type":"output_text","text":"使用有出处的历史证据"}]}}),
        ];
        let bytes = lines
            .into_iter()
            .map(|line| serde_json::to_string(&line).expect("JSON"))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(path, format!("{bytes}\n")).expect("write rollout");
    }

    #[test]
    fn import_is_repeatable_bounded_and_reports_source_drift() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let source = temporary.path().join("rollout.jsonl");
        let root = temporary.path().join("archives");
        write_rollout(&source);
        let source = fs::canonicalize(source).expect("canonical source");
        let identity = identity();
        let options = SourceArchiveImportOptions::new(identity, &root, vec![source.clone()])
            .expect("import options");
        let imported = import_source_archive(&options).expect("source import");
        assert_eq!(imported.outcome, SourceArchiveImportOutcome::Imported);
        assert_eq!(imported.manifest.document_count, 2);
        assert!(!imported.canonical_source_written);
        assert!(!imported.raw_source_copied);
        #[cfg(unix)]
        {
            assert_eq!(
                fs::metadata(&imported.archive_path)
                    .expect("archive metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
            for name in [MANIFEST_FILE, DOCUMENTS_FILE] {
                assert_eq!(
                    fs::metadata(imported.archive_path.join(name))
                        .expect("archive member metadata")
                        .permissions()
                        .mode()
                        & 0o777,
                    0o600
                );
            }
        }
        let reused = import_source_archive(&options).expect("repeat source import");
        assert_eq!(reused.outcome, SourceArchiveImportOutcome::Reused);
        assert_eq!(reused.archive_path, imported.archive_path);
        let inspected = inspect_source_archive(&SourceArchiveInspectOptions::new(
            identity,
            &imported.archive_path,
        ))
        .expect("inspect immutable archive");
        assert!(inspected.archive_readback_verified);
        assert!(!inspected.canonical_source_written);
        assert_eq!(inspected.manifest, imported.manifest);
        assert_eq!(inspected.file_observations.len(), 1);
        assert_eq!(
            inspected.file_observations[0].state,
            SourceArchiveFileState::Unchanged
        );

        let query = query_source_archive(
            &SourceArchiveQueryOptions::new(identity, &imported.archive_path, "历史检索证据")
                .expect("query options"),
        )
        .expect("source query");
        assert_eq!(query.candidates_considered, 2);
        assert_eq!(query.matched_documents, 2);
        assert!(query.items.iter().all(|item| {
            item.archive_readback_verified
                && item.document.authority_class == AUTHORITY_CLASS
                && item.document.action_authority == ACTION_AUTHORITY
                && !item.document.text.contains("secret developer contract")
                && !item.document.text.contains("must-not-copy")
                && !item.document.text.contains("example.invalid")
        }));
        fs::write(&source, "changed\n").expect("change source");
        let drifted = query_source_archive(
            &SourceArchiveQueryOptions::new(identity, &imported.archive_path, "历史")
                .expect("query options"),
        )
        .expect("query preserved archive");
        assert!(
            drifted
                .items
                .iter()
                .all(|item| item.source_file_state == SourceArchiveFileState::Changed)
        );
        fs::remove_file(&source).expect("remove source");
        let missing = query_source_archive(
            &SourceArchiveQueryOptions::new(identity, &imported.archive_path, "历史")
                .expect("query options"),
        )
        .expect("query preserved archive");
        assert!(
            missing
                .items
                .iter()
                .all(|item| item.source_file_state == SourceArchiveFileState::Missing)
        );
    }

    #[test]
    fn archive_rejects_wrong_identity_corruption_and_symlink_sources() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let source = temporary.path().join("rollout.jsonl");
        let root = temporary.path().join("archives");
        write_rollout(&source);
        let source = fs::canonicalize(source).expect("canonical source");
        let imported = import_source_archive(
            &SourceArchiveImportOptions::new(identity(), &root, vec![source.clone()])
                .expect("import options"),
        )
        .expect("source import");
        assert!(
            SourceArchiveQueryOptions::new(identity(), &imported.archive_path, "历史")
                .and_then(|options| query_source_archive(&options))
                .is_err(),
            "an archive cannot be queried under another verified identity"
        );
        #[cfg(unix)]
        {
            let documents_path = imported.archive_path.join(DOCUMENTS_FILE);
            fs::set_permissions(&documents_path, fs::Permissions::from_mode(0o644))
                .expect("expose archive permissions");
            assert!(
                validate_archive(&imported.archive_path, None).is_err(),
                "an archive with group or other access must fail closed"
            );
            fs::set_permissions(&documents_path, fs::Permissions::from_mode(0o600))
                .expect("restore archive permissions");
        }

        let documents_path = imported.archive_path.join(DOCUMENTS_FILE);
        let mut documents: Vec<SourceArchiveDocument> =
            serde_json::from_slice(&fs::read(&documents_path).expect("read documents"))
                .expect("decode documents");
        documents[0].document_id = "0".repeat(64);
        let document_bytes = serde_json::to_vec_pretty(&documents).expect("encode documents");
        fs::write(&documents_path, &document_bytes).expect("write forged documents");
        let manifest_path = imported.archive_path.join(MANIFEST_FILE);
        let mut manifest: SourceArchiveManifest =
            serde_json::from_slice(&fs::read(&manifest_path).expect("read manifest"))
                .expect("decode manifest");
        manifest.documents_sha256 = sha256_bytes(&document_bytes);
        manifest.manifest_sha256 = manifest_sha256(&manifest).expect("manifest digest");
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).expect("encode manifest"),
        )
        .expect("write forged manifest");
        assert!(
            validate_archive(&imported.archive_path, None).is_err(),
            "digest-consistent forged document identity must fail closed"
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let target = temporary.path().join("target.jsonl");
            let link = temporary.path().join("link.jsonl");
            write_rollout(&target);
            symlink(&target, &link).expect("source symlink");
            assert!(
                SourceArchiveImportOptions::new(identity(), &root, vec![link.clone()])
                    .and_then(|options| import_source_archive(&options))
                    .is_err(),
                "an explicit symlink source must fail closed"
            );
        }
    }

    #[test]
    fn import_recovers_from_failed_input_and_serializes_concurrent_publish() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let source = temporary.path().join("rollout.jsonl");
        let root = temporary.path().join("archives");
        fs::write(&source, "not-json\n").expect("write invalid rollout");
        let source = fs::canonicalize(source).expect("canonical source");
        let identity = identity();
        let failed = SourceArchiveImportOptions::new(identity, &root, vec![source.clone()])
            .and_then(|options| import_source_archive(&options));
        assert!(failed.is_err(), "invalid input must fail closed");

        write_rollout(&source);
        fs::create_dir_all(&root).expect("create archive root");
        fs::create_dir(root.join(".workvcs-source-orphan.tmp"))
            .expect("create orphan stage from an interrupted prior import");
        let options =
            SourceArchiveImportOptions::new(identity, &root, vec![source]).expect("import options");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let handles = (0..2)
            .map(|_| {
                let options = options.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    import_source_archive(&options).expect("concurrent source import")
                })
            })
            .collect::<Vec<_>>();
        let outcomes = handles
            .into_iter()
            .map(|handle| handle.join().expect("join source import").outcome)
            .collect::<Vec<_>>();
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| **outcome == SourceArchiveImportOutcome::Imported)
                .count(),
            1
        );
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| **outcome == SourceArchiveImportOutcome::Reused)
                .count(),
            1
        );
    }
}
