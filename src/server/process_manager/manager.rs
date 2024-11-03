/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use super::{Process, ProcessState, Program, ProgramManager, ProgramToRestart};
use crate::{
    config::{Config, ProgramConfig, SharedConfig},
    log_debug, log_error, log_info,
    logger::{Logger, SharedLogger},
};
use std::process::Stdio;
use std::{
    collections::HashMap,
    error::Error,
    fmt::Display,
    fs,
    ops::{Deref, DerefMut, Neg},
    process::Command,
    sync::{Arc, RwLock},
    thread::{self, JoinHandle},
    time::Duration,
};

/* -------------------------------------------------------------------------- */
/*                            Struct Implementation                           */
/* -------------------------------------------------------------------------- */
impl ProgramToRestart {
    fn add_or_increment(&mut self, program_name: &String, program_config: &ProgramConfig) {
        match self.0.get_mut(program_name) {
            Some((old_number, _)) => {
                // there where an other process from the same program
                *old_number += 1;
            }
            None => {
                // no ancient value so we insert one
                self.0
                    .insert(program_name.to_owned(), (1, program_config.to_owned()));
            }
        }
    }
}

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

    /// this function spawn all the replica of a given program given a reference to a programs config
    pub fn spawn_program(
        &mut self,
        program_name: &str,
        program_config: &ProgramConfig,
        logger: &Logger,
    ) {
        // for each process that must be spawn we spawned it using the given argument
        for process_number in 0..program_config.number_of_process {
            // if the child can't be spawn the command is probably wrong or the privilege is not right we cannot do anything,
            // retrying would be useless so we just log the error and go to the next which will probably fail too
            if let Err(error) = self.spawn_child(program_config, program_name) {
                log_error!(
                    logger,
                    "{}",
                    format!("Fatal couldn't spawn child : {process_number} because of : {error}")
                );
            }
        }
    }

    /// shutdown all child of a given process if no child exist for the given program name the function does nothing
    /// if the config does'nt contain the program then SIGKILL is use instead
    /// if the kill command failed (probably due to insufficient privilege) it's error is return
    pub(super) fn shutdown_childs(
        &mut self,
        name: &str,
        config: &Config, // TODO use it i've vonlonterly pass this as config and not rwlock for the monitor once function to not get lock since it lock in write and then call this function
        logger: &Logger,
    ) -> Result<(), KillingChildError> {
        match self.children.get_mut(name) {
            // if the given program have running process we iterate over them and send to them the correct signal
            Some(processes) => {
                /*
                we need to decide if we keep the child around for this we need to know
                if the process must be killed or gracefully shutdown
                if gracefully shutdown we need to keep the process else if the kill
                happened correctly we need to get ride of them if the kill didn't happened correctly
                we need to exit and warn the user that this did'nt go according to plan. we can't however
                remove the process
                */
                match config.programs.get(name) {
                    Some(config) => {
                        // here we keep the process just sending them the signal required by the config
                        for process in processes.iter_mut() {
                            log_info!(
                                logger,
                                "about to gracefully shutdown process number : {}",
                                process.get_child_id()
                            );
                            process.send_signal(&config.stop_signal)?;
                            process.set_status(ProcessStatus::Stopped);
                        }
                        Ok(())
                    }
                    None => {
                        // here we killed them
                        // PS if you want to learn more about closure you can try to transform the for loop in the iter version to see what happen
                        for process in processes.iter_mut() {
                            log_info!(
                                logger,
                                "about to kill process number : {}",
                                process.get_child_id()
                            );
                            process.kill()?;
                            process.set_status(ProcessStatus::Stopped);
                        }
                        Ok(())
                    }
                }
            }
            // if no process are found for a given name we return the corresponding error
            None => {
                log_info!(
                    logger,
                    "tried to remove child of process: {name} but none where found"
                );
                Err(KillingChildError::NoProgramFound)
            }
        }
        // here we don't remove the entry since the monitor will do it for use
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

    pub fn get_processes_state(&mut self) -> HashMap<String, Vec<ProcessState>> {
        let mut result: HashMap<String, Vec<ProcessState>> = HashMap::new();
        for (name, childs) in self.children.iter() {
            let mut processes_state: Vec<ProcessState> = Vec::new();
            for child in childs.iter() {
                processes_state.push(ProcessState::from(child));
            }
            result.insert(name.to_string(), processes_state);
        }
        result
    }
}

impl From<&RunningProcess> for ProcessState {
    fn from(process: &RunningProcess) -> Self {
        ProcessState {
            pid: process.get_child_id(),
            status: process.get_status(),
            start_time: process.get_start_time(),
            shutdown_time: process.get_shutdown_time(),
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                                 Sub Module                                 */
/* -------------------------------------------------------------------------- */
mod monitoring {
    use super::*;

    pub(super) fn check_inside(
        config_access: &mut std::sync::RwLockWriteGuard<'_, Config>,
        program_name: &String,
        vec_running_process: &mut Vec<Process>,
        logger: &Logger,
        program_to_restart: &mut ProgramToRestart,
        program_to_remove: &mut Vec<String>,
    ) {
        // check if the process name we are on is in the new config
        match config_access.programs.get_mut(program_name) {
            // the program running is still in the config, so we just need to perform check
            Some(program_config) => {
                // keep only the good child AKA healthy child
                vec_running_process.retain_mut(|running_process| {
                    match running_process.get_exit_code() {
                        Err(error) => {
                            log_error!(
                                logger,
                                "error gotten while trying to read a child status : {error}"
                            );
                            true // we keep 
                        }
                        Ok(None) => {
                            // the program is alive
                            // we need to check if it's time to kill the child
                            if running_process.has_received_shutdown_order()
                                && running_process
                                .its_time_to_kill_the_child(program_config)
                            {
                                if let Err(error) = running_process.kill() {
                                    log_error!(logger, "Can't kill a child: {error}");
                                    return true; // we keep it if we can't kill it
                                } else {
                                    return false; // we don't keep it if we successfully killed him
                                }
                            }
                            true // we keep every alive process except the one that should be dead and are successfully killed
                        }
                        Ok(Some(exit_code)) => {
                            // the program is dead
                            // we need to check if the program should be restarted
                            // first we need to check if the process is dead while starting
                            if !running_process.program_was_running(program_config)
                                && program_config.max_number_of_restart > 0
                            {
                                // we decrement the number of allowed restart for next time
                                program_config.max_number_of_restart -= 1;
                                program_to_restart.add_or_increment(program_name, program_config);
                            } else {
                                match exit_code {
                                    // if there is an exit code we ask the config to see if we need to restart the program
                                    Some(exit_code) => {
                                        if program_config.should_restart(exit_code) {
                                            program_to_restart.add_or_increment(program_name, program_config);
                                        }
                                    },
                                    // if no exit code we log the error
                                    None => {
                                        log_error!(logger, "Found a child with no exit status code in program: {program_name} adding it to the list for removal");
                                    },
                                }
                            }
                            false // we don't keep dead program
                        }
                    }
                });
            }
            // the program running is not in the config anymore so we need to murder his family
            None => {
                // then this program must be removed
                program_to_remove.push(program_name.to_owned());
            }
        };
    }

    pub(super) fn filter_inside(
        config_access: &std::sync::RwLockWriteGuard<'_, Config>,
        program_name: &String,
        vec_running_program: &mut [Process],
        program_to_restart: &mut ProgramToRestart,
    ) -> Result<(), KillingChildError> {
        // if the program have a config this is to prevent itering over child that can't be killed (killed because they was not in the config anymore)
        if let Some(config) = config_access.programs.get(program_name) {
            let number_of_non_stopping_process = vec_running_program
                .iter()
                .filter(|running_process| !running_process.has_received_shutdown_order())
                .count(); // this is the number of truly running process
            let mut overflowing_process_number =
                number_of_non_stopping_process as i64 - config.number_of_process as i64;
            match overflowing_process_number.cmp(&0) {
                std::cmp::Ordering::Less => {
                    // we need to start restarting some program then start event more if it's not enough

                    // we need to store the true restart number since we can call a &mut self method in a iter_mut block for very good reason ^^ think about it
                    match program_to_restart.get_mut(program_name) {
                        Some((restart_number, _config)) => {
                            *restart_number = overflowing_process_number.neg()
                        } // this become
                        None => {
                            program_to_restart.insert(
                                program_name.to_owned(),
                                (overflowing_process_number.neg(), config.to_owned()),
                            );
                        }
                    }
                }
                std::cmp::Ordering::Equal => {
                    // we have just the right number of process we don't need to do anything
                }
                std::cmp::Ordering::Greater => {
                    // we need to shutdown the difference
                    for running_process in vec_running_program.iter_mut().rev() {
                        if overflowing_process_number == 0 {
                            break;
                        }
                        if !running_process.has_received_shutdown_order() {
                            running_process.send_signal(&config.stop_signal)?;
                            overflowing_process_number -= 1;
                        }
                    }
                }
            };
        }
        Ok(())
    }
}

/* -------------------------------------------------------------------------- */
/*                            Trait Implementation                            */
/* -------------------------------------------------------------------------- */
impl Deref for ProgramToRestart {
    type Target = HashMap<String, (i64, ProgramConfig)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ProgramToRestart {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<ProgramManager> for SharedProcessManager {
    fn from(value: ProgramManager) -> Self {
        Arc::new(RwLock::new(value))
    }
}
