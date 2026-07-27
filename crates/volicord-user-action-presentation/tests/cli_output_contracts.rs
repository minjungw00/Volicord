use serde_json::json;
use std::{error::Error, fs, path::PathBuf};

const SNAPSHOT: &str = "tests/fixtures/cli-output-contracts.json";
const UPDATE_ENV: &str = "VOLICORD_UPDATE_CLI_OUTPUT_CONTRACTS";

#[test]
fn generated_cli_output_contracts_match_the_typed_owner() -> Result<(), Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let snapshot_path = manifest_dir.join(SNAPSHOT);
    let contracts = volicord_user_action_presentation::cli_output_contract_descriptors()
        .into_iter()
        .map(|descriptor| {
            json!({
                "id": descriptor.id(),
                "properties": descriptor.identifiers().properties(),
                "values": descriptor.identifiers().values(),
                "schema_names": descriptor.identifiers().schema_names(),
                "related_contracts": descriptor.related_contracts(),
                "example_schemas": descriptor.example_schemas(),
            })
        })
        .collect::<Vec<_>>();
    let generated = serde_json::to_string_pretty(&json!({
        "_generated": {
            "source": "volicord_user_action_presentation::cli_output_contract_descriptors",
            "check": "cargo test -p volicord-user-action-presentation --test cli_output_contracts",
            "update": "VOLICORD_UPDATE_CLI_OUTPUT_CONTRACTS=1 cargo test -p volicord-user-action-presentation --test cli_output_contracts"
        },
        "contracts": contracts
    }))? + "\n";

    if std::env::var_os(UPDATE_ENV).is_some() {
        fs::create_dir_all(
            snapshot_path
                .parent()
                .expect("CLI output descriptor snapshot has a parent"),
        )?;
        fs::write(&snapshot_path, generated)?;
        return Ok(());
    }

    let checked_in = fs::read_to_string(&snapshot_path)?;
    assert_eq!(
        checked_in, generated,
        "generated CLI output contracts drifted; run the update command recorded in the artifact"
    );
    Ok(())
}
