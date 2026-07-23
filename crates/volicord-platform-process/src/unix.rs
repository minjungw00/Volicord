use std::{
    io::{self, Read},
    os::{fd::AsFd, unix::process::CommandExt},
    process::{Child, Command},
};

use rustix::{
    fs::{fcntl_getfl, fcntl_setfl, OFlags},
    io::Errno,
    process::{kill_process_group, Pid, Signal},
};

use crate::{PipeRead, PlatformProcessError, PlatformProcessOperation};

pub(crate) struct ProcessContainment {
    child_pid: Option<Pid>,
}

impl ProcessContainment {
    pub(crate) fn new() -> Result<Self, PlatformProcessError> {
        Ok(Self { child_pid: None })
    }

    pub(crate) fn configure_command(&self, command: &mut Command) {
        command.process_group(0);
    }

    pub(crate) fn attach(&mut self, child: &Child) -> Result<(), PlatformProcessError> {
        let raw_pid = i32::try_from(child.id()).map_err(|_| {
            PlatformProcessError::invalid_child(
                "child_process_id_out_of_range",
                "child process ID did not fit the platform PID range",
            )
        })?;
        self.child_pid = Pid::from_raw(raw_pid);
        self.child_pid.is_some().then_some(()).ok_or_else(|| {
            PlatformProcessError::invalid_child(
                "child_process_id_unavailable",
                "child process ID was unavailable",
            )
        })
    }

    pub(crate) fn terminate_tree(&self) -> Result<(), PlatformProcessError> {
        let Some(pid) = self.child_pid else {
            return Ok(());
        };
        match kill_process_group(pid, Signal::KILL) {
            Ok(()) | Err(Errno::SRCH) => Ok(()),
            Err(error) => Err(PlatformProcessError::operating_system(
                PlatformProcessOperation::TerminateProcessTree,
                format!("failed to terminate the contained process group: {error}"),
            )),
        }
    }
}

pub(crate) fn configure_read_pipe(pipe: impl AsFd) -> Result<(), PlatformProcessError> {
    configure_pipe(pipe)
}

pub(crate) fn configure_write_pipe(pipe: impl AsFd) -> Result<(), PlatformProcessError> {
    configure_pipe(pipe)
}

fn configure_pipe(pipe: impl AsFd) -> Result<(), PlatformProcessError> {
    let flags = fcntl_getfl(&pipe).map_err(configure_pipe_error)?;
    fcntl_setfl(pipe, flags | OFlags::NONBLOCK).map_err(configure_pipe_error)?;
    Ok(())
}

pub(crate) fn read_pipe_available<R: Read + AsFd>(
    reader: &mut R,
    buffer: &mut [u8],
) -> Result<PipeRead, PlatformProcessError> {
    match reader.read(buffer) {
        Ok(0) => Ok(PipeRead::Eof),
        Ok(count) => Ok(PipeRead::Data(count)),
        Err(error) if error.kind() == io::ErrorKind::Interrupted => Ok(PipeRead::NoData),
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(PipeRead::NoData),
        Err(error) => Err(PlatformProcessError::operating_system(
            PlatformProcessOperation::ReadPipe,
            format!("failed to read the child pipe: {error}"),
        )),
    }
}

fn configure_pipe_error(error: Errno) -> PlatformProcessError {
    PlatformProcessError::operating_system(
        PlatformProcessOperation::ConfigurePipe,
        format!("failed to configure the child pipe for bounded polling: {error}"),
    )
}
