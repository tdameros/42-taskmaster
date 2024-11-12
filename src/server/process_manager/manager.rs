/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use super::{Program, ProgramError, ProgramManager, SharedProcessManager};
use crate::{
    config::Config,
    log_error,
    logger::{Logger, SharedLogger},
};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    thread::{self, JoinHandle},
    time::Duration,
};
use tcl::{message::Response, mylibc::{sigset_t, How, SIGHUP}};

/* -------------------------------------------------------------------------- */
/*                            Struct Implementation                           */
/* -------------------------------------------------------------------------- */
impl ProgramManager {
    /// return an instance of ProcessManager
    fn new(config: &Config) -> Self {
        let mut programs = HashMap::<String, Program>::default();
        let purgatory = HashMap::<String, Program>::default();

        config.iter().for_each(|(program_name, program_config)| {
            let program = Program::new(program_name.to_owned(), program_config.to_owned());
            programs.insert(program_name.to_owned(), program);
        });

        Self {
            programs,
            purgatory,
        }
    }

    fn monitor_once(&mut self, logger: &Logger) {
        self.monitor_program_once(logger);
        self.monitor_purgatory_once(logger);
    }

    /// this function iter over every process in programs and check update it's status
    fn monitor_program_once(&mut self, logger: &Logger) {
        self.programs.iter_mut().for_each(|(_name, program)| {
            program.monitor(logger);
        });
    }

    /// this function iter over every process in the purgatory and check update it's status
    fn monitor_purgatory_once(&mut self, logger: &Logger) {
        self.purgatory.iter_mut().for_each(|(_name, program)| {
            program.monitor(logger);
        });
        self.clean_purgatory();
    }

    /// try to conform to the new config
    pub fn reload_config(&mut self, config: &Config, logger: &Logger) {
        // remove unwanted program from the list of program
        self.drain_to_purgatory(config);
        // shut them down
        self.shutdown_purgatory(logger);
        // add the new program
        self.add_new_program(config);
    }

    /// this function add to self every program in the config that are not already present in self
    fn add_new_program(&mut self, config: &Config) {
        config.iter().for_each(|(name, config)| {
            if !self.programs.contains_key(name) {
                self.programs.insert(
                    name.to_owned(),
                    Program::new(name.to_owned(), config.to_owned()),
                );
            }
        });
    }

    fn drain_to_purgatory(&mut self, config: &Config) {
        self.purgatory.extend(
            self.programs
                .drain()
                .filter(|(_name, program)| !program.should_be_kept(config)),
        );
    }

    /// perform a shutdown of all the program inside the purgatory
    /// this may not be effective immediately as some program may need time to properly shutdown
    fn shutdown_purgatory(&mut self, logger: &Logger) {
        self.purgatory.iter_mut().for_each(|(_name, program)| {
            program.shutdown_all_process(logger);
        });
    }

    /// try to remove as many program as possible from the purgatory leaving only the still running program
    fn clean_purgatory(&mut self) {
        self.purgatory.iter_mut().for_each(|(_name, program)| {
            program.clean_inactive_process();
        });
        self.purgatory.retain(|_name, program| !program.is_clean());
    }

    /// this function spawn a thread the will monitor all process in self updating there status as needed, refreshing every refresh_period
    pub async fn monitor(
        shared_process_manager: SharedProcessManager,
        shared_logger: SharedLogger,
        refresh_period: Duration,
    ) -> Result<JoinHandle<()>, std::io::Error> {
        thread::Builder::new().spawn(move || {
            let how = How::SIG_BLOCK;
            let mut set = sigset_t::default();
            set.add(SIGHUP).unwrap();
            tcl::mylibc::pthread_sigmask(how, &set, None).unwrap();
            loop {
            shared_process_manager
                .write()
                .unwrap()
                .monitor_once(&shared_logger);
            thread::sleep(refresh_period);
        }})
    }

    /// Use for user manual starting of a program's process
    pub fn start_program(&mut self, program_name: &str, logger: &Logger) -> Response {
        self.programs.get_mut(program_name).map_or(
            Response::Error("couldn't found a program named : {program_name}".to_string()),
            |program| match program.start() {
                Ok(_) => Response::Success("Starting task succeed".to_string()),
                Err(e) => match e {
                    super::OrderError::PartialSuccess(errors) => {
                        let error_message = format!(
                            "Partial success starting program '{}'. Errors: {}",
                            program_name,
                            format_errors(&errors)
                        );
                        log_error!(logger, "{error_message}");
                        Response::Error(error_message)
                    }
                    super::OrderError::TotalFailure(errors) => {
                        let error_message = format!(
                            "Failed to start program '{}'. Errors: {}",
                            program_name,
                            format_errors(&errors)
                        );
                        log_error!(logger, "{error_message}");
                        Response::Error(error_message)
                    }
                },
            },
        )
    }

    /// use for user manual shutdown of a program's process
    pub fn stop_program(&mut self, program_name: &str, logger: &Logger) -> Response {
        self.programs.get_mut(program_name).map_or(
            Response::Error("couldn't found a program named : {program_name}".to_string()),
            |program| match program.stop() {
                Ok(_) => Response::Success("stopping task succeed".to_string()),
                Err(e) => match e {
                    super::OrderError::PartialSuccess(errors) => {
                        let error_message = format!(
                            "Partial success stopping program '{}'. Errors: {}",
                            program_name,
                            format_errors(&errors)
                        );
                        log_error!(logger, "{error_message}");
                        Response::Error(error_message)
                    }
                    super::OrderError::TotalFailure(errors) => {
                        let error_message = format!(
                            "Failed to stop program '{}'. Errors: {}",
                            program_name,
                            format_errors(&errors)
                        );
                        log_error!(logger, "{error_message}");
                        Response::Error(error_message)
                    }
                },
            },
        )
    }

    /// use for user manual restart of a program's process
    pub fn restart_program(&mut self, program_name: &str, logger: &Logger) -> Response {
        self.programs.get_mut(program_name).map_or(
            Response::Error("couldn't found a program named : {program_name}".to_string()),
            |program| match program.restart(logger) {
                Ok(_) => Response::Success("stopping task succeed".to_string()),
                Err(e) => match e {
                    super::OrderError::PartialSuccess(errors) => {
                        let error_message = format!(
                            "Partial success stopping program '{}'. Errors: {}",
                            program_name,
                            format_errors(&errors)
                        );
                        log_error!(logger, "{error_message}");
                        Response::Error(error_message)
                    }
                    super::OrderError::TotalFailure(errors) => {
                        let error_message = format!(
                            "Failed to stop program '{}'. Errors: {}",
                            program_name,
                            format_errors(&errors)
                        );
                        log_error!(logger, "{error_message}");
                        Response::Error(error_message)
                    }
                },
            },
        )
    }

    /// use for user manual status command
    pub fn get_status(&mut self) -> Response {
        self.into()
    }
}

fn format_errors(errors: &[ProgramError]) -> String {
    errors
        .iter()
        .map(|e| match e {
            ProgramError::Logic(msg) => format!("Logic error: {}", msg),
            ProgramError::Process(err) => format!("Process error: {:?}", err),
        })
        .collect::<Vec<String>>()
        .join(", ")
}

pub fn new_shared_process_manager(config: &Config) -> SharedProcessManager {
    Arc::new(RwLock::new(ProgramManager::new(config)))
}

/* -------------------------------------------------------------------------- */
/*                             From Implementation                            */
/* -------------------------------------------------------------------------- */
impl From<&mut ProgramManager> for Response {
    fn from(val: &mut ProgramManager) -> Self {
        Response::Status(
            val.programs
                .iter_mut()
                .map(|(_, program)| program.into())
                .collect(),
        )
    }
}
