//! A native fault owns one local geometry worker, never the reader process.
//! The transient transport carries only bounded graph rectangles/identities.
use super::engine::{Engine, DEADLINE, OUTPUT_LIMIT};
use rustix::{
    event::{poll, PollFd, PollFlags, Timespec},
    fs::{fcntl_getfl, fcntl_setfl, OFlags},
};
use std::{
    cell::RefCell,
    io::{self, Read, Write},
    os::{fd::AsFd, unix::process::ExitStatusExt},
    process::{Child, ChildStderr, ChildStdin, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};
const INPUT_LIMIT: usize = 65_536;
const HELLO: &[u8] = b"volicord-layout/1\n";
static FIRST_FAILURE_REPORTED: AtomicBool = AtomicBool::new(false);
thread_local! { static WORKER: RefCell<Option<Worker>> = const { RefCell::new(None) }; }

struct Worker {
    child: Child,
    input: ChildStdin,
    output: ChildStderr,
    stopped: bool,
    native_startup_diagnostic: Vec<u8>,
}

fn nonblocking(fd: &impl AsFd) -> Result<(), String> {
    let flags = fcntl_getfl(fd).map_err(|_| "local layout pipe unavailable")?;
    fcntl_setfl(fd, flags | OFlags::NONBLOCK).map_err(|_| "local layout pipe unavailable".into())
}
fn ready(fd: &impl AsFd, flags: PollFlags, end: Instant) -> Result<(), String> {
    loop {
        let remaining = end
            .checked_duration_since(Instant::now())
            .ok_or("local layout worker deadline exceeded")?;
        let timeout =
            Timespec::try_from(remaining).map_err(|_| "local layout deadline unavailable")?;
        match poll(&mut [PollFd::new(fd, flags)], Some(&timeout)) {
            Ok(0) => return Err("local layout worker deadline exceeded".into()),
            Ok(_) => return Ok(()),
            Err(rustix::io::Errno::INTR) => continue,
            Err(_) => return Err("local layout worker poll failed".into()),
        }
    }
}
fn read_exact(
    reader: &mut (impl Read + AsFd),
    mut bytes: &mut [u8],
    end: Instant,
) -> Result<(), String> {
    while !bytes.is_empty() {
        ready(reader, PollFlags::IN, end)?;
        match reader.read(bytes) {
            Ok(0) => return Err("local layout worker ended before a complete response".into()),
            Ok(count) => bytes = &mut bytes[count..],
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) => {}
            Err(_) => return Err("local layout worker read failed".into()),
        }
    }
    Ok(())
}
fn write_all(
    writer: &mut (impl Write + AsFd),
    mut bytes: &[u8],
    end: Instant,
) -> Result<(), String> {
    while !bytes.is_empty() {
        ready(writer, PollFlags::OUT, end)?;
        match writer.write(bytes) {
            Ok(0) => return Err("local layout worker stopped reading".into()),
            Ok(count) => bytes = &bytes[count..],
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) => {}
            Err(_) => return Err("local layout worker write failed".into()),
        }
    }
    Ok(())
}
fn outcome(status: std::process::ExitStatus) -> String {
    if let Some(code) = status.code() {
        format!("exit:{code}")
    } else if let Some(signal) = status.signal() {
        format!("signal:{signal}")
    } else {
        "unknown".into()
    }
}
impl Worker {
    fn command() -> Result<Command, String> {
        let executable =
            std::env::current_exe().map_err(|_| "local layout executable unavailable")?;
        #[cfg(test)]
        {
            let mut command = Command::new(executable);
            let test = TEST_WORKER
                .with(|slot| slot.replace("diagram::overview::worker::tests::worker_child"));
            command.args(["--ignored", "--exact", test, "--nocapture"]);
            command.env_clear().env("VOLICORD_LAYOUT_TEST_CHILD", "1");
            if let Some(value) = std::env::var_os("ASAN_OPTIONS") {
                command.env("ASAN_OPTIONS", value);
            }
            Ok(command)
        }
        #[cfg(not(test))]
        {
            let path = if executable
                .file_name()
                .is_some_and(|name| name == "volicord-viewer")
            {
                executable
            } else {
                let directory = executable
                    .parent()
                    .ok_or("local layout executable unavailable")?;
                let directory = if directory.file_name().is_some_and(|name| name == "deps") {
                    directory
                        .parent()
                        .ok_or("local layout executable unavailable")?
                } else {
                    directory
                };
                directory.join("volicord-viewer")
            };
            let mut command = Command::new(path);
            command.arg("--local-layout-worker").env_clear();
            Ok(command)
        }
    }
    fn start() -> Result<Self, String> {
        let end = Instant::now() + DEADLINE;
        let mut child = Self::command()?
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| "local layout worker could not start")?;
        let pipes = child.stdin.take().zip(child.stderr.take());
        let Some((input, output)) = pipes else {
            let _ = child.kill();
            let _ = child.wait();
            return Err("local layout worker pipes unavailable".into());
        };
        let mut worker = Self {
            child,
            input,
            output,
            stopped: false,
            native_startup_diagnostic: Vec::new(),
        };
        let result: Result<(), String> = (|| {
            nonblocking(&worker.input)?;
            nonblocking(&worker.output)?;
            let mut hello = [0; HELLO.len()];
            read_exact(&mut worker.output, &mut hello, end)?;
            if hello != HELLO {
                // No graph has been sent during initialization. Retain a bounded
                // native startup line, not arbitrary request or Source content.
                worker.native_startup_diagnostic.extend_from_slice(&hello);
                let mut more = [0; 1006];
                if let Ok(count) = worker.output.read(&mut more) {
                    worker
                        .native_startup_diagnostic
                        .extend_from_slice(&more[..count]);
                }
                return Err("local layout worker protocol rejected".into());
            }
            let ready = worker.response(end)?;
            if !ready.is_empty() {
                return Err("local layout worker readiness rejected".into());
            }
            Ok(())
        })();
        if let Err(error) = result {
            return Err(worker.failure("initialization", &error));
        }
        Ok(worker)
    }
    fn response(&mut self, end: Instant) -> Result<String, String> {
        let mut header = [0; 5];
        read_exact(&mut self.output, &mut header, end)?;
        let count = u32::from_be_bytes([header[1], header[2], header[3], header[4]]) as usize;
        if header[0] > 1 || count > OUTPUT_LIMIT || (header[0] == 1 && count > 512) {
            return Err("local layout worker response budget/protocol rejected".into());
        }
        let mut bytes = vec![0; count];
        read_exact(&mut self.output, &mut bytes, end)?;
        let value =
            String::from_utf8(bytes).map_err(|_| "local layout worker returned invalid UTF-8")?;
        if header[0] == 1 {
            // Engine errors have a closed, content-free diagnostic. Never echo
            // native stderr, arbitrary exception values or malformed transport.
            return Err(format!("local layout worker rejected execution: {value}"));
        }
        Ok(value)
    }
    fn layout(&mut self, input: &str) -> Result<String, String> {
        let end = Instant::now() + DEADLINE;
        write_all(&mut self.input, &(input.len() as u32).to_be_bytes(), end)?;
        write_all(&mut self.input, input.as_bytes(), end)?;
        let output = self.response(end)?;
        if output.is_empty() {
            return Err("local layout worker returned no geometry".into());
        }
        Ok(output)
    }
    fn failure(&mut self, stage: &str, primary: &str) -> String {
        let observed = self
            .child
            .try_wait()
            .ok()
            .flatten()
            .map(outcome)
            .unwrap_or_else(|| "running_or_unobserved".into());
        let cleanup = self.stop();
        let native = String::from_utf8_lossy(&self.native_startup_diagnostic)
            .escape_debug()
            .take(1024)
            .collect::<String>();
        format!("{primary}; stage={stage}; worker_pid={}; observed={observed}; cleanup={cleanup}; native_startup={native}",self.child.id())
    }
    fn stop(&mut self) -> String {
        if self.stopped {
            return "already_reaped".into();
        }
        let _ = self.child.kill();
        match self.child.wait() {
            Ok(status) => {
                self.stopped = true;
                outcome(status)
            }
            Err(_) => "unavailable".into(),
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        if !self.stopped {
            let _ = self.stop();
        }
    }
}

pub(super) fn layout(input: &str) -> Result<String, String> {
    if input.len() > INPUT_LIMIT {
        return Err("local layout input budget exceeded".into());
    }
    let result = WORKER.with(|slot| {
        let mut slot = slot
            .try_borrow_mut()
            .map_err(|_| "local layout already active")?;
        if slot.is_none() {
            *slot = Some(Worker::start()?);
        }
        let worker = slot.as_mut().ok_or("local layout worker unavailable")?;
        match worker.layout(input) {
            Ok(output) => Ok(output),
            Err(error) => {
                let failure = worker.failure("request", &error);
                *slot = None;
                Err(failure)
            }
        }
    });
    if let Err(error) = &result {
        if !FIRST_FAILURE_REPORTED.swap(true, Ordering::Relaxed) {
            // One content-free first failure per process. A closed diagnostic
            // sink cannot replace the actual geometry failure or stop reading.
            let bounded = error.chars().take(1024).collect::<String>();
            let _ = writeln!(
                io::stderr().lock(),
                "local diagram failure; engine={}; {bounded}",
                super::engine::ENGINE_ID
            );
        }
    }
    result
}
fn frame(output: &mut impl Write, failed: bool, bytes: &[u8]) -> io::Result<()> {
    output.write_all(&[u8::from(failed)])?;
    output.write_all(&(bytes.len() as u32).to_be_bytes())?;
    output.write_all(bytes)?;
    output.flush()
}
// The worker owns no Runtime Home, Source bodies or canonical handle. Kernel
// process disposal reclaims failed native state without calling into a damaged VM.
pub(super) fn run() -> i32 {
    // A reader that exits without Rust destructors must not leave native work
    // running. Check the parent again to cover death while installing the guard.
    let parent = rustix::process::getppid();
    if rustix::process::set_parent_process_death_signal(Some(rustix::process::Signal::KILL))
        .is_err()
        || rustix::process::getppid() != parent
    {
        return 2;
    }
    serve(Engine::new())
}
fn serve(engine: Result<Engine, String>) -> i32 {
    let mut input = io::stdin().lock();
    let mut output = io::stderr().lock();
    if output.write_all(HELLO).is_err() {
        return 2;
    }
    let engine = match engine {
        Ok(engine) => engine,
        Err(error) => {
            let _ = frame(&mut output, true, error.as_bytes());
            return 1;
        }
    };
    if frame(&mut output, false, &[]).is_err() {
        std::process::exit(2);
    }
    loop {
        let mut header = [0; 4];
        if input.read_exact(&mut header).is_err() {
            std::process::exit(0);
        }
        let count = u32::from_be_bytes(header) as usize;
        if count > INPUT_LIMIT {
            std::process::exit(2);
        }
        let mut bytes = vec![0; count];
        if input.read_exact(&mut bytes).is_err() {
            std::process::exit(2);
        }
        let Ok(graph) = std::str::from_utf8(&bytes) else {
            std::process::exit(2);
        };
        match engine.layout(graph) {
            Ok(value) => {
                if frame(&mut output, false, value.as_bytes()).is_err() {
                    std::process::exit(2);
                }
            }
            Err(error) => {
                let _ = frame(&mut output, true, error.as_bytes());
                std::process::exit(1);
            }
        }
    }
}

#[cfg(test)]
thread_local! {static TEST_WORKER: std::cell::Cell<&'static str> = const {std::cell::Cell::new("diagram::overview::worker::tests::worker_child")};}
#[cfg(test)]
pub(super) fn select_test_worker(test: &'static str) {
    WORKER.with(|slot| *slot.borrow_mut() = None);
    TEST_WORKER.with(|slot| slot.set(test));
}
#[cfg(test)]
mod tests {
    use super::*;
    const GRAPH: &str = r#"{"id":"root","layoutOptions":{"elk.algorithm":"layered"},"children":[{"id":"a","width":20,"height":20}]}"#;
    fn child() -> bool {
        std::env::var_os("VOLICORD_LAYOUT_TEST_CHILD").is_some()
    }
    #[test]
    #[ignore = "internal child entry, exercised by supervisor controls"]
    fn worker_child() {
        if child() {
            std::process::exit(run());
        }
    }
    #[test]
    #[ignore = "internal child entry, exercised by supervisor controls"]
    fn callback_failure_child() {
        if !child() {
            return;
        }
        let engine = Engine::new().and_then(|engine| {
            engine.enqueue_test_callback()?;
            Ok(engine)
        });
        std::process::exit(serve(engine));
    }
    #[test]
    #[ignore = "internal child entry, exercised by supervisor controls"]
    fn abrupt_child() {
        if child() {
            std::process::abort();
        }
    }
    #[test]
    #[ignore = "internal child entry, exercised by supervisor controls"]
    fn partial_child() {
        if !child() {
            return;
        }
        let mut output = io::stderr().lock();
        output.write_all(HELLO).unwrap();
        frame(&mut output, false, &[]).unwrap();
        let mut header = [0; 4];
        io::stdin().read_exact(&mut header).unwrap();
        output.write_all(&[0, 0, 0, 0, 20, b'x']).unwrap();
        std::process::exit(7);
    }
    #[test]
    #[ignore = "internal child entry, exercised by supervisor controls"]
    fn stalled_child() {
        if !child() {
            return;
        }
        let mut output = io::stderr().lock();
        output.write_all(HELLO).unwrap();
        frame(&mut output, false, &[]).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(60));
        std::process::exit(8);
    }
    #[test]
    #[ignore = "internal child entry, exercised by supervisor controls"]
    fn startup_diagnostic_child() {
        if !child() {
            return;
        }
        let _ = io::stderr().write_all(
            format!("controlled native startup diagnostic {}", "x".repeat(2048)).as_bytes(),
        );
        std::process::exit(12);
    }
    #[test]
    fn faults_partial_frames_and_timeouts_discard_the_process_and_recover() -> Result<(), String> {
        for test in [
            "abrupt_child",
            "partial_child",
            "stalled_child",
            "callback_failure_child",
            "startup_diagnostic_child",
        ] {
            let name = match test {
                "abrupt_child" => "diagram::overview::worker::tests::abrupt_child",
                "partial_child" => "diagram::overview::worker::tests::partial_child",
                "stalled_child" => "diagram::overview::worker::tests::stalled_child",
                "callback_failure_child" => {
                    "diagram::overview::worker::tests::callback_failure_child"
                }
                _ => "diagram::overview::worker::tests::startup_diagnostic_child",
            };
            select_test_worker(name);
            let failure = layout(GRAPH).expect_err("failed child cannot admit geometry");
            assert!(failure.contains("stage="), "{failure}");
            assert!(failure.contains("cleanup="), "{failure}");
            if test == "abrupt_child" {
                assert!(failure.contains("signal:6"), "{failure}");
            }
            if test == "partial_child" {
                assert!(failure.contains("exit:7"), "{failure}");
            }
            if test == "stalled_child" {
                assert!(failure.contains("deadline exceeded"), "{failure}");
                assert!(failure.contains("signal:9"), "{failure}");
            }
            if test == "startup_diagnostic_child" {
                assert!(
                    failure.contains("native_startup=controlled native startup diagnostic"),
                    "{failure}"
                );
                assert!(failure.len() < 1400, "native startup evidence is bounded");
            }
            if test == "callback_failure_child" {
                assert!(
                    failure.contains("asynchronous execution failed"),
                    "{failure}"
                );
            }
            assert!(WORKER.with(|slot| slot.borrow().is_none()));
            assert!(layout(GRAPH)?.contains("children"));
            assert!(layout(GRAPH)?.contains("children"), "supported reuse");
        }
        Ok(())
    }
    #[test]
    fn terminated_request_is_reaped_without_a_waitable_child() -> Result<(), String> {
        select_test_worker("diagram::overview::worker::tests::partial_child");
        let mut worker = Worker::start()?;
        let pid = rustix::process::Pid::from_raw(worker.child.id() as i32).ok_or("pid")?;
        let primary = worker.layout(GRAPH).expect_err("partial output is failure");
        let failure = worker.failure("request", &primary);
        assert!(failure.contains("exit:7"), "{failure}");
        assert!(matches!(
            rustix::process::waitpid(Some(pid), rustix::process::WaitOptions::NOHANG),
            Err(rustix::io::Errno::CHILD)
        ));
        Ok(())
    }
}
