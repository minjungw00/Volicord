use std::{
    collections::BTreeSet,
    ffi::OsString,
    fmt, fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::SystemTime,
};

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::json;
use sha2::{Digest, Sha256};
use volicord_command_model::{AuthorityBundleArgs, ExportArgs, ExportCommand};
use volicord_store::{
    export::{
        read_authority_bundle_snapshot, AuthorityBundleArtifact, AuthorityBundleRecord,
        AuthorityBundleSnapshot, AuthorityBundleTableCount,
    },
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    StoreError,
};

const AUTHORITY_BUNDLE_CREATED_BY: &str = "volicord export authority-bundle";
const AUTHORITY_BUNDLE_MANIFEST: &str = "manifest.json";
const AUTHORITY_BUNDLE_RECORDS: &str = "records.jsonl";
const AUTHORITY_BUNDLE_CHECKSUMS: &str = "checksums.sha256";
const AUTHORITY_BUNDLE_README: &str = "README.txt";
const AUTHORITY_BUNDLE_ARTIFACTS_DIR: &str = "artifacts";
const AUTHORITY_BUNDLE_NON_GUARANTEES: &[&str] = &[
    "not_tamper_proof_runtime_home_history",
    "not_correctness_proof",
    "not_test_sufficiency_proof",
    "not_review_completion_proof",
    "not_deployment_proof",
    "not_final_acceptance_or_residual_risk_acceptance",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportCommandError {
    Usage(String),
    Runtime(String),
}

impl ExportCommandError {
    fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }
}

impl fmt::Display for ExportCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ExportCommandError {}

impl From<StoreError> for ExportCommandError {
    fn from(error: StoreError) -> Self {
        Self::runtime(error.to_string())
    }
}

impl From<RuntimeHomeResolutionError> for ExportCommandError {
    fn from(error: RuntimeHomeResolutionError) -> Self {
        Self::runtime(error.to_string())
    }
}

impl From<std::io::Error> for ExportCommandError {
    fn from(error: std::io::Error) -> Self {
        Self::runtime(error.to_string())
    }
}

impl From<serde_json::Error> for ExportCommandError {
    fn from(error: serde_json::Error) -> Self {
        Self::runtime(error.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

pub fn run_export_command<F>(
    args: ExportArgs,
    env_var: F,
    current_dir: &Path,
) -> Result<String, ExportCommandError>
where
    F: Fn(&str) -> Option<OsString>,
{
    match args.command {
        ExportCommand::AuthorityBundle(options) => {
            run_authority_bundle_export(options, env_var, current_dir)
        }
    }
}

fn run_authority_bundle_export<F>(
    options: AuthorityBundleArgs,
    env_var: F,
    current_dir: &Path,
) -> Result<String, ExportCommandError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let runtime_home = resolve_runtime_home(&env_var, current_dir)?;
    let repo_root = resolve_repository_root(current_dir, options.repo.as_deref())?;
    let snapshot = read_authority_bundle_snapshot(&runtime_home, &repo_root)?;
    let output_path = absolute_path(current_dir, options.output);
    let bundle = write_authority_bundle(&runtime_home, &repo_root, &output_path, &snapshot)?;

    render_authority_bundle_output(
        if options.json {
            OutputFormat::Json
        } else {
            OutputFormat::Text
        },
        &bundle,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AuthorityBundleWriteResult {
    output_path: PathBuf,
    manifest_path: PathBuf,
    records_path: PathBuf,
    checksums_path: PathBuf,
    readme_path: PathBuf,
    record_count: usize,
    artifact_count: usize,
    copied_artifact_count: usize,
    checksum_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ArtifactBundleEntry {
    artifact_id: String,
    body_path: Option<String>,
    bundle_path: Option<String>,
    stored_sha256: Option<String>,
    exported_sha256: Option<String>,
    size_bytes: Option<u64>,
    content_type: Option<String>,
    status: String,
    integrity_status: String,
    copy_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChecksumEntry {
    path: String,
    sha256: String,
    size_bytes: u64,
}

fn write_authority_bundle(
    runtime_home: &Path,
    repo_root: &Path,
    output_path: &Path,
    snapshot: &AuthorityBundleSnapshot,
) -> Result<AuthorityBundleWriteResult, ExportCommandError> {
    prepare_bundle_output_dir(output_path)?;
    let records_path = output_path.join(AUTHORITY_BUNDLE_RECORDS);
    let readme_path = output_path.join(AUTHORITY_BUNDLE_README);
    let manifest_path = output_path.join(AUTHORITY_BUNDLE_MANIFEST);
    let checksums_path = output_path.join(AUTHORITY_BUNDLE_CHECKSUMS);
    let artifacts_dir = output_path.join(AUTHORITY_BUNDLE_ARTIFACTS_DIR);
    fs::create_dir_all(&artifacts_dir)?;

    write_records_jsonl(&records_path, &snapshot.records)?;
    write_text_file(&readme_path, authority_bundle_readme())?;
    let artifact_entries = copy_authority_bundle_artifacts(output_path, &snapshot.artifacts)?;
    let manifest = authority_bundle_manifest(
        runtime_home,
        repo_root,
        snapshot,
        &artifact_entries,
        current_timestamp(),
    );
    write_json_file(&manifest_path, &manifest)?;

    let checksum_entries =
        write_authority_bundle_checksums(output_path, &artifact_entries, &checksums_path)?;

    Ok(AuthorityBundleWriteResult {
        output_path: output_path.to_path_buf(),
        manifest_path,
        records_path,
        checksums_path,
        readme_path,
        record_count: snapshot.records.len(),
        artifact_count: snapshot.artifacts.len(),
        copied_artifact_count: artifact_entries
            .iter()
            .filter(|entry| entry.copy_status == "copied")
            .count(),
        checksum_count: checksum_entries.len(),
    })
}

fn prepare_bundle_output_dir(path: &Path) -> Result<(), ExportCommandError> {
    match path.try_exists() {
        Ok(true) => {
            let metadata = fs::metadata(path)?;
            if !metadata.is_dir() {
                return Err(ExportCommandError::runtime(format!(
                    "authority bundle output must be a directory: {}",
                    path.display()
                )));
            }
            if fs::read_dir(path)?.next().transpose()?.is_some() {
                return Err(ExportCommandError::runtime(format!(
                    "authority bundle output directory must be empty: {}",
                    path.display()
                )));
            }
        }
        Ok(false) => fs::create_dir_all(path)?,
        Err(error) => return Err(ExportCommandError::runtime(error.to_string())),
    }
    Ok(())
}

fn write_records_jsonl(
    path: &Path,
    records: &[AuthorityBundleRecord],
) -> Result<(), ExportCommandError> {
    let mut file = fs::File::create(path)?;
    for record in records {
        let value = json!({
            "database": record.database,
            "table": record.table,
            "row": record.row,
        });
        serde_json::to_writer(&mut file, &value)?;
        file.write_all(b"\n")?;
    }
    Ok(())
}

fn write_text_file(path: &Path, text: &str) -> Result<(), ExportCommandError> {
    let mut file = fs::File::create(path)?;
    file.write_all(text.as_bytes())?;
    Ok(())
}

fn write_json_file(path: &Path, value: &serde_json::Value) -> Result<(), ExportCommandError> {
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    write_text_file(path, &text)
}

fn copy_authority_bundle_artifacts(
    output_path: &Path,
    artifacts: &[AuthorityBundleArtifact],
) -> Result<Vec<ArtifactBundleEntry>, ExportCommandError> {
    let mut entries = Vec::new();
    for artifact in artifacts {
        let Some(source_path) = artifact.source_path.as_ref() else {
            entries.push(artifact_entry(
                artifact,
                None,
                None,
                "metadata_only_no_body",
            ));
            continue;
        };
        let Some(bundle_path) = artifact
            .body_path
            .as_deref()
            .and_then(artifact_bundle_relative_path)
        else {
            entries.push(artifact_entry(
                artifact,
                None,
                None,
                "metadata_only_invalid_body_path",
            ));
            continue;
        };
        let destination = output_path.join(&bundle_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        match fs::copy(source_path, &destination) {
            Ok(_) => {
                let checksum = sha256_file(&destination)?;
                entries.push(artifact_entry(
                    artifact,
                    Some(relative_path_text(&bundle_path)),
                    Some(checksum.sha256),
                    "copied",
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                entries.push(artifact_entry(
                    artifact,
                    None,
                    None,
                    "metadata_only_missing_body",
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                entries.push(artifact_entry(
                    artifact,
                    None,
                    None,
                    "metadata_only_unavailable_body",
                ));
            }
            Err(error) => return Err(ExportCommandError::runtime(error.to_string())),
        }
    }
    Ok(entries)
}

fn artifact_entry(
    artifact: &AuthorityBundleArtifact,
    bundle_path: Option<String>,
    exported_sha256: Option<String>,
    copy_status: &str,
) -> ArtifactBundleEntry {
    ArtifactBundleEntry {
        artifact_id: artifact.artifact_id.clone(),
        body_path: artifact.body_path.clone(),
        bundle_path,
        stored_sha256: artifact.stored_sha256.clone(),
        exported_sha256,
        size_bytes: artifact.size_bytes,
        content_type: artifact.content_type.clone(),
        status: artifact.status.clone(),
        integrity_status: artifact.integrity_status.clone(),
        copy_status: copy_status.to_owned(),
    }
}

fn artifact_bundle_relative_path(body_path: &str) -> Option<PathBuf> {
    let body_path = safe_relative_path(body_path)?;
    Some(PathBuf::from(AUTHORITY_BUNDLE_ARTIFACTS_DIR).join(body_path))
}

fn safe_relative_path(path: &str) -> Option<PathBuf> {
    if path.trim().is_empty() {
        return None;
    }
    let mut output = PathBuf::new();
    for component in Path::new(path).components() {
        match component {
            std::path::Component::Normal(part) => output.push(part),
            _ => return None,
        }
    }
    if output.as_os_str().is_empty() {
        None
    } else {
        Some(output)
    }
}

fn authority_bundle_manifest(
    runtime_home: &Path,
    repo_root: &Path,
    snapshot: &AuthorityBundleSnapshot,
    artifact_entries: &[ArtifactBundleEntry],
    created_at: String,
) -> serde_json::Value {
    let files = authority_bundle_file_manifest(artifact_entries);
    json!({
        "bundle_kind": "authority_bundle",
        "format": "volicord_authority_bundle",
        "created_by": AUTHORITY_BUNDLE_CREATED_BY,
        "created_at": created_at,
        "hash_algorithm": "sha256",
        "runtime_home": {
            "path": path_text(runtime_home),
        },
        "project": {
            "project_id": snapshot.project.project_id,
            "project_name": snapshot.project.project_name,
            "project_alias": snapshot.project.project_alias,
            "repo_root": path_text(repo_root),
            "registered_repo_root": path_text(&snapshot.project.repo_root),
            "project_home": path_text(&snapshot.project.project_home),
            "state_db_path": path_text(&snapshot.project.state_db_path),
            "status": snapshot.project.status,
        },
        "records": {
            "path": AUTHORITY_BUNDLE_RECORDS,
            "record_count": snapshot.records.len(),
            "tables": table_counts_json(&snapshot.table_counts),
        },
        "artifacts": {
            "directory": format!("{AUTHORITY_BUNDLE_ARTIFACTS_DIR}/"),
            "artifact_count": artifact_entries.len(),
            "copied_artifact_count": artifact_entries
                .iter()
                .filter(|entry| entry.copy_status == "copied")
                .count(),
            "items": artifact_entries_json(artifact_entries),
        },
        "files": files,
        "checksums_path": AUTHORITY_BUNDLE_CHECKSUMS,
        "non_guarantees": AUTHORITY_BUNDLE_NON_GUARANTEES,
    })
}

fn authority_bundle_file_manifest(
    artifact_entries: &[ArtifactBundleEntry],
) -> Vec<serde_json::Value> {
    let mut files = vec![
        json!({"path": AUTHORITY_BUNDLE_MANIFEST, "role": "manifest"}),
        json!({"path": AUTHORITY_BUNDLE_RECORDS, "role": "exported_records"}),
        json!({"path": AUTHORITY_BUNDLE_CHECKSUMS, "role": "checksum_manifest"}),
        json!({"path": AUTHORITY_BUNDLE_README, "role": "bundle_readme"}),
        json!({"path": format!("{AUTHORITY_BUNDLE_ARTIFACTS_DIR}/"), "role": "artifact_directory"}),
    ];
    files.extend(
        artifact_entries
            .iter()
            .filter_map(|entry| entry.bundle_path.as_ref())
            .map(|path| json!({"path": path, "role": "artifact_body"})),
    );
    files
}

fn table_counts_json(table_counts: &[AuthorityBundleTableCount]) -> Vec<serde_json::Value> {
    table_counts
        .iter()
        .map(|entry| {
            json!({
                "database": entry.database,
                "table": entry.table,
                "row_count": entry.row_count,
            })
        })
        .collect()
}

fn artifact_entries_json(entries: &[ArtifactBundleEntry]) -> Vec<serde_json::Value> {
    entries
        .iter()
        .map(|entry| {
            json!({
                "artifact_id": entry.artifact_id,
                "body_path": entry.body_path,
                "bundle_path": entry.bundle_path,
                "stored_sha256": entry.stored_sha256,
                "exported_sha256": entry.exported_sha256,
                "size_bytes": entry.size_bytes,
                "content_type": entry.content_type,
                "status": entry.status,
                "integrity_status": entry.integrity_status,
                "copy_status": entry.copy_status,
            })
        })
        .collect()
}

fn write_authority_bundle_checksums(
    output_path: &Path,
    artifact_entries: &[ArtifactBundleEntry],
    checksums_path: &Path,
) -> Result<Vec<ChecksumEntry>, ExportCommandError> {
    let mut paths = BTreeSet::from([
        AUTHORITY_BUNDLE_MANIFEST.to_owned(),
        AUTHORITY_BUNDLE_RECORDS.to_owned(),
        AUTHORITY_BUNDLE_README.to_owned(),
    ]);
    for artifact_path in artifact_entries
        .iter()
        .filter_map(|entry| entry.bundle_path.as_ref())
    {
        paths.insert(artifact_path.clone());
    }

    let mut entries = Vec::new();
    for path in paths {
        let checksum = sha256_file(&output_path.join(&path))?;
        entries.push(ChecksumEntry {
            path,
            sha256: checksum.sha256,
            size_bytes: checksum.size_bytes,
        });
    }

    let mut text = String::new();
    for entry in &entries {
        text.push_str(&format!("{}  {}\n", entry.sha256, entry.path));
    }
    write_text_file(checksums_path, &text)?;
    Ok(entries)
}

fn authority_bundle_readme() -> &'static str {
    "Volicord authority bundle\n\n\
This bundle is an integrity-labeled copy of local Volicord records for one registered Product Repository.\n\
Use checksums.sha256 to verify that exported files have not changed since this bundle was written.\n\n\
Contents:\n\
- manifest.json describes the bundle, selected project, record counts, artifact copy status, and non-guarantees.\n\
- records.jsonl contains exported storage rows as JSON Lines.\n\
- artifacts/ contains copied persistent artifact bodies when the current local artifact store made those bytes available.\n\
- checksums.sha256 contains SHA-256 checksums for manifest.json, records.jsonl, README.txt, and copied artifact body files.\n\n\
Non-guarantees:\n\
- This is not proof that the Runtime Home was never modified before export.\n\
- This is not tamper-proof storage, cryptographic signing, or an external audit log.\n\
- This is not a correctness, test sufficiency, review completion, or deployment proof.\n\
- This is not final acceptance or residual-risk acceptance.\n"
}

fn render_authority_bundle_output(
    format: OutputFormat,
    bundle: &AuthorityBundleWriteResult,
) -> Result<String, ExportCommandError> {
    match format {
        OutputFormat::Text => Ok(format!(
            "Authority bundle exported\noutput: {}\nmanifest: {}\nrecords: {}\nchecksums: {}\nrecord_count: {}\nartifacts_copied: {}\n",
            path_text(&bundle.output_path),
            path_text(&bundle.manifest_path),
            path_text(&bundle.records_path),
            path_text(&bundle.checksums_path),
            bundle.record_count,
            bundle.copied_artifact_count,
        )),
        OutputFormat::Json => {
            let value = json!({
                "action": "exported",
                "status": "complete",
                "bundle_kind": "authority_bundle",
                "output_path": path_text(&bundle.output_path),
                "manifest_path": path_text(&bundle.manifest_path),
                "records_path": path_text(&bundle.records_path),
                "checksums_path": path_text(&bundle.checksums_path),
                "readme_path": path_text(&bundle.readme_path),
                "record_count": bundle.record_count,
                "artifact_count": bundle.artifact_count,
                "copied_artifact_count": bundle.copied_artifact_count,
                "checksum_entry_count": bundle.checksum_count,
            });
            serde_json::to_string_pretty(&value)
                .map(|text| format!("{text}\n"))
                .map_err(|error| ExportCommandError::runtime(error.to_string()))
        }
    }
}

fn current_timestamp() -> String {
    DateTime::<Utc>::from(SystemTime::now()).to_rfc3339_opts(SecondsFormat::Secs, true)
}

struct FileChecksum {
    sha256: String,
    size_bytes: u64,
}

fn sha256_file(path: &Path) -> Result<FileChecksum, ExportCommandError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut size_bytes = 0_u64;
    let mut buffer = [0_u8; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        size_bytes += u64::try_from(read).expect("buffer read length should fit in u64");
        hasher.update(&buffer[..read]);
    }
    Ok(FileChecksum {
        sha256: lowercase_hex_bytes(&hasher.finalize()),
        size_bytes,
    })
}

fn lowercase_hex_bytes(bytes: &[u8]) -> String {
    let mut text = String::new();
    for byte in bytes {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

fn resolve_repository_root(
    current_dir: &Path,
    selected_path: Option<&Path>,
) -> Result<PathBuf, ExportCommandError> {
    let selected = selected_path.unwrap_or(current_dir);
    let absolute = absolute_path(current_dir, selected.to_path_buf());
    let canonical = fs::canonicalize(&absolute).map_err(|error| {
        ExportCommandError::runtime(format!(
            "repository path is not accessible: {} ({error})",
            absolute.display()
        ))
    })?;
    let metadata = fs::metadata(&canonical).map_err(|error| {
        ExportCommandError::runtime(format!(
            "repository path is not accessible: {} ({error})",
            canonical.display()
        ))
    })?;
    let mut cursor = if metadata.is_file() {
        canonical
            .parent()
            .ok_or_else(|| {
                ExportCommandError::runtime(format!(
                    "repository path has no parent directory: {}",
                    canonical.display()
                ))
            })?
            .to_path_buf()
    } else {
        canonical
    };

    loop {
        let git_path = cursor.join(".git");
        match git_path.try_exists() {
            Ok(true) => return Ok(cursor),
            Ok(false) => {}
            Err(error) => {
                return Err(ExportCommandError::runtime(format!(
                    "failed to inspect Git repository marker {}: {error}",
                    git_path.display()
                )));
            }
        }
        if !cursor.pop() {
            break;
        }
    }

    Err(ExportCommandError::runtime(format!(
        "no Git repository root found from {}; run `volicord project use PATH` from inside a Git repository or pass --repo PATH",
        absolute.display()
    )))
}

fn absolute_path(current_dir: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        current_dir.join(path)
    }
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}

fn relative_path_text(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}
