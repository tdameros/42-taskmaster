/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use super::{Program, ProgramError, ProgramManager, SharedProcessManager};
use crate::ring_buffer::RingBuffer;
use crate::{
    config::Config,
    log_error,
    logger::{Logger, SharedLogger},
};
use std::option::Option;
use std::{collections::HashMap, sync::Arc};
use tcl::message::Response;
use tokio::{
    sync::{broadcast, RwLock},
    task::JoinHandle,
    time::{sleep, Duration},
};
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

    async fn monitor_once(&mut self, logger: &Logger) {
        self.monitor_program_once(logger).await;
        self.monitor_purgatory_once(logger).await;
    }

    /// this function iter over every process in programs and check update it's status
    async fn monitor_program_once(&mut self, logger: &Logger) {
        for (_name, program) in self.programs.iter_mut() {
            program.monitor(logger).await;
        }
    }

    /// this function iter over every process in the purgatory and check update it's status
    async fn monitor_purgatory_once(&mut self, logger: &Logger) {
        for (_name, program) in self.purgatory.iter_mut() {
            program.monitor(logger).await;
        }
        self.clean_purgatory();
    }

    /// try to conform to the new config
    pub async fn reload_config(&mut self, config: &Config, logger: &Logger) {
        // remove unwanted program from the list of program
        self.drain_to_purgatory(config);
        // shut them down
        self.shutdown_purgatory(logger).await;
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
        let mut new_programs = HashMap::new();
        for (name, program) in self.programs.drain() {
            match program.should_be_kept(config) {
                true => new_programs.insert(name, program),
                false => self.purgatory.insert(name, program),
            };
        }
        self.programs = new_programs;
    }

    /// perform a shutdown of all the program inside the purgatory
    /// this may not be effective immediately as some program may need time to properly shutdown
    async fn shutdown_purgatory(&mut self, logger: &Logger) {
        for (_name, program) in self.purgatory.iter_mut() {
            program.shutdown_all_process(logger).await;
        }
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
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                shared_process_manager
                    .write()
                    .await
                    .monitor_once(&shared_logger)
                    .await;
                sleep(refresh_period).await;
            }
        })
    }

    /// Use for user manual starting of a program's process
    pub async fn start_program(&mut self, program_name: &str, logger: &Logger) -> Response {
        if let Some(program) = self.programs.get_mut(program_name) {
            match program.start().await {
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
            }
        } else {
            Response::Error(format!("couldn't found a program named : {}", program_name))
        }
    }

    /// use for user manual shutdown of a program's process
    pub async fn stop_program(&mut self, program_name: &str, logger: &Logger) -> Response {
        if let Some(program) = self.programs.get_mut(program_name) {
            match program.stop().await {
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
            }
        } else {
            Response::Error(format!("couldn't find a program named : {program_name}"))
        }
    }

    /// use for user manual restart of a program's process
    pub async fn restart_program(&mut self, program_name: &str, logger: &Logger) -> Response {
        if let Some(program) = self.programs.get_mut(program_name) {
            match program.restart(logger).await {
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
            }
        } else {
            Response::Error(format!("couldn't found a program named : {}", program_name))
        }
    }
    /// use for user manual status command
    pub fn get_status(&mut self) -> Response {
        self.into()
    }

    pub async fn subscribe(&mut self, program_name: &str) -> Option<broadcast::Receiver<String>> {
        match self.programs.get_mut(program_name) {
            Some(program) => Some(program.process_vec[0].subscribe().await),
            None => None,
        }
    }

    pub async fn get_history(&mut self, program_name: &str) -> Option<RingBuffer<String>> {
        match self.programs.get_mut(program_name) {
            Some(program) => Some(program.process_vec[0].get_stdout_history().await),
            None => None,
        }
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
