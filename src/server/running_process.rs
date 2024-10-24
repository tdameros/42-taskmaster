/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use std::{
    marker::{self, PhantomData},
    process::{Child, ExitStatus},
    time::SystemTime,
};

use crate::config::{ProgramConfig, Signal};

/* -------------------------------------------------------------------------- */
/*                                   Struct                                   */
/* -------------------------------------------------------------------------- */
#[derive(Debug)]
pub(super) struct RunningProcess {
    // the handle to the process
    child: Child,

    // the time when the process was launched
    started_since: SystemTime, // to clarify

    // use to determine when to abort the child
    time_since_shutdown: Option<SystemTime>,
}

/* -------------------------------------------------------------------------- */
/*                            Struct Implementation                           */
/* -------------------------------------------------------------------------- */
impl RunningProcess {
    /// create a new RunningProcess struct based on the given child
    pub(super) fn new(child: Child) -> Self {
        Self {
            child,
            started_since: SystemTime::now(),
            time_since_shutdown: None,
        }
    }

    /// return if the child is still alive or an error if one
    /// occurred while trying to know if the child was alive
    pub(super) fn is_alive(&mut self) -> Result<bool, std::io::Error> {
        Ok(self.child.try_wait()?.is_some())
    }

    /// try to return the child exit code if some is found,
    /// an error is returned if the exist status could not be read
    /// if the child is alive Ok(None) is return
    /// if the child is dead the Ok(Some(Option<i32>)) is return
    /// as on unix dead child might not have exit code see the documentation
    /// of std::process::Child::try_wait() for more info
    pub(super) fn get_exit_code(&mut self) -> Result<Option<Option<i32>>, std::io::Error> {
        Ok(self.child.try_wait()?.map(|status| status.code()))
    }

    /// return the child process_id
    pub(super) fn get_child_id(&self) -> u32 {
        self.child.id()
    }

    /// try to send a SIGKILL to the child returning an error if not able to
    /// this can be due to lack of privilege
    pub(super) fn kill(&mut self) -> Result<(), std::io::Error> {
        self.child.kill()
    }

    /// return true if the child is still alive while having receive a graceful
    /// shutdown request since longer than the maximum value present in the given config
    pub(super) fn its_time_to_kill_the_child(&self, program_config: &ProgramConfig) -> bool {
        self.time_since_shutdown
            .map(|time_since_shutdown| {
                program_config.time_to_stop_gracefully
                    < SystemTime::now()
                        .duration_since(time_since_shutdown)
                        .unwrap_or_default()
                        .as_secs() as u64
            })
            .unwrap_or(false)
    }

    /// return whenever the program was considered as running
    /// AKA paste the time allowed for starting
    pub(super) fn program_was_running(&self, program_config: &ProgramConfig) -> bool {
        let time_since_start = SystemTime::now()
            .duration_since(self.started_since)
            .unwrap_or_default();

        program_config.time_to_start > time_since_start.as_secs()
    }

    /// return whenever the program has already receive a graceful shutdown order
    pub(super) fn has_received_shutdown_order(&self) -> bool {
        self.time_since_shutdown.is_some()
    }

    /// send the given signal to the child, starting the gracefully shutdown timer
    pub(super) fn send_signal(&mut self, signal: &Signal) {
        // TODO use signal or other mean to send the correct signal
        self.time_since_shutdown = Some(SystemTime::now());
    }
}
