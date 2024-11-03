/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use std::path::Iter;

use crate::{
    config::{Config, ProgramConfig},
    log_error,
    logger::Logger,
};

use super::{Process, Program};

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
}
