/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use crate::{
    config::{Config, ProgramConfig, SharedConfig},
    log_debug, log_error, log_info,
    logger::{Logger, SharedLogger},
    running_process::Process,
};
use std::{
    borrow::Borrow,
    collections::HashMap,
    error::Error,
    fmt::Display,
    ops::{Deref, DerefMut, Neg},
    process::Command,
    sync::{Arc, RwLock},
    thread::{self, JoinHandle},
    time::Duration,
};

/* -------------------------------------------------------------------------- */
/*                                   Struct                                   */
/* -------------------------------------------------------------------------- */
/// this represent the running process
#[derive(Debug)]
pub(super) struct ProcessManager(HashMap<String, Vec<Process>>);

/// a sharable version of a process manager, it can be passe through thread safely + use in a concurrent environment without fear thank Rust !
pub(super) type SharedProcessManager = Arc<RwLock<ProcessManager>>;

pub(super) fn new_shared_process_manager(
    config: &RwLock<Config>,
    logger: &Logger,
) -> SharedProcessManager {
    Arc::new(RwLock::new(ProcessManager::new_from_config(config, logger)))
}

/// exist simply for an ease of implementation
#[derive(Debug, Default)]
struct ProgramToRestart(pub HashMap<String, (i64, ProgramConfig)>);

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

impl ProcessManager {
    /// return an instance of ProcessManager
    fn new(config: &Config) -> Self {
        let mut result = HashMap::<String, Vec<Process>>::default();

        config
            .programs
            .iter()
            .for_each(|(program_name, program_config)| {
                let mut process_vec = Vec::with_capacity(program_config.number_of_process);
                for _ in 0..program_config.number_of_process {
                    process_vec.push(Process::new());
                }
                result.insert(
                    program_name.to_owned(),
                    Vec::with_capacity(program_config.number_of_process),
                );
            });

        Self(result)
    }

    fn monitor_once(&mut self) {
        self.0.iter_mut().for_each(|(program_name, process_vec)| {
            process_vec.iter_mut().for_each(|process| {
                let exit_code = process.get_exit_code();
            });
        });
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

    /// this function spawn a child given the program config for a program it then insert a newly created
    /// running process struct into the vec of running process for the given program name in self,
    /// creating it if it doesn't exist yet
    fn spawn_child(
        &mut self,
        program_config: &ProgramConfig,
        name: &str,
    ) -> Result<(), std::io::Error> {
        // get the command and arguments
        let split_command: Vec<&str> = program_config.command.split_whitespace().collect();

        if !split_command.is_empty() {
            // create the command using the command property given by the program config
            let mut tmp_child = Command::new(split_command.first().expect("Unreachable"));

            // TODO change the pwd according to the config

            // TODO add env variable

            // adding arguments if there are any in the command section of program config
            if split_command.len() > 1 {
                tmp_child.args(&split_command[1..]);
            }

            // TODO stdout and err redirection

            // spawn the child returning if failed
            let child = tmp_child.spawn()?;

            // create a instance of running process with the info of this given child
            let process = Process::new(child);

            // insert the running process newly created to self at the end of the vector of running process for the given program name entry, creating a new empty vector if none where found
            self.children
                .entry(name.to_string())
                .or_default()
                .push(process);
        }

        Ok(())
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

    // /// do one round of monitoring
    // fn monitor_once(&mut self, config: &RwLock<Config>, logger: &Logger) {
    //     // query the new config
    //     let mut config_access = config
    //         .write()
    //         .expect("Some user of the config lock has panicked");
    //     log_debug!(logger, "{config_access:?}");

    //     let mut program_to_remove = Vec::new();
    //     let mut program_to_restart = ProgramToRestart::default();

    //     // iterate over all process
    //     self.children
    //         .iter_mut()
    //         .for_each(|(program_name, vec_running_process)| {
    //             monitoring::check_inside(
    //                 &mut config_access,
    //                 program_name,
    //                 vec_running_process,
    //                 logger,
    //                 &mut program_to_restart,
    //                 &mut program_to_remove,
    //             );
    //         });
    //     log_debug!(logger, "{self:?}");

    //     self.monitor_shutdown_childs(program_to_remove, &config_access, logger);
    //     // after this point self contain only running child, child in the shutdown phase, child were getting there status code returned an error and unkillable child
    //     // so if we filter on child that do not have a time_since_shutdown we have the number of child that are running and we can compare it to the desire number
    //     // to see if we need to kill additional child or start restarting the one we detected that we musted restart
    //     log_debug!(logger, "{self:?}");

    //     // remove excess program
    //     self.children
    //         .iter_mut()
    //         .for_each(|(program_name, vec_running_program)| {
    //             if let Err(error) = monitoring::filter_inside(
    //                 &config_access,
    //                 program_name,
    //                 vec_running_program,
    //                 &mut program_to_restart,
    //             ) {
    //                 match error {
    //                     KillingChildError::NoProgramFound => unreachable!(),
    //                     KillingChildError::CantKillProcess => {
    //                         log_error!(
    //                             logger,
    //                             "Can't kill a child of process: {program_name}: {error}"
    //                         );
    //                     }
    //                 }
    //             }
    //         });
    //     log_debug!(logger, "{self:?}");

    //     self.monitor_restart_childs(program_to_restart, logger);
    //     log_debug!(logger, "{self:?}");
    // }

    // fn monitor_restart_childs(&mut self, program_to_restart: ProgramToRestart, logger: &Logger) {
    //     // restart the program
    //     program_to_restart.0.iter().for_each(
    //         |(program_name, (number_of_process, program_config))| {
    //             for _ in 0..*number_of_process {
    //                 if let Err(error) = self.spawn_child(program_config, program_name) {
    //                     log_error!(logger, "Can't spawn child of {program_name}: {error}");
    //                 }
    //             }
    //         },
    //     );
    // }

    // fn monitor_shutdown_childs(
    //     &mut self,
    //     program_to_remove: Vec<String>,
    //     config_access: &std::sync::RwLockWriteGuard<'_, Config>,
    //     logger: &Logger,
    // ) {
    //     // here we kill child that are not in the config anymore
    //     for program in program_to_remove {
    //         match self.shutdown_childs(&program, config_access, logger) {
    //             Ok(_) => {
    //                 // child will shutdown and or be remove in next iteration of this function
    //             }
    //             Err(error) => match error {
    //                 KillingChildError::NoProgramFound => unreachable!(),
    //                 KillingChildError::CantKillProcess => {
    //                     log_error!(logger, "Can't kill a child of process: {program}: {error}");
    //                 }
    //             },
    //         }
    //     }
    // }

    /// this function spawn a thread the will monitor all process launch in self, refreshing every refresh_period
    pub(super) async fn monitor(
        shared_process_manager: SharedProcessManager,
        shared_config: SharedConfig,
        shared_logger: SharedLogger,
        refresh_period: Duration,
    ) -> Result<JoinHandle<()>, std::io::Error> {
        thread::Builder::new().spawn(move || loop {
            {
                log_debug!(shared_logger, "about to lock manager");
                // shared_process_manager
                //     .write()
                //     .expect("the lock has been poisoned")
                //     .monitor_once(&shared_config, &shared_logger);
            }

            thread::sleep(refresh_period);
        })
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

/* -------------------------------------------------------------------------- */
/*                                    Error                                   */
/* -------------------------------------------------------------------------- */
#[derive(Debug)]
pub(super) enum KillingChildError {
    NoProgramFound,
    CantKillProcess,
}

impl Error for KillingChildError {}

impl Display for KillingChildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<std::io::Error> for KillingChildError {
    fn from(_: std::io::Error) -> Self {
        KillingChildError::CantKillProcess
    }
}
