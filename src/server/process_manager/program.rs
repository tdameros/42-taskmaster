/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use std::{error::Error, fmt::Display};

use crate::{
    config::{Config, ProgramConfig},
    log_error,
    logger::Logger,
};

use super::{Process, ProcessError, ProcessState, Program, ProgramError};

/* -------------------------------------------------------------------------- */
/*                            Struct Implementation                           */
/* -------------------------------------------------------------------------- */
impl Program {
    pub(super) fn new(name: String, config: ProgramConfig) -> Self {
        let mut process_vec = Vec::with_capacity(config.number_of_process);

        for _ in 0..config.number_of_process {
            process_vec.push(Process::new(config.to_owned()));
        }

        Self {
            name,
            config,
            process_vec,
        }
    }

    /// update self state
    pub(super) fn monitor(&mut self, logger: &Logger) {
        self.process_vec.iter_mut().for_each(|process| {
            if let Err(e) = process.react_to_program_state() {
                log_error!(logger, "{e}");
            }
        });
    }

    /// in the event of a config reload this will tell if the given program should be kept as is
    pub(super) fn should_be_kept(&self, config: &Config) -> bool {
        config
            .get(&self.name)
            .map_or(false, |cfg| cfg == &self.config)
    }

    pub(super) fn shutdown_all_process(&mut self, logger: &Logger) {
        self.process_vec.iter_mut().for_each(|process| {
            if let Err(e) = process.send_signal(&self.config.stop_signal) {
                log_error!(logger, "{e}");
                if let Err(e) = process.kill() {
                    log_error!(logger, "{e}");
                }
            }
        });
    }

    pub(super) fn clean_inactive_process(&mut self) {
        use super::ProcessState as PS;
        self.process_vec.retain(|process| match process.state {
            PS::Starting | PS::Running | PS::Stopping => true,
            PS::NeverStartedYet
            | PS::Stopped
            | PS::Backoff
            | PS::ExitedExpectedly
            | PS::ExitedUnExpectedly
            | PS::Fatal
            | PS::Unknown => false,
        });
    }

    pub(super) fn is_clean(&self) -> bool {
        self.process_vec.is_empty()
    }

    /// start all the process of this program
    /// # Returns
    /// - `Ok(())` if every process could be spawn correctly
    /// - `Err(Process)` if something went wrong during the spawning of a process
    /// - `Err(Logic)` if at least one process were found to not be in the NeverStartedYet state
    pub(super) fn start(&mut self) -> Result<(), ProgramError> {
        for process in self.process_vec.iter_mut() {
            if process.state == ProcessState::NeverStartedYet {
                process.start()?;
            } else {
                return Err(ProgramError::Logic(
                    "One ore more process where found to not have the correct state to be started"
                        .to_string(),
                ));
            }
        }

        Ok(())
    }
}

/* -------------------------------------------------------------------------- */
/*                            Error Implementation                            */
/* -------------------------------------------------------------------------- */
impl Error for ProgramError {}

impl Display for ProgramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<ProcessError> for ProgramError {
    fn from(value: ProcessError) -> Self {
        ProgramError::Process(value)
    }
}
