/* -------------------------------------------------------------------------- */
/*                                   Module                                   */
/* -------------------------------------------------------------------------- */
pub(super) mod manager;
pub(super) mod state;
pub(super) mod process;

/* -------------------------------------------------------------------------- */
/*                                   Struct                                   */
/* -------------------------------------------------------------------------- */
#[derive(Debug)]
pub(super) struct Process {
    /// the handle to the process
    child: Option<std::process::Child>,

    /// the time when the process was launched, used to determine the
    /// transition from starting to running
    started_since: std::time::SystemTime, // to clarify

    /// use to determine when to abort the child
    time_since_shutdown: Option<std::time::SystemTime>,

    /// store the state of a given process
    state: ProcessState,
}

/// Represent the state of a given process
#[derive(Debug, Default)]
enum ProcessState {
    /// the default state, The process has been stopped due to a stop request or has never been started.
    #[default]
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

    /// The process exited from the RUNNING state (expectedly or unexpectedly).
    Exited,

    /// The process could not be started successfully.
    Fatal,

    /// The process is in an unknown state (error while getting the exit status).
    Unknown,
}

/// represent the error that can occur while performing action on the process class
#[derive(Debug)]
enum ProcessError {
    /// an operation was perform on a child but no child were found (aka not launched yet)
    NoChild,
    ExitStatusNotFound(std::io::Error),
    CantKillProcess(std::io::Error),
    Signal(std::io::Error),
}

/// this represent the running process
#[derive(Debug)]
pub(super) struct ProcessManager(std::collections::HashMap<String, Vec<Process>>);

/// a sharable version of a process manager, it can be passe through thread safely + use in a concurrent environment without fear thank Rust !
pub(super) type SharedProcessManager = std::sync::Arc<std::sync::RwLock<ProcessManager>>;

/// exist simply for an ease of implementation
#[derive(Debug, Default)]
struct ProgramToRestart(pub std::collections::HashMap<String, (i64, crate::config::ProgramConfig)>);
