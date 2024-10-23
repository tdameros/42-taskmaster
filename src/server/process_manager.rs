/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use crate::{
    config::{Config, ProgramConfig, SharedConfig}, log_error, log_info, logger::Logger
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
    pub fn spawn_program(&mut self, program_name: &str, program_config: &ProgramConfig, logger: &Logger) {
        // for each process that must be spawn we spawned it using the given argument
        for process_number in 0..program_config.number_of_process {
            // if the child can't be spawn the command is probably wrong or the privilege is not right we cannot do anything, 
            // retrying would be useless sor we just log the error and go to the next which will probably fail too
            if let Err(error) = self.spawn_child(program_config, program_name) {
                log_error!(logger, "{}", format!("Fatal couldn't spawn child : {process_number} because of : {error}"));
            }
        }
    }

    /// kill all child of a given process if no child exist for the given program name the function does nothing
    /// if the kill command failed (probably due to insufficient privilege) it's error is return
    pub fn kill_childs(
        &mut self,
        name: &str,
        shared_config: SharedConfig, // TODO use it
        logger: &Logger,
    ) -> Result<(), std::io::Error> {
        match self.children.get_mut(name) {
            // if the given program have running process we iterate over them and send to them the correct signal
            Some(processes) => {
                for process in processes.iter_mut() {
                    log_info!(logger, "about to kill process number : {}", process.handle.id());
                    // TODO use the config to send the correct signal to kill the process
                    process.handle.kill()?;

                    // use to prevent a refreshing of the time to wait before aborting the child
                    // in the case of a repeated order to stop said child (AKA spamming the stop command)
                    if process.time_since_killed.is_none() {
                        process.time_since_killed = Some(SystemTime::now());
                    }
                }
                processes.clear();
                log_info!(logger, "all process for the program : {name} has been killed");
            }
            // if no process are found for a given name we do nothing
            None => {}
        }

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
    fn monitor_once(&mut self, config: &RwLock<Config>) {
        // check the status of all the child
        // query the new config
        // check what need to be changed based on the new config

        todo!()
    }

    async fn monitor(shared_process_manager: SharedProcessManager, shared_config: SharedConfig) {
        thread::spawn(move || loop {
            {
                shared_process_manager
                    .write()
                    .expect("the lock has been poisoned")
                    .monitor_once(&shared_config);
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
