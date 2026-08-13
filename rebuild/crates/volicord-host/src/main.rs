use volicord_host::{run_stdio, HostAdapter};
use volicord_operations::{LocalOperations, RuntimeLayout};

fn main() {
    let result = RuntimeLayout::from_environment()
        .map(LocalOperations::new)
        .map(HostAdapter::new)
        .and_then(|mut adapter| {
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
