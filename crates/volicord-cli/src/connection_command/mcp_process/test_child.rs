use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

const TEST_CHILD_PREFIX: &str = "mcp_test_child-";
const TEST_CHILD_VERSION: &[u8] = b"volicord-mcp-test-child-integration-verification-tools\n";
const TEST_CHILD_SCENARIO_ARGUMENT: &str = "--mcp-test-child-scenario";

static TEST_CHILD_PATH: OnceLock<PathBuf> = OnceLock::new();

pub(super) fn command(scenario: &str) -> Command {
    let path = TEST_CHILD_PATH.get_or_init(executable_path);
    let mut command = Command::new(path);
    command.arg(TEST_CHILD_SCENARIO_ARGUMENT).arg(scenario);
    command
}

fn executable_path() -> PathBuf {
    let test_executable = std::env::current_exe().expect("current test executable");
    let dependencies = test_executable
        .parent()
        .expect("Cargo test dependencies directory");
    let mut candidates = std::fs::read_dir(dependencies)
        .expect("read Cargo test dependencies directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| is_test_child_candidate(path))
        .collect::<Vec<_>>();
    candidates.sort();
    candidates
        .into_iter()
        .find(|path| {
            Command::new(path)
                .arg(TEST_CHILD_SCENARIO_ARGUMENT)
                .arg("protocol-version")
                .output()
                .is_ok_and(|output| output.status.success() && output.stdout == TEST_CHILD_VERSION)
        })
        .unwrap_or_else(|| {
            panic!(
                "current MCP test child was not built under {}",
                dependencies.display()
            )
        })
}

fn is_test_child_candidate(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let Some(hash_and_suffix) = name.strip_prefix(TEST_CHILD_PREFIX) else {
        return false;
    };
    let hash = hash_and_suffix
        .strip_suffix(std::env::consts::EXE_SUFFIX)
        .unwrap_or(hash_and_suffix);
    !hash.is_empty() && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
}
