/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use std::{error::Error, fmt::Display};

use crate::{
    config::{Config, ProgramConfig},
    log_error,
    logger::Logger,
};

use super::{OrderError, Process, ProcessError, ProcessState, Program, ProgramError};

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

    /// Attempts to start all processes of this program.
    ///
    /// # Returns
    /// - `Ok(())` if all processes were started successfully or were already active.
    /// - `Err(OrderError::PartialSuccess(errors))` if at least one process was started successfully,
    ///   but some errors occurred (includes both logic and process errors).
    /// - `Err(OrderError::TotalFailure(errors))` if all attempts to start processes failed due to
    ///   process errors (no successes and no active processes).
    pub(super) fn start(&mut self) -> Result<(), OrderError> {
        let results: Vec<Result<(), ProgramError>> = self
            .process_vec
            .iter_mut()
            .map(|process| {
                if process.is_active() {
                    Err(ProgramError::Logic("Process is already active".to_string()))
                } else {
                    process.start().map_err(ProgramError::Process)
                }
            })
            .collect();

        determine_order_result(results)
    }

    /// Attempts to stop all processes of this program.
    ///
    /// # Returns
    /// - `Ok(())` if all processes were stopped successfully or were already inactive.
    /// - `Err(OrderError::PartialSuccess(errors))` if at least one process was stopped successfully,
    ///   but some errors occurred (includes both logic and process errors).
    /// - `Err(OrderError::TotalFailure(errors))` if all attempts to stop processes failed due to
    ///   process errors (no successes and no inactive processes).
    pub(super) fn stop(&mut self) -> Result<(), OrderError> {
        let results: Vec<Result<(), ProgramError>> = self
            .process_vec
            .iter_mut()
            .map(|process| {
                if !process.is_active() {
                    Err(ProgramError::Logic(
                        "Process is already inactive".to_string(),
                    ))
                } else {
                    process
                        .send_signal(&self.config.stop_signal)
                        .or_else(|_| process.kill())
                        .map_err(ProgramError::Process)
                }
            })
            .collect();

        determine_order_result(results)
    }
}

/// Determines the overall result of a bulk operation on processes (start, stop, or restart).
///
/// # Parameters
/// - `results`: A vector of individual process operation results.
///
/// # Returns
/// - `Ok(())` if all operations were successful.
/// - `Err(OrderError::PartialSuccess(errors))` if there were any logic errors or at least one success.
/// - `Err(OrderError::TotalFailure(errors))` if all errors were process errors and no successes.
fn determine_order_result(results: Vec<Result<(), ProgramError>>) -> Result<(), OrderError> {
    let (successes, errors): (Vec<_>, Vec<_>) = results.into_iter().partition(Result::is_ok);

    if errors.is_empty() {
        // the case were there was no error at all
        return Ok(());
    }

    let (logic_errors, process_errors): (Vec<_>, Vec<_>) = errors
        .into_iter()
        .map(Result::unwrap_err)
        .partition(|error| matches!(error, ProgramError::Logic(_)));

    if logic_errors.is_empty() && successes.is_empty() {
        // if no success and no skip(AKA logic error)
        Err(OrderError::TotalFailure(process_errors))
    } else {
        Err(OrderError::PartialSuccess(
            logic_errors
                .into_iter()
                .chain(process_errors.into_iter())
                .collect(),
        ))
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
