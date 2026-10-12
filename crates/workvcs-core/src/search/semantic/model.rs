use super::profile::{MODEL_MANIFEST_FORMAT_VERSION, ProfileSpec, profile_spec};
use super::{
    SemanticModelFileManifest, SemanticModelKind, SemanticModelManifest, SemanticModelProfile,
    SemanticModelProvisionOptions, SemanticModelProvisionOutcome, SemanticModelProvisionResult,
};
use crate::error::{Result, WorkVcsError};
use crate::identity::Digest;
use fastembed::{
    InitOptionsUserDefined, Pooling, RerankInitOptionsUserDefined, TextEmbedding, TextRerank,
    TokenizerFiles, UserDefinedEmbeddingModel, UserDefinedRerankingModel,
};
use onnx_protobuf::{
    AttributeProto, FunctionProto, GraphProto, ModelProto, SparseTensorProto, TensorProto,
};
use protobuf::Message;
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

const MODEL_MANIFEST_FILE: &str = "model-manifest.json";
const FASTEMBED_VERSION: &str = "7.1.1";
const ORT_VERSION: &str = "2.0.0-rc.13";

#[derive(Serialize)]
struct ModelManifestBasis<'a> {
    format_version: u32,
    profile: SemanticModelProfile,
    kind: SemanticModelKind,
    source: &'a str,
    revision: &'a str,
    files: &'a [SemanticModelFileManifest],
    max_tokens: usize,
    dimension: Option<usize>,
    query_prefix: Option<&'a str>,
    passage_prefix: Option<&'a str>,
    pooling: Option<&'a str>,
    output_key: &'a str,
    fastembed_version: &'a str,
    ort_version: &'a str,
}

pub(super) fn provision_model(
    options: &SemanticModelProvisionOptions,
) -> Result<SemanticModelProvisionResult> {
    ensure_supported_platform()?;
    let spec = profile_spec(options.profile());
    let source = canonical_source(options.source_directory())?;
    let destination = validate_destination(options.destination(), &source)?;

    if destination.exists() {
        let manifest = validate_model_directory(&destination, options.profile())?;
        run_canary(&destination, &manifest)?;
        return Ok(SemanticModelProvisionResult {
            outcome: SemanticModelProvisionOutcome::Reused,
            model_path: destination,
            manifest,
            inference_canary_passed: true,
            network_access_used: false,
        });
    }

    validate_source_tree(&source, spec)?;
    let parent = destination.parent().ok_or_else(|| {
        WorkVcsError::QueryInvalid("semantic model destination has no parent".to_owned())
    })?;
    let stage = parent.join(format!(
        ".workvcs-model-{}-{}.tmp",
        spec.profile.as_str(),
        uuid::Uuid::now_v7()
    ));
    fs::create_dir(&stage).map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "cannot create private semantic model staging directory {}: {error}",
            stage.display()
        ))
    })?;
    set_private_directory_permissions(&stage)?;

    let result = (|| {
        let mut files = Vec::with_capacity(spec.files.len());
        for file_spec in spec.files {
            let source_path = source.join(file_spec.source_name);
            let destination_path = stage.join(file_spec.name);
            let (sha256, byte_length) =
                copy_verified_regular_file(&source_path, &destination_path, file_spec.sha256)?;
            files.push(SemanticModelFileManifest {
                name: file_spec.name.to_owned(),
                sha256,
                byte_length,
            });
        }
        files.sort_by(|left, right| left.name.cmp(&right.name));

        validate_onnx_external_data(&stage.join("model.onnx"), spec)?;
        validate_materialized_tree(&stage, spec, false)?;

        let manifest = build_manifest(spec, files)?;
        write_json_file_new(&stage.join(MODEL_MANIFEST_FILE), &manifest)?;
        sync_directory(&stage)?;
        let validated = validate_model_directory(&stage, options.profile())?;
        run_canary(&stage, &validated)?;

        let _publication_lock = super::super::SearchPublicationLock::acquire(&destination)?;
        if destination.exists() {
            return Err(WorkVcsError::StorageFailure(format!(
                "semantic model destination {} appeared during provisioning and was preserved",
                destination.display()
            )));
        }
        super::super::atomic_publish_new_index(&stage, &destination)?;
        super::super::sync_parent(&destination)?;
        let published = validate_model_directory(&destination, options.profile())?;
        run_canary(&destination, &published)?;
        Ok(published)
    })();

    if result.is_err() && stage.exists() {
        let _ = fs::remove_dir_all(&stage);
    }
    let manifest = result?;
    Ok(SemanticModelProvisionResult {
        outcome: SemanticModelProvisionOutcome::Provisioned,
        model_path: destination,
        manifest,
        inference_canary_passed: true,
        network_access_used: false,
    })
}

pub fn load_embedding_model(model_path: &Path) -> Result<(SemanticModelManifest, TextEmbedding)> {
    let manifest =
        validate_model_directory(model_path, SemanticModelProfile::MultilingualE5LargeFp32V1)?;
    let tokenizer = tokenizer_files(model_path)?;
    let graph = read_regular_file(model_path.join("model.onnx"))?;
    let external = read_regular_file(model_path.join("model.onnx_data"))?;
    let model = UserDefinedEmbeddingModel::new(graph, tokenizer)
        .with_external_initializer("model.onnx_data".to_owned(), external)
        .with_pooling(Pooling::Mean);
    let embedding = TextEmbedding::try_new_from_user_defined(
        model,
        InitOptionsUserDefined::new().with_max_length(manifest.max_tokens),
    )
    .map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "cannot load verified local embedding model {}: {error}",
            model_path.display()
        ))
    })?;
    Ok((manifest, embedding))
}

pub fn load_reranker_model(model_path: &Path) -> Result<(SemanticModelManifest, TextRerank)> {
    let manifest =
        validate_model_directory(model_path, SemanticModelProfile::BgeRerankerV2M3Fp32V1)?;
    let tokenizer = tokenizer_files(model_path)?;
    let reranker = TextRerank::try_new_from_user_defined(
        UserDefinedRerankingModel::new(model_path.join("model.onnx"), tokenizer),
        RerankInitOptionsUserDefined::new().with_max_length(manifest.max_tokens),
    )
    .map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "cannot load verified local reranker model {}: {error}",
            model_path.display()
        ))
    })?;
    Ok((manifest, reranker))
}

pub(super) fn validate_model_directory(
    model_path: &Path,
    expected_profile: SemanticModelProfile,
) -> Result<SemanticModelManifest> {
    ensure_supported_platform()?;
    if !model_path.is_absolute() {
        return Err(WorkVcsError::QueryInvalid(
            "semantic model path must be absolute".to_owned(),
        ));
    }
    let spec = profile_spec(expected_profile);
    validate_materialized_tree(model_path, spec, true)?;
    let bytes = read_regular_file(model_path.join(MODEL_MANIFEST_FILE))?;
    let manifest: SemanticModelManifest = serde_json::from_slice(&bytes).map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "semantic model manifest {} is invalid JSON: {error}",
            model_path.join(MODEL_MANIFEST_FILE).display()
        ))
    })?;
    validate_manifest_fields(&manifest, spec)?;

    let expected_digest = model_manifest_digest(&manifest)?;
    if manifest.manifest_digest != expected_digest {
        return Err(WorkVcsError::StorageFailure(format!(
            "semantic model manifest digest mismatch at {}; provision the exact profile again",
            model_path.display()
        )));
    }
    let expected_files = spec
        .files
        .iter()
        .map(|file| (file.name, file.sha256))
        .collect::<BTreeMap<_, _>>();
    if manifest.files.len() != expected_files.len() {
        return Err(WorkVcsError::StorageFailure(format!(
            "semantic model {} has an incomplete file manifest",
            model_path.display()
        )));
    }
    for file in &manifest.files {
        let expected_sha = expected_files.get(file.name.as_str()).ok_or_else(|| {
            WorkVcsError::StorageFailure(format!(
                "semantic model manifest contains unexpected file {:?}",
                file.name
            ))
        })?;
        if file.sha256 != *expected_sha {
            return Err(WorkVcsError::StorageFailure(format!(
                "semantic model manifest hash for {} does not match profile {}",
                file.name, expected_profile
            )));
        }
        let (actual_sha, actual_length) = hash_regular_file(&model_path.join(&file.name))?;
        if actual_sha != file.sha256 || actual_length != file.byte_length {
            return Err(WorkVcsError::StorageFailure(format!(
                "semantic model file {} changed after provisioning; reprovision the model",
                model_path.join(&file.name).display()
            )));
        }
    }
    validate_onnx_external_data(&model_path.join("model.onnx"), spec)?;
    Ok(manifest)
}

fn build_manifest(
    spec: &ProfileSpec,
    files: Vec<SemanticModelFileManifest>,
) -> Result<SemanticModelManifest> {
    let mut manifest = SemanticModelManifest {
        format_version: MODEL_MANIFEST_FORMAT_VERSION,
        profile: spec.profile,
        kind: spec.kind,
        source: spec.source.to_owned(),
        revision: spec.revision.to_owned(),
        files,
        max_tokens: spec.max_tokens,
        dimension: spec.dimension,
        query_prefix: spec.query_prefix.map(ToOwned::to_owned),
        passage_prefix: spec.passage_prefix.map(ToOwned::to_owned),
        pooling: spec.pooling.map(ToOwned::to_owned),
        output_key: spec.output_key.to_owned(),
        fastembed_version: FASTEMBED_VERSION.to_owned(),
        ort_version: ORT_VERSION.to_owned(),
        created_at_us: crate::store::current_epoch_micros()?,
        manifest_digest: Digest::from_bytes([0; 32]),
    };
    manifest.manifest_digest = model_manifest_digest(&manifest)?;
    Ok(manifest)
}

fn model_manifest_digest(manifest: &SemanticModelManifest) -> Result<Digest> {
    let basis = ModelManifestBasis {
        format_version: manifest.format_version,
        profile: manifest.profile,
        kind: manifest.kind,
        source: &manifest.source,
        revision: &manifest.revision,
        files: &manifest.files,
        max_tokens: manifest.max_tokens,
        dimension: manifest.dimension,
        query_prefix: manifest.query_prefix.as_deref(),
        passage_prefix: manifest.passage_prefix.as_deref(),
        pooling: manifest.pooling.as_deref(),
        output_key: &manifest.output_key,
        fastembed_version: &manifest.fastembed_version,
        ort_version: &manifest.ort_version,
    };
    let bytes = serde_json::to_vec(&basis).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!(
            "cannot encode semantic model manifest basis: {error}"
        ))
    })?;
    Ok(Digest::domain_separated(
        "workvcs-semantic-model-manifest-v1",
        &bytes,
    ))
}

fn validate_manifest_fields(manifest: &SemanticModelManifest, spec: &ProfileSpec) -> Result<()> {
    if manifest.format_version != MODEL_MANIFEST_FORMAT_VERSION
        || manifest.profile != spec.profile
        || manifest.kind != spec.kind
        || manifest.source != spec.source
        || manifest.revision != spec.revision
        || manifest.max_tokens != spec.max_tokens
        || manifest.dimension != spec.dimension
        || manifest.query_prefix.as_deref() != spec.query_prefix
        || manifest.passage_prefix.as_deref() != spec.passage_prefix
        || manifest.pooling.as_deref() != spec.pooling
        || manifest.output_key != spec.output_key
        || manifest.fastembed_version != FASTEMBED_VERSION
        || manifest.ort_version != ORT_VERSION
    {
        return Err(WorkVcsError::StorageFailure(format!(
            "semantic model manifest does not match supported profile {}; provision the exact profile again",
            spec.profile
        )));
    }
    Ok(())
}

fn run_canary(model_path: &Path, manifest: &SemanticModelManifest) -> Result<()> {
    match manifest.kind {
        SemanticModelKind::Embedding => {
            let (_, mut model) = load_embedding_model(model_path)?;
            let vectors = model
                .embed(
                    [
                        "query: WorkVCS native semantic canary",
                        "passage: WorkVCS semantic results require native readback.",
                    ],
                    None,
                )
                .map_err(|error| {
                    WorkVcsError::StorageFailure(format!(
                        "semantic embedding canary failed for {}: {error}",
                        model_path.display()
                    ))
                })?;
            let dimension = manifest.dimension.unwrap_or_default();
            if vectors.len() != 2
                || vectors.iter().any(|vector| {
                    vector.len() != dimension || vector.iter().any(|value| !value.is_finite())
                })
            {
                return Err(WorkVcsError::StorageFailure(format!(
                    "semantic embedding canary returned invalid dimension or non-finite values for {}",
                    model_path.display()
                )));
            }
        }
        SemanticModelKind::Reranker => {
            let (_, mut model) = load_reranker_model(model_path)?;
            let scores = model
                .rerank(
                    "WorkVCS native semantic canary",
                    [
                        "WorkVCS semantic results require native readback.",
                        "An unrelated sentence.",
                    ],
                    false,
                    None,
                )
                .map_err(|error| {
                    WorkVcsError::StorageFailure(format!(
                        "semantic reranker canary failed for {}: {error}",
                        model_path.display()
                    ))
                })?;
            if scores.len() != 2 || scores.iter().any(|score| !score.score.is_finite()) {
                return Err(WorkVcsError::StorageFailure(format!(
                    "semantic reranker canary returned incomplete or non-finite scores for {}",
                    model_path.display()
                )));
            }
        }
    }
    Ok(())
}

fn tokenizer_files(model_path: &Path) -> Result<TokenizerFiles> {
    Ok(TokenizerFiles {
        tokenizer_file: read_regular_file(model_path.join("tokenizer.json"))?,
        config_file: read_regular_file(model_path.join("config.json"))?,
        special_tokens_map_file: read_regular_file(model_path.join("special_tokens_map.json"))?,
        tokenizer_config_file: read_regular_file(model_path.join("tokenizer_config.json"))?,
    })
}

fn validate_onnx_external_data(graph_path: &Path, spec: &ProfileSpec) -> Result<()> {
    let graph_bytes = read_regular_file(graph_path)?;
    let model = ModelProto::parse_from_bytes(&graph_bytes).map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "cannot parse ONNX graph {} before runtime load: {error}",
            graph_path.display()
        ))
    })?;
    let declared = spec
        .files
        .iter()
        .filter(|file| file.name.ends_with(".onnx_data"))
        .map(|file| file.name.to_owned())
        .collect::<BTreeSet<_>>();
    let mut referenced = BTreeSet::new();
    if let Some(graph) = model.graph.as_ref() {
        visit_graph(graph, &declared, &mut referenced)?;
    }
    for training in &model.training_info {
        if let Some(graph) = training.initialization.as_ref() {
            visit_graph(graph, &declared, &mut referenced)?;
        }
        if let Some(graph) = training.algorithm.as_ref() {
            visit_graph(graph, &declared, &mut referenced)?;
        }
    }
    for function in &model.functions {
        visit_function(function, &declared, &mut referenced)?;
    }
    if referenced != declared {
        return Err(WorkVcsError::StorageFailure(format!(
            "ONNX graph {} external-data references {:?}, expected exactly {:?}",
            graph_path.display(),
            referenced,
            declared
        )));
    }
    Ok(())
}

fn visit_function(
    function: &FunctionProto,
    declared: &BTreeSet<String>,
    referenced: &mut BTreeSet<String>,
) -> Result<()> {
    for attribute in &function.attribute_proto {
        visit_attribute(attribute, declared, referenced)?;
    }
    for node in &function.node {
        for attribute in &node.attribute {
            visit_attribute(attribute, declared, referenced)?;
        }
    }
    Ok(())
}

fn visit_graph(
    graph: &GraphProto,
    declared: &BTreeSet<String>,
    referenced: &mut BTreeSet<String>,
) -> Result<()> {
    for tensor in &graph.initializer {
        visit_tensor(tensor, declared, referenced)?;
    }
    for sparse in &graph.sparse_initializer {
        visit_sparse_tensor(sparse, declared, referenced)?;
    }
    for node in &graph.node {
        for attribute in &node.attribute {
            visit_attribute(attribute, declared, referenced)?;
        }
    }
    Ok(())
}

fn visit_attribute(
    attribute: &AttributeProto,
    declared: &BTreeSet<String>,
    referenced: &mut BTreeSet<String>,
) -> Result<()> {
    if let Some(tensor) = attribute.t.as_ref() {
        visit_tensor(tensor, declared, referenced)?;
    }
    if let Some(graph) = attribute.g.as_ref() {
        visit_graph(graph, declared, referenced)?;
    }
    if let Some(sparse) = attribute.sparse_tensor.as_ref() {
        visit_sparse_tensor(sparse, declared, referenced)?;
    }
    for tensor in &attribute.tensors {
        visit_tensor(tensor, declared, referenced)?;
    }
    for graph in &attribute.graphs {
        visit_graph(graph, declared, referenced)?;
    }
    for sparse in &attribute.sparse_tensors {
        visit_sparse_tensor(sparse, declared, referenced)?;
    }
    Ok(())
}

fn visit_sparse_tensor(
    sparse: &SparseTensorProto,
    declared: &BTreeSet<String>,
    referenced: &mut BTreeSet<String>,
) -> Result<()> {
    if let Some(values) = sparse.values.as_ref() {
        visit_tensor(values, declared, referenced)?;
    }
    if let Some(indices) = sparse.indices.as_ref() {
        visit_tensor(indices, declared, referenced)?;
    }
    Ok(())
}

fn visit_tensor(
    tensor: &TensorProto,
    declared: &BTreeSet<String>,
    referenced: &mut BTreeSet<String>,
) -> Result<()> {
    if tensor.external_data.is_empty() {
        return Ok(());
    }
    let mut keys = BTreeSet::new();
    let mut location = None;
    for entry in &tensor.external_data {
        if !keys.insert(entry.key.clone()) {
            return Err(WorkVcsError::StorageFailure(format!(
                "ONNX tensor {:?} has duplicate external-data key {:?}",
                tensor.name, entry.key
            )));
        }
        if entry.key == "location" {
            location = Some(entry.value.as_str());
        }
    }
    let location = location.ok_or_else(|| {
        WorkVcsError::StorageFailure(format!(
            "ONNX tensor {:?} uses external data without a location",
            tensor.name
        ))
    })?;
    let path = Path::new(location);
    let mut components = path.components();
    let valid = matches!(components.next(), Some(Component::Normal(_)))
        && components.next().is_none()
        && !path.is_absolute();
    if !valid || !declared.contains(location) {
        return Err(WorkVcsError::StorageFailure(format!(
            "ONNX tensor {:?} references undeclared or unsafe external-data path {:?}",
            tensor.name, location
        )));
    }
    referenced.insert(location.to_owned());
    Ok(())
}

fn validate_source_tree(source: &Path, spec: &ProfileSpec) -> Result<()> {
    let allowed_files = spec
        .files
        .iter()
        .map(|file| PathBuf::from(file.source_name))
        .collect::<BTreeSet<_>>();
    let mut allowed_directories = BTreeSet::new();
    for file in &allowed_files {
        let mut parent = file.parent();
        while let Some(path) = parent {
            if path.as_os_str().is_empty() {
                break;
            }
            allowed_directories.insert(path.to_path_buf());
            parent = path.parent();
        }
    }
    validate_tree_members(source, source, &allowed_files, &allowed_directories)
}

fn validate_materialized_tree(
    directory: &Path,
    spec: &ProfileSpec,
    include_manifest: bool,
) -> Result<()> {
    let mut allowed_files = spec
        .files
        .iter()
        .map(|file| PathBuf::from(file.name))
        .collect::<BTreeSet<_>>();
    if include_manifest {
        allowed_files.insert(PathBuf::from(MODEL_MANIFEST_FILE));
    }
    validate_tree_members(directory, directory, &allowed_files, &BTreeSet::new())
}

fn validate_tree_members(
    root: &Path,
    current: &Path,
    allowed_files: &BTreeSet<PathBuf>,
    allowed_directories: &BTreeSet<PathBuf>,
) -> Result<()> {
    let metadata = fs::symlink_metadata(current).map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "cannot inspect semantic model path {}: {error}",
            current.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(WorkVcsError::QueryInvalid(format!(
            "semantic model tree member {} must be a real directory",
            current.display()
        )));
    }
    for entry in fs::read_dir(current).map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "cannot read semantic model directory {}: {error}",
            current.display()
        ))
    })? {
        let entry = entry.map_err(|error| WorkVcsError::StorageFailure(error.to_string()))?;
        let path = entry.path();
        let relative = path.strip_prefix(root).map_err(|error| {
            WorkVcsError::StorageFailure(format!("cannot derive model-relative path: {error}"))
        })?;
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "cannot inspect semantic model member {}: {error}",
                path.display()
            ))
        })?;
        if metadata.file_type().is_symlink() {
            return Err(WorkVcsError::QueryInvalid(format!(
                "semantic model tree rejects symbolic link {}",
                path.display()
            )));
        }
        if metadata.is_dir() {
            if !allowed_directories.contains(relative) {
                return Err(WorkVcsError::QueryInvalid(format!(
                    "semantic model tree contains unexpected directory {}",
                    path.display()
                )));
            }
            validate_tree_members(root, &path, allowed_files, allowed_directories)?;
        } else if metadata.is_file() {
            if !allowed_files.contains(relative) {
                return Err(WorkVcsError::QueryInvalid(format!(
                    "semantic model tree contains unexpected file {}",
                    path.display()
                )));
            }
            require_single_link(&metadata, &path)?;
        } else {
            return Err(WorkVcsError::QueryInvalid(format!(
                "semantic model tree member {} must be a regular file",
                path.display()
            )));
        }
    }
    for required in allowed_files {
        if current == root && required.parent().is_none() && !root.join(required).exists() {
            return Err(WorkVcsError::QueryInvalid(format!(
                "semantic model tree is missing required file {}",
                root.join(required).display()
            )));
        }
    }
    Ok(())
}

fn copy_verified_regular_file(
    source: &Path,
    destination: &Path,
    expected_sha256: &str,
) -> Result<(String, u64)> {
    let mut input = open_regular_nofollow(source)?;
    let parent = destination.parent().ok_or_else(|| {
        WorkVcsError::StorageFailure("model staging file has no parent".to_owned())
    })?;
    if !parent.exists() {
        fs::create_dir(parent).map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "cannot create model staging subdirectory {}: {error}",
                parent.display()
            ))
        })?;
        set_private_directory_permissions(parent)?;
    }
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "cannot create model staging file {}: {error}",
                destination.display()
            ))
        })?;
    let mut hasher = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = input.read(&mut buffer).map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "cannot read model source file {}: {error}",
                source.display()
            ))
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        output.write_all(&buffer[..read]).map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "cannot write model staging file {}: {error}",
                destination.display()
            ))
        })?;
        total = total.saturating_add(read as u64);
    }
    output.sync_all().map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "cannot sync model staging file {}: {error}",
            destination.display()
        ))
    })?;
    let actual = hex::encode(hasher.finalize());
    if actual != expected_sha256 {
        return Err(WorkVcsError::QueryInvalid(format!(
            "semantic model source {} SHA-256 mismatch: expected {}, found {}",
            source.display(),
            expected_sha256,
            actual
        )));
    }
    let metadata = output.metadata().map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "cannot inspect model staging file {}: {error}",
            destination.display()
        ))
    })?;
    require_single_link(&metadata, destination)?;
    Ok((actual, total))
}

fn hash_regular_file(path: &Path) -> Result<(String, u64)> {
    let mut file = open_regular_nofollow(path)?;
    let mut hasher = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            WorkVcsError::StorageFailure(format!("cannot read {}: {error}", path.display()))
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        total = total.saturating_add(read as u64);
    }
    Ok((hex::encode(hasher.finalize()), total))
}

pub(super) fn read_regular_file(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let path = path.as_ref();
    let mut file = open_regular_nofollow(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|error| {
        WorkVcsError::StorageFailure(format!("cannot read {}: {error}", path.display()))
    })?;
    Ok(bytes)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn open_regular_nofollow(path: &Path) -> Result<File> {
    use rustix::fs::{Mode, OFlags, open};
    let fd = open(
        path,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::empty(),
    )
    .map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "cannot open regular model file {} without following links: {error}",
            path.display()
        ))
    })?;
    let file = File::from(fd);
    let metadata = file.metadata().map_err(|error| {
        WorkVcsError::StorageFailure(format!("cannot inspect {}: {error}", path.display()))
    })?;
    if !metadata.is_file() {
        return Err(WorkVcsError::QueryInvalid(format!(
            "semantic model member {} is not a regular file",
            path.display()
        )));
    }
    require_single_link(&metadata, path)?;
    Ok(file)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn open_regular_nofollow(_path: &Path) -> Result<File> {
    ensure_supported_platform().and_then(|_| unreachable!())
}

#[cfg(unix)]
fn require_single_link(metadata: &fs::Metadata, path: &Path) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    if metadata.nlink() != 1 {
        return Err(WorkVcsError::QueryInvalid(format!(
            "semantic model file {} must have exactly one hard link",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(not(unix))]
fn require_single_link(_metadata: &fs::Metadata, _path: &Path) -> Result<()> {
    ensure_supported_platform()
}

fn write_json_file_new(path: &Path, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!("cannot encode JSON: {error}"))
    })?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            WorkVcsError::StorageFailure(format!("cannot create {}: {error}", path.display()))
        })?;
    file.write_all(&bytes).map_err(|error| {
        WorkVcsError::StorageFailure(format!("cannot write {}: {error}", path.display()))
    })?;
    file.sync_all().map_err(|error| {
        WorkVcsError::StorageFailure(format!("cannot sync {}: {error}", path.display()))
    })
}

fn canonical_source(source: &Path) -> Result<PathBuf> {
    if !source.is_absolute()
        || source
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(WorkVcsError::QueryInvalid(
            "semantic model source path must be absolute without dot or parent components"
                .to_owned(),
        ));
    }
    fs::canonicalize(source).map_err(|error| {
        WorkVcsError::QueryInvalid(format!(
            "semantic model source {} is unavailable: {error}",
            source.display()
        ))
    })
}

fn validate_destination(destination: &Path, source: &Path) -> Result<PathBuf> {
    if !destination.is_absolute()
        || destination
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(WorkVcsError::QueryInvalid(
            "semantic model destination must be absolute without dot or parent components"
                .to_owned(),
        ));
    }
    let parent = destination.parent().ok_or_else(|| {
        WorkVcsError::QueryInvalid("semantic model destination has no parent".to_owned())
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "cannot create semantic model destination parent {}: {error}",
            parent.display()
        ))
    })?;
    let parent = fs::canonicalize(parent).map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "cannot canonicalize semantic model destination parent {}: {error}",
            parent.display()
        ))
    })?;
    let file_name = destination.file_name().ok_or_else(|| {
        WorkVcsError::QueryInvalid("semantic model destination has no final name".to_owned())
    })?;
    let destination = parent.join(file_name);
    if destination.starts_with(source) || source.starts_with(&destination) {
        return Err(WorkVcsError::QueryInvalid(
            "semantic model source and destination must not contain each other".to_owned(),
        ));
    }
    Ok(destination)
}

fn sync_directory(path: &Path) -> Result<()> {
    let directory = File::open(path).map_err(|error| {
        WorkVcsError::StorageFailure(format!("cannot open {} for sync: {error}", path.display()))
    })?;
    directory.sync_all().map_err(|error| {
        WorkVcsError::StorageFailure(format!("cannot sync {}: {error}", path.display()))
    })
}

#[cfg(unix)]
fn set_private_directory_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "cannot set private permissions on {}: {error}",
            path.display()
        ))
    })
}

#[cfg(not(unix))]
fn set_private_directory_permissions(_path: &Path) -> Result<()> {
    ensure_supported_platform()
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn ensure_supported_platform() -> Result<()> {
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn ensure_supported_platform() -> Result<()> {
    Err(WorkVcsError::QueryUnsupported(
        "semantic model provisioning requires no-follow opens, Unix file identity, and atomic no-replace directory publication"
            .to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use onnx_protobuf::StringStringEntryProto;

    #[test]
    fn external_data_path_rejects_parent_traversal() {
        let mut tensor = TensorProto::new();
        tensor.name = "weights".to_owned();
        let mut entry = StringStringEntryProto::new();
        entry.key = "location".to_owned();
        entry.value = "../model.onnx_data".to_owned();
        tensor.external_data.push(entry);
        let declared = BTreeSet::from(["model.onnx_data".to_owned()]);
        let mut referenced = BTreeSet::new();
        assert!(visit_tensor(&tensor, &declared, &mut referenced).is_err());
    }

    #[test]
    fn external_data_path_requires_declared_single_filename() {
        let mut tensor = TensorProto::new();
        tensor.name = "weights".to_owned();
        let mut entry = StringStringEntryProto::new();
        entry.key = "location".to_owned();
        entry.value = "model.onnx_data".to_owned();
        tensor.external_data.push(entry);
        let declared = BTreeSet::from(["model.onnx_data".to_owned()]);
        let mut referenced = BTreeSet::new();
        visit_tensor(&tensor, &declared, &mut referenced).expect("valid external data");
        assert_eq!(referenced, declared);
    }
}
