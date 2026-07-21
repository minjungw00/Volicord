use std::process::{Child, Command};

use crate::{PipeRead, PlatformProcessError, PlatformProcessOperation};

pub(crate) struct ProcessContainment;

impl ProcessContainment {
    pub(crate) fn new() -> Result<Self, PlatformProcessError> {
        Err(PlatformProcessError::unsupported(
            PlatformProcessOperation::CreateContainment,
            "bounded child-process containment is unavailable on this platform",
        ))
    }

    pub(crate) fn configure_command(&self, _command: &mut Command) {}

    pub(crate) fn attach(&mut self, _child: &Child) -> Result<(), PlatformProcessError> {
        Err(PlatformProcessError::unsupported(
            PlatformProcessOperation::AttachChild,
            "bounded child-process containment is unavailable on this platform",
        ))
    }

    pub(crate) fn terminate_tree(&self) -> Result<(), PlatformProcessError> {
        Ok(())
    }
}

pub(crate) fn configure_pipe<T>(_pipe: &T) -> Result<(), PlatformProcessError> {
    Err(PlatformProcessError::unsupported(
        PlatformProcessOperation::ConfigurePipe,
        "bounded child-pipe polling is unavailable on this platform",
    ))
}

pub(crate) fn read_pipe_available<R>(
    _reader: &mut R,
    _buffer: &mut [u8],
) -> Result<PipeRead, PlatformProcessError> {
    Err(PlatformProcessError::unsupported(
        PlatformProcessOperation::ReadPipe,
        "bounded child-pipe polling is unavailable on this platform",
    ))
}
