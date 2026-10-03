//! Observational Linux process registration. It never owns canonical authority.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    time::Instant,
};

pub struct McpLifecycle {
    path: PathBuf,
    value: Value,
    started: Instant,
}

fn digest_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut chunk = [0_u8; 65536];
    loop {
        let count = file.read(&mut chunk)?;
        if count == 0 {
            break;
        }
        hash.update(&chunk[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn path_hash(path: &Path) -> String {
    format!("{:x}", Sha256::digest(path.as_os_str().as_encoded_bytes()))
}
fn publish(path: &Path, value: &Value) -> io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let temporary = path.with_extension("pending");
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)?;
        file.write_all(&serde_json::to_vec(value)?)?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
impl McpLifecycle {
    /// Missing /proc or registration failure cannot block the stdio Product.
    pub fn register(runtime: &Path, host_session: &str) -> io::Result<Self> {
        let started = Instant::now();
        let pid = std::process::id();
        let stat = fs::read_to_string("/proc/self/stat")?;
        let fields: Vec<_> = stat
            .rsplit_once(") ")
            .ok_or_else(|| io::Error::other("invalid process stat"))?
            .1
            .split_whitespace()
            .collect();
        let start_ticks: u64 = fields
            .get(19)
            .ok_or_else(|| io::Error::other("missing process start identity"))?
            .parse()
            .map_err(io::Error::other)?;
        let boot_id = fs::read_to_string("/proc/sys/kernel/random/boot_id")?
            .trim()
            .to_owned();
        let executable = fs::read_link("/proc/self/exe")?;
        let executable_sha256 = digest_file(Path::new("/proc/self/exe"))?;
        let mut random = [0_u8; 16];
        getrandom::fill(&mut random).map_err(io::Error::other)?;
        let instance_id = random
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        // No argv, environment, RPC, source or conversation content is read.
        let runtime_binding = path_hash(&fs::canonicalize(runtime)?);
        let cwd_binding = path_hash(&std::env::current_dir()?);
        let directory = runtime.join("observations/mcp");
        for path in [runtime.join("observations"), directory.clone()] {
            volicord_local_platform::ensure_private_directory(&path).map_err(io::Error::other)?;
        }
        if fs::read_dir(&directory)?.count() >= 1024 {
            return Err(io::Error::other(
                "lifecycle registration retention bound reached",
            ));
        }
        let path = directory.join(format!("{instance_id}.json"));
        let value = json!({"kind":"volicord_mcp_lifecycle", "schema_version":1,
            "instance_id":instance_id, "pid":pid, "boot_id":boot_id, "start_ticks":start_ticks,
            "executable_sha256":executable_sha256, "executable_path":executable,
            "runtime_binding":runtime_binding, "cwd_binding":cwd_binding,
            "host_session":host_session, "host_session_authority":"server_generated_correlation_only",
            "state":"running", "registration_duration_ns":started.elapsed().as_nanos() as u64,
            "lifetime_ns":null});
        publish(&path, &value)?;
        Ok(Self {
            path,
            value,
            started,
        })
    }
}
impl Drop for McpLifecycle {
    fn drop(&mut self) {
        self.value["state"] = json!("stopped");
        self.value["lifetime_ns"] = json!(self.started.elapsed().as_nanos() as u64);
        if publish(&self.path, &self.value).is_err() {
            eprintln!("MCP lifecycle shutdown observation unavailable");
        }
    }
}
