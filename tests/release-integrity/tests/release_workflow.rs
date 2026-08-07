use std::{collections::BTreeSet, fs, path::Path};

use serde_yaml::Value;
use volicord_types::release_target::ReleaseTargetTriple;

const RELEASE_SMOKE_ACTION: &str = "./.github/actions/volicord-release-smoke";
const RELEASE_SMOKE_INPUT: &str = "binary-path";
const RELEASE_SMOKE_PACKAGE: &str = "volicord-release-smoke";
const SOURCE_BUNDLE_COMMAND: &str = "source-bundle --output";

#[test]
fn maintained_workflow_and_action_yaml_files_parse() {
    let root = repository_root();
    for path in [
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
        ".github/actions/volicord-release-smoke/action.yml",
    ] {
        let text = fs::read_to_string(root.join(path)).expect("read workflow YAML");
        serde_yaml::from_str::<Value>(&text).expect("parse workflow YAML");
    }
}

#[test]
fn canonical_smoke_action_invokes_the_dedicated_package_with_its_input() {
    let action = read_yaml(".github/actions/volicord-release-smoke/action.yml");
    assert_eq!(action["runs"]["using"].as_str(), Some("composite"));
    assert_eq!(
        action["inputs"][RELEASE_SMOKE_INPUT]["required"].as_bool(),
        Some(true)
    );
    let invocation_steps = action["runs"]["steps"]
        .as_sequence()
        .expect("composite action steps")
        .iter()
        .filter(|step| {
            step["run"].as_str().is_some_and(|run| {
                run.contains("cargo run")
                    && run.contains("-p volicord-release-smoke")
                    && run.contains("--bin")
                    && run.contains("inputs.binary-path")
            })
        })
        .count();
    assert_eq!(invocation_steps, 1);
}

#[test]
fn release_workflow_smokes_every_published_binary_once_before_staging() {
    let workflow = read_yaml(".github/workflows/release.yml");
    let build = &workflow["jobs"]["build-binaries"];
    let matrix = build["strategy"]["matrix"]["include"]
        .as_sequence()
        .expect("build matrix");
    let targets = matrix
        .iter()
        .map(|entry| {
            entry["target"]
                .as_str()
                .expect("target string")
                .parse::<ReleaseTargetTriple>()
                .expect("published target")
        })
        .collect::<BTreeSet<_>>();
    let expected = [
        ReleaseTargetTriple::X86_64UnknownLinuxGnu,
        ReleaseTargetTriple::Aarch64UnknownLinuxGnu,
        ReleaseTargetTriple::Aarch64AppleDarwin,
        ReleaseTargetTriple::X86_64AppleDarwin,
        ReleaseTargetTriple::X86_64PcWindowsMsvc,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    assert_eq!(targets, expected);

    let target_binaries = matrix
        .iter()
        .map(|entry| {
            (
                entry["target"].as_str().expect("target string"),
                entry["binary"]
                    .as_str()
                    .filter(|binary| !binary.is_empty())
                    .expect("nonempty binary name"),
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        target_binaries,
        BTreeSet::from([
            ("aarch64-apple-darwin", "volicord"),
            ("aarch64-unknown-linux-gnu", "volicord"),
            ("x86_64-apple-darwin", "volicord"),
            ("x86_64-pc-windows-msvc", "volicord.exe"),
            ("x86_64-unknown-linux-gnu", "volicord"),
        ])
    );

    let steps = build["steps"].as_sequence().expect("build steps");
    let build_index = steps
        .iter()
        .position(is_release_binary_build)
        .expect("Volicord release binary build step");
    let smoke_indices = smoke_action_indices(steps);
    assert_eq!(smoke_indices.len(), 1);
    let smoke_index = smoke_indices[0];
    assert!(build_index < smoke_index);
    let smoke = &steps[smoke_index];
    assert!(
        smoke["if"].is_null(),
        "smoke must run for every matrix entry"
    );
    let binary_input = smoke["with"][RELEASE_SMOKE_INPUT]
        .as_str()
        .expect("smoke binary input");
    assert!(binary_input.contains("matrix.target"));
    assert!(binary_input.contains("matrix.binary"));

    let staging_indices = steps
        .iter()
        .enumerate()
        .filter_map(|(index, step)| {
            step["name"]
                .as_str()
                .is_some_and(|name| name.starts_with("Stage immutable raw build artifact"))
                .then_some(index)
        })
        .collect::<Vec<_>>();
    assert!(!staging_indices.is_empty());
    assert!(staging_indices.iter().all(|index| smoke_index < *index));
    assert_no_direct_smoke_invocation(steps);

    for entry in matrix {
        assert!(entry["target"].as_str().is_some());
        assert!(entry["binary"].as_str().is_some());
        assert_eq!(smoke_indices.len(), 1);
    }
}

#[test]
fn ordinary_ci_delegates_build_smoke_and_aggregate_to_the_shared_final_plan() {
    let workflow = read_yaml(".github/workflows/ci.yml");
    let steps = workflow["jobs"]["checks"]["steps"]
        .as_sequence()
        .expect("CI check steps");
    assert_eq!(shared_final_plan_invocations(steps).len(), 1);
    assert!(smoke_action_indices(steps).is_empty());
    assert!(steps.iter().all(|step| !is_debug_binary_build(step)));
    assert_no_direct_smoke_invocation(steps);
}

#[test]
fn ci_and_release_publish_use_the_canonical_source_bundle_command() {
    let ci = read_yaml(".github/workflows/ci.yml");
    let ci_steps = ci["jobs"]["checks"]["steps"]
        .as_sequence()
        .expect("CI check steps");
    assert_eq!(shared_final_plan_invocations(ci_steps).len(), 1);
    assert!(source_bundle_invocations(ci_steps).is_empty());

    let release = read_yaml(".github/workflows/release.yml");
    let publish_steps = release["jobs"]["publish-release"]["steps"]
        .as_sequence()
        .expect("release publish steps");
    let release_invocations = source_bundle_invocations(publish_steps);
    assert_eq!(release_invocations.len(), 1);
    assert!(release_invocations[0].contains("dist/volicord-source.zip"));
    assert!(publish_steps
        .iter()
        .filter_map(|step| step["run"].as_str())
        .any(|run| {
            run.contains("(cd dist && sha256sum volicord-source.zip > volicord-source.zip.sha256)")
        }));
}

#[test]
fn ci_runs_the_complete_activation_journey_on_every_native_runtime_platform() {
    let workflow = read_yaml(".github/workflows/ci.yml");
    let linux = &workflow["jobs"]["checks"];
    assert_eq!(linux["runs-on"].as_str(), Some("ubuntu-24.04"));
    assert_eq!(
        shared_final_plan_invocations(linux["steps"].as_sequence().expect("Linux CI steps")).len(),
        1
    );

    let native = &workflow["jobs"]["operational-host-native-platforms"];
    let matrix = native["strategy"]["matrix"]["include"]
        .as_sequence()
        .expect("native operational matrix");
    let platforms = matrix
        .iter()
        .map(|entry| {
            (
                entry["platform"].as_str().expect("platform label"),
                entry["os"].as_str().expect("runner image"),
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        platforms,
        BTreeSet::from([("macOS", "macos-15"), ("native Windows", "windows-2022"),])
    );
    let journey_invocations = native["steps"]
        .as_sequence()
        .expect("native operational steps")
        .iter()
        .filter_map(|step| step["run"].as_str())
        .filter(|run| {
            run.contains("cargo test")
                && run.contains("-p volicord-cli")
                && run.contains("--test operational_host_e2e")
        })
        .count();
    assert_eq!(journey_invocations, 1);
}

#[test]
fn workflow_filters_cover_the_shared_smoke_boundary_and_contract_inputs() {
    let release_required = [
        ".github/actions/volicord-release-smoke/**",
        "crates/volicord-cli/**",
        "crates/volicord-mcp-protocol/**",
        "crates/volicord-test-process/**",
        "crates/volicord-types/**",
        "tests/release-smoke/**",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<BTreeSet<_>>();
    let ci = read_yaml(".github/workflows/ci.yml");
    let ci_pull_paths = workflow_paths(&ci, "pull_request");
    let ci_push_paths = workflow_paths(&ci, "push");
    let canonical = canonical_ci_trigger_paths();
    assert_eq!(ci_pull_paths, canonical);
    assert_eq!(ci_push_paths, canonical);
    assert!(canonical.contains("*"));
    for directory in [
        ".github/**",
        "crates/**",
        "docs/**",
        "scripts/**",
        "tests/**",
        "xtask/**",
    ] {
        assert!(canonical.contains(directory));
    }

    let release = read_yaml(".github/workflows/release.yml");
    let release_paths = workflow_paths(&release, "pull_request");
    assert!(release_required.is_subset(&release_paths));
}

#[test]
fn ci_fetches_history_and_passes_one_verified_event_base_to_final_validation() {
    let workflow = read_yaml(".github/workflows/ci.yml");
    assert_eq!(
        workflow["on"]["workflow_dispatch"]["inputs"]["base"]["required"].as_bool(),
        Some(true)
    );
    let steps = workflow["jobs"]["checks"]["steps"]
        .as_sequence()
        .expect("CI check steps");
    let checkout = steps
        .iter()
        .find(|step| step["uses"].as_str() == Some("actions/checkout@v4"))
        .expect("CI checkout");
    assert_eq!(checkout["with"]["fetch-depth"].as_u64(), Some(0));

    let resolver = steps
        .iter()
        .find(|step| step["id"].as_str() == Some("validation-base"))
        .and_then(|step| step["run"].as_str())
        .expect("CI base resolver");
    assert!(resolver.contains("ci-base"));
    assert!(resolver.contains("--event-name \"$GITHUB_EVENT_NAME\""));
    assert!(resolver.contains("--event-path \"$GITHUB_EVENT_PATH\""));
    assert!(resolver.contains("--github-output \"$GITHUB_OUTPUT\""));

    let final_invocations = shared_final_plan_invocations(steps);
    assert_eq!(final_invocations.len(), 1);
    assert!(final_invocations[0].contains("steps.validation-base.outputs.base"));
    assert!(!final_invocations[0].contains("--base HEAD"));
}

#[test]
fn product_cli_has_no_reverse_xtask_development_dependency() {
    let manifest = read_toml("crates/volicord-cli/Cargo.toml");
    let dev_dependencies = manifest["dev-dependencies"]
        .as_table()
        .expect("CLI dev-dependencies");
    assert!(!dev_dependencies.contains_key("xtask"));
}

#[test]
fn release_smoke_package_keeps_product_implementation_out_of_its_dependencies() {
    let manifest = read_toml("tests/release-smoke/Cargo.toml");
    let dependencies = manifest["dependencies"]
        .as_table()
        .expect("release-smoke dependencies");
    assert!(dependencies.contains_key("volicord-test-process"));
    assert!(dependencies.contains_key("volicord-mcp-protocol"));
    assert!(dependencies.contains_key("volicord-types"));
    for forbidden in [
        "volicord-cli",
        "volicord-mcp",
        "volicord-core",
        "volicord-store",
        "xtask",
    ] {
        assert!(!dependencies.contains_key(forbidden));
    }
}

fn smoke_action_indices(steps: &[Value]) -> Vec<usize> {
    steps
        .iter()
        .enumerate()
        .filter_map(|(index, step)| {
            (step["uses"].as_str() == Some(RELEASE_SMOKE_ACTION)).then_some(index)
        })
        .collect()
}

fn is_release_binary_build(step: &Value) -> bool {
    step["run"].as_str().is_some_and(|run| {
        run.contains("cargo build")
            && run.contains("--release")
            && run.contains("-p volicord-cli")
            && run.contains("--bin volicord")
            && run.contains("--target")
            && run.contains("matrix.target")
    })
}

fn is_debug_binary_build(step: &Value) -> bool {
    step["run"].as_str().is_some_and(|run| {
        run.contains("cargo build")
            && run.contains("-p volicord-cli")
            && run.contains("--bin volicord")
            && !run.contains("--release")
            && !run.contains("--target")
    })
}

fn assert_no_direct_smoke_invocation(steps: &[Value]) {
    let direct = steps
        .iter()
        .filter_map(|step| step["run"].as_str())
        .filter(|run| run.contains(RELEASE_SMOKE_PACKAGE))
        .count();
    assert_eq!(direct, 0);
}

fn source_bundle_invocations(steps: &[Value]) -> Vec<&str> {
    steps
        .iter()
        .filter_map(|step| step["run"].as_str())
        .filter(|run| {
            run.contains("cargo run")
                && run.contains("--locked")
                && run.contains("-p xtask")
                && run.contains(SOURCE_BUNDLE_COMMAND)
        })
        .collect()
}

fn shared_final_plan_invocations(steps: &[Value]) -> Vec<&str> {
    steps
        .iter()
        .filter_map(|step| step["run"].as_str())
        .filter(|run| {
            run.contains("cargo run")
                && run.contains("--locked")
                && run.contains("-p xtask")
                && run.contains("validate final")
                && run.contains("steps.validation-base.outputs.base")
        })
        .collect()
}

fn canonical_ci_trigger_paths() -> BTreeSet<String> {
    let routing = read_yaml("docs/owner-routing.yaml");
    routing["ci_trigger_policy"]["paths"]
        .as_sequence()
        .expect("canonical CI trigger paths")
        .iter()
        .map(|path| path.as_str().expect("CI trigger path").to_owned())
        .collect()
}

fn workflow_paths(workflow: &Value, event: &str) -> BTreeSet<String> {
    workflow["on"][event]["paths"]
        .as_sequence()
        .expect("workflow path filters")
        .iter()
        .map(|path| path.as_str().expect("workflow path filter").to_owned())
        .collect()
}

fn read_yaml(path: &str) -> Value {
    serde_yaml::from_str(
        &fs::read_to_string(repository_root().join(path)).expect("read YAML document"),
    )
    .expect("parse YAML document")
}

fn read_toml(path: &str) -> toml::Value {
    toml::from_str(&fs::read_to_string(repository_root().join(path)).expect("read Cargo manifest"))
        .expect("parse Cargo manifest")
}

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("release-integrity package is below repository root")
}
