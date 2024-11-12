/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use super::{Process, ProcessError, ProcessState};
use crate::config::{ProgramConfig, Signal};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::{
    fmt::Display,
    fs,
    process::{Command, ExitStatus, Stdio},
    time::SystemTime,
};

/* -------------------------------------------------------------------------- */
/*                            Struct Implementation                           */
/* -------------------------------------------------------------------------- */
impl Process {
    pub(super) fn new(config: ProgramConfig) -> Self {
        Self {
            config,
            ..Default::default()
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

    /// Returns the child process ID if the process is active.
    ///
    /// # Returns
    /// - `Some(u32)`: The process ID if the child is running, starting, or stopping.
    /// - `None`: If the child process is inactive or if there was an error updating the state.
    pub(super) fn get_child_id(&mut self) -> Option<u32> {
        if self.update_state().is_err() {
            return None;
        }
        use ProcessState as PS;
        match self.state {
            PS::Starting | PS::Running | PS::Stopping => {
                Some(self.child.as_ref().expect("shouldn't not happened").id())
            }
            PS::NeverStartedYet
            | PS::Stopped
            | PS::Backoff
            | PS::ExitedExpectedly
            | PS::ExitedUnExpectedly
            | PS::Fatal
            | PS::Unknown => None,
        }
    }

    /// Attempts to send a SIGKILL to the child process.
    ///
    /// # Errors
    ///
    /// - `ProcessError::NoChild` if there were no child process
    /// - `ProcessError::CantKillProcess` if we couldn't kill the process
    pub(super) fn kill(&mut self) -> Result<(), ProcessError> {
        self.child
            .as_mut()
            .ok_or(ProcessError::NoChild)
            .and_then(|child| {
                child
                    .kill()
                    .map_err(|error| {
                        self.state = ProcessState::Stopping;
                        ProcessError::CantKillProcess(error)
                    })
                    .map(|_| self.state = ProcessState::Stopped)
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
    pub(super) fn its_time_to_kill_the_child(&self) -> bool {
        self.time_since_shutdown
            .map(|shutdown_time| {
                SystemTime::now()
                    .duration_since(shutdown_time)
                    .map(|elapsed| elapsed.as_secs() > self.config.time_to_stop_gracefully)
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    /// Determines if the program has completed its starting phase.
    ///
    /// Returns:
    /// - `Some(true)` if the process has started and the time elapsed since it started exceeds the configured start-up time.
    /// - `Some(false)` if the process has started but hasn't exceeded the start-up time yet.
    /// - `None` if no starting time were found (AKA the program is not running).
    pub(super) fn is_no_longer_starting(&self) -> Option<bool> {
        self.started_since.map(|start_time| {
            SystemTime::now()
                .duration_since(start_time)
                .map(|elapsed| elapsed.as_secs() > self.config.time_to_start)
                .unwrap_or(false)
        })
    }

    /// Send the given signal to the child, starting the graceful shutdown timer.
    ///
    /// # Errors
    ///
    /// Returns a `ProcessError` if:
    /// - There is no child process (`ProcessError::NoChild`)
    /// - The signal sending operation fails (`ProcessError::SignalError`)
    pub(super) fn send_signal(&mut self, signal: &Signal) -> Result<(), ProcessError> {
        let child = self.child.as_ref().ok_or(ProcessError::NoChild)?;
        let signal_number = Self::signal_to_libc(signal);
        tcl::mylibc::kill(
            child.id() as tcl::mylibc::pid_t,
            signal_number as std::ffi::c_int,
        )
        .map_err(ProcessError::Signal)?;

        self.time_since_shutdown = Some(SystemTime::now());
        self.started_since = None;
        self.state = ProcessState::Stopping;
        Ok(())
    }

    /// Convert our Signal enum to libc signal constants
    fn signal_to_libc(signal: &Signal) -> std::ffi::c_int {
        match signal {
            Signal::SIGABRT => tcl::mylibc::SIGABRT,
            Signal::SIGALRM => tcl::mylibc::SIGALRM,
            Signal::SIGBUS => tcl::mylibc::SIGBUS,
            Signal::SIGCHLD => tcl::mylibc::SIGCHLD,
            Signal::SIGCONT => tcl::mylibc::SIGCONT,
            Signal::SIGFPE => tcl::mylibc::SIGFPE,
            Signal::SIGHUP => tcl::mylibc::SIGHUP,
            Signal::SIGILL => tcl::mylibc::SIGILL,
            Signal::SIGINT => tcl::mylibc::SIGINT,
            Signal::SIGKILL => tcl::mylibc::SIGKILL,
            Signal::SIGPIPE => tcl::mylibc::SIGPIPE,
            #[cfg(target_os = "linux")]
            Signal::SIGPOLL => tcl::mylibc::SIGPOLL,
            Signal::SIGPROF => tcl::mylibc::SIGPROF,
            Signal::SIGQUIT => tcl::mylibc::SIGQUIT,
            Signal::SIGSEGV => tcl::mylibc::SIGSEGV,
            Signal::SIGSTOP => tcl::mylibc::SIGSTOP,
            Signal::SIGSYS => tcl::mylibc::SIGSYS,
            Signal::SIGTERM => tcl::mylibc::SIGTERM,
            Signal::SIGTRAP => tcl::mylibc::SIGTRAP,
            Signal::SIGTSTP => tcl::mylibc::SIGTSTP,
            Signal::SIGTTIN => tcl::mylibc::SIGTTIN,
            Signal::SIGTTOU => tcl::mylibc::SIGTTOU,
            Signal::SIGUSR1 => tcl::mylibc::SIGUSR1,
            Signal::SIGUSR2 => tcl::mylibc::SIGUSR2,
            Signal::SIGURG => tcl::mylibc::SIGURG,
            Signal::SIGVTALRM => tcl::mylibc::SIGVTALRM,
            Signal::SIGXCPU => tcl::mylibc::SIGXCPU,
            Signal::SIGXFSZ => tcl::mylibc::SIGXFSZ,
            Signal::SIGWINCH => tcl::mylibc::SIGWINCH,
        }
    }

    /// check the child state and change it's status if needed
    ///
    /// Returns:
    /// - `Ok(())` if the exit_status could be acquire without issue.
    /// - `Err(ProcessError::ExitStatusNotFound)` if the exit status could not be read.
    pub(super) fn update_state(&mut self) -> Result<(), ProcessError> {
        use ProcessError as PE;
        use ProcessState as PS;
        match self.get_exit_code() {
            Ok(result) => {
                match self.state {
                    PS::Starting => self.update_starting(result),
                    PS::Running => self.update_running(result),
                    PS::Stopping => self.update_stopping(result),
                    PS::Unknown => self.update_unknown(result),
                    PS::Backoff
                    | PS::Stopped
                    | PS::Fatal
                    | PS::NeverStartedYet
                    | PS::ExitedExpectedly
                    | PS::ExitedUnExpectedly => unreachable!(),
                };

                Ok(())
            }
            Err(e) => match e {
                PE::NoChild => Ok(()),
                PE::ExitStatusNotFound(ref _e) => {
                    self.state = PS::Unknown;
                    Err(e)
                }
                PE::NoCommand
                | PE::CantKillProcess(_)
                | PE::Signal(_)
                | PE::CouldNotSpawnChild(_)
                | PE::FailedToCreateRedirection(_) => unreachable!(),
            },
        }
    }

    /// this function use the config to see if some cleaning or restarting need to happened
    /// it also call the update_state function before it so that the state we are working
    /// with are the more accurate possible
    ///
    /// Returns:
    /// - `Ok(())` if the exit_status could be acquire without issue and the state
    ///     and change that need to be done were done.
    /// - `Err(ProcessError::ExitStatusNotFound)` if the exit status could not be read.
    /// - `Err(ProcessError::NoCommand)` if the command argument is empty.
    /// - `Err(ProcessError::FailedToCreateRedirection)` if the redirection argument couldn't be accessed found or create.
    /// - `Err(ProcessError::CouldNotSpawnChild)` if the child was not able to be spawned
    /// - `Err(ProcessError::NoChild)` if there were no child process
    /// - `Err(ProcessError::CantKillProcess)` if we couldn't kill the process
    pub(super) fn react_to_program_state(
        &mut self,
        program_name: &str,
    ) -> Result<(), ProcessError> {
        self.update_state()?;
        use ProcessState as PS;
        match self.state {
            PS::NeverStartedYet => self.react_never_started_yet(),
            PS::Backoff => self.react_backoff(program_name),
            PS::Stopping => self.react_stopping(),
            PS::ExitedExpectedly => self.react_expected_exit(),
            PS::ExitedUnExpectedly => self.react_unexpected_exit(),
            PS::Fatal | PS::Starting | PS::Running | PS::Stopped => Ok(()),
            PS::Unknown => unreachable!(
                "as long as we return the error of update_state call before this match block"
            ),
        }
    }

    /// this function attempt to spawn a child if successful it will set the appropriate state
    /// # Returns
    /// - `Ok(())` if the child was spawn successfully
    /// - `Err(ProcessError::NoCommand)` if the command argument is empty.
    /// - `Err(ProcessError::FailedToCreateRedirection)` if the redirection argument couldn't be accessed found or create.
    /// - `Err(ProcessError::CouldNotSpawnChild)` if the child was not able to be spawned
    pub(super) fn start(&mut self) -> Result<(), ProcessError> {
        let mut split_command = self.config.command.split_whitespace();
        let program = split_command.next().ok_or(ProcessError::NoCommand)?;
        let original_umask: Option<tcl::mylibc::mode_t> =
            self.config.umask.map(tcl::mylibc::set_umask);
        let mut command = Command::new(program);

        command.envs(&self.config.environmental_variable_to_set);
        command.args(split_command);
        if let Some(dir) = &self.config.working_directory {
            command.current_dir(dir);
        }
        // privilege de-escalation
        if let Some(user) = &self.config.de_escalation_user {
            command.uid(user.uid);
            command.gid(user.gid);
        }
        self.set_command_redirection(&mut command)
            .map_err(ProcessError::FailedToCreateRedirection)?;

        let child = command.spawn().map_err(ProcessError::CouldNotSpawnChild)?;

        if let Some(umask) = original_umask {
            tcl::mylibc::set_umask(umask);
        }

        self.child = Some(child);
        self.state = ProcessState::Starting;
        self.started_since = Some(SystemTime::now());
        self.time_since_shutdown = None;

        Ok(())
    }

    fn set_command_redirection(&self, command: &mut Command) -> Result<(), std::io::Error> {
        match self.config.stdout_redirection.as_ref() {
            Some(stdout) => {
                let file = fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(stdout)?;
                command.stdout(file);
            }
            None => {
                command.stdout(Stdio::null());
            }
        }
        match self.config.stderr_redirection.as_ref() {
            Some(stderr) => {
                let file = fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(stderr)?;
                command.stderr(file);
            }
            None => {
                command.stderr(Stdio::null());
            }
        }
        Ok(())
    }

    /// this function simply set the child to None
    /// not if this is use while the child is alive it will create a zombie process
    pub(super) fn clean_child(&mut self) {
        self.child = None;
    }

    /// return true if the process still have an active child that mean if his state is either:
    /// - `Starting`
    /// - `Running`
    /// - `Stopping`
    /// - `Unknown`
    ///
    /// return false otherwise
    pub(super) fn is_active(&self) -> bool {
        self.child.is_some()
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

/* -------------------------------------------------------------------------- */
/*                             From Implementation                            */
/* -------------------------------------------------------------------------- */
impl From<&ProcessState> for tcl::message::ProcessState {
    fn from(val: &ProcessState) -> Self {
        use tcl::message::ProcessState as OPS;
        use ProcessState as PS;
        match val {
            PS::NeverStartedYet => OPS::NeverStartedYet,
            PS::Stopped => OPS::Stopped,
            PS::Starting => OPS::Starting,
            PS::Running => OPS::Running,
            PS::Backoff => OPS::Backoff,
            PS::Stopping => OPS::Stopping,
            PS::ExitedExpectedly => OPS::ExitedExpectedly,
            PS::ExitedUnExpectedly => OPS::ExitedUnExpectedly,
            PS::Fatal => OPS::Fatal,
            PS::Unknown => OPS::Unknown,
        }
    }
}

impl From<&mut Process> for tcl::message::ProcessStatus {
    fn from(val: &mut Process) -> Self {
        tcl::message::ProcessStatus {
            pid: val.get_child_id(),
            status: (&val.state).into(),
            start_time: val.started_since,
            shutdown_time: val.time_since_shutdown,
            number_of_restart: val.number_of_restart,
        }
    }
}
