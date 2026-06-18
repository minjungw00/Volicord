#![forbid(unsafe_code)]

use std::{
    collections::BTreeMap,
    env, fmt, fs,
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

use harness_store::bootstrap::{
    initialize_runtime_home, list_projects, list_surfaces, register_project, register_surface,
    ProjectRegistration, SurfaceRegistration, ACTIVE_PROJECT_STATUS,
};
use harness_types::{
    AccessClass, BASELINE_WORKFLOW_ACCESS_CLASSES, VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION,
};
use serde_json::{json, Map, Value};

const DEFAULT_SURFACE_KIND: &str = "cli";
const DEFAULT_ACCESS_CLASS: AccessClass = AccessClass::ReadStatus;
const BASELINE_WORKFLOW_PROFILE: &str = "baseline-workflow";
const ADMIN_METADATA_JSON: &str = r#"{"created_by":"harness_cli_admin"}"#;

type CliOptions = BTreeMap<String, Vec<String>>;

fn main() {
    let args = env::args().collect::<Vec<_>>();
    let current_dir = match env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("error: failed to read current directory: {error}");
            process::exit(1);
        }
    };

    match run_cli(args, |name| env::var_os(name), &current_dir) {
        Ok(output) => print!("{output}"),
        Err(CliError::Usage(message)) => {
            eprintln!("{message}");
            process::exit(2);
        }
        Err(CliError::Runtime(message)) => {
            eprintln!("error: {message}");
            process::exit(1);
        }
    }
}

fn run_cli<I, S, F>(args: I, env_var: F, current_dir: &Path) -> Result<String, CliError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    let command = args.get(1).map(String::as_str).unwrap_or("--help");

    match command {
        "-h" | "--help" | "help" => Ok(usage()),
        "init" => command_init(&args[2..], env_var, current_dir),
        "project" => command_project(&args[2..], env_var, current_dir),
        "surface" => command_surface(&args[2..], env_var, current_dir),
        other => Err(CliError::usage(format!(
            "unknown command: {other}\n\n{}",
            usage()
        ))),
    }
}

fn command_init<F>(args: &[String], env_var: F, current_dir: &Path) -> Result<String, CliError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let options = parse_options(args, &["runtime-home-id"])?;
    let runtime_home = resolve_runtime_home(env_var, current_dir)?;
    let runtime_home_id = options
        .value("runtime-home-id")
        .unwrap_or_else(|| generated_id("runtime_home"));
    let record = initialize_runtime_home(&runtime_home, &runtime_home_id, ADMIN_METADATA_JSON)?;

    Ok(format!(
        "runtime_home initialized\nruntime_home: {}\nruntime_home_id: {}\nregistry_db: {}\n",
        display_path(&record.runtime_home),
        record.runtime_home_id,
        display_path(&record.registry_db_path)
    ))
}

fn command_project<F>(args: &[String], env_var: F, current_dir: &Path) -> Result<String, CliError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Err(CliError::usage(project_usage()));
    };
    let runtime_home = resolve_runtime_home(env_var, current_dir)?;

    match subcommand {
        "register" => {
            let options = parse_options(&args[1..], &["project-id", "repo-root", "status"])?;
            let project_id = required_option(&options, "project-id")?;
            let repo_root = required_option(&options, "repo-root")?;
            let status = options
                .value("status")
                .unwrap_or_else(|| ACTIVE_PROJECT_STATUS.to_owned());
            let repo_root = canonical_existing_dir(current_dir, repo_root, "repo-root")?;

            let record = register_project(
                &runtime_home,
                ProjectRegistration {
                    project_id,
                    repo_root,
                    project_home: None,
                    status,
                    metadata_json: ADMIN_METADATA_JSON.to_owned(),
                },
            )?;

            Ok(format!(
                "project registered\nproject_id: {}\nrepo_root: {}\nproject_home: {}\nstate_db: {}\nstatus: {}\n",
                record.project_id,
                display_path(&record.repo_root),
                display_path(&record.project_home),
                display_path(&record.state_db_path),
                record.status
            ))
        }
        "list" => {
            let options = parse_options(&args[1..], &[])?;
            reject_options(&options)?;
            let projects = list_projects(&runtime_home)?;
            let mut output = String::from("project_id\trepo_root\tproject_home\tstatus\n");
            for project in projects {
                output.push_str(&format!(
                    "{}\t{}\t{}\t{}\n",
                    project.project_id,
                    display_path(&project.repo_root),
                    display_path(&project.project_home),
                    project.status
                ));
            }
            Ok(output)
        }
        "-h" | "--help" | "help" => Ok(project_usage()),
        other => Err(CliError::usage(format!(
            "unknown project command: {other}\n\n{}",
            project_usage()
        ))),
    }
}

fn command_surface<F>(args: &[String], env_var: F, current_dir: &Path) -> Result<String, CliError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Err(CliError::usage(surface_usage()));
    };
    let runtime_home = resolve_runtime_home(env_var, current_dir)?;

    match subcommand {
        "register" => {
            let options = parse_options_with_repeatable(
                &args[1..],
                &[
                    "project-id",
                    "surface-id",
                    "surface-instance-id",
                    "kind",
                    "name",
                    "access-class",
                    "profile",
                    "capability-profile",
                ],
                &["access-class"],
            )?;
            let project_id = required_option(&options, "project-id")?;
            let surface_id = required_option(&options, "surface-id")?;
            let surface_instance_id = options
                .value("surface-instance-id")
                .unwrap_or_else(|| generated_id("surface_instance"));
            let surface_kind = options
                .value("kind")
                .unwrap_or_else(|| DEFAULT_SURFACE_KIND.to_owned());
            let display_name = options.value("name");
            let access_classes = surface_access_classes(&options)?;
            let capability_profile_json =
                capability_profile_json(&access_classes, options.value_ref("capability-profile"))?;
            let local_access_json = local_access_json(&access_classes)?;

            let record = register_surface(
                &runtime_home,
                SurfaceRegistration {
                    project_id,
                    surface_id,
                    surface_instance_id,
                    surface_kind,
                    display_name,
                    capability_profile_json,
                    local_access_json,
                    metadata_json: ADMIN_METADATA_JSON.to_owned(),
                },
            )?;
            let access_class = access_class_from_local_access(&record.local_access_json)
                .unwrap_or_else(|| DEFAULT_ACCESS_CLASS.as_str().to_owned());

            Ok(format!(
                "surface registered\nproject_id: {}\nsurface_id: {}\nsurface_instance_id: {}\nsurface_kind: {}\naccess_class: {}\n",
                record.project_id,
                record.surface_id,
                record.surface_instance_id,
                record.surface_kind,
                access_class
            ))
        }
        "list" => {
            let options = parse_options(&args[1..], &["project-id"])?;
            let project_id = required_option(&options, "project-id")?;
            let surfaces = list_surfaces(&runtime_home, &project_id)?;
            let mut output = String::from(
                "project_id\tsurface_id\tsurface_instance_id\tsurface_kind\taccess_class\tdisplay_name\n",
            );
            for surface in surfaces {
                let access_class = access_class_from_local_access(&surface.local_access_json)
                    .unwrap_or_else(|| String::from(""));
                let display_name = surface.display_name.unwrap_or_default();
                output.push_str(&format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\n",
                    surface.project_id,
                    surface.surface_id,
                    surface.surface_instance_id,
                    surface.surface_kind,
                    access_class,
                    display_name
                ));
            }
            Ok(output)
        }
        "-h" | "--help" | "help" => Ok(surface_usage()),
        other => Err(CliError::usage(format!(
            "unknown surface command: {other}\n\n{}",
            surface_usage()
        ))),
    }
}

fn parse_options(args: &[String], allowed: &[&str]) -> Result<CliOptions, CliError> {
    parse_options_with_repeatable(args, allowed, &[])
}

fn parse_options_with_repeatable(
    args: &[String],
    allowed: &[&str],
    repeatable: &[&str],
) -> Result<CliOptions, CliError> {
    let mut options = BTreeMap::new();
    let mut index = 0;

    while index < args.len() {
        let token = &args[index];
        if token == "-h" || token == "--help" || token == "help" {
            return Err(CliError::usage(usage()));
        }
        if !token.starts_with("--") {
            return Err(CliError::usage(format!("unexpected argument: {token}")));
        }

        let without_prefix = &token[2..];
        let (name, value) = if let Some((name, value)) = without_prefix.split_once('=') {
            (name.to_owned(), value.to_owned())
        } else {
            index += 1;
            let Some(value) = args.get(index) else {
                return Err(CliError::usage(format!(
                    "missing value for --{without_prefix}"
                )));
            };
            (without_prefix.to_owned(), value.clone())
        };

        if !allowed.iter().any(|allowed_name| *allowed_name == name) {
            return Err(CliError::usage(format!("unknown option: --{name}")));
        }
        let values = options.entry(name.clone()).or_insert_with(Vec::new);
        if !values.is_empty()
            && !repeatable
                .iter()
                .any(|repeatable_name| *repeatable_name == name)
        {
            return Err(CliError::usage(format!("duplicate option: --{name}")));
        }
        values.push(value);

        index += 1;
    }

    Ok(options)
}

trait CliOptionsExt {
    fn value(&self, name: &str) -> Option<String>;
    fn value_ref(&self, name: &str) -> Option<&String>;
}

impl CliOptionsExt for CliOptions {
    fn value(&self, name: &str) -> Option<String> {
        self.value_ref(name).cloned()
    }

    fn value_ref(&self, name: &str) -> Option<&String> {
        self.get(name).and_then(|values| values.first())
    }
}

fn required_option(options: &CliOptions, name: &str) -> Result<String, CliError> {
    options
        .value_ref(name)
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .ok_or_else(|| CliError::usage(format!("missing required option: --{name}")))
}

fn reject_options(options: &CliOptions) -> Result<(), CliError> {
    if options.is_empty() {
        Ok(())
    } else {
        let names = options
            .keys()
            .map(|name| format!("--{name}"))
            .collect::<Vec<_>>()
            .join(", ");
        Err(CliError::usage(format!("unexpected option(s): {names}")))
    }
}

fn resolve_runtime_home<F>(env_var: F, current_dir: &Path) -> Result<PathBuf, CliError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    if let Some(value) = env_var("HARNESS_HOME") {
        if value.is_empty() {
            return Err(CliError::runtime("HARNESS_HOME must not be empty"));
        }
        return Ok(absolute_path(current_dir, PathBuf::from(value)));
    }

    let Some(home) = default_user_home(env_var) else {
        return Err(CliError::runtime(
            "could not determine a default home directory; set HARNESS_HOME",
        ));
    };
    Ok(home.join(".harness"))
}

fn default_user_home<F>(env_var: F) -> Option<PathBuf>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    env_var("HOME")
        .map(PathBuf::from)
        .or_else(|| env_var("USERPROFILE").map(PathBuf::from))
        .or_else(|| {
            let drive = env_var("HOMEDRIVE")?;
            let path = env_var("HOMEPATH")?;
            let mut home = PathBuf::from(drive);
            home.push(path);
            Some(home)
        })
}

fn canonical_existing_dir(
    current_dir: &Path,
    value: String,
    field: &'static str,
) -> Result<PathBuf, CliError> {
    let path = absolute_path(current_dir, PathBuf::from(value));
    let path = fs::canonicalize(&path)
        .map_err(|error| CliError::runtime(format!("{field} is not accessible: {error}")))?;
    if path.is_dir() {
        Ok(path)
    } else {
        Err(CliError::runtime(format!("{field} must be a directory")))
    }
}

fn absolute_path(current_dir: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        current_dir.join(path)
    }
}

fn capability_profile_json(
    access_classes: &[AccessClass],
    provided: Option<&String>,
) -> Result<String, CliError> {
    let mut value = match provided {
        Some(text) => serde_json::from_str::<Value>(text).map_err(|error| {
            CliError::usage(format!("invalid --capability-profile JSON: {error}"))
        })?,
        None => Value::Object(Map::new()),
    };

    let Some(object) = value.as_object_mut() else {
        return Err(CliError::usage(
            "--capability-profile must be a JSON object",
        ));
    };
    let primary = primary_access_class(access_classes)?;
    object.insert("access_class".to_owned(), json!(primary.as_str()));
    object
        .entry("supported_access_classes".to_owned())
        .or_insert_with(|| json!(access_class_strings(access_classes)));

    serde_json::to_string(&value)
        .map_err(|error| CliError::runtime(format!("failed to encode capability profile: {error}")))
}

fn local_access_json(access_classes: &[AccessClass]) -> Result<String, CliError> {
    let primary = primary_access_class(access_classes)?;
    serde_json::to_string(&json!({
        "access_class": primary.as_str(),
        "authorized_access_classes": access_class_strings(access_classes),
        "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
    }))
    .map_err(|error| CliError::runtime(format!("failed to encode local access metadata: {error}")))
}

fn access_class_from_local_access(text: &str) -> Option<String> {
    let value = serde_json::from_str::<Value>(text).ok()?;
    let access_classes = value
        .get("authorized_access_classes")
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    if !access_classes.is_empty() {
        return Some(access_classes.join(","));
    }
    value
        .get("access_class")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn surface_access_classes(options: &CliOptions) -> Result<Vec<AccessClass>, CliError> {
    let mut access_classes = Vec::new();
    if let Some(values) = options.get("access-class") {
        for value in values {
            push_access_class(&mut access_classes, parse_access_class(value)?);
        }
    }

    if let Some(profile) = options.value_ref("profile") {
        if profile != BASELINE_WORKFLOW_PROFILE {
            return Err(CliError::usage(format!("unknown profile: {profile}")));
        }
        push_access_classes(&mut access_classes, BASELINE_WORKFLOW_ACCESS_CLASSES);
    }

    if access_classes.is_empty() && !options.contains_key("profile") {
        push_access_class(&mut access_classes, DEFAULT_ACCESS_CLASS);
    }

    if access_classes.is_empty() {
        Err(CliError::usage(
            "surface registration requires at least one access class",
        ))
    } else {
        Ok(access_classes)
    }
}

fn parse_access_class(value: &str) -> Result<AccessClass, CliError> {
    serde_json::from_value(Value::String(value.to_owned()))
        .map_err(|_| CliError::usage(format!("unknown access class: {value}")))
}

fn push_access_classes<const N: usize>(target: &mut Vec<AccessClass>, values: [AccessClass; N]) {
    for value in values {
        push_access_class(target, value);
    }
}

fn push_access_class(target: &mut Vec<AccessClass>, value: AccessClass) {
    if !target.contains(&value) {
        target.push(value);
    }
}

fn primary_access_class(access_classes: &[AccessClass]) -> Result<AccessClass, CliError> {
    access_classes
        .first()
        .copied()
        .ok_or_else(|| CliError::usage("surface registration requires at least one access class"))
}

fn access_class_strings(access_classes: &[AccessClass]) -> Vec<&'static str> {
    access_classes.iter().map(|value| value.as_str()).collect()
}

fn generated_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{prefix}_{nanos}_{}", process::id())
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

fn usage() -> String {
    format!(
        "Usage:\n  harness init [--runtime-home-id ID]\n  {}\n  {}\n\nEnvironment:\n  HARNESS_HOME  Override Runtime Home path (default: $HOME/.harness)\n\nThese are local administrative setup commands, not public Harness API methods.\n",
        project_usage().trim_end(),
        surface_usage().trim_end()
    )
}

fn project_usage() -> String {
    "harness project register --project-id ID --repo-root PATH [--status active]\nharness project list\n"
        .to_owned()
}

fn surface_usage() -> String {
    "harness surface register --project-id ID --surface-id ID [--surface-instance-id ID] [--kind KIND] [--name NAME] [--access-class ACCESS_CLASS ...] [--profile baseline-workflow] [--capability-profile JSON]\nharness surface list --project-id ID\n"
        .to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliError {
    Usage(String),
    Runtime(String),
}

impl CliError {
    fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }

    fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for CliError {}

impl From<harness_store::StoreError> for CliError {
    fn from(error: harness_store::StoreError) -> Self {
        Self::Runtime(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::OsString,
        path::{Path, PathBuf},
    };

    use harness_store::sqlite::{project_state_db_path, registry_db_path};
    use harness_test_support::TempRuntimeHome;
    use rusqlite::Connection;

    use super::*;

    #[test]
    fn init_respects_harness_home_override() {
        let runtime_home = TempRuntimeHome::new("cli-init").expect("temp runtime home");
        let output = run_with_home(
            runtime_home.path(),
            ["harness", "init", "--runtime-home-id", "runtime_home_test"],
        )
        .expect("init should succeed");

        assert!(output.contains("runtime_home initialized\n"));
        assert!(output.contains("runtime_home_id: runtime_home_test\n"));
        assert!(registry_db_path(runtime_home.path()).exists());
    }

    #[test]
    fn default_runtime_home_uses_user_home() {
        let runtime_home = resolve_runtime_home(
            |name| {
                if name == "HOME" {
                    Some(OsString::from("/tmp/harness-cli-home"))
                } else {
                    None
                }
            },
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
        .expect("default runtime home should resolve");

        assert_eq!(
            runtime_home,
            PathBuf::from("/tmp/harness-cli-home/.harness")
        );
    }

    #[test]
    fn project_register_creates_registry_and_project_state() {
        let runtime_home = TempRuntimeHome::new("cli-project").expect("temp runtime home");
        run_with_home(
            runtime_home.path(),
            [
                "harness",
                "init",
                "--runtime-home-id",
                "runtime_home_project",
            ],
        )
        .expect("init should succeed");

        let output = run_with_home(
            runtime_home.path(),
            [
                "harness",
                "project",
                "register",
                "--project-id",
                "project_alpha",
                "--repo-root",
                ".",
            ],
        )
        .expect("project register should succeed");

        assert!(output.contains("project registered\n"));
        assert!(output.contains("project_id: project_alpha\n"));
        assert!(project_state_db_path(runtime_home.path(), "project_alpha").exists());

        let conn = Connection::open(project_state_db_path(runtime_home.path(), "project_alpha"))
            .expect("state database should open");
        let state_version: i64 = conn
            .query_row(
                "SELECT state_version FROM project_state WHERE project_id = 'project_alpha'",
                [],
                |row| row.get(0),
            )
            .expect("project state row should exist");
        assert_eq!(state_version, 0);
    }

    #[test]
    fn project_list_uses_deterministic_order() {
        let runtime_home = TempRuntimeHome::new("cli-project-list").expect("temp runtime home");
        run_with_home(
            runtime_home.path(),
            [
                "harness",
                "init",
                "--runtime-home-id",
                "runtime_home_project_list",
            ],
        )
        .expect("init should succeed");

        for project_id in ["project_b", "project_a"] {
            run_with_home(
                runtime_home.path(),
                [
                    "harness",
                    "project",
                    "register",
                    "--project-id",
                    project_id,
                    "--repo-root",
                    ".",
                ],
            )
            .expect("project register should succeed");
        }

        let output = run_with_home(runtime_home.path(), ["harness", "project", "list"])
            .expect("project list should succeed");
        let lines = output.lines().collect::<Vec<_>>();
        assert_eq!(lines[0], "project_id\trepo_root\tproject_home\tstatus");
        assert!(lines[1].starts_with("project_a\t"));
        assert!(lines[2].starts_with("project_b\t"));
    }

    #[test]
    fn surface_register_and_list_store_local_context_metadata() {
        let runtime_home = TempRuntimeHome::new("cli-surface").expect("temp runtime home");
        run_with_home(
            runtime_home.path(),
            [
                "harness",
                "init",
                "--runtime-home-id",
                "runtime_home_surface",
            ],
        )
        .expect("init should succeed");
        run_with_home(
            runtime_home.path(),
            [
                "harness",
                "project",
                "register",
                "--project-id",
                "project_surface",
                "--repo-root",
                ".",
            ],
        )
        .expect("project register should succeed");

        let output = run_with_home(
            runtime_home.path(),
            [
                "harness",
                "surface",
                "register",
                "--project-id",
                "project_surface",
                "--surface-id",
                "surface_cli",
                "--surface-instance-id",
                "surface_instance_test",
                "--kind",
                "cli",
                "--name",
                "Local CLI",
                "--access-class",
                "core_mutation",
                "--capability-profile",
                r#"{"local_reachability":true}"#,
            ],
        )
        .expect("surface register should succeed");

        assert_eq!(
            output,
            "surface registered\nproject_id: project_surface\nsurface_id: surface_cli\nsurface_instance_id: surface_instance_test\nsurface_kind: cli\naccess_class: core_mutation\n"
        );

        let list_output = run_with_home(
            runtime_home.path(),
            [
                "harness",
                "surface",
                "list",
                "--project-id",
                "project_surface",
            ],
        )
        .expect("surface list should succeed");
        assert_eq!(
            list_output,
            "project_id\tsurface_id\tsurface_instance_id\tsurface_kind\taccess_class\tdisplay_name\nproject_surface\tsurface_cli\tsurface_instance_test\tcli\tcore_mutation\tLocal CLI\n"
        );

        let conn = Connection::open(project_state_db_path(
            runtime_home.path(),
            "project_surface",
        ))
        .expect("state database should open");
        let (default_surface_id, capability_profile, local_access): (String, String, String) = conn
            .query_row(
                "SELECT default_surface_id, capability_profile_json, local_access_json
                   FROM project_state
                   JOIN surfaces USING (project_id)
                  WHERE project_id = 'project_surface'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("surface metadata should exist");
        assert_eq!(default_surface_id, "surface_cli");
        let capability = serde_json::from_str::<Value>(&capability_profile)
            .expect("capability profile should be JSON");
        assert_eq!(capability["access_class"], "core_mutation");
        assert_eq!(capability["local_reachability"], true);
        let local_access =
            serde_json::from_str::<Value>(&local_access).expect("local access should be JSON");
        assert_eq!(local_access["access_class"], "core_mutation");
        assert_eq!(
            local_access["authorized_access_classes"],
            json!(["core_mutation"])
        );
        assert_eq!(
            local_access["verification_basis"],
            "local_admin_registration"
        );
    }

    #[test]
    fn surface_register_repeatable_access_classes_are_deduplicated_in_order() {
        let runtime_home = TempRuntimeHome::new("cli-surface-repeat").expect("temp runtime home");
        run_with_home(
            runtime_home.path(),
            [
                "harness",
                "init",
                "--runtime-home-id",
                "runtime_home_surface_repeat",
            ],
        )
        .expect("init should succeed");
        run_with_home(
            runtime_home.path(),
            [
                "harness",
                "project",
                "register",
                "--project-id",
                "project_repeat",
                "--repo-root",
                ".",
            ],
        )
        .expect("project register should succeed");

        let output = run_with_home(
            runtime_home.path(),
            [
                "harness",
                "surface",
                "register",
                "--project-id",
                "project_repeat",
                "--surface-id",
                "surface_repeat",
                "--surface-instance-id",
                "surface_instance_repeat",
                "--access-class",
                "core_mutation",
                "--access-class",
                "read_status",
                "--access-class",
                "core_mutation",
            ],
        )
        .expect("surface register should succeed");

        assert!(output.contains("access_class: core_mutation,read_status\n"));
        let conn = Connection::open(project_state_db_path(runtime_home.path(), "project_repeat"))
            .expect("state database should open");
        let (capability_profile, local_access): (String, String) = conn
            .query_row(
                "SELECT capability_profile_json, local_access_json
                   FROM surfaces
                  WHERE project_id = 'project_repeat'
                    AND surface_id = 'surface_repeat'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("surface metadata should exist");
        let capability = serde_json::from_str::<Value>(&capability_profile)
            .expect("capability profile should be JSON");
        assert_eq!(capability["access_class"], "core_mutation");
        assert_eq!(
            capability["supported_access_classes"],
            json!(["core_mutation", "read_status"])
        );
        let local_access =
            serde_json::from_str::<Value>(&local_access).expect("local access should be JSON");
        assert_eq!(local_access["access_class"], "core_mutation");
        assert_eq!(
            local_access["authorized_access_classes"],
            json!(["core_mutation", "read_status"])
        );
    }

    #[test]
    fn surface_register_baseline_workflow_profile_persists_explicit_grants() {
        let runtime_home = TempRuntimeHome::new("cli-surface-profile").expect("temp runtime home");
        run_with_home(
            runtime_home.path(),
            [
                "harness",
                "init",
                "--runtime-home-id",
                "runtime_home_surface_profile",
            ],
        )
        .expect("init should succeed");
        run_with_home(
            runtime_home.path(),
            [
                "harness",
                "project",
                "register",
                "--project-id",
                "project_profile",
                "--repo-root",
                ".",
            ],
        )
        .expect("project register should succeed");

        let output = run_with_home(
            runtime_home.path(),
            [
                "harness",
                "surface",
                "register",
                "--project-id",
                "project_profile",
                "--surface-id",
                "surface_profile",
                "--surface-instance-id",
                "surface_instance_profile",
                "--profile",
                "baseline-workflow",
            ],
        )
        .expect("surface register should succeed");

        let expected = json!([
            "read_status",
            "core_mutation",
            "write_authorization",
            "artifact_registration",
            "run_recording"
        ]);
        assert!(output.contains(
            "access_class: read_status,core_mutation,write_authorization,artifact_registration,run_recording\n"
        ));
        let conn = Connection::open(project_state_db_path(
            runtime_home.path(),
            "project_profile",
        ))
        .expect("state database should open");
        let local_access: String = conn
            .query_row(
                "SELECT local_access_json
                   FROM surfaces
                  WHERE project_id = 'project_profile'
                    AND surface_id = 'surface_profile'",
                [],
                |row| row.get(0),
            )
            .expect("surface metadata should exist");
        let local_access =
            serde_json::from_str::<Value>(&local_access).expect("local access should be JSON");
        assert_eq!(local_access["access_class"], "read_status");
        assert_eq!(local_access["authorized_access_classes"], expected);
        assert!(local_access.get("profile").is_none());
    }

    #[test]
    fn surface_register_profile_and_explicit_classes_use_deterministic_union() {
        let runtime_home = TempRuntimeHome::new("cli-surface-union").expect("temp runtime home");
        run_with_home(
            runtime_home.path(),
            [
                "harness",
                "init",
                "--runtime-home-id",
                "runtime_home_surface_union",
            ],
        )
        .expect("init should succeed");
        run_with_home(
            runtime_home.path(),
            [
                "harness",
                "project",
                "register",
                "--project-id",
                "project_union",
                "--repo-root",
                ".",
            ],
        )
        .expect("project register should succeed");

        run_with_home(
            runtime_home.path(),
            [
                "harness",
                "surface",
                "register",
                "--project-id",
                "project_union",
                "--surface-id",
                "surface_union",
                "--surface-instance-id",
                "surface_instance_union",
                "--access-class",
                "run_recording",
                "--profile",
                "baseline-workflow",
            ],
        )
        .expect("surface register should succeed");

        let conn = Connection::open(project_state_db_path(runtime_home.path(), "project_union"))
            .expect("state database should open");
        let local_access: String = conn
            .query_row(
                "SELECT local_access_json
                   FROM surfaces
                  WHERE project_id = 'project_union'
                    AND surface_id = 'surface_union'",
                [],
                |row| row.get(0),
            )
            .expect("surface metadata should exist");
        let local_access =
            serde_json::from_str::<Value>(&local_access).expect("local access should be JSON");
        assert_eq!(
            local_access["authorized_access_classes"],
            json!([
                "run_recording",
                "read_status",
                "core_mutation",
                "write_authorization",
                "artifact_registration"
            ])
        );
        assert_eq!(local_access["access_class"], "run_recording");
    }

    #[test]
    fn project_register_requires_initialized_runtime_home() {
        let runtime_home = TempRuntimeHome::new("cli-uninitialized").expect("temp runtime home");
        let error = run_with_home(
            runtime_home.path(),
            [
                "harness",
                "project",
                "register",
                "--project-id",
                "project_missing_runtime",
                "--repo-root",
                ".",
            ],
        )
        .expect_err("project register should require init");

        assert!(matches!(error, CliError::Runtime(_)));
        assert!(error.to_string().contains("runtime_home not found"));
    }

    fn run_with_home<const N: usize>(
        runtime_home: &Path,
        args: [&str; N],
    ) -> Result<String, CliError> {
        run_cli(
            args,
            |name| {
                if name == "HARNESS_HOME" {
                    Some(OsString::from(runtime_home))
                } else {
                    None
                }
            },
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
    }
}
