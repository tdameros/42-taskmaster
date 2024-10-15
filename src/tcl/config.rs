/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

/* -------------------------------------------------------------------------- */
/*                                  Constants                                 */
/* -------------------------------------------------------------------------- */
const CONFIG_FILE_PATH: &str = "./";

/* -------------------------------------------------------------------------- */
/*                                   Struct                                   */
/* -------------------------------------------------------------------------- */
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub programs: Vec<ProgramConfig>,
}

/// this enum represent whenever a program should be auto restart if it's termination
/// has been detected
#[derive(Debug, Serialize, Deserialize, Default)]
pub enum AutoRestart {
    Always,

    /// if the exit code is not part of the expected exit code list
    Unexpected,

    #[default] // use the field below as default (needed for the default trait)
    Never,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ProgramConfig {
    /// The command to use to launch the program
    #[serde(default)]
    command: String,

    /// The number of processes to start and keep running
    #[serde(default)]
    number_of_process: u32,

    /// Whether to start this program at launch or not
    #[serde(default)]
    start_at_launch: bool,

    /// Whether the program should be restarted always, never, or on unexpected exits only
    #[serde(default)]
    auto_restart: AutoRestart,

    /// Which return codes represent an "expected" exit status
    #[serde(default)]
    expected_exit_code: Vec<u32>,

    /// How long the program should be running after it’s started for it to be considered "successfully started"
    #[serde(default)]
    time_to_start: u32,

    /// How many times a restart should be attempted before aborting
    #[serde(default)]
    max_number_of_restart: u32,

    /// Which signal should be used to stop (i.e. exit gracefully) the program
    #[serde(default)]
    stop_signal: Signal,

    /// How long to wait after a graceful stop before killing the program
    #[serde(default)]
    time_to_stop_gracefully: u32,

    /// Optional stdout redirection
    #[serde(default)]
    stdout_redirection: String,

    /// Optional stderr redirection
    #[serde(default)]
    stderr_redirection: String,

    /// Environment variables to set before launching the program
    #[serde(default)]
    environmental_variable_to_set: Vec<(String, String)>,

    /// A working directory to set before launching the program
    #[serde(default)]
    working_directory: String,

    /// An umask to set before launching the program
    #[serde(default)]
    umask: u32,
}

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

/* -------------------------------------------------------------------------- */
/*                               Implementation                               */
/* -------------------------------------------------------------------------- */
impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = Path::new(CONFIG_FILE_PATH);
        // println!("{:?}", path);
        let contents = fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&contents)?;
        Ok(config)
    }
}
