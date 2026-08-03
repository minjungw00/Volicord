#![forbid(unsafe_code)]

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use serde_json::Value;
use volicord_store::schema::{
    current_storage_manifest, generated_schema_metadata, PROJECT_STATE_SCHEMA_SQL,
    REGISTRY_SCHEMA_SQL,
};

const CANONICAL_SCHEMA_SQL_DIRECTORY: &str = "crates/volicord-store/src/schema";

const CANONICAL_SCHEMA_SQL_PATHS: &[&str] = &[
    "crates/volicord-store/src/schema/registry.sql",
    "crates/volicord-store/src/schema/project.sql",
];

const SNAPSHOT_CANONICAL_PATHS: &[&str] = &[
    "tests/integration/snapshots/api_request_schema_contract.json",
    "tests/integration/snapshots/mcp_read_only_tools_contract.json",
    "tests/integration/snapshots/mcp_workflow_tools_contract.json",
];

const RAW_DIGEST_TEXT_PATHS: &[&str] = &["tests/agent-evaluation/fixtures/catalog.json"];

const LF_ONLY_NON_CANONICAL_PATHS: &[&str] = &[
    "tests/agent-evaluation/fixtures/catalog.json",
    "tests/release-integrity/version-identifier-allowlist.json",
];

const LF_JSON_DIRECTORIES: &[&str] = &[
    "tests/agent-evaluation/fixtures",
    "tests/integration/snapshots",
    "tests/release-integrity",
];

#[test]
fn canonical_repository_files_are_lf_and_match_deterministic_serialization() {
    let root = repository_root();
    let attributes = root.join(".gitattributes");
    assert!(attributes.is_file(), "root .gitattributes is missing");
    assert_canonical_path_registry_is_complete(&root);

    for relative_path in lf_controlled_paths(&root) {
        assert_lf_only(&root.join(relative_path));
    }
    for relative_path in SNAPSHOT_CANONICAL_PATHS {
        assert_pretty_json_is_canonical(&root.join(relative_path));
    }
    for relative_path in CANONICAL_SCHEMA_SQL_PATHS {
        assert_canonical_text_bytes(&root.join(relative_path));
    }
}

#[test]
fn canonical_byte_validation_accepts_lf_and_rejects_crlf() {
    let root = repository_root();

    for relative_path in CANONICAL_SCHEMA_SQL_PATHS {
        let canonical = fs::read(root.join(relative_path)).expect("read canonical schema SQL");
        validate_canonical_text_bytes(&canonical).unwrap_or_else(|error| {
            panic!("LF canonical SQL must pass for {relative_path}: {error}")
        });

        let crlf = crlf_mutation(&canonical);
        assert!(
            crlf.contains(&b'\r'),
            "CRLF mutation must add carriage returns"
        );
        let error = validate_canonical_text_bytes(&crlf)
            .expect_err("CRLF schema SQL bytes must be non-canonical");
        assert!(
            error.contains("carriage-return"),
            "unexpected canonical-byte error for {relative_path}: {error}"
        );
    }
}

#[test]
fn checked_in_schema_metadata_matches_fixed_storage_digest_vectors() {
    let root = repository_root();
    assert_eq!(
        REGISTRY_SCHEMA_SQL.as_bytes(),
        fs::read(root.join(CANONICAL_SCHEMA_SQL_PATHS[0])).expect("read checked-in registry SQL")
    );
    assert_eq!(
        PROJECT_STATE_SCHEMA_SQL.as_bytes(),
        fs::read(root.join(CANONICAL_SCHEMA_SQL_PATHS[1])).expect("read checked-in project SQL")
    );

    let metadata = generated_schema_metadata().expect("generate metadata from canonical SQL");
    assert_eq!(
        metadata.canonical_ddl_digest,
        "sha256:5609acd19ec28cbe1427aba1b41634a67379494885f296681b714bdff0cbbbb8"
    );
    assert_eq!(
        metadata.integrity_constraints_digest,
        "sha256:b5da1960b709aaea6e0326c4cec986a108daec15fe8f83fa193d13d0f324741f"
    );

    let manifest = current_storage_manifest().expect("build manifest from canonical SQL");
    assert_eq!(manifest.canonical_ddl_digest, metadata.canonical_ddl_digest);
    assert_eq!(
        manifest.integrity_constraints_digest,
        metadata.integrity_constraints_digest
    );
}

#[test]
fn canonical_contract_paths_resolve_to_text_with_lf() {
    let root = repository_root();
    let paths = lf_controlled_paths(&root);
    assert_effective_lf_attributes(&root, &paths);
}

#[test]
fn autocrlf_checkout_preserves_canonical_contract_bytes() {
    let root = repository_root();
    let temporary = tempfile::tempdir().expect("create isolated Git test root");
    let source = temporary.path().join("source");
    let checkout = temporary.path().join("checkout");
    fs::create_dir(&source).expect("create isolated source repository");

    let controlled_paths = lf_controlled_paths(&root);
    copy_paths(&root, &source, &controlled_paths);

    run_git(&source, &["init", "--quiet"]);
    run_git(&source, &["config", "core.autocrlf", "true"]);
    run_git(&source, &["config", "user.name", "Volicord LF test"]);
    run_git(
        &source,
        &["config", "user.email", "lf-test@volicord.invalid"],
    );
    run_git(&source, &["add", "--all"]);
    run_git(
        &source,
        &[
            "commit",
            "--quiet",
            "--no-gpg-sign",
            "--no-verify",
            "-m",
            "canonical LF fixture",
        ],
    );

    let mut clone_command = isolated_git_command();
    let clone = clone_command
        .args(["clone", "--quiet", "--no-checkout"])
        .arg(&source)
        .arg(&checkout)
        .output()
        .expect("run isolated git clone");
    require_success("git clone --no-checkout", &clone);
    run_git(&checkout, &["config", "core.autocrlf", "true"]);
    run_git(&checkout, &["checkout", "--quiet", "HEAD"]);

    assert_effective_lf_attributes(&checkout, &controlled_paths);
    for relative_path in &controlled_paths {
        assert_lf_only(&checkout.join(relative_path));
        assert_eq!(
            fs::read(root.join(relative_path)).expect("read canonical source bytes"),
            fs::read(checkout.join(relative_path)).expect("read isolated checkout bytes"),
            "isolated autocrlf checkout changed {relative_path}"
        );
    }
    for relative_path in SNAPSHOT_CANONICAL_PATHS {
        assert_pretty_json_is_canonical(&checkout.join(relative_path));
    }
    for relative_path in CANONICAL_SCHEMA_SQL_PATHS {
        assert_canonical_text_bytes(&checkout.join(relative_path));
    }
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("release-integrity package is below repository root")
        .to_path_buf()
}

fn lf_controlled_paths(root: &Path) -> Vec<String> {
    let mut paths = vec![".gitattributes".to_owned()];
    paths.extend(RAW_DIGEST_TEXT_PATHS.iter().map(|path| (*path).to_owned()));
    for directory in LF_JSON_DIRECTORIES {
        let absolute = root.join(directory);
        for entry in fs::read_dir(&absolute)
            .unwrap_or_else(|error| panic!("read {}: {error}", absolute.display()))
        {
            let path = entry.expect("read LF-controlled directory entry").path();
            if path.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension == "json")
            {
                paths.push(repository_path(root, &path));
            }
        }
    }
    paths.extend(schema_sql_paths(root));
    paths.sort();
    paths.dedup();
    paths
}

fn assert_canonical_path_registry_is_complete(root: &Path) {
    let mut discovered = LF_JSON_DIRECTORIES
        .iter()
        .flat_map(|directory| json_paths(root, directory))
        .filter(|path| {
            !LF_ONLY_NON_CANONICAL_PATHS
                .iter()
                .any(|excluded| path == excluded)
        })
        .collect::<Vec<_>>();
    discovered.sort();

    let mut registered = SNAPSHOT_CANONICAL_PATHS
        .iter()
        .map(|path| (*path).to_owned())
        .collect::<Vec<_>>();
    registered.sort();

    assert_eq!(
        discovered, registered,
        "every canonical JSON in an exact-byte directory must be registered for deterministic serialization validation"
    );

    let mut discovered_schema_sql = schema_sql_paths(root);
    discovered_schema_sql.sort();
    let mut registered_schema_sql = CANONICAL_SCHEMA_SQL_PATHS
        .iter()
        .map(|path| (*path).to_owned())
        .collect::<Vec<_>>();
    registered_schema_sql.sort();
    assert_eq!(
        discovered_schema_sql, registered_schema_sql,
        "every canonical schema SQL file must be registered in CANONICAL_SCHEMA_SQL_PATHS"
    );
}

fn schema_sql_paths(root: &Path) -> Vec<String> {
    let absolute = root.join(CANONICAL_SCHEMA_SQL_DIRECTORY);
    fs::read_dir(&absolute)
        .unwrap_or_else(|error| panic!("read {}: {error}", absolute.display()))
        .map(|entry| entry.expect("read canonical schema directory entry").path())
        .filter(|path| {
            path.is_file() && path.extension().is_some_and(|extension| extension == "sql")
        })
        .map(|path| repository_path(root, &path))
        .collect()
}

fn json_paths(root: &Path, directory: &str) -> Vec<String> {
    let absolute = root.join(directory);
    fs::read_dir(&absolute)
        .unwrap_or_else(|error| panic!("read {}: {error}", absolute.display()))
        .map(|entry| entry.expect("read LF-controlled directory entry").path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension == "json")
        })
        .map(|path| repository_path(root, &path))
        .collect()
}

fn repository_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("path must be below repository root")
        .to_string_lossy()
        .replace('\\', "/")
}

fn assert_lf_only(path: &Path) {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    assert!(
        !bytes.contains(&b'\r'),
        "{} contains a carriage-return byte",
        path.display()
    );
}

fn assert_canonical_text_bytes(path: &Path) {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    validate_canonical_text_bytes(&bytes)
        .unwrap_or_else(|error| panic!("{} is not canonical: {error}", path.display()));
}

fn validate_canonical_text_bytes(bytes: &[u8]) -> Result<(), &'static str> {
    if bytes.contains(&b'\r') {
        return Err("canonical text contains a carriage-return byte");
    }
    if !bytes.ends_with(b"\n") {
        return Err("canonical text must end with one final LF");
    }
    if bytes.ends_with(b"\n\n") {
        return Err("canonical text must not end with more than one final LF");
    }
    Ok(())
}

fn assert_pretty_json_is_canonical(path: &Path) {
    let checked_in =
        fs::read(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let value: Value = serde_json::from_slice(&checked_in)
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
    let canonical = serde_json::to_vec_pretty(&value)
        .unwrap_or_else(|error| panic!("serialize {}: {error}", path.display()));
    assert_eq!(
        checked_in,
        with_final_lf(canonical),
        "{} does not match deterministic pretty JSON plus one final LF",
        path.display()
    );
}

fn with_final_lf(mut bytes: Vec<u8>) -> Vec<u8> {
    bytes.push(b'\n');
    bytes
}

fn crlf_mutation(bytes: &[u8]) -> Vec<u8> {
    let mut mutated =
        Vec::with_capacity(bytes.len() + bytes.iter().filter(|byte| **byte == b'\n').count());
    for byte in bytes {
        if *byte == b'\n' {
            mutated.push(b'\r');
        }
        mutated.push(*byte);
    }
    mutated
}

fn copy_paths(source_root: &Path, destination_root: &Path, relative_paths: &[String]) {
    for relative_path in relative_paths {
        let source = source_root.join(relative_path);
        let destination = destination_root.join(relative_path);
        fs::create_dir_all(destination.parent().expect("copied path has a parent"))
            .expect("create copied path parent");
        fs::copy(&source, &destination).unwrap_or_else(|error| {
            panic!(
                "copy {} to {}: {error}",
                source.display(),
                destination.display()
            )
        });
    }
}

fn assert_effective_lf_attributes(root: &Path, paths: &[String]) {
    assert!(
        root.join(".gitattributes").is_file(),
        "root .gitattributes is missing"
    );
    let mut command = isolated_git_command();
    let output = command
        .current_dir(root)
        .args(["check-attr", "-z", "text", "eol", "--"])
        .args(paths)
        .output()
        .expect("run git check-attr");
    require_success("git check-attr", &output);

    let fields = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .map(|field| String::from_utf8(field.to_vec()).expect("git attribute output is UTF-8"))
        .collect::<Vec<_>>();
    assert_eq!(fields.len() % 3, 0, "git check-attr output is incomplete");

    let mut attributes = BTreeMap::new();
    for triple in fields.chunks_exact(3) {
        attributes.insert((triple[0].clone(), triple[1].clone()), triple[2].clone());
    }
    for path in paths {
        assert_eq!(
            attributes
                .get(&(path.clone(), "text".to_owned()))
                .map(String::as_str),
            Some("set"),
            "{path} must resolve to text=set"
        );
        assert_eq!(
            attributes
                .get(&(path.clone(), "eol".to_owned()))
                .map(String::as_str),
            Some("lf"),
            "{path} must resolve to eol=lf"
        );
    }
}

fn run_git(repository: &Path, args: &[&str]) {
    let mut command = isolated_git_command();
    let output = command
        .current_dir(repository)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("run git {}: {error}", args.join(" ")));
    require_success(&format!("git {}", args.join(" ")), &output);
}

fn isolated_git_command() -> Command {
    let mut command = Command::new("git");
    command.env("GIT_CONFIG_NOSYSTEM", "1");
    command.env(
        "GIT_CONFIG_GLOBAL",
        if cfg!(windows) { "NUL" } else { "/dev/null" },
    );
    command
}

fn require_success(label: &str, output: &Output) {
    assert!(
        output.status.success(),
        "{label} failed with {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
