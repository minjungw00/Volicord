use crate::{cli::usage, cli::Cursor, Error, RuntimeLayout};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    env,
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
};
use toml_edit::{value, ArrayOfTables, DocumentMut, InlineTable, Item, Table};

const MANIFEST_KIND: &str = "volicord_codex_repository_integration";
const MANIFEST_NAME: &str = "volicord-integration.json";
const CONFIG_NAME: &str = "config.toml";
const SESSION_MATCHER: &str = "^(startup|resume|clear|compact)$";
const EXCLUDE_BEGIN: &str = "# BEGIN Volicord Codex integration";
const EXCLUDE_END: &str = "# END Volicord Codex integration";
const ACTIVATION_CONTEXT: &str = "Volicord is active because this repository was explicitly authorized. For every fresh project-scoped session, STOP before repository inspection, edits, or continuation: resolve the current repository first. If found, successfully Recall before inspecting, editing, or continuing work. If not found, explicitly initialize, record the current-host Goal, and establish a repository baseline. After that baseline and before the first ordinary repository write, screen every unresolved choice relevant to the requested outcome into exactly one category: repository/environment fact--resolve through research, not a user Question; accepted repository/product contract--apply it and do not reopen it to manufacture a Question; delegated implementation choice--the agent may choose within the active contract; implementation choices explicitly delegated by active architecture/product contracts, including renderer/layout/detail choices, are not user Questions; or material user-owned outcome--STOP before implementing that outcome and use the existing Question and Decision path. Strong material signals include user-visible default behavior, CLI/API compatibility behavior, externally observable error or failure policy, privacy/security posture, maintenance/support policy, and any outcome where repository research leaves multiple viable policies that materially change what the user or downstream automation experiences. Public invalid-input behavior and batch-failure continuation policy are material observable outcomes when research leaves multiple viable policies. A library default, conventional behavior, implementation simplicity, or agent recommendation does not authorize selecting a material user-owned outcome. For such an outcome, submit a Question Candidate, attach source-grounded repository research, review materiality, explicitly promote it, read the resulting inquiry frontier, present its actual alternatives, recommendation, and trade-offs, obtain an explicit current-host user response, then record and apply the Decision. Repository/environment facts remain research and must not be asked of the user. An agent recommendation is never a user Decision. Once applicable Decisions and contracts resolve the material outcome, ordinary code edits require no new approval ceremony. Record passed or failed Checkpoint verification only from the same actually observed command execution with a numeric exit status; output-only text is insufficient. Incidental inspection commands need not become Checkpoint verification facts. Meaningful completed or paused work uses a grounded Checkpoint. Non-project requests and unrelated greetings require no Volicord ceremony.";

#[derive(Clone, Debug, Deserialize, Serialize)]
struct OwnershipManifest {
    kind: String,
    schema_version: u32,
    repository: PathBuf,
    runtime: PathBuf,
    volicord: PathBuf,
    volicord_mcp: PathBuf,
    config_created: bool,
    excluded_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionStartInput {
    hook_event_name: String,
    session_id: String,
    cwd: PathBuf,
    source: String,
    model: String,
    permission_mode: String,
    transcript_path: Option<String>,
}

pub(crate) fn execute(
    runtime: RuntimeLayout,
    cursor: &mut Cursor,
    input: &mut dyn Read,
) -> Result<Option<Value>, Error> {
    match cursor.next("codex command")?.as_str() {
        "enable" => {
            let repository = PathBuf::from(cursor.next("absolute repository path")?);
            let executable = env::current_exe().map_err(|error| {
                Error::with_source("cannot locate the installed volicord executable", error)
            })?;
            enable(&repository, runtime.root(), &executable).map(Some)
        }
        "disable" => {
            let repository = PathBuf::from(cursor.next("absolute repository path")?);
            disable(&repository).map(Some)
        }
        "hook" => {
            let repository = PathBuf::from(cursor.next("authorized repository path")?);
            session_start(&repository, input)
        }
        _ => Err(usage(
            "codex requires enable ABSOLUTE_REPOSITORY or disable ABSOLUTE_REPOSITORY",
        )),
    }
}

fn enable(repository: &Path, runtime: &Path, executable: &Path) -> Result<Value, Error> {
    let repository = canonical_repository(repository)?;
    let runtime = absolute_runtime(runtime)?;
    let volicord = canonical_executable(executable, "volicord")?;
    let sibling = volicord
        .parent()
        .ok_or_else(|| Error::new("installed volicord executable has no parent directory"))?
        .join("volicord-mcp");
    let volicord_mcp = canonical_executable(&sibling, "volicord-mcp sibling")?;
    let codex_dir = repository.join(".codex");
    let config_path = codex_dir.join(CONFIG_NAME);
    let hooks_path = codex_dir.join("hooks.json");
    let manifest_path = codex_dir.join(MANIFEST_NAME);
    reject_symlink(&codex_dir, "repository .codex directory")?;
    reject_symlink(&config_path, "repository Codex config")?;
    reject_symlink(&hooks_path, "repository Codex hooks file")?;
    reject_symlink(&manifest_path, "Volicord Codex ownership manifest")?;

    let git = GitRepository::inspect(&repository)?;
    if let Some(git) = &git {
        for path in [
            format!(".codex/{CONFIG_NAME}"),
            ".codex/hooks.json".into(),
            format!(".codex/{MANIFEST_NAME}"),
        ] {
            if git.is_tracked(&path)? {
                return Err(conflict(format!("{path} is tracked by the repository")));
            }
        }
    }

    let existing_manifest = read_manifest(&manifest_path)?;
    let config_existed = config_path.exists();
    let config_source = if config_existed {
        fs::read_to_string(&config_path)
            .map_err(|error| Error::with_source("cannot read repository Codex config", error))?
    } else {
        String::new()
    };
    let mut document = parse_config(&config_source)?;
    if hooks_are_disabled(&document) {
        return Err(conflict(
            "repository Codex config explicitly disables lifecycle hooks",
        ));
    }

    if let Some(previous) = existing_manifest.as_ref() {
        validate_manifest(previous, &repository)?;
        ensure_owned_state(&document, previous)?;
        remove_owned_state(&mut document, previous)?;
    } else if has_volicord_state(&document) {
        return Err(conflict(
            "repository Codex config already contains an unowned Volicord MCP entry or SessionStart hook",
        ));
    }

    let config_created = existing_manifest
        .as_ref()
        .map_or(!config_existed, |previous| previous.config_created);
    let hook_command = hook_command(&volicord, &runtime, &repository);
    insert_owned_state(&mut document, &volicord_mcp, &runtime, &hook_command)?;

    let excluded_paths = if git.is_some() {
        let mut paths = vec![format!("/.codex/{MANIFEST_NAME}")];
        if config_created {
            paths.insert(0, format!("/.codex/{CONFIG_NAME}"));
        }
        paths
    } else {
        Vec::new()
    };
    let manifest = OwnershipManifest {
        kind: MANIFEST_KIND.into(),
        schema_version: 1,
        repository: repository.clone(),
        runtime: runtime.clone(),
        volicord: volicord.clone(),
        volicord_mcp: volicord_mcp.clone(),
        config_created,
        excluded_paths,
    };
    let exclusion_update = git
        .as_ref()
        .map(|git| git.updated_exclusion(existing_manifest.as_ref(), &manifest))
        .transpose()?;

    fs::create_dir_all(&codex_dir)
        .map_err(|error| Error::with_source("cannot create repository .codex directory", error))?;
    atomic_write(&config_path, document.to_string().as_bytes())?;
    if let (Some(git), Some(updated)) = (&git, exclusion_update) {
        git.write_exclusion(&updated)?;
    }
    let mut encoded = serde_json::to_vec_pretty(&manifest)
        .map_err(|error| Error::with_source("cannot render Codex ownership manifest", error))?;
    encoded.push(b'\n');
    atomic_write(&manifest_path, &encoded)?;

    Ok(json!({
        "operation":"codex_enable",
        "repository":repository,
        "config":config_path,
        "mcp_server":"volicord",
        "mcp_executable":volicord_mcp,
        "runtime":runtime,
        "session_start_matcher":SESSION_MATCHER,
        "project_trust":"user_controlled",
    }))
}

fn disable(repository: &Path) -> Result<Value, Error> {
    let repository = canonical_repository(repository)?;
    let codex_dir = repository.join(".codex");
    let config_path = codex_dir.join(CONFIG_NAME);
    let manifest_path = codex_dir.join(MANIFEST_NAME);
    reject_symlink(&codex_dir, "repository .codex directory")?;
    reject_symlink(&config_path, "repository Codex config")?;
    reject_symlink(&manifest_path, "Volicord Codex ownership manifest")?;
    let git = GitRepository::inspect(&repository)?;

    let Some(manifest) = read_manifest(&manifest_path)? else {
        if config_path.exists() {
            let source = fs::read_to_string(&config_path).map_err(|error| {
                Error::with_source("cannot read repository Codex config", error)
            })?;
            if has_volicord_state(&parse_config(&source)?) {
                return Err(conflict(
                    "repository Codex config contains Volicord state without an ownership manifest",
                ));
            }
        }
        return Ok(json!({"operation":"codex_disable","repository":repository,"changed":false}));
    };
    validate_manifest(&manifest, &repository)?;
    if let Some(git) = &git {
        for path in [
            format!(".codex/{CONFIG_NAME}"),
            format!(".codex/{MANIFEST_NAME}"),
        ] {
            if git.is_tracked(&path)? {
                return Err(conflict(format!(
                    "{path} became tracked after Volicord enabled it"
                )));
            }
        }
    }
    let source = fs::read_to_string(&config_path).map_err(|error| {
        Error::with_source("owned repository Codex config is unavailable", error)
    })?;
    let mut document = parse_config(&source)?;
    ensure_owned_state(&document, &manifest)?;
    remove_owned_state(&mut document, &manifest)?;
    let disabled_manifest = OwnershipManifest {
        excluded_paths: Vec::new(),
        ..manifest.clone()
    };
    let exclusion_update = git
        .as_ref()
        .map(|git| git.updated_exclusion(Some(&manifest), &disabled_manifest))
        .transpose()?;

    if manifest.config_created && document.iter().next().is_none() {
        fs::remove_file(&config_path)
            .map_err(|error| Error::with_source("cannot remove empty owned Codex config", error))?;
    } else {
        atomic_write(&config_path, document.to_string().as_bytes())?;
    }
    if let (Some(git), Some(updated)) = (&git, exclusion_update) {
        git.write_exclusion(&updated)?;
    }
    fs::remove_file(&manifest_path)
        .map_err(|error| Error::with_source("cannot remove Codex ownership manifest", error))?;
    if manifest.config_created {
        let _ = fs::remove_dir(&codex_dir);
    }
    Ok(json!({"operation":"codex_disable","repository":repository,"changed":true}))
}

fn session_start(repository: &Path, input: &mut dyn Read) -> Result<Option<Value>, Error> {
    if !repository.is_absolute() {
        return Err(Error::new("authorized repository path must be absolute"));
    }
    let event: SessionStartInput = serde_json::from_reader(input)
        .map_err(|error| Error::with_source("cannot parse Codex SessionStart input", error))?;
    if event.hook_event_name != "SessionStart"
        || !matches!(
            event.source.as_str(),
            "startup" | "resume" | "clear" | "compact"
        )
    {
        return Err(Error::new("unexpected Codex SessionStart event"));
    }
    let _official_fields = (
        &event.session_id,
        &event.model,
        &event.permission_mode,
        &event.transcript_path,
    );
    let Ok(cwd) = fs::canonicalize(&event.cwd) else {
        return Ok(None);
    };
    if cwd != repository && !cwd.starts_with(repository) {
        return Ok(None);
    }
    Ok(Some(json!({
        "hookSpecificOutput": {
            "hookEventName": "SessionStart",
            "additionalContext": ACTIVATION_CONTEXT,
        }
    })))
}

fn canonical_repository(repository: &Path) -> Result<PathBuf, Error> {
    if !repository.is_absolute() {
        return Err(Error::new("repository path must be absolute"));
    }
    let canonical = fs::canonicalize(repository)
        .map_err(|error| Error::with_source("cannot canonicalize repository path", error))?;
    if !canonical.is_dir() {
        return Err(Error::new("repository path must identify a directory"));
    }
    Ok(canonical)
}

fn absolute_runtime(runtime: &Path) -> Result<PathBuf, Error> {
    if !runtime.is_absolute() {
        return Err(Error::new("runtime root must be absolute"));
    }
    Ok(runtime.to_path_buf())
}

fn canonical_executable(path: &Path, label: &str) -> Result<PathBuf, Error> {
    let canonical = fs::canonicalize(path)
        .map_err(|error| Error::with_source(format!("cannot locate installed {label}"), error))?;
    let metadata = fs::metadata(&canonical)
        .map_err(|error| Error::with_source(format!("cannot inspect installed {label}"), error))?;
    if !metadata.is_file() {
        return Err(Error::new(format!("installed {label} is not a file")));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(Error::new(format!("installed {label} is not executable")));
        }
    }
    Ok(canonical)
}

fn hook_command(volicord: &Path, runtime: &Path, repository: &Path) -> String {
    format!(
        "{} --runtime {} codex hook {}",
        shell_quote(volicord),
        shell_quote(runtime),
        shell_quote(repository)
    )
}

fn shell_quote(path: &Path) -> String {
    let value = path.to_string_lossy();
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn parse_config(source: &str) -> Result<DocumentMut, Error> {
    source
        .parse::<DocumentMut>()
        .map_err(|error| Error::with_source("repository Codex config is not valid TOML", error))
}

fn insert_owned_state(
    document: &mut DocumentMut,
    mcp: &Path,
    runtime: &Path,
    command: &str,
) -> Result<(), Error> {
    let servers = table_mut(document, "mcp_servers")?;
    if servers.contains_key("volicord") {
        return Err(conflict("Volicord MCP table already exists"));
    }
    let mut server = Table::new();
    server["command"] = value(mcp.to_string_lossy().to_string());
    server["enabled"] = value(true);
    server["required"] = value(true);
    let mut environment = InlineTable::new();
    environment.insert(
        "VOLICORD_RUNTIME_DIR",
        toml_edit::Value::from(runtime.to_string_lossy().to_string()),
    );
    server["env"] = Item::Value(toml_edit::Value::InlineTable(environment));
    servers["volicord"] = Item::Table(server);

    let hooks = table_mut(document, "hooks")?;
    let groups = hooks
        .entry("SessionStart")
        .or_insert(Item::ArrayOfTables(ArrayOfTables::new()))
        .as_array_of_tables_mut()
        .ok_or_else(|| conflict("hooks.SessionStart is not an array of matcher tables"))?;
    let mut group = Table::new();
    group["matcher"] = value(SESSION_MATCHER);
    let mut handlers = ArrayOfTables::new();
    let mut handler = Table::new();
    handler["type"] = value("command");
    handler["command"] = value(command);
    handler["timeout"] = value(5);
    handler["statusMessage"] = value("Activating Volicord repository context");
    handler["additionalContextLimit"] = value(2000);
    handlers.push(handler);
    group["hooks"] = Item::ArrayOfTables(handlers);
    groups.push(group);
    Ok(())
}

fn ensure_owned_state(document: &DocumentMut, manifest: &OwnershipManifest) -> Result<(), Error> {
    let command = hook_command(&manifest.volicord, &manifest.runtime, &manifest.repository);
    if !mcp_matches(document, &manifest.volicord_mcp, &manifest.runtime) {
        return Err(conflict(
            "owned Volicord MCP configuration was changed or removed",
        ));
    }
    if find_hook(document, &command)?.is_none() {
        return Err(conflict(
            "owned Volicord SessionStart hook was changed or removed",
        ));
    }
    Ok(())
}

fn remove_owned_state(
    document: &mut DocumentMut,
    manifest: &OwnershipManifest,
) -> Result<(), Error> {
    let command = hook_command(&manifest.volicord, &manifest.runtime, &manifest.repository);
    let index = find_hook(document, &command)?
        .ok_or_else(|| conflict("owned Volicord SessionStart hook is unavailable"))?;
    let servers = document
        .get_mut("mcp_servers")
        .and_then(Item::as_table_mut)
        .ok_or_else(|| conflict("owned mcp_servers table is unavailable"))?;
    servers.remove("volicord");
    if servers.is_empty() {
        document.remove("mcp_servers");
    }
    let hooks = document["hooks"]
        .as_table_mut()
        .ok_or_else(|| conflict("owned hooks table is unavailable"))?;
    let groups = hooks["SessionStart"]
        .as_array_of_tables_mut()
        .ok_or_else(|| conflict("owned SessionStart matcher list is unavailable"))?;
    groups.remove(index);
    if groups.is_empty() {
        hooks.remove("SessionStart");
    }
    if hooks.is_empty() {
        document.remove("hooks");
    }
    Ok(())
}

fn has_volicord_state(document: &DocumentMut) -> bool {
    document
        .get("mcp_servers")
        .and_then(Item::as_table)
        .is_some_and(|table| table.contains_key("volicord"))
        || document
            .get("hooks")
            .and_then(Item::as_table)
            .and_then(|hooks| hooks.get("SessionStart"))
            .and_then(Item::as_array_of_tables)
            .is_some_and(|groups| {
                groups.iter().any(|group| {
                    group
                        .get("hooks")
                        .and_then(Item::as_array_of_tables)
                        .is_some_and(|handlers| {
                            handlers.iter().any(|handler| {
                                handler
                                    .get("command")
                                    .and_then(Item::as_str)
                                    .is_some_and(|command| command.contains(" codex hook "))
                            })
                        })
                })
            })
}

fn hooks_are_disabled(document: &DocumentMut) -> bool {
    document
        .get("features")
        .and_then(Item::as_table)
        .is_some_and(|features| {
            features.get("hooks").and_then(Item::as_bool) == Some(false)
                || features.get("codex_hooks").and_then(Item::as_bool) == Some(false)
        })
}

fn mcp_matches(document: &DocumentMut, mcp: &Path, runtime: &Path) -> bool {
    let Some(server) = document
        .get("mcp_servers")
        .and_then(Item::as_table)
        .and_then(|servers| servers.get("volicord"))
        .and_then(Item::as_table)
    else {
        return false;
    };
    if server.len() != 4
        || server.get("command").and_then(Item::as_str) != Some(&*mcp.to_string_lossy())
        || server.get("enabled").and_then(Item::as_bool) != Some(true)
        || server.get("required").and_then(Item::as_bool) != Some(true)
    {
        return false;
    }
    let Some(environment) = server.get("env").and_then(Item::as_inline_table) else {
        return false;
    };
    environment.len() == 1
        && environment
            .get("VOLICORD_RUNTIME_DIR")
            .and_then(toml_edit::Value::as_str)
            == Some(&*runtime.to_string_lossy())
}

fn find_hook(document: &DocumentMut, command: &str) -> Result<Option<usize>, Error> {
    let Some(groups) = document
        .get("hooks")
        .and_then(Item::as_table)
        .and_then(|hooks| hooks.get("SessionStart"))
        .map(|item| {
            item.as_array_of_tables()
                .ok_or_else(|| conflict("hooks.SessionStart is not an array of matcher tables"))
        })
        .transpose()?
    else {
        return Ok(None);
    };
    let matches = groups
        .iter()
        .enumerate()
        .filter_map(|(index, group)| hook_group_matches(group, command).then_some(index))
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err(conflict(
            "duplicate owned Volicord SessionStart hooks are present",
        ));
    }
    Ok(matches.first().copied())
}

fn hook_group_matches(group: &Table, command: &str) -> bool {
    if group.len() != 2 || group.get("matcher").and_then(Item::as_str) != Some(SESSION_MATCHER) {
        return false;
    }
    let Some(handlers) = group.get("hooks").and_then(Item::as_array_of_tables) else {
        return false;
    };
    let Some(handler) = handlers.get(0) else {
        return false;
    };
    handlers.len() == 1
        && handler.len() == 5
        && handler.get("type").and_then(Item::as_str) == Some("command")
        && handler.get("command").and_then(Item::as_str) == Some(command)
        && handler.get("timeout").and_then(Item::as_integer) == Some(5)
        && handler.get("statusMessage").and_then(Item::as_str)
            == Some("Activating Volicord repository context")
        && handler
            .get("additionalContextLimit")
            .and_then(Item::as_integer)
            == Some(2000)
}

fn table_mut<'a>(document: &'a mut DocumentMut, name: &str) -> Result<&'a mut Table, Error> {
    if !document.contains_key(name) {
        document[name] = Item::Table(Table::new());
    }
    document[name]
        .as_table_mut()
        .ok_or_else(|| conflict(format!("{name} is not a TOML table")))
}

fn read_manifest(path: &Path) -> Result<Option<OwnershipManifest>, Error> {
    if !path.exists() {
        return Ok(None);
    }
    let file = File::open(path)
        .map_err(|error| Error::with_source("cannot read Codex ownership manifest", error))?;
    serde_json::from_reader(file)
        .map(Some)
        .map_err(|error| Error::with_source("Codex ownership manifest is invalid", error))
}

fn validate_manifest(manifest: &OwnershipManifest, repository: &Path) -> Result<(), Error> {
    if manifest.kind != MANIFEST_KIND
        || manifest.schema_version != 1
        || manifest.repository != repository
        || !manifest.runtime.is_absolute()
        || !manifest.volicord.is_absolute()
        || !manifest.volicord_mcp.is_absolute()
    {
        return Err(conflict(
            "Codex ownership manifest does not match this repository",
        ));
    }
    Ok(())
}

fn reject_symlink(path: &Path, label: &str) -> Result<(), Error> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(conflict(format!("{label} is a symbolic link")))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Error::with_source(format!("cannot inspect {label}"), error)),
    }
}

fn atomic_write(path: &Path, content: &[u8]) -> Result<(), Error> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::new("generated Codex file has no parent directory"))?;
    let temporary = parent.join(format!(
        ".volicord-write-{}-{}",
        std::process::id(),
        path.file_name().and_then(OsStr::to_str).unwrap_or("state")
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| Error::with_source("cannot create temporary Codex file", error))?;
    if let Ok(metadata) = fs::metadata(path) {
        file.set_permissions(metadata.permissions())
            .map_err(|error| Error::with_source("cannot preserve Codex file permissions", error))?;
    }
    let result = (|| {
        file.write_all(content)?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(Error::with_source(
            "cannot atomically update Codex file",
            error,
        ));
    }
    Ok(())
}

fn conflict(detail: impl Into<String>) -> Error {
    Error::new(format!("Codex integration conflict: {}", detail.into()))
}

struct GitRepository {
    root: PathBuf,
    exclude: PathBuf,
}

impl GitRepository {
    fn inspect(repository: &Path) -> Result<Option<Self>, Error> {
        let top = git(repository, &["rev-parse", "--show-toplevel"])?;
        if !top.status.success() {
            return Ok(None);
        }
        let root_text = String::from_utf8(top.stdout)
            .map_err(|error| Error::with_source("Git repository root is not UTF-8", error))?;
        let root = fs::canonicalize(root_text.trim()).map_err(|error| {
            Error::with_source("cannot canonicalize Git repository root", error)
        })?;
        if root != repository {
            return Err(Error::new(
                "repository path must be the canonical Git worktree root",
            ));
        }
        let output = git(repository, &["rev-parse", "--git-path", "info/exclude"])?;
        if !output.status.success() {
            return Err(Error::new(
                "cannot locate repository-local Git exclusion file",
            ));
        }
        let text = String::from_utf8(output.stdout)
            .map_err(|error| Error::with_source("Git exclusion path is not UTF-8", error))?;
        let candidate = PathBuf::from(text.trim());
        let exclude = if candidate.is_absolute() {
            candidate
        } else {
            repository.join(candidate)
        };
        Ok(Some(Self { root, exclude }))
    }

    fn is_tracked(&self, relative: &str) -> Result<bool, Error> {
        let output = git(&self.root, &["ls-files", "--error-unmatch", "--", relative])?;
        Ok(output.status.success())
    }

    fn updated_exclusion(
        &self,
        previous: Option<&OwnershipManifest>,
        next: &OwnershipManifest,
    ) -> Result<String, Error> {
        let source = match fs::read_to_string(&self.exclude) {
            Ok(source) => source,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => {
                return Err(Error::with_source(
                    "cannot read repository-local Git exclusion file",
                    error,
                ))
            }
        };
        let mut updated = source;
        if let Some(previous) = previous {
            let block = exclusion_block(&previous.excluded_paths);
            if !block.is_empty() {
                let count = updated.matches(&block).count();
                if count != 1 {
                    return Err(conflict(
                        "owned repository-local Git exclusion block was changed or removed",
                    ));
                }
                updated = updated.replacen(&block, "", 1);
            }
        } else if updated.contains(EXCLUDE_BEGIN) || updated.contains(EXCLUDE_END) {
            return Err(conflict(
                "repository-local Git exclusions contain an unowned Volicord marker",
            ));
        }
        let block = exclusion_block(&next.excluded_paths);
        if !block.is_empty() {
            if !updated.is_empty() && !updated.ends_with('\n') {
                updated.push('\n');
            }
            updated.push_str(&block);
        }
        Ok(updated)
    }

    fn write_exclusion(&self, updated: &str) -> Result<(), Error> {
        if let Some(parent) = self.exclude.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                Error::with_source("cannot create repository-local Git info directory", error)
            })?;
        }
        atomic_write(&self.exclude, updated.as_bytes())
    }
}

fn exclusion_block(paths: &[String]) -> String {
    if paths.is_empty() {
        return String::new();
    }
    format!("{EXCLUDE_BEGIN}\n{}\n{EXCLUDE_END}\n", paths.join("\n"))
}

fn git(repository: &Path, args: &[&str]) -> Result<std::process::Output, Error> {
    Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(args)
        .output()
        .map_err(|error| Error::with_source("cannot execute Git for Codex integration", error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn executable_pair(root: &Path) -> (PathBuf, PathBuf) {
        let bin = root.join("bin");
        fs::create_dir(&bin).expect("bin");
        let cli = bin.join("volicord");
        let mcp = bin.join("volicord-mcp");
        for path in [&cli, &mcp] {
            fs::write(path, b"#!/bin/sh\nexit 0\n").expect("binary fixture");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(path, fs::Permissions::from_mode(0o700)).expect("mode");
            }
        }
        (cli, mcp)
    }

    fn git_repository(root: &Path, name: &str) -> PathBuf {
        let repository = root.join(name);
        fs::create_dir(&repository).expect("repository");
        let status = Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&repository)
            .status()
            .expect("git init");
        assert!(status.success());
        repository
    }

    #[test]
    fn enable_is_repository_scoped_idempotent_and_reversibly_preserves_unrelated_state() {
        let temporary = TempDir::new().expect("temporary");
        let authorized = git_repository(temporary.path(), "authorized");
        let unauthorized = git_repository(temporary.path(), "unauthorized");
        let codex_dir = authorized.join(".codex");
        fs::create_dir(&codex_dir).expect("codex dir");
        fs::write(
            codex_dir.join(CONFIG_NAME),
            "model = \"gpt-test\"\n\n[[hooks.Stop]]\n\n[[hooks.Stop.hooks]]\ntype = \"command\"\ncommand = \"true\"\n",
        )
        .expect("existing config");
        let status_before = Command::new("git")
            .args(["status", "--short"])
            .current_dir(&authorized)
            .output()
            .expect("status before enable")
            .stdout;
        let (cli, mcp) = executable_pair(temporary.path());
        let runtime = temporary.path().join("runtime");

        enable(&authorized, &runtime, &cli).expect("enable");
        let first = fs::read_to_string(codex_dir.join(CONFIG_NAME)).expect("config");
        enable(&authorized, &runtime, &cli).expect("idempotent enable");
        let second = fs::read_to_string(codex_dir.join(CONFIG_NAME)).expect("config");
        assert_eq!(first, second);
        assert!(first.contains("model = \"gpt-test\""));
        assert!(first.contains("[[hooks.Stop]]"));
        assert!(first.contains(&format!("command = \"{}\"", mcp.display())));
        assert!(first.contains("required = true"));
        assert!(first.contains(SESSION_MATCHER));
        assert!(!unauthorized.join(".codex").exists());
        let status = Command::new("git")
            .args(["status", "--short"])
            .current_dir(&authorized)
            .output()
            .expect("status");
        assert_eq!(status.stdout, status_before);

        disable(&authorized).expect("disable");
        let restored = fs::read_to_string(codex_dir.join(CONFIG_NAME)).expect("restored config");
        assert!(restored.contains("model = \"gpt-test\""));
        assert!(restored.contains("[[hooks.Stop]]"));
        assert!(!restored.contains("volicord"));
        assert!(!codex_dir.join(MANIFEST_NAME).exists());
        disable(&authorized).expect("idempotent disable");
    }

    #[test]
    fn owned_generated_config_is_removed_and_conflicts_are_bounded() {
        let temporary = TempDir::new().expect("temporary");
        let repository = git_repository(temporary.path(), "repository");
        let (cli, _) = executable_pair(temporary.path());
        let runtime = temporary.path().join("runtime");
        enable(&repository, &runtime, &cli).expect("enable");
        disable(&repository).expect("disable");
        assert!(!repository.join(".codex/config.toml").exists());

        fs::create_dir_all(repository.join(".codex")).expect("codex dir");
        fs::write(
            repository.join(".codex/config.toml"),
            "[mcp_servers.volicord]\ncommand = \"project-owned\"\n",
        )
        .expect("conflict config");
        let error = enable(&repository, &runtime, &cli).expect_err("unowned conflict");
        assert!(error.message().contains("Codex integration conflict"));
        assert!(fs::read_to_string(repository.join(".codex/config.toml"))
            .expect("unchanged")
            .contains("project-owned"));

        fs::write(
            repository.join(".codex/config.toml"),
            "model = \"tracked\"\n",
        )
        .expect("tracked config");
        let status = Command::new("git")
            .args(["add", "-f", ".codex/config.toml"])
            .current_dir(&repository)
            .status()
            .expect("git add");
        assert!(status.success());
        let error = enable(&repository, &runtime, &cli).expect_err("tracked conflict");
        assert!(error.message().contains("is tracked"));
    }

    #[test]
    fn session_start_is_bounded_to_the_authorized_repository_without_runtime_access() {
        let temporary = TempDir::new().expect("temporary");
        let authorized = temporary.path().join("authorized");
        let child = authorized.join("src");
        let unauthorized = temporary.path().join("unauthorized");
        fs::create_dir_all(&child).expect("authorized");
        fs::create_dir(&unauthorized).expect("unauthorized");
        let authorized = fs::canonicalize(authorized).expect("canonical authorized");
        let event = |cwd: &Path, source: &str| {
            serde_json::to_vec(&json!({
                "hook_event_name":"SessionStart",
                "session_id":"session",
                "cwd":cwd,
                "source":source,
                "model":"model",
                "permission_mode":"default",
                "transcript_path":null,
            }))
            .expect("event")
        };
        for source in ["startup", "resume", "clear", "compact"] {
            let encoded = event(&child, source);
            let mut input = encoded.as_slice();
            let output = session_start(&authorized, &mut input)
                .expect("matching hook")
                .expect("activation context");
            assert_eq!(
                output["hookSpecificOutput"]["hookEventName"],
                "SessionStart"
            );
            assert_eq!(
                output["hookSpecificOutput"]["additionalContext"],
                ACTIVATION_CONTEXT
            );
            let context = output["hookSpecificOutput"]["additionalContext"]
                .as_str()
                .expect("activation context text");
            assert!(context.contains("STOP before repository inspection, edits, or continuation"));
            assert!(context
                .contains("successfully Recall before inspecting, editing, or continuing work"));
            assert!(context.contains("record the current-host Goal"));
            assert!(context.contains("establish a repository baseline"));
            assert!(context.contains("before the first ordinary repository write"));
            assert!(context.contains("exactly one category"));
            assert!(context.contains("repository/environment fact--resolve through research"));
            assert!(context.contains("accepted repository/product contract--apply it"));
            assert!(context.contains("delegated implementation choice--the agent may choose"));
            assert!(context.contains("material user-owned outcome--STOP before implementing"));
            assert!(context.contains("user-visible default behavior"));
            assert!(context.contains("CLI/API compatibility behavior"));
            assert!(context.contains("externally observable error or failure policy"));
            assert!(context.contains("privacy/security posture"));
            assert!(context.contains("maintenance/support policy"));
            assert!(context.contains("Public invalid-input behavior"));
            assert!(context.contains("batch-failure continuation policy"));
            assert!(
                context.contains("explicitly delegated by active architecture/product contracts")
            );
            assert!(context.contains("including renderer/layout/detail choices"));
            assert!(context.contains("library default, conventional behavior"));
            assert!(context.contains("implementation simplicity, or agent recommendation"));
            assert!(context.contains("attach source-grounded repository research"));
            assert!(context.contains("explicitly promote it"));
            assert!(
                context.contains("present its actual alternatives, recommendation, and trade-offs")
            );
            assert!(context.contains("explicit current-host user response"));
            assert!(context.contains("record and apply the Decision"));
            assert!(context.contains("facts remain research and must not be asked of the user"));
            assert!(context
                .contains("accepted repository/product contract--apply it and do not reopen it"));
            assert!(context.contains("ordinary code edits require no new approval ceremony"));
            assert!(context
                .contains("same actually observed command execution with a numeric exit status"));
            assert!(context.contains("output-only text is insufficient"));
            assert!(
                context.contains("Meaningful completed or paused work uses a grounded Checkpoint")
            );
        }
        let encoded = event(&unauthorized, "startup");
        let mut input = encoded.as_slice();
        assert!(session_start(&authorized, &mut input)
            .expect("nonmatching hook")
            .is_none());

        let runtime = temporary.path().join("must-not-exist-runtime");
        for (cwd, expects_context) in [(&unauthorized, false), (&child, true)] {
            let encoded = event(cwd, "startup");
            let mut input = encoded.as_slice();
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let exit = crate::run_cli_with_input(
                [
                    "--runtime",
                    runtime.to_str().expect("runtime path"),
                    "codex",
                    "hook",
                    authorized.to_str().expect("authorized path"),
                ],
                &mut input,
                &mut stdout,
                &mut stderr,
            );
            assert_eq!(
                exit,
                crate::CliExit::SUCCESS,
                "{}",
                String::from_utf8_lossy(&stderr)
            );
            assert_eq!(!stdout.is_empty(), expects_context);
            assert!(!runtime.exists());
        }
    }
}
