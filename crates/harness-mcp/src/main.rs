#![forbid(unsafe_code)]

use std::process;

fn main() {
    if let Err(error) = harness_mcp::run_stdio_from_env() {
        eprintln!("error: {error}");
        process::exit(1);
    }
}
