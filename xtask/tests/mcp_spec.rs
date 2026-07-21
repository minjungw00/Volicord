use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

const COMMIT: &str = "38c84e9f93ad191d9eb26d92b945d17bd0efcaf3";
const ATTRIBUTION: &str = "Copyright (c) MCP fixture authors";
const LICENSE_TEXT: &str = "MIT License\n\nCopyright (c) MCP fixture authors\n";

#[derive(Clone)]
struct RevisionFixture {
    protocol_version: &'static str,
    release_status: &'static str,
    handshake_family: &'static str,
    upstream_release: &'static str,
    production_supported: bool,
    pre_release_only: bool,
    upstream_commit: String,
}

fn released_revisions() -> Vec<RevisionFixture> {
    [
        "2024-10-07",
        "2024-11-05",
        "2025-03-26",
        "2025-06-18",
        "2025-11-25",
    ]
    .into_iter()
    .map(|protocol_version| RevisionFixture {
        protocol_version,
        release_status: "released",
        handshake_family: "initialization-based",
        upstream_release: protocol_version,
        production_supported: true,
        pre_release_only: false,
        upstream_commit: COMMIT.to_owned(),
    })
    .collect()
}

fn draft_revision() -> RevisionFixture {
    RevisionFixture {
        protocol_version: "2026-07-28",
        release_status: "release-candidate",
        handshake_family: "per-request-metadata",
        upstream_release: "2026-07-28-RC",
        production_supported: false,
        pre_release_only: true,
        upstream_commit: COMMIT.to_owned(),
    }
}

fn all_revisions() -> Vec<RevisionFixture> {
    let mut revisions = released_revisions();
    revisions.push(draft_revision());
    revisions
}

fn fixture(
    revisions: &[RevisionFixture],
    checksum_mismatch: Option<&str>,
    missing_schema: Option<&str>,
) -> TempDir {
    let temp = tempfile::tempdir().expect("tempdir");
    let license_path = temp.path().join("licenses/MIT.txt");
    fs::create_dir_all(license_path.parent().expect("license parent"))
        .expect("create license directory");
    fs::write(&license_path, LICENSE_TEXT).expect("write license");

    let mut manifest = format!(
        "format_version = 1\nupstream_repository = \"https://github.com/modelcontextprotocol/modelcontextprotocol.git\"\n\n[[license]]\nid = \"test-license\"\nspdx_expression = \"MIT\"\nattribution = \"{ATTRIBUTION}\"\nupstream_release = \"2025-11-25\"\nupstream_commit = \"{COMMIT}\"\nupstream_path = \"LICENSE\"\nlocal_path = \"licenses/MIT.txt\"\nsha256 = \"{}\"\n",
        sha256(LICENSE_TEXT.as_bytes())
    );

    for revision in revisions {
        let local_path = if revision.pre_release_only {
            format!("draft/{}/schema.json", revision.protocol_version)
        } else {
            format!("{}/schema.json", revision.protocol_version)
        };
        let schema = match revision.handshake_family {
            "initialization-based" => {
                r#"{"definitions":{"InitializeRequest":{},"InitializeResult":{}}}"#
            }
            "per-request-metadata" => {
                r#"{"$defs":{"RequestMetaObject":{"properties":{"io.modelcontextprotocol/protocolVersion":{}}}}}"#
            }
            family => panic!("unknown fixture family {family}"),
        };
        if missing_schema != Some(revision.protocol_version) {
            write(temp.path(), &local_path, schema.as_bytes());
        }
        let checksum = if checksum_mismatch == Some(revision.protocol_version) {
            "0".repeat(64)
        } else {
            sha256(schema.as_bytes())
        };
        write!(
            &mut manifest,
            "\n[[revision]]\nprotocol_version = \"{}\"\nrelease_status = \"{}\"\nhandshake_family = \"{}\"\nupstream_release = \"{}\"\nupstream_commit = \"{}\"\nlicense_id = \"test-license\"\nproduction_supported = {}\nconformance_tested = false\npre_release_only = {}\n\n[[revision.artifact]]\nupstream_path = \"schema/{}/schema.json\"\nlocal_path = \"{}\"\nsha256 = \"{}\"\n",
            revision.protocol_version,
            revision.release_status,
            revision.handshake_family,
            revision.upstream_release,
            revision.upstream_commit,
            revision.production_supported,
            revision.pre_release_only,
            revision.protocol_version,
            local_path,
            checksum
        )
        .expect("render revision");
    }
    fs::write(temp.path().join("manifest.toml"), manifest).expect("write manifest");
    temp
}

fn write(root: &Path, relative: &str, contents: &[u8]) {
    let destination = root.join(relative);
    fs::create_dir_all(destination.parent().expect("artifact parent"))
        .expect("create artifact directory");
    fs::write(destination, contents).expect("write artifact");
}

fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut value = String::with_capacity(64);
    for byte in digest {
        write!(&mut value, "{byte:02x}").expect("format checksum");
    }
    value
}

#[test]
fn all_required_released_revisions_are_present_and_offline_check_succeeds() {
    let fixture = fixture(&all_revisions(), None, None);

    let report = xtask::check_mcp_spec_fixture(fixture.path()).expect("offline fixture check");

    assert_eq!(report.revision_count(), 6);
    assert_eq!(report.production_supported_count(), 5);
    assert_eq!(report.pre_release_only_count(), 1);
}

#[test]
fn rejects_a_missing_required_released_revision() {
    let mut revisions = all_revisions();
    revisions.retain(|revision| revision.protocol_version != "2024-10-07");
    let fixture = fixture(&revisions, None, None);

    let error = xtask::check_mcp_spec_fixture(fixture.path())
        .expect_err("missing released revision must fail");

    assert!(error
        .to_string()
        .contains("required released MCP revision 2024-10-07 is missing"));
}

#[test]
fn rejects_duplicate_protocol_strings() {
    let mut revisions = all_revisions();
    revisions.push(revisions[0].clone());
    let fixture = fixture(&revisions, None, None);

    let error = xtask::check_mcp_spec_fixture(fixture.path())
        .expect_err("duplicate protocol string must fail");

    assert!(error
        .to_string()
        .contains("duplicate MCP protocol string 2024-10-07"));
}

#[test]
fn rejects_pre_release_production_support() {
    let mut revisions = all_revisions();
    let draft = revisions.last_mut().expect("draft revision");
    draft.production_supported = true;
    let fixture = fixture(&revisions, None, None);

    let error = xtask::check_mcp_spec_fixture(fixture.path())
        .expect_err("production-supported pre-release must fail");

    assert!(error
        .to_string()
        .contains("must remain pre-release-only and outside production support"));
}

#[test]
fn rejects_checksum_mismatch() {
    let fixture = fixture(&all_revisions(), Some("2025-06-18"), None);

    let error =
        xtask::check_mcp_spec_fixture(fixture.path()).expect_err("checksum mismatch must fail");

    assert!(error.to_string().contains("checksum mismatch"));
}

#[test]
fn rejects_missing_schema() {
    let fixture = fixture(&all_revisions(), None, Some("2025-03-26"));

    let error =
        xtask::check_mcp_spec_fixture(fixture.path()).expect_err("missing schema must fail");

    assert!(error.to_string().contains("missing schema artifact"));
}

#[test]
fn rejects_non_deterministic_revision_ordering() {
    let mut revisions = all_revisions();
    revisions.reverse();
    let fixture = fixture(&revisions, None, None);

    let error = xtask::check_mcp_spec_fixture(fixture.path())
        .expect_err("non-deterministic ordering must fail");

    assert!(error
        .to_string()
        .contains("protocol versions must be unique and sorted"));
}

#[test]
fn rejects_mutable_upstream_reference() {
    let mut revisions = all_revisions();
    revisions[0].upstream_commit = "main".to_owned();
    let fixture = fixture(&revisions, None, None);

    let error = xtask::check_mcp_spec_fixture(fixture.path())
        .expect_err("mutable upstream reference must fail");

    assert!(error
        .to_string()
        .contains("full lowercase immutable upstream commit"));
}
