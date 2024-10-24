/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use crate::{
    config::{Config, ProgramConfig, SharedConfig},
    log_error, log_info,
    logger::{Logger, SharedLogger},
    running_process::RunningProcess,
};
use std::{
    collections::HashMap, error::Error, fmt::Display, process::Command, sync::{Arc, RwLock}, thread, time::{Duration, SystemTime}
};

/* -------------------------------------------------------------------------- */
/*                                   Struct                                   */
/* -------------------------------------------------------------------------- */
/// this represent the running process
#[derive(Debug)]
pub(super) struct ProcessManager {
    // we may have to move this into the library if we choose to use this struct as a base for the status command
    children: HashMap<String, Vec<RunningProcess>>,
}

/// a sharable version of a process manager, it can be passe through thread safely + use in a concurrent environment without fear thank Rust !
pub(super) type SharedProcessManager = Arc<RwLock<ProcessManager>>;

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

/* -------------------------------------------------------------------------- */
/*                            Struct Implementation                           */
/* -------------------------------------------------------------------------- */
impl ProcessManager {
    /// return a new ProcessManager
    fn new_from_config(config: &RwLock<Config>, logger: &Logger) -> Self {
        let mut process_manager = ProcessManager {
            children: Default::default(),
        };

        // here we iterate over the config keeping only the program that must be start at launch and then calling a function that will start each of them correctly
        config
            .read()
            .unwrap()
            .programs
            .iter()
            .filter(|(_, program_config)| program_config.start_at_launch)
            .for_each(|(program_name, program_config)| {
                process_manager.spawn_program(program_name, program_config, logger);
            });

        // we then return an instance of the process manager class filled with the running process handle
        process_manager
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
            // retrying would be useless sor we just log the error and go to the next which will probably fail too
            if let Err(error) = self.spawn_child(program_config, program_name) {
                log_error!(
                    logger,
                    "{}",
                    format!("Fatal couldn't spawn child : {process_number} because of : {error}")
                );
            }
        }
    }

    /// kill all child of a given process if no child exist for the given program name the function does nothing
    /// if the kill command failed (probably due to insufficient privilege) it's error is return
    pub(super) fn kill_childs(
        &mut self,
        name: &str,
        config: &RwLock<Config>, // TODO use it
        logger: &Logger,
    ) -> Result<(), KillingChildError> {
        // lock the config for immutable access
        let config_access = config.read().expect("One owner of this lock panicked");

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
                match config_access.programs.get(name) {
                    Some(config) => {
                        // here we keep the process just sending them the signal required by the config
                        processes.iter_mut().for_each(|process| {
                            log_info!(
                                logger,
                                "about to gracefully shutdown process number : {}",
                                process.get_child_id()
                            );
                            process.send_signal(&config.stop_signal);
                        });
                        Ok(())
                    },
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
                    },
                }
            }
            // if no process are found for a given name we return the corresponding error
            None => {
                log_info!(logger, "tried to remove child of process: {name} but none where found");
                Err(KillingChildError::NoProgramFound)
            }
        }
        // here we don't remove the entry since the monitor will do it for use
    }

    /// this function spawn a child given the program config for a program it then insert a newly created
    /// running process struct into the vec of running process for the given program name in self,
    /// creating it if it doesn't exist yet
    fn spawn_child(
        &mut self,
        program_config: &ProgramConfig,
        name: &str,
    ) -> Result<(), std::io::Error> {
        let split_command: Vec<&str> = program_config.command.split_whitespace().collect();

        if split_command.len() > 0 {
            //create the command using the command property given by the program config
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
            let process = RunningProcess::new(child);

            // insert the running process newly created to self at the end of the vector of running process for the given program name entry, creating a new empty vector if none where found
            self.children
                .entry(name.to_string())
                .or_default()
                .push(process);
        }

        Ok(())
    }

    /// do one round of monitoring
    fn monitor_once(&mut self, config: &RwLock<Config>, logger: &Logger) {
        // query the new config
        let config_access = config
            .read()
            .expect("Some user of the config lock has panicked");

        let mut program_to_remove = Vec::new();
        
        // iterate over all process
        self.children
            .iter_mut()
            .for_each(|(program_name, vec_running_process)| {
                // check if the process name we are on is in the new config
                match config_access.programs.get(program_name) {
                    // the program running is still in the config, so we just need to perform check
                    Some(program_config) => {
                        // keep only the good child AKA healthy child
                        vec_running_process.retain_mut(|running_process| {
                            match running_process.get_exit_code() {
                                Err(error) => {
                                    log_error!(
                                        logger,
                                        "error gotten while trying to read a child status"
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
                                    if running_process.program_was_running(program_config)
                                        && program_config.max_number_of_restart > 0
                                    {
                                        // we decrement the number of allowed restart for next time
                                        program_config.max_number_of_restart -= 1;

                                        
                                        // then we increment the number of time that this program should be restarted
                                        match process_id_to_restart.get_mut(program_name.as_str()) {
                                            Some(number) => *number += 1,
                                            None => {
                                                process_id_to_restart
                                                    .insert(program_name.as_str(), 1);
                                            }
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
                }
            });

        for program in program_to_remove
        // clean dead process from self
        // self.remove_dead_process(logger);
        // killing all the program that must die
        // check for each program the number of running child if too many kill them else spawn them
        // le coup des changement des redirection stdout et err je ne voie pas comment faire autrement que garder un copie de la config d'avant pour voir si changement et si changement soit on peut changer a la voler soit changer ne coute rien et donc on peut le faire peu importe, soit on ne peut pas changer mais ca m'etonnerais beaucoup beacoup, la question c'est plus esqu'on sait sur quoi le stdout est rediriger la maintenant, si on peu savoir alors on peut check et changer en fonction, si ca ne coute rien on peut passer sur tous et just actualiser

        todo!()
    }

    async fn monitor(
        shared_process_manager: SharedProcessManager,
        shared_config: SharedConfig,
        shared_logger: SharedLogger,
    ) {
        thread::spawn(move || loop {
            {
                shared_process_manager
                    .write()
                    .expect("the lock has been poisoned")
                    .monitor_once(&shared_config, &shared_logger);
            }

            thread::sleep(Duration::from_secs(1));
        });
    }
}

pub(super) fn new_shared_process_manager(
    config: &RwLock<Config>,
    logger: &Logger,
) -> SharedProcessManager {
    Arc::new(RwLock::new(ProcessManager::new_from_config(config, logger)))
}
