use std::{
    env,
    error::Error,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;

#[test]
#[ignore = "requires an installed Codex host and an explicit live repository"]
fn live_codex_record_managed_stdio_smoke() -> Result<(), Box<dyn Error>> {
    let repo = PathBuf::from(
        env::var_os("VOLICORD_LIVE_CODEX_SMOKE_REPO")
            .ok_or("set VOLICORD_LIVE_CODEX_SMOKE_REPO to an existing Git worktree")?,
    );
    let codex = env::var_os("CODEX_BIN").unwrap_or_else(|| "codex".into());
    let codex_version = Command::new(codex).arg("--version").output()?;
    assert!(
        codex_version.status.success(),
        "Codex --version failed: {}",
        String::from_utf8_lossy(&codex_version.stderr)
    );

    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let runtime_home = env::temp_dir().join(format!(
        "volicord-live-codex-record-{}-{nonce}",
        std::process::id()
    ));
    let output = Command::new(env!("CARGO_BIN_EXE_volicord"))
        .args(["init", "--host", "codex", "--profile", "record"])
        .arg("--repo")
        .arg(&repo)
        .arg("--home")
        .arg(&runtime_home)
        .args(["--dry-run", "--json"])
        .output()?;
    assert!(
        output.status.success(),
        "Volicord Codex Record dry run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(value["host"], "codex");
    assert_eq!(value["selected_profile"], "record");
    assert_eq!(value["control_surface"]["selected_profile"], "record");
    assert_eq!(value["control_surface"]["actor_identity_provable"], false);
    Ok(())
}
