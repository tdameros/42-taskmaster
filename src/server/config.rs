/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use serde::de::{self, Unexpected};
use serde::{Deserialize, Deserializer, Serialize};
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
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ProgramConfig {
    /// The command to use to launch the program
    #[serde(rename = "cmd", default)]
    pub(super) command: String,

    /// The number of processes to start and keep running
    #[serde(rename = "numprocs", default)]
    pub(super) number_of_process: usize,

    /// Whether to start this program at launch or not
    #[serde(rename = "autostart", default)]
    pub(super) start_at_launch: bool,

    /// Whether the program should be restarted always, never, or on unexpected exits only
    #[serde(rename = "autorestart", default)]
    pub(super) auto_restart: AutoRestart,

    /// Which return codes represent an "expected" exit status
    #[serde(rename = "exitcodes", default)]
    pub(super) expected_exit_code: Vec<i32>,

    /// How long the program should be running after it’s started for it to be considered "successfully started"
    #[serde(rename = "starttime", default)]
    pub(super) time_to_start: u64,

    /// How many times a restart should be attempted before aborting
    /// this is shared between replica
    #[serde(rename = "startretries", default)]
    pub(super) max_number_of_restart: u32,

    /// Which signal should be used to stop (i.e. exit gracefully) the program
    #[serde(rename = "stopsignal", default)]
    pub(super) stop_signal: Signal,

    /// How long to wait after a graceful stop before killing the program
    #[serde(rename = "stoptime", default)]
    pub(super) time_to_stop_gracefully: u64,

    /// Optional stdout redirection
    #[serde(rename = "stdout")]
    pub(super) stdout_redirection: Option<String>,

    /// Optional stderr redirection
    #[serde(rename = "stderr")]
    pub(super) stderr_redirection: Option<String>,

    /// Environment variables to set before launching the program
    #[serde(rename = "env")]
    pub(super) environmental_variable_to_set: Option<HashMap<String, String>>,

    /// A working directory to set before launching the program
    #[serde(rename = "workingdir")]
    pub(super) working_directory: Option<String>,

    /// An umask to set before launching the program
    #[serde(rename = "umask", default, deserialize_with = "parse_umask")]
    pub(super) umask: Option<libc::mode_t>,
}

/// this enum represent whenever a program should be auto restart if it's termination
/// has been detected
#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
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
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
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
    #[cfg(target_os = "linux")]
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

fn parse_umask<'de, D>(deserializer: D) -> Result<Option<libc::mode_t>, D::Error>
where
    D: Deserializer<'de>,
{
    let umask_deserialize = Option::<String>::deserialize(deserializer)?;
    if let Some(umask_str) = umask_deserialize {
        if !umask_str.chars().all(|c| ('0'..='7').contains(&c)) {
            return Err(de::Error::invalid_value(
                Unexpected::Str(&umask_str),
                &"octal number",
            ));
        }
        libc::mode_t::from_str_radix(&umask_str, 8)
            .map(Some)
            .map_err(|_| de::Error::custom("invalid umask"))
    } else {
        Ok(None)
    }
}

fn parse_user<'de, D>(deserializer: D) -> Result<Option<User>, D::Error>
where
    D: Deserializer<'de>,
{
    let user_deserialize = Option::<String>::deserialize(deserializer)?;
    match user_deserialize {
        Some(user_str) => {
            if let Some(user) = get_all_users()
                .iter()
                .find(|u| u.username == user_str)
                .cloned()
            {
                Ok(Some(user))
            } else {
                Err(de::Error::custom("invalid user"))
            }
        }
        None => Ok(None),
    }
}

fn get_all_users() -> Vec<User> {
    let mut users: Vec<User> = Vec::new();
    unsafe {
        libc::setpwent();
        while let Some(user) = libc::getpwent().as_mut() {
            let username = CStr::from_ptr(user.pw_name);
            if let Ok(username) = username.to_str() {
                users.push(User {
                    username: username.to_owned(),
                    uid: user.pw_uid,
                    gid: user.pw_gid,
                })
            }
        }
    }
    users
}

impl ProgramConfig {
    pub(super) fn should_restart(&self, exit_code: i32) -> bool {
        match self.expected_exit_code.contains(&exit_code) {
            true => self.auto_restart == AutoRestart::Always,
            false => self.auto_restart != AutoRestart::Never,
        }
    }
}
