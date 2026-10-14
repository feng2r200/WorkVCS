use super::{SearchSourceIdentity, search_lexical_terms, search_o200k_ordinary_token_count};
use crate::error::{Result, WorkVcsError};
use serde::{Deserialize, Serialize};
use sha2::{Digest as ShaDigest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

#[cfg(unix)]
use std::ffi::{CString, OsStr};
#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};

const LEGACY_ARCHIVE_FORMAT_VERSION: u32 = 1;
const AUTOMATIC_ARCHIVE_FORMAT_VERSION: u32 = 2;
const LEGACY_EXTRACTION_POLICY_VERSION: &str = "codex-rollout-user-assistant-text-v1";
const AUTOMATIC_EXTRACTION_POLICY_VERSION: &str = "codex-rollout-snapshot-occurrence-v2";
const AUTOMATIC_ATTRIBUTION_VERSION: u32 = 1;
const AUTOMATIC_ATTRIBUTION_SCHEME: &str = "registry_verified_snapshot_v1";
const DERIVED_PROVENANCE_CLASS: &str = "derived_only";
const LOCATOR_AUTHORITY: &str = "none";
const MANIFEST_FILE: &str = "manifest.json";
const DOCUMENTS_FILE: &str = "documents.json";
const AUTHORITY_CLASS: &str = "historical_reference_only";
const CURRENTNESS_CLASS: &str = "unknown_historical";
const ACTION_AUTHORITY: &str = "none";
const MAX_PROJECT_REGISTRY_BYTES: u64 = 16 * 1024 * 1024;
const MAX_ROUTING_ACTIVATION_BYTES: u64 = 64 * 1024;
const MAX_NATIVE_METADATA_BYTES: u64 = 16 * 1024;
const MAX_ARCHIVE_MANIFEST_BYTES: u64 = 16 * 1024 * 1024;
const MAX_ARCHIVE_DOCUMENT_BYTES: u64 = 256 * 1024 * 1024;
const STANDARD_PROJECT_REGISTRY_FILE: &str = "project-bindings.json";
const ROUTING_ACTIVATION_FILE: &str = "routing-activation-v1.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceArchiveFormat {
    CodexRolloutJsonl,
}

#[derive(Clone, Debug)]
struct ParsedRolloutLine {
    source_line: usize,
    value: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceArchiveSnapshotSessionMeta {
    pub source_line: usize,
    pub thread_id: Option<String>,
    pub raw_cwd: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SourceArchiveSnapshot {
    source_root: PathBuf,
    source_path: PathBuf,
    source_sha256: String,
    source_bytes: u64,
    opened_identity: SourceArchiveOpenedIdentity,
    bytes: Arc<[u8]>,
    parsed: Arc<[ParsedRolloutLine]>,
    observed_session_meta: Arc<[SourceArchiveSnapshotSessionMeta]>,
}

#[derive(Clone, Debug)]
pub struct SourceArchiveLiveSourceFence {
    source_root: PathBuf,
    source_path: PathBuf,
    source_sha256: String,
    source_bytes: u64,
    opened_identity: SourceArchiveOpenedIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SourceArchiveOpenedIdentity {
    device: u64,
    inode: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    length: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

impl SourceArchiveSnapshot {
    pub fn capture(source_root: impl AsRef<Path>, source: impl AsRef<Path>) -> Result<Self> {
        Self::capture_with_hook(source_root.as_ref(), source.as_ref(), |_| {})
    }

    fn capture_with_hook(
        source_root: &Path,
        source: &Path,
        mut before_component_open: impl FnMut(&Path),
    ) -> Result<Self> {
        require_absolute_clean_path(source_root, "source policy root")?;
        require_absolute_clean_path(source, "source path")?;
        require_jsonl(source)?;
        let relative = source.strip_prefix(source_root).map_err(|_| {
            WorkVcsError::IntegrityInvalid(format!(
                "source {} is outside registered source policy root {}",
                source.display(),
                source_root.display()
            ))
        })?;
        if relative.as_os_str().is_empty()
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "source {} is not a normal file path beneath registered source policy root {}",
                source.display(),
                source_root.display()
            )));
        }
        #[cfg(unix)]
        let (mut file, opened_identity) =
            open_source_beneath_root(source_root, relative, &mut before_component_open)?;
        #[cfg(not(unix))]
        let (mut file, opened_identity) = {
            let _ = before_component_open;
            return Err(WorkVcsError::ControlPlaneInvalid(
                "automatic source archive capture requires no-follow directory handles on this platform"
                    .to_owned(),
            ));
        };
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(storage_error("read source snapshot", source))?;
        let after_read = opened_identity_from_metadata(
            &file
                .metadata()
                .map_err(storage_error("inspect opened source snapshot", source))?,
        );
        if after_read != opened_identity || after_read.length != bytes.len() as u64 {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "source {} changed while its immutable snapshot was being captured",
                source.display()
            )));
        }
        let source_bytes = u64::try_from(bytes.len()).map_err(|_| {
            WorkVcsError::QueryInvalid("source rollout size does not fit u64".to_owned())
        })?;
        let source_sha256 = sha256_bytes(&bytes);
        let mut parsed = Vec::new();
        let mut observed_session_meta = Vec::new();
        for (offset, line) in bytes.split(|byte| *byte == b'\n').enumerate() {
            if line.is_empty() {
                continue;
            }
            let source_line = offset + 1;
            let value: serde_json::Value = serde_json::from_slice(line).map_err(|error| {
                WorkVcsError::QueryInvalid(format!(
                    "source rollout {} line {} is invalid JSON: {error}",
                    source.display(),
                    source_line
                ))
            })?;
            if value.get("type").and_then(serde_json::Value::as_str) == Some("session_meta") {
                observed_session_meta.push(SourceArchiveSnapshotSessionMeta {
                    source_line,
                    thread_id: value
                        .get("payload")
                        .and_then(|payload| payload.get("id"))
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned),
                    raw_cwd: value
                        .get("payload")
                        .and_then(|payload| payload.get("cwd"))
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned),
                });
            }
            parsed.push(ParsedRolloutLine { source_line, value });
        }
        Ok(Self {
            source_root: source_root.to_path_buf(),
            source_path: source.to_path_buf(),
            source_sha256,
            source_bytes,
            opened_identity,
            bytes: bytes.into(),
            parsed: parsed.into(),
            observed_session_meta: observed_session_meta.into(),
        })
    }

    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }

    pub fn source_bytes(&self) -> u64 {
        self.source_bytes
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn observed_session_meta(&self) -> &[SourceArchiveSnapshotSessionMeta] {
        &self.observed_session_meta
    }

    pub fn live_source_fence(&self) -> SourceArchiveLiveSourceFence {
        SourceArchiveLiveSourceFence {
            source_root: self.source_root.clone(),
            source_path: self.source_path.clone(),
            source_sha256: self.source_sha256.clone(),
            source_bytes: self.source_bytes,
            opened_identity: self.opened_identity.clone(),
        }
    }

    pub fn verify_live_source(&self) -> Result<bool> {
        self.live_source_fence().verify_live_source()
    }
}

impl SourceArchiveLiveSourceFence {
    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }

    pub fn source_bytes(&self) -> u64 {
        self.source_bytes
    }

    pub fn verify_live_source(&self) -> Result<bool> {
        #[cfg(unix)]
        {
            let relative = self
                .source_path
                .strip_prefix(&self.source_root)
                .map_err(|_| {
                    WorkVcsError::IntegrityInvalid(format!(
                        "source {} is outside registered source policy root {}",
                        self.source_path.display(),
                        self.source_root.display()
                    ))
                })?;
            let (mut file, opened_identity) =
                open_source_beneath_root(&self.source_root, relative, &mut |_| {})?;
            if opened_identity != self.opened_identity {
                return Ok(false);
            }
            let mut hasher = Sha256::new();
            let mut source_bytes = 0_u64;
            let mut buffer = [0_u8; 64 * 1024];
            loop {
                let read = file
                    .read(&mut buffer)
                    .map_err(storage_error("verify live source", &self.source_path))?;
                if read == 0 {
                    break;
                }
                source_bytes = source_bytes.checked_add(read as u64).ok_or_else(|| {
                    WorkVcsError::IntegrityInvalid(format!(
                        "source {} size overflowed while verifying its publication fence",
                        self.source_path.display()
                    ))
                })?;
                hasher.update(&buffer[..read]);
            }
            let after_read = opened_identity_from_metadata(&file.metadata().map_err(
                storage_error("reinspect live source publication fence", &self.source_path),
            )?);
            if after_read != opened_identity || source_bytes != opened_identity.length {
                return Ok(false);
            }
            let source_sha256 = hex::encode(hasher.finalize());
            Ok(source_bytes == self.source_bytes && source_sha256 == self.source_sha256)
        }
        #[cfg(not(unix))]
        {
            Err(WorkVcsError::ControlPlaneInvalid(
                "automatic source archive live verification requires no-follow directory handles on this platform"
                    .to_owned(),
            ))
        }
    }
}

#[cfg(unix)]
fn open_source_beneath_root(
    source_root: &Path,
    relative: &Path,
    before_component_open: &mut impl FnMut(&Path),
) -> Result<(fs::File, SourceArchiveOpenedIdentity)> {
    let mut directory = open_absolute_directory_without_symlinks(source_root)?;
    let root_metadata = directory.metadata().map_err(storage_error(
        "inspect opened source policy root",
        source_root,
    ))?;
    validate_snapshot_member_metadata(&root_metadata, source_root, true)?;
    let mut cursor = source_root.to_path_buf();
    let components = relative.components().collect::<Vec<_>>();
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(name) = component else {
            return Err(WorkVcsError::IntegrityInvalid(
                "source path below policy root contains a non-normal component".to_owned(),
            ));
        };
        cursor.push(name);
        let expected = lstatat(directory.as_raw_fd(), name, &cursor)?;
        before_component_open(&cursor);
        let final_component = index + 1 == components.len();
        let flags = libc::O_RDONLY
            | libc::O_CLOEXEC
            | libc::O_NOFOLLOW
            | libc::O_NONBLOCK
            | if final_component {
                0
            } else {
                libc::O_DIRECTORY
            };
        let fd = openat(directory.as_raw_fd(), name, flags, &cursor)?;
        let opened = unsafe { fs::File::from_raw_fd(fd) };
        let metadata = opened.metadata().map_err(storage_error(
            "inspect opened source path component",
            &cursor,
        ))?;
        let actual = opened_identity_from_metadata(&metadata);
        if actual.device != expected.device
            || actual.inode != expected.inode
            || actual.mode != expected.mode
        {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "source path component {} changed between no-follow inspection and open",
                cursor.display()
            )));
        }
        validate_snapshot_member_metadata(&metadata, &cursor, !final_component)?;
        if final_component {
            return Ok((opened, actual));
        }
        directory = opened;
    }
    Err(WorkVcsError::IntegrityInvalid(
        "source path below policy root is empty".to_owned(),
    ))
}

#[cfg(unix)]
fn open_absolute_directory_without_symlinks(path: &Path) -> Result<fs::File> {
    let mut directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(Path::new("/"))
        .map_err(storage_error(
            "open filesystem root for source policy",
            Path::new("/"),
        ))?;
    let mut cursor = PathBuf::from("/");
    for component in path.components() {
        match component {
            Component::RootDir => continue,
            Component::Normal(name) => {
                cursor.push(name);
                let expected = lstatat(directory.as_raw_fd(), name, &cursor)?;
                let fd = openat(
                    directory.as_raw_fd(),
                    name,
                    libc::O_RDONLY
                        | libc::O_CLOEXEC
                        | libc::O_NOFOLLOW
                        | libc::O_NONBLOCK
                        | libc::O_DIRECTORY,
                    &cursor,
                )?;
                let opened = unsafe { fs::File::from_raw_fd(fd) };
                let actual = opened_identity_from_metadata(&opened.metadata().map_err(
                    storage_error("inspect source policy path component", &cursor),
                )?);
                if actual.device != expected.device
                    || actual.inode != expected.inode
                    || actual.mode != expected.mode
                {
                    return Err(WorkVcsError::IntegrityInvalid(format!(
                        "source policy path component {} changed between no-follow inspection and open",
                        cursor.display()
                    )));
                }
                directory = opened;
            }
            _ => {
                return Err(WorkVcsError::IntegrityInvalid(format!(
                    "source policy root {} contains a non-normal component",
                    path.display()
                )));
            }
        }
    }
    Ok(directory)
}

#[cfg(unix)]
fn lstatat(directory: i32, name: &OsStr, path: &Path) -> Result<SourceArchiveOpenedIdentity> {
    let name = CString::new(name.as_bytes()).map_err(|_| {
        WorkVcsError::IntegrityInvalid(format!(
            "source path component {} contains a NUL byte",
            path.display()
        ))
    })?;
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            directory,
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        return Err(storage_error("inspect source path component", path)(
            std::io::Error::last_os_error(),
        ));
    }
    let stat = unsafe { stat.assume_init() };
    Ok(opened_identity_from_stat(&stat))
}

#[cfg(unix)]
fn openat(directory: i32, name: &OsStr, flags: i32, path: &Path) -> Result<i32> {
    let name = CString::new(name.as_bytes()).map_err(|_| {
        WorkVcsError::IntegrityInvalid(format!(
            "source path component {} contains a NUL byte",
            path.display()
        ))
    })?;
    let fd = unsafe { libc::openat(directory, name.as_ptr(), flags) };
    if fd < 0 {
        let error = std::io::Error::last_os_error();
        if matches!(
            error.raw_os_error(),
            Some(libc::ELOOP) | Some(libc::ENOTDIR)
        ) {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "source path component {} is a symlink or has an unsafe filesystem type",
                path.display()
            )));
        }
        return Err(storage_error("open source path component", path)(error));
    }
    Ok(fd)
}

#[cfg(unix)]
fn validate_snapshot_member_metadata(
    metadata: &fs::Metadata,
    path: &Path,
    directory: bool,
) -> Result<()> {
    if directory != metadata.is_dir()
        || (!directory && (!metadata.is_file() || metadata.file_type().is_symlink()))
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o022 != 0
    {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "source snapshot path {} has an unsafe type, owner, or write permission",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn opened_identity_from_metadata(metadata: &fs::Metadata) -> SourceArchiveOpenedIdentity {
    SourceArchiveOpenedIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        mode: metadata.mode(),
        uid: metadata.uid(),
        gid: metadata.gid(),
        length: metadata.len(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    }
}

#[cfg(unix)]
fn portable_unix_mode(mode: libc::mode_t) -> u32 {
    // Darwin mode_t is u16, while Linux mode_t is already u32.
    #[allow(clippy::useless_conversion)]
    let mode = u32::from(mode);
    mode
}

#[cfg(unix)]
fn opened_identity_from_stat(metadata: &libc::stat) -> SourceArchiveOpenedIdentity {
    // Darwin dev_t is narrower than the device identity returned by MetadataExt.
    #[allow(clippy::unnecessary_cast)]
    let device = metadata.st_dev as u64;
    SourceArchiveOpenedIdentity {
        device,
        inode: metadata.st_ino,
        mode: portable_unix_mode(metadata.st_mode),
        uid: metadata.st_uid,
        gid: metadata.st_gid,
        length: metadata.st_size as u64,
        modified_seconds: 0,
        modified_nanoseconds: 0,
        changed_seconds: 0,
        changed_nanoseconds: 0,
    }
}

fn read_live_control_file(path: &Path, max_bytes: u64, label: &str) -> Result<Vec<u8>> {
    read_live_regular_file(path, max_bytes, label, true)
}

#[cfg(unix)]
fn read_live_regular_file(
    path: &Path,
    max_bytes: u64,
    label: &str,
    require_private: bool,
) -> Result<Vec<u8>> {
    require_absolute_clean_path(path, label)?;
    let parent = path.parent().ok_or_else(|| {
        WorkVcsError::IntegrityInvalid(format!("{label} {} has no parent", path.display()))
    })?;
    let name = path.file_name().ok_or_else(|| {
        WorkVcsError::IntegrityInvalid(format!("{label} {} has no file name", path.display()))
    })?;
    let directory = open_absolute_directory_without_symlinks(parent)?;
    let expected = lstatat(directory.as_raw_fd(), name, path)?;
    let fd = openat(
        directory.as_raw_fd(),
        name,
        libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        path,
    )?;
    let mut file = unsafe { fs::File::from_raw_fd(fd) };
    let metadata = file
        .metadata()
        .map_err(storage_error("inspect live regular file", path))?;
    let opened = opened_identity_from_metadata(&metadata);
    if opened.device != expected.device
        || opened.inode != expected.inode
        || opened.mode != expected.mode
        || !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > max_bytes
        || (require_private
            && (metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o022 != 0))
    {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "{label} {} has an unsafe identity, type, ownership, permission, or size",
            path.display()
        )));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut bytes)
        .map_err(storage_error("read live regular file", path))?;
    let after = opened_identity_from_metadata(
        &file
            .metadata()
            .map_err(storage_error("reinspect live regular file", path))?,
    );
    if after != opened || bytes.len() as u64 != opened.length {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "{label} {} changed while it was read",
            path.display()
        )));
    }
    Ok(bytes)
}

#[cfg(not(unix))]
fn read_live_regular_file(
    path: &Path,
    _max_bytes: u64,
    label: &str,
    _require_private: bool,
) -> Result<Vec<u8>> {
    Err(WorkVcsError::ControlPlaneInvalid(format!(
        "live {label} verification for {} requires Unix no-follow directory handles",
        path.display()
    )))
}

#[cfg(not(unix))]
fn opened_identity_from_metadata(metadata: &fs::Metadata) -> SourceArchiveOpenedIdentity {
    SourceArchiveOpenedIdentity {
        device: 0,
        inode: 0,
        mode: 0,
        uid: 0,
        gid: 0,
        length: metadata.len(),
        modified_seconds: 0,
        modified_nanoseconds: 0,
        changed_seconds: 0,
        changed_nanoseconds: 0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceArchiveRolePolicy {
    UserOnly,
    UserAndAssistant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceArchiveOwnerResolutionScheme {
    NativeIdentityV1,
    SafeAliasV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceArchiveAliasFingerprint {
    pub anchor: PathBuf,
    pub link_path: PathBuf,
    pub link_target: PathBuf,
    pub link_target_sha256: String,
    pub device: u64,
    pub inode: u64,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub length: u64,
    pub modified_seconds: i64,
    pub modified_nanoseconds: i64,
    pub changed_seconds: i64,
    pub changed_nanoseconds: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceArchiveOwnerResolutionEvidence {
    pub scheme: SourceArchiveOwnerResolutionScheme,
    pub normalized_cwd: PathBuf,
    pub candidate_cwd: PathBuf,
    pub boundary: PathBuf,
    pub probe_cwd: PathBuf,
    pub native_identity_kind: String,
    pub native_identity: String,
    pub native_identity_path: PathBuf,
    pub native_metadata_basis: SourceArchiveNativeMetadataBasis,
    pub native_metadata_sha256: String,
    pub alias: Option<SourceArchiveAliasFingerprint>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceArchiveNativeMetadataComponent {
    pub label: String,
    pub path: PathBuf,
    pub bytes: u64,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_target: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceArchiveNativeMetadataBasis {
    pub identity_root: PathBuf,
    pub marker_kind: String,
    pub marker_path: Option<PathBuf>,
    pub git_admin_path: Option<PathBuf>,
    pub components: Vec<SourceArchiveNativeMetadataComponent>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceArchiveAttributedSessionMeta {
    pub source_line: usize,
    pub thread_id: String,
    pub raw_cwd: PathBuf,
    pub owner_resolution: SourceArchiveOwnerResolutionEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceArchiveRegistryBasis {
    pub registry_id: String,
    pub registry_revision: u64,
    pub registry_digest: String,
    pub registry_snapshot_json: String,
    pub member_native_owners: Vec<SourceArchiveNativeOwner>,
}

#[derive(Clone, Debug, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceArchiveNativeOwner {
    pub identity_kind: String,
    pub identity_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceArchivePolicyBasis {
    pub source_id: String,
    pub source_entry_revision: u64,
    pub source_policy_format_version: u32,
    pub source_policy_revision: u64,
    pub source_policy_digest_sha256: String,
    pub source_policy_entries: Vec<crate::automation::AutomationSourcePolicyEntry>,
    pub role_policy: SourceArchiveRolePolicy,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceArchiveAutomaticAttribution {
    pub attribution_version: u32,
    pub resolution_scheme: String,
    pub provenance_class: String,
    pub locator_authority: String,
    pub source_snapshot_sha256: String,
    pub member_identity: SearchSourceIdentity,
    pub registry_basis: SourceArchiveRegistryBasis,
    pub source_policy_basis: SourceArchivePolicyBasis,
    pub observed_session_meta: Vec<SourceArchiveAttributedSessionMeta>,
    pub observed_at_us: u128,
}

#[derive(Clone, Debug)]
struct VerifiedSourceArchiveAutomaticAttribution {
    attribution: SourceArchiveAutomaticAttribution,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceArchiveRegistryRoute {
    Home(PathBuf),
    RegistryOnly(PathBuf),
}

impl SourceArchiveRegistryRoute {
    fn registry_path(&self) -> PathBuf {
        match self {
            Self::Home(home) => home.join(STANDARD_PROJECT_REGISTRY_FILE),
            Self::RegistryOnly(path) => path.clone(),
        }
    }

    fn routing_activation_path(&self, registry_path: &Path) -> PathBuf {
        match self {
            Self::Home(home) => home.join(ROUTING_ACTIVATION_FILE),
            Self::RegistryOnly(_) => PathBuf::from(format!("{}.d", registry_path.display()))
                .join(ROUTING_ACTIVATION_FILE),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceArchiveAutomationRoot {
    DefaultForRegistry,
    Explicit(PathBuf),
}

/// The verified publisher accepts only route/root selectors and a captured snapshot.
/// Derived activation paths and owner receipts cannot be supplied through the public API.
///
/// Legal one-shot requests and the stable explicit v1 constructor remain available:
///
/// ```no_run
/// use std::path::PathBuf;
/// use workvcs_core::{
///     Engine, SearchSourceIdentity, SourceArchiveAutomationRoot, SourceArchiveImportOptions,
///     SourceArchiveRegistryRoute, SourceArchiveRegistryVerifiedImportRequest,
///     SourceArchiveSnapshot,
/// };
/// fn public_controls(snapshot: SourceArchiveSnapshot, identity: SearchSourceIdentity) {
///     let home = SourceArchiveRegistryVerifiedImportRequest::new(
///         SourceArchiveRegistryRoute::Home(PathBuf::from("/tmp/home")),
///         SourceArchiveAutomationRoot::DefaultForRegistry,
///         "/tmp/archive-home",
///         snapshot.clone(),
///         "codex-rollouts",
///     ).unwrap();
///     let registry = SourceArchiveRegistryVerifiedImportRequest::new(
///         SourceArchiveRegistryRoute::RegistryOnly(PathBuf::from("/tmp/registry.json")),
///         SourceArchiveAutomationRoot::Explicit(PathBuf::from("/tmp/automation")),
///         "/tmp/archive-registry",
///         snapshot,
///         "codex-rollouts",
///     ).unwrap();
///     let _ = Engine::import_registry_verified_source_archive(home);
///     let _ = Engine::import_registry_verified_source_archive(registry);
///     let _ = SourceArchiveImportOptions::new(
///         identity,
///         "/tmp/explicit-archive",
///         vec![PathBuf::from("/tmp/rollout.jsonl")],
///     ).unwrap();
/// }
/// ```
///
/// A caller cannot access an activation path on a publicly constructed request:
///
/// ```compile_fail
/// use std::path::PathBuf;
/// use workvcs_core::{
///     SourceArchiveAutomationRoot, SourceArchiveRegistryRoute,
///     SourceArchiveRegistryVerifiedImportRequest, SourceArchiveSnapshot,
/// };
/// fn inject(snapshot: SourceArchiveSnapshot) {
///     let request = SourceArchiveRegistryVerifiedImportRequest::new(
///         SourceArchiveRegistryRoute::Home(PathBuf::from("/tmp/home")),
///         SourceArchiveAutomationRoot::DefaultForRegistry,
///         "/tmp/archive",
///         snapshot,
///         "codex-rollouts",
///     ).unwrap();
///     let _ = request.routing_activation_path;
/// }
/// ```
///
/// A caller cannot access a derived member identity on a publicly constructed request:
///
/// ```compile_fail
/// use std::path::PathBuf;
/// use workvcs_core::{
///     SourceArchiveAutomationRoot, SourceArchiveRegistryRoute,
///     SourceArchiveRegistryVerifiedImportRequest, SourceArchiveSnapshot,
/// };
/// fn inject(snapshot: SourceArchiveSnapshot) {
///     let request = SourceArchiveRegistryVerifiedImportRequest::new(
///         SourceArchiveRegistryRoute::RegistryOnly(PathBuf::from("/tmp/registry.json")),
///         SourceArchiveAutomationRoot::Explicit(PathBuf::from("/tmp/automation")),
///         "/tmp/archive",
///         snapshot,
///         "codex-rollouts",
///     ).unwrap();
///     let _ = request.member_identity;
/// }
/// ```
///
/// A caller cannot access an owner receipt on a publicly constructed request:
///
/// ```compile_fail
/// use std::path::PathBuf;
/// use workvcs_core::{
///     SourceArchiveAutomationRoot, SourceArchiveRegistryRoute,
///     SourceArchiveRegistryVerifiedImportRequest, SourceArchiveSnapshot,
/// };
/// fn inject(snapshot: SourceArchiveSnapshot) {
///     let request = SourceArchiveRegistryVerifiedImportRequest::new(
///         SourceArchiveRegistryRoute::Home(PathBuf::from("/tmp/home")),
///         SourceArchiveAutomationRoot::DefaultForRegistry,
///         "/tmp/archive",
///         snapshot,
///         "codex-rollouts",
///     ).unwrap();
///     let _ = request.owner_receipt;
/// }
/// ```
///
/// The core issuer is private to the crate:
///
/// ```compile_fail
/// let _ = workvcs_core::search::source_archive::issue_live_automatic_attribution;
/// ```
///
/// A public automatic receipt cannot be inserted into explicit import options:
///
/// ```compile_fail
/// use std::path::PathBuf;
/// use workvcs_core::{
///     SearchSourceIdentity, SourceArchiveAutomaticAttribution, SourceArchiveImportOptions,
/// };
/// fn inject(receipt: SourceArchiveAutomaticAttribution, identity: SearchSourceIdentity) {
///     let mut options = SourceArchiveImportOptions::new(
///         identity,
///         "/tmp/archive",
///         vec![PathBuf::from("/tmp/rollout.jsonl")],
///     ).unwrap();
///     options.automatic_attribution = Some(receipt);
/// }
/// ```
#[derive(Clone, Debug)]
pub struct SourceArchiveRegistryVerifiedImportRequest {
    registry_route: SourceArchiveRegistryRoute,
    automation_root: SourceArchiveAutomationRoot,
    archive_root: PathBuf,
    snapshot: SourceArchiveSnapshot,
    source_id: String,
}

impl SourceArchiveRegistryVerifiedImportRequest {
    pub fn new(
        registry_route: SourceArchiveRegistryRoute,
        automation_root: SourceArchiveAutomationRoot,
        archive_root: impl Into<PathBuf>,
        snapshot: SourceArchiveSnapshot,
        source_id: impl Into<String>,
    ) -> Result<Self> {
        let source_id = source_id.into();
        if source_id.is_empty() {
            return Err(WorkVcsError::QueryInvalid(
                "registry-verified source import requires a source id".to_owned(),
            ));
        }
        Ok(Self {
            registry_route,
            automation_root,
            archive_root: archive_root.into(),
            snapshot,
            source_id,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SourceArchiveRegistryVerifiedImportResult {
    pub member_identity: SearchSourceIdentity,
    pub import: SourceArchiveImportResult,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceArchiveRegistryVerifiedOwnerResolution {
    pub member_identity: SearchSourceIdentity,
    pub observed_session_meta: Vec<SourceArchiveAttributedSessionMeta>,
}

impl SourceArchiveRegistryVerifiedOwnerResolution {
    /// Re-observe every filesystem identity used by this resolution.
    ///
    /// Reconciliation callers use this immediately before publication so an
    /// alias retarget, Git marker replacement, or metadata change after the
    /// initial owner lookup cannot silently retain the earlier attribution.
    pub fn revalidate_live_evidence(&self) -> Result<()> {
        if self.observed_session_meta.is_empty() {
            return Err(WorkVcsError::IntegrityInvalid(
                "registry-verified owner resolution has no observed session metadata".to_owned(),
            ));
        }
        for metadata in &self.observed_session_meta {
            validate_live_owner_resolution_evidence(&metadata.owner_resolution)?;
        }
        Ok(())
    }
}

impl SourceArchiveAutomaticAttribution {
    fn issue_verified(
        source_snapshot_sha256: impl Into<String>,
        member_identity: SearchSourceIdentity,
        registry_basis: SourceArchiveRegistryBasis,
        source_policy_basis: SourceArchivePolicyBasis,
        observed_session_meta: Vec<SourceArchiveAttributedSessionMeta>,
        observed_at_us: u128,
    ) -> Result<VerifiedSourceArchiveAutomaticAttribution> {
        let attribution = Self {
            attribution_version: AUTOMATIC_ATTRIBUTION_VERSION,
            resolution_scheme: AUTOMATIC_ATTRIBUTION_SCHEME.to_owned(),
            provenance_class: DERIVED_PROVENANCE_CLASS.to_owned(),
            locator_authority: LOCATOR_AUTHORITY.to_owned(),
            source_snapshot_sha256: source_snapshot_sha256.into(),
            member_identity,
            registry_basis,
            source_policy_basis,
            observed_session_meta,
            observed_at_us,
        };
        validate_automatic_attribution_evidence(&attribution)?;
        Ok(VerifiedSourceArchiveAutomaticAttribution { attribution })
    }
}

#[derive(Clone, Debug)]
pub struct SourceArchiveImportOptions {
    identity: SearchSourceIdentity,
    archive_root: PathBuf,
    sources: Vec<PathBuf>,
    snapshots: Vec<SourceArchiveSnapshot>,
    include_assistant: bool,
    automatic_attribution: Option<SourceArchiveAutomaticAttribution>,
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
            snapshots: Vec::new(),
            include_assistant: true,
            automatic_attribution: None,
        })
    }

    fn from_snapshot(
        identity: SearchSourceIdentity,
        archive_root: impl Into<PathBuf>,
        snapshot: SourceArchiveSnapshot,
        attribution: VerifiedSourceArchiveAutomaticAttribution,
    ) -> Result<Self> {
        let attribution = attribution.attribution;
        let include_assistant = match attribution.source_policy_basis.role_policy {
            SourceArchiveRolePolicy::UserOnly => false,
            SourceArchiveRolePolicy::UserAndAssistant => true,
        };
        let options = Self {
            identity,
            archive_root: archive_root.into(),
            sources: Vec::new(),
            snapshots: vec![snapshot],
            include_assistant,
            automatic_attribution: Some(attribution),
        };
        validate_snapshot_attribution(&options)?;
        Ok(options)
    }

    pub fn include_assistant(mut self, include: bool) -> Self {
        self.include_assistant = include;
        self
    }
}

fn issue_live_automatic_attribution(
    request: &SourceArchiveRegistryVerifiedImportRequest,
) -> Result<(
    SearchSourceIdentity,
    VerifiedSourceArchiveAutomaticAttribution,
)> {
    let project_registry_path = request.registry_route.registry_path();
    require_absolute_clean_path(&project_registry_path, "ProjectRef registry path")?;
    let routing_activation_path = request
        .registry_route
        .routing_activation_path(&project_registry_path);
    require_absolute_clean_path(&routing_activation_path, "routing activation path")?;
    let automation_root = match &request.automation_root {
        SourceArchiveAutomationRoot::DefaultForRegistry => PathBuf::from(format!(
            "{}.d/automation/v1",
            project_registry_path.display()
        )),
        SourceArchiveAutomationRoot::Explicit(root) => root.clone(),
    };
    require_absolute_clean_path(&automation_root, "automation root")?;

    let registry_bytes = read_live_control_file(
        &project_registry_path,
        MAX_PROJECT_REGISTRY_BYTES,
        "ProjectRef registry",
    )?;
    let registry = crate::control_plane::ProjectRegistryV2::from_json_bytes(&registry_bytes)?;
    let canonical_registry_bytes = registry.stored_json_bytes()?;
    if registry_bytes != canonical_registry_bytes {
        return Err(WorkVcsError::IntegrityInvalid(
            "live ProjectRef registry bytes are not its canonical stored representation".to_owned(),
        ));
    }
    let activation_bytes = read_live_control_file(
        &routing_activation_path,
        MAX_ROUTING_ACTIVATION_BYTES,
        "routing activation",
    )?;
    let activation =
        crate::control_plane::RoutingActivationCandidate::from_json_bytes(&activation_bytes)?;
    activation.validate_registry(&registry)?;

    let policy = crate::automation::read_automation_source_policy(&automation_root)?;
    let selected = selected_source_policy_entry(&policy.sources, &request.source_id)?;
    let (identity, observed_session_meta) =
        resolve_snapshot_owner(&request.snapshot, &registry, &project_registry_path)?;
    if !selected.enabled
        || selected.revoked
        || selected
            .excluded_project_refs
            .binary_search(&identity.project_ref_id)
            .is_ok()
    {
        return Err(WorkVcsError::IntegrityInvalid(
            "live automatic source attribution selected an inactive, revoked, or excluded source policy entry"
                .to_owned(),
        ));
    }
    if request.snapshot.source_root != selected.rollout_root {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "captured source root {} does not equal selected source policy root {}",
            request.snapshot.source_root.display(),
            selected.rollout_root.display()
        )));
    }
    require_normal_descendant(
        &request.snapshot.source_path,
        &selected.rollout_root,
        "captured source path",
    )?;

    let mut member_native_owners = registry
        .locators()
        .iter()
        .filter(|locator| {
            locator.project_ref_id() == identity.project_ref_id
                && locator.role() == crate::control_plane::LocatorRole::Identity
                && locator.state() == crate::control_plane::LocatorState::Active
                && locator.authority() != crate::control_plane::LocatorAuthority::SemanticProject
        })
        .map(|locator| SourceArchiveNativeOwner {
            identity_kind: match locator.authority() {
                crate::control_plane::LocatorAuthority::Repository => "git-common-dir",
                crate::control_plane::LocatorAuthority::Cwd => "cwd",
                crate::control_plane::LocatorAuthority::SemanticProject => {
                    unreachable!("semantic locators were filtered")
                }
            }
            .to_owned(),
            identity_path: PathBuf::from(locator.normalized_value()),
        })
        .collect::<Vec<_>>();
    member_native_owners.sort();
    member_native_owners.dedup();

    for metadata in &observed_session_meta {
        validate_live_owner_resolution_evidence(&metadata.owner_resolution)?;
    }

    let role_policy = match selected.role_policy {
        crate::automation::AutomationRolePolicy::UserOnly => SourceArchiveRolePolicy::UserOnly,
        crate::automation::AutomationRolePolicy::UserAndAssistant => {
            SourceArchiveRolePolicy::UserAndAssistant
        }
    };
    let attribution = SourceArchiveAutomaticAttribution::issue_verified(
        request.snapshot.source_sha256(),
        identity,
        SourceArchiveRegistryBasis {
            registry_id: registry.registry_id().to_string(),
            registry_revision: registry.revision(),
            registry_digest: registry.digest()?.to_string(),
            registry_snapshot_json: String::from_utf8(canonical_registry_bytes).map_err(
                |error| {
                    WorkVcsError::CanonicalEncodingInvalid(format!(
                        "canonical ProjectRef registry snapshot is not UTF-8: {error}"
                    ))
                },
            )?,
            member_native_owners,
        },
        SourceArchivePolicyBasis {
            source_id: selected.source_id.clone(),
            source_entry_revision: selected.revision,
            source_policy_format_version: policy.format_version,
            source_policy_revision: policy.revision,
            source_policy_digest_sha256: policy.digest_sha256,
            source_policy_entries: policy.sources,
            role_policy,
        },
        observed_session_meta,
        u128::try_from(crate::store::current_epoch_micros()?).map_err(|_| {
            WorkVcsError::StorageFailure(
                "automatic attribution observation time is before the Unix epoch".to_owned(),
            )
        })?,
    )?;
    if attribution.attribution.member_identity != identity {
        return Err(WorkVcsError::IntegrityInvalid(
            "live automatic source attribution selected a different member".to_owned(),
        ));
    }
    Ok((identity, attribution))
}

pub(crate) fn import_registry_verified_source_archive(
    request: SourceArchiveRegistryVerifiedImportRequest,
) -> Result<SourceArchiveRegistryVerifiedImportResult> {
    require_absolute_clean_path(&request.archive_root, "source archive root")?;
    let (identity, attribution) = issue_live_automatic_attribution(&request)?;
    let options = SourceArchiveImportOptions::from_snapshot(
        identity,
        &request.archive_root,
        request.snapshot,
        attribution,
    )?;
    let import = import_source_archive(&options)?;
    Ok(SourceArchiveRegistryVerifiedImportResult {
        member_identity: identity,
        import,
    })
}

pub(crate) fn resolve_registry_verified_source_owner(
    route: SourceArchiveRegistryRoute,
    snapshot: &SourceArchiveSnapshot,
) -> Result<SourceArchiveRegistryVerifiedOwnerResolution> {
    let project_registry_path = route.registry_path();
    require_absolute_clean_path(&project_registry_path, "ProjectRef registry path")?;
    let routing_activation_path = route.routing_activation_path(&project_registry_path);
    require_absolute_clean_path(&routing_activation_path, "routing activation path")?;
    let registry_bytes = read_live_control_file(
        &project_registry_path,
        MAX_PROJECT_REGISTRY_BYTES,
        "ProjectRef registry",
    )?;
    let registry = crate::control_plane::ProjectRegistryV2::from_json_bytes(&registry_bytes)?;
    if registry_bytes != registry.stored_json_bytes()? {
        return Err(WorkVcsError::IntegrityInvalid(
            "live ProjectRef registry bytes are not its canonical stored representation".to_owned(),
        ));
    }
    let activation = crate::control_plane::RoutingActivationCandidate::from_json_bytes(
        &read_live_control_file(
            &routing_activation_path,
            MAX_ROUTING_ACTIVATION_BYTES,
            "routing activation",
        )?,
    )?;
    activation.validate_registry(&registry)?;
    let (member_identity, observed_session_meta) =
        resolve_snapshot_owner(snapshot, &registry, &project_registry_path)?;
    Ok(SourceArchiveRegistryVerifiedOwnerResolution {
        member_identity,
        observed_session_meta,
    })
}

fn resolve_snapshot_owner(
    snapshot: &SourceArchiveSnapshot,
    registry: &crate::control_plane::ProjectRegistryV2,
    registry_path: &Path,
) -> Result<(
    SearchSourceIdentity,
    Vec<SourceArchiveAttributedSessionMeta>,
)> {
    if snapshot.observed_session_meta.is_empty() {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "rollout source {} has no session_meta metadata",
            snapshot.source_path.display()
        )));
    }
    let members = registry
        .bindings()
        .iter()
        .map(|binding| {
            (
                binding.project_ref_id(),
                SearchSourceIdentity::new(
                    binding.project_ref_id(),
                    binding.store_id(),
                    binding.workspace_id(),
                    binding.branch_id(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut owners = BTreeMap::<SourceArchiveNativeOwner, SearchSourceIdentity>::new();
    let mut boundaries = Vec::<PathBuf>::new();
    for locator in registry.locators().iter().filter(|locator| {
        locator.role() == crate::control_plane::LocatorRole::Identity
            && locator.state() == crate::control_plane::LocatorState::Active
            && locator.authority() != crate::control_plane::LocatorAuthority::SemanticProject
    }) {
        let Some(identity) = members.get(&locator.project_ref_id()).copied() else {
            continue;
        };
        let identity_kind = match locator.authority() {
            crate::control_plane::LocatorAuthority::Repository => "git-common-dir",
            crate::control_plane::LocatorAuthority::Cwd => "cwd",
            crate::control_plane::LocatorAuthority::SemanticProject => unreachable!(),
        };
        let identity_path = PathBuf::from(locator.normalized_value());
        require_absolute_clean_path(&identity_path, "registry native identity path")?;
        let key = SourceArchiveNativeOwner {
            identity_kind: identity_kind.to_owned(),
            identity_path: identity_path.clone(),
        };
        if let Some(previous) = owners.insert(key, identity)
            && previous != identity
        {
            return Err(WorkVcsError::ControlPlaneInvalid(
                "one active native owner locator maps to multiple registry members".to_owned(),
            ));
        }
        if identity_kind == "git-common-dir" {
            boundaries.extend(repository_identity_boundaries(&identity_path)?);
        } else {
            boundaries.push(identity_path);
        }
    }
    boundaries.sort();
    boundaries.dedup();

    let mut selected_identity = None;
    let mut attributed = Vec::with_capacity(snapshot.observed_session_meta.len());
    for metadata in snapshot.observed_session_meta.iter() {
        let thread_id = metadata.thread_id.as_deref().ok_or_else(|| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "rollout source {} session_meta line {} has no thread identity",
                snapshot.source_path.display(),
                metadata.source_line
            ))
        })?;
        let raw_cwd = metadata.raw_cwd.as_deref().ok_or_else(|| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "rollout source {} session_meta line {} has no trusted cwd",
                snapshot.source_path.display(),
                metadata.source_line
            ))
        })?;
        if thread_id.is_empty() {
            return Err(WorkVcsError::ControlPlaneInvalid(format!(
                "rollout source {} session_meta line {} has an empty thread identity",
                snapshot.source_path.display(),
                metadata.source_line
            )));
        }
        let normalized_cwd = normalize_absolute_path_without_io(Path::new(raw_cwd))?;
        let (identity, owner_resolution) =
            resolve_snapshot_cwd_owner(&normalized_cwd, &owners, &boundaries, registry_path)?;
        if selected_identity.is_some_and(|selected| selected != identity) {
            return Err(WorkVcsError::ControlPlaneInvalid(format!(
                "rollout source {} session_meta contexts resolve to different ProjectRef member tuples",
                snapshot.source_path.display()
            )));
        }
        selected_identity = Some(identity);
        attributed.push(SourceArchiveAttributedSessionMeta {
            source_line: metadata.source_line,
            thread_id: thread_id.to_owned(),
            raw_cwd: PathBuf::from(raw_cwd),
            owner_resolution,
        });
    }
    Ok((
        selected_identity.expect("a non-empty metadata set selected one registry member"),
        attributed,
    ))
}

fn repository_identity_boundaries(common_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut boundaries = Vec::new();
    if common_dir.file_name() == Some(OsStr::new(".git"))
        && let Some(root) = common_dir.parent()
        && root.parent().is_some()
    {
        boundaries.push(root.to_path_buf());
    }
    let worktrees_root = common_dir.join("worktrees");
    let entries = match fs::read_dir(&worktrees_root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(boundaries),
        Err(error) => {
            return Err(storage_error("scan Git worktree metadata", &worktrees_root)(error));
        }
    };
    for entry in entries {
        let entry = entry.map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "cannot scan Git worktree metadata {}: {error}",
                worktrees_root.display()
            ))
        })?;
        let admin = entry.path();
        let metadata = fs::symlink_metadata(&admin)
            .map_err(storage_error("inspect Git worktree metadata", &admin))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "Git worktree metadata {} is not a real directory",
                admin.display()
            )));
        }
        let relative = admin.strip_prefix(&worktrees_root).map_err(|_| {
            WorkVcsError::IntegrityInvalid("Git worktree admin escaped its common-dir".to_owned())
        })?;
        if relative.components().count() != 1 {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "Git worktree metadata {} is not a direct child of {}",
                admin.display(),
                worktrees_root.display()
            )));
        }
        let gitdir_path = admin.join("gitdir");
        let gitdir = read_live_regular_file(
            &gitdir_path,
            MAX_NATIVE_METADATA_BYTES,
            "Git worktree backlink",
            false,
        )?;
        let target = parse_git_metadata_target(&gitdir, "", &gitdir_path, "Git worktree backlink")?;
        let target = if target.is_absolute() {
            target
        } else {
            admin.join(target)
        };
        let marker = normalize_absolute_path_without_io(&target)?;
        if marker.file_name() != Some(OsStr::new(".git")) {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "Git worktree backlink {} does not name a .git marker",
                gitdir_path.display()
            )));
        }
        let root = marker.parent().ok_or_else(|| {
            WorkVcsError::IntegrityInvalid(format!(
                "Git worktree backlink {} has no worktree root",
                gitdir_path.display()
            ))
        })?;
        boundaries.push(root.to_path_buf());
    }
    boundaries.sort();
    boundaries.dedup();
    Ok(boundaries)
}

fn resolve_snapshot_cwd_owner(
    normalized_cwd: &Path,
    owners: &BTreeMap<SourceArchiveNativeOwner, SearchSourceIdentity>,
    boundaries: &[PathBuf],
    registry_path: &Path,
) -> Result<(SearchSourceIdentity, SourceArchiveOwnerResolutionEvidence)> {
    let candidates = historical_cwd_candidates_without_io(normalized_cwd)?;
    let mut resolutions = Vec::new();
    for candidate in &candidates {
        for boundary in boundaries
            .iter()
            .filter(|boundary| candidate.starts_with(boundary))
        {
            open_absolute_directory_without_symlinks(candidate)?;
            resolutions.push((candidate.clone(), candidate.clone(), boundary.clone(), None));
        }
    }
    if resolutions.is_empty()
        && let Some((candidate, probe, boundary, alias)) =
            resolve_trusted_alias_candidates(&candidates, boundaries)?
    {
        resolutions.push((candidate, probe, boundary, Some(alias)));
    }
    resolutions.sort_by(|left, right| {
        left.1
            .cmp(&right.1)
            .then_with(|| {
                right
                    .2
                    .components()
                    .count()
                    .cmp(&left.2.components().count())
            })
            .then_with(|| left.2.cmp(&right.2))
            .then_with(|| left.0.cmp(&right.0))
    });
    resolutions.dedup();
    let Some(first) = resolutions.first() else {
        return Err(WorkVcsError::ProjectOwnerUnbound {
            registry_path: registry_path.display().to_string(),
            resolution_rank: "verified-local-boundary".to_owned(),
            unmapped_locators: 1,
            resolution_diagnostics: 0,
        });
    };
    if resolutions.iter().any(|resolution| resolution.1 != first.1) {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "historical cwd resolves to multiple distinct active probes".to_owned(),
        ));
    }
    let (candidate_cwd, probe_cwd, boundary, alias) = resolutions
        .into_iter()
        .next()
        .expect("resolution is present");
    let observation = observe_live_native_owner(&probe_cwd)?;
    let owner = SourceArchiveNativeOwner {
        identity_kind: observation.identity_kind.clone(),
        identity_path: observation.identity_path.clone(),
    };
    let identity =
        owners
            .get(&owner)
            .copied()
            .ok_or_else(|| WorkVcsError::ProjectOwnerUnbound {
                registry_path: registry_path.display().to_string(),
                resolution_rank: if owner.identity_kind == "git-common-dir" {
                    "repository"
                } else {
                    "cwd"
                }
                .to_owned(),
                unmapped_locators: 1,
                resolution_diagnostics: 0,
            })?;
    let native_identity = observation
        .identity_path
        .to_str()
        .ok_or_else(|| {
            WorkVcsError::IntegrityInvalid("native owner identity is not UTF-8".to_owned())
        })?
        .to_owned();
    let evidence = SourceArchiveOwnerResolutionEvidence {
        scheme: if alias.is_some() {
            SourceArchiveOwnerResolutionScheme::SafeAliasV1
        } else {
            SourceArchiveOwnerResolutionScheme::NativeIdentityV1
        },
        normalized_cwd: normalized_cwd.to_path_buf(),
        candidate_cwd,
        boundary,
        probe_cwd,
        native_identity_kind: observation.identity_kind,
        native_identity,
        native_identity_path: observation.identity_path,
        native_metadata_basis: observation.metadata_basis,
        native_metadata_sha256: observation.metadata_sha256,
        alias,
    };
    validate_owner_resolution_evidence(&evidence, &owners.keys().cloned().collect::<Vec<_>>())?;
    Ok((identity, evidence))
}

#[cfg(unix)]
fn resolve_trusted_alias_candidates(
    candidates: &[PathBuf],
    boundaries: &[PathBuf],
) -> Result<Option<(PathBuf, PathBuf, PathBuf, SourceArchiveAliasFingerprint)>> {
    let mut resolutions = Vec::new();
    let mut safely_inspected_candidate = false;
    let mut last_anchor_error = None;
    for candidate in candidates {
        require_absolute_clean_path(candidate, "historical cwd candidate")?;
        for boundary in boundaries {
            let Some(anchor) = common_path_ancestor(candidate, boundary) else {
                continue;
            };
            if normal_component_count(&anchor) < 2 {
                continue;
            }
            let relative = candidate
                .strip_prefix(&anchor)
                .expect("shared path ancestor is a prefix");
            let Some(Component::Normal(alias_name)) = relative.components().next() else {
                continue;
            };
            if let Err(error) = open_absolute_directory_without_symlinks(&anchor) {
                last_anchor_error = Some(error);
                continue;
            }
            let alias_path = anchor.join(alias_name);
            let metadata = match fs::symlink_metadata(&alias_path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    safely_inspected_candidate = true;
                    continue;
                }
                Err(error) => {
                    return Err(storage_error(
                        "inspect historical alias candidate",
                        &alias_path,
                    )(error));
                }
            };
            safely_inspected_candidate = true;
            if !metadata.file_type().is_symlink() {
                continue;
            }
            let fingerprint = snapshot_alias_fingerprint(&anchor, &alias_path)?;
            let suffix = candidate.strip_prefix(&alias_path).map_err(|_| {
                WorkVcsError::ControlPlaneInvalid(format!(
                    "historical alias candidate {} is not a prefix of {}",
                    alias_path.display(),
                    candidate.display()
                ))
            })?;
            let expanded =
                normalize_absolute_path_without_io(&fingerprint.link_target.join(suffix))?;
            if !expanded.starts_with(boundary) {
                continue;
            }
            open_absolute_directory_without_symlinks(&expanded)?;
            let stable_fingerprint = snapshot_alias_fingerprint(&anchor, &alias_path)?;
            if stable_fingerprint != fingerprint {
                return Err(WorkVcsError::ControlPlaneInvalid(format!(
                    "historical alias {} changed while its active-boundary resolution was verified",
                    alias_path.display()
                )));
            }
            resolutions.push((
                candidate.clone(),
                expanded,
                boundary.clone(),
                stable_fingerprint,
            ));
        }
    }
    resolutions.sort_by(|left, right| {
        left.1
            .cmp(&right.1)
            .then_with(|| {
                right
                    .2
                    .components()
                    .count()
                    .cmp(&left.2.components().count())
            })
            .then_with(|| left.2.cmp(&right.2))
            .then_with(|| left.0.cmp(&right.0))
    });
    resolutions.dedup();
    let Some(first) = resolutions.first() else {
        if !safely_inspected_candidate && let Some(error) = last_anchor_error {
            return Err(error);
        }
        return Ok(None);
    };
    if resolutions.iter().any(|resolution| resolution.1 != first.1) {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "historical cwd has ambiguous active-boundary alias resolutions".to_owned(),
        ));
    }
    Ok(resolutions.into_iter().next())
}

#[cfg(not(unix))]
fn resolve_trusted_alias_candidates(
    _candidates: &[PathBuf],
    _boundaries: &[PathBuf],
) -> Result<Option<(PathBuf, PathBuf, PathBuf, SourceArchiveAliasFingerprint)>> {
    Ok(None)
}

fn common_path_ancestor(left: &Path, right: &Path) -> Option<PathBuf> {
    let mut shared = PathBuf::new();
    let mut matched = false;
    for (left_component, right_component) in left.components().zip(right.components()) {
        if left_component != right_component {
            break;
        }
        shared.push(left_component.as_os_str());
        matched = true;
    }
    matched.then_some(shared)
}

fn normal_component_count(path: &Path) -> usize {
    path.components()
        .filter(|component| matches!(component, Component::Normal(_)))
        .count()
}

#[cfg(unix)]
fn snapshot_alias_fingerprint(
    anchor: &Path,
    link_path: &Path,
) -> Result<SourceArchiveAliasFingerprint> {
    let before = fs::symlink_metadata(link_path).map_err(storage_error(
        "inspect historical alias candidate",
        link_path,
    ))?;
    if !before.file_type().is_symlink() {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "historical alias {} changed to a non-symlink",
            link_path.display()
        )));
    }
    let before_identity = opened_identity_from_metadata(&before);
    let target = fs::read_link(link_path)
        .map_err(storage_error("read historical alias candidate", link_path))?;
    require_absolute_clean_path(&target, "historical alias target")?;
    let after = fs::symlink_metadata(link_path).map_err(storage_error(
        "reinspect historical alias candidate",
        link_path,
    ))?;
    if !after.file_type().is_symlink() || opened_identity_from_metadata(&after) != before_identity {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "historical alias {} changed while it was read",
            link_path.display()
        )));
    }
    let target_bytes = path_raw_bytes(&target);
    Ok(SourceArchiveAliasFingerprint {
        anchor: anchor.to_path_buf(),
        link_path: link_path.to_path_buf(),
        link_target: target,
        link_target_sha256: sha256_bytes(&target_bytes),
        device: before_identity.device,
        inode: before_identity.inode,
        mode: before_identity.mode,
        uid: before_identity.uid,
        gid: before_identity.gid,
        length: before_identity.length,
        modified_seconds: before_identity.modified_seconds,
        modified_nanoseconds: before_identity.modified_nanoseconds,
        changed_seconds: before_identity.changed_seconds,
        changed_nanoseconds: before_identity.changed_nanoseconds,
    })
}

fn selected_source_policy_entry<'a>(
    entries: &'a [crate::automation::AutomationSourcePolicyEntry],
    source_id: &str,
) -> Result<&'a crate::automation::AutomationSourcePolicyEntry> {
    let matches = entries
        .iter()
        .filter(|entry| entry.source_id == source_id)
        .collect::<Vec<_>>();
    let [entry] = matches.as_slice() else {
        return Err(WorkVcsError::IntegrityInvalid(
            "live automatic source attribution did not select exactly one source policy entry"
                .to_owned(),
        ));
    };
    Ok(*entry)
}

fn require_normal_descendant(path: &Path, root: &Path, label: &str) -> Result<()> {
    require_absolute_clean_path(root, "source policy root")?;
    require_absolute_clean_path(path, label)?;
    let relative = path.strip_prefix(root).map_err(|_| {
        WorkVcsError::IntegrityInvalid(format!(
            "{label} {} is outside source policy root {}",
            path.display(),
            root.display()
        ))
    })?;
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "{label} {} is not a normal descendant of source policy root {}",
            path.display(),
            root.display()
        )));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceArchiveImportOutcome {
    Imported,
    Reused,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceArchiveFileManifest {
    pub source_path: PathBuf,
    pub source_sha256: String,
    pub source_bytes: u64,
    pub imported_messages: usize,
    pub ignored_roles: BTreeMap<String, usize>,
    pub ignored_content_blocks: BTreeMap<String, usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_identity: Option<SourceArchiveContainerIdentity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub observed_session_meta: Vec<SourceArchiveObservedSessionMeta>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceArchiveContainerIdentity {
    pub scheme: String,
    pub container_id: String,
    pub canonical_source_path: PathBuf,
    pub source_snapshot_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceArchiveObservedSessionMeta {
    pub source_line: usize,
    pub thread_id: String,
    pub raw_cwd: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceArchiveMessageIdentity {
    Native {
        source_message_id: String,
    },
    DerivedOccurrenceV1 {
        occurrence_id: String,
        source_container_id: String,
        source_snapshot_sha256: String,
        source_line: usize,
        role: String,
        text_sha256: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceArchiveMessageOrigin {
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceArchiveDocument {
    pub document_id: String,
    pub source_path: PathBuf,
    pub source_sha256: String,
    pub source_line: usize,
    pub source_thread_id: Option<String>,
    pub source_message_id: Option<String>,
    pub source_timestamp: Option<String>,
    pub source_role: String,
    pub text: String,
    pub text_sha256: String,
    pub authority_class: String,
    pub currentness_class: String,
    pub action_authority: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_container_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_identity: Option<SourceArchiveMessageIdentity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_turn_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_origin: Option<SourceArchiveMessageOrigin>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automatic_attribution: Option<SourceArchiveAutomaticAttribution>,
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
struct LegacyArchiveBasis<'a> {
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

#[derive(Serialize)]
struct AutomaticAttributionBasis<'a> {
    attribution_version: u32,
    resolution_scheme: &'a str,
    provenance_class: &'a str,
    locator_authority: &'a str,
    source_snapshot_sha256: &'a str,
    member_identity: SearchSourceIdentity,
    registry_basis: &'a SourceArchiveRegistryBasis,
    source_policy_basis: &'a SourceArchivePolicyBasis,
    observed_session_meta: &'a [SourceArchiveAttributedSessionMeta],
}

#[derive(Serialize)]
struct AutomaticArchiveBasis<'a> {
    format_version: u32,
    source_format: SourceArchiveFormat,
    extraction_policy_version: &'a str,
    identity: SearchSourceIdentity,
    include_assistant: bool,
    authority_class: &'a str,
    currentness_class: &'a str,
    action_authority: &'a str,
    files: &'a [SourceArchiveFileManifest],
    automatic_attribution: AutomaticAttributionBasis<'a>,
}

fn automatic_attribution_basis(
    attribution: &SourceArchiveAutomaticAttribution,
) -> AutomaticAttributionBasis<'_> {
    AutomaticAttributionBasis {
        attribution_version: attribution.attribution_version,
        resolution_scheme: &attribution.resolution_scheme,
        provenance_class: &attribution.provenance_class,
        locator_authority: &attribution.locator_authority,
        source_snapshot_sha256: &attribution.source_snapshot_sha256,
        member_identity: attribution.member_identity,
        registry_basis: &attribution.registry_basis,
        source_policy_basis: &attribution.source_policy_basis,
        observed_session_meta: &attribution.observed_session_meta,
    }
}

fn validate_snapshot_attribution(options: &SourceArchiveImportOptions) -> Result<()> {
    let Some(attribution) = &options.automatic_attribution else {
        return Ok(());
    };
    validate_automatic_attribution_evidence(attribution)?;
    let [snapshot] = options.snapshots.as_slice() else {
        return Err(WorkVcsError::QueryInvalid(
            "automatic source archive import requires exactly one captured snapshot".to_owned(),
        ));
    };
    let expected_role_policy = if options.include_assistant {
        SourceArchiveRolePolicy::UserAndAssistant
    } else {
        SourceArchiveRolePolicy::UserOnly
    };
    let selected_policy = selected_source_policy_entry(
        &attribution.source_policy_basis.source_policy_entries,
        &attribution.source_policy_basis.source_id,
    )?;
    if !options.sources.is_empty()
        || attribution.attribution_version != AUTOMATIC_ATTRIBUTION_VERSION
        || attribution.resolution_scheme != AUTOMATIC_ATTRIBUTION_SCHEME
        || attribution.provenance_class != DERIVED_PROVENANCE_CLASS
        || attribution.locator_authority != LOCATOR_AUTHORITY
        || attribution.source_snapshot_sha256 != snapshot.source_sha256
        || attribution.member_identity != options.identity
        || attribution.source_policy_basis.role_policy != expected_role_policy
    {
        return Err(WorkVcsError::IntegrityInvalid(
            "automatic source archive attribution basis is incomplete or contradictory".to_owned(),
        ));
    }
    if snapshot.source_root != selected_policy.rollout_root {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "captured source root {} does not equal embedded source policy root {}",
            snapshot.source_root.display(),
            selected_policy.rollout_root.display()
        )));
    }
    require_normal_descendant(
        &snapshot.source_path,
        &selected_policy.rollout_root,
        "captured source path",
    )?;
    if snapshot.observed_session_meta.is_empty()
        || snapshot.observed_session_meta.len() != attribution.observed_session_meta.len()
    {
        return Err(WorkVcsError::IntegrityInvalid(
            "automatic source archive attribution does not cover every session_meta record"
                .to_owned(),
        ));
    }
    for (observed, attributed) in snapshot
        .observed_session_meta
        .iter()
        .zip(&attribution.observed_session_meta)
    {
        let thread_id = observed.thread_id.as_deref().ok_or_else(|| {
            WorkVcsError::IntegrityInvalid(format!(
                "source snapshot {} session_meta line {} has no thread identity",
                snapshot.source_path.display(),
                observed.source_line
            ))
        })?;
        let raw_cwd = observed.raw_cwd.as_deref().ok_or_else(|| {
            WorkVcsError::IntegrityInvalid(format!(
                "source snapshot {} session_meta line {} has no trusted cwd",
                snapshot.source_path.display(),
                observed.source_line
            ))
        })?;
        if attributed.source_line != observed.source_line
            || attributed.thread_id != thread_id
            || attributed.raw_cwd != Path::new(raw_cwd)
            || attributed.source_line == 0
            || attributed.thread_id.is_empty()
        {
            return Err(WorkVcsError::IntegrityInvalid(
                "automatic source archive attribution session metadata does not match the captured snapshot"
                    .to_owned(),
            ));
        }
        require_absolute_clean_path(&attributed.raw_cwd, "attributed session cwd")?;
    }
    Ok(())
}

fn validate_automatic_attribution_evidence(
    attribution: &SourceArchiveAutomaticAttribution,
) -> Result<()> {
    if attribution.attribution_version != AUTOMATIC_ATTRIBUTION_VERSION
        || attribution.resolution_scheme != AUTOMATIC_ATTRIBUTION_SCHEME
        || attribution.provenance_class != DERIVED_PROVENANCE_CLASS
        || attribution.locator_authority != LOCATOR_AUTHORITY
        || !is_sha256(&attribution.source_snapshot_sha256)
        || attribution.source_policy_basis.source_id.is_empty()
        || attribution.source_policy_basis.source_entry_revision == 0
        || attribution.source_policy_basis.source_policy_revision == 0
        || attribution.observed_at_us == 0
        || attribution.observed_session_meta.is_empty()
    {
        return Err(WorkVcsError::IntegrityInvalid(
            "automatic source archive attribution basis is incomplete or contradictory".to_owned(),
        ));
    }
    validate_registry_basis(attribution)?;
    validate_source_policy_basis(attribution)?;
    let owners = &attribution.registry_basis.member_native_owners;
    if owners.is_empty()
        || owners.windows(2).any(|pair| pair[0] >= pair[1])
        || owners.iter().any(|owner| {
            !matches!(owner.identity_kind.as_str(), "cwd" | "git-common-dir")
                || require_absolute_clean_path(&owner.identity_path, "registry native owner")
                    .is_err()
        })
    {
        return Err(WorkVcsError::IntegrityInvalid(
            "automatic source archive registry owner basis is empty or non-canonical".to_owned(),
        ));
    }
    let mut observed_lines = BTreeSet::new();
    for metadata in &attribution.observed_session_meta {
        if metadata.source_line == 0
            || metadata.thread_id.is_empty()
            || !observed_lines.insert(metadata.source_line)
        {
            return Err(WorkVcsError::IntegrityInvalid(
                "automatic source archive attribution contains invalid or duplicate session metadata"
                    .to_owned(),
            ));
        }
        require_absolute_clean_path(&metadata.raw_cwd, "attributed session cwd")?;
        let normalized_raw = normalize_absolute_path_without_io(&metadata.raw_cwd)?;
        if normalized_raw != metadata.owner_resolution.normalized_cwd {
            return Err(WorkVcsError::IntegrityInvalid(
                "automatic source archive raw cwd does not match its normalized owner evidence"
                    .to_owned(),
            ));
        }
        validate_owner_resolution_evidence(&metadata.owner_resolution, owners)?;
    }
    Ok(())
}

fn validate_registry_basis(attribution: &SourceArchiveAutomaticAttribution) -> Result<()> {
    let basis = &attribution.registry_basis;
    let registry = crate::control_plane::ProjectRegistryV2::from_json_bytes(
        basis.registry_snapshot_json.as_bytes(),
    )
    .map_err(|error| {
        WorkVcsError::IntegrityInvalid(format!(
            "automatic source archive registry snapshot is invalid: {error}"
        ))
    })?;
    if !registry
        .stored_json_bytes()
        .is_ok_and(|bytes| bytes == basis.registry_snapshot_json.as_bytes())
        || registry.registry_id().to_string() != basis.registry_id
        || registry.revision() != basis.registry_revision
        || !registry
            .digest()
            .is_ok_and(|digest| digest.to_string() == basis.registry_digest)
    {
        return Err(WorkVcsError::IntegrityInvalid(
            "automatic source archive registry basis does not match its canonical snapshot"
                .to_owned(),
        ));
    }
    let identity = attribution.member_identity;
    let binding = registry
        .bindings()
        .iter()
        .find(|binding| binding.project_ref_id() == identity.project_ref_id)
        .ok_or_else(|| {
            WorkVcsError::IntegrityInvalid(
                "automatic source archive member has no registry binding".to_owned(),
            )
        })?;
    if binding.store_id() != identity.store_id
        || binding.workspace_id() != identity.workspace_id
        || binding.branch_id() != identity.branch_id
    {
        return Err(WorkVcsError::IntegrityInvalid(
            "automatic source archive member tuple does not match its registry snapshot".to_owned(),
        ));
    }
    let mut expected_owners = registry
        .locators()
        .iter()
        .filter(|locator| {
            locator.project_ref_id() == identity.project_ref_id
                && locator.role() == crate::control_plane::LocatorRole::Identity
                && locator.state() == crate::control_plane::LocatorState::Active
                && locator.authority() != crate::control_plane::LocatorAuthority::SemanticProject
        })
        .map(|locator| SourceArchiveNativeOwner {
            identity_kind: match locator.authority() {
                crate::control_plane::LocatorAuthority::Repository => "git-common-dir",
                crate::control_plane::LocatorAuthority::Cwd => "cwd",
                crate::control_plane::LocatorAuthority::SemanticProject => {
                    unreachable!("semantic locators were filtered")
                }
            }
            .to_owned(),
            identity_path: PathBuf::from(locator.normalized_value()),
        })
        .collect::<Vec<_>>();
    expected_owners.sort();
    expected_owners.dedup();
    if expected_owners.is_empty() || expected_owners != basis.member_native_owners {
        return Err(WorkVcsError::IntegrityInvalid(
            "automatic source archive native owners do not match the selected registry member"
                .to_owned(),
        ));
    }
    Ok(())
}

fn validate_source_policy_basis(attribution: &SourceArchiveAutomaticAttribution) -> Result<()> {
    #[derive(Serialize)]
    struct SourcePolicyDigestBody<'a> {
        format_version: u32,
        revision: u64,
        sources: &'a [crate::automation::AutomationSourcePolicyEntry],
    }

    let basis = &attribution.source_policy_basis;
    let expected_digest = sha256_json(&SourcePolicyDigestBody {
        format_version: basis.source_policy_format_version,
        revision: basis.source_policy_revision,
        sources: &basis.source_policy_entries,
    })?;
    if basis.source_policy_format_version
        != crate::automation::AUTOMATION_SOURCE_POLICY_FORMAT_VERSION
        || basis.source_policy_entries.is_empty()
        || expected_digest != basis.source_policy_digest_sha256
    {
        return Err(WorkVcsError::IntegrityInvalid(
            "automatic source archive policy basis does not match its embedded snapshot".to_owned(),
        ));
    }
    let matches = basis
        .source_policy_entries
        .iter()
        .filter(|entry| entry.source_id == basis.source_id)
        .collect::<Vec<_>>();
    let [entry] = matches.as_slice() else {
        return Err(WorkVcsError::IntegrityInvalid(
            "automatic source archive policy basis does not select exactly one source entry"
                .to_owned(),
        ));
    };
    require_absolute_clean_path(&entry.rollout_root, "embedded source policy root")?;
    let expected_role = match entry.role_policy {
        crate::automation::AutomationRolePolicy::UserOnly => SourceArchiveRolePolicy::UserOnly,
        crate::automation::AutomationRolePolicy::UserAndAssistant => {
            SourceArchiveRolePolicy::UserAndAssistant
        }
    };
    if !entry.enabled
        || entry.revoked
        || entry.revision != basis.source_entry_revision
        || expected_role != basis.role_policy
    {
        return Err(WorkVcsError::IntegrityInvalid(
            "automatic source archive selected policy entry is inactive or contradictory"
                .to_owned(),
        ));
    }
    Ok(())
}

pub(crate) fn import_source_archive(
    options: &SourceArchiveImportOptions,
) -> Result<SourceArchiveImportResult> {
    let automatic = options.automatic_attribution.is_some();
    let paths = if automatic {
        options
            .snapshots
            .iter()
            .map(|snapshot| snapshot.source_path.clone())
            .collect::<Vec<_>>()
    } else {
        collect_sources(&options.sources)?
    };
    let root = prepare_archive_root(&options.archive_root, &paths)?;
    let mut files = Vec::with_capacity(paths.len());
    let mut documents = Vec::new();
    if automatic {
        validate_snapshot_attribution(options)?;
        for snapshot in &options.snapshots {
            let (file, mut extracted) =
                extract_codex_rollout_snapshot(snapshot, options.include_assistant)?;
            files.push(file);
            documents.append(&mut extracted);
        }
    } else {
        for source in paths {
            let (file, mut extracted) =
                extract_codex_rollout_legacy(&source, options.include_assistant)?;
            files.push(file);
            documents.append(&mut extracted);
        }
    }
    files.sort_by(|left, right| left.source_path.cmp(&right.source_path));
    documents.sort_by(|left, right| left.document_id.cmp(&right.document_id));
    require_unique_documents(&documents)?;
    if documents.is_empty() {
        return Err(WorkVcsError::QueryInvalid(
            "source archive import found no user or selected assistant text messages".to_owned(),
        ));
    }

    let (format_version, extraction_policy_version, basis_sha256) =
        if let Some(attribution) = &options.automatic_attribution {
            (
                AUTOMATIC_ARCHIVE_FORMAT_VERSION,
                AUTOMATIC_EXTRACTION_POLICY_VERSION,
                sha256_json(&AutomaticArchiveBasis {
                    format_version: AUTOMATIC_ARCHIVE_FORMAT_VERSION,
                    source_format: SourceArchiveFormat::CodexRolloutJsonl,
                    extraction_policy_version: AUTOMATIC_EXTRACTION_POLICY_VERSION,
                    identity: options.identity,
                    include_assistant: options.include_assistant,
                    authority_class: AUTHORITY_CLASS,
                    currentness_class: CURRENTNESS_CLASS,
                    action_authority: ACTION_AUTHORITY,
                    files: &files,
                    automatic_attribution: automatic_attribution_basis(attribution),
                })?,
            )
        } else {
            (
                LEGACY_ARCHIVE_FORMAT_VERSION,
                LEGACY_EXTRACTION_POLICY_VERSION,
                sha256_json(&LegacyArchiveBasis {
                    format_version: LEGACY_ARCHIVE_FORMAT_VERSION,
                    source_format: SourceArchiveFormat::CodexRolloutJsonl,
                    extraction_policy_version: LEGACY_EXTRACTION_POLICY_VERSION,
                    identity: options.identity,
                    include_assistant: options.include_assistant,
                    authority_class: AUTHORITY_CLASS,
                    currentness_class: CURRENTNESS_CLASS,
                    action_authority: ACTION_AUTHORITY,
                    files: &files,
                })?,
            )
        };
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
            format_version,
            source_format: SourceArchiveFormat::CodexRolloutJsonl,
            extraction_policy_version: extraction_policy_version.to_owned(),
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
            automatic_attribution: options.automatic_attribution.clone(),
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

fn extract_codex_rollout_legacy(
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
            source_thread_id: Some(source_thread_id.clone()),
            source_message_id: Some(message_id.to_owned()),
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
            source_container_id: None,
            message_identity: None,
            source_turn_id: None,
            message_origin: None,
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
            container_identity: None,
            observed_session_meta: Vec::new(),
        },
        documents,
    ))
}

fn extract_codex_rollout_snapshot(
    snapshot: &SourceArchiveSnapshot,
    include_assistant: bool,
) -> Result<(SourceArchiveFileManifest, Vec<SourceArchiveDocument>)> {
    let observed_session_meta = snapshot
        .observed_session_meta
        .iter()
        .map(|metadata| {
            let thread_id = metadata.thread_id.clone().ok_or_else(|| {
                WorkVcsError::QueryInvalid(format!(
                    "source rollout {} line {} session_meta has no thread identity",
                    snapshot.source_path.display(),
                    metadata.source_line
                ))
            })?;
            let raw_cwd = metadata.raw_cwd.clone().ok_or_else(|| {
                WorkVcsError::QueryInvalid(format!(
                    "source rollout {} line {} session_meta has no trusted cwd",
                    snapshot.source_path.display(),
                    metadata.source_line
                ))
            })?;
            if thread_id.is_empty() {
                return Err(WorkVcsError::QueryInvalid(format!(
                    "source rollout {} line {} session_meta has an empty thread identity",
                    snapshot.source_path.display(),
                    metadata.source_line
                )));
            }
            let raw_cwd = PathBuf::from(raw_cwd);
            require_absolute_clean_path(&raw_cwd, "session_meta cwd")?;
            Ok(SourceArchiveObservedSessionMeta {
                source_line: metadata.source_line,
                thread_id,
                raw_cwd,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if observed_session_meta.is_empty() {
        return Err(WorkVcsError::QueryInvalid(format!(
            "source rollout {} has no session_meta metadata",
            snapshot.source_path.display()
        )));
    }
    let container_id = source_container_id(&snapshot.source_path, &snapshot.source_sha256)?;
    let container_identity = SourceArchiveContainerIdentity {
        scheme: "source_snapshot_container_v1".to_owned(),
        container_id: container_id.clone(),
        canonical_source_path: snapshot.source_path.clone(),
        source_snapshot_sha256: snapshot.source_sha256.clone(),
    };
    let mut ignored_roles = BTreeMap::new();
    let mut ignored_content_blocks = BTreeMap::new();
    let mut documents = Vec::new();
    for parsed in snapshot.parsed.iter() {
        let value = &parsed.value;
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
        let source_message_id = match payload.get("id") {
            None => None,
            Some(serde_json::Value::String(value)) if valid_native_message_id(value) => {
                Some(value.clone())
            }
            Some(_) => {
                return Err(WorkVcsError::QueryInvalid(format!(
                    "source rollout {} line {} has a malformed native message id",
                    snapshot.source_path.display(),
                    parsed.source_line
                )));
            }
        };
        let message_identity = if let Some(source_message_id) = &source_message_id {
            SourceArchiveMessageIdentity::Native {
                source_message_id: source_message_id.clone(),
            }
        } else {
            SourceArchiveMessageIdentity::DerivedOccurrenceV1 {
                occurrence_id: derived_occurrence_id(
                    &container_id,
                    &snapshot.source_sha256,
                    parsed.source_line,
                    role,
                    &text_sha256,
                ),
                source_container_id: container_id.clone(),
                source_snapshot_sha256: snapshot.source_sha256.clone(),
                source_line: parsed.source_line,
                role: role.to_owned(),
                text_sha256: text_sha256.clone(),
            }
        };
        let source_turn_id = payload
            .get("internal_chat_message_metadata_passthrough")
            .and_then(|metadata| metadata.get("turn_id"))
            .map(|turn_id| {
                let turn_id = turn_id.as_str().ok_or_else(|| {
                    WorkVcsError::QueryInvalid(format!(
                        "source rollout {} line {} has a malformed turn lineage id",
                        snapshot.source_path.display(),
                        parsed.source_line
                    ))
                })?;
                if turn_id.is_empty() {
                    return Err(WorkVcsError::QueryInvalid(format!(
                        "source rollout {} line {} has an empty turn lineage id",
                        snapshot.source_path.display(),
                        parsed.source_line
                    )));
                }
                Ok(turn_id.to_owned())
            })
            .transpose()?;
        let document_id = source_document_id_v2(
            &container_id,
            &snapshot.source_sha256,
            parsed.source_line,
            &message_identity,
            role,
            &text_sha256,
        )?;
        documents.push(SourceArchiveDocument {
            document_id,
            source_path: snapshot.source_path.clone(),
            source_sha256: snapshot.source_sha256.clone(),
            source_line: parsed.source_line,
            source_thread_id: None,
            source_message_id,
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
            source_container_id: Some(container_id.clone()),
            message_identity: Some(message_identity),
            source_turn_id,
            message_origin: Some(SourceArchiveMessageOrigin::Unknown),
        });
    }
    Ok((
        SourceArchiveFileManifest {
            source_path: snapshot.source_path.clone(),
            source_sha256: snapshot.source_sha256.clone(),
            source_bytes: snapshot.source_bytes,
            imported_messages: documents.len(),
            ignored_roles,
            ignored_content_blocks,
            container_identity: Some(container_identity),
            observed_session_meta,
        },
        documents,
    ))
}

fn validate_archive(
    path: &Path,
    expected_identity: Option<SearchSourceIdentity>,
) -> Result<(SourceArchiveManifest, Vec<SourceArchiveDocument>)> {
    validate_archive_with_hook(path, expected_identity, || {})
}

fn validate_archive_with_hook(
    path: &Path,
    expected_identity: Option<SearchSourceIdentity>,
    before_member_open: impl FnOnce(),
) -> Result<(SourceArchiveManifest, Vec<SourceArchiveDocument>)> {
    require_real_directory(path)?;
    require_private_archive_permissions(path)?;
    #[cfg(unix)]
    let archive_directory = open_absolute_directory_without_symlinks(path)?;
    #[cfg(unix)]
    let archive_identity = opened_identity_from_metadata(
        &archive_directory
            .metadata()
            .map_err(storage_error("inspect opened source archive", path))?,
    );
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
    before_member_open();
    #[cfg(unix)]
    let manifest_bytes = read_archive_member(
        &archive_directory,
        path,
        MANIFEST_FILE,
        MAX_ARCHIVE_MANIFEST_BYTES,
    )?;
    #[cfg(not(unix))]
    let manifest_bytes = read_live_regular_file(
        &path.join(MANIFEST_FILE),
        MAX_ARCHIVE_MANIFEST_BYTES,
        "source archive manifest",
        true,
    )?;
    let manifest_value: serde_json::Value =
        serde_json::from_slice(&manifest_bytes).map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "source archive manifest {} is invalid: {error}",
                path.display()
            ))
        })?;
    let format_version = manifest_value
        .get("format_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| {
            WorkVcsError::StorageFailure(format!(
                "source archive manifest {} has no valid format version",
                path.display()
            ))
        })?;
    if !matches!(
        format_version,
        value if value == u64::from(LEGACY_ARCHIVE_FORMAT_VERSION)
            || value == u64::from(AUTOMATIC_ARCHIVE_FORMAT_VERSION)
    ) {
        return Err(WorkVcsError::StorageFailure(format!(
            "source archive {} has unsupported format version {format_version}",
            path.display()
        )));
    }
    let manifest: SourceArchiveManifest =
        serde_json::from_value(manifest_value).map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "source archive manifest {} is invalid: {error}",
                path.display()
            ))
        })?;
    if manifest.source_format != SourceArchiveFormat::CodexRolloutJsonl
        || manifest.authority_class != AUTHORITY_CLASS
        || manifest.currentness_class != CURRENTNESS_CLASS
        || manifest.action_authority != ACTION_AUTHORITY
        || expected_identity.is_some_and(|identity| identity != manifest.identity)
        || manifest.manifest_sha256 != manifest_sha256(&manifest)?
    {
        return Err(WorkVcsError::StorageFailure(format!(
            "source archive {} manifest identity, policy, or digest mismatch",
            path.display()
        )));
    }
    validate_manifest_version_contract(&manifest, path)?;
    #[cfg(unix)]
    let document_bytes = read_archive_member(
        &archive_directory,
        path,
        DOCUMENTS_FILE,
        MAX_ARCHIVE_DOCUMENT_BYTES,
    )?;
    #[cfg(not(unix))]
    let document_bytes = read_live_regular_file(
        &path.join(DOCUMENTS_FILE),
        MAX_ARCHIVE_DOCUMENT_BYTES,
        "source archive documents",
        true,
    )?;
    #[cfg(unix)]
    {
        let current = fs::symlink_metadata(path)
            .map_err(storage_error("reinspect source archive path", path))?;
        if current.file_type().is_symlink()
            || !current.is_dir()
            || opened_identity_from_metadata(&current) != archive_identity
        {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "source archive {} changed identity while it was read",
                path.display()
            )));
        }
    }
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
            || document.source_line == 0
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
        validate_document_version_contract(&manifest, file, document)?;
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

#[cfg(unix)]
fn read_archive_member(
    archive_directory: &fs::File,
    archive_path: &Path,
    member_name: &str,
    max_bytes: u64,
) -> Result<Vec<u8>> {
    let member_path = archive_path.join(member_name);
    let name = OsStr::new(member_name);
    let expected = lstatat(archive_directory.as_raw_fd(), name, &member_path)?;
    let descriptor = openat(
        archive_directory.as_raw_fd(),
        name,
        libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        &member_path,
    )?;
    let mut file = unsafe { fs::File::from_raw_fd(descriptor) };
    let metadata = file
        .metadata()
        .map_err(storage_error("inspect source archive member", &member_path))?;
    let opened = opened_identity_from_metadata(&metadata);
    if opened.device != expected.device
        || opened.inode != expected.inode
        || opened.mode != expected.mode
        || !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o022 != 0
        || metadata.len() > max_bytes
    {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "source archive member {} has an unsafe identity, type, owner, permission, or size",
            member_path.display()
        )));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut bytes)
        .map_err(storage_error("read source archive member", &member_path))?;
    let after = opened_identity_from_metadata(&file.metadata().map_err(storage_error(
        "reinspect source archive member",
        &member_path,
    ))?);
    if after != opened || bytes.len() as u64 != opened.length {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "source archive member {} changed while it was read",
            member_path.display()
        )));
    }
    Ok(bytes)
}

fn validate_manifest_version_contract(manifest: &SourceArchiveManifest, path: &Path) -> Result<()> {
    match manifest.format_version {
        LEGACY_ARCHIVE_FORMAT_VERSION => {
            if manifest.extraction_policy_version != LEGACY_EXTRACTION_POLICY_VERSION
                || manifest.automatic_attribution.is_some()
                || manifest.files.iter().any(|file| {
                    file.container_identity.is_some() || !file.observed_session_meta.is_empty()
                })
                || manifest.basis_sha256
                    != sha256_json(&LegacyArchiveBasis {
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
                    "source archive {} has contradictory v1 provenance",
                    path.display()
                )));
            }
        }
        AUTOMATIC_ARCHIVE_FORMAT_VERSION => {
            let Some(attribution) = &manifest.automatic_attribution else {
                return Err(WorkVcsError::StorageFailure(format!(
                    "source archive {} v2 manifest omitted automatic attribution",
                    path.display()
                )));
            };
            validate_automatic_attribution_evidence(attribution).map_err(|error| {
                WorkVcsError::StorageFailure(format!(
                    "source archive {} has invalid automatic attribution: {error}",
                    path.display()
                ))
            })?;
            let [file] = manifest.files.as_slice() else {
                return Err(WorkVcsError::StorageFailure(format!(
                    "source archive {} v2 manifest must bind exactly one captured source",
                    path.display()
                )));
            };
            let Some(container) = &file.container_identity else {
                return Err(WorkVcsError::StorageFailure(format!(
                    "source archive {} v2 manifest omitted container identity",
                    path.display()
                )));
            };
            let selected_policy = selected_source_policy_entry(
                &attribution.source_policy_basis.source_policy_entries,
                &attribution.source_policy_basis.source_id,
            )
            .map_err(|error| {
                WorkVcsError::StorageFailure(format!(
                    "source archive {} has no unique embedded source policy: {error}",
                    path.display()
                ))
            })?;
            require_normal_descendant(
                &file.source_path,
                &selected_policy.rollout_root,
                "archived source path",
            )
            .map_err(|error| {
                WorkVcsError::StorageFailure(format!(
                    "source archive {} has a source outside its embedded policy root: {error}",
                    path.display()
                ))
            })?;
            let expected_role_policy = if manifest.include_assistant {
                SourceArchiveRolePolicy::UserAndAssistant
            } else {
                SourceArchiveRolePolicy::UserOnly
            };
            let observed_from_attribution = attribution
                .observed_session_meta
                .iter()
                .map(|metadata| SourceArchiveObservedSessionMeta {
                    source_line: metadata.source_line,
                    thread_id: metadata.thread_id.clone(),
                    raw_cwd: metadata.raw_cwd.clone(),
                })
                .collect::<Vec<_>>();
            if manifest.extraction_policy_version != AUTOMATIC_EXTRACTION_POLICY_VERSION
                || attribution.attribution_version != AUTOMATIC_ATTRIBUTION_VERSION
                || attribution.resolution_scheme != AUTOMATIC_ATTRIBUTION_SCHEME
                || attribution.provenance_class != DERIVED_PROVENANCE_CLASS
                || attribution.locator_authority != LOCATOR_AUTHORITY
                || attribution.member_identity != manifest.identity
                || attribution.source_snapshot_sha256 != file.source_sha256
                || attribution.source_policy_basis.role_policy != expected_role_policy
                || crate::RegistryId::parse_canonical(&attribution.registry_basis.registry_id)
                    .is_err()
                || attribution.registry_basis.registry_revision == 0
                || crate::control_plane::ControlPlaneDigest::from_text(
                    &attribution.registry_basis.registry_digest,
                )
                .is_err()
                || attribution.source_policy_basis.source_id.is_empty()
                || attribution.source_policy_basis.source_entry_revision == 0
                || attribution.source_policy_basis.source_policy_revision == 0
                || !is_sha256(&attribution.source_policy_basis.source_policy_digest_sha256)
                || attribution.observed_at_us == 0
                || file.observed_session_meta.is_empty()
                || file.observed_session_meta != observed_from_attribution
                || container.scheme != "source_snapshot_container_v1"
                || container.canonical_source_path != file.source_path
                || container.source_snapshot_sha256 != file.source_sha256
                || container.container_id
                    != source_container_id(&file.source_path, &file.source_sha256)?
                || manifest.basis_sha256
                    != sha256_json(&AutomaticArchiveBasis {
                        format_version: manifest.format_version,
                        source_format: manifest.source_format,
                        extraction_policy_version: &manifest.extraction_policy_version,
                        identity: manifest.identity,
                        include_assistant: manifest.include_assistant,
                        authority_class: &manifest.authority_class,
                        currentness_class: &manifest.currentness_class,
                        action_authority: &manifest.action_authority,
                        files: &manifest.files,
                        automatic_attribution: automatic_attribution_basis(attribution),
                    })?
            {
                return Err(WorkVcsError::StorageFailure(format!(
                    "source archive {} has contradictory v2 provenance",
                    path.display()
                )));
            }
            let mut observed_lines = BTreeSet::new();
            for (observed, attributed) in file
                .observed_session_meta
                .iter()
                .zip(&attribution.observed_session_meta)
            {
                if observed.source_line == 0
                    || observed.thread_id.is_empty()
                    || !observed_lines.insert(observed.source_line)
                    || attributed.raw_cwd != attributed.owner_resolution.normalized_cwd
                {
                    return Err(WorkVcsError::StorageFailure(format!(
                        "source archive {} has invalid or duplicate observed session metadata",
                        path.display()
                    )));
                }
                require_absolute_clean_path(&observed.raw_cwd, "observed session cwd")?;
                validate_owner_resolution_evidence(
                    &attributed.owner_resolution,
                    &attribution.registry_basis.member_native_owners,
                )?;
            }
        }
        version => {
            return Err(WorkVcsError::StorageFailure(format!(
                "source archive {} has unsupported format version {version}",
                path.display()
            )));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LiveNativeOwnerObservation {
    identity_kind: String,
    identity_path: PathBuf,
    metadata_basis: SourceArchiveNativeMetadataBasis,
    metadata_sha256: String,
}

fn validate_live_owner_resolution_evidence(
    evidence: &SourceArchiveOwnerResolutionEvidence,
) -> Result<()> {
    #[cfg(not(unix))]
    {
        let _ = evidence;
        return Err(WorkVcsError::ControlPlaneInvalid(
            "live automatic owner verification requires Unix filesystem metadata".to_owned(),
        ));
    }
    #[cfg(unix)]
    {
        if let Some(alias) = &evidence.alias {
            open_absolute_directory_without_symlinks(&alias.anchor)?;
            let before = fs::symlink_metadata(&alias.link_path).map_err(storage_error(
                "inspect live historical alias",
                &alias.link_path,
            ))?;
            if !before.file_type().is_symlink() {
                return Err(WorkVcsError::IntegrityInvalid(format!(
                    "historical alias {} is no longer a symlink",
                    alias.link_path.display()
                )));
            }
            let target = fs::read_link(&alias.link_path).map_err(storage_error(
                "read live historical alias",
                &alias.link_path,
            ))?;
            let after = fs::symlink_metadata(&alias.link_path).map_err(storage_error(
                "reinspect live historical alias",
                &alias.link_path,
            ))?;
            let matches = target == alias.link_target
                && before.dev() == alias.device
                && before.ino() == alias.inode
                && before.mode() == alias.mode
                && before.uid() == alias.uid
                && before.gid() == alias.gid
                && before.len() == alias.length
                && before.mtime() == alias.modified_seconds
                && before.mtime_nsec() == alias.modified_nanoseconds
                && before.ctime() == alias.changed_seconds
                && before.ctime_nsec() == alias.changed_nanoseconds
                && before.dev() == after.dev()
                && before.ino() == after.ino()
                && before.mode() == after.mode()
                && before.len() == after.len()
                && before.mtime() == after.mtime()
                && before.mtime_nsec() == after.mtime_nsec()
                && before.ctime() == after.ctime()
                && before.ctime_nsec() == after.ctime_nsec();
            if !matches {
                return Err(WorkVcsError::IntegrityInvalid(format!(
                    "historical alias {} changed after its captured owner observation",
                    alias.link_path.display()
                )));
            }
        }
        let live = observe_live_native_owner(&evidence.probe_cwd)?;
        if live.identity_kind != evidence.native_identity_kind
            || live.identity_path != evidence.native_identity_path
            || live.metadata_basis != evidence.native_metadata_basis
            || live.metadata_sha256 != evidence.native_metadata_sha256
        {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "historical owner {} no longer matches its live native resolver observation",
                evidence.probe_cwd.display()
            )));
        }
        Ok(())
    }
}

#[cfg(unix)]
fn observe_live_native_owner(cwd: &Path) -> Result<LiveNativeOwnerObservation> {
    require_absolute_clean_path(cwd, "historical owner probe cwd")?;
    let directory = open_absolute_directory_without_symlinks(cwd)?;
    if !directory
        .metadata()
        .map_err(storage_error("inspect historical owner probe cwd", cwd))?
        .is_dir()
    {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "historical owner probe cwd {} is not a directory",
            cwd.display()
        )));
    }
    let canonical_cwd = fs::canonicalize(cwd).map_err(storage_error(
        "canonicalize historical owner probe cwd",
        cwd,
    ))?;
    let mut cursor = canonical_cwd.as_path();
    loop {
        let marker = cursor.join(".git");
        match fs::symlink_metadata(&marker) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(WorkVcsError::IntegrityInvalid(format!(
                    "Git metadata marker {} is a symlink",
                    marker.display()
                )));
            }
            Ok(metadata) if metadata.is_dir() => {
                let git_admin_path = canonical_live_directory(&marker, "Git metadata directory")?;
                let head_path = git_admin_path.join("HEAD");
                let head = read_live_regular_file(
                    &head_path,
                    MAX_NATIVE_METADATA_BYTES,
                    "Git HEAD",
                    false,
                )?;
                let basis = SourceArchiveNativeMetadataBasis {
                    identity_root: cursor.to_path_buf(),
                    marker_kind: "git_directory".to_owned(),
                    marker_path: Some(marker),
                    git_admin_path: Some(git_admin_path.clone()),
                    components: vec![live_metadata_component("head", head_path, &head, None)],
                };
                let metadata_sha256 = sha256_json(&basis)?;
                return Ok(LiveNativeOwnerObservation {
                    identity_kind: "git-common-dir".to_owned(),
                    identity_path: git_admin_path,
                    metadata_basis: basis,
                    metadata_sha256,
                });
            }
            Ok(metadata) if metadata.is_file() => {
                let pointer = read_live_regular_file(
                    &marker,
                    MAX_NATIVE_METADATA_BYTES,
                    "Git worktree pointer",
                    false,
                )?;
                let pointer_target = parse_git_metadata_target(
                    &pointer,
                    "gitdir: ",
                    &marker,
                    "Git worktree pointer",
                )?;
                let pointer_target = if pointer_target.is_absolute() {
                    pointer_target
                } else {
                    cursor.join(pointer_target)
                };
                let git_admin_path =
                    canonical_live_directory(&pointer_target, "Git worktree metadata directory")?;
                let commondir_path = git_admin_path.join("commondir");
                let commondir = read_live_regular_file(
                    &commondir_path,
                    MAX_NATIVE_METADATA_BYTES,
                    "Git commondir",
                    false,
                )?;
                let common_target =
                    parse_git_metadata_target(&commondir, "", &commondir_path, "Git commondir")?;
                let common_target = if common_target.is_absolute() {
                    common_target
                } else {
                    git_admin_path.join(common_target)
                };
                let common_dir = canonical_live_directory(&common_target, "Git common-dir")?;
                let worktrees_root = common_dir.join("worktrees");
                let relative = git_admin_path.strip_prefix(&worktrees_root).map_err(|_| {
                    WorkVcsError::IntegrityInvalid(format!(
                        "Git worktree metadata directory {} is outside {}",
                        git_admin_path.display(),
                        worktrees_root.display()
                    ))
                })?;
                if relative.components().count() != 1 {
                    return Err(WorkVcsError::IntegrityInvalid(format!(
                        "Git worktree metadata directory {} is not one direct child of {}",
                        git_admin_path.display(),
                        worktrees_root.display()
                    )));
                }
                let gitdir_path = git_admin_path.join("gitdir");
                let gitdir = read_live_regular_file(
                    &gitdir_path,
                    MAX_NATIVE_METADATA_BYTES,
                    "Git worktree backlink",
                    false,
                )?;
                let backlink =
                    parse_git_metadata_target(&gitdir, "", &gitdir_path, "Git worktree backlink")?;
                let backlink = if backlink.is_absolute() {
                    backlink
                } else {
                    git_admin_path.join(backlink)
                };
                let expected_marker = fs::canonicalize(&marker)
                    .map_err(storage_error("canonicalize Git worktree pointer", &marker))?;
                let actual_marker = fs::canonicalize(&backlink).map_err(storage_error(
                    "canonicalize Git worktree backlink",
                    &backlink,
                ))?;
                if expected_marker != actual_marker {
                    return Err(WorkVcsError::IntegrityInvalid(format!(
                        "Git worktree backlink {} does not reference {}",
                        gitdir_path.display(),
                        marker.display()
                    )));
                }
                let head_path = git_admin_path.join("HEAD");
                let head = read_live_regular_file(
                    &head_path,
                    MAX_NATIVE_METADATA_BYTES,
                    "Git HEAD",
                    false,
                )?;
                let basis = SourceArchiveNativeMetadataBasis {
                    identity_root: cursor.to_path_buf(),
                    marker_kind: "git_worktree_pointer".to_owned(),
                    marker_path: Some(marker.clone()),
                    git_admin_path: Some(git_admin_path.clone()),
                    components: vec![
                        live_metadata_component(
                            "worktree-pointer",
                            marker,
                            &pointer,
                            Some(git_admin_path.clone()),
                        ),
                        live_metadata_component(
                            "commondir",
                            commondir_path,
                            &commondir,
                            Some(common_dir.clone()),
                        ),
                        live_metadata_component(
                            "gitdir",
                            gitdir_path,
                            &gitdir,
                            Some(actual_marker),
                        ),
                        live_metadata_component("head", head_path, &head, None),
                    ],
                };
                let metadata_sha256 = sha256_json(&basis)?;
                return Ok(LiveNativeOwnerObservation {
                    identity_kind: "git-common-dir".to_owned(),
                    identity_path: common_dir,
                    metadata_basis: basis,
                    metadata_sha256,
                });
            }
            Ok(_) => {
                return Err(WorkVcsError::IntegrityInvalid(format!(
                    "Git metadata marker {} has an unsafe type",
                    marker.display()
                )));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(WorkVcsError::StorageFailure(format!(
                    "cannot inspect Git metadata marker {}: {error}",
                    marker.display()
                )));
            }
        }
        let Some(parent) = cursor.parent() else {
            break;
        };
        if parent == cursor {
            break;
        }
        cursor = parent;
    }
    let basis = SourceArchiveNativeMetadataBasis {
        identity_root: canonical_cwd.clone(),
        marker_kind: "none".to_owned(),
        marker_path: None,
        git_admin_path: None,
        components: Vec::new(),
    };
    Ok(LiveNativeOwnerObservation {
        identity_kind: "cwd".to_owned(),
        identity_path: canonical_cwd,
        metadata_sha256: sha256_json(&basis)?,
        metadata_basis: basis,
    })
}

#[cfg(unix)]
fn canonical_live_directory(path: &Path, label: &str) -> Result<PathBuf> {
    let canonical =
        fs::canonicalize(path).map_err(storage_error("canonicalize live directory", path))?;
    let opened = open_absolute_directory_without_symlinks(&canonical)?;
    if !opened
        .metadata()
        .map_err(storage_error("inspect live directory", &canonical))?
        .is_dir()
    {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "{label} {} is not a directory",
            canonical.display()
        )));
    }
    Ok(canonical)
}

#[cfg(unix)]
fn parse_git_metadata_target(
    bytes: &[u8],
    prefix: &str,
    path: &Path,
    label: &str,
) -> Result<PathBuf> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        WorkVcsError::IntegrityInvalid(format!("{label} {} is not UTF-8: {error}", path.display()))
    })?;
    let mut lines = text.lines().filter(|line| !line.is_empty());
    let line = lines.next().ok_or_else(|| {
        WorkVcsError::IntegrityInvalid(format!("{label} {} is empty", path.display()))
    })?;
    if lines.next().is_some() {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "{label} {} contains multiple non-empty lines",
            path.display()
        )));
    }
    let value = line.strip_prefix(prefix).ok_or_else(|| {
        WorkVcsError::IntegrityInvalid(format!("{label} {} has an invalid format", path.display()))
    })?;
    if value.is_empty() {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "{label} {} has an empty target",
            path.display()
        )));
    }
    Ok(PathBuf::from(value))
}

fn live_metadata_component(
    label: &str,
    path: PathBuf,
    bytes: &[u8],
    resolved_target: Option<PathBuf>,
) -> SourceArchiveNativeMetadataComponent {
    SourceArchiveNativeMetadataComponent {
        label: label.to_owned(),
        path,
        bytes: bytes.len() as u64,
        sha256: sha256_bytes(bytes),
        resolved_target,
    }
}

fn validate_owner_resolution_evidence(
    evidence: &SourceArchiveOwnerResolutionEvidence,
    registry_owners: &[SourceArchiveNativeOwner],
) -> Result<()> {
    require_absolute_clean_path(&evidence.normalized_cwd, "normalized historical cwd")?;
    require_absolute_clean_path(&evidence.candidate_cwd, "historical cwd candidate")?;
    require_absolute_clean_path(&evidence.boundary, "historical owner boundary")?;
    require_absolute_clean_path(&evidence.probe_cwd, "historical owner probe cwd")?;
    require_absolute_clean_path(&evidence.native_identity_path, "native owner identity")?;
    let identity_text = evidence.native_identity_path.to_str().ok_or_else(|| {
        WorkVcsError::StorageFailure(
            "automatic source archive native owner path is not UTF-8".to_owned(),
        )
    })?;
    let candidates = historical_cwd_candidates_without_io(&evidence.normalized_cwd)?;
    if !evidence.probe_cwd.starts_with(&evidence.boundary)
        || !matches!(
            evidence.native_identity_kind.as_str(),
            "cwd" | "git-common-dir"
        )
        || evidence.native_identity != identity_text
        || !candidates.contains(&evidence.candidate_cwd)
        || !is_sha256(&evidence.native_metadata_sha256)
        || !registry_owners.contains(&SourceArchiveNativeOwner {
            identity_kind: evidence.native_identity_kind.clone(),
            identity_path: evidence.native_identity_path.clone(),
        })
    {
        return Err(WorkVcsError::StorageFailure(
            "automatic source archive native owner evidence is incomplete".to_owned(),
        ));
    }
    validate_native_metadata_basis(evidence)?;
    match (evidence.scheme, &evidence.alias) {
        (SourceArchiveOwnerResolutionScheme::NativeIdentityV1, None)
            if evidence.candidate_cwd == evidence.probe_cwd =>
        {
            Ok(())
        }
        (SourceArchiveOwnerResolutionScheme::SafeAliasV1, Some(alias)) => {
            require_absolute_clean_path(&alias.anchor, "historical alias anchor")?;
            require_absolute_clean_path(&alias.link_path, "historical alias path")?;
            require_absolute_clean_path(&alias.link_target, "historical alias target")?;
            let expected_anchor = common_path_ancestor(&evidence.candidate_cwd, &evidence.boundary)
                .ok_or_else(|| {
                    WorkVcsError::StorageFailure(
                        "automatic source archive alias has no shared active-boundary anchor"
                            .to_owned(),
                    )
                })?;
            let relative = evidence
                .candidate_cwd
                .strip_prefix(&expected_anchor)
                .expect("shared path ancestor is a prefix");
            let Some(Component::Normal(alias_name)) = relative.components().next() else {
                return Err(WorkVcsError::StorageFailure(
                    "automatic source archive alias does not name the first diverging path child"
                        .to_owned(),
                ));
            };
            let expected_link_path = expected_anchor.join(alias_name);
            let target_bytes = path_raw_bytes(&alias.link_target);
            let suffix = evidence
                .candidate_cwd
                .strip_prefix(&alias.link_path)
                .map_err(|_| {
                    WorkVcsError::StorageFailure(
                        "automatic source archive alias path is not a prefix of its cwd candidate"
                            .to_owned(),
                    )
                })?;
            let expanded = normalize_absolute_path_without_io(&alias.link_target.join(suffix))?;
            // Keep readback portable without depending on the Unix-only stat helper.
            #[allow(clippy::useless_conversion)]
            let file_type_mask = u32::from(libc::S_IFMT);
            #[allow(clippy::useless_conversion)]
            let symlink_file_type = u32::from(libc::S_IFLNK);
            if normal_component_count(&expected_anchor) < 2
                || alias.anchor != expected_anchor
                || alias.link_path != expected_link_path
                || alias.link_path.parent() != Some(alias.anchor.as_path())
                || alias.link_target_sha256 != sha256_bytes(&target_bytes)
                || alias.length != target_bytes.len() as u64
                || expanded != evidence.probe_cwd
                || alias.mode & file_type_mask != symlink_file_type
            {
                return Err(WorkVcsError::StorageFailure(
                    "automatic source archive alias fingerprint or expansion is contradictory"
                        .to_owned(),
                ));
            }
            Ok(())
        }
        _ => Err(WorkVcsError::StorageFailure(
            "automatic source archive alias evidence contradicts its resolution scheme".to_owned(),
        )),
    }
}

fn validate_native_metadata_basis(evidence: &SourceArchiveOwnerResolutionEvidence) -> Result<()> {
    let basis = &evidence.native_metadata_basis;
    require_absolute_clean_path(&basis.identity_root, "native owner identity root")?;
    if evidence.native_metadata_sha256 != sha256_json(basis)? {
        return Err(WorkVcsError::StorageFailure(
            "automatic source archive native metadata digest does not match its structured basis"
                .to_owned(),
        ));
    }
    for component in &basis.components {
        if component.label.is_empty() || component.bytes == 0 || !is_sha256(&component.sha256) {
            return Err(WorkVcsError::StorageFailure(
                "automatic source archive native metadata component is incomplete".to_owned(),
            ));
        }
        require_absolute_clean_path(&component.path, "native metadata component")?;
    }
    match evidence.native_identity_kind.as_str() {
        "cwd"
            if basis.marker_kind == "none"
                && basis.marker_path.is_none()
                && basis.git_admin_path.is_none()
                && basis.components.is_empty()
                && basis.identity_root == evidence.native_identity_path
                && evidence.probe_cwd == basis.identity_root =>
        {
            Ok(())
        }
        "git-common-dir" if basis.marker_kind == "git_directory" => {
            let marker = basis.identity_root.join(".git");
            let head_is_exact = basis.components.first().is_some_and(|head| {
                basis.components.len() == 1
                    && head.label == "head"
                    && head.path == marker.join("HEAD")
                    && head.resolved_target.is_none()
            });
            if basis.marker_path.as_ref() != Some(&marker)
                || basis.git_admin_path.as_ref() != Some(&evidence.native_identity_path)
                || evidence.native_identity_path != marker
                || !evidence.probe_cwd.starts_with(&basis.identity_root)
                || !head_is_exact
            {
                return Err(WorkVcsError::StorageFailure(
                    "automatic source archive Git directory metadata basis is contradictory"
                        .to_owned(),
                ));
            }
            Ok(())
        }
        "git-common-dir" if basis.marker_kind == "git_worktree_pointer" => {
            let marker = basis.identity_root.join(".git");
            let Some(admin) = basis.git_admin_path.as_ref() else {
                return Err(WorkVcsError::StorageFailure(
                    "automatic source archive worktree metadata omitted its admin path".to_owned(),
                ));
            };
            let expected = [
                ("worktree-pointer", marker.clone(), Some(admin.clone())),
                (
                    "commondir",
                    admin.join("commondir"),
                    Some(evidence.native_identity_path.clone()),
                ),
                ("gitdir", admin.join("gitdir"), Some(marker.clone())),
                ("head", admin.join("HEAD"), None),
            ];
            let worktrees_root = evidence.native_identity_path.join("worktrees");
            let admin_is_direct_child = admin
                .strip_prefix(&worktrees_root)
                .is_ok_and(|relative| relative.components().count() == 1);
            if basis.marker_path.as_ref() != Some(&marker)
                || !evidence.probe_cwd.starts_with(&basis.identity_root)
                || !admin_is_direct_child
                || basis.components.len() != expected.len()
                || basis.components.iter().zip(expected).any(
                    |(component, (label, path, resolved_target))| {
                        component.label != label
                            || component.path != path
                            || component.resolved_target != resolved_target
                    },
                )
            {
                return Err(WorkVcsError::StorageFailure(
                    "automatic source archive worktree metadata basis is contradictory".to_owned(),
                ));
            }
            Ok(())
        }
        _ => Err(WorkVcsError::StorageFailure(
            "automatic source archive native metadata basis contradicts its identity kind"
                .to_owned(),
        )),
    }
}

fn normalize_absolute_path_without_io(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        return Err(WorkVcsError::StorageFailure(format!(
            "historical path {} is not absolute",
            path.display()
        )));
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::Normal(value) => normalized.push(value),
            Component::ParentDir => {
                return Err(WorkVcsError::StorageFailure(format!(
                    "historical path {} contains a parent-directory component",
                    path.display()
                )));
            }
        }
    }
    Ok(normalized)
}

fn historical_cwd_candidates_without_io(cwd: &Path) -> Result<Vec<PathBuf>> {
    let normalized = normalize_absolute_path_without_io(cwd)?;
    let mut candidates = vec![normalized.clone()];
    #[cfg(target_os = "macos")]
    for public_root in ["/var", "/tmp", "/etc"] {
        if normalized.starts_with(public_root) {
            let relative = normalized.strip_prefix("/").expect("absolute path");
            candidates.push(Path::new("/private").join(relative));
            break;
        }
        let private_root = Path::new("/private").join(
            Path::new(public_root)
                .strip_prefix("/")
                .expect("public root is absolute"),
        );
        if normalized.starts_with(&private_root) {
            let relative = normalized
                .strip_prefix("/private")
                .expect("private path prefix");
            candidates.push(Path::new("/").join(relative));
            break;
        }
    }
    candidates.sort();
    candidates.dedup();
    Ok(candidates)
}

#[cfg(unix)]
fn path_raw_bytes(path: &Path) -> Vec<u8> {
    path.as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
fn path_raw_bytes(path: &Path) -> Vec<u8> {
    path.to_string_lossy().as_bytes().to_vec()
}

fn validate_document_version_contract(
    manifest: &SourceArchiveManifest,
    file: &SourceArchiveFileManifest,
    document: &SourceArchiveDocument,
) -> Result<()> {
    if document.source_role == "assistant" && !manifest.include_assistant {
        return Err(WorkVcsError::StorageFailure(format!(
            "source archive document {} contradicts the user-only role policy",
            document.document_id
        )));
    }
    match manifest.format_version {
        LEGACY_ARCHIVE_FORMAT_VERSION => {
            let source_thread_id = document.source_thread_id.as_deref().unwrap_or_default();
            let source_message_id = document.source_message_id.as_deref().unwrap_or_default();
            if source_thread_id.is_empty()
                || source_message_id.is_empty()
                || document.source_container_id.is_some()
                || document.message_identity.is_some()
                || document.source_turn_id.is_some()
                || document.message_origin.is_some()
                || document.document_id
                    != source_document_id(
                        &document.source_sha256,
                        document.source_line,
                        source_message_id,
                        &document.source_role,
                        &document.text_sha256,
                    )
            {
                return Err(WorkVcsError::StorageFailure(format!(
                    "source archive document {} has contradictory v1 provenance",
                    document.document_id
                )));
            }
        }
        AUTOMATIC_ARCHIVE_FORMAT_VERSION => {
            let Some(container) = &file.container_identity else {
                return Err(WorkVcsError::StorageFailure(
                    "source archive v2 document has no file container identity".to_owned(),
                ));
            };
            let Some(message_identity) = &document.message_identity else {
                return Err(WorkVcsError::StorageFailure(format!(
                    "source archive document {} omitted v2 message identity",
                    document.document_id
                )));
            };
            if document.source_thread_id.is_some()
                || document.source_container_id.as_deref() != Some(&container.container_id)
                || document.message_origin != Some(SourceArchiveMessageOrigin::Unknown)
                || document
                    .source_turn_id
                    .as_deref()
                    .is_some_and(str::is_empty)
            {
                return Err(WorkVcsError::StorageFailure(format!(
                    "source archive document {} has contradictory container, origin, or lineage provenance",
                    document.document_id
                )));
            }
            match message_identity {
                SourceArchiveMessageIdentity::Native { source_message_id } => {
                    if !valid_native_message_id(source_message_id)
                        || document.source_message_id.as_deref() != Some(source_message_id)
                    {
                        return Err(WorkVcsError::StorageFailure(format!(
                            "source archive document {} has contradictory native identity",
                            document.document_id
                        )));
                    }
                }
                SourceArchiveMessageIdentity::DerivedOccurrenceV1 {
                    occurrence_id,
                    source_container_id,
                    source_snapshot_sha256,
                    source_line,
                    role,
                    text_sha256,
                } => {
                    if document.source_message_id.is_some()
                        || source_container_id != &container.container_id
                        || source_snapshot_sha256 != &document.source_sha256
                        || *source_line != document.source_line
                        || role != &document.source_role
                        || text_sha256 != &document.text_sha256
                        || occurrence_id
                            != &derived_occurrence_id(
                                &container.container_id,
                                &document.source_sha256,
                                document.source_line,
                                &document.source_role,
                                &document.text_sha256,
                            )
                    {
                        return Err(WorkVcsError::StorageFailure(format!(
                            "source archive document {} has contradictory derived occurrence identity",
                            document.document_id
                        )));
                    }
                }
            }
            if document.document_id
                != source_document_id_v2(
                    &container.container_id,
                    &document.source_sha256,
                    document.source_line,
                    message_identity,
                    &document.source_role,
                    &document.text_sha256,
                )?
            {
                return Err(WorkVcsError::StorageFailure(format!(
                    "source archive document {} has an invalid v2 document identity",
                    document.document_id
                )));
            }
        }
        _ => unreachable!("manifest version checked before document validation"),
    }
    Ok(())
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
    if let Err(error) = fs::symlink_metadata(&file.source_path) {
        return if error.kind() == std::io::ErrorKind::NotFound {
            SourceArchiveFileState::Missing
        } else {
            SourceArchiveFileState::Unreadable
        };
    }
    match safe_live_source_fingerprint(&file.source_path) {
        Ok((source_bytes, source_sha256))
            if source_bytes == file.source_bytes && source_sha256 == file.source_sha256 =>
        {
            SourceArchiveFileState::Unchanged
        }
        Ok(_) => SourceArchiveFileState::Changed,
        Err(_) => SourceArchiveFileState::Unreadable,
    }
}

#[cfg(unix)]
fn safe_live_source_fingerprint(path: &Path) -> Result<(u64, String)> {
    require_absolute_clean_path(path, "source archive live-observation path")?;
    let parent = path.parent().ok_or_else(|| {
        WorkVcsError::IntegrityInvalid(format!(
            "source archive live-observation path {} has no parent",
            path.display()
        ))
    })?;
    let name = path.file_name().ok_or_else(|| {
        WorkVcsError::IntegrityInvalid(format!(
            "source archive live-observation path {} has no file name",
            path.display()
        ))
    })?;
    let (mut source, opened_identity) =
        open_source_beneath_root(parent, Path::new(name), &mut |_| {})?;
    let mut hasher = Sha256::new();
    let mut source_bytes = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = source
            .read(&mut buffer)
            .map_err(storage_error("observe live source archive input", path))?;
        if read == 0 {
            break;
        }
        source_bytes = source_bytes.checked_add(read as u64).ok_or_else(|| {
            WorkVcsError::IntegrityInvalid(format!(
                "source archive live-observation size overflowed for {}",
                path.display()
            ))
        })?;
        hasher.update(&buffer[..read]);
    }
    let after_read = opened_identity_from_metadata(
        &source
            .metadata()
            .map_err(storage_error("reinspect live source archive input", path))?,
    );
    if after_read != opened_identity || source_bytes != opened_identity.length {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "source archive live-observation input {} changed while it was read",
            path.display()
        )));
    }
    Ok((source_bytes, hex::encode(hasher.finalize())))
}

#[cfg(not(unix))]
fn safe_live_source_fingerprint(path: &Path) -> Result<(u64, String)> {
    Err(WorkVcsError::ControlPlaneInvalid(format!(
        "safe source archive live observation for {} requires Unix no-follow directory handles",
        path.display()
    )))
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

fn source_container_id(source_path: &Path, source_sha256: &str) -> Result<String> {
    #[derive(Serialize)]
    struct ContainerBasis<'a> {
        scheme: &'a str,
        source_path: &'a Path,
        source_snapshot_sha256: &'a str,
    }
    sha256_json(&ContainerBasis {
        scheme: "source_snapshot_container_v1",
        source_path,
        source_snapshot_sha256: source_sha256,
    })
}

fn valid_native_message_id(value: &str) -> bool {
    let candidate = value.strip_prefix("msg_").unwrap_or(value);
    uuid::Uuid::parse_str(candidate)
        .is_ok_and(|parsed| parsed.hyphenated().to_string() == candidate)
}

fn derived_occurrence_id(
    source_container_id: &str,
    source_sha256: &str,
    source_line: usize,
    role: &str,
    text_sha256: &str,
) -> String {
    sha256_bytes(
        format!(
            "workvcs-derived-occurrence-v1\0{source_container_id}\0{source_sha256}\0{source_line}\0{role}\0{text_sha256}"
        )
        .as_bytes(),
    )
}

fn source_document_id_v2(
    source_container_id: &str,
    source_sha256: &str,
    source_line: usize,
    message_identity: &SourceArchiveMessageIdentity,
    role: &str,
    text_sha256: &str,
) -> Result<String> {
    #[derive(Serialize)]
    struct DocumentBasis<'a> {
        scheme: &'a str,
        source_container_id: &'a str,
        source_snapshot_sha256: &'a str,
        source_line: usize,
        message_identity: &'a SourceArchiveMessageIdentity,
        role: &'a str,
        text_sha256: &'a str,
    }
    sha256_json(&DocumentBasis {
        scheme: "workvcs-source-document-v2",
        source_container_id,
        source_snapshot_sha256: source_sha256,
        source_line,
        message_identity,
        role,
        text_sha256,
    })
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.as_bytes().iter().all(|byte| byte.is_ascii_hexdigit())
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

    fn write_snapshot_rollout(path: &Path, cwd: &Path) {
        let first_thread = uuid::Uuid::now_v7().to_string();
        let second_thread = uuid::Uuid::now_v7().to_string();
        let shared_turn = uuid::Uuid::now_v7().to_string();
        let native_assistant = format!("msg_{}", uuid::Uuid::now_v7());
        let native_user = uuid::Uuid::now_v7().to_string();
        let lines = [
            serde_json::json!({"type":"session_meta","payload":{"id":first_thread,"cwd":cwd}}),
            serde_json::json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"shared occurrence text"}],"internal_chat_message_metadata_passthrough":{"turn_id":shared_turn}}}),
            serde_json::json!({"type":"response_item","payload":{"type":"message","role":"assistant","id":native_assistant,"content":[{"type":"output_text","text":"selected assistant text"}],"internal_chat_message_metadata_passthrough":{"turn_id":shared_turn}}}),
            serde_json::json!({"type":"response_item","payload":{"type":"message","role":"developer","content":[{"type":"input_text","text":"developer excluded"}]}}),
            serde_json::json!({"type":"response_item","payload":{"type":"message","role":"system","content":[{"type":"input_text","text":"system excluded"}]}}),
            serde_json::json!({"type":"response_item","payload":{"type":"message","role":"tool","content":[{"type":"output_text","text":"tool excluded"}]}}),
            serde_json::json!({"type":"session_meta","payload":{"id":second_thread,"cwd":cwd}}),
            serde_json::json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"shared occurrence text"}],"internal_chat_message_metadata_passthrough":{"turn_id":shared_turn}}}),
            serde_json::json!({"type":"response_item","payload":{"type":"message","role":"user","id":native_user,"content":[{"type":"input_text","text":"native user text"}],"internal_chat_message_metadata_passthrough":{"turn_id":shared_turn}}}),
            serde_json::json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"image_url","url":"https://example.invalid/private"}]}}),
        ];
        let bytes = lines
            .into_iter()
            .map(|line| serde_json::to_string(&line).expect("JSON"))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(path, format!("{bytes}\n")).expect("write snapshot rollout");
    }

    fn capture_test_snapshot(
        temporary: &tempfile::TempDir,
        source: &Path,
    ) -> Result<SourceArchiveSnapshot> {
        SourceArchiveSnapshot::capture(
            temporary
                .path()
                .canonicalize()
                .expect("canonical temporary root"),
            source.canonicalize().expect("canonical source"),
        )
    }

    fn automatic_import_options(
        snapshot: &SourceArchiveSnapshot,
        identity: SearchSourceIdentity,
        archive_root: &Path,
        role_policy: SourceArchiveRolePolicy,
        observed_at_us: u128,
    ) -> SourceArchiveImportOptions {
        try_automatic_import_options(
            snapshot,
            identity,
            archive_root,
            role_policy,
            observed_at_us,
            &snapshot.source_root,
        )
        .expect("live verified automatic import options")
    }

    fn try_automatic_import_options(
        snapshot: &SourceArchiveSnapshot,
        identity: SearchSourceIdentity,
        archive_root: &Path,
        role_policy: SourceArchiveRolePolicy,
        observed_at_us: u128,
        policy_root: &Path,
    ) -> Result<SourceArchiveImportOptions> {
        try_automatic_import_options_with_route(
            snapshot,
            identity,
            archive_root,
            role_policy,
            observed_at_us,
            policy_root,
            SourceArchiveRegistryRoute::RegistryOnly(
                snapshot.source_root.join("test-project-registry.json"),
            ),
        )
    }

    fn try_automatic_import_options_with_route(
        snapshot: &SourceArchiveSnapshot,
        identity: SearchSourceIdentity,
        archive_root: &Path,
        role_policy: SourceArchiveRolePolicy,
        observed_at_us: u128,
        policy_root: &Path,
        registry_route: SourceArchiveRegistryRoute,
    ) -> Result<SourceArchiveImportOptions> {
        let observed_session_meta: Vec<SourceArchiveAttributedSessionMeta> = snapshot
            .observed_session_meta()
            .iter()
            .map(|metadata| {
                let raw_cwd = PathBuf::from(metadata.raw_cwd.as_deref().expect("snapshot cwd"));
                let canonical_cwd = raw_cwd.canonicalize().expect("canonical owner cwd");
                let native_metadata_basis = SourceArchiveNativeMetadataBasis {
                    identity_root: canonical_cwd.clone(),
                    marker_kind: "none".to_owned(),
                    marker_path: None,
                    git_admin_path: None,
                    components: Vec::new(),
                };
                SourceArchiveAttributedSessionMeta {
                    source_line: metadata.source_line,
                    thread_id: metadata
                        .thread_id
                        .as_deref()
                        .expect("snapshot thread")
                        .to_owned(),
                    raw_cwd: raw_cwd.clone(),
                    owner_resolution: SourceArchiveOwnerResolutionEvidence {
                        scheme: SourceArchiveOwnerResolutionScheme::NativeIdentityV1,
                        normalized_cwd: raw_cwd.clone(),
                        candidate_cwd: canonical_cwd.clone(),
                        boundary: canonical_cwd.clone(),
                        probe_cwd: canonical_cwd.clone(),
                        native_identity_kind: "cwd".to_owned(),
                        native_identity: canonical_cwd.display().to_string(),
                        native_identity_path: canonical_cwd,
                        native_metadata_sha256: sha256_json(&native_metadata_basis)
                            .expect("native metadata digest"),
                        native_metadata_basis,
                        alias: None,
                    },
                }
            })
            .collect();
        let native_owner = observed_session_meta[0]
            .owner_resolution
            .native_identity_path
            .clone();
        let registry_id =
            crate::RegistryId::parse_canonical("01900000-0000-7000-8000-000000000010")
                .expect("registry id");
        let registry_value = serde_json::json!({
            "version": 2,
            "registry_id": registry_id,
            "revision": 7,
            "projects": [{
                "project_ref_id": identity.project_ref_id,
                "maturity": "provisional",
                "display_name": null,
                "created_at": "2026-01-01T00:00:00Z",
                "created_by": "cwd_first_write"
            }],
            "locators": [{
                "locator_id": crate::ProjectLocatorId::parse_canonical(
                    "01900000-0000-7000-8000-000000000020"
                ).expect("locator id"),
                "project_ref_id": identity.project_ref_id,
                "role": "identity",
                "authority": "cwd",
                "provider": "filesystem",
                "namespace": format!("registry:{registry_id}"),
                "kind": "canonical_directory",
                "normalized_value": native_owner,
                "assurance": "verified_derived",
                "source_adapter": "workvcs-filesystem-v1",
                "evidence_digest": crate::control_plane::ControlPlaneDigest::raw(b"test-owner"),
                "observed_at": "2026-01-01T00:00:00Z",
                "state": "active"
            }],
            "bindings": [{
                "project_ref_id": identity.project_ref_id,
                "store_path": snapshot.source_root.join("test.sqlite"),
                "store_id": identity.store_id,
                "workspace_id": identity.workspace_id,
                "branch_id": identity.branch_id,
                "bound_at": "2026-01-01T00:00:00Z",
                "binding_source": "first_write"
            }],
            "links": [],
            "observations": [],
            "migration": null
        });
        let registry = crate::control_plane::ProjectRegistryV2::from_json_bytes(
            &serde_json::to_vec(&registry_value).expect("registry JSON"),
        )
        .expect("registry snapshot");
        let policy_role = match role_policy {
            SourceArchiveRolePolicy::UserOnly => crate::automation::AutomationRolePolicy::UserOnly,
            SourceArchiveRolePolicy::UserAndAssistant => {
                crate::automation::AutomationRolePolicy::UserAndAssistant
            }
        };
        let source_policy_entries = vec![crate::automation::AutomationSourcePolicyEntry {
            source_id: "rollouts".to_owned(),
            revision: 3,
            rollout_root: policy_root.to_path_buf(),
            role_policy: policy_role,
            excluded_project_refs: Vec::new(),
            enabled: true,
            revoked: false,
            created_at_us: 1,
            updated_at_us: 1,
        }];
        #[derive(Serialize)]
        struct PolicyBody<'a> {
            format_version: u32,
            revision: u64,
            sources: &'a [crate::automation::AutomationSourcePolicyEntry],
        }
        let policy_body = PolicyBody {
            format_version: crate::automation::AUTOMATION_SOURCE_POLICY_FORMAT_VERSION,
            revision: 11,
            sources: &source_policy_entries,
        };
        let automation_root = snapshot.source_root.join("test-automation-control");
        fs::create_dir_all(&automation_root).expect("automation control root");
        #[cfg(unix)]
        fs::set_permissions(&automation_root, fs::Permissions::from_mode(0o700))
            .expect("protect automation control root");
        let policy_path = crate::automation::automation_source_policy_path(&automation_root);
        fs::write(
            &policy_path,
            serde_json::to_vec(&policy_body).expect("source policy JSON"),
        )
        .expect("write source policy");
        #[cfg(unix)]
        fs::set_permissions(&policy_path, fs::Permissions::from_mode(0o600))
            .expect("protect source policy");
        let registry_path = registry_route.registry_path();
        fs::create_dir_all(registry_path.parent().expect("registry parent"))
            .expect("registry parent");
        #[cfg(unix)]
        fs::set_permissions(
            registry_path.parent().expect("registry parent"),
            fs::Permissions::from_mode(0o700),
        )
        .expect("protect registry parent");
        fs::write(
            &registry_path,
            registry.stored_json_bytes().expect("stored registry"),
        )
        .expect("write registry");
        #[cfg(unix)]
        fs::set_permissions(&registry_path, fs::Permissions::from_mode(0o600))
            .expect("protect registry");
        let activation = crate::control_plane::RoutingActivationCandidate::for_registry(&registry)
            .expect("routing activation");
        let activation_path = registry_route.routing_activation_path(&registry_path);
        fs::create_dir_all(activation_path.parent().expect("activation parent"))
            .expect("activation parent");
        #[cfg(unix)]
        fs::set_permissions(
            activation_path.parent().expect("activation parent"),
            fs::Permissions::from_mode(0o700),
        )
        .expect("protect activation parent");
        fs::write(
            &activation_path,
            activation
                .stored_json_bytes()
                .expect("stored routing activation"),
        )
        .expect("write routing activation");
        #[cfg(unix)]
        fs::set_permissions(&activation_path, fs::Permissions::from_mode(0o600))
            .expect("protect routing activation");
        let request = SourceArchiveRegistryVerifiedImportRequest::new(
            registry_route,
            SourceArchiveAutomationRoot::Explicit(automation_root),
            archive_root,
            snapshot.clone(),
            "rollouts",
        )?;
        let (resolved_identity, mut attribution) = issue_live_automatic_attribution(&request)?;
        if resolved_identity != identity
            || attribution.attribution.observed_session_meta != observed_session_meta
        {
            return Err(WorkVcsError::IntegrityInvalid(
                "test registry-verified publisher did not independently reproduce its expected owner"
                    .to_owned(),
            ));
        }
        attribution.attribution.observed_at_us = observed_at_us;
        validate_automatic_attribution_evidence(&attribution.attribution)?;
        SourceArchiveImportOptions::from_snapshot(
            resolved_identity,
            archive_root,
            snapshot.clone(),
            attribution,
        )
    }

    fn read_documents(path: &Path) -> Vec<SourceArchiveDocument> {
        serde_json::from_slice(&fs::read(path.join(DOCUMENTS_FILE)).expect("read source documents"))
            .expect("decode source documents")
    }

    fn rewrite_documents_and_manifest(
        archive: &Path,
        documents: &[SourceArchiveDocument],
        manifest: &mut SourceArchiveManifest,
    ) {
        let document_bytes = serde_json::to_vec_pretty(documents).expect("encode documents");
        fs::write(archive.join(DOCUMENTS_FILE), &document_bytes).expect("rewrite documents");
        manifest.documents_sha256 = sha256_bytes(&document_bytes);
        manifest.manifest_sha256 = manifest_sha256(manifest).expect("manifest digest");
        fs::write(
            archive.join(MANIFEST_FILE),
            serde_json::to_vec_pretty(manifest).expect("encode manifest"),
        )
        .expect("rewrite manifest");
    }

    fn rewrite_automatic_manifest(archive: &Path, manifest: &mut SourceArchiveManifest) {
        let attribution = manifest
            .automatic_attribution
            .as_ref()
            .expect("automatic attribution");
        manifest.basis_sha256 = sha256_json(&AutomaticArchiveBasis {
            format_version: manifest.format_version,
            source_format: manifest.source_format,
            extraction_policy_version: &manifest.extraction_policy_version,
            identity: manifest.identity,
            include_assistant: manifest.include_assistant,
            authority_class: &manifest.authority_class,
            currentness_class: &manifest.currentness_class,
            action_authority: &manifest.action_authority,
            files: &manifest.files,
            automatic_attribution: automatic_attribution_basis(attribution),
        })
        .expect("automatic basis digest");
        manifest.manifest_sha256 = manifest_sha256(manifest).expect("manifest digest");
        fs::write(
            archive.join(MANIFEST_FILE),
            serde_json::to_vec_pretty(manifest).expect("encode automatic manifest"),
        )
        .expect("rewrite automatic manifest");
    }

    fn refresh_embedded_policy_digest(manifest: &mut SourceArchiveManifest) {
        #[derive(Serialize)]
        struct PolicyBody<'a> {
            format_version: u32,
            revision: u64,
            sources: &'a [crate::automation::AutomationSourcePolicyEntry],
        }
        let attribution = manifest
            .automatic_attribution
            .as_mut()
            .expect("automatic attribution");
        let basis = &mut attribution.source_policy_basis;
        basis.source_policy_digest_sha256 = sha256_json(&PolicyBody {
            format_version: basis.source_policy_format_version,
            revision: basis.source_policy_revision,
            sources: &basis.source_policy_entries,
        })
        .expect("source policy digest");
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
        let original_manifest_bytes =
            fs::read(imported.archive_path.join(MANIFEST_FILE)).expect("read v1 manifest");
        let original_document_bytes =
            fs::read(imported.archive_path.join(DOCUMENTS_FILE)).expect("read v1 documents");
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
        assert_eq!(
            fs::read(imported.archive_path.join(MANIFEST_FILE)).expect("re-read v1 manifest"),
            original_manifest_bytes,
            "v1 inspection and query must not migrate or rewrite the manifest"
        );
        assert_eq!(
            fs::read(imported.archive_path.join(DOCUMENTS_FILE)).expect("re-read v1 documents"),
            original_document_bytes,
            "v1 inspection and query must not migrate or rewrite documents"
        );
    }

    #[test]
    fn automatic_snapshot_preserves_occurrences_lineage_and_all_metadata() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let source = temporary.path().join("rollout.jsonl");
        let root = temporary.path().join("archives");
        let cwd = temporary.path().join("owner");
        fs::create_dir(&cwd).expect("owner cwd");
        write_snapshot_rollout(&source, &cwd);
        let snapshot = capture_test_snapshot(&temporary, &source).expect("capture snapshot");
        let identity = identity();
        let first_options = automatic_import_options(
            &snapshot,
            identity,
            &root,
            SourceArchiveRolePolicy::UserAndAssistant,
            10,
        );
        let imported = import_source_archive(&first_options).expect("snapshot import");
        assert_eq!(
            imported.manifest.format_version,
            AUTOMATIC_ARCHIVE_FORMAT_VERSION
        );
        assert_eq!(imported.manifest.files[0].observed_session_meta.len(), 2);
        assert_eq!(
            imported
                .manifest
                .automatic_attribution
                .as_ref()
                .expect("automatic attribution")
                .observed_session_meta
                .len(),
            2
        );
        let documents = read_documents(&imported.archive_path);
        assert_eq!(documents.len(), 4);
        assert!(documents.iter().all(|document| {
            document.source_thread_id.is_none()
                && document.message_origin == Some(SourceArchiveMessageOrigin::Unknown)
                && document.source_turn_id.is_some()
                && !document.text.contains("excluded")
                && !document.text.contains("example.invalid")
        }));
        let derived = documents
            .iter()
            .filter_map(|document| match document.message_identity.as_ref() {
                Some(SourceArchiveMessageIdentity::DerivedOccurrenceV1 {
                    occurrence_id, ..
                }) => Some((document, occurrence_id)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(derived.len(), 2);
        assert!(
            derived
                .iter()
                .all(|(document, _)| document.source_message_id.is_none())
        );
        assert_ne!(
            derived[0].1, derived[1].1,
            "physical occurrences must not fold"
        );
        assert_ne!(
            derived[0].0.document_id, derived[1].0.document_id,
            "repeated turn ids must not collapse separate messages"
        );
        assert_eq!(
            derived[0].0.source_turn_id, derived[1].0.source_turn_id,
            "same-turn physical messages must preserve their shared lineage"
        );
        let native = documents
            .iter()
            .filter(|document| {
                matches!(
                    document.message_identity,
                    Some(SourceArchiveMessageIdentity::Native { .. })
                )
            })
            .count();
        assert_eq!(native, 2);

        let second_options = automatic_import_options(
            &snapshot,
            identity,
            &root,
            SourceArchiveRolePolicy::UserAndAssistant,
            99,
        );
        let reused = import_source_archive(&second_options).expect("repeat snapshot import");
        assert_eq!(reused.outcome, SourceArchiveImportOutcome::Reused);
        assert_eq!(reused.archive_path, imported.archive_path);
        assert_eq!(
            reused
                .manifest
                .automatic_attribution
                .as_ref()
                .expect("stored attribution")
                .observed_at_us,
            10,
            "observation time must not fork an unchanged immutable basis"
        );
    }

    #[test]
    fn automatic_user_only_and_container_identity_are_source_qualified() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let first_source = temporary.path().join("first.jsonl");
        let second_source = temporary.path().join("second.jsonl");
        let root = temporary.path().join("archives");
        let cwd = temporary.path().join("owner");
        fs::create_dir(&cwd).expect("owner cwd");
        write_snapshot_rollout(&first_source, &cwd);
        fs::copy(&first_source, &second_source).expect("copy identical rollout bytes");
        let identity = identity();
        let mut imports = Vec::new();
        for source in [&first_source, &second_source] {
            let snapshot = capture_test_snapshot(&temporary, source).expect("capture snapshot");
            let options = automatic_import_options(
                &snapshot,
                identity,
                &root,
                SourceArchiveRolePolicy::UserOnly,
                10,
            );
            imports.push(import_source_archive(&options).expect("snapshot import"));
        }
        assert_ne!(imports[0].archive_path, imports[1].archive_path);
        let first_documents = read_documents(&imports[0].archive_path);
        let second_documents = read_documents(&imports[1].archive_path);
        assert_eq!(first_documents.len(), 3);
        assert_eq!(second_documents.len(), 3);
        assert!(
            first_documents
                .iter()
                .all(|document| document.source_role == "user")
        );
        assert!(
            second_documents
                .iter()
                .all(|document| document.source_role == "user")
        );
        assert_ne!(
            imports[0].manifest.files[0]
                .container_identity
                .as_ref()
                .expect("first container")
                .container_id,
            imports[1].manifest.files[0]
                .container_identity
                .as_ref()
                .expect("second container")
                .container_id,
            "byte-identical files at different source paths need distinct containers"
        );
        let first_ids = first_documents
            .iter()
            .map(|document| document.document_id.as_str())
            .collect::<BTreeSet<_>>();
        let second_ids = second_documents
            .iter()
            .map(|document| document.document_id.as_str())
            .collect::<BTreeSet<_>>();
        assert!(first_ids.is_disjoint(&second_ids));
    }

    #[cfg(unix)]
    #[test]
    fn live_source_fence_does_not_retain_snapshot_payload_or_parsed_tree() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let source = temporary.path().join("thread.jsonl");
        let cwd = temporary.path().join("owner");
        fs::create_dir(&cwd).expect("owner cwd");
        write_snapshot_rollout(&source, &cwd);
        let snapshot = capture_test_snapshot(&temporary, &source).expect("capture snapshot");
        let weak_bytes = Arc::downgrade(&snapshot.bytes);
        let weak_parsed = Arc::downgrade(&snapshot.parsed);
        let fence = snapshot.live_source_fence();

        drop(snapshot);

        assert!(
            weak_bytes.upgrade().is_none(),
            "the publication fence must not retain source bytes"
        );
        assert!(
            weak_parsed.upgrade().is_none(),
            "the publication fence must not retain the parsed JSON tree"
        );
        assert!(fence.verify_live_source().expect("verify live source"));
    }

    #[cfg(unix)]
    #[test]
    fn historical_alias_resolution_is_anchor_bounded_and_rejects_ambiguity() {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::tempdir().expect("temporary directory");
        let root = temporary
            .path()
            .canonicalize()
            .expect("canonical fixture root");
        let anchor = root.join("Users").join("ld");
        let projects = anchor.join("Projects");
        let project = projects.join("project");
        fs::create_dir_all(&project).expect("physical project");
        let workspaces = anchor.join("Workspaces");
        symlink(&projects, &workspaces).expect("bounded workspace alias");
        let candidate = workspaces.join("project");
        let identity = identity();
        let owners = BTreeMap::from([(
            SourceArchiveNativeOwner {
                identity_kind: "cwd".to_owned(),
                identity_path: project.clone(),
            },
            identity,
        )]);
        let (direct_identity, direct_evidence) = resolve_snapshot_cwd_owner(
            &project,
            &owners,
            std::slice::from_ref(&project),
            &root.join("registry.json"),
        )
        .expect("the same live target resolves directly");
        assert_eq!(direct_identity, identity);
        assert_eq!(direct_evidence.probe_cwd, project);
        assert!(direct_evidence.alias.is_none());
        let (resolved, evidence) = resolve_snapshot_cwd_owner(
            &candidate,
            &owners,
            std::slice::from_ref(&project),
            &root.join("registry.json"),
        )
        .expect("bounded alias resolves");
        assert_eq!(resolved, identity);
        assert_eq!(evidence.candidate_cwd, candidate);
        assert_eq!(evidence.probe_cwd, project);
        let alias = evidence.alias.expect("alias evidence");
        assert_eq!(alias.anchor, anchor);
        assert_eq!(alias.link_path, workspaces);

        let external = tempfile::tempdir_in("/tmp").expect("external lexical root");
        let external_alias = external.path().join("foreign-project");
        symlink(&project, &external_alias).expect("real external alias");
        let error = resolve_snapshot_cwd_owner(
            &external_alias,
            &owners,
            std::slice::from_ref(&project),
            &root.join("registry.json"),
        )
        .expect_err("an alias outside a sufficiently deep shared anchor must fail closed");
        assert!(matches!(error, WorkVcsError::ProjectOwnerUnbound { .. }));

        let other_projects = anchor.join("OtherProjects");
        let other_project = other_projects.join("project");
        fs::create_dir_all(&other_project).expect("second physical project");
        let archives = anchor.join("Archives");
        symlink(&other_projects, &archives).expect("second bounded alias");
        let ambiguous = resolve_trusted_alias_candidates(
            &[candidate, archives.join("project")],
            &[project, other_project],
        )
        .expect_err("distinct verified probes must not be selected by ordering");
        assert!(matches!(ambiguous, WorkVcsError::ControlPlaneInvalid(_)));
        assert!(ambiguous.to_string().contains("ambiguous"));
    }

    #[cfg(unix)]
    #[test]
    fn registry_verified_factory_requires_live_control_plane_observation() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let source = temporary.path().join("thread.jsonl");
        let cwd = temporary.path().join("owner");
        fs::create_dir(&cwd).expect("owner cwd");
        write_snapshot_rollout(&source, &cwd);
        let snapshot = capture_test_snapshot(&temporary, &source).expect("capture snapshot");
        let fixture_root = temporary
            .path()
            .canonicalize()
            .expect("canonical fixture root");

        let request = SourceArchiveRegistryVerifiedImportRequest::new(
            SourceArchiveRegistryRoute::RegistryOnly(
                fixture_root.join("missing-project-registry.json"),
            ),
            SourceArchiveAutomationRoot::Explicit(fixture_root.join("missing-automation-root")),
            fixture_root.join("archives"),
            snapshot,
            "rollouts",
        )
        .expect("request shape");
        let error = import_registry_verified_source_archive(request)
            .expect_err("offline evidence must not issue registry-verified attribution");

        assert!(
            error.to_string().contains("missing-project-registry.json"),
            "{error}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn registry_verified_publisher_binds_home_and_registry_only_to_their_exact_markers() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let fixture_root = temporary.path().canonicalize().expect("fixture root");
        let source = fixture_root.join("thread.jsonl");
        let cwd = fixture_root.join("owner");
        fs::create_dir(&cwd).expect("owner cwd");
        write_snapshot_rollout(&source, &cwd);
        let snapshot = SourceArchiveSnapshot::capture(&fixture_root, &source).expect("snapshot");
        let home = fixture_root.join("selected-home");
        let expected_identity = identity();
        try_automatic_import_options_with_route(
            &snapshot,
            expected_identity,
            &fixture_root.join("unused-options"),
            SourceArchiveRolePolicy::UserOnly,
            1,
            &fixture_root,
            SourceArchiveRegistryRoute::Home(home.clone()),
        )
        .expect("configured-home publisher setup");

        let request = SourceArchiveRegistryVerifiedImportRequest::new(
            SourceArchiveRegistryRoute::Home(home.clone()),
            SourceArchiveAutomationRoot::Explicit(fixture_root.join("test-automation-control")),
            fixture_root.join("home-archives"),
            snapshot.clone(),
            "rollouts",
        )
        .expect("home request");
        let imported = import_registry_verified_source_archive(request).expect("home import");
        assert_eq!(imported.member_identity, expected_identity);

        let wrong_route = SourceArchiveRegistryVerifiedImportRequest::new(
            SourceArchiveRegistryRoute::RegistryOnly(home.join(STANDARD_PROJECT_REGISTRY_FILE)),
            SourceArchiveAutomationRoot::Explicit(fixture_root.join("test-automation-control")),
            fixture_root.join("wrong-route-archives"),
            snapshot,
            "rollouts",
        )
        .expect("wrong route request");
        let error = import_registry_verified_source_archive(wrong_route)
            .expect_err("a home marker must not activate a registry-only route");
        assert!(
            error.to_string().contains("project-bindings.json.d"),
            "{error}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn two_explicit_registry_control_planes_remain_independently_valid() {
        for suffix in ["a", "b"] {
            let temporary = tempfile::tempdir().expect("temporary directory");
            let fixture_root = temporary.path().canonicalize().expect("fixture root");
            let source = fixture_root.join("thread.jsonl");
            let cwd = fixture_root.join("owner");
            fs::create_dir(&cwd).expect("owner cwd");
            write_snapshot_rollout(&source, &cwd);
            let snapshot =
                SourceArchiveSnapshot::capture(&fixture_root, &source).expect("snapshot");
            let registry = fixture_root.join(format!("registry-{suffix}.json"));
            let expected_identity = identity();
            try_automatic_import_options_with_route(
                &snapshot,
                expected_identity,
                &fixture_root.join("unused-options"),
                SourceArchiveRolePolicy::UserOnly,
                1,
                &fixture_root,
                SourceArchiveRegistryRoute::RegistryOnly(registry.clone()),
            )
            .expect("explicit publisher setup");
            let request = SourceArchiveRegistryVerifiedImportRequest::new(
                SourceArchiveRegistryRoute::RegistryOnly(registry),
                SourceArchiveAutomationRoot::Explicit(fixture_root.join("test-automation-control")),
                fixture_root.join("archives"),
                snapshot,
                "rollouts",
            )
            .expect("explicit request");
            assert_eq!(
                import_registry_verified_source_archive(request)
                    .expect("explicit import")
                    .member_identity,
                expected_identity
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn automatic_snapshot_rejects_leaf_and_intermediate_symlink_swaps() {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::tempdir().expect("temporary directory");
        let fixture_root = temporary
            .path()
            .canonicalize()
            .expect("canonical fixture root");
        let source_root = fixture_root.join("rollouts");
        let outside_root = fixture_root.join("outside");
        fs::create_dir(&source_root).expect("source root");
        fs::create_dir(&outside_root).expect("outside root");
        let cwd = fixture_root.join("owner");
        fs::create_dir(&cwd).expect("owner cwd");

        let leaf = source_root.join("leaf.jsonl");
        let outside_leaf = outside_root.join("leaf.jsonl");
        write_snapshot_rollout(&leaf, &cwd);
        write_snapshot_rollout(&outside_leaf, &cwd);
        let mut swapped_leaf = false;
        let leaf_error = SourceArchiveSnapshot::capture_with_hook(&source_root, &leaf, |cursor| {
            if !swapped_leaf && cursor == leaf {
                fs::remove_file(&leaf).expect("remove raced leaf");
                symlink(&outside_leaf, &leaf).expect("swap leaf to outside symlink");
                swapped_leaf = true;
            }
        })
        .expect_err("leaf symlink swap must fail closed");
        assert!(swapped_leaf);
        assert!(matches!(
            leaf_error,
            WorkVcsError::StorageFailure(_) | WorkVcsError::IntegrityInvalid(_)
        ));

        fs::remove_file(&leaf).expect("remove leaf symlink");
        let fifo = source_root.join("fifo.jsonl");
        write_snapshot_rollout(&fifo, &cwd);
        let mut swapped_fifo = false;
        let fifo_error = SourceArchiveSnapshot::capture_with_hook(&source_root, &fifo, |cursor| {
            if !swapped_fifo && cursor == fifo {
                fs::remove_file(&fifo).expect("remove raced FIFO leaf");
                let fifo_name = CString::new(fifo.as_os_str().as_bytes()).expect("FIFO path");
                let result = unsafe { libc::mkfifo(fifo_name.as_ptr(), 0o600) };
                assert_eq!(
                    result,
                    0,
                    "create raced FIFO: {}",
                    std::io::Error::last_os_error()
                );
                swapped_fifo = true;
            }
        })
        .expect_err("FIFO replacement must fail without blocking");
        assert!(swapped_fifo);
        assert!(matches!(
            fifo_error,
            WorkVcsError::StorageFailure(_) | WorkVcsError::IntegrityInvalid(_)
        ));
        fs::remove_file(&fifo).expect("remove FIFO");

        let nested = source_root.join("nested");
        let displaced = source_root.join("displaced");
        let outside_nested = outside_root.join("nested");
        fs::create_dir(&nested).expect("nested source directory");
        fs::create_dir(&outside_nested).expect("outside nested directory");
        let nested_source = nested.join("thread.jsonl");
        write_snapshot_rollout(&nested_source, &cwd);
        write_snapshot_rollout(&outside_nested.join("thread.jsonl"), &cwd);
        let mut swapped_directory = false;
        let directory_error =
            SourceArchiveSnapshot::capture_with_hook(&source_root, &nested_source, |cursor| {
                if !swapped_directory && cursor == nested {
                    fs::rename(&nested, &displaced).expect("displace inspected directory");
                    symlink(&outside_nested, &nested)
                        .expect("swap intermediate directory to outside symlink");
                    swapped_directory = true;
                }
            })
            .expect_err("intermediate symlink swap must fail closed");
        assert!(swapped_directory);
        assert!(matches!(
            directory_error,
            WorkVcsError::StorageFailure(_) | WorkVcsError::IntegrityInvalid(_)
        ));
    }

    #[test]
    fn automatic_snapshot_rejects_malformed_native_ids_and_forged_v2_provenance() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let cwd = temporary.path().join("owner");
        fs::create_dir(&cwd).expect("owner cwd");
        let invalid_json = temporary.path().join("invalid.jsonl");
        fs::write(&invalid_json, b"{not-json}\n").expect("write invalid JSON");
        assert!(
            capture_test_snapshot(&temporary, &invalid_json).is_err(),
            "invalid JSON must fail while capturing the immutable snapshot"
        );
        let no_selected = temporary.path().join("no-selected.jsonl");
        let no_selected_lines = [
            serde_json::json!({"type":"session_meta","payload":{"id":uuid::Uuid::now_v7(),"cwd":cwd}}),
            serde_json::json!({"type":"response_item","payload":{"type":"message","role":"developer","content":[{"type":"input_text","text":"not selectable"}]}}),
            serde_json::json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"image_url","url":"https://example.invalid/private"}]}}),
        ];
        fs::write(
            &no_selected,
            format!(
                "{}\n",
                no_selected_lines
                    .into_iter()
                    .map(|line| serde_json::to_string(&line).expect("JSON"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        )
        .expect("write no-selected rollout");
        let no_selected_snapshot =
            capture_test_snapshot(&temporary, &no_selected).expect("capture no-selected");
        let no_selected_identity = identity();
        let no_selected_options = automatic_import_options(
            &no_selected_snapshot,
            no_selected_identity,
            &temporary.path().join("no-selected-archives"),
            SourceArchiveRolePolicy::UserOnly,
            10,
        );
        assert!(
            import_source_archive(&no_selected_options).is_err(),
            "non-selected roles and non-text user content must not create an archive"
        );
        let malformed = temporary.path().join("malformed.jsonl");
        let thread = uuid::Uuid::now_v7();
        let lines = [
            serde_json::json!({"type":"session_meta","payload":{"id":thread,"cwd":cwd}}),
            serde_json::json!({"type":"response_item","payload":{"type":"message","role":"user","id":"not-a-native-id","content":[{"type":"input_text","text":"do not derive over malformed identity"}]}}),
        ];
        fs::write(
            &malformed,
            format!(
                "{}\n",
                lines
                    .into_iter()
                    .map(|line| serde_json::to_string(&line).expect("JSON"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        )
        .expect("write malformed rollout");
        let identity = identity();
        let snapshot = capture_test_snapshot(&temporary, &malformed).expect("capture malformed");
        let options = automatic_import_options(
            &snapshot,
            identity,
            &temporary.path().join("bad-archives"),
            SourceArchiveRolePolicy::UserOnly,
            10,
        );
        assert!(
            import_source_archive(&options).is_err(),
            "a malformed present native id must not be treated as missing"
        );

        let source = temporary.path().join("valid.jsonl");
        write_snapshot_rollout(&source, &cwd);
        let snapshot = capture_test_snapshot(&temporary, &source).expect("capture valid");
        let options = automatic_import_options(
            &snapshot,
            identity,
            &temporary.path().join("valid-archives"),
            SourceArchiveRolePolicy::UserOnly,
            10,
        );
        let imported = import_source_archive(&options).expect("valid import");
        let mut documents = read_documents(&imported.archive_path);
        let derived = documents
            .iter_mut()
            .find(|document| {
                matches!(
                    document.message_identity,
                    Some(SourceArchiveMessageIdentity::DerivedOccurrenceV1 { .. })
                )
            })
            .expect("derived document");
        let Some(SourceArchiveMessageIdentity::DerivedOccurrenceV1 { occurrence_id, .. }) =
            derived.message_identity.as_mut()
        else {
            unreachable!("selected derived identity")
        };
        *occurrence_id = "0".repeat(64);
        let mut manifest = imported.manifest.clone();
        rewrite_documents_and_manifest(&imported.archive_path, &documents, &mut manifest);
        assert!(
            validate_archive(&imported.archive_path, Some(identity)).is_err(),
            "digest-consistent forged occurrence provenance must fail closed"
        );

        let provenance_source = temporary.path().join("provenance.jsonl");
        write_snapshot_rollout(&provenance_source, &cwd);
        let snapshot = capture_test_snapshot(&temporary, &provenance_source)
            .expect("capture provenance source");
        let options = automatic_import_options(
            &snapshot,
            identity,
            &temporary.path().join("provenance-archives"),
            SourceArchiveRolePolicy::UserOnly,
            10,
        );
        let provenance_import = import_source_archive(&options).expect("provenance import");
        let mut forged_manifest = provenance_import.manifest.clone();
        forged_manifest
            .automatic_attribution
            .as_mut()
            .expect("automatic attribution")
            .provenance_class = "canonical".to_owned();
        let forged_attribution = forged_manifest
            .automatic_attribution
            .as_ref()
            .expect("automatic attribution");
        forged_manifest.basis_sha256 = sha256_json(&AutomaticArchiveBasis {
            format_version: forged_manifest.format_version,
            source_format: forged_manifest.source_format,
            extraction_policy_version: &forged_manifest.extraction_policy_version,
            identity: forged_manifest.identity,
            include_assistant: forged_manifest.include_assistant,
            authority_class: &forged_manifest.authority_class,
            currentness_class: &forged_manifest.currentness_class,
            action_authority: &forged_manifest.action_authority,
            files: &forged_manifest.files,
            automatic_attribution: automatic_attribution_basis(forged_attribution),
        })
        .expect("forged basis digest");
        forged_manifest.manifest_sha256 =
            manifest_sha256(&forged_manifest).expect("forged manifest digest");
        fs::write(
            provenance_import.archive_path.join(MANIFEST_FILE),
            serde_json::to_vec_pretty(&forged_manifest).expect("encode forged manifest"),
        )
        .expect("write forged manifest");
        assert!(
            validate_archive(&provenance_import.archive_path, Some(identity)).is_err(),
            "digest-consistent canonical authority forgery must fail closed"
        );
    }

    #[cfg(unix)]
    #[test]
    fn automatic_readback_rejects_digest_consistent_false_owner_and_alias_closure() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let cwd = temporary.path().join("owner");
        fs::create_dir(&cwd).expect("owner cwd");
        let source = temporary.path().join("closure.jsonl");
        write_snapshot_rollout(&source, &cwd);
        let snapshot = capture_test_snapshot(&temporary, &source).expect("capture snapshot");
        let identity = identity();
        let options = automatic_import_options(
            &snapshot,
            identity,
            &temporary.path().join("closure-archives"),
            SourceArchiveRolePolicy::UserOnly,
            10,
        );
        let imported = import_source_archive(&options).expect("closure import");
        let original = imported.manifest.clone();

        let mut false_native_metadata = original.clone();
        false_native_metadata
            .automatic_attribution
            .as_mut()
            .expect("automatic attribution")
            .observed_session_meta[0]
            .owner_resolution
            .native_metadata_sha256 = "a".repeat(64);
        rewrite_automatic_manifest(&imported.archive_path, &mut false_native_metadata);
        assert!(
            validate_archive(&imported.archive_path, Some(identity)).is_err(),
            "an arbitrary shape-valid native metadata digest must fail"
        );

        let mut false_policy_digest = original.clone();
        false_policy_digest
            .automatic_attribution
            .as_mut()
            .expect("automatic attribution")
            .source_policy_basis
            .source_policy_digest_sha256 = "b".repeat(64);
        rewrite_automatic_manifest(&imported.archive_path, &mut false_policy_digest);
        assert!(
            validate_archive(&imported.archive_path, Some(identity)).is_err(),
            "an arbitrary shape-valid policy digest must fail"
        );

        let mut unrelated_probe = original.clone();
        let unrelated = temporary.path().join("unrelated-owner");
        unrelated_probe.files[0].observed_session_meta[0].raw_cwd = unrelated.clone();
        let attributed = &mut unrelated_probe
            .automatic_attribution
            .as_mut()
            .expect("automatic attribution")
            .observed_session_meta[0];
        attributed.raw_cwd = unrelated.clone();
        attributed.owner_resolution.normalized_cwd = unrelated.clone();
        attributed.owner_resolution.candidate_cwd = unrelated.clone();
        attributed.owner_resolution.boundary = temporary.path().to_path_buf();
        attributed.owner_resolution.probe_cwd = unrelated;
        rewrite_automatic_manifest(&imported.archive_path, &mut unrelated_probe);
        assert!(
            validate_archive(&imported.archive_path, Some(identity)).is_err(),
            "a digest-consistent valid registry owner with an unrelated cwd probe must fail"
        );

        let mut false_native = original.clone();
        let evidence = &mut false_native
            .automatic_attribution
            .as_mut()
            .expect("automatic attribution")
            .observed_session_meta[0]
            .owner_resolution;
        let false_path = cwd.parent().expect("cwd parent").join("false-owner");
        evidence.native_identity = false_path.display().to_string();
        evidence.native_identity_path = false_path.clone();
        evidence.native_metadata_basis.identity_root = false_path;
        evidence.native_metadata_sha256 =
            sha256_json(&evidence.native_metadata_basis).expect("false native metadata digest");
        rewrite_automatic_manifest(&imported.archive_path, &mut false_native);
        assert!(
            validate_archive(&imported.archive_path, Some(identity)).is_err(),
            "a shape-valid native owner outside the selected registry member must fail"
        );

        let mut wrong_target_hash = original.clone();
        let evidence = &mut wrong_target_hash
            .automatic_attribution
            .as_mut()
            .expect("automatic attribution")
            .observed_session_meta[0]
            .owner_resolution;
        let target = evidence.probe_cwd.clone();
        evidence.scheme = SourceArchiveOwnerResolutionScheme::SafeAliasV1;
        evidence.alias = Some(SourceArchiveAliasFingerprint {
            anchor: evidence
                .candidate_cwd
                .parent()
                .expect("candidate parent")
                .to_path_buf(),
            link_path: evidence.candidate_cwd.clone(),
            link_target: target.clone(),
            link_target_sha256: "0".repeat(64),
            device: 1,
            inode: 2,
            mode: portable_unix_mode(libc::S_IFLNK) | 0o777,
            uid: 1,
            gid: 1,
            length: path_raw_bytes(&target).len() as u64,
            modified_seconds: 1,
            modified_nanoseconds: 1,
            changed_seconds: 1,
            changed_nanoseconds: 1,
        });
        rewrite_automatic_manifest(&imported.archive_path, &mut wrong_target_hash);
        assert!(
            validate_archive(&imported.archive_path, Some(identity)).is_err(),
            "a digest-consistent manifest with the wrong raw alias-target hash must fail"
        );

        let mut unsafe_anchor = original.clone();
        let external_candidate = PathBuf::from("/tmp/workvcs-unsafe-anchor");
        unsafe_anchor.files[0].observed_session_meta[0].raw_cwd = external_candidate.clone();
        let attributed = &mut unsafe_anchor
            .automatic_attribution
            .as_mut()
            .expect("automatic attribution")
            .observed_session_meta[0];
        attributed.raw_cwd = external_candidate.clone();
        let evidence = &mut attributed.owner_resolution;
        evidence.normalized_cwd = external_candidate.clone();
        evidence.candidate_cwd = external_candidate.clone();
        let target = evidence.probe_cwd.clone();
        evidence.scheme = SourceArchiveOwnerResolutionScheme::SafeAliasV1;
        evidence.alias = Some(SourceArchiveAliasFingerprint {
            anchor: PathBuf::from("/tmp"),
            link_path: external_candidate,
            link_target: target.clone(),
            link_target_sha256: sha256_bytes(&path_raw_bytes(&target)),
            device: 1,
            inode: 2,
            mode: portable_unix_mode(libc::S_IFLNK) | 0o777,
            uid: 1,
            gid: 1,
            length: path_raw_bytes(&target).len() as u64,
            modified_seconds: 1,
            modified_nanoseconds: 1,
            changed_seconds: 1,
            changed_nanoseconds: 1,
        });
        rewrite_automatic_manifest(&imported.archive_path, &mut unsafe_anchor);
        assert!(
            validate_archive(&imported.archive_path, Some(identity)).is_err(),
            "a digest-consistent v2 alias outside a finite trusted anchor must fail"
        );

        let mut wrong_expansion = original;
        let evidence = &mut wrong_expansion
            .automatic_attribution
            .as_mut()
            .expect("automatic attribution")
            .observed_session_meta[0]
            .owner_resolution;
        let target = evidence.boundary.join("different-target");
        evidence.scheme = SourceArchiveOwnerResolutionScheme::SafeAliasV1;
        evidence.alias = Some(SourceArchiveAliasFingerprint {
            anchor: evidence
                .candidate_cwd
                .parent()
                .expect("candidate parent")
                .to_path_buf(),
            link_path: evidence.candidate_cwd.clone(),
            link_target: target.clone(),
            link_target_sha256: sha256_bytes(&path_raw_bytes(&target)),
            device: 1,
            inode: 2,
            mode: portable_unix_mode(libc::S_IFLNK) | 0o777,
            uid: 1,
            gid: 1,
            length: path_raw_bytes(&target).len() as u64,
            modified_seconds: 1,
            modified_nanoseconds: 1,
            changed_seconds: 1,
            changed_nanoseconds: 1,
        });
        rewrite_automatic_manifest(&imported.archive_path, &mut wrong_expansion);
        assert!(
            validate_archive(&imported.archive_path, Some(identity)).is_err(),
            "a digest-consistent alias whose expansion misses the recorded probe must fail"
        );
    }

    #[cfg(unix)]
    #[test]
    fn owner_path_closure_accepts_git_controls_and_rejects_forged_relationships() {
        let component = |label: &str, path: PathBuf, resolved_target: Option<PathBuf>| {
            SourceArchiveNativeMetadataComponent {
                label: label.to_owned(),
                path,
                bytes: 1,
                sha256: "0".repeat(64),
                resolved_target,
            }
        };
        let git_root = PathBuf::from("/repo");
        let git_dir = git_root.join(".git");
        let directory_basis = SourceArchiveNativeMetadataBasis {
            identity_root: git_root.clone(),
            marker_kind: "git_directory".to_owned(),
            marker_path: Some(git_dir.clone()),
            git_admin_path: Some(git_dir.clone()),
            components: vec![component("head", git_dir.join("HEAD"), None)],
        };
        let directory_evidence = SourceArchiveOwnerResolutionEvidence {
            scheme: SourceArchiveOwnerResolutionScheme::NativeIdentityV1,
            normalized_cwd: git_root.join("sub"),
            candidate_cwd: git_root.join("sub"),
            boundary: git_root.clone(),
            probe_cwd: git_root.join("sub"),
            native_identity_kind: "git-common-dir".to_owned(),
            native_identity: git_dir.display().to_string(),
            native_identity_path: git_dir.clone(),
            native_metadata_sha256: sha256_json(&directory_basis).unwrap(),
            native_metadata_basis: directory_basis,
            alias: None,
        };
        let directory_owner = SourceArchiveNativeOwner {
            identity_kind: "git-common-dir".to_owned(),
            identity_path: git_dir.clone(),
        };
        validate_owner_resolution_evidence(
            &directory_evidence,
            std::slice::from_ref(&directory_owner),
        )
        .expect("adjacent Git directory control");
        let mut unrelated = directory_evidence;
        unrelated.normalized_cwd = PathBuf::from("/unrelated");
        unrelated.candidate_cwd = PathBuf::from("/unrelated");
        unrelated.boundary = PathBuf::from("/");
        unrelated.probe_cwd = PathBuf::from("/unrelated");
        assert!(validate_owner_resolution_evidence(&unrelated, &[directory_owner]).is_err());

        let worktree_root = PathBuf::from("/worktree");
        let marker = worktree_root.join(".git");
        let common_dir = PathBuf::from("/repo/.git");
        let admin = common_dir.join("worktrees/member");
        let worktree_basis = SourceArchiveNativeMetadataBasis {
            identity_root: worktree_root.clone(),
            marker_kind: "git_worktree_pointer".to_owned(),
            marker_path: Some(marker.clone()),
            git_admin_path: Some(admin.clone()),
            components: vec![
                component("worktree-pointer", marker.clone(), Some(admin.clone())),
                component(
                    "commondir",
                    admin.join("commondir"),
                    Some(common_dir.clone()),
                ),
                component("gitdir", admin.join("gitdir"), Some(marker.clone())),
                component("head", admin.join("HEAD"), None),
            ],
        };
        let worktree_evidence = SourceArchiveOwnerResolutionEvidence {
            scheme: SourceArchiveOwnerResolutionScheme::NativeIdentityV1,
            normalized_cwd: worktree_root.join("nested"),
            candidate_cwd: worktree_root.join("nested"),
            boundary: worktree_root.clone(),
            probe_cwd: worktree_root.join("nested"),
            native_identity_kind: "git-common-dir".to_owned(),
            native_identity: common_dir.display().to_string(),
            native_identity_path: common_dir.clone(),
            native_metadata_sha256: sha256_json(&worktree_basis).unwrap(),
            native_metadata_basis: worktree_basis,
            alias: None,
        };
        let worktree_owner = SourceArchiveNativeOwner {
            identity_kind: "git-common-dir".to_owned(),
            identity_path: common_dir,
        };
        validate_owner_resolution_evidence(
            &worktree_evidence,
            std::slice::from_ref(&worktree_owner),
        )
        .expect("adjacent linked-worktree control");
        for forged_admin in [
            PathBuf::from("/repo/.git/outside/member"),
            PathBuf::from("/repo/.git/worktrees/member/nested"),
        ] {
            let mut forged = worktree_evidence.clone();
            forged.native_metadata_basis.git_admin_path = Some(forged_admin.clone());
            forged.native_metadata_basis.components[0].resolved_target = Some(forged_admin.clone());
            forged.native_metadata_basis.components[1].path = forged_admin.join("commondir");
            forged.native_metadata_basis.components[2].path = forged_admin.join("gitdir");
            forged.native_metadata_basis.components[3].path = forged_admin.join("HEAD");
            forged.native_metadata_sha256 = sha256_json(&forged.native_metadata_basis).unwrap();
            assert!(
                validate_owner_resolution_evidence(&forged, std::slice::from_ref(&worktree_owner))
                    .is_err()
            );
        }
        let mut wrong_backlink = worktree_evidence;
        wrong_backlink.native_metadata_basis.components[2].resolved_target =
            Some(PathBuf::from("/other/.git"));
        wrong_backlink.native_metadata_sha256 =
            sha256_json(&wrong_backlink.native_metadata_basis).unwrap();
        assert!(validate_owner_resolution_evidence(&wrong_backlink, &[worktree_owner]).is_err());
    }

    #[test]
    fn automatic_import_and_readback_bind_exact_source_policy_root() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let actual_root = temporary.path().join("rollouts-actual");
        let approved_root = temporary.path().join("rollouts-approved");
        let sibling_prefix_root = temporary.path().join("rollouts-act");
        let cwd = temporary.path().join("owner");
        for path in [&actual_root, &approved_root, &sibling_prefix_root, &cwd] {
            fs::create_dir(path).expect("fixture directory");
        }
        let source = actual_root.join("thread.jsonl");
        write_snapshot_rollout(&source, &cwd);
        let actual_root = actual_root.canonicalize().expect("canonical actual root");
        let approved_root = approved_root
            .canonicalize()
            .expect("canonical approved root");
        let sibling_prefix_root = sibling_prefix_root
            .canonicalize()
            .expect("canonical sibling-prefix root");
        let source = source.canonicalize().expect("canonical source");
        let snapshot = SourceArchiveSnapshot::capture(&actual_root, &source)
            .expect("capture source under actual root");
        let identity = identity();
        let archive_root = temporary.path().join("policy-root-archives");
        assert!(
            try_automatic_import_options(
                &snapshot,
                identity,
                &archive_root,
                SourceArchiveRolePolicy::UserOnly,
                10,
                &approved_root,
            )
            .is_err(),
            "live issuance must reject a snapshot captured under a different policy root"
        );

        let options = automatic_import_options(
            &snapshot,
            identity,
            &archive_root,
            SourceArchiveRolePolicy::UserOnly,
            10,
        );
        let imported = import_source_archive(&options).expect("valid automatic import");
        let original = imported.manifest.clone();
        for forged_root in [&approved_root, &sibling_prefix_root] {
            let mut forged = original.clone();
            forged
                .automatic_attribution
                .as_mut()
                .expect("automatic attribution")
                .source_policy_basis
                .source_policy_entries[0]
                .rollout_root = forged_root.clone();
            refresh_embedded_policy_digest(&mut forged);
            rewrite_automatic_manifest(&imported.archive_path, &mut forged);
            assert!(
                validate_archive(&imported.archive_path, Some(identity)).is_err(),
                "digest-consistent source path outside embedded policy root {} must fail",
                forged_root.display()
            );
        }
    }

    #[test]
    fn archive_readback_rejects_unknown_or_contradictory_schema_versions() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let source = temporary.path().join("rollout.jsonl");
        let root = temporary.path().join("archives");
        write_rollout(&source);
        let identity = identity();
        let imported = import_source_archive(
            &SourceArchiveImportOptions::new(identity, &root, vec![source])
                .expect("import options"),
        )
        .expect("legacy import");
        let manifest_path = imported.archive_path.join(MANIFEST_FILE);
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest_path).expect("read legacy manifest"))
                .expect("decode legacy manifest");
        value["format_version"] = serde_json::json!(99);
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&value).expect("encode unknown manifest"),
        )
        .expect("write unknown manifest");
        assert!(
            validate_archive(&imported.archive_path, Some(identity)).is_err(),
            "unknown archive versions must fail closed before interpretation"
        );

        value["format_version"] = serde_json::json!(LEGACY_ARCHIVE_FORMAT_VERSION);
        value["unexpected_future_field"] = serde_json::json!(true);
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&value).expect("encode unknown field"),
        )
        .expect("write unknown field");
        assert!(
            validate_archive(&imported.archive_path, Some(identity)).is_err(),
            "recognized versions with unknown fields must fail closed"
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

    #[cfg(unix)]
    #[test]
    fn archive_readback_rejects_member_and_parent_swaps_without_blocking() {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::symlink;

        let temporary = tempfile::tempdir().expect("temporary directory");
        let source = temporary.path().join("rollout.jsonl");
        let root = temporary.path().join("archives");
        write_rollout(&source);
        let imported = import_source_archive(
            &SourceArchiveImportOptions::new(
                identity(),
                &root,
                vec![source.canonicalize().expect("canonical source")],
            )
            .expect("import options"),
        )
        .expect("source import");

        let manifest = imported.archive_path.join(MANIFEST_FILE);
        let held_manifest = imported.archive_path.join("manifest-held.json");
        let started = std::time::Instant::now();
        let result = validate_archive_with_hook(&imported.archive_path, None, || {
            fs::rename(&manifest, &held_manifest).expect("hold manifest");
            let fifo = CString::new(manifest.as_os_str().as_bytes()).expect("FIFO path");
            assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
        });
        assert!(result.is_err());
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        fs::remove_file(&manifest).expect("remove FIFO");
        fs::rename(&held_manifest, &manifest).expect("restore manifest");

        let documents = imported.archive_path.join(DOCUMENTS_FILE);
        let held_documents = imported.archive_path.join("documents-held.json");
        let outside = temporary.path().join("outside.json");
        fs::write(&outside, b"[]").expect("outside bytes");
        let result = validate_archive_with_hook(&imported.archive_path, None, || {
            fs::rename(&documents, &held_documents).expect("hold documents");
            symlink(&outside, &documents).expect("documents symlink");
        });
        assert!(result.is_err());
        fs::remove_file(&documents).expect("remove documents link");
        fs::rename(&held_documents, &documents).expect("restore documents");

        let held_archive = root.join("held-archive");
        let replacement = temporary.path().join("replacement-archive");
        fs::create_dir(&replacement).expect("replacement archive");
        fs::set_permissions(&replacement, fs::Permissions::from_mode(0o700))
            .expect("protect replacement archive");
        let result = validate_archive_with_hook(&imported.archive_path, None, || {
            fs::rename(&imported.archive_path, &held_archive).expect("hold archive");
            symlink(&replacement, &imported.archive_path).expect("replace archive parent");
        });
        assert!(result.is_err());
        fs::remove_file(&imported.archive_path).expect("remove archive link");
        fs::rename(&held_archive, &imported.archive_path).expect("restore archive");
        validate_archive(&imported.archive_path, None).expect("restored archive remains readable");
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
