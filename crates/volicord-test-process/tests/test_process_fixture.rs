use std::{
    env,
    ffi::OsStr,
    io::{self, Read, Write},
    process::{self, Command},
    thread,
};

const SCENARIO_ARGUMENT: &str = "--test-process-fixture";
const VERSION: &[u8] = b"volicord-test-process-fixture-current\n";

fn main() {
    let mut arguments = env::args_os().skip(1);
    if arguments.next().as_deref() != Some(OsStr::new(SCENARIO_ARGUMENT)) {
        return;
    }
    let scenario = arguments
        .next()
        .expect("fixture scenario")
        .into_string()
        .expect("UTF-8 fixture scenario");
    match scenario.as_str() {
        "version" => io::stdout().write_all(VERSION).expect("write version"),
        "stdout-stderr" => {
            io::stdout().write_all(b"fixture stdout\n").unwrap();
            io::stderr().write_all(b"fixture stderr\n").unwrap();
        }
        "echo-stdin" => {
            let mut input = Vec::new();
            io::stdin().read_to_end(&mut input).unwrap();
            io::stdout().write_all(&input).unwrap();
        }
        "exit-23" | "exit-immediately" => process::exit(23),
        "hang" | "hold-pipes" => park_forever(),
        "stdout-bytes" => write_repeated(&mut io::stdout(), b'o', count(&mut arguments)),
        "stderr-bytes" => write_repeated(&mut io::stderr(), b'e', count(&mut arguments)),
        "sustained-stderr" => {
            write_repeated(&mut io::stderr(), b'e', 256 * 1024);
            io::stdout().write_all(b"stdout remained active\n").unwrap();
            io::stdout().flush().unwrap();
            io::stderr().write_all(b"stderr complete\n").unwrap();
        }
        "descendant-retains-pipes" => {
            spawn_pipe_holding_descendant();
        }
        "paths-and-arguments" => {
            let argument = arguments.next().expect("argument with spaces");
            assert!(arguments.next().is_none(), "unexpected fixture argument");
            println!(
                "cwd={}\narg={}",
                env::current_dir().unwrap().display(),
                argument.to_string_lossy()
            );
        }
        "environment" => {
            println!(
                "added={}\nremoved={}",
                env::var("VOLICORD_TEST_PROCESS_ADDED").unwrap_or_default(),
                env::var_os("VOLICORD_TEST_PROCESS_REMOVED").is_none()
            );
        }
        other => panic!("unknown fixture scenario: {other}"),
    }
}

fn count(arguments: &mut impl Iterator<Item = std::ffi::OsString>) -> usize {
    arguments
        .next()
        .expect("byte count")
        .into_string()
        .expect("UTF-8 byte count")
        .parse()
        .expect("numeric byte count")
}

fn write_repeated(output: &mut impl Write, byte: u8, count: usize) {
    let chunk = vec![byte; 4096];
    let mut remaining = count;
    while remaining != 0 {
        let current = remaining.min(chunk.len());
        output.write_all(&chunk[..current]).unwrap();
        remaining -= current;
    }
    output.flush().unwrap();
}

#[allow(clippy::zombie_processes)]
fn spawn_pipe_holding_descendant() {
    let descendant = Command::new(env::current_exe().expect("fixture executable"))
        .arg(SCENARIO_ARGUMENT)
        .arg("hold-pipes")
        .spawn()
        .expect("spawn pipe-holding descendant");
    println!("descendant={}", descendant.id());
    io::stdout().flush().unwrap();
}

fn park_forever() -> ! {
    loop {
        thread::park();
    }
}
