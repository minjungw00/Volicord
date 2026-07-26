use super::*;

#[test]
fn property_arbitrary_json_rpc_values_never_panic() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-arbitrary-json-rpc-property")?;
    let connection_adapter = adapter(&fixture)?;
    for seed in 0_u64..2_048 {
        let message = generated_json_rpc_value(seed, 0);
        let result = catch_unwind(AssertUnwindSafe(|| {
            let mut state = session_state();
            let _ = apply_json_rpc_message(&connection_adapter, &mut state, message);
        }));
        assert!(result.is_ok(), "JSON-RPC input seed {seed} panicked");
    }
    Ok(())
}
