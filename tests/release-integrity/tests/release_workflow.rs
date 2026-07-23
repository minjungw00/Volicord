use std::{collections::BTreeSet, fs, path::Path};

use serde_yaml::Value;
use volicord_types::ReleaseTargetTriple;

#[test]
fn maintained_workflow_yaml_files_parse() {
    let root = repository_root();
    for path in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
        let text = fs::read_to_string(root.join(path)).expect("read workflow");
        serde_yaml::from_str::<Value>(&text).expect("parse workflow YAML");
    }
}

#[test]
fn release_workflow_builds_and_packages_every_published_target() {
    let root = repository_root();
    let workflow: Value = serde_yaml::from_str(
        &fs::read_to_string(root.join(".github/workflows/release.yml"))
            .expect("read release workflow"),
    )
    .expect("parse release workflow YAML");
    let jobs = workflow["jobs"].as_mapping().expect("jobs mapping");
    let build = jobs
        .get(Value::String("build-binaries".to_owned()))
        .expect("build-binaries job");
    let targets = build["strategy"]["matrix"]["include"]
        .as_sequence()
        .expect("build matrix")
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
    let binaries = build["strategy"]["matrix"]["include"]
        .as_sequence()
        .expect("build matrix")
        .iter()
        .map(|entry| {
            (
                entry["target"].as_str().expect("target string"),
                entry["binary"].as_str().expect("binary string"),
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        binaries,
        BTreeSet::from([
            ("aarch64-apple-darwin", "volicord"),
            ("aarch64-unknown-linux-gnu", "volicord"),
            ("x86_64-apple-darwin", "volicord"),
            ("x86_64-pc-windows-msvc", "volicord.exe"),
            ("x86_64-unknown-linux-gnu", "volicord"),
        ])
    );
    let build_runs = build["steps"]
        .as_sequence()
        .expect("build steps")
        .iter()
        .filter_map(|step| step["run"].as_str())
        .collect::<Vec<_>>();
    let smoke_runs = build_runs
        .iter()
        .filter(|run| run.contains("release-binary-smoke"))
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(
        smoke_runs,
        vec![
            "cargo run --locked -p xtask -- release-binary-smoke --bin \"target/${{ matrix.target }}/release/${{ matrix.binary }}\""
        ]
    );

    let publish = jobs
        .get(Value::String("publish-release".to_owned()))
        .expect("publish-release job");
    let runs = publish["steps"]
        .as_sequence()
        .expect("publish steps")
        .iter()
        .filter_map(|step| step["run"].as_str())
        .collect::<Vec<_>>();
    assert!(runs
        .iter()
        .any(|run| run.contains("scripts/package-release-artifacts.sh")));
}

#[test]
fn ordinary_ci_builds_and_runs_the_canonical_binary_smoke_harness() {
    let root = repository_root();
    let workflow: Value = serde_yaml::from_str(
        &fs::read_to_string(root.join(".github/workflows/ci.yml")).expect("read CI workflow"),
    )
    .expect("parse CI workflow YAML");
    let steps = workflow["jobs"]["checks"]["steps"]
        .as_sequence()
        .expect("CI check steps");
    let runs = steps
        .iter()
        .filter_map(|step| step["run"].as_str())
        .collect::<Vec<_>>();
    assert!(runs.contains(&"cargo build --locked -p volicord-cli --bin volicord"));
    assert!(runs.contains(
        &"cargo run --locked -p xtask -- release-binary-smoke --bin target/debug/volicord"
    ));
}

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("release-integrity package is below repository root")
}
