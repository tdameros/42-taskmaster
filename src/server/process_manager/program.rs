/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use std::{error::Error, fmt::Display, thread::sleep, time::Duration};
use tcl::message::Response;

use super::{OrderError, Process, ProcessError, Program, ProgramError};
use crate::{
    config::{Config, ProgramConfig},
    log_error,
    logger::Logger,
};

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

    /// Restarts the program by stopping all processes, waiting briefly, monitoring, and then starting processes.
    ///
    /// # Returns
    /// - `Ok(())` if all processes were successfully restarted.
    /// - `Err(OrderError::PartialSuccess(errors))` if some processes were restarted successfully, but errors occurred.
    /// - `Err(OrderError::TotalFailure(errors))` if all restart attempts failed.
    ///
    /// # Note
    /// This function includes a 1-second delay between stop and start operations.
    pub(super) fn restart(&mut self, logger: &Logger) -> Result<(), OrderError> {
        let stop_results = self.stop();
        sleep(Duration::from_secs(1));
        self.monitor(logger);
        let start_results = self.start();

        squish_order_result(stop_results, start_results)
    }
}

/// Combines the results of stopping and starting operations on processes.
///
/// # Parameters
/// - `stop_results`: The result of the stop operation.
/// - `start_results`: The result of the start operation.
///
/// # Returns
/// - `Ok(())` if both operations succeeded.
/// - `Err(OrderError::PartialSuccess(errors))` if at least one operation was partially successful or had errors.
/// - `Err(OrderError::TotalFailure(errors))` if both operations failed completely with no successes.
fn squish_order_result(
    stop_results: Result<(), OrderError>,
    start_results: Result<(), OrderError>,
) -> Result<(), OrderError> {
    match (stop_results, start_results) {
        (Ok(()), Ok(())) => Ok(()),
        (res1, res2) => {
            let mut all_errors = Vec::new();
            let mut all_total_failure = true;

            for res in [res1, res2].into_iter() {
                match res {
                    Ok(()) => all_total_failure = false,
                    Err(OrderError::PartialSuccess(errors)) => {
                        all_total_failure = false;
                        all_errors.extend(errors);
                    }
                    Err(OrderError::TotalFailure(errors)) => {
                        all_errors.extend(errors);
                    }
                }
            }

            if all_total_failure {
                Err(OrderError::TotalFailure(all_errors))
            } else {
                Err(OrderError::PartialSuccess(all_errors))
            }
        }
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

impl Error for OrderError {}

impl Display for OrderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

/* -------------------------------------------------------------------------- */
/*                             From Implementation                            */
/* -------------------------------------------------------------------------- */
impl Into<tcl::message::ProgramStatus> for &mut Program {
    fn into(self) -> tcl::message::ProgramStatus {
        tcl::message::ProgramStatus {
            name: self.name.to_owned(),
            status: self
                .process_vec
                .iter_mut()
                .map(|process| process.into())
                .collect(),
        }
    }
}

impl Into<Response> for OrderError {
    fn into(self) -> Response {
        Response::Error(self.to_string())
    }
}
