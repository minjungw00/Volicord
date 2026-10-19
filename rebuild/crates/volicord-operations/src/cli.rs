use crate::{
    bounded_repository_analysis_json, complete_repository_analysis_json,
    operations::{parse_identity, select_document},
    ConfirmationDecision, ConfirmationRequestId, Error, GuardedEffectCategory, GuardedEffectDraft,
    GuardedRisk, LocalOperations, RequestingProvenance, RuntimeLayout,
};
use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::{json, Value};
use std::{
    env,
    ffi::{OsStr, OsString},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
};
use volicord_context::{
    BundleComparison, BundleConflictClass, BundleMergeStatus, CanonicalRecordId, CheckpointDraft,
    CheckpointKind, ContextItemCorrectionDraft, ContextItemId, CorrectionKind,
    DecisionCorrectionDraft, DecisionId, MergeResolution, MergeResolutionMode, OperationId,
    Principal, PrincipalKind, ProjectId, SourceId, UserAcceptanceFact, UserAcceptanceState,
    UserReviewFact, UserReviewState, VerificationFact, VerificationState, WorkState,
};
use volicord_privacy::{
    ProviderIntentProvenance, ProviderOptInPolicy, ProviderRetentionPolicy, SecretFilteringPolicy,
    SourceExclusionPolicy,
};
use volicord_projections::{
    build_project_understanding, DocumentKind, DocumentRequest, FixedLocale, GeneratorIdentity,
    NarrativeRealizationState, OutputFormat, RequestedDestination, UnderstandingBound,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CliExit(i32);

impl CliExit {
    pub const SUCCESS: Self = Self(0);
    pub const USAGE: Self = Self(2);
    pub const FAILURE: Self = Self(1);
    pub const fn code(self) -> i32 {
        self.0
    }
}

pub fn run_cli<I, S>(args: I, stdout: &mut dyn Write, stderr: &mut dyn Write) -> CliExit
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    run_cli_with_input(args, &mut std::io::empty(), stdout, stderr)
}

pub fn run_cli_with_input<I, S>(
    args: I,
    input: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> CliExit
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut argv = vec![OsString::from("volicord")];
    argv.extend(args.into_iter().map(Into::into));
    let matches = match command().try_get_matches_from(argv) {
        Ok(matches) => matches,
        Err(error) => {
            let exit = if error.use_stderr() {
                CliExit::USAGE
            } else {
                CliExit::SUCCESS
            };
            if error.use_stderr() {
                let _ = write!(stderr, "{error}");
            } else {
                let _ = write!(stdout, "{error}");
            }
            return exit;
        }
    };
    let explanation_operation = matches!(
        matches.subcommand_name(),
        Some("work" | "decision" | "repository")
    );
    match execute(matches, input, stdout) {
        Ok(()) => CliExit::SUCCESS,
        Err(error) => {
            let _ = writeln!(stderr, "Error: {}", error.message());
            if !explanation_operation {
                let _ = writeln!(
                    stderr,
                    "Try 'volicord --help' or the command's '--help' for usage."
                );
            }
            CliExit::FAILURE
        }
    }
}

fn execute(matches: ArgMatches, input: &mut dyn Read, stdout: &mut dyn Write) -> Result<(), Error> {
    let runtime = if let Some(value) = matches.get_one::<PathBuf>("runtime") {
        RuntimeLayout::new(value.clone())?
    } else {
        RuntimeLayout::from_environment()?
    };
    let mut format = OutputMode {
        json: matches.get_flag("json"),
        locale: match matches
            .get_one::<String>("locale")
            .map(String::as_str)
            .unwrap_or("en")
        {
            "ko" => CliLocale::Korean,
            _ => CliLocale::English,
        },
    };
    let selection = ProjectSelection {
        explicit: matches
            .get_one::<String>("project")
            .map(|value| project_id(value))
            .transpose()?,
        repository: matches.get_one::<PathBuf>("repository").cloned(),
    };
    let (name, command_matches) = matches
        .subcommand()
        .ok_or_else(|| Error::new("a command is required"))?;
    // Lifecycle hooks are a host protocol even when ordinary CLI output is human-readable.
    if name == "codex" && command_matches.subcommand_name() == Some("hook") {
        format.json = true;
    }
    let operations = LocalOperations::new(runtime.clone());
    let value = dispatch(
        name,
        command_matches,
        &operations,
        &runtime,
        &selection,
        input,
    )?;
    if let Some(value) = value {
        render(&value, format, stdout)?;
    }
    Ok(())
}

const USAGE: &str = "volicord [OPTIONS] <COMMAND>";

#[derive(Clone, Copy)]
enum CliLocale {
    English,
    Korean,
}

#[derive(Clone, Copy)]
struct OutputMode {
    json: bool,
    locale: CliLocale,
}

struct ProjectSelection {
    explicit: Option<ProjectId>,
    repository: Option<PathBuf>,
}

fn command() -> Command {
    Command::new("volicord")
        .about("Understand a repository, preserve its decisions, and resume work")
        .long_about("Volicord is a local-first project understanding and decision-memory tool. Ordinary project commands resolve the repository from --repository or the current directory, so a Project ID is rarely needed.")
        .arg_required_else_help(true)
        .disable_help_subcommand(true)
        .after_help("Examples:\n  volicord init \"My Project\"\n  volicord status\n  volicord analyze\n  volicord recall\n  volicord document preview handoff-resume\n  volicord context export --output /tmp/project.volicord.json\n  volicord doctor check")
        .arg(path_arg("runtime", "runtime", "Use an explicit Runtime Home").global(true))
        .arg(path_arg("repository", "repository", "Resolve the Project from this repository").global(true))
        .arg(Arg::new("project").long("project").value_name("PROJECT_ID").help("Select a Project explicitly when repository resolution is ambiguous").global(true))
        .arg(Arg::new("json").long("json").help("Emit machine-readable JSON").action(ArgAction::SetTrue).global(true))
        .arg(Arg::new("locale").long("locale").value_name("LOCALE").value_parser(["en", "ko"]).default_value("en").help("Locale for fixed CLI text: en or ko").global(true))
        .subcommand(Command::new("init").about("Initialize and bind a Project to a repository").arg(Arg::new("name").value_name("NAME").help("Project display name")).arg(Arg::new("no_bind").long("no-bind").help("Initialize without binding a repository").action(ArgAction::SetTrue)).after_help("Examples:\n  volicord init \"Payments Service\"\n  volicord --repository /work/payments init \"Payments Service\""))
        .subcommand(Command::new("bind").about("Bind an existing Project to this repository").arg(Arg::new("revision").long("revision").value_name("REVISION")).after_help("Example:\n  volicord --project PROJECT_ID --repository /work/clone bind"))
        .subcommand(Command::new("status").arg(reading_language()).about("Show current Project Understanding").after_help("Example:\n  volicord status\n\nUse --json for automation and --project only when path-based resolution is ambiguous."))
        .subcommand(Command::new("analyze").about("Analyze the current repository").arg(repeat_arg("exclude", "PATH", "Exclude a repository-relative path")))
        .subcommand(Command::new("recall").arg(reading_language()).about("Resume from bounded Project memory"))
        .subcommand(Command::new("questions").about("Show the current material Question frontier").arg(repeat_arg("scope", "SCOPE", "Restrict the material scope")))
        .subcommand(Command::new("decisions").arg(reading_language()).about("Inspect current and historical Decisions"))
        .subcommand(explanation_command("work"))
        .subcommand(explanation_command("decision"))
        .subcommand(repository_command())
        .subcommand(document_command())
        .subcommand(viewer_command())
        .subcommand(context_command())
        .subcommand(privacy_command())
        .subcommand(doctor_command())
        .subcommand(codex_command())
        .subcommand(advanced_command())
}

fn reading_language() -> Arg {
    Arg::new("language")
        .long("language")
        .default_value("en")
        .help("Exact generated answer language")
}
fn reading_options(matches: &ArgMatches) -> (&str, FixedLocale) {
    (
        matches
            .get_one::<String>("language")
            .map(String::as_str)
            .unwrap_or("en"),
        match matches.get_one::<String>("locale").map(String::as_str) {
            Some("ko") => FixedLocale::Korean,
            _ => FixedLocale::English,
        },
    )
}
fn path_arg(id: &'static str, long: &'static str, help: &'static str) -> Arg {
    Arg::new(id)
        .long(long)
        .value_name("PATH")
        .value_parser(clap::value_parser!(PathBuf))
        .help(help)
}

fn repeat_arg(id: &'static str, value: &'static str, help: &'static str) -> Arg {
    Arg::new(id)
        .long(id)
        .value_name(value)
        .action(ArgAction::Append)
        .help(help)
}

fn explanation_command(kind: &'static str) -> Command {
    let subject = || {
        Arg::new(if kind == "repository" {
            "analysis"
        } else {
            kind
        })
        .long(if kind == "repository" {
            "analysis"
        } else {
            kind
        })
        .required(true)
        .value_name("SUBJECT_ID")
    };
    let language = || {
        Arg::new("language")
            .long("language")
            .default_value("en")
            .value_name("LANGUAGE")
    };
    let mut command = Command::new(kind)
        .about("Read and explain a specific subject")
        .subcommand_required(true)
        .subcommand(
            Command::new("explain")
                .about("Explicit active-host explanation; never invokes a background provider")
                .subcommand_required(true)
                .subcommand(
                    Command::new("prepare")
                        .about("Read full evidence and generation instructions")
                        .arg(subject())
                        .arg(language()),
                )
                .subcommand(
                    Command::new("record")
                        .about(
                            "Retain an explicitly generated host response with current grounding",
                        )
                        .arg(subject())
                        .arg(language())
                        .arg(
                            path_arg("input", "input", "Host-generated realization JSON")
                                .required(true),
                        ),
                )
                .subcommand(
                    Command::new("delete")
                        .about("Delete all retained explanations for this subject")
                        .arg(subject()),
                ),
        );
    if kind == "work" || kind == "decision" {
        command = command.mut_subcommand("explain", |explain| {
            explain.subcommand(
                Command::new("compose")
                    .about("Bind a host-authored article; no generation or retention")
                    .arg(subject())
                    .arg(
                        path_arg("input", "input", "Host-authored Work draft JSON").required(true),
                    ),
            )
        });
        if kind == "work" {
            command = command.subcommand(work_source_command()).subcommand(Command::new("verification").about("Read independent typed verification and retained execution facts; never generates").arg(Arg::new("work").long("work").required(true)));
        }
    }
    command
}

fn work_source_command() -> Command {
    let work = || Arg::new("work").long("work").required(true);
    let analysis = || Arg::new("analysis").long("analysis").required(true);
    let source = || Arg::new("source").long("source").required(true);
    let retention = || {
        Arg::new("retain-seconds")
            .long("retain-seconds")
            .required(true)
            .value_parser(clap::value_parser!(u64).range(1..=604800))
    };
    Command::new("source")
        .about("Explicit local Work evidence; reads never capture or renew retention")
        .subcommand_required(true)
        .subcommand(Command::new("observe")
            .about("Observe one included UTF-8 file or bounded byte region in the selected analysis")
            .arg(work()).arg(analysis()).arg(retention())
            .arg(Arg::new("path").long("path").required(true))
            .arg(Arg::new("before-source").long("before-source"))
            .arg(Arg::new("start-byte").long("start-byte").requires("end-byte")
                .value_parser(clap::value_parser!(usize)))
            .arg(Arg::new("end-byte").long("end-byte").requires("start-byte")
                .value_parser(clap::value_parser!(usize))))
        .subcommand(Command::new("run")
            .about("Execute an explicitly permitted local command and retain bounded non-secret output")
            .after_help("The selected analysis identifies the requested basis; it does not prove the execution-time tree or assertion coverage. Command exit is in the Source outcome, distinct from successful evidence recording. No browser/MCP execution authority is added.")
            .arg(work()).arg(analysis()).arg(retention())
            .arg(Arg::new("label").long("label").required(true))
            .arg(Arg::new("timeout-seconds").long("timeout-seconds").required(true)
                .value_parser(clap::value_parser!(u64).range(1..=300)))
            .arg(Arg::new("command").required(true).num_args(1..).last(true)
                .value_name("PROGRAM [ARG...]")))
        .subcommand(Command::new("list").about("List only exact-Work observation metadata and read selectors").arg(work()))
        .subcommand(Command::new("read").about("Read one permitted retained observation; no filesystem fallback").arg(work()).arg(source()))
        .subcommand(Command::new("delete").about("Delete one Work Source copy, retaining canonical identity").arg(work()).arg(source()))
}

fn dispatch_work_source(
    operations: &LocalOperations,
    selection: &ProjectSelection,
    actions: &ArgMatches,
) -> Result<Value, Error> {
    let (action, args) = actions
        .subcommand()
        .ok_or_else(|| Error::new("a Source action is required"))?;
    let project = resolve_project(operations, selection)?;
    let work = ContextItemId::from_bytes(parse_identity(required(args, "work")?)?);
    let receipt = match action {
        "list" => return operations.explanation_source_manifest(project, work),
        "read" => {
            return operations.read_explanation_source(
                project,
                work,
                SourceId::from_bytes(parse_identity(required(args, "source")?)?),
            )
        }
        "delete" => {
            return Ok(
                json!({"deleted":operations.delete_work_source_observation(project, work,
            SourceId::from_bytes(parse_identity(required(args, "source")?)?))?}),
            )
        }
        "observe" | "run" => {
            let analysis = crate::AnalysisSnapshotId::from_hex(required(args, "analysis")?)
                .map_err(Error::new)?;
            let retain_seconds = *args
                .get_one::<u64>("retain-seconds")
                .ok_or_else(|| Error::new("retention is required"))?;
            if action == "observe" {
                operations.observe_work_file(crate::FileObservationRequest {
                    project_id: project,
                    work_item_id: work,
                    analysis_snapshot_id: analysis,
                    path: required(args, "path")?.into(),
                    before_source_id: args
                        .get_one::<String>("before-source")
                        .map(|s| parse_identity(s).map(SourceId::from_bytes))
                        .transpose()?,
                    byte_range: args.get_one::<usize>("start-byte").map(|start| {
                        crate::FileByteRange {
                            start_byte: *start,
                            end_byte: *args.get_one::<usize>("end-byte").unwrap_or(start),
                        }
                    }),
                    retain_seconds,
                })?
            } else {
                let mut command = args
                    .get_many::<String>("command")
                    .ok_or_else(|| Error::new("command is required"))?;
                let program = command
                    .next()
                    .ok_or_else(|| Error::new("program is required"))?
                    .clone();
                operations.run_work_command(crate::WorkCommandRequest {
                    project_id: project,
                    work_item_id: work,
                    analysis_snapshot_id: analysis,
                    command_label: required(args, "label")?.into(),
                    program,
                    arguments: command.cloned().collect(),
                    retain_seconds,
                    timeout: std::time::Duration::from_secs(
                        *args
                            .get_one::<u64>("timeout-seconds")
                            .ok_or_else(|| Error::new("timeout is required"))?,
                    ),
                })?
            }
        }
        _ => return Err(Error::new("unsupported Work Source action")),
    };
    Ok(
        json!({"operation":"work_source_capture", "source_id":receipt.source_id.to_string(),
        "observation":receipt, "read_command":format!("volicord work source read --work {work} --source {}", receipt.source_id)}),
    )
}

fn repository_command() -> Command {
    let mut command = explanation_command("repository");
    // Keep one explanation shape; add only repository-specific investigation.
    command = command.mut_subcommand("explain", |explain| explain
        .subcommand(Command::new("read").about("Read the latest retained repository explanation; never generates").arg(reading_language()))
        .subcommand(Command::new("evidence").about("Inspect full exact-snapshot metadata and diagnostics; no Source bodies").arg(Arg::new("analysis").long("analysis").required(true)))
        .subcommand(Command::new("compose").about("Derive exact citations from host-authored units and selected Source ranges; does not retain")
            .arg(Arg::new("analysis").long("analysis").required(true))
            .arg(path_arg("input", "input", "Host-authored draft JSON").required(true))));
    let analysis = || Arg::new("analysis").long("analysis").required(true);
    let source = || Arg::new("source").long("source").required(true);
    command.subcommand(
        Command::new("source")
            .about("Explicit bounded repository Source observations")
            .subcommand_required(true)
            .subcommand(
                Command::new("observe")
                    .arg(analysis())
                    .arg(Arg::new("path").long("path").required(true))
                    .arg(
                        Arg::new("retain-seconds")
                            .long("retain-seconds")
                            .required(true)
                            .value_parser(clap::value_parser!(u64)),
                    )
                    .arg(
                        Arg::new("start-byte")
                            .long("start-byte")
                            .requires("end-byte")
                            .value_parser(clap::value_parser!(usize)),
                    )
                    .arg(
                        Arg::new("end-byte")
                            .long("end-byte")
                            .requires("start-byte")
                            .value_parser(clap::value_parser!(usize)),
                    ),
            )
            .subcommand(Command::new("read").arg(analysis()).arg(source()))
            .subcommand(Command::new("delete").arg(analysis()).arg(source())),
    )
}
fn read_explanation_input(args: &ArgMatches) -> Result<Value, Error> {
    let file = std::fs::File::open(required_path(args, "input")?)
        .map_err(|e| Error::with_source("cannot open host input", e))?;
    let mut bytes = Vec::new();
    file.take(volicord_projections::EXPLANATION_INPUT_BYTE_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| Error::with_source("cannot read host input", e))?;
    if bytes.len() > volicord_projections::EXPLANATION_INPUT_BYTE_LIMIT {
        return Err(Error::new(
            "host input exceeds 65536 UTF-8 bytes; reduce the draft and preserve grounding",
        ));
    }
    serde_json::from_slice(&bytes).map_err(|_| Error::new("invalid host input JSON"))
}
fn dispatch_repository(
    operations: &LocalOperations,
    selection: &ProjectSelection,
    args: &ArgMatches,
) -> Result<Value, Error> {
    let project = resolve_project(operations, selection)?;
    let (action, args) = args
        .subcommand()
        .ok_or_else(|| Error::new("repository action required"))?;
    if action == "explain" {
        let (action, params) = args
            .subcommand()
            .ok_or_else(|| Error::new("explain action required"))?;
        if action == "read" {
            return Ok(
                json!({"operation":"repository_explanation_read","answers":operations.read_repository_explanation(project,required(params,"language")?)?}),
            );
        }
        if action == "evidence" {
            let analysis = crate::AnalysisSnapshotId::from_hex(required(params, "analysis")?)
                .map_err(Error::new)?;
            return operations.repository_explanation_evidence(project, analysis);
        }
        if action == "compose" {
            let analysis = crate::AnalysisSnapshotId::from_hex(required(params, "analysis")?)
                .map_err(Error::new)?;
            let draft = serde_json::from_value(read_explanation_input(params)?)
                .map_err(|_| Error::new("invalid repository reader draft"))?;
            return Ok(
                json!({"operation":"repository_explanation_compose","realization":operations.compose_repository_explanation(project,analysis,draft)?}),
            );
        }
        return dispatch_explanation_action(operations, selection, "repository", args);
    }
    let (action, params) = args
        .subcommand()
        .ok_or_else(|| Error::new("Source action required"))?;
    let analysis =
        crate::AnalysisSnapshotId::from_hex(required(params, "analysis")?).map_err(Error::new)?;
    match action {
        "observe" => Ok(
            json!({"operation":"repository_source_observe","observation":operations.observe_repository_file(crate::RepositoryFileObservationRequest {
            project_id:project,analysis_snapshot_id:analysis,path:required(params,"path")?.into(),retain_seconds:*params.get_one::<u64>("retain-seconds").ok_or_else(||Error::new("retention required"))?,
            byte_range:params.get_one::<usize>("start-byte").map(|start|crate::FileByteRange{start_byte:*start,end_byte:*params.get_one::<usize>("end-byte").unwrap_or(start)})
        })?}),
        ),
        "read" => Ok(operations.read_repository_explanation_source(
            project,
            analysis,
            SourceId::from_bytes(parse_identity(required(params, "source")?)?),
        )?),
        "delete" => Ok(
            json!({"deleted":operations.delete_repository_source_observation(project,analysis,SourceId::from_bytes(parse_identity(required(params,"source")?)?))?}),
        ),
        _ => Err(Error::new("unsupported repository Source action")),
    }
}

fn document_command() -> Command {
    let kind = || {
        Arg::new("kind")
            .value_name("KIND")
            .required(true)
            .value_parser([
                "project-architecture-guide",
                "decision-report",
                "implementation-plan",
                "handoff-resume",
            ])
    };
    let format = || {
        Arg::new("format")
            .long("format")
            .value_name("FORMAT")
            .value_parser(["markdown", "html"])
            .default_value("markdown")
    };
    let language = || {
        Arg::new("language")
            .long("language")
            .value_name("LANGUAGE")
            .default_value("en")
    };
    Command::new("document")
        .about("Preview or export source-grounded documents")
        .subcommand_required(true)
        .subcommand(
            Command::new("preview")
                .about("Preview a generated document")
                .arg(kind())
                .arg(format())
                .arg(language()),
        )
        .subcommand(
            Command::new("export")
                .about("Export a generated document to an explicit destination")
                .arg(kind())
                .arg(format())
                .arg(language())
                .arg(path_arg("output", "output", "Absolute output path").required(true)),
        )
}

fn viewer_command() -> Command {
    Command::new("viewer")
        .about("Open or locate the local Project Understanding Viewer")
        .subcommand_required(true)
        .subcommand(Command::new("locate").about("Show the Viewer executable path"))
        .subcommand(
            Command::new("open")
                .about("Run the local Viewer until interrupted")
                .arg(Arg::new("bind").long("bind").value_name("LOOPBACK_ADDRESS"))
                .arg(viewer_view())
                .arg(viewer_language()),
        )
        .subcommand(
            Command::new("export")
                .about("Export a self-contained read-only Viewer snapshot")
                .arg(path_arg("output", "output", "Absolute snapshot destination").required(true))
                .arg(viewer_language()),
        )
}

fn viewer_view() -> Arg {
    Arg::new("view")
        .long("view")
        .value_name("VIEW")
        .value_parser(["overview", "work", "code", "decisions", "tools"])
        .default_value("overview")
}

fn viewer_language() -> Arg {
    Arg::new("language")
        .long("language")
        .value_name("LANGUAGE")
        .default_value("en")
}

fn context_command() -> Command {
    Command::new("context")
        .about("Export, import, compare, and merge portable context")
        .subcommand_required(true)
        .subcommand(
            Command::new("export")
                .about("Export the current Project's portable context")
                .arg(path_arg("output", "output", "Absolute bundle destination").required(true)),
        )
        .subcommand(
            Command::new("import")
                .about("Import portable context")
                .arg(path_arg("input", "input", "Absolute bundle source").required(true)),
        )
        .subcommand(
            Command::new("compare")
                .about("Compare an incoming portable bundle")
                .arg(path_arg("input", "input", "Absolute incoming bundle").required(true))
                .arg(path_arg("base", "base", "Absolute common-base bundle")),
        )
        .subcommand(
            Command::new("merge")
                .about("Automatically merge only conflict-free portable context")
                .arg(path_arg("input", "input", "Absolute incoming bundle").required(true))
                .arg(path_arg("base", "base", "Absolute common-base bundle")),
        )
        .subcommand(
            Command::new("resolve")
                .about("Apply an explicit user resolution to a portable conflict")
                .arg(path_arg("input", "input", "Absolute incoming bundle").required(true))
                .arg(
                    Arg::new("conflict_set")
                        .long("conflict-set")
                        .required(true)
                        .value_name("IDENTITY"),
                )
                .arg(
                    Arg::new("revision")
                        .long("revision")
                        .required(true)
                        .value_name("REVISION"),
                )
                .arg(
                    Arg::new("source")
                        .long("source")
                        .required(true)
                        .value_name("SOURCE_ID"),
                )
                .arg(Arg::new("mode").long("mode").required(true).value_parser([
                    "choose-local",
                    "choose-incoming",
                    "context-branch",
                    "explicit-merged",
                ]))
                .arg(path_arg("base", "base", "Absolute common-base bundle"))
                .arg(path_arg(
                    "merged_bundle",
                    "merged-bundle",
                    "Absolute explicitly merged bundle",
                )),
        )
}

fn privacy_command() -> Command {
    Command::new("privacy")
        .about("Inspect or change background provider authorization")
        .subcommand_required(true)
        .subcommand(
            Command::new("status")
                .arg(reading_language())
                .about("Show local privacy and provider configuration"),
        )
        .subcommand(
            Command::new("enable")
                .about("Enable a provider for explicit source scopes")
                .arg(Arg::new("provider").required(true))
                .arg(Arg::new("model").required(true))
                .arg(
                    Arg::new("source")
                        .long("source")
                        .required(true)
                        .value_name("SOURCE_ID"),
                )
                .arg(
                    repeat_arg("scope", "SCOPE", "Authorize an exact source scope").required(true),
                ),
        )
        .subcommand(
            Command::new("disable")
                .about("Disable provider use without revoking its policy history")
                .arg(
                    Arg::new("source")
                        .long("source")
                        .required(true)
                        .value_name("SOURCE_ID"),
                ),
        )
        .subcommand(
            Command::new("revoke")
                .about("Revoke provider authorization")
                .arg(
                    Arg::new("source")
                        .long("source")
                        .required(true)
                        .value_name("SOURCE_ID"),
                ),
        )
}

fn doctor_command() -> Command {
    Command::new("doctor")
        .about("Inspect health and recover rebuildable state")
        .subcommand_required(true)
        .subcommand(Command::new("check").about("Check runtime and current Project health"))
        .subcommand(
            Command::new("repair")
                .about("Repair derived analysis or an interrupted forgetting operation")
                .arg(
                    Arg::new("forgetting")
                        .long("forgetting")
                        .value_name("OPERATION_ID"),
                )
                .arg(repeat_arg(
                    "exclude",
                    "PATH",
                    "Exclude a repository-relative path",
                )),
        )
        .subcommand(
            Command::new("reindex")
                .about("Discard and rebuild the current Project's derived index")
                .arg(repeat_arg(
                    "exclude",
                    "PATH",
                    "Exclude a repository-relative path",
                )),
        )
}

fn codex_command() -> Command {
    Command::new("codex")
        .about("Manage the repository-scoped Codex integration")
        .subcommand_required(true)
        .subcommand(Command::new("enable").about("Enable Volicord for a trusted repository"))
        .subcommand(
            Command::new("disable").about("Remove only Volicord-owned repository integration"),
        )
        .subcommand(Command::new("hook").hide(true))
}

fn advanced_command() -> Command {
    Command::new("advanced")
        .about("Audit, canonical-memory, checkpoint, and Guarded fallback operations")
        .subcommand_required(true)
        .subcommand(records_command())
        .subcommand(Command::new("candidates").about("Inspect bounded Session Candidate metadata"))
        .subcommand(
            Command::new("checkpoint")
                .about("Record a source-grounded Checkpoint")
                .arg(Arg::new("kind").required(true).value_parser([
                    "completion",
                    "pause",
                    "handoff",
                ]))
                .arg(Arg::new("source").long("source").required(true))
                .arg(Arg::new("work_item").long("work-item").required(true))
                .arg(Arg::new("next_step").long("next-step").required(true))
                .arg(Arg::new("handoff_to").long("handoff-to")),
        )
        .subcommand(guarded_command())
}

fn records_command() -> Command {
    Command::new("records")
        .about("Inspect or explicitly maintain canonical records")
        .subcommand_required(true)
        .subcommand(Command::new("list").about("Inspect canonical records"))
        .subcommand(
            Command::new("source")
                .about("Record a current-host user Source")
                .arg(Arg::new("host").long("host").required(true))
                .arg(Arg::new("session").long("session").required(true))
                .arg(Arg::new("text").long("text").required(true)),
        )
        .subcommand(record_change(
            "correct-context",
            "Correct Context Item wording",
        ))
        .subcommand(record_change(
            "correct-decision",
            "Correct Decision rationale wording",
        ))
        .subcommand(
            Command::new("supersede-decision")
                .about("Supersede a Decision with an explicit user choice")
                .arg(Arg::new("identity").required(true))
                .arg(Arg::new("source").long("source").required(true))
                .arg(Arg::new("alternative").long("alternative").required(true))
                .arg(Arg::new("rationale").long("rationale")),
        )
        .subcommand(
            Command::new("forget")
                .about("Forget one canonical record with user authorization")
                .arg(Arg::new("kind").required(true).value_parser([
                    "source",
                    "question",
                    "decision",
                    "context_item",
                    "checkpoint",
                ]))
                .arg(Arg::new("identity").required(true))
                .arg(Arg::new("source").long("source").required(true)),
        )
}

fn record_change(name: &'static str, about: &'static str) -> Command {
    Command::new(name)
        .about(about)
        .arg(Arg::new("identity").required(true))
        .arg(Arg::new("revision").long("revision").required(true))
        .arg(Arg::new("source").long("source").required(true))
        .arg(Arg::new("text").long("text").required(true))
}

fn guarded_command() -> Command {
    let response = |name: &'static str, about: &'static str| {
        Command::new(name)
            .about(about)
            .arg(Arg::new("request").required(true))
            .arg(Arg::new("revision").long("revision").required(true))
            .arg(Arg::new("fingerprint").long("fingerprint").required(true))
            .arg(Arg::new("host").long("host").required(true))
            .arg(Arg::new("session").long("session").required(true))
            .arg(Arg::new("response").long("response").required(true))
    };
    Command::new("guarded")
        .about("Use the exact-match CLI fallback for a high-risk effect")
        .subcommand_required(true)
        .subcommand(
            Command::new("request")
                .about("Create a Guarded Effect Candidate")
                .arg(Arg::new("category").required(true).value_parser([
                    "destructive-delete",
                    "migration",
                    "external-publication",
                    "cost",
                    "credential",
                    "external-source-transmission",
                    "external-message",
                    "production-data",
                    "security-setting",
                ]))
                .arg(Arg::new("action").long("action").required(true))
                .arg(Arg::new("target").long("target").required(true))
                .arg(Arg::new("effect").long("effect").required(true))
                .arg(Arg::new("risk").long("risk").required(true))
                .arg(Arg::new("expires").long("expires").required(true))
                .arg(
                    repeat_arg("scope", "SCOPE", "Bind the request to an exact scope")
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("show")
                .about("Inspect an exact Guarded request")
                .arg(Arg::new("request").required(true)),
        )
        .subcommand(response("confirm", "Confirm an exact Guarded request"))
        .subcommand(response("deny", "Deny an exact Guarded request"))
}

fn dispatch(
    name: &str,
    matches: &ArgMatches,
    operations: &LocalOperations,
    runtime: &RuntimeLayout,
    selection: &ProjectSelection,
    input: &mut dyn Read,
) -> Result<Option<Value>, Error> {
    let value = match name {
        "init" => {
            let repository = if matches.get_flag("no_bind") {
                None
            } else {
                Some(repository_path(selection)?)
            };
            let name = matches
                .get_one::<String>("name")
                .cloned()
                .or_else(|| {
                    repository
                        .as_ref()
                        .and_then(|path| path.file_name())
                        .and_then(OsStr::to_str)
                        .map(str::to_owned)
                })
                .unwrap_or_else(|| "Volicord Project".to_owned());
            let mut cursor = cursor(["init", name.as_str()]);
            if let Some(repository) = repository {
                cursor.push("--repository");
                cursor.push(path_text(&repository)?);
            }
            project(operations, &mut cursor)?
        }
        "bind" => {
            let selected_project = selection.explicit.ok_or_else(|| Error::new("bind requires --project PROJECT_ID; portable Project identity cannot be inferred from a new clone"))?;
            let repository = repository_path(selection)?;
            let mut cursor = cursor([
                "bind",
                &selected_project.to_string(),
                path_text(&repository)?,
            ]);
            if let Some(revision) = matches.get_one::<String>("revision") {
                cursor.push("--revision");
                cursor.push(revision);
            }
            project(operations, &mut cursor)?
        }
        "status" => {
            let (language, locale) = reading_options(matches);
            status(
                operations,
                resolve_project(operations, selection)?,
                language,
                locale,
            )?
        }
        "analyze" => {
            let project = resolve_project(operations, selection)?;
            let mut cursor = cursor([project.to_string()]);
            append_many(&mut cursor, matches, "exclude", "--exclude");
            analyze(operations, &mut cursor, false)?
        }
        "recall" => {
            let mut cursor = cursor([resolve_project(operations, selection)?.to_string()]);
            {
                let (language, locale) = reading_options(matches);
                recall(operations, &mut cursor, language, locale)?
            }
        }
        "questions" => {
            let mut cursor = cursor([
                "frontier",
                &resolve_project(operations, selection)?.to_string(),
            ]);
            append_values(&mut cursor, matches, "scope");
            inquiry(operations, &mut cursor)?
        }
        "decisions" => {
            let (language, locale) = reading_options(matches);
            decisions(
                operations,
                resolve_project(operations, selection)?,
                language,
                locale,
            )?
        }
        "repository" => dispatch_repository(operations, selection, matches)?,
        "work" | "decision" => dispatch_explanation(operations, selection, name, matches)?,
        "document" => dispatch_document(operations, selection, matches)?,
        "viewer" => return dispatch_viewer(runtime, operations, selection, matches),
        "context" => dispatch_context(operations, selection, matches)?,
        "privacy" => dispatch_privacy(operations, selection, matches)?,
        "doctor" => dispatch_doctor(operations, selection, matches)?,
        "advanced" => dispatch_advanced(operations, selection, matches)?,
        "codex" => {
            let (action, _) = matches
                .subcommand()
                .ok_or_else(|| Error::new("a Codex action is required"))?;
            let repository = repository_path(selection)?;
            let mut cursor = cursor([action, path_text(&repository)?]);
            let value = crate::codex::execute(runtime.clone(), &mut cursor, input)?;
            cursor.done()?;
            return Ok(value);
        }
        _ => return Err(Error::new("unsupported command")),
    };
    Ok(Some(value))
}

fn dispatch_explanation(
    operations: &LocalOperations,
    selection: &ProjectSelection,
    kind: &str,
    matches: &ArgMatches,
) -> Result<Value, Error> {
    if kind == "work" {
        if let Some(args) = matches.subcommand_matches("verification") {
            let project = resolve_project(operations, selection)?;
            let work = ContextItemId::from_bytes(parse_identity(required(args, "work")?)?);
            return Ok(
                json!({"operation":"work_verification_read","project_id":project.to_string(),"work_item_id":work.to_string(),"verification_report":operations.read_work_verification(project,work)?,"read_only":true}),
            );
        }
        if let Some(actions) = matches.subcommand_matches("source") {
            return dispatch_work_source(operations, selection, actions);
        }
    }
    let (_, actions) = matches
        .subcommand()
        .ok_or_else(|| Error::new("explain is required"))?;
    dispatch_explanation_action(operations, selection, kind, actions)
}
fn dispatch_explanation_action(
    operations: &LocalOperations,
    selection: &ProjectSelection,
    kind: &str,
    actions: &ArgMatches,
) -> Result<Value, Error> {
    let (action, args) = actions
        .subcommand()
        .ok_or_else(|| Error::new("an explanation action is required"))?;
    let project = resolve_project(operations, selection)?;
    let subject = match kind {
        "repository" => volicord_projections::ExplanationSubject::Repository(
            crate::AnalysisSnapshotId::from_hex(required(args, "analysis")?).map_err(Error::new)?,
        ),
        "work" => volicord_projections::ExplanationSubject::Work(ContextItemId::from_bytes(
            parse_identity(required(args, kind)?)?,
        )),
        _ => volicord_projections::ExplanationSubject::Decision(
            volicord_context::DecisionId::from_bytes(parse_identity(required(args, kind)?)?),
        ),
    };
    match action {
        "compose" => {
            let draft = serde_json::from_value(read_explanation_input(args)?)
                .map_err(|_| Error::new("invalid host-authored article draft"))?;
            let realization = match subject {
                volicord_projections::ExplanationSubject::Work(work) => {
                    operations.compose_work_explanation(project, work, draft)?
                }
                volicord_projections::ExplanationSubject::Decision(decision) => {
                    operations.compose_decision_explanation(project, decision, draft)?
                }
                _ => return Err(Error::new("Work or Decision subject required")),
            };
            Ok(
                json!({"operation":format!("{kind}_explanation_compose"), "realization":realization}),
            )
        }
        "prepare" => {
            let plan =
                operations.prepare_explanation(project, subject, required(args, "language")?)?;
            let inventory = match subject {
                volicord_projections::ExplanationSubject::Work(work) => {
                    operations.explanation_source_manifest(project, work)?
                }
                _ => json!({"scope":"recorded_decision_only","body_authority":"not_granted"}),
            };
            Ok(json!({"operation":"explanation_prepare", "plan":plan,"source_inventory":inventory}))
        }
        "record" => {
            let file = std::fs::File::open(required_path(args, "input")?)
                .map_err(|e| Error::with_source("cannot open generated response", e))?;
            let mut bytes = Vec::new();
            file.take(volicord_projections::EXPLANATION_INPUT_BYTE_LIMIT as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| Error::with_source("cannot read generated response", e))?;
            if bytes.len() > volicord_projections::EXPLANATION_INPUT_BYTE_LIMIT {
                return Err(Error::new(format!("explanation input file exceeds {} UTF-8 bytes (read at least {}); remove formatting whitespace or reduce response prose/metadata, preserve grounding, then retry; compact realization limit remains {} bytes",
                    volicord_projections::EXPLANATION_INPUT_BYTE_LIMIT, bytes.len(), volicord_projections::EXPLANATION_BYTE_LIMIT)));
            }
            let header: Value = serde_json::from_slice(&bytes)
                .map_err(|e| Error::with_source("invalid generated response JSON", e))?;
            if header["format_kind"] != volicord_projections::EXPLANATION_KIND
                || header["format_version"] != volicord_projections::EXPLANATION_VERSION
            {
                return Err(Error::new(
                    "unsupported explanation format; regenerate from current preparation",
                ));
            }
            let response = serde_json::from_value(header)
                .map_err(|e| Error::with_source("invalid explanation realization", e))?;
            Ok(
                json!({"operation":"explanation_record","explanation":operations.record_explanation(project,subject,required(args,"language")?,response)?}),
            )
        }
        "delete" => Ok(
            json!({"operation":"explanation_delete", "deleted":operations.delete_explanations(project,subject)?}),
        ),
        _ => Err(Error::new("unsupported explanation action")),
    }
}

fn dispatch_document(
    operations: &LocalOperations,
    selection: &ProjectSelection,
    matches: &ArgMatches,
) -> Result<Value, Error> {
    let (action, args) = matches
        .subcommand()
        .ok_or_else(|| Error::new("a document action is required"))?;
    let project = resolve_project(operations, selection)?;
    let kind = required(args, "kind")?;
    let format = required(args, "format")?;
    let mut values = vec![
        action.to_owned(),
        project.to_string(),
        kind.to_owned(),
        format.to_owned(),
    ];
    if action == "export" {
        values.push(path_text(required_path(args, "output")?)?.to_owned());
    }
    values.push(required(args, "language")?.to_owned());
    let mut cursor = Cursor::from_strings(values);
    let fixed_locale = if matches
        .get_one::<String>("locale")
        .is_some_and(|value| value == "ko")
    {
        FixedLocale::Korean
    } else {
        FixedLocale::English
    };
    documents(operations, &mut cursor, fixed_locale)
}

fn dispatch_context(
    operations: &LocalOperations,
    selection: &ProjectSelection,
    matches: &ArgMatches,
) -> Result<Value, Error> {
    let (action, args) = matches
        .subcommand()
        .ok_or_else(|| Error::new("a context action is required"))?;
    let mut values = vec![action.to_owned()];
    match action {
        "export" => {
            values.push(resolve_project(operations, selection)?.to_string());
            values.push(path_text(required_path(args, "output")?)?.to_owned());
        }
        "import" => values.push(path_text(required_path(args, "input")?)?.to_owned()),
        "compare" | "merge" => {
            values.push(path_text(required_path(args, "input")?)?.to_owned());
            push_optional_path(&mut values, args, "base", "--base")?;
        }
        "resolve" => {
            values.push(path_text(required_path(args, "input")?)?.to_owned());
            values.push(required(args, "conflict_set")?.to_owned());
            values.push(required(args, "revision")?.to_owned());
            values.push(required(args, "source")?.to_owned());
            values.push(required(args, "mode")?.to_owned());
            push_optional_path(&mut values, args, "base", "--base")?;
            push_optional_path(&mut values, args, "merged_bundle", "--merged-bundle")?;
        }
        _ => return Err(Error::new("unsupported context action")),
    }
    let mut cursor = Cursor::from_strings(values);
    portable(operations, &mut cursor)
}

fn dispatch_privacy(
    operations: &LocalOperations,
    selection: &ProjectSelection,
    matches: &ArgMatches,
) -> Result<Value, Error> {
    let (action, args) = matches
        .subcommand()
        .ok_or_else(|| Error::new("a privacy action is required"))?;
    let project = resolve_project(operations, selection)?;
    let mut values = vec![action.to_owned(), project.to_string()];
    match action {
        "status" => {}
        "enable" => {
            values.push(required(args, "provider")?.to_owned());
            values.push(required(args, "model")?.to_owned());
            values.push(required(args, "source")?.to_owned());
            values.extend(many(args, "scope"));
        }
        "disable" | "revoke" => values.push(required(args, "source")?.to_owned()),
        _ => return Err(Error::new("unsupported privacy action")),
    }
    let mut cursor = Cursor::from_strings(values);
    privacy(operations, &mut cursor)
}

fn dispatch_doctor(
    operations: &LocalOperations,
    selection: &ProjectSelection,
    matches: &ArgMatches,
) -> Result<Value, Error> {
    let (action, args) = matches
        .subcommand()
        .ok_or_else(|| Error::new("a doctor action is required"))?;
    match action {
        "check" => {
            let project = resolve_project_optional(operations, selection)?;
            let mut cursor =
                Cursor::from_strings(project.map(|id| vec![id.to_string()]).unwrap_or_default());
            health(operations, &mut cursor)
        }
        "repair" => {
            let project = resolve_project(operations, selection)?;
            let mut values = vec![project.to_string()];
            if let Some(operation) = args.get_one::<String>("forgetting") {
                values.extend(["forgetting".to_owned(), operation.to_owned()]);
            } else {
                values.push("derived-analysis".to_owned());
                append_many_strings(&mut values, args, "exclude", "--exclude");
            }
            let mut cursor = Cursor::from_strings(values);
            repair(operations, &mut cursor)
        }
        "reindex" => {
            let mut cursor = cursor([resolve_project(operations, selection)?.to_string()]);
            append_many(&mut cursor, args, "exclude", "--exclude");
            reindex(operations, &mut cursor)
        }
        _ => Err(Error::new("unsupported doctor action")),
    }
}

fn dispatch_advanced(
    operations: &LocalOperations,
    selection: &ProjectSelection,
    matches: &ArgMatches,
) -> Result<Value, Error> {
    let (group, args) = matches
        .subcommand()
        .ok_or_else(|| Error::new("an advanced action is required"))?;
    let project = resolve_project(operations, selection)?;
    match group {
        "records" => dispatch_records(operations, project, args),
        "candidates" => {
            let mut cursor = cursor([project.to_string()]);
            candidates(operations, &mut cursor)
        }
        "checkpoint" => {
            let kind = required(args, "kind")?;
            let mut values = vec![
                "record".to_owned(),
                project.to_string(),
                kind.to_owned(),
                required(args, "source")?.to_owned(),
                required(args, "work_item")?.to_owned(),
                required(args, "next_step")?.to_owned(),
            ];
            if let Some(target) = args.get_one::<String>("handoff_to") {
                values.push(target.clone());
            }
            let mut cursor = Cursor::from_strings(values);
            checkpoint(operations, &mut cursor)
        }
        "guarded" => dispatch_guarded(operations, project, args),
        _ => Err(Error::new("unsupported advanced action")),
    }
}

fn dispatch_records(
    operations: &LocalOperations,
    project: ProjectId,
    matches: &ArgMatches,
) -> Result<Value, Error> {
    let (action, args) = matches
        .subcommand()
        .ok_or_else(|| Error::new("a record action is required"))?;
    let mut values = vec![action.to_owned(), project.to_string()];
    match action {
        "list" => values[0] = "inspect".to_owned(),
        "source" => {
            values[0] = "user-source".to_owned();
            values.extend([
                required(args, "host")?.to_owned(),
                required(args, "session")?.to_owned(),
                required(args, "text")?.to_owned(),
            ]);
        }
        "correct-context" | "correct-decision" => values.extend([
            required(args, "identity")?.to_owned(),
            required(args, "revision")?.to_owned(),
            required(args, "source")?.to_owned(),
            required(args, "text")?.to_owned(),
        ]),
        "supersede-decision" => {
            values.extend([
                required(args, "identity")?.to_owned(),
                required(args, "source")?.to_owned(),
                required(args, "alternative")?.to_owned(),
            ]);
            if let Some(rationale) = args.get_one::<String>("rationale") {
                values.push(rationale.clone());
            }
        }
        "forget" => values.extend([
            required(args, "kind")?.to_owned(),
            required(args, "identity")?.to_owned(),
            required(args, "source")?.to_owned(),
        ]),
        _ => return Err(Error::new("unsupported record action")),
    }
    let mut cursor = Cursor::from_strings(values);
    canonical(operations, &mut cursor)
}

fn dispatch_guarded(
    operations: &LocalOperations,
    project: ProjectId,
    matches: &ArgMatches,
) -> Result<Value, Error> {
    let (action, args) = matches
        .subcommand()
        .ok_or_else(|| Error::new("a Guarded action is required"))?;
    let mut values = vec![action.to_owned()];
    match action {
        "request" => {
            values.extend([
                project.to_string(),
                required(args, "category")?.to_owned(),
                required(args, "action")?.to_owned(),
                required(args, "target")?.to_owned(),
                required(args, "effect")?.to_owned(),
                required(args, "risk")?.to_owned(),
                required(args, "expires")?.to_owned(),
            ]);
            values.extend(many(args, "scope"));
        }
        "show" => values.push(required(args, "request")?.to_owned()),
        "confirm" | "deny" => values.extend([
            required(args, "request")?.to_owned(),
            required(args, "revision")?.to_owned(),
            required(args, "fingerprint")?.to_owned(),
            required(args, "host")?.to_owned(),
            required(args, "session")?.to_owned(),
            required(args, "response")?.to_owned(),
        ]),
        _ => return Err(Error::new("unsupported Guarded action")),
    }
    let mut cursor = Cursor::from_strings(values);
    guarded(operations, &mut cursor)
}

fn repository_path(selection: &ProjectSelection) -> Result<PathBuf, Error> {
    match &selection.repository {
        Some(path) => Ok(path.clone()),
        None => env::current_dir().map_err(|error| {
            Error::with_source("cannot determine the current repository path", error)
        }),
    }
}

fn resolve_project(
    operations: &LocalOperations,
    selection: &ProjectSelection,
) -> Result<ProjectId, Error> {
    if let Some(project) = selection.explicit {
        return Ok(project);
    }
    let repository = repository_path(selection)?;
    match operations.resolve_project(&repository)? {
        crate::ProjectResolution::Found { project, .. } => Ok(project.id),
        crate::ProjectResolution::NotFound { canonical_repository_path } => Err(Error::new(format!(
            "no Project is bound to {}; run 'volicord init' here, or use '--project PROJECT_ID' when selecting an existing Project",
            canonical_repository_path.display()
        ))),
    }
}

fn resolve_project_optional(
    operations: &LocalOperations,
    selection: &ProjectSelection,
) -> Result<Option<ProjectId>, Error> {
    if selection.explicit.is_some() {
        return Ok(selection.explicit);
    }
    match operations.resolve_project(&repository_path(selection)?)? {
        crate::ProjectResolution::Found { project, .. } => Ok(Some(project.id)),
        crate::ProjectResolution::NotFound { .. } => Ok(None),
    }
}

fn status(
    operations: &LocalOperations,
    project: ProjectId,
    language: &str,
    locale: FixedLocale,
) -> Result<Value, Error> {
    let projection = operations.project_projection(project)?;
    let understanding = build_project_understanding(&projection, UnderstandingBound::default());
    let work_json =
        |w: &volicord_projections::UnderstandingWork| crate::work_reading_json(w, language, locale);
    let decision_json = |d: &volicord_projections::UnderstandingDecision| {
        crate::decision_reading_json(&d.decision, language, locale)
    };
    Ok(json!({
        "operation":"project_status",
        "repository_explanation":volicord_projections::explanation_answers(&projection.repository_explanations,language,locale),
        "candidate_inspection":crate::work_learning_inspection_json(&projection),
        "repository_analysis":understanding.repository_analysis,
        "project_id":understanding.project_id.to_string(),
        "project_name":understanding.project_name,
        "health":debug_name(understanding.health),
        "canonical_revision":understanding.canonical_revision,
        "project_purpose":understanding.project_purpose.into_iter().map(|item| json!({"statement":item.statement,"source_ids":item.source_basis.into_iter().map(|id| id.to_string()).collect::<Vec<_>>() })).collect::<Vec<_>>(),
        "selection":work_selection_json(understanding.selection),
        "selected_work":understanding.selected_work.as_ref().map(work_json),
        "work_category_counts": {
            "current":section_count_json(&understanding.work_overview.current),
            "completed":section_count_json(&understanding.work_overview.completed),
            "remaining":section_count_json(&understanding.work_overview.remaining),
            "next_steps":section_count_json(&understanding.work_overview.next_steps)
        },
        "current_work":understanding.current_work.iter().map(work_json).collect::<Vec<_>>(),
        "completed_work":understanding.completed_work.iter().map(work_json).collect::<Vec<_>>(),
        "remaining_work":understanding.remaining_work.iter().map(work_json).collect::<Vec<_>>(),
        "work_history":understanding.work_history.iter().map(work_json).collect::<Vec<_>>(),
        "unresolved_work_grouping":understanding.unresolved_work_grouping.iter().map(|gap| json!({"record_kind":gap.record_kind,"identity":gap.identity,"reason":gap.reason})).collect::<Vec<_>>(),
        "next_steps":understanding.work_overview.next_steps.items.iter().map(work_json).collect::<Vec<_>>(),
        "active_decisions":understanding.active_decisions.iter().map(decision_json).collect::<Vec<_>>(),
        "selected_work_decisions":understanding.selected_work_decisions.iter().map(decision_json).collect::<Vec<_>>(),
        "open_questions":understanding.open_questions.into_iter().map(|item| json!({"identity":item.question_id.to_string(),"revision":item.revision,"prompt":item.prompt,"on_frontier":item.on_current_frontier})).collect::<Vec<_>>(),
        "risks_assumptions_and_limits":understanding.risks_assumptions_and_limits.into_iter().map(|item| item.statement).chain(understanding.known_limits).collect::<Vec<_>>(),
        "architecture": {"components":understanding.architecture.components.len(),"relationships":understanding.architecture.relationships.len(),"gaps":understanding.architecture.gaps.into_iter().map(|gap| gap.reason).collect::<Vec<_>>()},
        "evidence": {"sources":understanding.evidence.sources.len(),"snapshots":understanding.evidence.snapshots.len(),"issues":understanding.evidence.issues.into_iter().map(|issue| issue.reason).collect::<Vec<_>>()},
        "omissions":understanding.omissions.into_iter().map(|item| json!({"section":item.section,"count":item.omitted_count})).collect::<Vec<_>>()
    }))
}

fn section_count_json(section: &volicord_projections::WorkSection) -> Value {
    json!({"complete":section.complete,"total":section.complete.then_some(section.total),
        "displayed":section.items.len(),"omitted":section.complete.then_some(section.omitted)})
}

fn decisions(
    operations: &LocalOperations,
    project: ProjectId,
    language: &str,
    locale: FixedLocale,
) -> Result<Value, Error> {
    let brief = operations.recall(project)?;
    Ok(
        json!({"operation":"decisions","project_id":project.to_string(),"project_name":brief.project_name,
        "decisions":brief.decisions.iter().map(|d|crate::decision_reading_json(d,language,locale)).collect::<Vec<_>>(),"omissions":brief.omitted_count}),
    )
}

fn dispatch_viewer(
    runtime: &RuntimeLayout,
    operations: &LocalOperations,
    selection: &ProjectSelection,
    matches: &ArgMatches,
) -> Result<Option<Value>, Error> {
    let (action, args) = matches
        .subcommand()
        .ok_or_else(|| Error::new("a Viewer action is required"))?;
    let executable = viewer_executable()?;
    if action == "locate" {
        return Ok(Some(json!({"operation":"viewer_locate","path":executable})));
    }
    let project = resolve_project(operations, selection)?;
    if !executable.is_file() {
        return Err(Error::new(format!(
            "Viewer executable was not found at {}; install volicord-viewer beside volicord or run 'volicord viewer locate'",
            executable.display()
        )));
    }
    let mut command = ProcessCommand::new(&executable);
    command
        .arg("--runtime")
        .arg(runtime.root())
        .arg("--project")
        .arg(project.to_string())
        .arg("--locale")
        .arg(
            matches
                .get_one::<String>("locale")
                .map(String::as_str)
                .unwrap_or("en"),
        )
        .arg("--language")
        .arg(required(args, "language")?);
    if action == "open" {
        command.arg("--view").arg(required(args, "view")?);
        if let Some(bind) = args.get_one::<String>("bind") {
            command.arg("--bind").arg(bind);
        }
    } else if action == "export" {
        command
            .arg("--snapshot")
            .arg(required_path(args, "output")?);
    } else {
        return Err(Error::new("unsupported Viewer action"));
    }
    let status = command
        .status()
        .map_err(|error| Error::with_source("cannot start the Viewer", error))?;
    if !status.success() {
        return Err(Error::new(format!("Viewer exited with {status}")));
    }
    Ok(Some(
        json!({"operation":format!("viewer_{action}"),"project_id":project.to_string(),"executable":executable,"outcome":"completed"}),
    ))
}

fn viewer_executable() -> Result<PathBuf, Error> {
    let executable = env::current_exe()
        .map_err(|error| Error::with_source("cannot locate the volicord executable", error))?;
    Ok(executable
        .parent()
        .ok_or_else(|| Error::new("the volicord executable has no parent directory"))?
        .join("volicord-viewer"))
}

fn render(value: &Value, mode: OutputMode, stdout: &mut dyn Write) -> Result<(), Error> {
    if mode.json {
        serde_json::to_writer_pretty(&mut *stdout, value)
            .map_err(|error| Error::with_source("cannot render CLI result", error))?;
        writeln!(stdout).map_err(|error| Error::with_source("cannot write CLI result", error))?;
        return Ok(());
    }
    if value.get("operation").and_then(Value::as_str) == Some("document_preview") {
        if let Some(content) = value.get("content").and_then(Value::as_str) {
            write!(stdout, "{content}")
                .map_err(|error| Error::with_source("cannot write document preview", error))?;
            if !content.ends_with('\n') {
                writeln!(stdout)
                    .map_err(|error| Error::with_source("cannot write document preview", error))?;
            }
            return Ok(());
        }
    }
    let operation = value
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or("result");
    writeln!(stdout, "{}", operation_title(operation, mode.locale))
        .map_err(|error| Error::with_source("cannot write CLI result", error))?;
    if matches!(operation, "analyze" | "analysis_rebuild") {
        return render_analysis_summary(value, mode.locale, stdout);
    }
    if operation == "work_verification_read" {
        return render_verification_report(stdout, &value["verification_report"], mode.locale);
    }
    if operation == "repository_explanation_read" {
        return render_repository_answer(stdout, &value["answers"], mode.locale);
    }
    // Explicit ordinary-reading fields; exact binding/audit stays in --json.
    // Question/result/state selection is owned by the shared answer projection.
    let reading_fields: Option<&[&str]> = match operation {
        "project_status" => Some(&[
            "repository_explanation",
            "project_name",
            "repository_analysis",
            "health",
            "project_purpose",
            "work_category_counts",
            "selected_work",
            "current_work",
            "completed_work",
            "remaining_work",
            "work_history",
            "unresolved_work_grouping",
            "next_steps",
            "active_decisions",
            "selected_work_decisions",
            "candidate_inspection",
            "open_questions",
            "risks_assumptions_and_limits",
            "architecture",
            "omissions",
        ]),
        "recall" => Some(&[
            "project_name",
            "project_purpose",
            "selected_work",
            "decisions",
            "open_questions",
            "risks_assumptions_and_limits",
            "declared_assumptions",
            "known_limits",
            "omissions",
            "omitted_count",
        ]),
        "decisions" => Some(&["project_name", "decisions", "omissions"]),
        _ => None,
    };
    if let Some(fields) = reading_fields {
        for key in fields {
            if let Some(field) = value.get(key) {
                if *key == "repository_explanation" {
                    render_repository_answer(stdout, field, mode.locale)?;
                    continue;
                }
                if [
                    "project_purpose",
                    "open_questions",
                    "risks_assumptions_and_limits",
                    "unresolved_work_grouping",
                    "omissions",
                ]
                .contains(key)
                {
                    // Active canonical quotation/prompt consumers, with exact
                    // identity and Source basis available in JSON inspection.
                    let values = field
                        .as_array()
                        .map(|values| {
                            values
                                .iter()
                                .map(|item| {
                                    item.get("statement")
                                        .or_else(|| item.get("prompt"))
                                        .or_else(|| item.get("expandable_basis"))
                                        .or_else(|| item.get("reason"))
                                        .unwrap_or(item)
                                        .clone()
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    render_field(stdout, key, &Value::Array(values), 0, mode.locale)?;
                } else {
                    render_field(stdout, key, field, 0, mode.locale)?;
                }
            }
        }
        write_line(
            stdout,
            match mode.locale {
                CliLocale::English => "Exact evidence and audit: use --json.",
                CliLocale::Korean => "정확한 근거와 감사 정보: --json으로 검사하세요.",
            },
        )?;
        return Ok(());
    }
    if let Some(object) = value.as_object() {
        for (key, field) in object {
            if key != "operation" {
                render_field(stdout, key, field, 0, mode.locale)?;
            }
        }
    }
    Ok(())
}

fn render_verification_report(
    stdout: &mut dyn Write,
    report: &Value,
    locale: CliLocale,
) -> Result<(), Error> {
    let ko = matches!(locale, CliLocale::Korean);
    write_line(
        stdout,
        if ko {
            "검증 보고 · 실행과 assertion은 별개"
        } else {
            "Verification report · process exit is separate from assertions"
        },
    )?;
    for check in report["checks"].as_array().into_iter().flatten().take(64) {
        write_line(
            stdout,
            format!(
                "  {}@{} verification[{}] · {} · {} · {}",
                check["original"]["checkpoint_id"].as_str().unwrap_or(""),
                check["original"]["checkpoint_revision"],
                check["original"]["verification_index"],
                check["reported_at_unix_micros"],
                check["reported_state"].as_str().unwrap_or("unknown"),
                check["applicability"].as_str().unwrap_or("unknown")
            ),
        )?;
        if let Some(quote) = check["original_outcome"].as_str() {
            write_line(
                stdout,
                format!(
                    "    {}: {quote}",
                    if ko {
                        "원래 보고 인용"
                    } else {
                        "Original report quotation"
                    }
                ),
            )?;
        }
    }
    for execution in report["executions"]
        .as_array()
        .into_iter()
        .flatten()
        .take(32)
    {
        write_line(
            stdout,
            format!(
                "  {} · Source {}@{} · {} · exit {} · {} · body {}",
                execution["command_label"].as_str().unwrap_or(""),
                execution["source_id"].as_str().unwrap_or(""),
                execution["source_revision"],
                execution["recorded_at_unix_micros"],
                execution["exit_code"],
                execution["termination"].as_str().unwrap_or("unknown"),
                execution["body_state"].as_str().unwrap_or("unknown")
            ),
        )?;
        if execution["observation"]["state"] == "observed" {
            let o = &execution["observation"]["value"];
            write_line(
                stdout,
                format!(
                    "    {} → {} · {} · analysis {} · {}",
                    o["started_at_unix_micros"],
                    o["ended_at_unix_micros"],
                    o["operation_id"].as_str().unwrap_or(""),
                    o["analysis_snapshot_id"].as_str().unwrap_or(""),
                    o["target_state"].as_str().unwrap_or("unknown")
                ),
            )?;
        }
    }
    let checks = report["checks"].as_array().map_or(0, Vec::len);
    let runs = report["executions"].as_array().map_or(0, Vec::len);
    write_line(
        stdout,
        format!(
            "{}: checks {checks} / executions {runs}; displayed {}/{}. {}",
            if ko { "전체" } else { "Total" },
            checks.min(64),
            runs.min(32),
            if ko {
                "테스트 ID·환경·현재 coverage·재검증 관계는 미관측입니다. 전체 원본: --json / canonical_inspect / work source read."
            } else {
                "Test IDs, environment, current coverage and retest relation are unobserved. Full originals: --json / canonical_inspect / work source read."
            }
        ),
    )
}

fn render_repository_answer(
    stdout: &mut dyn Write,
    answers: &Value,
    locale: CliLocale,
) -> Result<(), Error> {
    write_line(
        stdout,
        match locale {
            CliLocale::English => "Repository explanation · host interpretation",
            CliLocale::Korean => "저장소 설명 · 호스트 해석",
        },
    )?;
    if let Some(units) = answers["prose"].as_array() {
        for unit in units {
            write_line(stdout, unit["text"].as_str().unwrap_or(""))?;
            if let Some(citations) = unit["citations"].as_array() {
                for c in citations {
                    write_line(
                        stdout,
                        format!(
                            "  {}:{}–{}",
                            c["locator"].as_str().unwrap_or(""),
                            c["start_line"],
                            c["end_line"]
                        ),
                    )?;
                }
            }
        }
    }
    if let Some(diagnostic) = answers["diagnostic"].as_str() {
        write_line(stdout, diagnostic)?;
    }
    Ok(())
}

fn render_analysis_summary(
    value: &Value,
    locale: CliLocale,
    stdout: &mut dyn Write,
) -> Result<(), Error> {
    let label = |en, ko| match locale {
        CliLocale::English => en,
        CliLocale::Korean => ko,
    };
    let summary = &value["human_summary"];
    let word = |value: &Value| value.as_str().unwrap_or("unknown").to_owned();
    let language = |value: &Value| {
        if let Some(name) = value["name"].as_str() {
            return name.to_owned();
        }
        match value["kind"].as_str().unwrap_or("repository") {
            "rust" => "Rust",
            "python" => "Python",
            "java" => "Java",
            "java_script" => "JavaScript",
            "type_script" => "TypeScript",
            "c" => "C",
            "cpp" => "C++",
            "go" => "Go",
            other => other,
        }
        .to_owned()
    };
    write_line(
        stdout,
        format!(
            "{}: {}",
            label("Scope", "분석 범위"),
            value["repository_scope"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    )?;
    write_line(
        stdout,
        format!(
            "{}: {}",
            label("Operation", "실행 상태"),
            word(&value["state"])
        ),
    )?;
    write_line(
        stdout,
        format!(
            "{}: {} {}; {} {}; {} {}; {} {}",
            label("Results", "분석 결과"),
            summary["files"],
            label("included files", "포함 파일"),
            summary["entities"],
            label("structural entities", "구조 엔터티"),
            summary["relations"],
            label("stored relations", "저장 관계"),
            summary["excluded_areas"],
            label("excluded inventory areas", "제외 inventory 영역")
        ),
    )?;
    write_line(
        stdout,
        label("Coverage (static evidence)", "분석 범위 (정적 근거)"),
    )?;
    for row in summary["coverage"].as_array().into_iter().flatten() {
        write_line(
            stdout,
            format!(
                "  {}: {} {}; structural {}; semantic {}; {} {}; {} {}",
                language(&row["language"]),
                row["files"],
                label("files", "파일"),
                word(&row["structural"]),
                word(&row["semantic"]),
                row["entities"].as_u64().unwrap_or(0),
                label("entities", "엔터티"),
                row["relations"].as_u64().unwrap_or(0),
                label("semantic relations", "의미 관계")
            ),
        )?;
    }
    if summary["omitted_coverage_count"].as_u64().unwrap_or(0) > 0 {
        write_line(
            stdout,
            format!(
                "  {}: {}; --json",
                label("Additional language coverage", "추가 언어 분석 범위"),
                summary["omitted_coverage_count"]
            ),
        )?;
    }
    let counts = summary["limitation_counts"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| format!("{} {}", r["count"], word(&r["state"])))
        .collect::<Vec<_>>()
        .join(", ");
    write_line(
        stdout,
        format!(
            "{}: {}",
            label("Affected capability scopes", "영향받는 분석 영역"),
            if counts.is_empty() {
                label("none", "없음")
            } else {
                &counts
            }
        ),
    )?;
    for limit in summary["limitations"].as_array().into_iter().flatten() {
        let affected = limit["affected_areas"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        write_line(
            stdout,
            format!(
                "  {} / {} / {} / {}: {}. {} {}",
                language(&limit["language"]),
                word(&limit["area"]),
                word(&limit["capability"]),
                word(&limit["state"]),
                limit["reason"]
                    .as_str()
                    .unwrap_or(label("coverage limited", "분석 범위 제한")),
                limit["consequence"].as_str().unwrap_or(""),
                limit["usable_remainder"].as_str().unwrap_or("")
            ),
        )?;
        if !affected.is_empty() {
            write_line(
                stdout,
                format!(
                    "    {}: {} (+{}; --json)",
                    label("Affected areas", "영향 영역"),
                    affected,
                    limit["omitted_affected_area_count"]
                ),
            )?;
        }
    }
    if summary["omitted_limitation_count"].as_u64().unwrap_or(0) > 0 {
        write_line(
            stdout,
            format!(
                "  {}: {}; --json",
                label("Additional affected scopes", "추가 영향 영역"),
                summary["omitted_limitation_count"]
            ),
        )?;
    }
    write_line(stdout, label(
        "Next: use the covered results; inspect --json for complete diagnostics. Restore failed prerequisites and rerun volicord analyze after source changes. Unsupported capabilities require analyzer support.",
        "다음: 사용 가능한 결과를 활용하고 전체 진단은 --json으로 확인하세요. 실패 원인을 해결하거나 소스를 변경한 뒤 volicord analyze를 실행하세요. 지원하지 않는 기능은 analyzer 지원이 필요합니다."))
}

fn render_field(
    stdout: &mut dyn Write,
    key: &str,
    value: &Value,
    indent: usize,
    locale: CliLocale,
) -> Result<(), Error> {
    let padding = "  ".repeat(indent);
    if let Some(answers) = value.get("answers") {
        write_line(stdout, format_args!("{padding}{key}:"))?;
        if let Some(title) = value["reading_title"].as_str() {
            write_line(stdout, format_args!("{padding}  {title}"))?;
        }
        if answers
            .get("provenance")
            .is_some_and(|value| !value.is_null())
        {
            write_line(
                stdout,
                match locale {
                    CliLocale::English => {
                        "Host interpretation; grounded reports, not independent verification."
                    }
                    CliLocale::Korean => {
                        "호스트 해석입니다. 근거 있는 보고이며 독립 검증을 뜻하지 않습니다."
                    }
                },
            )?;
        }
        for (section, facts) in [("prose", false), ("facts", true)] {
            if facts {
                write_line(
                    stdout,
                    match locale {
                        CliLocale::English => "Recorded facts and derived states:",
                        CliLocale::Korean => "기록된 사실과 파생 상태:",
                    },
                )?;
            }
            for a in answers[section].as_array().into_iter().flatten() {
                if !facts && a["question"] == "Heading" && value["reading_title"].is_string() {
                    continue; // This authored heading was already printed above.
                }
                if let Some(text) = a["text"].as_str() {
                    write_line(stdout, format_args!("{padding}  {text}"))?;
                }
            }
        }
        for excerpt in answers["source_excerpts"].as_array().into_iter().flatten() {
            let c = &excerpt["citation"];
            write_line(
                stdout,
                format!(
                    "{padding}  {}:{}–{} · {} · {}",
                    c["locator"].as_str().unwrap_or_default(),
                    c["start_line"],
                    c["end_line"],
                    c["use_kind"].as_str().unwrap_or_default(),
                    excerpt["comparison"]
                        .as_str()
                        .unwrap_or("historical_observation")
                ),
            )?;
            if excerpt["source_representation"] == "bounded_excerpt" {
                write_line(stdout, match locale {
                    CliLocale::English => "Bounded file observation; surrounding code was not retained. Coordinates refer to the repository file.",
                    CliLocale::Korean => "제한된 파일 관측이며 주변 코드는 보존되지 않았습니다. 좌표는 저장소 파일 기준입니다.",
                })?;
            }
            if let Some(text) = excerpt["text"].as_str() {
                write_line(stdout, text)?;
            }
            if excerpt["omitted_utf8_bytes"].as_u64().unwrap_or(0) > 0 {
                write_line(
                    stdout,
                    format!(
                        "{padding}  {}: {}",
                        match locale {
                            CliLocale::English => "Omitted UTF-8 bytes",
                            CliLocale::Korean => "생략된 UTF-8 바이트",
                        },
                        excerpt["omitted_utf8_bytes"]
                    ),
                )?;
            }
        }
        if answers["omitted_source_excerpt_count"]
            .as_u64()
            .unwrap_or(0)
            > 0
        {
            write_line(
                stdout,
                format!(
                    "{padding}  {}: {}",
                    match locale {
                        CliLocale::English => "Omitted cited excerpts",
                        CliLocale::Korean => "생략된 인용 발췌",
                    },
                    answers["omitted_source_excerpt_count"]
                ),
            )?;
        }
        if let Some(report) = value.get("verification_report") {
            render_verification_report(stdout, report, locale)?;
        }
        return Ok(());
    }
    let label = field_label(key, locale);
    match value {
        Value::Null => write_line(stdout, format_args!("{padding}{label}: -")),
        Value::Bool(value) => write_line(stdout, format_args!("{padding}{label}: {value}")),
        Value::Number(value) => write_line(stdout, format_args!("{padding}{label}: {value}")),
        Value::String(value) => write_line(stdout, format_args!("{padding}{label}: {value}")),
        Value::Array(values) if values.is_empty() => {
            write_line(stdout, format_args!("{padding}{label}: -"))
        }
        Value::Array(values) => {
            write_line(stdout, format_args!("{padding}{label}:"))?;
            for item in values {
                match item {
                    Value::Object(fields) => {
                        write_line(stdout, format_args!("{padding}  -"))?;
                        if item.get("answers").is_some() {
                            render_field(stdout, key, item, indent + 2, locale)?;
                        } else {
                            for (child, value) in fields {
                                render_field(stdout, child, value, indent + 2, locale)?;
                            }
                        }
                    }
                    Value::String(value) => {
                        write_line(stdout, format_args!("{padding}  - {value}"))?
                    }
                    other => write_line(stdout, format_args!("{padding}  - {other}"))?,
                }
            }
            Ok(())
        }
        Value::Object(fields) => {
            write_line(stdout, format_args!("{padding}{label}:"))?;
            for (child, value) in fields {
                render_field(stdout, child, value, indent + 1, locale)?;
            }
            Ok(())
        }
    }
}

fn write_line(stdout: &mut dyn Write, args: impl std::fmt::Display) -> Result<(), Error> {
    stdout
        .write_fmt(format_args!("{args}"))
        .and_then(|()| stdout.write_all(b"\n"))
        .map_err(|error| Error::with_source("cannot write CLI result", error))
}

fn operation_title(operation: &str, locale: CliLocale) -> String {
    let english = match operation {
        "project_init" => "Project initialized",
        "project_bind" => "Project bound",
        "project_status" => "Project Understanding",
        "analyze" => "Repository analysis",
        "recall" => "Recall",
        "inquiry_frontier" => "Current Questions",
        "decisions" => "Decisions",
        "health" => "Volicord doctor",
        "privacy_status" => "Privacy and provider configuration",
        "viewer_locate" => "Viewer location",
        _ => operation,
    };
    match locale {
        CliLocale::English => english.to_owned(),
        CliLocale::Korean => match operation {
            "project_init" => "프로젝트를 초기화했습니다".to_owned(),
            "project_bind" => "프로젝트를 연결했습니다".to_owned(),
            "project_status" => "프로젝트 이해".to_owned(),
            "analyze" => "저장소 분석".to_owned(),
            "recall" => "리콜".to_owned(),
            "inquiry_frontier" => "현재 질문".to_owned(),
            "decisions" => "결정".to_owned(),
            "health" => "Volicord 진단".to_owned(),
            "privacy_status" => "개인정보 및 제공자 설정".to_owned(),
            "viewer_locate" => "뷰어 위치".to_owned(),
            _ => english.to_owned(),
        },
    }
}

fn field_label(key: &str, locale: CliLocale) -> String {
    let english = key.replace('_', " ");
    if matches!(locale, CliLocale::English) {
        return english;
    }
    match key {
        "project_name" => "프로젝트 이름".into(),
        "health" | "state" => "상태".into(),
        "goals" => "목표와 이유".into(),
        "current_work" => "현재 작업".into(),
        "completed_work" => "완료된 작업".into(),
        "remaining_work" => "남은 작업".into(),
        "next_steps" | "next_step" => "다음 단계".into(),
        "active_decisions" | "decisions" => "결정".into(),
        "open_questions" => "열린 질문".into(),
        "risks_assumptions_and_limits" => "위험, 가정 및 한계".into(),
        "architecture" => "아키텍처".into(),
        "evidence" => "근거".into(),
        "issues" => "문제".into(),
        "path" => "경로".into(),
        _ => english,
    }
}

fn required<'a>(matches: &'a ArgMatches, id: &str) -> Result<&'a str, Error> {
    matches
        .get_one::<String>(id)
        .map(String::as_str)
        .ok_or_else(|| Error::new(format!("missing required {id}")))
}

fn required_path<'a>(matches: &'a ArgMatches, id: &str) -> Result<&'a Path, Error> {
    matches
        .get_one::<PathBuf>(id)
        .map(PathBuf::as_path)
        .ok_or_else(|| Error::new(format!("missing required {id}")))
}

fn path_text(path: &Path) -> Result<&str, Error> {
    path.to_str()
        .ok_or_else(|| Error::new("path must be valid UTF-8"))
}

fn many(matches: &ArgMatches, id: &str) -> Vec<String> {
    matches
        .get_many::<String>(id)
        .map(|values| values.cloned().collect())
        .unwrap_or_default()
}

fn append_values(cursor: &mut Cursor, matches: &ArgMatches, id: &str) {
    for value in many(matches, id) {
        cursor.push(value);
    }
}

fn append_many(cursor: &mut Cursor, matches: &ArgMatches, id: &str, option: &str) {
    for value in many(matches, id) {
        cursor.push(option);
        cursor.push(value);
    }
}

fn append_many_strings(values: &mut Vec<String>, matches: &ArgMatches, id: &str, option: &str) {
    for value in many(matches, id) {
        values.push(option.to_owned());
        values.push(value);
    }
}

fn push_optional_path(
    values: &mut Vec<String>,
    matches: &ArgMatches,
    id: &str,
    option: &str,
) -> Result<(), Error> {
    if let Some(path) = matches.get_one::<PathBuf>(id) {
        values.push(option.to_owned());
        values.push(path_text(path)?.to_owned());
    }
    Ok(())
}

fn cursor<I, S>(values: I) -> Cursor
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Cursor::new(
        values
            .into_iter()
            .map(|value| value.as_ref().to_os_string())
            .collect(),
    )
}

pub(crate) fn usage(detail: &str) -> Error {
    Error::new(format!("usage: {USAGE}\n{detail}"))
}

fn project(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    match cursor.next("project command")?.as_str() {
        "init" => {
            let name = cursor.next("project display name")?;
            let repository = if cursor.peek("--repository") {
                cursor.next("--repository")?;
                Some(PathBuf::from(cursor.next("repository path")?))
            } else {
                None
            };
            let value = operations.initialize_project(name, repository.as_deref())?;
            Ok(json!({
                "operation": "project_init",
                "project_id": value.project.id.to_string(),
                "display_name": value.project.display_name,
                "revision": value.project.revision,
                "binding": value.binding.map(|binding| json!({"path": binding.binding.absolute_path, "revision": binding.binding.revision, "clone_identity": binding.clone_identity, "worktree_identity": binding.worktree_identity})),
            }))
        }
        "bind" => {
            let project = project_id(&cursor.next("Project ID")?)?;
            let path = PathBuf::from(cursor.next("repository path")?);
            let revision = if cursor.peek("--revision") {
                cursor.next("--revision")?;
                Some(number(&cursor.next("binding revision")?)?)
            } else {
                None
            };
            let value = operations.bind_project(project, revision, &path)?;
            Ok(
                json!({"operation":"project_bind", "project_id":project.to_string(), "path":value.binding.absolute_path, "revision":value.binding.revision, "clone_identity":value.clone_identity, "worktree_identity":value.worktree_identity}),
            )
        }
        _ => Err(usage("project requires init or bind")),
    }
}

fn health(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    let project = cursor
        .optional()
        .map(|value| project_id(&value))
        .transpose()?;
    let report = operations.health(project);
    Ok(json!({
        "operation":"health", "state":debug_name(report.state), "runtime_root":report.runtime_root,
        "canonical_available":report.canonical_available, "candidate_available":report.candidate_available,
        "privacy_available":report.privacy_available, "guarded_available":report.guarded_available, "forgetting_available":report.forgetting_available, "repository_available":report.repository_available,
        "issues":report.issues.into_iter().map(|issue| json!({"kind":debug_name(issue.kind),"scope":issue.scope,"detail":issue.detail})).collect::<Vec<_>>()
    }))
}

fn analyze(
    operations: &LocalOperations,
    cursor: &mut Cursor,
    rebuild: bool,
) -> Result<Value, Error> {
    let project = project_id(&cursor.next("Project ID")?)?;
    let mut excludes = Vec::new();
    while cursor.peek("--exclude") {
        cursor.next("--exclude")?;
        excludes.push(cursor.next("excluded path")?);
    }
    let result = if rebuild {
        operations.rebuild_analysis(project, excludes)?
    } else {
        operations.analyze(project, excludes)?
    };
    let analysis = result
        .value
        .as_ref()
        .ok_or_else(|| Error::new("analysis ended without an inspectable result"))?;
    let summary = complete_repository_analysis_json(&analysis.analysis);
    Ok(json!({
        "operation":if rebuild {"analysis_rebuild"} else {"analyze"}, "operation_id":result.operation_id.to_string(), "state":debug_name(result.state),
        "duration_micros":result.duration_micros, "repository_snapshot":analysis.repository.identity.to_string(), "analysis_snapshot":analysis.analysis.identity.to_string(),
        "stored_at":analysis.stored_at, "completed_scopes":result.partial.completed_scopes, "partial_scopes":result.partial.partial_scopes,
        "failed_scopes":result.partial.failed_scopes, "omitted_scopes":result.partial.omitted_scopes,
        "capability_reports":summary["capability_reports"], "diagnostics":summary["diagnostics"],
        "diagnostics_omitted_count":summary["diagnostics_omitted_count"], "diagnostic":result.diagnostic
        ,"repository_scope":result.requested_scope,
        "human_summary":crate::model::human_repository_analysis_summary(&analysis.analysis)
    }))
}

fn reindex(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    let project = project_id(&cursor.next("Project ID")?)?;
    let excludes = excluded_paths(cursor)?;
    repair_json("reindex", operations.reindex(project, excludes)?)
}

fn repair(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    let project = project_id(&cursor.next("Project ID")?)?;
    let scope = cursor.next("repair scope")?;
    if scope == "forgetting" {
        let operation = operation_id(&cursor.next("forgetting operation ID")?)?;
        return forgetting_json(operations.repair_forgetting(project, operation)?);
    }
    if scope != "derived-analysis" {
        return Err(Error::new(
            "unsupported repair scope; supported forms: repair PROJECT derived-analysis [--exclude PATH ...] or repair PROJECT forgetting OPERATION_ID",
        ));
    }
    let excludes = excluded_paths(cursor)?;
    repair_json("repair", operations.repair(project, scope, excludes)?)
}

fn excluded_paths(cursor: &mut Cursor) -> Result<Vec<String>, Error> {
    let mut excludes = Vec::new();
    while cursor.peek("--exclude") {
        cursor.next("--exclude")?;
        excludes.push(cursor.next("excluded path")?);
    }
    Ok(excludes)
}

fn repair_json(operation: &str, result: crate::RepairOutcome) -> Result<Value, Error> {
    let analysis =
        result.operation.value.as_ref().ok_or_else(|| {
            Error::new("derived reconstruction ended without an inspectable result")
        })?;
    let summary = bounded_repository_analysis_json(&analysis.analysis);
    Ok(json!({
        "operation":operation,
        "operation_id":result.operation.operation_id.to_string(),
        "state":debug_name(result.operation.state),
        "kind":debug_name(result.kind),
        "scope":result.affected_scope,
        "diagnosis":result.diagnosis,
        "discarded_entries":result.discarded_entries,
        "analysis_snapshot":analysis.analysis.identity.to_string(),
        "stored_at":analysis.stored_at,
        "completed_scopes":result.operation.partial.completed_scopes,
        "partial_scopes":result.operation.partial.partial_scopes,
        "failed_scopes":result.operation.partial.failed_scopes,
        "omitted_scopes":result.operation.partial.omitted_scopes,
        "capability_reports":summary["capability_reports"],
        "diagnostics":summary["diagnostics"],
        "diagnostics_omitted_count":summary["diagnostics_omitted_count"],
        "diagnostic":result.operation.diagnostic,
    }))
}

fn portable(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    match cursor.next("portable command")?.as_str() {
        "export" => {
            let project = project_id(&cursor.next("Project ID")?)?;
            let destination = absolute_path(&cursor.next("bundle destination")?)?;
            let result = operations.export_bundle(project, &destination)?;
            Ok(
                json!({"operation":"portable_export","project_id":result.project_id.to_string(),"path":result.path,"checksum":result.checksum,"history_basis":result.history_basis,"bytes_written":result.bytes_written}),
            )
        }
        "import" => {
            let source = absolute_path(&cursor.next("bundle source")?)?;
            let result = operations.import_bundle(&source)?;
            Ok(
                json!({"operation":"portable_import","project_id":result.project_id.to_string(),"checksum":result.checksum,"history_basis":result.history_basis,"status":debug_name(result.status)}),
            )
        }
        "compare" => {
            let incoming = absolute_path(&cursor.next("incoming bundle")?)?;
            let base = optional_base(cursor)?;
            ensure_no_portable_trailing_arguments(cursor)?;
            let comparison = operations.compare_portable_bundle(base.as_deref(), &incoming)?;
            Ok(portable_comparison_json(&comparison))
        }
        "merge" => {
            let incoming = absolute_path(&cursor.next("incoming bundle")?)?;
            let base = optional_base(cursor)?;
            ensure_no_portable_trailing_arguments(cursor)?;
            let result = operations.merge_portable_bundle(base.as_deref(), &incoming, None)?;
            Ok(portable_merge_json("portable_merge", result))
        }
        "resolve" => {
            let incoming = absolute_path(&cursor.next("incoming bundle")?)?;
            let conflict_set_identity = cursor.next("conflict-set identity")?;
            let conflict_revision = number(&cursor.next("conflict-set revision")?)?;
            let user_turn_source_id = source_id(&cursor.next("current-host user Source ID")?)?;
            let mode_name = cursor.next("resolution mode")?;
            let base = optional_base(cursor)?;
            let mode = match mode_name.as_str() {
                "choose-local" => MergeResolutionMode::ChooseLocal,
                "choose-incoming" => MergeResolutionMode::ChooseIncoming,
                "context-branch" => MergeResolutionMode::ContextBranch,
                "explicit-merged" => {
                    if !cursor.peek("--merged-bundle") {
                        return Err(usage(
                            "portable resolve explicit-merged requires --merged-bundle ABSOLUTE_PATH",
                        ));
                    }
                    cursor.next("--merged-bundle")?;
                    MergeResolutionMode::ExplicitMerged {
                        bundle_path: absolute_path(&cursor.next("explicit merged bundle")?)?,
                    }
                }
                _ => {
                    return Err(usage(
                        "portable resolution mode must be choose-local, choose-incoming, explicit-merged, or context-branch",
                    ))
                }
            };
            ensure_no_portable_trailing_arguments(cursor)?;
            let result = operations.merge_portable_bundle(
                base.as_deref(),
                &incoming,
                Some(MergeResolution {
                    conflict_set_identity,
                    conflict_revision,
                    user_turn_source_id,
                    mode,
                }),
            )?;
            Ok(portable_merge_json("portable_resolve", result))
        }
        _ => Err(usage(
            "portable requires export, import, compare, merge, or resolve",
        )),
    }
}

fn optional_base(cursor: &mut Cursor) -> Result<Option<PathBuf>, Error> {
    if cursor.peek("--base") {
        cursor.next("--base")?;
        absolute_path(&cursor.next("common-base bundle")?).map(Some)
    } else {
        Ok(None)
    }
}

fn ensure_no_portable_trailing_arguments(cursor: &Cursor) -> Result<(), Error> {
    if cursor.has_remaining() {
        Err(usage("unexpected portable arguments"))
    } else {
        Ok(())
    }
}

fn portable_comparison_json(comparison: &BundleComparison) -> Value {
    json!({
        "operation":"portable_compare",
        "project_id":comparison.project_id.to_string(),
        "conflict_set_identity":comparison.conflict_set_identity,
        "conflict_revision":comparison.conflict_revision,
        "requires_user_resolution":comparison.requires_user_resolution(),
        "already_present":comparison.already_present,
        "common_base":comparison.common_base.as_ref().map(bundle_basis_json),
        "local":bundle_basis_json(&comparison.local),
        "incoming":bundle_basis_json(&comparison.incoming),
        "conflicts":comparison.conflicts.iter().map(|conflict| json!({
            "conflict_identity":conflict.conflict_identity,
            "class":portable_conflict_class(conflict.class),
            "affected_identities":conflict.affected_identities,
            "base_basis":conflict.base_basis,
            "local_basis":conflict.local_basis,
            "incoming_basis":conflict.incoming_basis,
            "sources":conflict.sources.iter().map(|source| json!({
                "source_identity":source.source_identity,
                "base":source.base.map(debug_name),
                "local":source.local.map(debug_name),
                "incoming":source.incoming.map(debug_name),
            })).collect::<Vec<_>>(),
            "consequence":conflict.consequence,
            "uncertainty":conflict.uncertainty,
            "automatic_resolution_allowed":conflict.automatic_resolution_allowed,
            "user_judgment_reason":conflict.user_judgment_reason,
        })).collect::<Vec<_>>(),
    })
}

fn bundle_basis_json(basis: &volicord_context::BundleBasis) -> Value {
    json!({
        "checksum":basis.checksum,
        "history_basis":basis.history_basis,
        "common_base_basis":basis.common_base_basis,
    })
}

fn portable_merge_json(
    operation: &str,
    result: volicord_context::OperationResult<volicord_context::BundleMerge>,
) -> Value {
    let value = result.value;
    json!({
        "operation":operation,
        "project_id":value.project_id.to_string(),
        "conflict_set_identity":value.conflict_set_identity,
        "conflict_revision":value.conflict_revision,
        "common_base_basis":value.common_base_basis,
        "local_history_basis":value.local_history_basis,
        "incoming_history_basis":value.incoming_history_basis,
        "result_history_basis":value.result_history_basis,
        "status":portable_merge_status(value.status),
        "resolution_source_id":value.resolution_source_id.map(|source| source.to_string()),
        "affected_identities":value.affected_identities,
        "branch_history_basis":value.branch_history_basis,
        "replayed":result.replayed,
    })
}

const fn portable_conflict_class(class: BundleConflictClass) -> &'static str {
    match class {
        BundleConflictClass::IndependentAdditions => "independent_additions",
        BundleConflictClass::SameRecordRevision => "same_record_revision",
        BundleConflictClass::SemanticDecisionConflict => "semantic_decision_conflict",
        BundleConflictClass::DeleteModifyConflict => "delete_modify_conflict",
        BundleConflictClass::SourceBindingConflict => "source_binding_conflict",
        BundleConflictClass::CommonBaseUnavailable => "common_base_unavailable",
    }
}

const fn portable_merge_status(status: BundleMergeStatus) -> &'static str {
    match status {
        BundleMergeStatus::AlreadyPresent => "already_present",
        BundleMergeStatus::MergedAutomatically => "merged_automatically",
        BundleMergeStatus::Resolved => "resolved",
        BundleMergeStatus::Branched => "branched",
        BundleMergeStatus::Unresolved => "unresolved",
    }
}

fn canonical(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    match cursor.next("canonical command")?.as_str() {
        "inspect" => {
            let project = project_id(&cursor.next("Project ID")?)?;
            let projection = operations.project_projection(project)?;
            Ok(
                json!({"operation":"canonical_inspect","project_id":project.to_string(),"health":debug_name(projection.health),"records":projection.canonical_inspection.into_iter().map(|item| json!({"kind":debug_name(item.kind),"identity":item.identity,"revision":item.revision,"lifecycle_state":item.lifecycle_state,"statement_role":item.statement_role,"summary":item.summary,"source_basis":item.source_basis.into_iter().map(|id| id.to_string()).collect::<Vec<_>>()})).collect::<Vec<_>>(),"issues":projection.issues.into_iter().map(|issue| json!({"kind":debug_name(issue.kind),"scope":issue.affected_scope,"reason":issue.reason})).collect::<Vec<_>>() }),
            )
        }
        "user-source" => {
            let project = project_id(&cursor.next("Project ID")?)?;
            let result = operations.record_user_source(
                project,
                cursor.next("host")?,
                cursor.next("session")?,
                cursor.next("turn text")?,
            )?;
            mutation_json("canonical_user_source", result)
        }
        "correct-context" => {
            let project = project_id(&cursor.next("Project ID")?)?;
            let item = ContextItemId::from_bytes(parse_identity(&cursor.next("Context Item ID")?)?);
            let revision = number(&cursor.next("expected revision")?)?;
            let source = source_id(&cursor.next("user Source ID")?)?;
            let text = cursor.next("corrected statement")?;
            mutation_json(
                "correct_context",
                operations.correct_context_item(
                    project,
                    item,
                    ContextItemCorrectionDraft {
                        expected_revision: revision,
                        corrected_statement: text,
                        kind: CorrectionKind::Expression,
                        user_authorization_source_id: source,
                    },
                )?,
            )
        }
        "correct-decision" => {
            let project = project_id(&cursor.next("Project ID")?)?;
            let decision = DecisionId::from_bytes(parse_identity(&cursor.next("Decision ID")?)?);
            let revision = number(&cursor.next("expected revision")?)?;
            let source = source_id(&cursor.next("user Source ID")?)?;
            let rationale = cursor.next("corrected rationale")?;
            mutation_json(
                "correct_decision",
                operations.correct_decision(
                    project,
                    decision,
                    DecisionCorrectionDraft {
                        expected_revision: revision,
                        corrected_user_rationale: Some(rationale),
                        kind: CorrectionKind::Expression,
                        user_authorization_source_id: source,
                    },
                )?,
            )
        }
        "supersede-decision" => {
            let project = project_id(&cursor.next("Project ID")?)?;
            let previous =
                DecisionId::from_bytes(parse_identity(&cursor.next("previous Decision ID")?)?);
            let source = source_id(&cursor.next("current-host user Source ID")?)?;
            let alternative = cursor.next("displayed alternative key")?;
            let rationale = cursor.optional();
            mutation_json(
                "supersede_decision",
                operations.supersede_decision_choice(
                    project,
                    previous,
                    source,
                    alternative,
                    rationale,
                )?,
            )
        }
        "forget" => {
            let project = project_id(&cursor.next("Project ID")?)?;
            let kind = cursor.next("record kind")?;
            let identity = parse_identity(&cursor.next("record ID")?)?;
            let authorization = source_id(&cursor.next("user authorization Source ID")?)?;
            let record = match kind.as_str() {
                "source" => CanonicalRecordId::Source(SourceId::from_bytes(identity)),
                "question" => CanonicalRecordId::Question(volicord_context::QuestionId::from_bytes(identity)),
                "decision" => CanonicalRecordId::Decision(DecisionId::from_bytes(identity)),
                "context_item" => CanonicalRecordId::ContextItem(ContextItemId::from_bytes(identity)),
                "checkpoint" => CanonicalRecordId::Checkpoint(volicord_context::CheckpointId::from_bytes(identity)),
                _ => return Err(usage("forgettable kind must be source, question, decision, context_item, or checkpoint")),
            };
            forgetting_json(operations.forget_record(project, record, authorization)?)
        }
        _ => Err(usage(
            "canonical requires inspect, user-source, correct-context, correct-decision, supersede-decision, or forget",
        )),
    }
}

fn candidates(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    let project = project_id(&cursor.next("Project ID")?)?;
    let projection = operations.project_projection(project)?;
    Ok(
        json!({"operation":"candidate_inspection","project_id":project.to_string(),"health":debug_name(projection.health),"candidates":projection.candidate_inspection.into_iter().map(|candidate| json!({"identity":candidate.candidate_id.to_string(),"exists":candidate.exists,"health":debug_name(candidate.health),"revision":candidate.revision,"kind":candidate.kind.map(debug_name),"summary":candidate.bounded_summary,"content_cleaned":candidate.content_cleaned,"promotion_disposition":candidate.promotion_disposition.map(debug_name)})).collect::<Vec<_>>() }),
    )
}

fn privacy(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    match cursor.next("privacy command")?.as_str() {
        "status" => {
            let project = project_id(&cursor.next("Project ID")?)?;
            let status = operations.privacy_status(project)?;
            Ok(
                json!({"operation":"privacy_status","project_id":project.to_string(),"configuration_state":debug_name(status.configuration_state),"policy_revision":status.current_opt_in.as_ref().map(|value| value.revision),"policy_state":status.current_opt_in.as_ref().map(|value| debug_name(value.state)),"provider":status.current_opt_in.as_ref().map(|value| value.policy.provider.clone()),"model":status.current_opt_in.as_ref().map(|value| value.policy.model.clone()),"purpose":status.current_opt_in.as_ref().map(|value| value.policy.purpose.clone()),"requested_capability":status.current_opt_in.as_ref().map(|value| value.policy.requested_capability.clone()),"allowed_source_scopes":status.current_opt_in.as_ref().map(|value| value.policy.allowed_source_scopes.clone()),"exclusions":status.current_opt_in.as_ref().map(|value| json!({"path_prefixes":value.policy.exclusions.path_prefixes,"file_classes":value.policy.exclusions.file_classes.iter().map(|class| debug_name(*class)).collect::<Vec<_>>(),"basis":value.policy.exclusions.basis})),"filtering":status.current_opt_in.as_ref().map(|value| json!({"enabled":value.policy.filtering.enabled,"known_limits":value.policy.filtering.known_limits})),"provider_retention":status.current_opt_in.as_ref().map(|value| json!({"expectation":value.policy.retention.provider_expectation,"known_limits":value.policy.retention.provider_known_limits,"local_basis":value.policy.retention.local_basis})),"request_count":status.requests.len(),"requests":status.requests.iter().map(|request| json!({"request_id":request.id.to_string(),"provider":request.provider,"model":request.model,"purpose":request.purpose,"repository_snapshot":request.repository_snapshot.to_string(),"analysis_snapshot":request.analysis_snapshot.to_string(),"outcome":debug_name(request.outcome),"diagnostic":request.diagnostic,"manifest":request.manifest.iter().map(|entry| json!({"source_id":entry.source.identity().to_string(),"locator":entry.locator,"scope_outcome":debug_name(entry.scope_outcome),"filter_outcome":debug_name(entry.filter_outcome),"transmission_outcome":debug_name(entry.transmission_outcome),"transmitted_bytes":entry.transmitted_bytes})).collect::<Vec<_>>()})).collect::<Vec<_>>(),"managed_derived_count":status.managed_derived.len()}),
            )
        }
        "enable" => {
            let project = project_id(&cursor.next("Project ID")?)?;
            let provider = cursor.next("provider")?;
            let model = cursor.next("model")?;
            let source = source_id(&cursor.next("current-host user Source ID")?)?;
            let scopes = cursor.remaining();
            if scopes.is_empty() {
                return Err(usage(
                    "privacy enable requires at least one explicit source scope",
                ));
            }
            let codex_provider = provider == crate::CODEX_CLI_PROVIDER;
            let policy = ProviderOptInPolicy {
                project_id: project,
                provider,
                model,
                purpose: "background semantic analysis".into(),
                requested_capability: "semantic".into(),
                allowed_source_scopes: scopes,
                exclusions: SourceExclusionPolicy {
                    path_prefixes: Vec::new(),
                    file_classes: Vec::new(),
                    basis: "explicit CLI scope".into(),
                },
                filtering: SecretFilteringPolicy {
                    enabled: true,
                    line_markers: vec!["SECRET".into(), "TOKEN".into(), "PASSWORD".into()],
                    replacement: "[filtered]".into(),
                    known_limits: vec!["marker filtering is not complete secret detection".into()],
                },
                retention: ProviderRetentionPolicy {
                    local_annotation_retained_until: None,
                    local_basis: "until explicit deletion".into(),
                    provider_expectation: if codex_provider {
                        "the Codex CLI transport exposes no provider-side deletion operation".into()
                    } else {
                        "provider-specific retention and deletion support is not established".into()
                    },
                    provider_known_limits: vec![if codex_provider {
                        "local deletion cannot be reported as provider-side deletion for the Codex CLI transport"
                            .into()
                    } else {
                        "provider-side deletion support is unknown until the configured adapter reports an observed outcome"
                            .into()
                    }],
                },
            };
            let event = operations.enable_provider(
                policy,
                privacy_intent(source, "enable background semantic provider"),
            )?;
            Ok(
                json!({"operation":"privacy_enable","project_id":project.to_string(),"revision":event.revision,"state":debug_name(event.state),"provider":event.policy.provider,"model":event.policy.model,"allowed_source_scopes":event.policy.allowed_source_scopes}),
            )
        }
        "disable" | "revoke" => {
            let action = cursor.previous().unwrap_or_default();
            let project = project_id(&cursor.next("Project ID")?)?;
            let source = source_id(&cursor.next("current-host user Source ID")?)?;
            let event = if action == "disable" {
                operations.disable_provider(
                    project,
                    privacy_intent(source, "disable background semantic provider"),
                )?
            } else {
                operations.revoke_provider(
                    project,
                    privacy_intent(source, "revoke background semantic provider"),
                )?
            };
            Ok(
                json!({"operation":format!("privacy_{action}"),"project_id":project.to_string(),"revision":event.revision,"state":debug_name(event.state)}),
            )
        }
        _ => Err(usage("privacy requires status, enable, disable, or revoke")),
    }
}

fn recall(
    operations: &LocalOperations,
    cursor: &mut Cursor,
    language: &str,
    locale: FixedLocale,
) -> Result<Value, Error> {
    let project = project_id(&cursor.next("Project ID")?)?;
    let brief = operations.recall(project)?;
    let mut output = crate::resume_brief_json(&brief, language, locale);
    output["operation"] = json!("recall");
    Ok(output)
}

fn documents(
    operations: &LocalOperations,
    cursor: &mut Cursor,
    fixed_locale: FixedLocale,
) -> Result<Value, Error> {
    let action = cursor.next("documents command")?;
    let project = project_id(&cursor.next("Project ID")?)?;
    let kind = document_kind(&cursor.next("document kind")?)?;
    let format = output_format(&cursor.next("output format")?)?;
    let destination = if action == "export" {
        Some(absolute_path(&cursor.next("destination")?)?)
    } else if action == "preview" {
        None
    } else {
        return Err(usage("documents requires preview or export"));
    };
    let language = cursor.optional().unwrap_or_else(|| "en".into());
    let requested_destinations = destination
        .as_ref()
        .map(|path| RequestedDestination {
            document_kind: kind,
            output_format: format,
            path: path.display().to_string(),
        })
        .into_iter()
        .collect();
    let request = DocumentRequest {
        requested_language: language,
        fixed_locale,
        generated_at: now()?,
        generator: GeneratorIdentity {
            generator: "volicord-local-operations".into(),
            agent: None,
            model: None,
        },
        requested_destinations,
    };
    let set = operations.documents(project, &request)?;
    let document = select_document(&set, kind);
    if let NarrativeRealizationState::Unavailable { reason } =
        &document.metadata.narrative_realization
    {
        return Ok(json!({
            "operation":if destination.is_some() { "document_export" } else { "document_preview" },
            "project_id":project.to_string(),
            "kind":kind.slug(),
            "format":debug_name(format),
            "outcome":"unavailable",
            "requested_language":document.metadata.requested_language,
            "reason":reason,
            "published":false,
            "canonical_mutation":false
        }));
    }
    let artifact = if format == OutputFormat::Markdown {
        &document.markdown
    } else {
        &document.html
    };
    if let Some(path) = destination {
        let published = operations.publish_document(document, format, &path)?;
        Ok(
            json!({"operation":"document_export","project_id":project.to_string(),"kind":kind.slug(),"format":debug_name(format),"destination":published.destination,"bytes":published.bytes,"durability":published.durability,"canonical_mutation":false}),
        )
    } else {
        Ok(
            json!({"operation":"document_preview","project_id":project.to_string(),"kind":kind.slug(),"format":debug_name(format),"content":artifact.content,"canonical_mutation":false}),
        )
    }
}

fn inquiry(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    if cursor.next("inquiry command")? != "frontier" {
        return Err(usage(
            "inquiry currently supports: inquiry frontier PROJECT [SCOPE ...]",
        ));
    }
    let project = project_id(&cursor.next("Project ID")?)?;
    let frontier = operations.inquiry_frontier(project, cursor.remaining())?;
    Ok(
        json!({"operation":"inquiry_frontier","project_id":project.to_string(),"questions":frontier.questions.into_iter().map(|question| json!({"identity":question.question_id.to_string(),"revision":question.displayed_revision,"prompt":question.prompt_basis,"why_now":question.why_it_matters_now,"material_scope":question.material_scope,"what_unlocks":question.what_the_answer_unlocks})).collect::<Vec<_>>(),"diagnostics":frontier.diagnostics.into_iter().map(|diagnostic| json!({"kind":debug_name(diagnostic.kind),"question_id":diagnostic.question_id.to_string(),"detail":diagnostic.detail})).collect::<Vec<_>>() }),
    )
}

fn checkpoint(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    if cursor.next("checkpoint command")? != "record" {
        return Err(usage(
            "checkpoint currently supports: checkpoint record PROJECT KIND SOURCE WORK_ITEM NEXT_STEP [HANDOFF_TARGET]",
        ));
    }
    let project = project_id(&cursor.next("Project ID")?)?;
    let kind = match cursor.next("checkpoint kind")?.as_str() {
        "completion" => CheckpointKind::Completion,
        "pause" => CheckpointKind::Pause,
        "handoff" => CheckpointKind::Handoff,
        _ => {
            return Err(usage(
                "checkpoint kind must be completion, pause, or handoff",
            ))
        }
    };
    let source = source_id(&cursor.next("grounding Source ID")?)?;
    let work_item_id = context_item_id(&cursor.next("Work Item ID")?)?;
    let goal = operations
        .canonical_basis(project)?
        .context_items
        .into_iter()
        .find(|item| item.id == work_item_id)
        .map(|item| item.statement)
        .ok_or_else(|| Error::new("Work Item Goal Context was not found"))?;
    let next_step = cursor.next("next step")?;
    let handoff_to = if kind == CheckpointKind::Handoff {
        Some(cursor.next("explicit handoff target")?)
    } else {
        None
    };
    if cursor.has_remaining() {
        return Err(usage(
            "checkpoint accepts HANDOFF_TARGET only for handoff and accepts no trailing arguments",
        ));
    }
    let project_revision = operations.canonical_basis(project)?.project.revision;
    let work_state = match kind {
        CheckpointKind::Completion => WorkState::Completed,
        CheckpointKind::Pause | CheckpointKind::Handoff => WorkState::Paused,
    };
    let result = operations.record_checkpoint(
        project,
        CheckpointDraft {
            expected_project_revision: project_revision,
            work_item_id: Some(work_item_id),
            kind,
            goal,
            work_state,
            state_change: Some("explicit CLI checkpoint".into()),
            source_basis: vec![source],
            changed_source_basis: Vec::new(),
            changed_paths: Vec::new(),
            applied_decisions: Vec::new(),
            verification: vec![VerificationFact {
                state: VerificationState::NotRun,
                source_id: None,
                outcome: None,
            }],
            user_review: UserReviewFact {
                state: UserReviewState::NotRequested,
                source_id: None,
            },
            user_acceptance: UserAcceptanceFact {
                state: UserAcceptanceState::NotRequested,
                source_id: None,
            },
            known_limits: Vec::new(),
            non_goals: Vec::new(),
            open_questions: Vec::new(),
            next_step,
            handoff_to,
        },
    )?;
    mutation_json("checkpoint_record", result)
}

fn guarded(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    match cursor.next("guarded command")?.as_str() {
        "request" => {
            let project = project_id(&cursor.next("Project ID")?)?;
            let category = guarded_category(&cursor.next("risk category")?)?;
            let exact_action = cursor.next("exact action")?;
            let target = cursor.next("exact target")?;
            let expected_effect = cursor.next("expected effect")?;
            let concrete_consequence = cursor.next("concrete risk")?;
            let expires_at = number(&cursor.next("expiration Unix microseconds")?)?;
            let expires_at = i64::try_from(expires_at)
                .map(volicord_context::TimestampMicros::from_unix_micros)
                .map_err(|_| Error::new("expiration exceeds the supported timestamp range"))?;
            let scope = cursor.remaining();
            if scope.is_empty() {
                return Err(usage("guarded request requires at least one bounded scope"));
            }
            let candidate = operations.create_guarded_request(GuardedEffectDraft {
                project_id: project,
                exact_action,
                target,
                expected_effect,
                risk: GuardedRisk {
                    category,
                    concrete_consequence,
                },
                scope,
                expires_at,
                requesting_provenance: RequestingProvenance {
                    actor: Principal {
                        kind: PrincipalKind::Agent,
                        identity: "volicord-cli".into(),
                    },
                    host: Some("cli".into()),
                    session: Some("cli".into()),
                    basis: vec!["explicit CLI Guarded Effect Candidate".into()],
                },
            })?;
            Ok(guarded_request_json("guarded_request", &candidate))
        }
        "show" => {
            let request = confirmation_request_id(&cursor.next("confirmation request ID")?)?;
            let candidate = operations.guarded_request(request)?;
            Ok(guarded_request_json("guarded_show", &candidate))
        }
        "confirm" | "deny" => {
            let decision_text = cursor.previous().unwrap_or_default();
            let request = confirmation_request_id(&cursor.next("confirmation request ID")?)?;
            let revision = number(&cursor.next("request revision")?)?;
            let fingerprint = cursor.next("effect fingerprint")?;
            let host = cursor.next("current host")?;
            let session = cursor.next("current session")?;
            let turn = cursor.next("explicit user response")?;
            let decision = if decision_text == "confirm" {
                ConfirmationDecision::Confirmed
            } else {
                ConfirmationDecision::Denied
            };
            let response = operations.record_confirmation(
                request,
                revision,
                &fingerprint,
                decision,
                host,
                session,
                turn,
            )?;
            Ok(json!({
                "operation":format!("guarded_{decision_text}"),
                "confirmation_request_identity":response.confirmation_request_identity.to_string(),
                "request_revision":response.request_revision,
                "effect_fingerprint":response.effect_fingerprint,
                "decision":debug_name(response.decision),
                "user_response_source_id":response.user_response_source_id.to_string(),
                "confirmation_response_identity":response.confirmation_response_identity.to_string()
            }))
        }
        _ => Err(usage("guarded requires request, show, confirm, or deny")),
    }
}

fn guarded_request_json(operation: &str, candidate: &crate::GuardedEffectCandidate) -> Value {
    json!({
        "operation":operation,
        "confirmation_request_identity":candidate.confirmation_request_identity.to_string(),
        "request_revision":candidate.request_revision,
        "project_id":candidate.project_id.to_string(),
        "exact_action":candidate.exact_action,
        "target":candidate.target,
        "expected_effect":candidate.expected_effect,
        "risk_category":debug_name(candidate.risk.category),
        "risk_consequence":candidate.risk.concrete_consequence,
        "scope":candidate.scope,
        "expiration_unix_micros":candidate.expires_at.as_unix_micros(),
        "requesting_actor":format!("{:?}:{}", candidate.requesting_provenance.actor.kind, candidate.requesting_provenance.actor.identity),
        "requesting_provenance":candidate.requesting_provenance.basis,
        "effect_fingerprint":candidate.effect_fingerprint
    })
}

fn mutation_json(operation: &str, value: crate::CanonicalMutationOutcome) -> Result<Value, Error> {
    Ok(
        json!({"operation":operation,"record_kind":value.record_kind,"identity":value.identity,"revision":value.revision,"replayed":value.replayed}),
    )
}

fn forgetting_json(value: crate::ForgettingOutcome) -> Result<Value, Error> {
    Ok(json!({
        "operation":"canonical_forget",
        "forgetting_operation_id":value.operation_id.to_string(),
        "record_kind":value.record_kind,
        "identity":value.identity,
        "state":debug_name(value.state),
        "canonical_committed":value.canonical_committed,
        "candidate_cleanup_completed":value.candidate_cleanup_completed,
        "managed_derived_cleanup_completed":value.managed_derived_cleanup_completed,
        "residue_verified":value.residue_verified,
        "replayed":value.replayed,
        "provider_deletion":debug_name(value.provider_deletion),
        "diagnostic":value.diagnostic,
    }))
}

fn privacy_intent(source: SourceId, basis: &str) -> ProviderIntentProvenance {
    ProviderIntentProvenance {
        actor: Principal {
            kind: PrincipalKind::User,
            identity: "current-host-user".into(),
        },
        host: "cli".into(),
        session: "cli".into(),
        user_turn_source: source,
        basis: basis.into(),
    }
}

fn project_id(value: &str) -> Result<ProjectId, Error> {
    Ok(ProjectId::from_bytes(parse_identity(value)?))
}
fn operation_id(value: &str) -> Result<OperationId, Error> {
    Ok(OperationId::from_bytes(parse_identity(value)?))
}
fn confirmation_request_id(value: &str) -> Result<ConfirmationRequestId, Error> {
    Ok(ConfirmationRequestId::from_bytes(parse_identity(value)?))
}
fn source_id(value: &str) -> Result<SourceId, Error> {
    Ok(SourceId::from_bytes(parse_identity(value)?))
}
fn context_item_id(value: &str) -> Result<ContextItemId, Error> {
    Ok(ContextItemId::from_bytes(parse_identity(value)?))
}
fn number(value: &str) -> Result<u64, Error> {
    value
        .parse()
        .map_err(|error| Error::with_source("expected an unsigned integer", error))
}
fn absolute_path(value: &str) -> Result<PathBuf, Error> {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        Ok(path)
    } else {
        Err(Error::new("path must be absolute"))
    }
}
fn now() -> Result<volicord_context::TimestampMicros, Error> {
    use volicord_context::Clock;
    volicord_context::SystemClock
        .now()
        .map_err(|error| Error::with_source("system clock is unavailable", error))
}

fn document_kind(value: &str) -> Result<DocumentKind, Error> {
    match value {
        "project-architecture-guide" => Ok(DocumentKind::ProjectArchitectureGuide),
        "decision-report" => Ok(DocumentKind::DecisionReport),
        "implementation-plan" => Ok(DocumentKind::ImplementationPlan),
        "handoff-resume" => Ok(DocumentKind::HandoffResume),
        _ => Err(usage("unknown document kind")),
    }
}
fn output_format(value: &str) -> Result<OutputFormat, Error> {
    match value {
        "markdown" => Ok(OutputFormat::Markdown),
        "html" => Ok(OutputFormat::Html),
        _ => Err(usage("output format must be markdown or html")),
    }
}
fn guarded_category(value: &str) -> Result<GuardedEffectCategory, Error> {
    match value {
        "destructive-delete" => Ok(GuardedEffectCategory::DestructiveFileOrDataDeletion),
        "migration" => Ok(GuardedEffectCategory::IrreversibleOrLargeScaleMigration),
        "external-publication" => Ok(GuardedEffectCategory::ExternalDeploymentOrPublicPublication),
        "cost" => Ok(GuardedEffectCategory::PaymentOrContinuingCost),
        "credential" => Ok(GuardedEffectCategory::SecretOrCredentialAccessOrChange),
        "external-source-transmission" => {
            Ok(GuardedEffectCategory::PersonalDataOrSourceCodeExternalTransmission)
        }
        "external-message" => Ok(GuardedEffectCategory::ExternalMessageEmailOrIssue),
        "production-data" => Ok(GuardedEffectCategory::ProductionDataChange),
        "security-setting" => {
            Ok(GuardedEffectCategory::PermissionAuthenticationOrSecuritySettingChange)
        }
        _ => Err(usage("unknown Guarded risk category")),
    }
}
fn debug_name(value: impl std::fmt::Debug) -> String {
    format!("{value:?}").to_lowercase()
}

pub(crate) struct Cursor {
    args: Vec<OsString>,
    index: usize,
    previous: Option<String>,
}
impl Cursor {
    fn new(args: Vec<OsString>) -> Self {
        Self {
            args,
            index: 0,
            previous: None,
        }
    }
    fn from_strings(args: Vec<String>) -> Self {
        Self::new(args.into_iter().map(OsString::from).collect())
    }
    fn push(&mut self, value: impl Into<OsString>) {
        self.args.push(value.into());
    }
    pub(crate) fn next(&mut self, label: &str) -> Result<String, Error> {
        let value = self
            .args
            .get(self.index)
            .ok_or_else(|| usage(&format!("missing {label}")))?;
        let value = value
            .to_str()
            .ok_or_else(|| Error::new(format!("{label} must be valid UTF-8")))?
            .to_owned();
        self.index += 1;
        self.previous = Some(value.clone());
        Ok(value)
    }
    fn optional(&mut self) -> Option<String> {
        if self.index < self.args.len() {
            self.next("argument").ok()
        } else {
            None
        }
    }
    fn peek(&self, value: &str) -> bool {
        self.args
            .get(self.index)
            .is_some_and(|arg| arg == OsStr::new(value))
    }
    fn remaining(&mut self) -> Vec<String> {
        let mut values = Vec::new();
        while self.index < self.args.len() {
            if let Ok(value) = self.next("argument") {
                values.push(value);
            }
        }
        values
    }
    fn previous(&self) -> Option<String> {
        self.previous.clone()
    }
    fn has_remaining(&self) -> bool {
        self.index < self.args.len()
    }
    pub(crate) fn done(&self) -> Result<(), Error> {
        if self.index == self.args.len() {
            Ok(())
        } else {
            Err(usage("unexpected trailing arguments"))
        }
    }
}

fn work_selection_json(selection: volicord_projections::WorkSelection) -> Value {
    use volicord_projections::{WorkSelectionBasis, WorkSelector};
    let kind = match selection.selector {
        WorkSelector::LatestWork => "latest_work",
        WorkSelector::ExactWork(_) => "exact_work",
        WorkSelector::Repository => "repository",
    };
    let basis = match selection.basis {
        WorkSelectionBasis::ExactGoal { revision } => {
            json!({"kind":"exact_goal","revision":revision})
        }
        WorkSelectionBasis::LatestGoal { revision } => {
            json!({"kind":"latest_goal","revision":revision})
        }
        WorkSelectionBasis::LatestCheckpoint {
            checkpoint_id,
            revision,
        } => {
            json!({"kind":"latest_checkpoint","checkpoint_id":checkpoint_id.to_string(),"revision":revision})
        }
        WorkSelectionBasis::UnassociatedCheckpoint {
            checkpoint_id,
            revision,
        } => {
            json!({"kind":"unassociated_checkpoint","checkpoint_id":checkpoint_id.to_string(),"revision":revision})
        }
        WorkSelectionBasis::Repository => json!({"kind":"repository"}),
        WorkSelectionBasis::NoWork => json!({"kind":"no_work"}),
    };
    json!({"kind":kind,"work_item_id":selection.work_item_id.map(|id| id.to_string()),"basis":basis})
}
