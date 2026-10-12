use super::model::{load_embedding_model, load_reranker_model};
use super::profile::{
    BGE_MAX_TOKENS, E5_MAX_TOKENS, E5_WINDOW_OVERLAP, RRF_K, SEMANTIC_GENERATION_FORMAT_VERSION,
    SEMANTIC_RANKING_POLICY_VERSION, SEMANTIC_WINDOW_POLICY_VERSION, VECTOR_NAME,
};
use super::window::{load_tokenizer, model_windows, pair_token_count};
use super::{
    SemanticDerivedStatus, SemanticGenerationManifest, SemanticModelProfile,
    SemanticPassageCatalogEntry, SemanticPassageScore, SemanticQueryOptions, SemanticQueryResult,
    SemanticRankingMode, SemanticResourceObservations, SemanticSearchItem,
};
use crate::error::{Result, WorkVcsError};
use crate::identity::{Digest, EntityId};
use crate::search::{SearchQueryOptions, SearchRefreshOutcome, SourceDocument, SourceSnapshot};
use crate::store::StoreConnection;
use fastembed::{TextEmbedding, TextRerank};
use qdrant_edge::{
    Distance, EdgeConfigBuilder, EdgeOptimizersConfig, EdgeShard, EdgeShardRead,
    EdgeVectorParamsBuilder, NamedQuery, PointId, PointInsertOperations, PointOperations,
    PointStructPersisted, QueryEnum, QueryRequest, QueryRequestBuilder, ReadOnlyEdgeShard,
    ScoringQuery, SearchParams, SegmentManifestState, SegmentsManifest, UpdateOperation,
    VectorInternal, VectorStructInternal, VectorStructPersisted,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::Instant;

const GENERATION_MANIFEST_FILE: &str = "manifest.json";
const CATALOG_FILE: &str = "catalog.json";
const GENERATION_LOCK_FILE: &str = "generation.lock";
const QDRANT_DIRECTORY: &str = "qdrant";
const QDRANT_SEGMENT_MANIFEST: &str = "segments_manifest.json";
const EMBEDDING_BATCH_SIZE: usize = 8;
const RERANKER_BATCH_SIZE: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ObjectKey {
    source_type: String,
    entity_id: EntityId,
}

impl ObjectKey {
    fn from_document(document: &SourceDocument) -> Self {
        Self {
            source_type: document.source_type.as_str().to_owned(),
            entity_id: document.entity_id,
        }
    }

    fn from_catalog(entry: &SemanticPassageCatalogEntry) -> Self {
        Self {
            source_type: entry.source_type.as_str().to_owned(),
            entity_id: entry.entity_id,
        }
    }
}

#[derive(Serialize)]
struct GenerationBasis<'a> {
    format_version: u32,
    ranking_policy_version: &'a str,
    window_policy_version: &'a str,
    identity: crate::search::SearchSourceIdentity,
    temporal_scope: crate::search::SearchTemporalScope,
    source_commit_id: crate::identity::CommitId,
    source_state_digest: Digest,
    object_version_basis_digest: Digest,
    object_count: usize,
    embedding_model_manifest_digest: Digest,
    embedding_profile: SemanticModelProfile,
    dimension: usize,
    distance: &'a str,
    vector_name: &'a str,
    exact_search: bool,
    request_limit_rule: &'a str,
    quantization: &'a str,
    qdrant_edge_version: &'a str,
    fastembed_version: &'a str,
    ort_version: &'a str,
}

struct GenerationLock {
    file: File,
}

impl GenerationLock {
    fn create(directory: &Path) -> Result<Self> {
        let path = directory.join(GENERATION_LOCK_FILE);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| {
                WorkVcsError::StorageFailure(format!(
                    "cannot create semantic generation lock {}: {error}",
                    path.display()
                ))
            })?;
        Self::lock(file, &path)
    }

    fn acquire(directory: &Path) -> Result<Self> {
        let path = directory.join(GENERATION_LOCK_FILE);
        let file = OpenOptions::new().read(true).open(&path).map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "cannot open semantic generation lock {}: {error}",
                path.display()
            ))
        })?;
        Self::lock(file, &path)
    }

    fn lock(file: File, path: &Path) -> Result<Self> {
        fs4::FileExt::try_lock(&file).map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "semantic generation {} is already open or cannot be exclusively locked: {error}",
                path.display()
            ))
        })?;
        Ok(Self { file })
    }
}

impl Drop for GenerationLock {
    fn drop(&mut self) {
        let _ = fs4::FileExt::unlock(&self.file);
    }
}

struct ResourceObserver {
    system: sysinfo::System,
    pid: Option<sysinfo::Pid>,
    start: u64,
    max: u64,
}

impl ResourceObserver {
    fn new() -> Self {
        let mut observer = Self {
            system: sysinfo::System::new_all(),
            pid: sysinfo::get_current_pid().ok(),
            start: 0,
            max: 0,
        };
        observer.start = observer.sample();
        observer.max = observer.start;
        observer
    }

    fn sample(&mut self) -> u64 {
        if let Some(pid) = self.pid {
            self.system
                .refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
            let value = self
                .system
                .process(pid)
                .map(sysinfo::Process::memory)
                .unwrap_or_default();
            self.max = self.max.max(value);
            value
        } else {
            0
        }
    }
}

pub(super) fn query_semantic(
    source: &StoreConnection,
    source_path: &Path,
    options: &SemanticQueryOptions,
) -> Result<SemanticQueryResult> {
    let started = Instant::now();
    let mut resource = ResourceObserver::new();
    validate_query_options(options)?;

    let (embedding_manifest, mut embedding) = load_embedding_model(&options.embedding_model_path)?;
    if embedding_manifest.profile != options.embedding_profile {
        return Err(WorkVcsError::QueryInvalid(format!(
            "selected embedding profile {} does not match provisioned model {}",
            options.embedding_profile, embedding_manifest.profile
        )));
    }
    let embedding_tokenizer = load_tokenizer(&options.embedding_model_path)?;
    let mut reranker = load_selected_reranker(options)?;
    let reranker_tokenizer = if options.ranking_mode.needs_reranker() {
        Some(load_tokenizer(
            options
                .reranker_model_path
                .as_ref()
                .expect("reranker path validated"),
        )?)
    } else {
        None
    };
    resource.sample();

    let snapshot = super::super::source_snapshot(source, options.identity, options.commit_id)?;
    let basis_digest = generation_basis_digest(&snapshot, embedding_manifest.manifest_digest)?;
    let generation_root = prepare_generation_root(
        &options.generation_root,
        source_path,
        &options.protected_paths,
        &options.embedding_model_path,
        options.reranker_model_path.as_deref(),
    )?;
    let generation_path = generation_root.join(basis_digest.to_hex());
    let publication_lock = super::super::SearchPublicationLock::acquire(&generation_path)?;
    let maintenance = if generation_path.exists() {
        SearchRefreshOutcome::Reused
    } else {
        build_generation(
            source,
            &snapshot,
            &generation_path,
            basis_digest,
            &embedding_manifest,
            &embedding_tokenizer,
            &mut embedding,
        )?;
        SearchRefreshOutcome::Refreshed
    };
    drop(publication_lock);
    resource.sample();

    let _generation_lock = GenerationLock::acquire(&generation_path)?;
    let (generation_manifest, catalog) = validate_generation(
        &generation_path,
        &snapshot,
        basis_digest,
        embedding_manifest.manifest_digest,
    )?;
    let query_windows = model_windows(
        &embedding_tokenizer,
        &options.query,
        "query: ",
        E5_MAX_TOKENS,
        E5_WINDOW_OVERLAP,
    )?;
    let query_model_tokens = query_windows.iter().map(|window| window.model_tokens).sum();
    let passage_model_tokens = catalog.iter().map(|entry| entry.model_tokens).sum();

    let vector_scores = exact_vector_scores(
        &generation_path,
        &catalog,
        &query_windows
            .iter()
            .map(|window| window.prefixed_text.clone())
            .collect::<Vec<_>>(),
        &mut embedding,
    )?;
    resource.sample();

    let (reranker_scores, reranker_pair_tokens, reranker_pair_count) =
        match (reranker.as_mut(), reranker_tokenizer.as_ref()) {
            (Some(reranker), Some(tokenizer)) => full_corpus_reranker_scores(
                &query_windows
                    .iter()
                    .map(|window| window.text.clone())
                    .collect::<Vec<_>>(),
                &catalog,
                tokenizer,
                reranker,
            )?,
            _ => (BTreeMap::new(), 0, 0),
        };
    resource.sample();

    let lexical_ranks = if options.ranking_mode.needs_lexical() {
        complete_lexical_ranks(source, source_path, options, &snapshot)?
    } else {
        BTreeMap::new()
    };

    let mut items = rank_and_materialize(
        &snapshot,
        &catalog,
        &vector_scores,
        &reranker_scores,
        &lexical_ranks,
        options.ranking_mode,
    )?;
    let matched_objects = items.len();
    let omitted_results = options
        .result_limit
        .map(|limit| items.len().saturating_sub(limit))
        .unwrap_or_default();
    if let Some(limit) = options.result_limit {
        items.truncate(limit);
    }
    let serialized_item_tokens = items.iter().map(|item| item.serialized_tokens).sum();

    if !super::super::source_still_matches_snapshot(source, &snapshot)? {
        let end = resource.sample();
        return Ok(SemanticQueryResult {
            status: SemanticDerivedStatus::RefreshRequired,
            reason_code: Some("source_changed".to_owned()),
            reason: Some(format!(
                "source branch {} changed after semantic snapshot {}",
                snapshot.identity.branch_id, snapshot.source_commit_id
            )),
            guidance: Some(
                "repeat the semantic query to build or reuse the generation for the new native head"
                    .to_owned(),
            ),
            ranking_mode: options.ranking_mode,
            query: options.query.clone(),
            source_identity: snapshot.identity,
            temporal_scope: snapshot.temporal_scope,
            source_commit_id: snapshot.source_commit_id,
            source_state_digest: snapshot.source_state_digest,
            generation_path: generation_path.clone(),
            generation_manifest,
            embedding_model_path: options.embedding_model_path.clone(),
            reranker_model_path: options.reranker_model_path.clone(),
            generation_maintenance: maintenance,
            result_limit: options.result_limit,
            candidates_considered: snapshot.documents.len(),
            matched_objects: 0,
            omitted_results: 0,
            serialized_item_tokens: 0,
            items: Vec::new(),
            canonical_readback_verified: false,
            canonical_source_written: false,
            derived_files_written: maintenance == SearchRefreshOutcome::Refreshed,
            observations: observations(
                &mut resource,
                end,
                started,
                query_model_tokens,
                passage_model_tokens,
                reranker_pair_tokens,
                query_windows.len(),
                catalog.len(),
                reranker_pair_count,
                &generation_path,
                options.ranking_mode,
            )?,
        });
    }

    let end = resource.sample();
    Ok(SemanticQueryResult {
        status: SemanticDerivedStatus::Ready,
        reason_code: None,
        reason: None,
        guidance: None,
        ranking_mode: options.ranking_mode,
        query: options.query.clone(),
        source_identity: snapshot.identity,
        temporal_scope: snapshot.temporal_scope,
        source_commit_id: snapshot.source_commit_id,
        source_state_digest: snapshot.source_state_digest,
        generation_path: generation_path.clone(),
        generation_manifest,
        embedding_model_path: options.embedding_model_path.clone(),
        reranker_model_path: options.reranker_model_path.clone(),
        generation_maintenance: maintenance,
        result_limit: options.result_limit,
        candidates_considered: snapshot.documents.len(),
        matched_objects,
        omitted_results,
        serialized_item_tokens,
        items,
        canonical_readback_verified: true,
        canonical_source_written: false,
        derived_files_written: maintenance == SearchRefreshOutcome::Refreshed,
        observations: observations(
            &mut resource,
            end,
            started,
            query_model_tokens,
            passage_model_tokens,
            reranker_pair_tokens,
            query_windows.len(),
            catalog.len(),
            reranker_pair_count,
            &generation_path,
            options.ranking_mode,
        )?,
    })
}

#[allow(clippy::too_many_arguments)]
fn observations(
    resource: &mut ResourceObserver,
    end: u64,
    started: Instant,
    query_model_tokens: usize,
    passage_model_tokens: usize,
    reranker_pair_tokens: usize,
    query_window_count: usize,
    passage_count: usize,
    reranker_pair_count: usize,
    generation_path: &Path,
    mode: SemanticRankingMode,
) -> Result<SemanticResourceObservations> {
    Ok(SemanticResourceObservations {
        elapsed_millis: started.elapsed().as_millis(),
        resident_memory_start_bytes: resource.start,
        resident_memory_end_bytes: end,
        resident_memory_observed_max_bytes: resource.max,
        query_model_tokens,
        passage_model_tokens,
        reranker_pair_tokens,
        query_window_count,
        passage_count,
        reranker_pair_count,
        generation_bytes: directory_bytes(generation_path)?,
        embedding_batch_size: EMBEDDING_BATCH_SIZE,
        reranker_batch_size: mode.needs_reranker().then_some(RERANKER_BATCH_SIZE),
    })
}

fn validate_query_options(options: &SemanticQueryOptions) -> Result<()> {
    if options.embedding_profile != SemanticModelProfile::MultilingualE5LargeFp32V1 {
        return Err(WorkVcsError::QueryInvalid(format!(
            "profile {} cannot provide the required 1024-dimensional E5 generation",
            options.embedding_profile
        )));
    }
    if options.ranking_mode.needs_reranker() {
        if options.reranker_profile != Some(SemanticModelProfile::BgeRerankerV2M3Fp32V1)
            || options.reranker_model_path.is_none()
        {
            return Err(WorkVcsError::QueryInvalid(
                "selected ranking mode requires explicit bge-reranker-v2-m3-fp32-v1 profile and model path"
                    .to_owned(),
            ));
        }
    }
    if options.ranking_mode.needs_lexical() && options.lexical_root.is_none() {
        return Err(WorkVcsError::QueryInvalid(
            "selected hybrid ranking mode requires an explicit lexical index directory".to_owned(),
        ));
    }
    Ok(())
}

fn load_selected_reranker(options: &SemanticQueryOptions) -> Result<Option<TextRerank>> {
    if !options.ranking_mode.needs_reranker() {
        return Ok(None);
    }
    let path = options
        .reranker_model_path
        .as_ref()
        .expect("reranker path validated");
    let (manifest, reranker) = load_reranker_model(path)?;
    if Some(manifest.profile) != options.reranker_profile {
        return Err(WorkVcsError::QueryInvalid(format!(
            "selected reranker profile {:?} does not match provisioned model {}",
            options.reranker_profile, manifest.profile
        )));
    }
    Ok(Some(reranker))
}

fn generation_basis_digest(snapshot: &SourceSnapshot, model_digest: Digest) -> Result<Digest> {
    let basis = GenerationBasis {
        format_version: SEMANTIC_GENERATION_FORMAT_VERSION,
        ranking_policy_version: SEMANTIC_RANKING_POLICY_VERSION,
        window_policy_version: SEMANTIC_WINDOW_POLICY_VERSION,
        identity: snapshot.identity,
        temporal_scope: snapshot.temporal_scope,
        source_commit_id: snapshot.source_commit_id,
        source_state_digest: snapshot.source_state_digest,
        object_version_basis_digest: snapshot.object_version_basis_digest,
        object_count: snapshot.documents.len(),
        embedding_model_manifest_digest: model_digest,
        embedding_profile: SemanticModelProfile::MultilingualE5LargeFp32V1,
        dimension: 1024,
        distance: "cosine",
        vector_name: VECTOR_NAME,
        exact_search: true,
        request_limit_rule: "complete_manifest_point_count",
        quantization: "disabled",
        qdrant_edge_version: "0.8.0",
        fastembed_version: "7.1.1",
        ort_version: "2.0.0-rc.13",
    };
    let bytes = serde_json::to_vec(&basis).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!(
            "cannot encode semantic generation basis: {error}"
        ))
    })?;
    Ok(Digest::domain_separated(
        "workvcs-semantic-generation-basis-v1",
        &bytes,
    ))
}

fn prepare_generation_root(
    root: &Path,
    source_path: &Path,
    protected_paths: &[PathBuf],
    embedding_model_path: &Path,
    reranker_model_path: Option<&Path>,
) -> Result<PathBuf> {
    if !root.is_absolute()
        || root
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(WorkVcsError::QueryInvalid(
            "semantic generation directory must be absolute without dot or parent components"
                .to_owned(),
        ));
    }
    let parent = root.parent().ok_or_else(|| {
        WorkVcsError::QueryInvalid("semantic generation directory has no parent".to_owned())
    })?;
    fs::create_dir_all(parent).map_err(storage_error("create generation parent", parent))?;
    let parent = fs::canonicalize(parent)
        .map_err(storage_error("canonicalize generation parent", parent))?;
    let name = root.file_name().ok_or_else(|| {
        WorkVcsError::QueryInvalid("semantic generation directory has no final name".to_owned())
    })?;
    let root = parent.join(name);
    let source =
        fs::canonicalize(source_path).map_err(storage_error("canonicalize Store", source_path))?;
    let embedding = fs::canonicalize(embedding_model_path).map_err(storage_error(
        "canonicalize embedding model",
        embedding_model_path,
    ))?;
    if root == source
        || root == embedding
        || root.starts_with(&embedding)
        || embedding.starts_with(&root)
    {
        return Err(WorkVcsError::QueryInvalid(
            "semantic generation directory must not alias the Store or contain the embedding model"
                .to_owned(),
        ));
    }
    if let Some(reranker) = reranker_model_path {
        let reranker = fs::canonicalize(reranker)
            .map_err(storage_error("canonicalize reranker model", reranker))?;
        if root == reranker || root.starts_with(&reranker) || reranker.starts_with(&root) {
            return Err(WorkVcsError::QueryInvalid(
                "semantic generation directory must not contain or be contained by the reranker model"
                    .to_owned(),
            ));
        }
    }
    for protected in protected_paths {
        let protected = fs::canonicalize(protected)
            .map_err(storage_error("canonicalize protected path", protected))?;
        if root == protected {
            return Err(WorkVcsError::QueryInvalid(format!(
                "semantic generation directory aliases protected path {}",
                protected.display()
            )));
        }
    }
    if root.exists() {
        let metadata =
            fs::symlink_metadata(&root).map_err(storage_error("inspect generation root", &root))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(WorkVcsError::QueryInvalid(format!(
                "semantic generation root {} must be a real directory",
                root.display()
            )));
        }
    } else {
        fs::create_dir(&root).map_err(storage_error("create generation root", &root))?;
    }
    Ok(root)
}

fn build_generation(
    source: &StoreConnection,
    snapshot: &SourceSnapshot,
    destination: &Path,
    basis_digest: Digest,
    embedding_manifest: &super::SemanticModelManifest,
    tokenizer: &tokenizers::Tokenizer,
    embedding: &mut TextEmbedding,
) -> Result<()> {
    let root = destination.parent().expect("generation destination parent");
    let stage = root.join(format!(".workvcs-semantic-{}.tmp", uuid::Uuid::now_v7()));
    fs::create_dir(&stage).map_err(storage_error("create generation stage", &stage))?;
    let result = (|| {
        let _lock = GenerationLock::create(&stage)?;
        let catalog = build_catalog(snapshot, tokenizer)?;
        let catalog_bytes = serde_json::to_vec_pretty(&catalog).map_err(|error| {
            WorkVcsError::CanonicalEncodingInvalid(format!(
                "cannot encode semantic passage catalog: {error}"
            ))
        })?;
        let catalog_digest =
            Digest::domain_separated("workvcs-semantic-passage-catalog-v1", &catalog_bytes);
        write_file_new(&stage.join(CATALOG_FILE), &catalog_bytes)?;

        let passage_inputs = catalog
            .iter()
            .map(|entry| format!("passage: {}", entry.text))
            .collect::<Vec<_>>();
        let vectors = if passage_inputs.is_empty() {
            Vec::new()
        } else {
            embedding
                .embed(&passage_inputs, Some(EMBEDDING_BATCH_SIZE))
                .map_err(|error| {
                    WorkVcsError::StorageFailure(format!(
                        "cannot embed complete semantic passage catalog: {error}"
                    ))
                })?
        };
        if vectors.len() != catalog.len()
            || vectors
                .iter()
                .any(|vector| vector.len() != 1024 || vector.iter().any(|value| !value.is_finite()))
        {
            return Err(WorkVcsError::StorageFailure(
                "embedding output does not match the complete 1024-dimensional passage catalog"
                    .to_owned(),
            ));
        }

        let point_id_digest = point_id_digest(catalog.iter().map(|entry| entry.point_id));
        let (qdrant_config, qdrant_search_threads, optimized) = if catalog.is_empty() {
            fs::create_dir(stage.join(QDRANT_DIRECTORY))
                .map_err(storage_error("create empty qdrant directory", &stage))?;
            (
                serde_json::json!({
                    "empty_generation": true,
                    "resolved_search_thread_count": 0,
                }),
                0,
                false,
            )
        } else {
            build_qdrant(&stage.join(QDRANT_DIRECTORY), &catalog, vectors)?
        };

        let manifest = SemanticGenerationManifest {
            format_version: SEMANTIC_GENERATION_FORMAT_VERSION,
            ranking_policy_version: SEMANTIC_RANKING_POLICY_VERSION.to_owned(),
            window_policy_version: SEMANTIC_WINDOW_POLICY_VERSION.to_owned(),
            identity: snapshot.identity,
            temporal_scope: snapshot.temporal_scope,
            source_commit_id: snapshot.source_commit_id,
            source_state_digest: snapshot.source_state_digest,
            object_version_basis_digest: snapshot.object_version_basis_digest,
            object_count: snapshot.documents.len(),
            point_count: catalog.len(),
            embedding_model_manifest_digest: embedding_manifest.manifest_digest,
            embedding_profile: embedding_manifest.profile,
            dimension: 1024,
            distance: "cosine".to_owned(),
            vector_name: VECTOR_NAME.to_owned(),
            exact_search: true,
            request_limit_rule: "complete_manifest_point_count".to_owned(),
            quantization: "disabled".to_owned(),
            qdrant_edge_version: "0.8.0".to_owned(),
            fastembed_version: "7.1.1".to_owned(),
            ort_version: "2.0.0-rc.13".to_owned(),
            rust_target: rust_target(),
            provider: "CPUExecutionProvider".to_owned(),
            qdrant_config,
            qdrant_search_threads,
            optimize_returned: optimized,
            catalog_digest,
            point_id_digest,
            created_at_us: crate::store::current_epoch_micros()?,
            basis_digest,
        };
        write_json_new(&stage.join(GENERATION_MANIFEST_FILE), &manifest)?;
        sync_directory(&stage)?;
        validate_generation(
            &stage,
            snapshot,
            basis_digest,
            embedding_manifest.manifest_digest,
        )?;
        if !super::super::source_still_matches_snapshot(source, snapshot)? {
            return Err(WorkVcsError::BranchHeadConflict(format!(
                "semantic source branch {} changed while building generation for {}",
                snapshot.identity.branch_id, snapshot.source_commit_id
            )));
        }
        super::super::atomic_publish_new_index(&stage, destination)?;
        super::super::sync_parent(destination)?;
        Ok(())
    })();
    if result.is_err() && stage.exists() {
        let _ = fs::remove_dir_all(&stage);
    }
    result
}

fn build_catalog(
    snapshot: &SourceSnapshot,
    tokenizer: &tokenizers::Tokenizer,
) -> Result<Vec<SemanticPassageCatalogEntry>> {
    let mut catalog = Vec::new();
    for document in &snapshot.documents {
        let windows = model_windows(
            tokenizer,
            &document.statement,
            "passage: ",
            E5_MAX_TOKENS,
            E5_WINDOW_OVERLAP,
        )?;
        for window in windows {
            catalog.push(SemanticPassageCatalogEntry {
                point_id: u64::try_from(catalog.len() + 1).map_err(|_| {
                    WorkVcsError::StorageFailure("semantic point ID overflow".to_owned())
                })?,
                source_type: document.source_type,
                entity_id: document.entity_id,
                entity_version_id: document.entity_version_id,
                entity_state_digest: document.state_digest,
                statement_digest: document.statement_digest,
                status: document.status.clone(),
                record_kind: document.record_kind.clone(),
                scope_json: document.scope_json.clone(),
                provenance_json: document.provenance_json.clone(),
                byte_start: window.byte_start,
                byte_end: window.byte_end,
                tokenizer_token_start: window.token_start,
                tokenizer_token_end: window.token_end,
                model_tokens: window.model_tokens,
                text: window.text,
            });
        }
    }
    Ok(catalog)
}

fn build_qdrant(
    path: &Path,
    catalog: &[SemanticPassageCatalogEntry],
    vectors: Vec<Vec<f32>>,
) -> Result<(serde_json::Value, usize, bool)> {
    fs::create_dir(path).map_err(storage_error("create qdrant shard", path))?;
    let optimizers = EdgeOptimizersConfig {
        default_segment_number: Some(1),
        indexing_threshold: Some(1),
        ..EdgeOptimizersConfig::default()
    };
    let config = EdgeConfigBuilder::new()
        .on_disk_payload(true)
        .optimizers(optimizers.clone())
        .vector(
            VECTOR_NAME,
            EdgeVectorParamsBuilder::new(1024, Distance::Cosine)
                .on_disk(true)
                .build(),
        )
        .build();
    let search_threads = config.search_thread_count();
    let config_json = serde_json::json!({
        "declared": config,
        "resolved": {
            "on_disk_payload": true,
            "search_thread_count": search_threads,
            "optimizer_default_segment_number": optimizers.get_number_segments(),
            "optimizer_indexing_threshold_kb": optimizers.get_indexing_threshold_kb(),
            "quantization": "disabled",
            "vector_storage": "on_disk",
        }
    });
    let shard = EdgeShard::new(path, config).map_err(qdrant_error("create shard"))?;
    let points = catalog
        .iter()
        .zip(vectors)
        .map(|(entry, vector)| PointStructPersisted {
            id: PointId::NumId(entry.point_id),
            vector: VectorStructPersisted::from(VectorStructInternal::Named(HashMap::from([(
                VECTOR_NAME.to_owned(),
                VectorInternal::from(vector),
            )]))),
            payload: None,
        })
        .collect();
    shard
        .update(UpdateOperation::PointOperation(
            PointOperations::UpsertPoints(PointInsertOperations::PointsList(points)),
        ))
        .map_err(qdrant_error("upsert complete point catalog"))?;
    let optimized = shard.optimize().map_err(qdrant_error("optimize shard"))?;
    shard.flush().map_err(qdrant_error("flush shard"))?;

    let probe = vec![1.0_f32; 1024];
    let writable = shard
        .query(exact_query(probe.clone(), catalog.len()))
        .map_err(qdrant_error("query flushed writable shard"))?;
    require_complete_qdrant_response(&writable, catalog)?;
    drop(shard);

    write_qdrant_segment_manifest(path)?;
    let readonly = ReadOnlyEdgeShard::open_mmap(path)
        .map_err(qdrant_error("open immutable read-only shard"))?;
    let reopened = readonly
        .query(exact_query(probe, catalog.len()))
        .map_err(qdrant_error("query immutable read-only shard"))?;
    require_complete_qdrant_response(&reopened, catalog)?;
    drop(readonly);
    Ok((config_json, search_threads, optimized))
}

fn write_qdrant_segment_manifest(shard_path: &Path) -> Result<()> {
    let segments_path = shard_path.join("segments");
    let mut segments = BTreeMap::new();
    for entry in fs::read_dir(&segments_path)
        .map_err(storage_error("scan qdrant segments", &segments_path))?
    {
        let entry = entry.map_err(|error| WorkVcsError::StorageFailure(error.to_string()))?;
        let path = entry.path();
        let metadata =
            fs::symlink_metadata(&path).map_err(storage_error("inspect qdrant segment", &path))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(WorkVcsError::StorageFailure(format!(
                "qdrant segment member {} is not a real directory",
                path.display()
            )));
        }
        let name = entry.file_name().into_string().map_err(|_| {
            WorkVcsError::StorageFailure("qdrant segment directory name is not UTF-8".to_owned())
        })?;
        let uuid = uuid::Uuid::parse_str(&name).map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "qdrant segment directory {name:?} is not a UUID: {error}"
            ))
        })?;
        segments.insert(uuid, SegmentManifestState::Active);
    }
    if segments.is_empty() {
        return Err(WorkVcsError::StorageFailure(
            "qdrant shard has no segment directory after non-empty build".to_owned(),
        ));
    }
    let bytes = serde_json::to_vec_pretty(&segments).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!(
            "cannot encode ordered qdrant segment manifest: {error}"
        ))
    })?;
    let parsed: SegmentsManifest = serde_json::from_slice(&bytes).map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "ordered qdrant segment manifest did not roundtrip through public API: {error}"
        ))
    })?;
    if parsed.len() != segments.len() {
        return Err(WorkVcsError::StorageFailure(
            "qdrant segment manifest public API roundtrip changed segment count".to_owned(),
        ));
    }
    let destination = shard_path.join(QDRANT_SEGMENT_MANIFEST);
    let temp = shard_path.join(format!(".segments-manifest-{}.tmp", uuid::Uuid::now_v7()));
    write_file_new(&temp, &bytes)?;
    super::super::atomic_publish_new_index(&temp, &destination)?;
    super::super::sync_parent(&destination)
}

fn validate_generation(
    path: &Path,
    snapshot: &SourceSnapshot,
    expected_basis: Digest,
    expected_model: Digest,
) -> Result<(SemanticGenerationManifest, Vec<SemanticPassageCatalogEntry>)> {
    let manifest_bytes = fs::read(path.join(GENERATION_MANIFEST_FILE)).map_err(storage_error(
        "read semantic generation manifest",
        &path.join(GENERATION_MANIFEST_FILE),
    ))?;
    let manifest: SemanticGenerationManifest =
        serde_json::from_slice(&manifest_bytes).map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "semantic generation manifest {} is invalid: {error}",
                path.display()
            ))
        })?;
    if manifest.format_version != SEMANTIC_GENERATION_FORMAT_VERSION
        || manifest.ranking_policy_version != SEMANTIC_RANKING_POLICY_VERSION
        || manifest.window_policy_version != SEMANTIC_WINDOW_POLICY_VERSION
        || manifest.identity != snapshot.identity
        || manifest.temporal_scope != snapshot.temporal_scope
        || manifest.source_commit_id != snapshot.source_commit_id
        || manifest.source_state_digest != snapshot.source_state_digest
        || manifest.object_version_basis_digest != snapshot.object_version_basis_digest
        || manifest.object_count != snapshot.documents.len()
        || manifest.embedding_model_manifest_digest != expected_model
        || manifest.embedding_profile != SemanticModelProfile::MultilingualE5LargeFp32V1
        || manifest.dimension != 1024
        || manifest.distance != "cosine"
        || manifest.vector_name != VECTOR_NAME
        || !manifest.exact_search
        || manifest.request_limit_rule != "complete_manifest_point_count"
        || manifest.quantization != "disabled"
        || manifest.qdrant_edge_version != "0.8.0"
        || manifest.fastembed_version != "7.1.1"
        || manifest.ort_version != "2.0.0-rc.13"
        || manifest.basis_digest != expected_basis
    {
        return Err(WorkVcsError::StorageFailure(format!(
            "semantic generation {} does not match the complete source/model/runtime basis; use a new generation",
            path.display()
        )));
    }
    let catalog_bytes = fs::read(path.join(CATALOG_FILE)).map_err(storage_error(
        "read semantic catalog",
        &path.join(CATALOG_FILE),
    ))?;
    if Digest::domain_separated("workvcs-semantic-passage-catalog-v1", &catalog_bytes)
        != manifest.catalog_digest
    {
        return Err(WorkVcsError::StorageFailure(format!(
            "semantic generation {} catalog digest mismatch",
            path.display()
        )));
    }
    let catalog: Vec<SemanticPassageCatalogEntry> = serde_json::from_slice(&catalog_bytes)
        .map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "semantic generation catalog {} is invalid: {error}",
                path.display()
            ))
        })?;
    if catalog.len() != manifest.point_count
        || point_id_digest(catalog.iter().map(|entry| entry.point_id)) != manifest.point_id_digest
    {
        return Err(WorkVcsError::StorageFailure(format!(
            "semantic generation {} point/catalog correspondence is incomplete",
            path.display()
        )));
    }
    validate_catalog_readback(snapshot, &catalog)?;
    if catalog.is_empty() {
        if !path.join(QDRANT_DIRECTORY).is_dir() {
            return Err(WorkVcsError::StorageFailure(
                "empty semantic generation is missing its qdrant marker directory".to_owned(),
            ));
        }
    } else if !path
        .join(QDRANT_DIRECTORY)
        .join(QDRANT_SEGMENT_MANIFEST)
        .is_file()
    {
        return Err(WorkVcsError::StorageFailure(
            "semantic generation is missing the qdrant read-only segment manifest".to_owned(),
        ));
    }
    Ok((manifest, catalog))
}

fn validate_catalog_readback(
    snapshot: &SourceSnapshot,
    catalog: &[SemanticPassageCatalogEntry],
) -> Result<()> {
    let documents = snapshot
        .documents
        .iter()
        .map(|document| (ObjectKey::from_document(document), document))
        .collect::<BTreeMap<_, _>>();
    for (position, entry) in catalog.iter().enumerate() {
        if entry.point_id != u64::try_from(position + 1).unwrap_or(u64::MAX) {
            return Err(WorkVcsError::StorageFailure(
                "semantic passage point IDs must be unique consecutive one-based integers"
                    .to_owned(),
            ));
        }
        let document = documents
            .get(&ObjectKey::from_catalog(entry))
            .ok_or_else(|| {
                WorkVcsError::StorageFailure(format!(
                    "semantic catalog point {} has no native source object",
                    entry.point_id
                ))
            })?;
        if entry.entity_version_id != document.entity_version_id
            || entry.entity_state_digest != document.state_digest
            || entry.statement_digest != document.statement_digest
            || entry.status != document.status
            || entry.record_kind != document.record_kind
            || entry.scope_json != document.scope_json
            || entry.provenance_json != document.provenance_json
            || entry.byte_start > entry.byte_end
            || entry.byte_end > document.statement.len()
            || !document.statement.is_char_boundary(entry.byte_start)
            || !document.statement.is_char_boundary(entry.byte_end)
            || document.statement[entry.byte_start..entry.byte_end] != entry.text
        {
            return Err(WorkVcsError::StorageFailure(format!(
                "semantic catalog point {} failed native identity, digest, metadata, statement, or UTF-8 range readback",
                entry.point_id
            )));
        }
    }
    let catalog_objects = catalog
        .iter()
        .map(ObjectKey::from_catalog)
        .collect::<BTreeSet<_>>();
    let native_objects = snapshot
        .documents
        .iter()
        .map(ObjectKey::from_document)
        .collect::<BTreeSet<_>>();
    if catalog_objects != native_objects {
        return Err(WorkVcsError::StorageFailure(
            "semantic catalog does not cover every eligible native object".to_owned(),
        ));
    }
    Ok(())
}

fn exact_vector_scores(
    generation_path: &Path,
    catalog: &[SemanticPassageCatalogEntry],
    query_inputs: &[String],
    embedding: &mut TextEmbedding,
) -> Result<BTreeMap<u64, f32>> {
    if catalog.is_empty() {
        return Ok(BTreeMap::new());
    }
    let query_vectors = embedding
        .embed(query_inputs, Some(EMBEDDING_BATCH_SIZE))
        .map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "cannot embed complete semantic query windows: {error}"
            ))
        })?;
    if query_vectors.len() != query_inputs.len()
        || query_vectors
            .iter()
            .any(|vector| vector.len() != 1024 || vector.iter().any(|value| !value.is_finite()))
    {
        return Err(WorkVcsError::StorageFailure(
            "semantic query embedding output is incomplete, wrong-dimensional, or non-finite"
                .to_owned(),
        ));
    }
    let shard = ReadOnlyEdgeShard::open_mmap(&generation_path.join(QDRANT_DIRECTORY))
        .map_err(qdrant_error("open semantic generation read-only"))?;
    let mut scores = BTreeMap::new();
    for vector in query_vectors {
        let response = shard
            .query(exact_query(vector, catalog.len()))
            .map_err(qdrant_error("execute complete exact semantic query"))?;
        require_complete_qdrant_response(&response, catalog)?;
        for point in response {
            let PointId::NumId(point_id) = point.id else {
                return Err(WorkVcsError::StorageFailure(
                    "semantic qdrant response contains a non-numeric point ID".to_owned(),
                ));
            };
            if !point.score.is_finite() {
                return Err(WorkVcsError::StorageFailure(
                    "semantic qdrant response contains a non-finite score".to_owned(),
                ));
            }
            scores
                .entry(point_id)
                .and_modify(|score: &mut f32| *score = score.max(point.score))
                .or_insert(point.score);
        }
    }
    if scores.len() != catalog.len() {
        return Err(WorkVcsError::StorageFailure(
            "semantic vector scoring did not cover every passage point".to_owned(),
        ));
    }
    Ok(scores)
}

fn full_corpus_reranker_scores(
    query_windows: &[String],
    catalog: &[SemanticPassageCatalogEntry],
    tokenizer: &tokenizers::Tokenizer,
    reranker: &mut TextRerank,
) -> Result<(BTreeMap<u64, f32>, usize, usize)> {
    if catalog.is_empty() {
        return Ok((BTreeMap::new(), 0, 0));
    }
    let passages = catalog
        .iter()
        .map(|entry| entry.text.clone())
        .collect::<Vec<_>>();
    let mut pair_tokens = 0_usize;
    let mut pair_count = 0_usize;
    let mut scores = BTreeMap::new();
    for query in query_windows {
        for passage in &passages {
            let tokens = pair_token_count(tokenizer, query, passage)?;
            if tokens > BGE_MAX_TOKENS {
                return Err(WorkVcsError::StorageFailure(format!(
                    "reranker pair requires {tokens} tokens, exceeding declared context {BGE_MAX_TOKENS}; windowing contract is invalid"
                )));
            }
            pair_tokens = pair_tokens.saturating_add(tokens);
            pair_count = pair_count.saturating_add(1);
        }
        let passage_refs = passages.iter().map(String::as_str).collect::<Vec<_>>();
        let ranked = reranker
            .rerank(
                query.as_str(),
                passage_refs.as_slice(),
                false,
                Some(RERANKER_BATCH_SIZE),
            )
            .map_err(|error| {
                WorkVcsError::StorageFailure(format!(
                    "full-corpus semantic reranking failed: {error}"
                ))
            })?;
        if ranked.len() != catalog.len() {
            return Err(WorkVcsError::StorageFailure(
                "full-corpus semantic reranking returned an incomplete passage set".to_owned(),
            ));
        }
        let mut seen = BTreeSet::new();
        for result in ranked {
            if result.index >= catalog.len()
                || !seen.insert(result.index)
                || !result.score.is_finite()
            {
                return Err(WorkVcsError::StorageFailure(
                    "full-corpus reranker returned duplicate, unknown, or non-finite output"
                        .to_owned(),
                ));
            }
            let point_id = catalog[result.index].point_id;
            scores
                .entry(point_id)
                .and_modify(|score: &mut f32| *score = score.max(result.score))
                .or_insert(result.score);
        }
    }
    if scores.len() != catalog.len() {
        return Err(WorkVcsError::StorageFailure(
            "reranker scoring did not cover every semantic passage".to_owned(),
        ));
    }
    Ok((scores, pair_tokens, pair_count))
}

fn complete_lexical_ranks(
    source: &StoreConnection,
    source_path: &Path,
    options: &SemanticQueryOptions,
    snapshot: &SourceSnapshot,
) -> Result<BTreeMap<ObjectKey, usize>> {
    let Some(expression) = super::super::tokenizer::fts_query(&options.query) else {
        return Ok(BTreeMap::new());
    };
    let mut lexical = SearchQueryOptions::managed(
        options.identity,
        options
            .lexical_root
            .as_ref()
            .expect("lexical root validated"),
        &options.query,
    )?;
    if let Some(commit_id) = options.commit_id {
        lexical = lexical.at_commit(commit_id);
    }
    for protected in &options.protected_paths {
        lexical = lexical.with_protected_path(protected);
    }
    let result =
        super::super::managed_query_index(source, source_path, &lexical, snapshot, &expression)?;
    if result.status != crate::search::SearchDerivedStatus::Ready {
        return Err(WorkVcsError::BranchHeadConflict(
            "native source changed while producing complete lexical ranks".to_owned(),
        ));
    }
    Ok(result
        .items
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            (
                ObjectKey {
                    source_type: item.source_type.as_str().to_owned(),
                    entity_id: item.entity_id,
                },
                index + 1,
            )
        })
        .collect())
}

fn rank_and_materialize(
    snapshot: &SourceSnapshot,
    catalog: &[SemanticPassageCatalogEntry],
    vector_passages: &BTreeMap<u64, f32>,
    reranker_passages: &BTreeMap<u64, f32>,
    lexical_ranks: &BTreeMap<ObjectKey, usize>,
    mode: SemanticRankingMode,
) -> Result<Vec<SemanticSearchItem>> {
    let mut object_vector = BTreeMap::<ObjectKey, f32>::new();
    let mut object_reranker = BTreeMap::<ObjectKey, f32>::new();
    for entry in catalog {
        let key = ObjectKey::from_catalog(entry);
        if let Some(score) = vector_passages.get(&entry.point_id) {
            object_vector
                .entry(key.clone())
                .and_modify(|current| *current = current.max(*score))
                .or_insert(*score);
        }
        if let Some(score) = reranker_passages.get(&entry.point_id) {
            object_reranker
                .entry(key)
                .and_modify(|current| *current = current.max(*score))
                .or_insert(*score);
        }
    }
    let vector_ranks = component_ranks(&object_vector);
    let reranker_ranks = component_ranks(&object_reranker);
    let hybrid_scores = snapshot
        .documents
        .iter()
        .map(|document| {
            let key = ObjectKey::from_document(document);
            let score =
                rrf(vector_ranks.get(&key).copied()) + rrf(lexical_ranks.get(&key).copied());
            (key, score)
        })
        .collect::<BTreeMap<_, _>>();
    let hybrid_ranks = component_ranks_f64(&hybrid_scores);

    let final_scores = snapshot
        .documents
        .iter()
        .map(|document| {
            let key = ObjectKey::from_document(document);
            let score = match mode {
                SemanticRankingMode::Vector => {
                    f64::from(*object_vector.get(&key).unwrap_or(&f32::NEG_INFINITY))
                }
                SemanticRankingMode::Rerank => {
                    f64::from(*object_reranker.get(&key).unwrap_or(&f32::NEG_INFINITY))
                }
                SemanticRankingMode::HybridRrf => *hybrid_scores.get(&key).unwrap_or(&0.0),
                SemanticRankingMode::RerankVectorRrf => {
                    rrf(reranker_ranks.get(&key).copied()) + rrf(vector_ranks.get(&key).copied())
                }
                SemanticRankingMode::RerankHybridRrf => {
                    rrf(reranker_ranks.get(&key).copied()) + rrf(hybrid_ranks.get(&key).copied())
                }
            };
            (key, score)
        })
        .collect::<BTreeMap<_, _>>();
    let ordered = sorted_f64(&final_scores);

    let entries_by_object = catalog.iter().fold(
        BTreeMap::<ObjectKey, Vec<&SemanticPassageCatalogEntry>>::new(),
        |mut grouped, entry| {
            grouped
                .entry(ObjectKey::from_catalog(entry))
                .or_default()
                .push(entry);
            grouped
        },
    );
    let documents = snapshot
        .documents
        .iter()
        .map(|document| (ObjectKey::from_document(document), document))
        .collect::<BTreeMap<_, _>>();
    let mut items = Vec::with_capacity(ordered.len());
    for key in ordered {
        let document = documents.get(&key).ok_or_else(|| {
            WorkVcsError::StorageFailure("semantic rank has no native document".to_owned())
        })?;
        let vector_max = object_vector.get(&key).copied();
        let reranker_max = object_reranker.get(&key).copied();
        let mut passages = entries_by_object
            .get(&key)
            .into_iter()
            .flatten()
            .map(|entry| SemanticPassageScore {
                point_id: entry.point_id,
                byte_start: entry.byte_start,
                byte_end: entry.byte_end,
                text: entry.text.clone(),
                vector_score: vector_passages.get(&entry.point_id).copied(),
                reranker_score: reranker_passages.get(&entry.point_id).copied(),
                winning_vector_range: vector_passages.get(&entry.point_id).copied() == vector_max,
                winning_reranker_range: reranker_passages.get(&entry.point_id).copied()
                    == reranker_max,
                readback_verified: true,
            })
            .collect::<Vec<_>>();
        passages.sort_by(|left, right| {
            let left_score = if mode.needs_reranker() {
                left.reranker_score.unwrap_or(f32::NEG_INFINITY)
            } else {
                left.vector_score.unwrap_or(f32::NEG_INFINITY)
            };
            let right_score = if mode.needs_reranker() {
                right.reranker_score.unwrap_or(f32::NEG_INFINITY)
            } else {
                right.vector_score.unwrap_or(f32::NEG_INFINITY)
            };
            right_score
                .total_cmp(&left_score)
                .then_with(|| left.byte_start.cmp(&right.byte_start))
                .then_with(|| left.byte_end.cmp(&right.byte_end))
                .then_with(|| left.point_id.cmp(&right.point_id))
        });
        let mut item = SemanticSearchItem {
            source_type: document.source_type,
            entity_id: document.entity_id,
            entity_version_id: document.entity_version_id,
            entity_state_digest: document.state_digest,
            statement_digest: document.statement_digest,
            status: document.status.clone(),
            record_kind: document.record_kind.clone(),
            scope_json: document.scope_json.clone(),
            provenance_json: document.provenance_json.clone(),
            canonical_statement: document.statement.clone(),
            score: *final_scores.get(&key).unwrap_or(&0.0),
            vector_score: vector_max,
            reranker_score: reranker_max,
            lexical_rank: lexical_ranks.get(&key).copied(),
            vector_rank: vector_ranks.get(&key).copied(),
            reranker_rank: reranker_ranks.get(&key).copied(),
            passages,
            readback_verified: true,
            serialized_tokens: 0,
        };
        let bytes = serde_json::to_vec(&item).map_err(|error| {
            WorkVcsError::CanonicalEncodingInvalid(format!(
                "cannot serialize semantic search item: {error}"
            ))
        })?;
        let text = std::str::from_utf8(&bytes).map_err(|error| {
            WorkVcsError::CanonicalEncodingInvalid(format!(
                "semantic search item JSON is not UTF-8: {error}"
            ))
        })?;
        item.serialized_tokens = super::super::search_o200k_ordinary_token_count(text);
        items.push(item);
    }
    Ok(items)
}

fn component_ranks(scores: &BTreeMap<ObjectKey, f32>) -> BTreeMap<ObjectKey, usize> {
    let mut keys = scores.keys().cloned().collect::<Vec<_>>();
    keys.sort_by(|left, right| {
        scores[right]
            .total_cmp(&scores[left])
            .then_with(|| left.cmp(right))
    });
    keys.into_iter()
        .enumerate()
        .map(|(index, key)| (key, index + 1))
        .collect()
}

fn component_ranks_f64(scores: &BTreeMap<ObjectKey, f64>) -> BTreeMap<ObjectKey, usize> {
    sorted_f64(scores)
        .into_iter()
        .enumerate()
        .map(|(index, key)| (key, index + 1))
        .collect()
}

fn sorted_f64(scores: &BTreeMap<ObjectKey, f64>) -> Vec<ObjectKey> {
    let mut keys = scores.keys().cloned().collect::<Vec<_>>();
    keys.sort_by(|left, right| {
        scores[right]
            .total_cmp(&scores[left])
            .then_with(|| left.cmp(right))
    });
    keys
}

fn rrf(rank: Option<usize>) -> f64 {
    rank.map(|rank| 1.0 / (RRF_K + rank) as f64)
        .unwrap_or_default()
}

fn exact_query(vector: Vec<f32>, limit: usize) -> QueryRequest {
    QueryRequestBuilder::new(limit)
        .query(ScoringQuery::Vector(QueryEnum::Nearest(NamedQuery::new(
            VectorInternal::from(vector),
            VECTOR_NAME.to_owned(),
        ))))
        .params(SearchParams {
            exact: true,
            indexed_only: false,
            ..SearchParams::default()
        })
        .build()
}

fn require_complete_qdrant_response(
    response: &[qdrant_edge::ScoredPoint],
    catalog: &[SemanticPassageCatalogEntry],
) -> Result<()> {
    let expected = catalog
        .iter()
        .map(|entry| entry.point_id)
        .collect::<BTreeSet<_>>();
    let mut actual = BTreeSet::new();
    for point in response {
        let PointId::NumId(point_id) = point.id else {
            return Err(WorkVcsError::StorageFailure(
                "qdrant exact response contains a non-numeric point ID".to_owned(),
            ));
        };
        if !actual.insert(point_id) || !point.score.is_finite() {
            return Err(WorkVcsError::StorageFailure(
                "qdrant exact response contains a duplicate ID or non-finite score".to_owned(),
            ));
        }
    }
    if response.len() != catalog.len() || actual != expected {
        return Err(WorkVcsError::StorageFailure(format!(
            "qdrant exact response is incomplete: expected {} unique points, received {}",
            catalog.len(),
            actual.len()
        )));
    }
    Ok(())
}

fn point_id_digest(ids: impl IntoIterator<Item = u64>) -> Digest {
    let mut bytes = Vec::new();
    for id in ids {
        bytes.extend_from_slice(&id.to_be_bytes());
    }
    Digest::domain_separated("workvcs-semantic-point-ids-v1", &bytes)
}

fn write_json_new(path: &Path, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!("cannot encode JSON: {error}"))
    })?;
    write_file_new(path, &bytes)
}

fn write_file_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(storage_error("create derived file", path))?;
    file.write_all(bytes)
        .map_err(storage_error("write derived file", path))?;
    file.sync_all()
        .map_err(storage_error("sync derived file", path))
}

fn sync_directory(path: &Path) -> Result<()> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(storage_error("sync directory", path))
}

fn directory_bytes(path: &Path) -> Result<u64> {
    let mut total = 0_u64;
    for entry in fs::read_dir(path).map_err(storage_error("read generation directory", path))? {
        let entry = entry.map_err(|error| WorkVcsError::StorageFailure(error.to_string()))?;
        let member = entry.path();
        let metadata = fs::symlink_metadata(&member)
            .map_err(storage_error("inspect generation member", &member))?;
        if metadata.file_type().is_symlink() {
            return Err(WorkVcsError::StorageFailure(format!(
                "semantic generation contains symbolic link {}",
                member.display()
            )));
        }
        if metadata.is_dir() {
            total = total.saturating_add(directory_bytes(&member)?);
        } else if metadata.is_file() {
            total = total.saturating_add(metadata.len());
        } else {
            return Err(WorkVcsError::StorageFailure(format!(
                "semantic generation contains non-file member {}",
                member.display()
            )));
        }
    }
    Ok(total)
}

fn rust_target() -> String {
    format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS)
}

fn storage_error<'a>(
    action: &'static str,
    path: &'a Path,
) -> impl FnOnce(std::io::Error) -> WorkVcsError + 'a {
    move |error| {
        WorkVcsError::StorageFailure(format!("cannot {action} {}: {error}", path.display()))
    }
}

fn qdrant_error<E: std::fmt::Display>(action: &'static str) -> impl FnOnce(E) -> WorkVcsError {
    move |error| WorkVcsError::StorageFailure(format!("cannot {action}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::SearchSourceType;

    fn catalog_entry(point_id: u64) -> SemanticPassageCatalogEntry {
        let statement = format!("semantic qdrant passage {point_id}");
        SemanticPassageCatalogEntry {
            point_id,
            source_type: SearchSourceType::Record,
            entity_id: EntityId::new_v7(),
            entity_version_id: crate::identity::EntityVersionId::new_v7(),
            entity_state_digest: Digest::domain_separated("test-state", statement.as_bytes()),
            statement_digest: Digest::domain_separated("test-statement", statement.as_bytes()),
            status: "accepted".to_owned(),
            record_kind: Some("finding".to_owned()),
            scope_json: "{}".to_owned(),
            provenance_json: None,
            byte_start: 0,
            byte_end: statement.len(),
            tokenizer_token_start: 0,
            tokenizer_token_end: 4,
            model_tokens: 6,
            text: statement,
        }
    }

    fn key(id: EntityId) -> ObjectKey {
        ObjectKey {
            source_type: SearchSourceType::Record.as_str().to_owned(),
            entity_id: id,
        }
    }

    fn source_document(entry: &SemanticPassageCatalogEntry) -> SourceDocument {
        SourceDocument {
            source_type: entry.source_type,
            entity_id: entry.entity_id,
            entity_version_id: entry.entity_version_id,
            state_digest: entry.entity_state_digest,
            statement_digest: entry.statement_digest,
            status: entry.status.clone(),
            record_kind: entry.record_kind.clone(),
            statement: entry.text.clone(),
            scope_json: entry.scope_json.clone(),
            provenance_json: entry.provenance_json.clone(),
        }
    }

    fn source_snapshot(entries: &[SemanticPassageCatalogEntry]) -> SourceSnapshot {
        SourceSnapshot {
            identity: crate::search::SearchSourceIdentity::new(
                crate::identity::ProjectRefId::new_v7(),
                crate::identity::StoreId::new_v7(),
                crate::identity::WorkspaceId::new_v7(),
                crate::identity::BranchId::new_v7(),
            ),
            temporal_scope: crate::search::SearchTemporalScope::Historical,
            source_commit_id: crate::identity::CommitId::new_v7(),
            source_state_digest: Digest::domain_separated("test-source", b"state"),
            object_version_basis_digest: Digest::domain_separated("test-source", b"versions"),
            documents: entries.iter().map(source_document).collect(),
        }
    }

    #[test]
    fn reciprocal_rank_fusion_uses_one_based_k_60() {
        assert_eq!(rrf(Some(1)), 1.0 / 61.0);
        assert_eq!(rrf(Some(2)), 1.0 / 62.0);
        assert_eq!(rrf(None), 0.0);
    }

    #[test]
    fn component_rank_ties_use_native_identity() {
        let first = EntityId::new_v7();
        let second = EntityId::new_v7();
        let (low, high) = if first < second {
            (first, second)
        } else {
            (second, first)
        };
        let scores = BTreeMap::from([(key(high), 0.5), (key(low), 0.5)]);
        let ranks = component_ranks(&scores);
        assert_eq!(ranks[&key(low)], 1);
        assert_eq!(ranks[&key(high)], 2);
    }

    #[test]
    fn qdrant_generation_reopens_readonly_with_every_point() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let shard = directory.path().join("qdrant");
        let catalog = (1..=3).map(catalog_entry).collect::<Vec<_>>();
        let vectors = vec![vec![0.0; 1024], vec![0.5; 1024], vec![1.0; 1024]];
        let (_, _, optimized) = build_qdrant(&shard, &catalog, vectors).expect("qdrant build");
        assert!(optimized, "qdrant build must produce an immutable segment");
        let readonly = ReadOnlyEdgeShard::open_mmap(&shard).expect("read-only reopen");
        let response = readonly
            .query(exact_query(vec![1.0; 1024], catalog.len()))
            .expect("exact read-only query");
        require_complete_qdrant_response(&response, &catalog).expect("complete point coverage");
    }

    #[test]
    fn generation_lock_fails_closed_for_a_second_owner() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let first = GenerationLock::create(directory.path()).expect("first owner");
        let second = GenerationLock::acquire(directory.path());
        assert!(second.is_err(), "a second shard owner must fail closed");
        drop(first);
        GenerationLock::acquire(directory.path()).expect("lock released with owner");
    }

    #[test]
    fn qdrant_generation_rejects_wrong_vector_dimension() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let shard = directory.path().join("qdrant");
        let catalog = vec![catalog_entry(1)];
        let result = build_qdrant(&shard, &catalog, vec![vec![1.0; 16]]);
        assert!(
            result.is_err(),
            "wrong-dimensional vectors must be rejected"
        );
    }

    #[test]
    fn generation_basis_changes_with_model_source_and_temporal_scope() {
        let entries = vec![catalog_entry(1)];
        let snapshot = source_snapshot(&entries);
        let first_model = Digest::domain_separated("test-model", b"first");
        let second_model = Digest::domain_separated("test-model", b"second");
        let basis = generation_basis_digest(&snapshot, first_model).expect("generation basis");
        assert_ne!(
            basis,
            generation_basis_digest(&snapshot, second_model).expect("changed model basis")
        );

        let mut changed_source = snapshot.clone();
        changed_source.source_state_digest = Digest::domain_separated("test-source", b"changed");
        assert_ne!(
            basis,
            generation_basis_digest(&changed_source, first_model).expect("changed source basis")
        );

        let mut current = snapshot;
        current.temporal_scope = crate::search::SearchTemporalScope::Current;
        assert_ne!(
            basis,
            generation_basis_digest(&current, first_model).expect("changed temporal basis")
        );
    }

    #[test]
    fn catalog_readback_rejects_damage_drift_and_incomplete_coverage() {
        let entries = vec![catalog_entry(1), catalog_entry(2)];
        let snapshot = source_snapshot(&entries);
        validate_catalog_readback(&snapshot, &entries).expect("valid native readback");

        let mut damaged = entries.clone();
        damaged[0].text.push('!');
        damaged[0].byte_end += 1;
        assert!(
            validate_catalog_readback(&snapshot, &damaged).is_err(),
            "changed passage bytes must fail native readback"
        );

        let mut drifted = entries.clone();
        drifted[0].entity_state_digest = Digest::domain_separated("test-state", b"drifted");
        assert!(
            validate_catalog_readback(&snapshot, &drifted).is_err(),
            "changed native identity digest must fail readback"
        );

        assert!(
            validate_catalog_readback(&snapshot, &entries[..1]).is_err(),
            "a catalog missing one eligible native object must fail"
        );
    }
}
