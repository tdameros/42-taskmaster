/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::{fs, path::Path};
use tcl::error::TaskmasterError;

/* -------------------------------------------------------------------------- */
/*                                  Constants                                 */
/* -------------------------------------------------------------------------- */
const CONFIG_FILE_PATH: &str = "./config.yaml";

/* -------------------------------------------------------------------------- */
/*                                   Struct                                   */
/* -------------------------------------------------------------------------- */
pub(super) type SharedConfig = Arc<RwLock<Config>>;

/// struct representing the process the server should monitor
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub programs: HashMap<String, ProgramConfig>,
}

/// represent all configuration of a monitored program
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ProgramConfig {
    /// The command to use to launch the program
    #[serde(rename = "cmd", default)]
    command: String,

    /// The number of processes to start and keep running
    #[serde(rename = "numprocs", default)]
    pub(super) number_of_process: u32,

    /// Whether to start this program at launch or not
    #[serde(rename = "autostart", default)]
    pub(super) start_at_launch: bool,

    /// Whether the program should be restarted always, never, or on unexpected exits only
    #[serde(rename = "autorestart", default)]
    auto_restart: AutoRestart,

    /// Which return codes represent an "expected" exit status
    #[serde(rename = "exitcodes", default)]
    expected_exit_code: Vec<u32>,

    /// How long the program should be running after it’s started for it to be considered "successfully started"
    #[serde(rename = "starttime", default)]
    time_to_start: u32,

    /// How many times a restart should be attempted before aborting
    #[serde(rename = "startretries", default)]
    max_number_of_restart: u32,

    /// Which signal should be used to stop (i.e. exit gracefully) the program
    #[serde(rename = "stopsignal", default)]
    stop_signal: Signal,

    /// How long to wait after a graceful stop before killing the program
    #[serde(rename = "stoptime", default)]
    time_to_stop_gracefully: u32,

    /// Optional stdout redirection
    #[serde(rename = "stdout", default)]
    stdout_redirection: String,

    /// Optional stderr redirection
    #[serde(rename = "stderr", default)]
    stderr_redirection: String,

    /// Environment variables to set before launching the program
    #[serde(rename = "env", default)]
    environmental_variable_to_set: HashMap<String, String>,
    // environmental_variable_to_set: Vec<(String, String)>,
    /// A working directory to set before launching the program
    #[serde(rename = "workingdir", default)]
    working_directory: String,

    /// An umask to set before launching the program
    #[serde(rename = "umask", default)]
    umask: u32,
}

/// this enum represent whenever a program should be auto restart if it's termination
/// has been detected
#[derive(Debug, Serialize, Deserialize, Default)]
pub enum AutoRestart {
    #[serde(rename = "always")]
    Always,

    /// if the exit code is not part of the expected exit code list
    #[serde(rename = "unexpected")]
    Unexpected,

    #[default] // use the field below as default (needed for the default trait)
    #[serde(rename = "never")]
    Never,
}

/// represent all the signal
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Serialize, Deserialize, Default)]
pub enum Signal {
    SIGABRT,
    SIGALRM,
    SIGBUS,
    SIGCHLD,
    SIGCONT,
    SIGFPE,
    SIGHUP,
    SIGILL,
    SIGINT,
    SIGKILL,
    SIGPIPE,
    SIGPOLL,
    SIGPROF,
    SIGQUIT,
    SIGSEGV,
    SIGSTOP,
    SIGSYS,
    #[default]
    SIGTERM,
    SIGTRAP,
    SIGTSTP,
    SIGTTIN,
    SIGTTOU,
    SIGUSR1,
    SIGUSR2,
    SIGURG,
    SIGVTALRM,
    SIGXCPU,
    SIGXFSZ,
    SIGWINCH,
}

pub(super) fn new_shared_config() -> Result<SharedConfig, Box<dyn std::error::Error>> {
    Ok(Arc::new(RwLock::new(Config::load()?)))
}

/* -------------------------------------------------------------------------- */
/*                               Implementation                               */
/* -------------------------------------------------------------------------- */
impl Config {
    /// create a config base on the file located in the root of the project
    pub fn load() -> Result<Self, TaskmasterError> {
        let path = Path::new(CONFIG_FILE_PATH);
        let contents = fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&contents)?;
        Ok(config)
    }
}
