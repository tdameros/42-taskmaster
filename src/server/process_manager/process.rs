/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::{fmt::Display, process::ExitStatus, time::SystemTime};
use crate::config::{ProgramConfig, Signal};
use super::{Process, ProcessError, ProcessState};

/* -------------------------------------------------------------------------- */
/*                                   Struct                                   */
/* -------------------------------------------------------------------------- */
/* -------------------------------------------------------------------------- */
/*                            Struct Implementation                           */
/* -------------------------------------------------------------------------- */
impl Process {
    /// create a new RunningProcess struct based on the given child
    pub(super) fn new() -> Self {
        Self {
            started_since: SystemTime::now(),
            child: Default::default(),
            time_since_shutdown: Default::default(),
            state: Default::default(),
        }
    }

    /// Attempts to retrieve the child process's exit code.
    ///
    /// # Returns
    /// - `Ok(Some(i32))` if the child has exited and an exit code is available.
    /// - `Ok(None)` if the child is still running.
    /// - `Err(ProcessError::NoChild)` if the child process was not launched.
    /// - `Err(ProcessError::ExitStatusNotFound)` if the exit status could not be read.
    ///
    /// # Note
    /// On Unix systems, if the process was terminated by a signal, the signal number is returned as the exit code.
    pub(super) fn get_exit_code(&mut self) -> Result<Option<i32>, ProcessError> {
        let child = self.child.as_mut().ok_or(ProcessError::NoChild)?;

        match child.try_wait() {
            Ok(Some(status)) => Ok(Some(Self::extract_exit_code(status))),
            Ok(None) => Ok(None),
            Err(e) => Err(ProcessError::ExitStatusNotFound(e)),
        }
    }

    #[cfg(unix)]
    fn extract_exit_code(status: ExitStatus) -> i32 {
        status.code().unwrap_or_else(|| {
            status
                .signal()
                .expect("Process terminated by signal, but no signal number found")
        })
    }

    #[cfg(not(unix))]
    fn extract_exit_code(status: ExitStatus) -> i32 {
        status
            .code()
            .expect("Exit code should always be available on non-unix systems")
    }

    /// return the child process_id if the child is running
    pub(super) fn get_child_id(&self) -> Option<u32> {
        self.child.as_ref().and_then(|child| Some(child.id()))
    }

    /// Attempts to send a SIGKILL to the child process.
    ///
    /// # Errors
    ///
    /// Returns a `ProcessError` if:
    /// - There is no child process (`ProcessError::NoChild`)
    /// - The kill operation fails, possibly due to lack of privileges (`ProcessError::CantKillProcess`)
    pub(super) fn kill(&mut self) -> Result<(), ProcessError> {
        self.child
            .as_mut()
            .ok_or(ProcessError::NoChild)
            .and_then(|child| {
                child
                    .kill()
                    .map_err(|error| ProcessError::CantKillProcess(error))
            })
    }

    /// Determines if it's time to forcefully terminate the child process.
    ///
    /// Returns true if and only if:
    /// 1. A graceful shutdown was requested (time_since_shutdown is Some), AND
    /// 2. The time elapsed since the shutdown request exceeds the configured grace period
    ///
    /// # Arguments
    ///
    /// * `program_config` - The configuration for the program, containing the grace period
    pub(super) fn its_time_to_kill_the_child(&self, program_config: &ProgramConfig) -> bool {
        self.time_since_shutdown
            .map(|shutdown_time| {
                SystemTime::now()
                    .duration_since(shutdown_time)
                    .map(|elapsed| elapsed.as_secs() > program_config.time_to_stop_gracefully)
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    /// Determines if the program has completed its starting phase.
    ///
    /// Returns true if:
    /// The time elapsed since the process started exceeds the configured start-up time.
    ///
    /// # Arguments
    ///
    /// * `program_config` - The configuration for the program, containing the start-up time
    pub(super) fn is_no_longer_starting(&self, program_config: &ProgramConfig) -> bool {
        SystemTime::now()
            .duration_since(self.started_since)
            .map(|elapsed| elapsed.as_secs() > program_config.time_to_start)
            .unwrap_or(false)
    }

    /// Send the given signal to the child, starting the graceful shutdown timer.
    ///
    /// # Errors
    /// 
    /// Returns a `ProcessError` if:
    /// - There is no child process (`ProcessError::NoChild`)
    /// - The signal sending operation fails (`ProcessError::SignalError`)
    pub(super) fn send_signal(&mut self, signal: &Signal) -> Result<(), ProcessError> {
        let signal_number = Self::signal_to_libc(signal);
        
        let child = self.child.as_ref().ok_or(ProcessError::NoChild)?;
        
        let result = unsafe {
            libc::kill(child.id() as libc::pid_t, signal_number as libc::c_int)
        };

        if result == -1 {
            return Err(ProcessError::Signal(std::io::Error::last_os_error()));
        }

        self.time_since_shutdown = Some(SystemTime::now());
        Ok(())
    }

    /// Convert our Signal enum to libc signal constants
    fn signal_to_libc(signal: &Signal) -> libc::c_int {
        match signal {
            Signal::SIGABRT => libc::SIGABRT,
            Signal::SIGALRM => libc::SIGALRM,
            Signal::SIGBUS => libc::SIGBUS,
            Signal::SIGCHLD => libc::SIGCHLD,
            Signal::SIGCONT => libc::SIGCONT,
            Signal::SIGFPE => libc::SIGFPE,
            Signal::SIGHUP => libc::SIGHUP,
            Signal::SIGILL => libc::SIGILL,
            Signal::SIGINT => libc::SIGINT,
            Signal::SIGKILL => libc::SIGKILL,
            Signal::SIGPIPE => libc::SIGPIPE,
            Signal::SIGPOLL => libc::SIGPOLL,
            Signal::SIGPROF => libc::SIGPROF,
            Signal::SIGQUIT => libc::SIGQUIT,
            Signal::SIGSEGV => libc::SIGSEGV,
            Signal::SIGSTOP => libc::SIGSTOP,
            Signal::SIGSYS => libc::SIGSYS,
            Signal::SIGTERM => libc::SIGTERM,
            Signal::SIGTRAP => libc::SIGTRAP,
            Signal::SIGTSTP => libc::SIGTSTP,
            Signal::SIGTTIN => libc::SIGTTIN,
            Signal::SIGTTOU => libc::SIGTTOU,
            Signal::SIGUSR1 => libc::SIGUSR1,
            Signal::SIGUSR2 => libc::SIGUSR2,
            Signal::SIGURG => libc::SIGURG,
            Signal::SIGVTALRM => libc::SIGVTALRM,
            Signal::SIGXCPU => libc::SIGXCPU,
            Signal::SIGXFSZ => libc::SIGXFSZ,
            Signal::SIGWINCH => libc::SIGWINCH,
        }
    }

    pub(super) fn update_status(&mut self) {
        let result = self.get_exit_code();
        match self.state {
            ProcessState::Stopped => self.process_stopped(result),
            ProcessState::Starting => todo!(),
            ProcessState::Running => todo!(),
            ProcessState::Backoff => todo!(),
            ProcessState::Stopping => todo!(),
            ProcessState::Exited => todo!(),
            ProcessState::Fatal => todo!(),
            ProcessState::Unknown => todo!(),
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                            Error Implementation                            */
/* -------------------------------------------------------------------------- */
impl std::error::Error for ProcessError {}

impl Display for ProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
