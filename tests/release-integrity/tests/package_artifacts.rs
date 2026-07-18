#![cfg(unix)]

use std::{fs, path::Path, process::Command};

use sha2::{Digest, Sha256};
use volicord_types::ReleaseTargetTriple;

#[test]
fn package_script_rehashes_every_archived_volicord_binary() {
    let root = repository_root();
    let temporary = tempfile::tempdir().expect("temporary release roots");
    let builds = temporary.path().join("builds");
    let dist = temporary.path().join("dist");
    fs::create_dir(&builds).expect("build root");

    for target in published_targets() {
        let artifact = builds.join(format!("volicord-build-{target}-123-1"));
        fs::create_dir(&artifact).expect("artifact directory");
        let binary_name = if target == ReleaseTargetTriple::X86_64PcWindowsMsvc {
            "volicord.exe"
        } else {
            "volicord"
        };
        let bytes = format!("synthetic Volicord binary for {target}\n").into_bytes();
        fs::write(artifact.join(binary_name), &bytes).expect("synthetic binary");
        let digest = format!("{:x}", Sha256::digest(&bytes));
        fs::write(
            artifact.join("volicord.sha256"),
            format!("{digest}  {binary_name}\n"),
        )
        .expect("binary digest");
    }

    let output = Command::new("sh")
        .arg(root.join("scripts/package-release-artifacts.sh"))
        .arg(&builds)
        .arg(&dist)
        .arg("123")
        .arg("1")
        .output()
        .expect("run package script");
    assert!(
        output.status.success(),
        "package script failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    for target in published_targets() {
        let extension = if target == ReleaseTargetTriple::X86_64PcWindowsMsvc {
            "zip"
        } else {
            "tar.gz"
        };
        let archive = dist.join(format!("volicord-{target}.{extension}"));
        assert!(archive.is_file(), "missing {}", archive.display());
        let checksum = dist.join(format!("volicord-{target}.{extension}.sha256"));
        assert!(checksum.is_file(), "missing {}", checksum.display());
    }
}

fn published_targets() -> [ReleaseTargetTriple; 5] {
    [
        ReleaseTargetTriple::X86_64UnknownLinuxGnu,
        ReleaseTargetTriple::Aarch64UnknownLinuxGnu,
        ReleaseTargetTriple::Aarch64AppleDarwin,
        ReleaseTargetTriple::X86_64AppleDarwin,
        ReleaseTargetTriple::X86_64PcWindowsMsvc,
    ]
}

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("release-integrity package is below repository root")
}
