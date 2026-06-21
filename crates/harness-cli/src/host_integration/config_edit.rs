use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use serde_json::Value;

use super::{HostConfigError, HostConflict, HostConflictKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileSnapshot {
    Missing,
    Present { bytes: Vec<u8> },
}

pub(crate) fn read_snapshot(path: &Path) -> Result<FileSnapshot, HostConfigError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => {
            let bytes = fs::read(path).map_err(|error| {
                HostConfigError::Io(format!("failed to read {}: {error}", path.display()))
            })?;
            Ok(FileSnapshot::Present { bytes })
        }
        Ok(metadata) if metadata.file_type().is_dir() => {
            Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::UnsafeTarget,
                format!("configuration target is a directory: {}", path.display()),
            )))
        }
        Ok(_) => Err(HostConfigError::Conflict(HostConflict::new(
            HostConflictKind::UnsafeTarget,
            format!(
                "configuration target has unsupported filesystem type: {}",
                path.display()
            ),
        ))),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            Ok(FileSnapshot::Missing)
        }
        Err(error) => Err(HostConfigError::Io(format!(
            "failed to inspect {}: {error}",
            path.display()
        ))),
    }
}

pub(crate) fn read_text_snapshot(
    path: &Path,
) -> Result<(FileSnapshot, Option<String>), HostConfigError> {
    let snapshot = read_snapshot(path)?;
    let text = match &snapshot {
        FileSnapshot::Missing => None,
        FileSnapshot::Present { bytes } => {
            Some(String::from_utf8(bytes.clone()).map_err(|error| {
                HostConfigError::Malformed(format!(
                    "configuration target is not UTF-8 text: {}: {error}",
                    path.display()
                ))
            })?)
        }
    };
    Ok((snapshot, text))
}

pub(crate) fn read_json_object(
    path: &Path,
) -> Result<(FileSnapshot, serde_json::Map<String, Value>), HostConfigError> {
    let (snapshot, text) = read_text_snapshot(path)?;
    let object = match text {
        None => serde_json::Map::new(),
        Some(text) if text.trim().is_empty() => serde_json::Map::new(),
        Some(text) => {
            let value = serde_json::from_str::<Value>(&text).map_err(|error| {
                HostConfigError::Malformed(format!(
                    "failed to parse JSON configuration {}: {error}",
                    path.display()
                ))
            })?;
            value.as_object().cloned().ok_or_else(|| {
                HostConfigError::Malformed(format!(
                    "JSON configuration must be an object: {}",
                    path.display()
                ))
            })?
        }
    };
    Ok((snapshot, object))
}

pub(crate) fn write_json_object_if_fresh(
    path: &Path,
    object: &serde_json::Map<String, Value>,
    snapshot: &FileSnapshot,
) -> Result<(), HostConfigError> {
    let mut text =
        serde_json::to_string_pretty(&Value::Object(object.clone())).map_err(|error| {
            HostConfigError::Malformed(format!("failed to render JSON configuration: {error}"))
        })?;
    text.push('\n');
    write_if_fresh(path, text.as_bytes(), snapshot)
}

pub(crate) fn write_if_fresh(
    target: &Path,
    content: &[u8],
    snapshot: &FileSnapshot,
) -> Result<(), HostConfigError> {
    compare_snapshot(target, snapshot)?;
    let parent = target.parent().ok_or_else(|| {
        HostConfigError::Io(format!(
            "configuration target has no parent directory: {}",
            target.display()
        ))
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        HostConfigError::Io(format!(
            "failed to create configuration directory {}: {error}",
            parent.display()
        ))
    })?;

    let existing_permissions = match fs::symlink_metadata(target) {
        Ok(metadata) if metadata.file_type().is_file() => Some(metadata.permissions()),
        Ok(_) => {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::UnsafeTarget,
                format!(
                    "configuration target has unsupported filesystem type: {}",
                    target.display()
                ),
            )));
        }
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            None
        }
        Err(error) => {
            return Err(HostConfigError::Io(format!(
                "failed to inspect {}: {error}",
                target.display()
            )));
        }
    };

    let (temp_path, mut file) = create_temp_file_for(target)?;
    let write_result = (|| -> io::Result<()> {
        file.write_all(content)?;
        file.flush()?;
        if let Some(permissions) = existing_permissions {
            file.set_permissions(permissions)?;
        }
        file.sync_all()?;
        Ok(())
    })();
    drop(file);

    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(HostConfigError::Io(format!(
            "failed to write temporary configuration file {}: {error}",
            temp_path.display()
        )));
    }

    compare_snapshot(target, snapshot).inspect_err(|_| {
        let _ = fs::remove_file(&temp_path);
    })?;

    fs::rename(&temp_path, target).map_err(|error| {
        let _ = fs::remove_file(&temp_path);
        HostConfigError::Io(format!(
            "failed to move configuration file into place at {}: {error}",
            target.display()
        ))
    })
}

pub(crate) fn remove_file_if_fresh(
    target: &Path,
    snapshot: &FileSnapshot,
) -> Result<(), HostConfigError> {
    compare_snapshot(target, snapshot)?;
    fs::remove_file(target).map_err(|error| {
        HostConfigError::Io(format!("failed to remove {}: {error}", target.display()))
    })
}

fn compare_snapshot(target: &Path, expected: &FileSnapshot) -> Result<(), HostConfigError> {
    let current = read_snapshot(target)?;
    if &current == expected {
        Ok(())
    } else {
        Err(HostConfigError::StalePlan(format!(
            "configuration target changed since planning: {}",
            target.display()
        )))
    }
}

fn create_temp_file_for(target: &Path) -> Result<(PathBuf, fs::File), HostConfigError> {
    let parent = target.parent().ok_or_else(|| {
        HostConfigError::Io(format!(
            "configuration target has no parent directory: {}",
            target.display()
        ))
    })?;
    let file_name = target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "config".to_owned());
    for attempt in 0..1000u32 {
        let temp_path = parent.join(format!(".{file_name}.tmp-{}-{attempt}", std::process::id()));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
            Ok(file) => return Ok((temp_path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(HostConfigError::Io(format!(
                    "failed to create temporary configuration file {}: {error}",
                    temp_path.display()
                )));
            }
        }
    }
    Err(HostConfigError::Io(format!(
        "failed to allocate a temporary configuration file for {}",
        target.display()
    )))
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn stale_plan_detection_rejects_changed_file() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("host-edit-stale")?;
        let target = dir.join("config.json");
        fs::write(&target, "{}\n")?;
        let snapshot = read_snapshot(&target)?;
        fs::write(&target, "{\"changed\":true}\n")?;

        let error = write_if_fresh(&target, b"{\"next\":true}\n", &snapshot)
            .expect_err("changed file should be rejected");

        assert!(matches!(error, HostConfigError::StalePlan(_)));
        assert_eq!(fs::read_to_string(&target)?, "{\"changed\":true}\n");
        assert_no_temp_files(&dir)?;
        Ok(())
    }

    #[test]
    fn atomic_write_cleans_up_after_recheck_failure() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("host-edit-cleanup")?;
        let target = dir.join("config.json");
        let snapshot = read_snapshot(&target)?;
        fs::write(&target, "{}\n")?;

        let error = write_if_fresh(&target, b"{\"next\":true}\n", &snapshot)
            .expect_err("late-created file should be rejected");

        assert!(matches!(error, HostConfigError::StalePlan(_)));
        assert_eq!(fs::read_to_string(&target)?, "{}\n");
        assert_no_temp_files(&dir)?;
        Ok(())
    }

    fn temp_dir(prefix: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&path)?;
        Ok(path)
    }

    fn assert_no_temp_files(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            assert!(
                !name.contains(".tmp-"),
                "temporary file should have been cleaned up: {name}"
            );
        }
        Ok(())
    }
}
