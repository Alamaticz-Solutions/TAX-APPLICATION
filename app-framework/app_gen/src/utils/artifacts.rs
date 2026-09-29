use std::{
    cell::RefCell,
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::Command,
    sync::{Mutex, OnceLock},
};

use anyhow::Context;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::app_workspace::AppWorkspace;

use super::console;

#[derive(Debug, Clone)]
pub enum ArtifactContent {
    Text(String),
    CopyFrom(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverwriteMode {
    Always,
    IfMissing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactOwnership {
    Generated,
    HumanOwned,
}

impl ArtifactOwnership {
    fn as_str(self) -> &'static str {
        match self {
            Self::Generated => "generated",
            Self::HumanOwned => "human_owned",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Artifact {
    pub path: PathBuf,
    pub content: ArtifactContent,
    pub overwrite: OverwriteMode,
    pub ownership: ArtifactOwnership,
}

#[derive(Debug, Clone, Serialize)]
struct ArtifactRecord {
    path: String,
    ownership: ArtifactOwnership,
    overwrite: &'static str,
    action: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ArtifactProvenance {
    version: u32,
    generated_at: String,
    generator: &'static str,
    app_root: String,
    framework_root: String,
    generator_root: String,
    config_root: String,
    templates_root: String,
    artifact_count: usize,
    config_sha256: String,
    templates_sha256: String,
    generator_source_sha256: String,
    artifacts_sha256: String,
}

static ARTIFACT_RECORDS: OnceLock<Mutex<Vec<ArtifactRecord>>> = OnceLock::new();
const ARTIFACT_MAX_BYTES: u64 = 256 * 1024 * 1024;

thread_local! {
    static ARTIFACT_RUN: RefCell<Option<ArtifactRun>> = const { RefCell::new(None) };
}

#[derive(Debug, Clone)]
struct ArtifactRun {
    aliases: Vec<PathAlias>,
}

#[derive(Debug, Clone)]
struct PathAlias {
    lexical_root: PathBuf,
    canonical_root: PathBuf,
}

#[derive(Debug, Clone)]
struct ResolvedOutputPath {
    path: PathBuf,
    canonical_root: PathBuf,
}

fn records() -> &'static Mutex<Vec<ArtifactRecord>> {
    ARTIFACT_RECORDS.get_or_init(|| Mutex::new(vec![]))
}

pub fn begin_run(workspace: &AppWorkspace) -> anyhow::Result<()> {
    let app_root = canonical_non_symlink_root(&workspace.app_root, "product app root")?;
    let config_root =
        canonical_existing_output_root(&workspace.config_root, "product config root")?;
    let report_root = ensure_canonical_output_root(&workspace.report_root, "product report root")?;
    preflight_output_trees(&app_root, &config_root, &report_root, workspace)?;
    records()
        .lock()
        .map_err(|_| anyhow::anyhow!("artifact manifest lock poisoned"))?
        .clear();
    let mut aliases = Vec::new();
    for (lexical_root, canonical_root) in [
        (&workspace.report_root, &report_root),
        (&workspace.config_root, &config_root),
        (&workspace.app_root, &app_root),
    ] {
        push_path_alias(&mut aliases, lexical_root, canonical_root);
        push_path_alias(&mut aliases, canonical_root, canonical_root);
    }
    ARTIFACT_RUN.with(|state| *state.borrow_mut() = Some(ArtifactRun { aliases }));
    Ok(())
}

fn push_path_alias(aliases: &mut Vec<PathAlias>, lexical_root: &Path, canonical_root: &Path) {
    if aliases
        .iter()
        .any(|alias| alias.lexical_root == lexical_root && alias.canonical_root == canonical_root)
    {
        return;
    }
    aliases.push(PathAlias {
        lexical_root: lexical_root.to_path_buf(),
        canonical_root: canonical_root.to_path_buf(),
    });
}

impl Artifact {
    pub fn generated_text(path: PathBuf, content: String, overwrite: OverwriteMode) -> Self {
        Self {
            path,
            content: ArtifactContent::Text(content),
            overwrite,
            ownership: ArtifactOwnership::Generated,
        }
    }

    pub fn human_owned_text(path: PathBuf, content: String) -> Self {
        Self {
            path,
            content: ArtifactContent::Text(content),
            overwrite: OverwriteMode::IfMissing,
            ownership: ArtifactOwnership::HumanOwned,
        }
    }

    pub fn generated_copy_from(path: PathBuf, source: PathBuf, overwrite: OverwriteMode) -> Self {
        Self {
            path,
            content: ArtifactContent::CopyFrom(source),
            overwrite,
            ownership: ArtifactOwnership::Generated,
        }
    }
}

pub fn emit(artifact: &Artifact) -> anyhow::Result<()> {
    let output_path = prepare_output_path(&artifact.path)?;
    if should_skip(&output_path, artifact.overwrite) {
        log_skipped(&artifact.path, artifact.ownership);
        record(artifact, "skipped")?;
        return Ok(());
    }

    match &artifact.content {
        ArtifactContent::Text(content) => {
            let normalized = normalize_text(content);
            write_regular_file(&output_path, normalized.as_bytes())?;
            if is_rust_file(&output_path) {
                format_rust_file(&output_path)?;
            }
            console::write(&artifact.path);
        }
        ArtifactContent::CopyFrom(source) => {
            let bytes = read_regular_file(source).with_context(|| {
                format!(
                    "could not safely read generated artifact source {}",
                    source.display()
                )
            })?;
            write_regular_file(&output_path, &bytes)?;
            console::copy(source, &artifact.path);
        }
    }

    record(artifact, "written")?;
    Ok(())
}

fn normalize_text(content: &str) -> String {
    let mut lines = content.lines().map(str::trim_end).collect::<Vec<_>>();
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    let mut normalized = lines.join("\n");
    normalized.push('\n');
    normalized
}

fn is_rust_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("rs"))
        .unwrap_or(false)
}

fn format_rust_file(path: &Path) -> anyhow::Result<()> {
    let status = Command::new("rustfmt")
        .arg("--edition")
        .arg("2021")
        .arg(path)
        .status()
        .with_context(|| {
            format!(
                "could not run rustfmt for generated Rust file {}; install rustfmt or run through scripts/appfw doctor",
                path.display()
            )
        })?;

    if !status.success() {
        anyhow::bail!(
            "rustfmt failed for generated Rust file {} with status {}",
            path.display(),
            status
        );
    }

    Ok(())
}

pub fn emit_text<F>(path: PathBuf, overwrite: OverwriteMode, build_content: F) -> anyhow::Result<()>
where
    F: FnOnce() -> anyhow::Result<String>,
{
    let output_path = prepare_output_path(&path)?;
    if should_skip(&output_path, overwrite) {
        console::skip(&path, "already exists");
        record_raw(
            path,
            ArtifactOwnership::Generated,
            overwrite,
            "skipped",
            None,
        )?;
        return Ok(());
    }

    let content = build_content()?;
    emit(&Artifact::generated_text(path, content, overwrite))
}

pub fn emit_human_text<F>(path: PathBuf, build_content: F) -> anyhow::Result<()>
where
    F: FnOnce() -> anyhow::Result<String>,
{
    let output_path = prepare_output_path(&path)?;
    if should_skip(&output_path, OverwriteMode::IfMissing) {
        log_skipped(&path, ArtifactOwnership::HumanOwned);
        record_raw(
            path,
            ArtifactOwnership::HumanOwned,
            OverwriteMode::IfMissing,
            "skipped",
            None,
        )?;
        return Ok(());
    }

    let content = build_content()?;
    emit(&Artifact::human_owned_text(path, content))
}

#[allow(dead_code)]
pub fn emit_all(artifacts: Vec<Artifact>) -> anyhow::Result<()> {
    for artifact in artifacts {
        emit(&artifact)?;
    }

    Ok(())
}

fn should_skip(path: &Path, overwrite: OverwriteMode) -> bool {
    overwrite == OverwriteMode::IfMissing && path.exists()
}

fn log_skipped(path: &Path, ownership: ArtifactOwnership) {
    console::skip(path, format!("{} file already exists", ownership.as_str()));
}

fn record(artifact: &Artifact, action: &'static str) -> anyhow::Result<()> {
    let source = match &artifact.content {
        ArtifactContent::Text(_) => None,
        ArtifactContent::CopyFrom(source) => Some(source.as_path()),
    };
    record_raw(
        artifact.path.clone(),
        artifact.ownership,
        artifact.overwrite,
        action,
        source,
    )
}

fn record_raw(
    path: PathBuf,
    ownership: ArtifactOwnership,
    overwrite: OverwriteMode,
    action: &'static str,
    source: Option<&Path>,
) -> anyhow::Result<()> {
    let overwrite = match overwrite {
        OverwriteMode::Always => "always",
        OverwriteMode::IfMissing => "if_missing",
    };
    let content_sha256 = Some(bounded_file_sha256(&path).with_context(|| {
        format!(
            "could not hash recorded artifact bytes without weakening evidence: {}",
            path.display()
        )
    })?);
    let source_sha256 = match source {
        Some(source) => Some(bounded_file_sha256_uncontained(source).with_context(|| {
            format!(
                "could not hash recorded artifact source without weakening evidence: {}",
                source.display()
            )
        })?),
        None => None,
    };
    records()
        .lock()
        .map_err(|_| anyhow::anyhow!("artifact manifest lock poisoned"))?
        .push(ArtifactRecord {
            path: path.display().to_string(),
            ownership,
            overwrite,
            action,
            content_sha256,
            source_sha256,
        });
    Ok(())
}

pub fn write_manifest(workspace: &AppWorkspace) -> anyhow::Result<()> {
    let target_dir = &workspace.report_root;
    let target_file = target_dir.join("artifacts.json");
    prepare_output_path(&target_file)?;
    let mut records = records()
        .lock()
        .map_err(|_| anyhow::anyhow!("artifact manifest lock poisoned"))?;
    finalize_records(&mut records)?;
    let records = records.clone();
    safe_write(&target_file, serde_json::to_string_pretty(&records)? + "\n")?;
    console::write(&target_file);
    write_provenance(workspace, &target_dir, records.len(), &target_file)?;
    Ok(())
}

fn finalize_records(records: &mut Vec<ArtifactRecord>) -> anyhow::Result<()> {
    for record in records.iter_mut() {
        let path = Path::new(&record.path);
        let output_path =
            inspect_output_path(path, PathRequirement::ExistingFile).with_context(|| {
                format!(
                    "recorded artifact is invalid in final generator output: {}",
                    path.display()
                )
            })?;
        record.path = output_path.display().to_string();
        record.content_sha256 = Some(bounded_file_sha256(&output_path)?);
    }

    records.sort_by(|left, right| {
        (
            left.path.as_str(),
            left.ownership.as_str(),
            left.overwrite,
            left.action,
            left.source_sha256.as_deref(),
        )
            .cmp(&(
                right.path.as_str(),
                right.ownership.as_str(),
                right.overwrite,
                right.action,
                right.source_sha256.as_deref(),
            ))
    });
    let mut reconciled: Vec<ArtifactRecord> = Vec::with_capacity(records.len());
    for record in records.drain(..) {
        let Some(existing) = reconciled
            .last_mut()
            .filter(|item| item.path == record.path)
        else {
            reconciled.push(record);
            continue;
        };
        if existing.ownership != record.ownership
            || existing.overwrite != record.overwrite
            || existing.content_sha256 != record.content_sha256
            || existing.source_sha256 != record.source_sha256
        {
            anyhow::bail!(
                "duplicate artifact emissions have conflicting authoritative semantics: {}",
                record.path
            );
        }
        existing.action = merged_artifact_action(existing.action, record.action)?;
    }
    *records = reconciled;
    Ok(())
}

fn merged_artifact_action(left: &'static str, right: &'static str) -> anyhow::Result<&'static str> {
    if !matches!(left, "written" | "skipped") || !matches!(right, "written" | "skipped") {
        anyhow::bail!("artifact emission recorded an unsupported action");
    }
    match (left, right) {
        ("written", _) | (_, "written") => Ok("written"),
        ("skipped", "skipped") => Ok("skipped"),
        _ => unreachable!("artifact actions were validated above"),
    }
}

fn write_provenance(
    workspace: &AppWorkspace,
    target_dir: &Path,
    artifact_count: usize,
    manifest_file: &Path,
) -> anyhow::Result<()> {
    let provenance = ArtifactProvenance {
        version: 1,
        generated_at: chrono::offset::Utc::now().to_rfc3339(),
        generator: "app_gen",
        app_root: workspace.app_root.display().to_string(),
        framework_root: workspace.framework_root.display().to_string(),
        generator_root: workspace.generator_root.display().to_string(),
        config_root: workspace.config_root.display().to_string(),
        templates_root: workspace.templates_root.display().to_string(),
        artifact_count,
        config_sha256: tree_sha256(&workspace.config_root)?,
        templates_sha256: tree_sha256(&workspace.templates_root)?,
        generator_source_sha256: tree_sha256(&workspace.generator_root.join("src"))?,
        artifacts_sha256: bounded_file_sha256(manifest_file)?,
    };
    let target_file = target_dir.join("artifact_provenance.json");
    safe_write(
        &target_file,
        serde_json::to_string_pretty(&provenance)? + "\n",
    )?;
    console::write(&target_file);
    Ok(())
}

pub fn safe_write(path: &Path, content: impl AsRef<[u8]>) -> anyhow::Result<()> {
    let output_path = prepare_output_path(path)?;
    write_regular_file(&output_path, content.as_ref())
        .with_context(|| format!("could not safely write {}", path.display()))?;
    Ok(())
}

fn write_regular_file(path: &Path, content: &[u8]) -> anyhow::Result<()> {
    let initial = match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
                anyhow::bail!(
                    "output leaf is not a regular non-symlink file: {}",
                    path.display()
                );
            }
            Some(metadata)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(error)
                .with_context(|| format!("could not inspect output {}", path.display()))
        }
    };
    if let Some(initial) = initial.as_ref() {
        require_unaliased_output_leaf(initial, true, path)?;
    }

    let mut options = fs::OpenOptions::new();
    options.write(true);
    if initial.is_none() {
        options.create_new(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options.open(path).with_context(|| {
        format!(
            "could not open output without following its leaf: {}",
            path.display()
        )
    })?;
    let opened = file
        .metadata()
        .with_context(|| format!("could not inspect open output {}", path.display()))?;
    require_unaliased_output_leaf(&opened, initial.is_some(), path)?;
    if let Some(initial) = initial.as_ref() {
        if !same_file_object(initial, &opened) {
            anyhow::bail!("output changed identity while opening: {}", path.display());
        }
    }
    let linked = fs::symlink_metadata(path)
        .with_context(|| format!("could not re-inspect output {}", path.display()))?;
    require_unaliased_output_leaf(&linked, initial.is_some(), path)?;
    if linked.file_type().is_symlink()
        || !linked.file_type().is_file()
        || !same_file_object(&opened, &linked)
    {
        anyhow::bail!(
            "output changed type or identity while opening: {}",
            path.display()
        );
    }

    file.set_len(0).with_context(|| {
        format!(
            "could not truncate output after identity verification: {}",
            path.display()
        )
    })?;
    file.write_all(content)
        .with_context(|| format!("could not write output bytes {}", path.display()))?;
    file.flush()
        .with_context(|| format!("could not flush output bytes {}", path.display()))?;
    let final_opened = file
        .metadata()
        .with_context(|| format!("could not re-inspect open output {}", path.display()))?;
    let final_linked = fs::symlink_metadata(path)
        .with_context(|| format!("could not re-inspect output link {}", path.display()))?;
    require_unaliased_output_leaf(&final_opened, initial.is_some(), path)?;
    require_unaliased_output_leaf(&final_linked, initial.is_some(), path)?;
    if final_linked.file_type().is_symlink()
        || !same_file_object(&opened, &final_opened)
        || !same_file_object(&opened, &final_linked)
        || final_opened.len() != content.len() as u64
    {
        anyhow::bail!("output changed while writing: {}", path.display());
    }
    Ok(())
}

#[cfg(unix)]
fn require_unaliased_output_leaf(
    metadata: &fs::Metadata,
    _preexisting: bool,
    path: &Path,
) -> anyhow::Result<()> {
    use std::os::unix::fs::MetadataExt;

    if metadata.nlink() != 1 {
        anyhow::bail!(
            "output must have exactly one hard link for contained mutation: {} has link count {}",
            path.display(),
            metadata.nlink()
        );
    }
    Ok(())
}

#[cfg(not(unix))]
fn require_unaliased_output_leaf(
    _metadata: &fs::Metadata,
    preexisting: bool,
    path: &Path,
) -> anyhow::Result<()> {
    if preexisting {
        anyhow::bail!(
            "existing output cannot be safely overwritten because this platform does not expose a stable hard-link count: {}",
            path.display()
        );
    }
    Ok(())
}

fn read_regular_file(path: &Path) -> anyhow::Result<Vec<u8>> {
    let initial = fs::symlink_metadata(path)
        .with_context(|| format!("could not inspect artifact source {}", path.display()))?;
    if initial.file_type().is_symlink() || !initial.file_type().is_file() {
        anyhow::bail!(
            "artifact source is not a regular non-symlink file: {}",
            path.display()
        );
    }
    if initial.len() > ARTIFACT_MAX_BYTES {
        anyhow::bail!(
            "artifact source {} is {} bytes; reading stops at the {} byte limit",
            path.display(),
            initial.len(),
            ARTIFACT_MAX_BYTES
        );
    }
    let mut file = fs::File::open(path)
        .with_context(|| format!("could not open artifact source {}", path.display()))?;
    let opened = file
        .metadata()
        .with_context(|| format!("could not inspect open artifact source {}", path.display()))?;
    if !same_file_identity(&initial, &opened) {
        anyhow::bail!(
            "artifact source changed type or identity while opening: {}",
            path.display()
        );
    }
    let mut bytes = Vec::with_capacity(initial.len() as usize);
    let mut buffer = [0_u8; 64 * 1024];
    let mut total = 0_u64;
    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("could not read artifact source {}", path.display()))?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(read as u64)
            .context("artifact source byte counter overflow")?;
        if total > ARTIFACT_MAX_BYTES {
            anyhow::bail!(
                "artifact source {} grew beyond the {} byte limit",
                path.display(),
                ARTIFACT_MAX_BYTES
            );
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    let final_opened = file.metadata().with_context(|| {
        format!(
            "could not re-inspect open artifact source {}",
            path.display()
        )
    })?;
    let final_link = fs::symlink_metadata(path)
        .with_context(|| format!("could not re-inspect artifact source {}", path.display()))?;
    if !same_file_identity(&initial, &final_opened)
        || !same_file_identity(&initial, &final_link)
        || total != final_opened.len()
    {
        anyhow::bail!("artifact source changed while reading: {}", path.display());
    }
    Ok(bytes)
}

pub fn prepare_output_path(path: &Path) -> anyhow::Result<PathBuf> {
    let resolved = resolve_output_path(path)?;
    inspect_contained_path(
        &resolved.canonical_root,
        &resolved.path,
        PathRequirement::MayBeMissingFile,
    )?;
    create_missing_directories(&resolved.canonical_root, &resolved.path)?;
    inspect_contained_path(
        &resolved.canonical_root,
        &resolved.path,
        PathRequirement::MayBeMissingFile,
    )?;
    Ok(resolved.path)
}

fn inspect_output_path(path: &Path, requirement: PathRequirement) -> anyhow::Result<PathBuf> {
    let resolved = resolve_output_path(path)?;
    inspect_contained_path(&resolved.canonical_root, &resolved.path, requirement)?;
    Ok(resolved.path)
}

fn resolve_output_path(path: &Path) -> anyhow::Result<ResolvedOutputPath> {
    ARTIFACT_RUN.with(|state| {
        let state = state.borrow();
        let run = state.as_ref().context(
            "artifact output safety is not initialized; call begin_run before generation",
        )?;
        let alias = run
            .aliases
            .iter()
            .filter(|alias| path.starts_with(&alias.lexical_root))
            .max_by_key(|alias| alias.lexical_root.components().count())
            .with_context(|| {
                format!(
                    "generator output path is outside configured product roots: {}",
                    path.display()
                )
            })?;
        let relative = path
            .strip_prefix(&alias.lexical_root)
            .context("configured output alias prefix disappeared")?;
        Ok(ResolvedOutputPath {
            path: alias.canonical_root.join(relative),
            canonical_root: alias.canonical_root.clone(),
        })
    })
}

#[derive(Debug, Clone, Copy)]
enum PathRequirement {
    MayBeMissingFile,
    MayBeMissingDirectory,
    ExistingFile,
}

fn canonical_non_symlink_root(root: &Path, label: &str) -> anyhow::Result<PathBuf> {
    let metadata = fs::symlink_metadata(root)
        .with_context(|| format!("could not inspect {label} {}", root.display()))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        anyhow::bail!(
            "{label} must be a non-symlink directory: {}",
            root.display()
        );
    }
    fs::canonicalize(root)
        .with_context(|| format!("could not canonicalize {label} {}", root.display()))
}

fn canonical_existing_output_root(root: &Path, label: &str) -> anyhow::Result<PathBuf> {
    if !root.is_absolute() {
        anyhow::bail!("{label} must be absolute: {}", root.display());
    }
    if root.parent().is_none() {
        anyhow::bail!("{label} cannot be a filesystem root: {}", root.display());
    }

    let canonical = canonical_non_symlink_root(root, label)?;
    if !lexical_output_root_matches_canonical(root, &canonical) {
        anyhow::bail!(
            "{label} must not contain symlink ancestors: {}",
            root.display()
        );
    }
    Ok(canonical)
}

fn ensure_canonical_output_root(root: &Path, label: &str) -> anyhow::Result<PathBuf> {
    if !root.is_absolute() {
        anyhow::bail!("{label} must be absolute: {}", root.display());
    }
    if root.parent().is_none() {
        anyhow::bail!("{label} cannot be a filesystem root: {}", root.display());
    }

    let mut existing = root;
    let mut suffix = Vec::new();
    loop {
        match fs::symlink_metadata(existing) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
                    anyhow::bail!(
                        "{label} existing ancestor must be a non-symlink directory: {}",
                        existing.display()
                    );
                }
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                suffix.push(
                    existing
                        .file_name()
                        .with_context(|| format!("{label} has no file name"))?
                        .to_owned(),
                );
                existing = existing
                    .parent()
                    .with_context(|| format!("{label} has no existing ancestor"))?;
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("could not inspect {label} {}", existing.display()))
            }
        }
    }

    let canonical_existing = fs::canonicalize(existing)
        .with_context(|| format!("could not canonicalize {label} {}", existing.display()))?;
    let mut expected_canonical = canonical_existing.clone();
    for component in suffix.iter().rev() {
        expected_canonical.push(component);
    }
    if !lexical_output_root_matches_canonical(root, &expected_canonical) {
        anyhow::bail!(
            "{label} must not contain symlink ancestors: {}",
            root.display()
        );
    }

    let mut canonical = canonical_existing;
    for component in suffix.iter().rev() {
        canonical.push(component);
        match fs::symlink_metadata(&canonical) {
            Ok(metadata) if metadata.file_type().is_dir() => {}
            Ok(_) => anyhow::bail!(
                "{label} contains a symlink or non-directory: {}",
                canonical.display()
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&canonical).with_context(|| {
                    format!("could not create {label} directory {}", canonical.display())
                })?;
                let metadata = fs::symlink_metadata(&canonical).with_context(|| {
                    format!(
                        "could not re-inspect {label} directory {}",
                        canonical.display()
                    )
                })?;
                if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
                    anyhow::bail!(
                        "{label} creation did not produce a non-symlink directory: {}",
                        canonical.display()
                    );
                }
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "could not inspect {label} directory {}",
                        canonical.display()
                    )
                })
            }
        }
    }

    let canonical = canonical_non_symlink_root(&canonical, label)?;
    debug_assert!(lexical_output_root_matches_canonical(root, &canonical));
    Ok(canonical)
}

fn lexical_output_root_matches_canonical(lexical: &Path, canonical: &Path) -> bool {
    if lexical == canonical {
        return true;
    }
    #[cfg(target_os = "macos")]
    for (alias, resolved) in [
        (Path::new("/var"), Path::new("/private/var")),
        (Path::new("/tmp"), Path::new("/private/tmp")),
        (Path::new("/etc"), Path::new("/private/etc")),
    ] {
        if let Ok(suffix) = lexical.strip_prefix(alias) {
            if resolved.join(suffix) == canonical {
                return true;
            }
        }
    }
    false
}

fn inspect_contained_path(
    canonical_root: &Path,
    path: &Path,
    requirement: PathRequirement,
) -> anyhow::Result<()> {
    if !path.is_absolute() {
        anyhow::bail!("generator output path must be absolute: {}", path.display());
    }
    let relative = path.strip_prefix(canonical_root).with_context(|| {
        format!(
            "generator output path escapes product root {}: {}",
            canonical_root.display(),
            path.display()
        )
    })?;
    if relative.as_os_str().is_empty() {
        anyhow::bail!("generator output path cannot be the product root itself");
    }

    let mut current = canonical_root.to_path_buf();
    let components = relative.components().collect::<Vec<_>>();
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(component) = component else {
            anyhow::bail!(
                "generator output path contains an invalid component: {}",
                path.display()
            );
        };
        current.push(component);
        let is_leaf = index + 1 == components.len();
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    anyhow::bail!(
                        "generator output path contains a symlink: {}",
                        current.display()
                    );
                }
                if !is_leaf && !metadata.file_type().is_dir() {
                    anyhow::bail!(
                        "generator output path contains a non-directory ancestor: {}",
                        current.display()
                    );
                }
                if is_leaf {
                    match requirement {
                        PathRequirement::ExistingFile if !metadata.file_type().is_file() => {
                            anyhow::bail!(
                                "generator output is not a regular file: {}",
                                current.display()
                            );
                        }
                        PathRequirement::MayBeMissingFile if !metadata.file_type().is_file() => {
                            anyhow::bail!(
                                "generator output leaf has unsupported type: {}",
                                current.display()
                            );
                        }
                        PathRequirement::MayBeMissingDirectory
                            if !metadata.file_type().is_dir() =>
                        {
                            anyhow::bail!(
                                "generator output root leaf has unsupported type: {}",
                                current.display()
                            );
                        }
                        _ => {}
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if matches!(requirement, PathRequirement::ExistingFile) {
                    anyhow::bail!(
                        "required generator output path is missing: {}",
                        current.display()
                    );
                }
                break;
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "could not inspect generator output path {}",
                        current.display()
                    )
                });
            }
        }
    }
    Ok(())
}

fn preflight_output_trees(
    canonical_root: &Path,
    config_root: &Path,
    report_root: &Path,
    workspace: &AppWorkspace,
) -> anyhow::Result<()> {
    // Config source may intentionally live beside the product checkout. Its
    // generated contract is bounded by the independently configured config
    // root rather than by the app root.
    inspect_contained_path(
        config_root,
        &config_root.join("_specs/CONFIG_CONTRACT.md"),
        PathRequirement::MayBeMissingFile,
    )?;
    reject_symlinks_in_existing_tree(config_root)?;

    let mut roots = vec![
        canonical_root.join("backend/config/generated"),
        canonical_root.join("backend/src/schemas"),
        canonical_root.join("backend/src/routes"),
        canonical_root.join("backend/src/handlers"),
        canonical_root.join("backend/src/operations"),
        canonical_root.join("database/_pkg"),
        canonical_root.join("api_tests/src/schemas"),
        canonical_root.join("frontend/.appfw-ui"),
        canonical_root.join("frontend/src/generated"),
    ];
    if !workspace.uses_split_roots() {
        roots.push(canonical_root.join("database/src"));
    }

    for root in roots {
        inspect_contained_path(
            canonical_root,
            &root,
            PathRequirement::MayBeMissingDirectory,
        )?;
        reject_symlinks_in_existing_tree(&root)?;
    }
    reject_symlinks_in_existing_tree(report_root)?;
    inspect_contained_path(
        canonical_root,
        &canonical_root.join("podman-compose.yml"),
        PathRequirement::MayBeMissingFile,
    )?;
    Ok(())
}

fn reject_symlinks_in_existing_tree(path: &Path) -> anyhow::Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error).with_context(|| {
                format!("could not inspect output preflight path {}", path.display())
            })
        }
    };
    if metadata.file_type().is_symlink() {
        anyhow::bail!(
            "generator output preflight found a symlink: {}",
            path.display()
        );
    }
    if !metadata.file_type().is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(path).with_context(|| {
        format!(
            "could not read output preflight directory {}",
            path.display()
        )
    })? {
        reject_symlinks_in_existing_tree(
            &entry
                .with_context(|| format!("could not read entry in {}", path.display()))?
                .path(),
        )?;
    }
    Ok(())
}

fn create_missing_directories(canonical_root: &Path, path: &Path) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .context("generator output path has no parent")?;
    let relative = parent.strip_prefix(canonical_root).with_context(|| {
        format!(
            "generator output parent escapes product root: {}",
            parent.display()
        )
    })?;
    let mut current = canonical_root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(component) = component else {
            anyhow::bail!(
                "generator output parent contains an invalid component: {}",
                parent.display()
            );
        };
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_dir() => {}
            Ok(_) => anyhow::bail!(
                "generator output parent contains a symlink or non-directory: {}",
                current.display()
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current).with_context(|| {
                    format!(
                        "could not create generator output directory {}",
                        current.display()
                    )
                })?;
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "could not inspect generator output parent {}",
                        current.display()
                    )
                })
            }
        }
    }
    Ok(())
}

fn bounded_file_sha256(path: &Path) -> anyhow::Result<String> {
    let resolved = resolve_output_path(path)?;
    inspect_contained_path(
        &resolved.canonical_root,
        &resolved.path,
        PathRequirement::ExistingFile,
    )?;
    bounded_file_sha256_uncontained(&resolved.path)
}

fn bounded_file_sha256_uncontained(path: &Path) -> anyhow::Result<String> {
    let digest = bounded_file_sha256_raw_uncontained(path)?;
    Ok(format!(
        "sha256:{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ))
}

fn bounded_file_sha256_raw_uncontained(path: &Path) -> anyhow::Result<Vec<u8>> {
    let initial = fs::symlink_metadata(path)
        .with_context(|| format!("could not inspect {} for hashing", path.display()))?;
    if !initial.file_type().is_file() {
        anyhow::bail!("hash input is not a regular file: {}", path.display());
    }
    if initial.len() > ARTIFACT_MAX_BYTES {
        anyhow::bail!(
            "artifact {} is {} bytes; hashing stops at the {} byte limit",
            path.display(),
            initial.len(),
            ARTIFACT_MAX_BYTES
        );
    }
    let mut file = fs::File::open(path)
        .with_context(|| format!("could not open {} for hashing", path.display()))?;
    let opened = file
        .metadata()
        .with_context(|| format!("could not inspect open hash input {}", path.display()))?;
    if !same_file_identity(&initial, &opened) {
        anyhow::bail!(
            "artifact changed type or identity while opening for hashing: {}",
            path.display()
        );
    }
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut total = 0_u64;
    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("could not read {} for hashing", path.display()))?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(read as u64)
            .context("artifact hash byte counter overflow")?;
        if total > ARTIFACT_MAX_BYTES {
            anyhow::bail!(
                "artifact {} grew beyond the {} byte hash limit",
                path.display(),
                ARTIFACT_MAX_BYTES
            );
        }
        hasher.update(&buffer[..read]);
    }
    let final_opened = file
        .metadata()
        .with_context(|| format!("could not re-inspect open hash input {}", path.display()))?;
    let final_link = fs::symlink_metadata(path)
        .with_context(|| format!("could not re-inspect hash input {}", path.display()))?;
    if !same_file_identity(&initial, &final_opened)
        || !same_file_identity(&initial, &final_link)
        || total != final_opened.len()
    {
        anyhow::bail!("artifact changed while hashing: {}", path.display());
    }
    Ok(hasher.finalize().to_vec())
}

#[cfg(unix)]
fn same_file_identity(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    (
        left.dev(),
        left.ino(),
        left.mode(),
        left.len(),
        left.mtime(),
        left.mtime_nsec(),
        left.ctime(),
        left.ctime_nsec(),
    ) == (
        right.dev(),
        right.ino(),
        right.mode(),
        right.len(),
        right.mtime(),
        right.mtime_nsec(),
        right.ctime(),
        right.ctime_nsec(),
    )
}

#[cfg(not(unix))]
fn same_file_identity(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.file_type().is_file() && right.file_type().is_file() && left.len() == right.len()
}

#[cfg(unix)]
fn same_file_object(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    (left.dev(), left.ino(), left.mode()) == (right.dev(), right.ino(), right.mode())
}

#[cfg(not(unix))]
fn same_file_object(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.file_type().is_file() && right.file_type().is_file()
}

fn tree_sha256(path: &Path) -> anyhow::Result<String> {
    let mut files = vec![];
    collect_files(path, &mut files)?;
    files.sort();

    let mut hasher = Sha256::new();
    for file in files {
        let rel = file.strip_prefix(path).unwrap_or(file.as_path());
        hasher.update(rel.display().to_string().as_bytes());
        hasher.update(b"\0");
        let digest = bounded_file_sha256_raw_uncontained(&file)?;
        hasher.update(digest);
        hasher.update(b"\0");
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in
        fs::read_dir(dir).with_context(|| format!("could not read directory {}", dir.display()))?
    {
        let path = entry
            .with_context(|| format!("could not read entry in {}", dir.display()))?
            .path();
        if path.is_dir() {
            collect_files(&path, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn finalization_reconciles_post_emit_mutation_and_sorts_records() {
        let root = temp_root("final-bytes");
        fs::create_dir_all(&root).expect("create artifact test root");
        let root = activate_test_root(&root);
        let later = root.join("later.yaml");
        let earlier = root.join("earlier.yaml");
        fs::write(&later, "intermediate\n").expect("write intermediate bytes");
        fs::write(&earlier, "stable\n").expect("write stable bytes");

        let stale_hash = bounded_file_sha256_uncontained(&later).expect("hash intermediate bytes");
        let mut records = vec![
            record(&later, Some(stale_hash.clone())),
            record(&earlier, None),
        ];
        fs::write(&later, "final relationship-resolved bytes\n")
            .expect("mutate artifact after first emission");

        finalize_records(&mut records).expect("reconcile final artifact bytes");

        assert_eq!(records[0].path, earlier.display().to_string());
        assert_eq!(records[1].path, later.display().to_string());
        assert_ne!(
            records[1].content_sha256.as_deref(),
            Some(stale_hash.as_str())
        );
        assert_eq!(
            records[1].content_sha256,
            Some(bounded_file_sha256_uncontained(&later).expect("hash final bytes"))
        );

        fs::remove_dir_all(root).expect("remove artifact test root");
    }

    #[test]
    fn finalization_coalesces_identical_duplicate_emissions_deterministically() {
        let root = temp_root("identical-duplicate-emissions");
        fs::create_dir_all(&root).expect("create artifact test root");
        let root = activate_test_root(&root);
        let artifact = root.join("generated.yaml");
        fs::write(&artifact, "final bytes\n").expect("write artifact");

        let mut written = record(&artifact, None);
        written.action = "written";
        written.source_sha256 = Some("sha256:shared-source".to_string());
        let mut skipped = written.clone();
        skipped.action = "skipped";
        let mut records = vec![skipped, written];

        finalize_records(&mut records).expect("coalesce identical emissions");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].path, artifact.display().to_string());
        assert_eq!(records[0].action, "written");
        assert_eq!(
            records[0].content_sha256,
            Some(bounded_file_sha256_uncontained(&artifact).expect("hash final artifact"))
        );

        fs::remove_dir_all(root).expect("remove artifact test root");
    }

    #[test]
    fn finalization_rejects_duplicate_emissions_with_conflicting_authoritative_semantics() {
        let root = temp_root("conflicting-duplicate-emissions");
        fs::create_dir_all(&root).expect("create artifact test root");
        let root = activate_test_root(&root);
        let artifact = root.join("generated.yaml");
        fs::write(&artifact, "final bytes\n").expect("write artifact");

        let first = record(&artifact, None);
        let mut conflicting = first.clone();
        conflicting.ownership = ArtifactOwnership::HumanOwned;
        let error = finalize_records(&mut vec![first, conflicting])
            .expect_err("conflicting duplicate emissions must fail closed");

        assert!(format!("{error:#}").contains("conflicting authoritative semantics"));
        fs::remove_dir_all(root).expect("remove artifact test root");
    }

    #[test]
    fn finalization_fails_closed_for_missing_or_non_regular_artifacts() {
        let root = temp_root("invalid-final-state");
        fs::create_dir_all(&root).expect("create artifact test root");
        let root = activate_test_root(&root);

        let missing = root.join("missing.yaml");
        let missing_error = finalize_records(&mut vec![record(&missing, None)])
            .expect_err("missing artifact must fail");
        assert!(format!("{missing_error:#}").contains("required generator output path is missing"));

        let directory = root.join("directory-artifact");
        fs::create_dir_all(&directory).expect("create directory artifact");
        let directory_error = finalize_records(&mut vec![record(&directory, None)])
            .expect_err("directory artifact must fail");
        assert!(format!("{directory_error:#}").contains("not a regular file"));

        fs::remove_dir_all(root).expect("remove artifact test root");
    }

    #[cfg(unix)]
    #[test]
    fn finalization_rejects_symlink_artifacts_without_following_them() {
        use std::os::unix::fs::symlink;

        let root = temp_root("symlink-final-state");
        fs::create_dir_all(&root).expect("create artifact test root");
        let root = activate_test_root(&root);
        let target = root.join("target.yaml");
        let link = root.join("link.yaml");
        fs::write(&target, "target bytes\n").expect("write symlink target");
        symlink(&target, &link).expect("create artifact symlink");

        let error = finalize_records(&mut vec![record(&link, None)])
            .expect_err("symlink artifact must fail");
        assert!(format!("{error:#}").contains("contains a symlink"));

        fs::remove_dir_all(root).expect("remove artifact test root");
    }

    #[cfg(unix)]
    #[test]
    fn output_preflight_rejects_symlink_ancestor_without_outside_write() {
        use std::os::unix::fs::symlink;

        let root = temp_root("ancestor-symlink");
        let outside = temp_root("ancestor-symlink-outside");
        fs::create_dir_all(&root).expect("create product root");
        fs::create_dir_all(&outside).expect("create outside root");
        fs::write(outside.join("sentinel"), "unchanged\n").expect("write outside sentinel");
        let root = activate_test_root(&root);
        symlink(&outside, root.join("generated")).expect("create redirected output parent");

        let error = prepare_output_path(&root.join("generated/escaped.yaml"))
            .expect_err("ancestor symlink must stop the write");

        assert!(error.to_string().contains("contains a symlink"));
        assert!(!outside.join("escaped.yaml").exists());
        assert_eq!(
            fs::read_to_string(outside.join("sentinel")).expect("read outside sentinel"),
            "unchanged\n"
        );
        fs::remove_file(root.join("generated")).expect("remove output symlink");
        fs::remove_dir_all(root).expect("remove product root");
        fs::remove_dir_all(outside).expect("remove outside root");
    }

    #[test]
    fn configured_external_report_root_is_bounded_and_writable() {
        let base = temp_root("external-report-root");
        let config_root = base.join("external-model");
        let report_root = base.join("reports/appfw");
        fs::create_dir_all(config_root.join("schemas")).expect("create external config root");
        let workspace = test_workspace_with_report(&base, &config_root, report_root.clone());

        begin_run(&workspace).expect("initialize independent report root");
        let report = report_root.join("validation.json");
        safe_write(&report, b"{\"valid\":true}\n").expect("write configured external report");
        assert_eq!(
            fs::read_to_string(&report).expect("read external report"),
            "{\"valid\":true}\n"
        );
        bounded_file_sha256(&report).expect("hash report beneath configured report root");

        let outside = base.join("outside.json");
        let error = safe_write(&outside, b"must not be written")
            .expect_err("write outside configured roots must fail");
        assert!(error
            .to_string()
            .contains("outside configured product roots"));
        assert!(!outside.exists());

        fs::remove_dir_all(base).expect("remove external report fixture");
    }

    #[test]
    fn configured_external_config_root_receives_contract_and_remains_bounded() {
        let base = temp_root("external-config-root");
        let config_root = base.join("external-model");
        fs::create_dir_all(config_root.join("schemas")).expect("create external config root");
        let workspace = test_workspace(&base, &config_root);

        begin_run(&workspace).expect("initialize independent config root");
        crate::config_contract::emit(&workspace).expect("emit config contract");

        let contract = config_root.join("_specs/CONFIG_CONTRACT.md");
        assert!(
            contract.is_file(),
            "external config contract must be emitted"
        );
        assert!(fs::read_to_string(&contract)
            .expect("read external config contract")
            .contains("# app_gen Config Contract"));

        let outside = base.join("outside/escaped.md");
        let error = safe_write(&outside, b"must not be written")
            .expect_err("write outside all configured roots must fail");
        assert!(error
            .to_string()
            .contains("outside configured product roots"));
        assert!(!outside.exists());

        fs::remove_dir_all(base).expect("remove external config fixture");
    }

    #[cfg(unix)]
    #[test]
    fn configured_external_config_root_rejects_symlink_leaf_and_ancestor() {
        use std::os::unix::fs::symlink;

        let leaf_base = temp_root("config-root-symlink-leaf");
        let leaf_target = leaf_base.join("target-model");
        let leaf_alias = leaf_base.join("config-alias");
        fs::create_dir_all(leaf_target.join("schemas")).expect("create leaf target model");
        symlink(&leaf_target, &leaf_alias).expect("create config-root leaf symlink");
        let leaf_workspace = test_workspace(&leaf_base, &leaf_alias);

        let leaf_error = begin_run(&leaf_workspace)
            .expect_err("configured config-root symlink leaf must fail closed");
        assert!(leaf_error.to_string().contains("non-symlink directory"));
        assert!(!leaf_target.join("_specs/CONFIG_CONTRACT.md").exists());
        fs::remove_file(leaf_alias).expect("remove config-root leaf symlink");
        fs::remove_dir_all(leaf_base).expect("remove config-root leaf fixture");

        let ancestor_base = temp_root("config-root-symlink-ancestor");
        let ancestor_target = ancestor_base.join("target");
        let ancestor_alias = ancestor_base.join("alias");
        fs::create_dir_all(ancestor_target.join("model/schemas"))
            .expect("create ancestor target model");
        symlink(&ancestor_target, &ancestor_alias).expect("create config-root ancestor symlink");
        let ancestor_config = ancestor_alias.join("model");
        let ancestor_workspace = test_workspace(&ancestor_base, &ancestor_config);

        let ancestor_error = begin_run(&ancestor_workspace)
            .expect_err("configured config-root symlink ancestor must fail closed");
        assert!(ancestor_error.to_string().contains("symlink ancestors"));
        assert!(!ancestor_target
            .join("model/_specs/CONFIG_CONTRACT.md")
            .exists());
        fs::remove_file(ancestor_alias).expect("remove config-root ancestor symlink");
        fs::remove_dir_all(ancestor_base).expect("remove config-root ancestor fixture");
    }

    #[cfg(unix)]
    #[test]
    fn configured_external_config_root_rejects_symlinked_contract_parent() {
        use std::os::unix::fs::symlink;

        let base = temp_root("config-contract-parent-symlink");
        let config_root = base.join("external-model");
        let outside = base.join("outside");
        fs::create_dir_all(config_root.join("schemas")).expect("create external config root");
        fs::create_dir_all(&outside).expect("create outside config target");
        fs::write(outside.join("sentinel"), "unchanged\n").expect("write outside sentinel");
        symlink(&outside, config_root.join("_specs")).expect("redirect contract parent");
        let workspace = test_workspace(&base, &config_root);

        let error = begin_run(&workspace)
            .expect_err("symlinked config-contract parent must fail before writing");
        assert!(error.to_string().contains("contains a symlink"));
        assert!(!outside.join("CONFIG_CONTRACT.md").exists());
        assert_eq!(
            fs::read_to_string(outside.join("sentinel")).expect("read outside sentinel"),
            "unchanged\n"
        );

        fs::remove_file(config_root.join("_specs")).expect("remove contract-parent symlink");
        fs::remove_dir_all(base).expect("remove contract-parent fixture");
    }

    #[cfg(unix)]
    #[test]
    fn configured_report_root_rejects_symlink_leaf_and_ancestor() {
        use std::os::unix::fs::symlink;

        let leaf_base = temp_root("report-root-symlink");
        let leaf_target = leaf_base.join("target");
        let leaf_report = leaf_base.join("report");
        fs::create_dir_all(&leaf_target).expect("create report symlink target");
        symlink(&leaf_target, &leaf_report).expect("create report-root symlink");
        let leaf_config = leaf_base.join("model");
        fs::create_dir_all(leaf_config.join("schemas")).expect("create report leaf config");
        let leaf_workspace =
            test_workspace_with_report(&leaf_base, &leaf_config, leaf_report.clone());
        let leaf_error = begin_run(&leaf_workspace)
            .expect_err("configured report symlink must fail closed through workspace");
        assert!(leaf_error.to_string().contains("non-symlink directory"));
        fs::remove_file(leaf_report).expect("remove report-root symlink");
        fs::remove_dir_all(leaf_base).expect("remove report-root symlink fixture");

        let ancestor_base = temp_root("report-root-ancestor-symlink");
        let ancestor_target = ancestor_base.join("target");
        let ancestor_alias = ancestor_base.join("alias");
        fs::create_dir_all(ancestor_target.join("report")).expect("create report target");
        symlink(&ancestor_target, &ancestor_alias).expect("create report-root ancestor symlink");
        let ancestor_report = ancestor_alias.join("report");
        let ancestor_config = ancestor_base.join("model");
        fs::create_dir_all(ancestor_config.join("schemas")).expect("create report ancestor config");
        let ancestor_workspace =
            test_workspace_with_report(&ancestor_base, &ancestor_config, ancestor_report);
        let ancestor_error = begin_run(&ancestor_workspace)
            .expect_err("configured report ancestor symlink must fail closed through workspace");
        assert!(ancestor_error.to_string().contains("symlink ancestors"));

        let missing_report = ancestor_alias.join("missing/report");
        let missing_workspace =
            test_workspace_with_report(&ancestor_base, &ancestor_config, missing_report);
        let missing_error = begin_run(&missing_workspace)
            .expect_err("missing report below symlink ancestor must fail before creation");
        assert!(
            missing_error.to_string().contains("non-symlink directory"),
            "{missing_error:#}"
        );
        assert!(!ancestor_target.join("missing").exists());
        fs::remove_file(ancestor_alias).expect("remove report-root ancestor symlink");
        fs::remove_dir_all(ancestor_base).expect("remove report-root ancestor fixture");
    }

    #[cfg(unix)]
    #[test]
    fn safe_write_rejects_symlink_leaf_without_overwriting_target() {
        use std::os::unix::fs::symlink;

        let root = temp_root("safe-write-symlink-leaf");
        let outside = temp_root("safe-write-symlink-leaf-outside");
        fs::create_dir_all(&root).expect("create product root");
        fs::create_dir_all(&outside).expect("create outside root");
        let sentinel = outside.join("sentinel");
        fs::write(&sentinel, "unchanged\n").expect("write outside sentinel");
        let root = activate_test_root(&root);
        let redirected = root.join("redirected.json");
        symlink(&sentinel, &redirected).expect("create redirected output leaf");

        let error = safe_write(&redirected, b"replacement\n")
            .expect_err("symlink output leaf must stop the write");

        assert!(format!("{error:#}").contains("contains a symlink"));
        assert_eq!(
            fs::read_to_string(&sentinel).expect("read outside sentinel"),
            "unchanged\n"
        );
        fs::remove_file(redirected).expect("remove redirected output leaf");
        fs::remove_dir_all(root).expect("remove product root");
        fs::remove_dir_all(outside).expect("remove outside root");
    }

    #[cfg(unix)]
    #[test]
    fn safe_write_rejects_hardlinked_leaf_without_overwriting_target() {
        use std::os::unix::fs::MetadataExt;

        let root = temp_root("safe-write-hardlink-leaf");
        let outside = temp_root("safe-write-hardlink-leaf-outside");
        fs::create_dir_all(&root).expect("create product root");
        fs::create_dir_all(&outside).expect("create outside root");
        let sentinel = outside.join("sentinel");
        fs::write(&sentinel, "unchanged\n").expect("write outside sentinel");
        let root = activate_test_root(&root);
        let aliased = root.join("aliased.json");
        fs::hard_link(&sentinel, &aliased).expect("create hardlinked output leaf");
        assert_eq!(
            fs::symlink_metadata(&aliased)
                .expect("inspect hardlinked output")
                .nlink(),
            2
        );

        let error = safe_write(&aliased, b"replacement\n")
            .expect_err("hardlinked output leaf must stop the write");

        assert!(format!("{error:#}").contains("exactly one hard link"));
        assert_eq!(
            fs::read_to_string(&sentinel).expect("read outside sentinel"),
            "unchanged\n"
        );
        fs::remove_file(aliased).expect("remove hardlinked output leaf");
        fs::remove_dir_all(root).expect("remove product root");
        fs::remove_dir_all(outside).expect("remove outside root");
    }

    #[cfg(unix)]
    #[test]
    fn copy_emission_rejects_symlink_source_without_writing_output() {
        use std::os::unix::fs::symlink;

        let root = temp_root("copy-source-symlink");
        let outside = temp_root("copy-source-symlink-outside");
        fs::create_dir_all(&root).expect("create product root");
        fs::create_dir_all(&outside).expect("create outside root");
        let sentinel = outside.join("source.txt");
        fs::write(&sentinel, "outside source\n").expect("write outside source");
        let root = activate_test_root(&root);
        let source_link = root.join("source-link.txt");
        symlink(&sentinel, &source_link).expect("create source symlink");
        let output = root.join("generated/copied.txt");
        let artifact = Artifact::generated_copy_from(
            output.clone(),
            source_link.clone(),
            OverwriteMode::Always,
        );

        let error = emit(&artifact).expect_err("symlink source must stop copy emission");

        assert!(format!("{error:#}").contains("not a regular non-symlink file"));
        assert!(!output.exists());
        assert_eq!(
            fs::read_to_string(&sentinel).expect("read outside source"),
            "outside source\n"
        );
        fs::remove_file(source_link).expect("remove source symlink");
        fs::remove_dir_all(root).expect("remove product root");
        fs::remove_dir_all(outside).expect("remove outside root");
    }

    #[test]
    fn record_raw_fails_closed_when_content_or_source_hash_cannot_be_retained() {
        let root = temp_root("record-hash-failure");
        fs::create_dir_all(&root).expect("create artifact root");
        let root = activate_test_root(&root);

        let missing_artifact = root.join("missing.yaml");
        let content_error = record_raw(
            missing_artifact,
            ArtifactOwnership::Generated,
            OverwriteMode::Always,
            "written",
            None,
        )
        .expect_err("missing final artifact hash must fail closed");
        assert!(format!("{content_error:#}")
            .contains("could not hash recorded artifact bytes without weakening evidence"));

        let artifact = root.join("generated.yaml");
        fs::write(&artifact, "generated bytes\n").expect("write generated artifact");
        let source_error = record_raw(
            artifact,
            ArtifactOwnership::Generated,
            OverwriteMode::Always,
            "written",
            Some(&root.join("missing-source.yaml")),
        )
        .expect_err("missing source hash must fail closed");
        assert!(format!("{source_error:#}")
            .contains("could not hash recorded artifact source without weakening evidence"));

        fs::remove_dir_all(root).expect("remove artifact root");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn canonical_output_root_accepts_macos_var_private_var_alias() {
        let lexical_root = temp_root("macos-var-alias");
        fs::create_dir_all(lexical_root.join(".appfw")).expect("create lexical product root");
        let canonical_root = fs::canonicalize(&lexical_root).expect("canonicalize product root");
        assert_ne!(
            lexical_root, canonical_root,
            "fixture must exercise /var alias"
        );

        let lexical_report = lexical_root.join(".appfw/target/appfw");
        let canonical_report = ensure_canonical_output_root(&lexical_report, "test report root")
            .expect("resolve lexical report root through standard macOS alias");

        assert_eq!(canonical_report, canonical_root.join(".appfw/target/appfw"));
        fs::remove_dir_all(canonical_root).expect("remove macOS alias fixture");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn begin_run_and_finalize_accept_macos_lexical_and_canonical_aliases_once() {
        let lexical_root = temp_root("macos-var-full-run");
        fs::create_dir_all(lexical_root.join(".appfw/model")).expect("create lexical config root");
        let canonical_root = fs::canonicalize(&lexical_root).expect("canonicalize product root");
        assert_ne!(
            lexical_root, canonical_root,
            "fixture must exercise /var and /private/var aliases"
        );
        let workspace = AppWorkspace {
            app_root: lexical_root.clone(),
            framework_root: lexical_root.clone(),
            generator_root: lexical_root.join("app_gen"),
            config_root: lexical_root.join(".appfw/model"),
            templates_root: lexical_root.join("app_gen/_templates"),
            report_root: lexical_root.join(".appfw/target/appfw"),
        };

        begin_run(&workspace).expect("start artifact run through lexical /var alias");
        let lexical_artifact = lexical_root.join("frontend/src/generated/portable.ts");
        safe_write(&lexical_artifact, b"export const portable = true;\n")
            .expect("write through lexical alias");
        let mut run_records = vec![record(&lexical_artifact, None)];
        finalize_records(&mut run_records)
            .expect("finalize lexical artifact through canonical alias");

        let canonical_artifact = canonical_root.join("frontend/src/generated/portable.ts");
        assert_eq!(run_records.len(), 1);
        assert_eq!(
            run_records[0].path,
            canonical_artifact.display().to_string()
        );
        assert_eq!(
            run_records[0].content_sha256,
            Some(bounded_file_sha256_uncontained(&canonical_artifact).expect("hash final bytes"))
        );

        fs::remove_dir_all(canonical_root).expect("remove macOS full-run fixture");
    }

    #[test]
    fn bounded_hash_rejects_artifact_larger_than_handoff_limit() {
        let root = temp_root("oversized-hash");
        fs::create_dir_all(&root).expect("create hash root");
        let oversized = root.join("oversized.bin");
        fs::File::create(&oversized)
            .expect("create sparse oversized artifact")
            .set_len(ARTIFACT_MAX_BYTES + 1)
            .expect("size sparse oversized artifact");

        let error = bounded_file_sha256_uncontained(&oversized)
            .expect_err("oversized artifact hashing must stop");

        assert!(error.to_string().contains("hashing stops"));
        fs::remove_dir_all(root).expect("remove hash root");
    }

    fn record(path: &Path, content_sha256: Option<String>) -> ArtifactRecord {
        ArtifactRecord {
            path: path.display().to_string(),
            ownership: ArtifactOwnership::Generated,
            overwrite: "always",
            action: "written",
            content_sha256,
            source_sha256: None,
        }
    }

    fn temp_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "appfw-artifacts-{label}-{}-{nanos}",
            std::process::id()
        ))
    }

    fn test_workspace(base: &Path, config_root: &Path) -> AppWorkspace {
        test_workspace_with_report(base, config_root, base.join("product/.appfw/target/appfw"))
    }

    fn test_workspace_with_report(
        base: &Path,
        config_root: &Path,
        report_root: PathBuf,
    ) -> AppWorkspace {
        let app_root = base.join("product");
        let framework_root = base.join("framework");
        let generator_root = framework_root.join("app_gen");
        let templates_root = generator_root.join("_templates");
        fs::create_dir_all(&app_root).expect("create test product root");
        fs::create_dir_all(generator_root.join("src")).expect("create test generator source");
        fs::create_dir_all(&templates_root).expect("create test template root");
        AppWorkspace::from_roots(
            app_root,
            framework_root,
            generator_root,
            config_root.to_path_buf(),
            templates_root,
            report_root,
            base,
        )
        .expect("construct test workspace")
    }

    fn activate_test_root(root: &Path) -> PathBuf {
        let app_root = fs::canonicalize(root).expect("canonicalize artifact test root");
        ARTIFACT_RUN.with(|state| {
            *state.borrow_mut() = Some(ArtifactRun {
                aliases: vec![PathAlias {
                    lexical_root: app_root.clone(),
                    canonical_root: app_root.clone(),
                }],
            })
        });
        app_root
    }
}
