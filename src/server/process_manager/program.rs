/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

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
}
