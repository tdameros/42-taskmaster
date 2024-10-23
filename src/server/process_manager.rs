/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use crate::{
    config::{Config, ProgramConfig, SharedConfig},
    log_error, log_info,
    logger::{Logger, SharedLogger},
};
use std::{
    collections::HashMap,
    process::{Child, Command},
    sync::{Arc, RwLock},
    thread,
    time::{Duration, SystemTime},
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

#[derive(Debug)]
struct RunningProcess {
    // the handle to the process
    handle: Child,

    // the time when the process was launched
    started_since: SystemTime, // to clarify

    // use to determine when to abort the child
    time_since_killed: Option<SystemTime>,
}

/// a sharable version of a process manager, it can be passe through thread safely + use in a concurrent environment without fear thank Rust !
pub(super) type SharedProcessManager = Arc<RwLock<ProcessManager>>;

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
    pub fn kill_childs(
        &mut self,
        name: &str,
        config: &RwLock<Config>, // TODO use it
        logger: &Logger,
    ) -> Result<(), std::io::Error> {
        match self.children.get_mut(name) {
            // if the given program have running process we iterate over them and send to them the correct signal
            Some(processes) => {
                for process in processes.iter_mut() {
                    log_info!(
                        logger,
                        "about to kill process number : {}",
                        process.handle.id()
                    );
                    // TODO use the config to send the correct signal to kill the process
                    process.handle.kill()?;

                    // use to prevent a refreshing of the time to wait before aborting the child
                    // in the case of a repeated order to stop said child (AKA spamming the stop command)
                    if process.time_since_killed.is_none() {
                        process.time_since_killed = Some(SystemTime::now());
                    }
                }
                processes.clear();
                log_info!(
                    logger,
                    "all process for the program : {name} has been killed"
                );
            }
            // if no process are found for a given name we do nothing
            None => {}
        }
        // remove the program from the hashmap key of running program since all of it's child where killed
        self.children.remove_entry(name); // TODO check this behavior once we introduce the delay before killing by which i mean sending a signal to the child because now the child die instant if we have the privilege

        Ok(())
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
            let process = RunningProcess {
                handle: child,
                started_since: SystemTime::now(),
                time_since_killed: None,
            };

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
        let mut program_name_to_kill = Vec::new();

        // iterate over all process
        self.children
            .iter_mut()
            .for_each(|(program_name, vec_running_process)| {
                // check if the process name we are on is in the new config
                match config_access.programs.get(program_name) {
                    // the program running is still in the config, so we just need to perform check
                    Some(program_config) => {
                        // check the status of all the child
                        vec_running_process.iter_mut().for_each(|running_process| {
                            if let Some(exit_status) = running_process
                                .handle
                                .try_wait()
                                .expect("error gotten while trying to read a child status")
                            {
                                // if the process is supposed to be dead then we send a SIGKILL to him
                                if running_process.time_since_killed.is_some_and(
                                    |time_since_killed| {
                                        program_config.time_to_stop_gracefully
                                            < SystemTime::now()
                                                .duration_since(time_since_killed)
                                                .unwrap_or_default().as_secs()
                                                .into()
                                    },
                                ) {
                                    let _ = running_process.handle.kill().inspect_err(|error| {log_error!(logger, "Can't kill a child: {error}");});
                                } else if running_process.time_since_killed.is_none() {
                                    // we did'nt killed the child
                                    
                                }
                            } else {

                            }
                        });
                    }
                    // the program running is not in the config anymore so we need to murder his family
                    None => {
                        program_name_to_kill.push(program_name.to_owned());
                    }
                }
            });

        // killing all the program that must die
        for program_name in program_name_to_kill.iter() {
            self.kill_childs(program_name, config, logger);
        }
        // if dead check the exit code and config to see how many time i can restart if dead before launch time
        // else check the auto restart policy
        // check for each program the number of running child if too many kill them else spawn them
        // le coup des changement des redirection stdout et err je ne voie pas comment faire autrement que garder un copie de la config d'avant pour voir si changement et si changement soit on peut changer a la voler soit changer ne coute rien et donc on peut le faire peu importe, soit on ne peut pas changer mais ca m'etonnerais beaucoup beacoup, la question c'est plus esqu'on sait sur quoi le stdout est rediriger la maintenant, si on peu savoir alors on peut check et changer en fonction, si ca ne coute rien on peut passer sur tous et just actualiser

        todo!()
    }

    async fn monitor(shared_process_manager: SharedProcessManager, shared_config: SharedConfig, shared_logger: SharedLogger) {
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
