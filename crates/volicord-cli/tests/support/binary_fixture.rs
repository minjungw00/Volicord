use std::{
    error::Error,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Output, Stdio},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use volicord_store::{
    agent_connections::CONNECTION_MODE_WORKFLOW,
    bootstrap::{
        initialize_runtime_home, write_installation_profile, InstallationProfileRegistration,
    },
};
use volicord_test_support::TempRuntimeHome;

#[cfg(unix)]
use super::assertions::{stderr, stdout};

const PROCESS_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) fn run_without_home<const N: usize>(args: [&str; N]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(volicord_bin()).args(args).output()?)
}

pub(crate) fn run_with_home_env<const N: usize>(
    runtime_home: &Path,
    args: [&str; N],
    envs: &[(&str, String)],
) -> Result<Output, Box<dyn Error>> {
    let mut command = Command::new(volicord_bin());
    command.args(args).env("VOLICORD_HOME", runtime_home);
    for (name, value) in envs {
        command.env(name, value);
    }
    Ok(command.output()?)
}

pub(crate) fn run_with_home_env_in_dir<const N: usize>(
    runtime_home: &Path,
    args: [&str; N],
    envs: &[(&str, String)],
    current_dir: &Path,
) -> Result<Output, Box<dyn Error>> {
    let mut command = Command::new(volicord_bin());
    command
        .args(args)
        .env("VOLICORD_HOME", runtime_home)
        .current_dir(current_dir);
    for (name, value) in envs {
        command.env(name, value);
    }
    Ok(command.output()?)
}

pub(crate) fn run_without_binding<const N: usize>(
    args: [&str; N],
) -> Result<Output, Box<dyn Error>> {
    let mut command = base_command();
    command.arg("mcp");
    command.args(args);
    Ok(command.output()?)
}

pub(crate) fn base_command() -> Command {
    let mut command = Command::new(volicord_bin());
    command.env_clear();
    #[cfg(target_os = "linux")]
    if std::fs::read_to_string("/proc/sys/kernel/osrelease").is_ok_and(|release| {
        let release = release.to_ascii_lowercase();
        release.contains("microsoft-standard") || release.contains("wsl2")
    }) {
        command.env(
            "WSL_DISTRO_NAME",
            volicord_types::PINNED_WSL2_DISTRIBUTION_NAME,
        );
    }
    command.current_dir(env!("CARGO_MANIFEST_DIR"));
    command
}

pub(crate) fn prepare_runtime_home(
    runtime_home: &Path,
    mcp_command: &Path,
) -> Result<(), Box<dyn Error>> {
    initialize_runtime_home(runtime_home, "runtime_home_binary_admin_fixture", "{}")?;
    write_installation_profile(
        runtime_home,
        InstallationProfileRegistration {
            installation_id: "default".to_owned(),
            volicord_command: path_text(mcp_command),
            volicord_mcp_command: path_text(mcp_command),
            bin_dir: mcp_command
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| runtime_home.join("bin")),
            default_connection_mode: CONNECTION_MODE_WORKFLOW.to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    Ok(())
}

pub(crate) fn write_test_installation_profile(runtime_home: &Path) -> Result<(), Box<dyn Error>> {
    write_installation_profile(
        runtime_home,
        InstallationProfileRegistration {
            installation_id: "default".to_owned(),
            volicord_command: "volicord".to_owned(),
            volicord_mcp_command: "volicord".to_owned(),
            bin_dir: runtime_home.join("bin"),
            default_connection_mode: CONNECTION_MODE_WORKFLOW.to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    Ok(())
}

pub(crate) fn create_git_repo(
    runtime_home: &TempRuntimeHome,
    name: impl AsRef<Path>,
) -> Result<PathBuf, Box<dyn Error>> {
    let repo_root = runtime_home.create_product_repo(name)?;
    std::fs::create_dir_all(repo_root.join(".git"))?;
    Ok(repo_root)
}

#[cfg(unix)]
pub(crate) fn create_real_git_repo(
    runtime_home: &TempRuntimeHome,
    name: impl AsRef<Path>,
) -> Result<PathBuf, Box<dyn Error>> {
    let repo_root = runtime_home.create_product_repo(name)?;
    init_real_git_repo(&repo_root)?;
    Ok(repo_root)
}

#[cfg(unix)]
fn init_real_git_repo(repo_root: &Path) -> Result<(), Box<dyn Error>> {
    let output = Command::new("git")
        .arg("init")
        .arg("-q")
        .current_dir(repo_root)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "git init failed\nstdout:\n{}\nstderr:\n{}",
            stdout(&output),
            stderr(&output)
        )
        .into());
    }
    Ok(())
}

pub(crate) fn path_text(path: &Path) -> String {
    path.display().to_string()
}

pub(crate) fn volicord_bin() -> &'static str {
    env!("CARGO_BIN_EXE_volicord")
}

pub(crate) enum ChildStdin {
    KeepOpen,
    WriteAndClose(String),
}

pub(crate) struct CapturedChildOutput {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

struct RunningChild {
    child: Option<Child>,
    stdout: Option<JoinHandle<io::Result<Vec<u8>>>>,
    stderr: Option<JoinHandle<io::Result<Vec<u8>>>>,
}

impl RunningChild {
    fn spawn(mut command: Command, stdin: ChildStdin) -> io::Result<Self> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("stdout was not piped"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| io::Error::other("stderr was not piped"))?;
        let stdout = thread::spawn(move || read_to_end(stdout));
        let stderr = thread::spawn(move || read_to_end(stderr));

        match stdin {
            ChildStdin::KeepOpen => {}
            ChildStdin::WriteAndClose(input) => {
                let mut child_stdin = child
                    .stdin
                    .take()
                    .ok_or_else(|| io::Error::other("stdin was not piped"))?;
                child_stdin.write_all(input.as_bytes())?;
            }
        }

        Ok(Self {
            child: Some(child),
            stdout: Some(stdout),
            stderr: Some(stderr),
        })
    }

    fn wait(mut self, timeout: Duration) -> io::Result<CapturedChildOutput> {
        let started = Instant::now();
        loop {
            let child = self
                .child
                .as_mut()
                .ok_or_else(|| io::Error::other("child already reaped"))?;
            if let Some(status) = child.try_wait()? {
                self.child.take();
                return Ok(CapturedChildOutput {
                    status,
                    stdout: join_reader(self.stdout.take())?,
                    stderr: join_reader(self.stderr.take())?,
                });
            }
            if started.elapsed() >= timeout {
                let _ = child.kill();
                let _ = child.wait();
                let stdout = join_reader(self.stdout.take()).unwrap_or_default();
                let stderr = join_reader(self.stderr.take()).unwrap_or_default();
                self.child.take();
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!(
                        "child process timed out after {:?}\nstdout:\n{}\nstderr:\n{}",
                        timeout,
                        String::from_utf8_lossy(&stdout),
                        String::from_utf8_lossy(&stderr)
                    ),
                ));
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for RunningChild {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(stdout) = self.stdout.take() {
            let _ = stdout.join();
        }
        if let Some(stderr) = self.stderr.take() {
            let _ = stderr.join();
        }
    }
}

fn read_to_end(mut reader: impl Read) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();
    reader.read_to_end(&mut output)?;
    Ok(output)
}

fn join_reader(handle: Option<JoinHandle<io::Result<Vec<u8>>>>) -> io::Result<Vec<u8>> {
    let handle = handle.ok_or_else(|| io::Error::other("missing reader"))?;
    handle
        .join()
        .map_err(|_| io::Error::other("reader thread panicked"))?
}

pub(crate) fn run_child(
    command: Command,
    stdin: ChildStdin,
) -> Result<CapturedChildOutput, Box<dyn Error>> {
    Ok(RunningChild::spawn(command, stdin)?.wait(PROCESS_TIMEOUT)?)
}
