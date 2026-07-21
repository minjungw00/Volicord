use std::{
    io::{self, Read},
    os::windows::io::AsRawHandle,
    process::{Child, Command},
    ptr,
};

use windows_sys::Win32::{
    Foundation::{CloseHandle, ERROR_BROKEN_PIPE, ERROR_PIPE_NOT_CONNECTED, HANDLE},
    System::{
        JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        },
        Pipes::PeekNamedPipe,
    },
};

use crate::{PipeRead, PlatformProcessError, PlatformProcessOperation};

struct OwnedJobHandle {
    handle: HANDLE,
}

impl OwnedJobHandle {
    fn from_created(handle: HANDLE) -> Self {
        Self { handle }
    }
}

#[allow(unsafe_code)]
impl Drop for OwnedJobHandle {
    fn drop(&mut self) {
        if self.handle.is_null() {
            return;
        }
        // SAFETY: `handle` is the non-null Job Object handle returned by
        // `CreateJobObjectW` and owned exclusively by this non-Clone RAII value.
        // No API used here transfers ownership, and this drop runs once, so the
        // handle is live, unaliased for closure, and cannot be used after close.
        unsafe {
            CloseHandle(self.handle);
        }
        self.handle = ptr::null_mut();
    }
}

pub(crate) struct ProcessContainment {
    job: OwnedJobHandle,
    attached: bool,
}

impl ProcessContainment {
    #[allow(unsafe_code)]
    pub(crate) fn new() -> Result<Self, PlatformProcessError> {
        // SAFETY: both optional pointers are null, so Windows creates an
        // unnamed Job Object with default security. The returned handle is
        // checked for null and immediately transferred into one RAII owner.
        let handle = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
        if handle.is_null() {
            return Err(PlatformProcessError::operating_system(
                PlatformProcessOperation::CreateContainment,
                format!(
                    "failed to create process containment: {}",
                    io::Error::last_os_error()
                ),
            ));
        }
        let job = OwnedJobHandle::from_created(handle);

        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: `job.handle` is a live owned Job Object handle. `limits` is
        // the exact `repr(C)` structure required by this information class;
        // its pointer is valid and aligned for the declared structure size for
        // the whole call. Windows only reads it and takes no ownership, so the
        // stack value cannot be aliased mutably or used after its lifetime.
        let configured = unsafe {
            SetInformationJobObject(
                job.handle,
                JobObjectExtendedLimitInformation,
                ptr::from_ref(&limits).cast(),
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if configured == 0 {
            return Err(PlatformProcessError::operating_system(
                PlatformProcessOperation::ConfigureContainment,
                format!(
                    "failed to configure process containment: {}",
                    io::Error::last_os_error()
                ),
            ));
        }
        Ok(Self {
            job,
            attached: false,
        })
    }

    pub(crate) fn configure_command(&self, _command: &mut Command) {}

    #[allow(unsafe_code)]
    pub(crate) fn attach(&mut self, child: &Child) -> Result<(), PlatformProcessError> {
        let child_handle = child.as_raw_handle() as HANDLE;
        // SAFETY: the Job Object handle is live and exclusively owned by
        // `self`. `child_handle` is borrowed from a live `Child` for this call.
        // `AssignProcessToJobObject` takes ownership of neither handle, closes
        // neither handle, and stores no Rust pointer, so there is no ownership
        // transfer, mutable alias, use-after-close, or lifetime escape.
        let assigned = unsafe { AssignProcessToJobObject(self.job.handle, child_handle) };
        if assigned == 0 {
            return Err(PlatformProcessError::operating_system(
                PlatformProcessOperation::AttachChild,
                format!(
                    "failed to attach the child to process containment: {}",
                    io::Error::last_os_error()
                ),
            ));
        }
        self.attached = true;
        Ok(())
    }

    #[allow(unsafe_code)]
    pub(crate) fn terminate_tree(&self) -> Result<(), PlatformProcessError> {
        if !self.attached {
            return Ok(());
        }
        // SAFETY: `self.job.handle` remains live for the shared borrow and is
        // owned by the RAII field. Termination neither transfers nor closes the
        // handle, and Drop cannot run concurrently with this borrow, preventing
        // aliasing with closure or use after close.
        let terminated = unsafe { TerminateJobObject(self.job.handle, 1) };
        if terminated == 0 {
            Err(PlatformProcessError::operating_system(
                PlatformProcessOperation::TerminateProcessTree,
                format!(
                    "failed to terminate the contained process tree: {}",
                    io::Error::last_os_error()
                ),
            ))
        } else {
            Ok(())
        }
    }
}

pub(crate) fn configure_pipe<T>(_pipe: &T) -> Result<(), PlatformProcessError> {
    Ok(())
}

#[allow(unsafe_code)]
pub(crate) fn read_pipe_available<R: Read + AsRawHandle>(
    reader: &mut R,
    buffer: &mut [u8],
) -> Result<PipeRead, PlatformProcessError> {
    let handle = reader.as_raw_handle() as HANDLE;
    let mut available = 0_u32;
    // SAFETY: `handle` is borrowed from a live child-pipe reader for the whole
    // call and is not transferred or closed. The optional inspection buffers
    // are null with zero size; `available` is a valid aligned output pointer.
    // The caller's byte buffer is not passed to FFI and stays exclusively
    // borrowed until the later safe `Read`, so no buffer alias or use-after-
    // close can occur.
    let peeked = unsafe {
        PeekNamedPipe(
            handle,
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            &mut available,
            ptr::null_mut(),
        )
    };
    if peeked == 0 {
        let error = io::Error::last_os_error();
        return match error.raw_os_error().map(|code| code as u32) {
            Some(code) if code == ERROR_BROKEN_PIPE || code == ERROR_PIPE_NOT_CONNECTED => {
                Ok(PipeRead::Eof)
            }
            _ => Err(PlatformProcessError::operating_system(
                PlatformProcessOperation::ReadPipe,
                format!("failed to inspect the child pipe: {error}"),
            )),
        };
    }
    if available == 0 {
        return Ok(PipeRead::NoData);
    }

    let requested = buffer.len().min(available as usize);
    match reader.read(&mut buffer[..requested]) {
        Ok(0) => Ok(PipeRead::Eof),
        Ok(count) => Ok(PipeRead::Data(count)),
        Err(error) if error.kind() == io::ErrorKind::Interrupted => Ok(PipeRead::NoData),
        Err(error) => Err(PlatformProcessError::operating_system(
            PlatformProcessOperation::ReadPipe,
            format!("failed to read the child pipe: {error}"),
        )),
    }
}
