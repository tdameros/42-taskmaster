/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use tcl::message::Response;

use super::{Program, ProgramManager};
use crate::{
    config::{Config, SharedConfig},
    logger::{Logger, SharedLogger},
};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    thread::{self, JoinHandle},
    time::Duration,
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

    fn monitor_once(&mut self, logger: &Logger) {
        self.monitor_program_once(logger);
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
    fn reload_config(&mut self, config: &Config, logger: &Logger) {
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
    pub(super) async fn monitor(
        &mut self,
        shared_process_manager: SharedProcessManager,
        shared_config: SharedConfig,
        shared_logger: SharedLogger,
        refresh_period: Duration,
    ) -> Result<JoinHandle<()>, std::io::Error> {
        let shared = Arc::new(RwLock::new(self));
        thread::Builder::new().spawn(move || loop {
            self.monitor_once();
            thread::sleep(refresh_period);
        })
    }

    /// Use for user manual starting of a program
    pub(super) fn start_program(&mut self, program_name: &str) -> Response {
        self.programs.get_mut(program_name).map_or(
            Response::Error("couldn't found a program named : {program_name}".to_string()),
            |program| match program.start() {
                Ok(_) => Response::Success("Starting task succeed".to_string()),
                Err(_) => {
                    Response::Error("Something went wrong while spawning process".to_string())
                }
            },
        )
    }
}
