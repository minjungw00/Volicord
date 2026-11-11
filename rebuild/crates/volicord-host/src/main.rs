use volicord_host::{run_stdio, tool_definition, HostAdapter};
use volicord_operations::{LocalOperations, RuntimeLayout};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), volicord_operations::Error> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if !arguments.is_empty() {
        if arguments.len() != 2 || arguments[0] != "--tool-schema" {
            return Err(volicord_operations::Error::new(
                "usage: volicord-mcp [--tool-schema TOOL]",
            ));
        }
        let definition = arguments[1]
            .to_str()
            .and_then(tool_definition)
            .ok_or_else(|| volicord_operations::Error::new("unknown MCP tool name"))?;
        println!("{definition}");
        return Ok(());
    }
    RuntimeLayout::from_environment()
        .map(LocalOperations::new)
        .map(HostAdapter::new)
        .and_then(|mut adapter| {
            #[cfg(target_os = "linux")]
            let _lifecycle = match volicord_host::McpLifecycle::register(
                adapter.operations().layout().root(),
                adapter.host_session(),
            ) {
                Ok(registration) => Some(registration),
                Err(_) => {
                    eprintln!("MCP lifecycle observation unavailable");
                    None
                }
            };
            run_stdio(
                &mut adapter,
                std::io::stdin().lock(),
                std::io::stdout().lock(),
            )
            .map_err(|error| volicord_operations::Error::new(error.to_string()))
        })
}
