use volicord_host::{run_stdio, HostAdapter};
use volicord_operations::{LocalOperations, RuntimeLayout};

fn main() {
    let result = RuntimeLayout::from_environment()
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
        });
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
