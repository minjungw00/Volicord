use std::{env, io};

fn main() {
    let exit = volicord_operations::run_cli_with_input(
        env::args_os().skip(1),
        &mut io::stdin().lock(),
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    );
    std::process::exit(exit.code());
}
