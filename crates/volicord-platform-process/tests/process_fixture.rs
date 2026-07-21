use std::{
    io::{self, Read, Write},
    process::Command,
    thread,
};

const SCENARIO_ARGUMENT: &str = "--platform-process-fixture";
const VERSION: &[u8] = b"volicord-platform-process-fixture-current\n";

fn main() {
    let mut arguments = std::env::args().skip(1);
    if arguments.next().as_deref() != Some(SCENARIO_ARGUMENT) {
        return;
    }
    let scenario = arguments.next().expect("fixture scenario");
    assert!(arguments.next().is_none(), "unexpected fixture argument");

    match scenario.as_str() {
        "version" => io::stdout().write_all(VERSION).expect("write version"),
        "wait-stdin" => {
            let mut input = Vec::new();
            io::stdin().read_to_end(&mut input).expect("read stdin");
        }
        "write-then-wait" => {
            io::stdout().write_all(b"ready").expect("write stdout");
            io::stdout().flush().expect("flush stdout");
            let mut input = Vec::new();
            io::stdin().read_to_end(&mut input).expect("read stdin");
        }
        "spawn-descendant" => {
            let mut signal = [0_u8; 1];
            io::stdin()
                .read_exact(&mut signal)
                .expect("read spawn signal");
            spawn_descendant();
            io::stdout()
                .write_all(b"spawned\n")
                .expect("write descendant marker");
            io::stdout().flush().expect("flush descendant marker");
            loop {
                thread::park();
            }
        }
        "wait-forever" => loop {
            thread::park();
        },
        "close-stdout" => {}
        other => panic!("unknown fixture scenario: {other}"),
    }
}

#[allow(clippy::zombie_processes)]
fn spawn_descendant() {
    // The containment test deliberately terminates this process through the
    // parent process group or Job Object, so the fixture must not wait here.
    Command::new(std::env::current_exe().expect("current fixture executable"))
        .arg(SCENARIO_ARGUMENT)
        .arg("wait-forever")
        .spawn()
        .expect("spawn descendant");
}
