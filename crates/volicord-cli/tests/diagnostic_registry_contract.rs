use serde_json::json;
use std::{error::Error, fs, path::PathBuf};

const SNAPSHOT: &str = "tests/fixtures/diagnostic-registry.json";
const UPDATE_ENV: &str = "VOLICORD_UPDATE_DIAGNOSTIC_REGISTRY";

#[test]
fn generated_diagnostic_registry_matches_typed_owners() -> Result<(), Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let snapshot_path = manifest_dir.join(SNAPSHOT);
    let generated = serde_json::to_string_pretty(&json!({
        "_generated": {
            "source": "volicord_cli::diagnostic_registry::current_diagnostic_codes",
            "check": "cargo test -p volicord-cli --test diagnostic_registry_contract",
            "update": "VOLICORD_UPDATE_DIAGNOSTIC_REGISTRY=1 cargo test -p volicord-cli --test diagnostic_registry_contract"
        },
        "codes": volicord_cli::diagnostic_registry::current_diagnostic_codes()
    }))? + "\n";

    if std::env::var_os(UPDATE_ENV).is_some() {
        fs::create_dir_all(
            snapshot_path
                .parent()
                .expect("diagnostic registry snapshot has a parent"),
        )?;
        fs::write(&snapshot_path, generated)?;
        return Ok(());
    }

    let checked_in = fs::read_to_string(&snapshot_path)?;
    assert_eq!(
        checked_in, generated,
        "generated diagnostic registry drifted; run the update command recorded in the artifact"
    );
    Ok(())
}
