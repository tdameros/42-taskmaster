/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use crate::config::ProgramConfig;

/* -------------------------------------------------------------------------- */
/*                                   Module                                   */
/* -------------------------------------------------------------------------- */
mod manager;
mod process;
mod state;

/* -------------------------------------------------------------------------- */
/*                                   Struct                                   */
/* -------------------------------------------------------------------------- */
/// represent a process ;managed by taskmaster
#[derive(Debug, Default)]
pub(super) struct Process {
    /// the handle to the process
    child: Option<std::process::Child>,

    /// the time when the process was launched, used to determine the
    /// transition from starting to running
    started_since: Option<std::time::SystemTime>,

    /// use to determine when to abort the child
    time_since_shutdown: Option<std::time::SystemTime>,

    /// store the state of a given process
    state: ProcessState,

    /// the config that the process is based on
    config: ProgramConfig,

    /// current number of restart, it increment only when the process was
    /// restarted when it was consider to be in a starting state
    number_of_restart: u32,
}

/// Represent the state of a given process
#[derive(Debug, Default)]
enum ProcessState {
    /// the default state, has never been started.
    #[default]
    NeverStartedYet,

    /// The process has been stopped due to a stop request
    Stopped,

    /// The process is starting due to a start request.
    Starting,

    /// The process is running.
    Running,

    /// The process entered the Starting state but subsequently exited too quickly
    /// (before the time defined in time_to_start) to move to the Running state.
    Backoff,

    /// The process is stopping due to a stop request.
    Stopping,

    /// The process exited from the RUNNING state expectedly.
    ExitedExpectedly,

    /// The process exited from the RUNNING state unexpectedly.
    ExitedUnExpectedly,

    /// The process could not be started successfully.
    Fatal,

    /// The process is in an unknown state (error while getting the exit status).
    Unknown,
}

/// represent the error that can occur while performing action on the process class
#[derive(Debug)]
enum ProcessError {
    /// an operation was perform on a child but no child were found (aka stopped or not launch yet)
    NoChild,
    ExitStatusNotFound(std::io::Error),
    CantKillProcess(std::io::Error),
    /// ??
    Signal(std::io::Error),
    /// if no command was found to start the child
    NoCommand,
    CouldNotSpawnChild(std::io::Error),
    FailedToCreateRedirection(std::io::Error),
}

/// this represent the running process
#[derive(Debug)]
pub(super) struct ProcessManager {
    /// represent the currently monitored programs
    programs: std::collections::HashMap<String, Vec<Process>>,

    /// the place were programs go we they are no longer part of the config
    /// and we nee to wait for them to shutdown
    purgatory: std::collections::HashMap<String, Vec<Process>>,
}

/// a sharable version of a process manager, it can be passe through thread safely + use in a concurrent environment without fear thank Rust !
pub(super) type SharedProcessManager = std::sync::Arc<std::sync::RwLock<ProcessManager>>;

/// exist simply for an ease of implementation
#[derive(Debug, Default)]
struct ProgramToRestart(pub std::collections::HashMap<String, (i64, crate::config::ProgramConfig)>);
