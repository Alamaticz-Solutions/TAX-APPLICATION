use std::{
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
};

use sha2::{Digest, Sha256};

const REQUIRED_HELPERS: &[&str] = &["appfw", "app_gen", "appfw_introspect", "database"];
const PACKAGE_WORK_ITEM_SOURCE: &str = "app_gen/_golden/downstream_apps/second-consumer/starter/.appfw/model/schemas/consumer/entity_types/work_item.yaml";
const CONSUMER_WORK_ITEM_TARGET: &str = ".appfw/model/schemas/consumer/entity_types/work_item.yaml";
const PENDING_UPGRADE_JOURNAL: &str = ".appfw/package-first-upgrade.pending.json";
const COMPLETED_UPGRADE_STATE: &str = ".appfw/package-first-upgrade.json";

#[derive(Debug, PartialEq, Eq)]
pub struct PackageFirstContract {
    pub consumer: String,
    pub package_root: PathBuf,
    pub version_identity: String,
    pub helpers: Vec<String>,
    pub no_checkout: bool,
    pub deterministic_generation: bool,
    pub compatibility: String,
    pub upgrade_guidance: String,
    pub lifecycle: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PackageUpgrade {
    pub from_version: String,
    pub to_version: String,
    pub consumer_root: PathBuf,
}

pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("package-first") => run_package_first(&args[1..]),
        Some("compatibility") => run_compatibility(&args[1..]),
        Some("upgrade") => run_upgrade(&args[1..]),
        Some("recover") => run_recover(&args[1..]),
        _ => Err(
            "usage: appfw lifecycle package-first|compatibility|upgrade|recover ...".to_string(),
        ),
    }
}

fn run_recover(args: &[String]) -> Result<(), String> {
    let options = lifecycle_options(args)?;
    if options.len() != 1 || !options.contains_key("--consumer-root") {
        return Err("usage: appfw lifecycle recover --consumer-root <consumer-root>".to_string());
    }
    let consumer_root = required_path(&options, "--consumer-root")?;
    recover_package_transition(&consumer_root)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "consumer_root": consumer_root,
            "recovered": true,
            "next": "retry lifecycle upgrade explicitly; recovery does not continue an upgrade",
        }))
        .map_err(|error| error.to_string())?
    );
    Ok(())
}

fn run_package_first(args: &[String]) -> Result<(), String> {
    let mut package_root = None;
    let mut consumer = "second-consumer".to_string();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--package-root" => {
                package_root = args.get(index + 1).map(PathBuf::from);
                index += 2;
            }
            "--consumer" => {
                consumer = args
                    .get(index + 1)
                    .ok_or_else(|| "--consumer requires a value".to_string())?
                    .clone();
                index += 2;
            }
            value => return Err(format!("unknown lifecycle option: {value}")),
        }
    }

    let package_root = package_root.ok_or_else(|| "--package-root is required".to_string())?;
    let contract = inspect_package(&package_root, consumer)?;
    let output = serde_json::json!({
        "consumer": contract.consumer,
        "package_root": contract.package_root,
        "version_identity": contract.version_identity,
        "helpers": contract.helpers,
        "no_checkout": contract.no_checkout,
        "deterministic_generation": contract.deterministic_generation,
        "compatibility": contract.compatibility,
        "upgrade_guidance": contract.upgrade_guidance,
        "lifecycle": contract.lifecycle,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&output)
            .map_err(|error| format!("failed to serialize lifecycle contract: {error}"))?
    );
    Ok(())
}

fn run_compatibility(args: &[String]) -> Result<(), String> {
    let options = lifecycle_options(args)?;
    let package_root = required_path(&options, "--package-root")?;
    let expected_version = required_value(&options, "--expected-version")?;
    let contract = inspect_package(&package_root, "compatibility-check".to_string())?;
    let compatible = contract.version_identity == expected_version;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "package_root": package_root,
            "expected_version": expected_version,
            "actual_version": contract.version_identity,
            "compatible": compatible,
            "compatibility_scope": "package-version-only; matching versions do not claim archive or content-hash equivalence",
            "failure_guidance": "select an extracted package with the expected version, then rerun generate --check and focused tests",
        }))
        .map_err(|error| error.to_string())?
    );
    if compatible {
        Ok(())
    } else {
        Err("package version is incompatible; see failure_guidance".to_string())
    }
}

fn run_upgrade(args: &[String]) -> Result<(), String> {
    let options = lifecycle_options(args)?;
    let upgrade = upgrade_package(
        &required_path(&options, "--from-package-root")?,
        &required_path(&options, "--to-package-root")?,
        &required_path(&options, "--consumer-root")?,
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "from_version": upgrade.from_version,
            "to_version": upgrade.to_version,
            "consumer_root": upgrade.consumer_root,
            "upgraded": true,
            "next": ["run generate --check", "run focused tests", "run compatibility"],
        }))
        .map_err(|error| error.to_string())?
    );
    Ok(())
}

fn lifecycle_options(
    args: &[String],
) -> Result<std::collections::BTreeMap<String, String>, String> {
    let mut options = std::collections::BTreeMap::new();
    let mut index = 0;
    while index < args.len() {
        let option = &args[index];
        if !option.starts_with("--") {
            return Err(format!("unknown lifecycle option: {option}"));
        }
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("{option} requires a value"))?;
        options.insert(option.clone(), value.clone());
        index += 2;
    }
    Ok(options)
}

fn required_path(
    options: &std::collections::BTreeMap<String, String>,
    name: &str,
) -> Result<PathBuf, String> {
    options
        .get(name)
        .map(PathBuf::from)
        .ok_or_else(|| format!("{name} is required"))
}

fn required_value(
    options: &std::collections::BTreeMap<String, String>,
    name: &str,
) -> Result<String, String> {
    options
        .get(name)
        .cloned()
        .ok_or_else(|| format!("{name} is required"))
}

pub fn inspect_package(
    package_root: &Path,
    consumer: String,
) -> Result<PackageFirstContract, String> {
    let manifest_path = package_root.join("app-framework-package.json");
    let manifest = std::fs::read_to_string(&manifest_path)
        .map_err(|_| format!("package identity is missing: {}", manifest_path.display()))?;
    let version_identity = serde_json::from_str::<serde_json::Value>(&manifest)
        .ok()
        .and_then(|value| {
            value
                .get("version")
                .and_then(|version| version.as_str())
                .map(str::to_string)
        })
        .ok_or_else(|| format!("package version is missing: {}", manifest_path.display()))?;

    let helpers = REQUIRED_HELPERS
        .iter()
        .map(|helper| {
            let path = package_root.join("bin").join(helper);
            if !path.is_file() {
                return Err(format!("packaged helper is missing: {}", path.display()));
            }
            Ok((*helper).to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;

    for (required, expected_kind) in [
        ("scripts/appfw", "regular file"),
        ("docs/start/delivery-profiles.json", "regular file"),
    ] {
        let path = package_root.join(required);
        if !path.is_file() {
            return Err(format!(
                "package lifecycle input must be a {expected_kind}: {}",
                path.display()
            ));
        }
    }
    let templates = package_root.join("app_gen/_templates");
    if !templates.is_dir() {
        return Err(format!(
            "package lifecycle input must be a directory: {}",
            templates.display()
        ));
    }
    if package_root.join(".git").exists() {
        return Err(
            "package-first lifecycle requires an extracted package without a checkout".to_string(),
        );
    }

    Ok(PackageFirstContract {
        consumer,
        package_root: package_root.to_path_buf(),
        version_identity,
        helpers,
        no_checkout: true,
        deterministic_generation: true,
        compatibility: "package-version-only; archive SHA-256 identity is a separate proof gate"
            .to_string(),
        upgrade_guidance: "select a new package version, rerun generate --check, then rerun compatibility and focused tests".to_string(),
        lifecycle: vec![
            "create".to_string(),
            "generate".to_string(),
            "local-feedback".to_string(),
            "test".to_string(),
            "upgrade".to_string(),
            "compatibility".to_string(),
        ],
    })
}

pub fn upgrade_package(
    from_root: &Path,
    to_root: &Path,
    consumer_root: &Path,
) -> Result<PackageUpgrade, String> {
    let from = inspect_package(from_root, "upgrade-source".to_string())?;
    let to = inspect_package(to_root, "upgrade-target".to_string())?;
    if from.version_identity == to.version_identity {
        return Err("upgrade target must have a different package version".to_string());
    }
    let transition = prepare_package_work_item_transition(to_root, consumer_root)?;
    apply_package_work_item_transition(
        transition,
        &from.version_identity,
        &to.version_identity,
        false,
        false,
    )?;
    Ok(PackageUpgrade {
        from_version: from.version_identity,
        to_version: to.version_identity,
        consumer_root: consumer_root.to_path_buf(),
    })
}

struct PackageWorkItemTransition {
    consumer_root: PathBuf,
    source_bytes: Vec<u8>,
    target_path: PathBuf,
    original_bytes: Vec<u8>,
    applied_sha256: String,
    generated_files: Vec<TransitionFile>,
    empty_enum_dirs: Vec<PathBuf>,
    pending_path: PathBuf,
    completed_path: PathBuf,
    original_completed_state: Option<Vec<u8>>,
}

struct TransitionFile {
    path: PathBuf,
    bytes: Vec<u8>,
}

fn prepare_package_work_item_transition(
    package_root: &Path,
    consumer_root: &Path,
) -> Result<PackageWorkItemTransition, String> {
    let package_root = canonical_root(package_root, "package")?;
    let consumer_root = canonical_root(consumer_root, "consumer")?;
    let source_path = contained_existing_file(
        &package_root,
        PACKAGE_WORK_ITEM_SOURCE,
        "package WorkItem source",
    )?;
    let target_path = contained_existing_file(
        &consumer_root,
        CONSUMER_WORK_ITEM_TARGET,
        "consumer WorkItem upgrade target",
    )?;
    let source_bytes = fs::read(&source_path).map_err(|error| {
        format!(
            "failed to read package WorkItem upgrade source {}: {error}",
            source_path.display()
        )
    })?;
    let original_bytes = fs::read(&target_path).map_err(|error| {
        format!(
            "failed to snapshot consumer WorkItem upgrade target {}: {error}",
            target_path.display()
        )
    })?;
    let pending_path = contained_new_file(
        &consumer_root,
        PENDING_UPGRADE_JOURNAL,
        "pending lifecycle journal",
    )?;
    let completed_path = contained_new_file(
        &consumer_root,
        COMPLETED_UPGRADE_STATE,
        "completed lifecycle state",
    )?;
    match fs::symlink_metadata(&pending_path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
                return Err(format!(
                    "pending lifecycle journal must be a regular file: {}",
                    pending_path.display()
                ));
            }
            return Err(format!("unresolved package upgrade pending journal exists: {}; recover the recorded bytes before retrying", pending_path.display()));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "failed to inspect pending lifecycle journal {}: {error}",
                pending_path.display()
            ))
        }
    }
    let original_completed_state = match fs::symlink_metadata(&completed_path) {
        Ok(metadata) if !metadata.file_type().is_symlink() && metadata.file_type().is_file() => {
            Some(fs::read(&completed_path).map_err(|error| {
                format!(
                    "failed to snapshot completed lifecycle state {}: {error}",
                    completed_path.display()
                )
            })?)
        }
        Ok(_) => {
            return Err(format!(
                "completed lifecycle state is not a regular file: {}",
                completed_path.display()
            ))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(format!(
                "failed to inspect completed lifecycle state {}: {error}",
                completed_path.display()
            ))
        }
    };
    let (generated_files, empty_enum_dirs) = snapshot_generated_transition(&consumer_root)?;

    Ok(PackageWorkItemTransition {
        consumer_root,
        applied_sha256: sha256_hex(&source_bytes),
        source_bytes,
        target_path,
        original_bytes,
        generated_files,
        empty_enum_dirs,
        pending_path,
        completed_path,
        original_completed_state,
    })
}

fn apply_package_work_item_transition(
    transition: PackageWorkItemTransition,
    from_version: &str,
    to_version: &str,
    inject_failure_after_model: bool,
    inject_compensation_failure: bool,
) -> Result<String, String> {
    let generated_files = transition
        .generated_files
        .iter()
        .map(|file| {
            Ok(serde_json::json!({
                "path": relative_to(&transition.consumer_root, &file.path)?,
                "bytes_hex": hex(&file.bytes),
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let generated_dirs = transition
        .empty_enum_dirs
        .iter()
        .map(|path| relative_to(&transition.consumer_root, path))
        .collect::<Result<Vec<_>, _>>()?;
    let journal = serde_json::json!({
        "schema": "appfw_package_first_pending@2",
        "source": PACKAGE_WORK_ITEM_SOURCE,
        "target": CONSUMER_WORK_ITEM_TARGET,
        "original_model_hex": hex(&transition.original_bytes),
        "generated_files": generated_files,
        "generated_dirs": generated_dirs,
        "completed_state": transition.original_completed_state.as_ref().map(|bytes| hex(bytes)),
    });
    let journal_bytes = serde_json::to_vec_pretty(&journal)
        .map_err(|error| format!("failed to serialize pending lifecycle journal: {error}"))?;
    atomic_write(&transition.pending_path, &journal_bytes)?;

    let result = (|| {
        atomic_write(&transition.target_path, &transition.source_bytes)?;
        verify_package_work_item_bytes(
            &transition.source_bytes,
            &fs::read(&transition.target_path).map_err(|error| {
                format!(
                    "failed to verify consumer WorkItem upgrade target {}: {error}",
                    transition.target_path.display()
                )
            })?,
        )?;
        if inject_failure_after_model {
            return Err("injected failure after model replacement".to_string());
        }
        for file in &transition.generated_files {
            fs::remove_file(&file.path).map_err(|error| {
                format!(
                    "failed to remove generated enum transition file {}: {error}",
                    file.path.display()
                )
            })?;
        }
        for directory in &transition.empty_enum_dirs {
            if fs::read_dir(directory)
                .map_err(|error| {
                    format!(
                        "failed to inspect enum transition directory {}: {error}",
                        directory.display()
                    )
                })?
                .next()
                .is_none()
            {
                fs::remove_dir(directory).map_err(|error| {
                    format!(
                        "failed to remove empty enum transition directory {}: {error}",
                        directory.display()
                    )
                })?;
            }
        }
        let state = serde_json::to_vec_pretty(&serde_json::json!({
            "from_version": from_version,
            "to_version": to_version,
            "source": "package-first lifecycle",
            "compatibility_scope": "package-version-only; archive SHA-256 identity is a separate proof gate",
            "model_transition": { "source": PACKAGE_WORK_ITEM_SOURCE, "target": CONSUMER_WORK_ITEM_TARGET, "applied_sha256": transition.applied_sha256 },
        }))
        .map_err(|error| format!("failed to serialize consumer upgrade state: {error}"))?;
        atomic_write(&transition.completed_path, &state)?;
        fs::remove_file(&transition.pending_path).map_err(|error| {
            format!(
                "failed to remove pending lifecycle journal {}: {error}",
                transition.pending_path.display()
            )
        })?;
        Ok(transition.applied_sha256.clone())
    })();

    if let Err(error) = result {
        if inject_compensation_failure {
            return Err(format!("{error}; compensation failed: injected test-only compensation failure; unresolved pending journal retained at {}", transition.pending_path.display()));
        }
        if let Err(compensation) = compensate_package_transition(&transition) {
            return Err(format!("{error}; compensation failed: {compensation}; unresolved pending journal retained at {}", transition.pending_path.display()));
        }
        return Err(error);
    }
    result
}

fn verify_package_work_item_bytes(expected: &[u8], actual: &[u8]) -> Result<(), String> {
    if expected == actual {
        Ok(())
    } else {
        Err(
            "consumer WorkItem upgrade target does not match the package source after write"
                .to_string(),
        )
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn snapshot_generated_transition(
    consumer_root: &Path,
) -> Result<(Vec<TransitionFile>, Vec<PathBuf>), String> {
    let schemas_root = consumer_root.join(".appfw/model/schemas");
    if !schemas_root.is_dir() {
        return Ok((Vec::new(), Vec::new()));
    }

    let mut files = Vec::new();
    let mut directories = Vec::new();

    for entry in std::fs::read_dir(&schemas_root)
        .map_err(|error| format!("failed to inspect consumer schemas: {error}"))?
    {
        let entry =
            entry.map_err(|error| format!("failed to inspect consumer schema entry: {error}"))?;
        if !entry
            .file_type()
            .map_err(|error| format!("failed to inspect consumer schema type: {error}"))?
            .is_dir()
        {
            continue;
        }

        let enum_dir = entry.path().join("gql_enum_types");
        let metadata = match fs::symlink_metadata(&enum_dir) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(format!(
                    "failed to inspect enum transition directory {}: {error}",
                    enum_dir.display()
                ))
            }
        };
        if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
            return Err(format!(
                "enum transition directory must be a contained directory: {}",
                enum_dir.display()
            ));
        }
        if !enum_dir.starts_with(consumer_root) {
            return Err(format!(
                "enum transition directory escapes consumer root: {}",
                enum_dir.display()
            ));
        }
        let generated = enum_dir.join("_res.yaml");
        match fs::symlink_metadata(&generated) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
                    return Err(format!(
                        "generated enum transition file must be a regular file: {}",
                        generated.display()
                    ));
                }
                files.push(TransitionFile {
                    path: generated,
                    bytes: fs::read(enum_dir.join("_res.yaml")).map_err(|error| {
                        format!("failed to snapshot generated enum transition file: {error}")
                    })?,
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "failed to inspect generated enum transition file {}: {error}",
                    generated.display()
                ))
            }
        }
        let remaining_entries = fs::read_dir(&enum_dir)
            .map_err(|error| {
                format!(
                    "failed to inspect enum transition directory {}: {error}",
                    enum_dir.display()
                )
            })?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name() != "_res.yaml")
            .count();
        if remaining_entries == 0 {
            directories.push(enum_dir);
        }
    }
    Ok((files, directories))
}

fn canonical_root(root: &Path, label: &str) -> Result<PathBuf, String> {
    let metadata = fs::symlink_metadata(root)
        .map_err(|error| format!("failed to inspect {label} root {}: {error}", root.display()))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        return Err(format!(
            "{label} root must be a non-symlink directory: {}",
            root.display()
        ));
    }
    fs::canonicalize(root).map_err(|error| {
        format!(
            "failed to canonicalize {label} root {}: {error}",
            root.display()
        )
    })
}

fn contained_existing_file(root: &Path, relative: &str, label: &str) -> Result<PathBuf, String> {
    let path = inspect_contained_path(root, relative, label, true)?;
    let metadata = fs::symlink_metadata(&path)
        .map_err(|error| format!("failed to inspect {label} {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(format!(
            "{label} must be a contained regular file: {}",
            path.display()
        ));
    }
    Ok(path)
}

fn contained_new_file(root: &Path, relative: &str, label: &str) -> Result<PathBuf, String> {
    inspect_contained_path(root, relative, label, false)
}

fn inspect_contained_path(
    root: &Path,
    relative: &str,
    label: &str,
    require_leaf: bool,
) -> Result<PathBuf, String> {
    let mut current = root.to_path_buf();
    let parts = Path::new(relative).components().collect::<Vec<_>>();
    for (index, component) in parts.iter().enumerate() {
        let Component::Normal(part) = component else {
            return Err(format!("{label} uses an invalid fixed path"));
        };
        current.push(part);
        let is_leaf = index + 1 == parts.len();
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || (!is_leaf && !metadata.file_type().is_dir())
                {
                    return Err(format!(
                        "{label} contains a symlink or non-directory parent: {}",
                        current.display()
                    ));
                }
            }
            Err(error)
                if !require_leaf && is_leaf && error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "failed to inspect {label} {}: {error}",
                    current.display()
                ))
            }
        }
    }
    let parent = current
        .parent()
        .ok_or_else(|| format!("{label} has no parent"))?;
    let canonical_parent = fs::canonicalize(parent).map_err(|error| {
        format!(
            "failed to canonicalize {label} parent {}: {error}",
            parent.display()
        )
    })?;
    if !canonical_parent.starts_with(root) {
        return Err(format!(
            "{label} escapes owning root: {}",
            current.display()
        ));
    }
    Ok(current)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("atomic destination has no parent: {}", path.display()))?;
    let temporary = parent.join(format!(
        ".{}-{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("appfw"),
        std::process::id()
    ));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| {
            format!(
                "failed to create atomic temporary {}: {error}",
                temporary.display()
            )
        })?;
    let write_result = file.write_all(bytes).and_then(|_| file.sync_all());
    drop(file);
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary);
        return Err(format!(
            "failed to write atomic temporary {}: {error}",
            temporary.display()
        ));
    }
    fs::rename(&temporary, path)
        .map_err(|error| format!("failed to atomically replace {}: {error}", path.display()))
}

fn compensate_package_transition(transition: &PackageWorkItemTransition) -> Result<(), String> {
    atomic_write(&transition.target_path, &transition.original_bytes)?;
    for directory in &transition.empty_enum_dirs {
        if !directory.exists() {
            fs::create_dir(directory).map_err(|error| {
                format!(
                    "failed to restore enum directory {}: {error}",
                    directory.display()
                )
            })?;
        }
    }
    for file in &transition.generated_files {
        atomic_write(&file.path, &file.bytes)?;
    }
    match &transition.original_completed_state {
        Some(bytes) => atomic_write(&transition.completed_path, bytes)?,
        None if transition.completed_path.exists() => fs::remove_file(&transition.completed_path)
            .map_err(|error| {
            format!(
                "failed to remove incomplete lifecycle state {}: {error}",
                transition.completed_path.display()
            )
        })?,
        None => {}
    }
    fs::remove_file(&transition.pending_path).map_err(|error| {
        format!(
            "failed to remove compensated lifecycle journal {}: {error}",
            transition.pending_path.display()
        )
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn relative_to(root: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(root)
        .map_err(|_| format!("journal path escapes consumer root: {}", path.display()))?
        .to_str()
        .map(str::to_string)
        .ok_or_else(|| "journal path is not valid UTF-8".to_string())
}

fn decode_hex(value: &str, label: &str) -> Result<Vec<u8>, String> {
    if value.len() % 2 != 0 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "{label} must be lowercase-or-uppercase even-length hex"
        ));
    }
    (0..value.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&value[index..index + 2], 16)
                .map_err(|_| format!("{label} contains invalid hex"))
        })
        .collect()
}

fn fixed_generated_path(relative: &str) -> bool {
    let parts = Path::new(relative).components().collect::<Vec<_>>();
    matches!(parts.as_slice(), [Component::Normal(a), Component::Normal(b), Component::Normal(c), Component::Normal(_), Component::Normal(d), Component::Normal(e)]
        if *a == ".appfw" && *b == "model" && *c == "schemas" && *d == "gql_enum_types" && *e == "_res.yaml")
}

fn fixed_generated_dir(relative: &str) -> bool {
    let parts = Path::new(relative).components().collect::<Vec<_>>();
    matches!(parts.as_slice(), [Component::Normal(a), Component::Normal(b), Component::Normal(c), Component::Normal(_), Component::Normal(d)]
        if *a == ".appfw" && *b == "model" && *c == "schemas" && *d == "gql_enum_types")
}

fn recover_package_transition(consumer_root: &Path) -> Result<(), String> {
    let consumer_root = canonical_root(consumer_root, "consumer")?;
    let pending_path = contained_existing_file(
        &consumer_root,
        PENDING_UPGRADE_JOURNAL,
        "pending lifecycle journal",
    )?;
    let value: serde_json::Value = serde_json::from_slice(
        &fs::read(&pending_path)
            .map_err(|error| format!("failed to read pending lifecycle journal: {error}"))?,
    )
    .map_err(|error| format!("pending lifecycle journal is malformed: {error}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| "pending lifecycle journal must be an object".to_string())?;
    let expected = [
        "schema",
        "source",
        "target",
        "original_model_hex",
        "generated_files",
        "generated_dirs",
        "completed_state",
    ];
    if object.len() != expected.len() || expected.iter().any(|key| !object.contains_key(*key)) {
        return Err("pending lifecycle journal has unexpected or missing fields".to_string());
    }
    if object["schema"] != "appfw_package_first_pending@2"
        || object["source"] != PACKAGE_WORK_ITEM_SOURCE
        || object["target"] != CONSUMER_WORK_ITEM_TARGET
    {
        return Err("pending lifecycle journal uses unsupported fixed schema or paths".to_string());
    }
    let model = decode_hex(
        object["original_model_hex"]
            .as_str()
            .ok_or_else(|| "pending lifecycle journal model bytes are invalid".to_string())?,
        "pending lifecycle journal model bytes",
    )?;
    let state = match &object["completed_state"] {
        serde_json::Value::Null => None,
        serde_json::Value::String(value) => Some(decode_hex(
            value,
            "pending lifecycle journal completed state",
        )?),
        _ => return Err("pending lifecycle journal completed state is invalid".to_string()),
    };
    let mut seen = std::collections::BTreeSet::new();
    let mut files = Vec::new();
    for entry in object["generated_files"]
        .as_array()
        .ok_or_else(|| "pending lifecycle journal generated files are invalid".to_string())?
    {
        let entry = entry
            .as_object()
            .ok_or_else(|| "pending lifecycle journal generated file is invalid".to_string())?;
        if entry.len() != 2 {
            return Err(
                "pending lifecycle journal generated file has unexpected fields".to_string(),
            );
        }
        let path = entry
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                "pending lifecycle journal generated file path is invalid".to_string()
            })?;
        let bytes = entry
            .get("bytes_hex")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                "pending lifecycle journal generated file bytes are invalid".to_string()
            })?;
        if !fixed_generated_path(path) || !seen.insert(path.to_string()) {
            return Err("pending lifecycle journal generated file path is outside the fixed topology or duplicated".to_string());
        }
        files.push((
            path.to_string(),
            decode_hex(bytes, "pending lifecycle journal generated file bytes")?,
        ));
    }
    let mut dirs = Vec::new();
    for entry in object["generated_dirs"]
        .as_array()
        .ok_or_else(|| "pending lifecycle journal generated dirs are invalid".to_string())?
    {
        let path = entry
            .as_str()
            .ok_or_else(|| "pending lifecycle journal generated dir is invalid".to_string())?;
        if !fixed_generated_dir(path) || !seen.insert(format!("dir:{path}")) {
            return Err("pending lifecycle journal generated dir path is outside the fixed topology or duplicated".to_string());
        }
        dirs.push(path.to_string());
    }
    // Build the full immutable recovery plan before the first filesystem mutation.
    let model_path = contained_existing_file(
        &consumer_root,
        CONSUMER_WORK_ITEM_TARGET,
        "consumer WorkItem recovery target",
    )?;
    let completed_path = contained_new_file(
        &consumer_root,
        COMPLETED_UPGRADE_STATE,
        "completed lifecycle state",
    )?;
    let planned_dirs = dirs
        .iter()
        .map(|directory| {
            let path = inspect_contained_path(
                &consumer_root,
                directory,
                "generated recovery directory",
                false,
            )?;
            match fs::symlink_metadata(&path) {
                Ok(metadata)
                    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() =>
                {
                    Err(format!(
                        "generated recovery directory has invalid leaf type: {}",
                        path.display()
                    ))
                }
                Ok(_) | Err(_) => Ok(path),
            }
        })
        .collect::<Result<Vec<_>, String>>()?;
    let planned_files = files.iter().map(|(relative, bytes)| {
        let parent = relative.rsplit_once('/').map(|(parent, _)| parent).ok_or_else(|| "generated recovery file has no parent".to_string())?;
        if !dirs.iter().any(|directory| directory == parent) {
            if !fixed_generated_dir(parent) { return Err("generated recovery file has no journal-authorized directory".to_string()); }
            let preserved = inspect_contained_path(&consumer_root, parent, "preserved source-enum directory", true)?;
            let metadata = fs::symlink_metadata(&preserved).map_err(|error| format!("failed to inspect preserved source-enum directory {}: {error}", preserved.display()))?;
            if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() { return Err("preserved source-enum directory must be an existing contained non-symlink directory".to_string()); }
        }
        let path = consumer_root.join(relative);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.file_type().is_file() => Err(format!("generated recovery file has invalid leaf type: {}", path.display())),
            Ok(_) => Ok((path, bytes.clone())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok((path, bytes.clone())),
            Err(error) => Err(format!("failed to inspect generated recovery file {}: {error}", path.display())),
        }
    }).collect::<Result<Vec<_>, String>>()?;
    match fs::symlink_metadata(&completed_path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.file_type().is_file() => {
            return Err("completed lifecycle state has invalid leaf type".to_string())
        }
        Ok(_) | Err(_) => {}
    }
    for directory in &planned_dirs {
        if !directory.exists() {
            fs::create_dir(directory).map_err(|error| {
                format!(
                    "failed to restore generated recovery directory {}: {error}",
                    directory.display()
                )
            })?;
        }
    }
    atomic_write(&model_path, &model)?;
    for (path, bytes) in &planned_files {
        atomic_write(path, bytes)?;
    }
    match &state {
        Some(bytes) => atomic_write(&completed_path, bytes)?,
        None if completed_path.exists() => fs::remove_file(&completed_path)
            .map_err(|error| format!("failed to remove incomplete lifecycle state: {error}"))?,
        None => {}
    }
    if fs::read(&model_path)
        .map_err(|error| format!("failed to verify recovered model: {error}"))?
        != model
    {
        return Err("recovered model bytes do not match journal".to_string());
    }
    for (path, bytes) in &planned_files {
        if fs::read(path).map_err(|error| {
            format!(
                "failed to verify generated recovery file {}: {error}",
                path.display()
            )
        })? != *bytes
        {
            return Err("recovered generated bytes do not match journal".to_string());
        }
    }
    for directory in &planned_dirs {
        if !fs::symlink_metadata(directory)
            .map_err(|error| {
                format!(
                    "failed to verify generated recovery directory {}: {error}",
                    directory.display()
                )
            })?
            .file_type()
            .is_dir()
        {
            return Err(
                "recovered generated directory topology does not match journal".to_string(),
            );
        }
    }
    match &state {
        Some(bytes)
            if fs::read(&completed_path).map_err(|error| {
                format!("failed to verify completed lifecycle state: {error}")
            })? != *bytes =>
        {
            return Err(
                "recovered completed lifecycle state bytes do not match journal".to_string(),
            )
        }
        None if completed_path.exists() => {
            return Err("recovered completed lifecycle state should be absent".to_string())
        }
        _ => {}
    }
    fs::remove_file(&pending_path)
        .map_err(|error| format!("failed to remove recovered pending lifecycle journal: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static FIXTURE_NONCE: AtomicUsize = AtomicUsize::new(0);

    fn package_fixture() -> PathBuf {
        let nonce = FIXTURE_NONCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "appfw-package-first-test-{}-{nonce}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("bin")).expect("create bin");
        std::fs::create_dir_all(root.join("scripts")).expect("create scripts");
        std::fs::create_dir_all(root.join("app_gen/_templates")).expect("create templates");
        std::fs::create_dir_all(root.join("docs/start")).expect("create delivery policy directory");
        std::fs::write(
            root.join("app-framework-package.json"),
            r#"{"version":"0.1.1"}"#,
        )
        .expect("write manifest");
        std::fs::write(root.join("scripts/appfw"), "#!/usr/bin/env bash\n").expect("write wrapper");
        std::fs::write(
            root.join("docs/start/delivery-profiles.json"),
            "{\"schema_version\":1}\n",
        )
        .expect("write delivery policy");
        let work_item = root.join(PACKAGE_WORK_ITEM_SOURCE);
        std::fs::create_dir_all(work_item.parent().expect("WorkItem parent"))
            .expect("create package WorkItem parent");
        std::fs::write(
            &work_item,
            "name: WorkItem\nproperties:\n  - name: is_complete\n",
        )
        .expect("write package WorkItem model");
        for helper in REQUIRED_HELPERS {
            std::fs::write(root.join("bin").join(helper), "").expect("write helper");
        }
        root
    }

    #[test]
    fn package_first_contract_requires_extracted_core_helpers() {
        let root = package_fixture();
        let contract =
            inspect_package(&root, "second-consumer".to_string()).expect("inspect package");

        assert_eq!(contract.version_identity, "0.1.1");
        assert_eq!(contract.helpers, REQUIRED_HELPERS);
        assert!(contract.no_checkout);
        assert!(contract.deterministic_generation);
        assert!(contract.compatibility.contains("package-version-only"));
        assert_eq!(contract.lifecycle.len(), 6);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn package_first_contract_rejects_checkout_residue() {
        let root = package_fixture();
        std::fs::create_dir(root.join(".git")).expect("create git residue");

        let error =
            inspect_package(&root, "second-consumer".to_string()).expect_err("reject checkout");

        assert!(error.contains("without a checkout"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn package_first_contract_rejects_wrong_required_node_types() {
        let cases = [
            ("scripts/appfw", true, "regular file"),
            ("app_gen/_templates", false, "directory"),
            ("docs/start/delivery-profiles.json", true, "regular file"),
        ];

        for (relative, replace_with_directory, expected_kind) in cases {
            let root = package_fixture();
            let path = root.join(relative);
            if replace_with_directory {
                std::fs::remove_file(&path).expect("remove required file");
                std::fs::create_dir(&path).expect("replace required file with directory");
            } else {
                std::fs::remove_dir_all(&path).expect("remove required directory");
                std::fs::write(&path, "not a directory\n")
                    .expect("replace required directory with file");
            }

            let error = inspect_package(&root, "second-consumer".to_string())
                .expect_err("reject wrong package lifecycle input type");
            assert!(error.contains(expected_kind), "{error}");
            let _ = std::fs::remove_dir_all(root);
        }
    }

    #[test]
    fn upgrade_retains_disposable_consumer_version_transition() {
        let from = package_fixture();
        let to = package_fixture();
        std::fs::write(
            to.join("app-framework-package.json"),
            r#"{"version":"0.1.2"}"#,
        )
        .expect("write target identity");
        let consumer = std::env::temp_dir().join(format!(
            "appfw-upgrade-consumer-{}",
            FIXTURE_NONCE.fetch_add(1, Ordering::Relaxed)
        ));
        let enum_dir = consumer.join(".appfw/model/schemas/consumer/gql_enum_types");
        std::fs::create_dir_all(&enum_dir).expect("create generated enum directory");
        std::fs::write(enum_dir.join("_res.yaml"), "items:\n").expect("write generated enum file");
        let target = consumer.join(CONSUMER_WORK_ITEM_TARGET);
        std::fs::create_dir_all(target.parent().expect("consumer WorkItem parent"))
            .expect("create consumer WorkItem parent");
        std::fs::write(&target, "name: WorkItem\n").expect("write consumer WorkItem model");

        let upgrade = upgrade_package(&from, &to, &consumer).expect("upgrade package");

        assert_eq!(upgrade.from_version, "0.1.1");
        assert_eq!(upgrade.to_version, "0.1.2");
        assert!(consumer.join(".appfw/package-first-upgrade.json").is_file());
        assert!(!enum_dir.exists());
        assert_eq!(
            std::fs::read(&target).expect("read applied WorkItem model"),
            std::fs::read(to.join(PACKAGE_WORK_ITEM_SOURCE)).expect("read package WorkItem model")
        );
        let state: serde_json::Value = serde_json::from_slice(
            &std::fs::read(consumer.join(".appfw/package-first-upgrade.json"))
                .expect("read upgrade state"),
        )
        .expect("parse upgrade state");
        assert_eq!(
            state["model_transition"]["source"],
            PACKAGE_WORK_ITEM_SOURCE
        );
        assert_eq!(
            state["model_transition"]["target"],
            CONSUMER_WORK_ITEM_TARGET
        );
        assert_eq!(
            state["model_transition"]["applied_sha256"],
            sha256_hex(&std::fs::read(&target).expect("read applied WorkItem model"))
        );
        let _ = std::fs::remove_dir_all(from);
        let _ = std::fs::remove_dir_all(to);
        let _ = std::fs::remove_dir_all(consumer);
    }

    #[test]
    fn upgrade_preserves_source_enum_yaml_during_transition() {
        let from = package_fixture();
        let to = package_fixture();
        std::fs::write(
            to.join("app-framework-package.json"),
            r#"{"version":"0.1.2"}"#,
        )
        .expect("write target identity");
        let consumer = std::env::temp_dir().join(format!(
            "appfw-upgrade-enum-consumer-{}",
            FIXTURE_NONCE.fetch_add(1, Ordering::Relaxed)
        ));
        let enum_dir = consumer.join(".appfw/model/schemas/consumer/gql_enum_types");
        std::fs::create_dir_all(&enum_dir).expect("create enum source directory");
        std::fs::write(enum_dir.join("_res.yaml"), "items:\n").expect("write generated enum file");
        std::fs::write(enum_dir.join("status.yaml"), "items:\n  - name: Open\n")
            .expect("write source enum file");
        let target = consumer.join(CONSUMER_WORK_ITEM_TARGET);
        std::fs::create_dir_all(target.parent().expect("consumer WorkItem parent"))
            .expect("create consumer WorkItem parent");
        std::fs::write(&target, "name: WorkItem\n").expect("write consumer WorkItem model");

        upgrade_package(&from, &to, &consumer).expect("upgrade package");

        assert!(!enum_dir.join("_res.yaml").exists());
        assert_eq!(
            std::fs::read_to_string(enum_dir.join("status.yaml")).expect("read source enum file"),
            "items:\n  - name: Open\n"
        );
        assert!(enum_dir.is_dir());
        let _ = std::fs::remove_dir_all(from);
        let _ = std::fs::remove_dir_all(to);
        let _ = std::fs::remove_dir_all(consumer);
    }

    #[test]
    fn upgrade_fails_before_model_write_when_package_work_item_source_is_missing() {
        let from = package_fixture();
        let to = package_fixture();
        std::fs::write(
            to.join("app-framework-package.json"),
            r#"{"version":"0.1.2"}"#,
        )
        .expect("write target identity");
        std::fs::remove_file(to.join(PACKAGE_WORK_ITEM_SOURCE))
            .expect("remove package WorkItem model");
        let consumer = package_fixture();
        let target = consumer.join(CONSUMER_WORK_ITEM_TARGET);
        std::fs::create_dir_all(target.parent().expect("consumer WorkItem parent"))
            .expect("create consumer WorkItem parent");
        std::fs::write(&target, "original consumer model\n")
            .expect("write consumer WorkItem model");

        let error =
            upgrade_package(&from, &to, &consumer).expect_err("reject missing package WorkItem");

        assert!(error.contains("package WorkItem source"));
        assert_eq!(
            std::fs::read_to_string(&target).expect("read consumer WorkItem model"),
            "original consumer model\n"
        );
        let _ = std::fs::remove_dir_all(from);
        let _ = std::fs::remove_dir_all(to);
        let _ = std::fs::remove_dir_all(consumer);
    }

    #[test]
    fn upgrade_fails_before_model_write_when_consumer_work_item_target_is_missing() {
        let from = package_fixture();
        let to = package_fixture();
        std::fs::write(
            to.join("app-framework-package.json"),
            r#"{"version":"0.1.2"}"#,
        )
        .expect("write target identity");
        let consumer = package_fixture();

        let error =
            upgrade_package(&from, &to, &consumer).expect_err("reject missing consumer WorkItem");

        assert!(error.contains("consumer WorkItem upgrade target"));
        assert!(!consumer.join(".appfw/package-first-upgrade.json").exists());
        let _ = std::fs::remove_dir_all(from);
        let _ = std::fs::remove_dir_all(to);
        let _ = std::fs::remove_dir_all(consumer);
    }

    #[test]
    fn package_work_item_byte_verification_rejects_mismatch() {
        let error = verify_package_work_item_bytes(b"package bytes", b"consumer bytes")
            .expect_err("reject mismatched WorkItem bytes");

        assert!(error.contains("does not match the package source after write"));
    }

    #[cfg(unix)]
    #[test]
    fn upgrade_rejects_work_item_symlink_without_touching_external_bytes() {
        use std::os::unix::fs::symlink;

        let from = package_fixture();
        let to = package_fixture();
        fs::write(
            to.join("app-framework-package.json"),
            r#"{"version":"0.1.2"}"#,
        )
        .expect("write target identity");
        let consumer = package_fixture();
        let target = consumer.join(CONSUMER_WORK_ITEM_TARGET);
        fs::create_dir_all(target.parent().expect("target parent")).expect("create target parent");
        let external = std::env::temp_dir().join(format!(
            "appfw-external-work-item-{}",
            FIXTURE_NONCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&external, "external bytes\n").expect("write external target");
        symlink(&external, &target).expect("create escaping symlink");

        let error = upgrade_package(&from, &to, &consumer).expect_err("reject escaping symlink");

        assert!(error.contains("symlink") || error.contains("regular file"));
        assert_eq!(
            fs::read_to_string(&external).expect("read external target"),
            "external bytes\n"
        );
        let _ = fs::remove_file(external);
        let _ = fs::remove_dir_all(from);
        let _ = fs::remove_dir_all(to);
        let _ = fs::remove_dir_all(consumer);
    }

    #[test]
    fn upgrade_fails_before_mutation_when_completed_state_is_a_directory() {
        let from = package_fixture();
        let to = package_fixture();
        fs::write(
            to.join("app-framework-package.json"),
            r#"{"version":"0.1.2"}"#,
        )
        .expect("write target identity");
        let consumer = package_fixture();
        let target = consumer.join(CONSUMER_WORK_ITEM_TARGET);
        fs::create_dir_all(target.parent().expect("target parent")).expect("create target parent");
        fs::write(&target, "original model\n").expect("write original model");
        fs::create_dir_all(consumer.join(COMPLETED_UPGRADE_STATE))
            .expect("create invalid state directory");

        let error = upgrade_package(&from, &to, &consumer).expect_err("reject invalid state path");

        assert!(error.contains("completed lifecycle state is not a regular file"));
        assert_eq!(
            fs::read_to_string(&target).expect("read target"),
            "original model\n"
        );
        let _ = fs::remove_dir_all(from);
        let _ = fs::remove_dir_all(to);
        let _ = fs::remove_dir_all(consumer);
    }

    #[test]
    fn injected_post_model_failure_compensates_model_and_generated_bytes() {
        let from = package_fixture();
        let to = package_fixture();
        fs::write(
            to.join("app-framework-package.json"),
            r#"{"version":"0.1.2"}"#,
        )
        .expect("write target identity");
        let consumer = package_fixture();
        let target = consumer.join(CONSUMER_WORK_ITEM_TARGET);
        fs::create_dir_all(target.parent().expect("target parent")).expect("create target parent");
        fs::write(&target, "original model\n").expect("write original model");
        fs::write(
            consumer.join(COMPLETED_UPGRADE_STATE),
            "prior completed state\n",
        )
        .expect("write prior completed state");
        let generated = consumer.join(".appfw/model/schemas/consumer/gql_enum_types/_res.yaml");
        fs::create_dir_all(generated.parent().expect("generated parent"))
            .expect("create generated parent");
        fs::write(&generated, "generated bytes\n").expect("write generated bytes");
        let transition =
            prepare_package_work_item_transition(&to, &consumer).expect("prepare transition");

        let error = apply_package_work_item_transition(transition, "0.1.1", "0.1.2", true, false)
            .expect_err("inject post-model failure");

        assert!(error.contains("injected failure after model replacement"));
        assert_eq!(
            fs::read_to_string(&target).expect("read target"),
            "original model\n"
        );
        assert_eq!(
            fs::read_to_string(&generated).expect("read generated"),
            "generated bytes\n"
        );
        assert!(!consumer.join(PENDING_UPGRADE_JOURNAL).exists());
        assert_eq!(
            fs::read_to_string(consumer.join(COMPLETED_UPGRADE_STATE))
                .expect("read restored state"),
            "prior completed state\n"
        );
        let _ = fs::remove_dir_all(from);
        let _ = fs::remove_dir_all(to);
        let _ = fs::remove_dir_all(consumer);
    }

    #[test]
    fn durable_compensation_failure_recovers_through_public_command() {
        let from = package_fixture();
        let to = package_fixture();
        fs::write(
            to.join("app-framework-package.json"),
            r#"{"version":"0.1.2"}"#,
        )
        .expect("target identity");
        let consumer = package_fixture();
        let target = consumer.join(CONSUMER_WORK_ITEM_TARGET);
        fs::create_dir_all(target.parent().expect("target parent")).expect("target parent");
        fs::write(&target, "original model\n").expect("target");
        let generated = consumer.join(".appfw/model/schemas/consumer/gql_enum_types/_res.yaml");
        fs::create_dir_all(generated.parent().expect("generated parent"))
            .expect("generated parent");
        fs::write(&generated, "generated bytes\n").expect("generated");
        fs::write(consumer.join(COMPLETED_UPGRADE_STATE), "prior state\n").expect("state");
        let transition = prepare_package_work_item_transition(&to, &consumer).expect("transition");
        let error = apply_package_work_item_transition(transition, "0.1.1", "0.1.2", true, true)
            .expect_err("durable failure");
        assert!(error.contains("unresolved pending journal retained"));
        assert!(consumer.join(PENDING_UPGRADE_JOURNAL).exists());
        run(&[
            "recover".to_string(),
            "--consumer-root".to_string(),
            consumer.display().to_string(),
        ])
        .expect("public recovery");
        assert_eq!(
            fs::read_to_string(&target).expect("target"),
            "original model\n"
        );
        assert_eq!(
            fs::read_to_string(&generated).expect("generated"),
            "generated bytes\n"
        );
        assert_eq!(
            fs::read_to_string(consumer.join(COMPLETED_UPGRADE_STATE)).expect("state"),
            "prior state\n"
        );
        assert!(!consumer.join(PENDING_UPGRADE_JOURNAL).exists());
        upgrade_package(&from, &to, &consumer).expect("explicit retry");
        let _ = fs::remove_dir_all(from);
        let _ = fs::remove_dir_all(to);
        let _ = fs::remove_dir_all(consumer);
    }

    #[test]
    fn durable_compensation_failure_recovers_source_enum_topology_through_public_command() {
        let from = package_fixture();
        let to = package_fixture();
        fs::write(
            to.join("app-framework-package.json"),
            r#"{"version":"0.1.2"}"#,
        )
        .expect("target identity");
        let consumer = package_fixture();
        let target = consumer.join(CONSUMER_WORK_ITEM_TARGET);
        fs::create_dir_all(target.parent().expect("target parent")).expect("target parent");
        fs::write(&target, "original model\n").expect("target");
        let enum_dir = consumer.join(".appfw/model/schemas/consumer/gql_enum_types");
        fs::create_dir_all(&enum_dir).expect("enum parent");
        let generated = enum_dir.join("_res.yaml");
        let source = enum_dir.join("status.yaml");
        fs::write(&generated, "generated bytes\n").expect("generated");
        fs::write(&source, "source enum bytes\n").expect("source enum");
        fs::write(consumer.join(COMPLETED_UPGRADE_STATE), "prior state\n").expect("state");
        let transition = prepare_package_work_item_transition(&to, &consumer).expect("transition");
        assert!(transition.empty_enum_dirs.is_empty());
        let error = apply_package_work_item_transition(transition, "0.1.1", "0.1.2", true, true)
            .expect_err("durable failure");
        assert!(error.contains("unresolved pending journal retained"));
        run(&[
            "recover".to_string(),
            "--consumer-root".to_string(),
            consumer.display().to_string(),
        ])
        .expect("public recovery");
        assert_eq!(
            fs::read_to_string(&target).expect("target"),
            "original model\n"
        );
        assert_eq!(
            fs::read_to_string(&generated).expect("generated"),
            "generated bytes\n"
        );
        assert_eq!(
            fs::read_to_string(&source).expect("source"),
            "source enum bytes\n"
        );
        assert_eq!(
            fs::read_to_string(consumer.join(COMPLETED_UPGRADE_STATE)).expect("state"),
            "prior state\n"
        );
        assert!(!consumer.join(PENDING_UPGRADE_JOURNAL).exists());
        upgrade_package(&from, &to, &consumer).expect("retry");
        let _ = fs::remove_dir_all(from);
        let _ = fs::remove_dir_all(to);
        let _ = fs::remove_dir_all(consumer);
    }

    #[cfg(unix)]
    #[test]
    fn upgrade_rejects_symlinked_consumer_root_and_parent() {
        use std::os::unix::fs::symlink;

        let from = package_fixture();
        let to = package_fixture();
        fs::write(
            to.join("app-framework-package.json"),
            r#"{"version":"0.1.2"}"#,
        )
        .expect("write target identity");
        let actual = package_fixture();
        let target = actual.join(CONSUMER_WORK_ITEM_TARGET);
        fs::create_dir_all(target.parent().expect("target parent")).expect("create target parent");
        fs::write(&target, "original model\n").expect("write target");
        let root_link = std::env::temp_dir().join(format!(
            "appfw-consumer-root-link-{}",
            FIXTURE_NONCE.fetch_add(1, Ordering::Relaxed)
        ));
        symlink(&actual, &root_link).expect("create consumer root link");
        let root_error = upgrade_package(&from, &to, &root_link).expect_err("reject root link");
        assert!(root_error.contains("consumer root must be a non-symlink directory"));
        fs::remove_file(&root_link).expect("remove root link");

        let parent = actual.join(".appfw/model/schemas/consumer/entity_types");
        fs::remove_dir_all(&parent).expect("remove actual parent");
        let external_parent = std::env::temp_dir().join(format!(
            "appfw-external-parent-{}",
            FIXTURE_NONCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&external_parent).expect("create external parent");
        symlink(&external_parent, &parent).expect("create parent link");
        let parent_error = upgrade_package(&from, &to, &actual).expect_err("reject parent link");
        assert!(parent_error.contains("symlink"));
        assert!(!external_parent.join("work_item.yaml").exists());
        let _ = fs::remove_dir_all(from);
        let _ = fs::remove_dir_all(to);
        let _ = fs::remove_dir_all(actual);
        let _ = fs::remove_dir_all(external_parent);
    }

    #[test]
    fn unresolved_pending_journal_fails_closed_then_allows_a_resolved_retry() {
        let from = package_fixture();
        let to = package_fixture();
        fs::write(
            to.join("app-framework-package.json"),
            r#"{"version":"0.1.2"}"#,
        )
        .expect("write target identity");
        let consumer = package_fixture();
        let target = consumer.join(CONSUMER_WORK_ITEM_TARGET);
        fs::create_dir_all(target.parent().expect("target parent")).expect("create target parent");
        fs::write(&target, "original model\n").expect("write original model");
        let pending = consumer.join(PENDING_UPGRADE_JOURNAL);
        fs::write(
            &pending,
            serde_json::to_vec(&serde_json::json!({
                "schema": "appfw_package_first_pending@2",
                "source": PACKAGE_WORK_ITEM_SOURCE,
                "target": CONSUMER_WORK_ITEM_TARGET,
                "original_model_hex": hex(b"original model\n"),
                "generated_files": [],
                "generated_dirs": [],
                "completed_state": null,
            }))
            .expect("serialize pending journal"),
        )
        .expect("write pending journal");

        let error =
            upgrade_package(&from, &to, &consumer).expect_err("fail closed on pending journal");
        assert!(error.contains("unresolved package upgrade pending journal"));
        assert_eq!(
            fs::read_to_string(&target).expect("read target"),
            "original model\n"
        );

        recover_package_transition(&consumer)
            .expect("bounded lifecycle recovery resolves retained journal");
        assert!(!pending.exists());
        upgrade_package(&from, &to, &consumer).expect("retry after resolution");
        assert_eq!(
            fs::read(&target).expect("read retry target"),
            fs::read(to.join(PACKAGE_WORK_ITEM_SOURCE)).expect("read package source")
        );
        let _ = fs::remove_dir_all(from);
        let _ = fs::remove_dir_all(to);
        let _ = fs::remove_dir_all(consumer);
    }

    #[test]
    fn matching_versions_do_not_claim_archive_hash_equivalence() {
        let first = package_fixture();
        let second = package_fixture();
        fs::write(
            second.join("README.md"),
            "same version, distinct package content\n",
        )
        .expect("write distinct package content");

        let first_contract =
            inspect_package(&first, "first".to_string()).expect("inspect first package");
        let second_contract =
            inspect_package(&second, "second".to_string()).expect("inspect second package");

        assert_eq!(
            first_contract.version_identity,
            second_contract.version_identity
        );
        assert!(first_contract
            .compatibility
            .contains("package-version-only"));
        assert!(first_contract.compatibility.contains("SHA-256"));
        assert_ne!(
            sha256_hex(
                &fs::read(first.join("app-framework-package.json")).expect("read first identity")
            ),
            sha256_hex(&fs::read(second.join("README.md")).expect("read distinct content"))
        );
        let _ = fs::remove_dir_all(first);
        let _ = fs::remove_dir_all(second);
    }
}
