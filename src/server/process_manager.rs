/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use crate::{
    config::{Config, ProgramConfig, SharedConfig},
    logger::{Logger},
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
    handle: Child,
    started_since: SystemTime, // to clarify
}

pub(super) type SharedProcessManager = Arc<RwLock<ProcessManager>>;

/* -------------------------------------------------------------------------- */
/*                            Struct Implementation                           */
/* -------------------------------------------------------------------------- */
// these are more of a place holder than anything
impl ProcessManager {
    /// return a new ProcessManager
    fn new_from_config(config: &RwLock<Config>, logger: &Logger) -> Self {
        let mut process_manager = ProcessManager {
            children: Default::default(),
        };
        config
            .read()
            .unwrap()
            .programs
            .iter()
            .filter(|(_, program_config)| program_config.start_at_launch)
            .for_each(|(program_name, program_config)| {
                process_manager.spawn_program(program_name, program_config);
            });
        process_manager
    }

    pub fn spawn_program(&mut self, program_name: &str, program_config: &ProgramConfig) {
        for _process_number in 0..program_config.number_of_process {
            if let Err(error) = self.spawn_child(program_config, &program_name) {
                // log_error!(logger, "error happened while spawning a process of the program : {program_name}: {error}");
                // todo!(); // w'll see depending on what error could happen in the spawn command
            }
        }
    }

    /// return a the handle to the process child has a mutable reference
    fn get_child(&mut self, name: &str) -> Option<&mut Child> {
        todo!()
    }

    /// kill a given child
    pub fn kill_childs(
        &mut self,
        name: &str,
        shared_config: SharedConfig,
    ) -> Result<(), std::io::Error> {
        match self.children.get_mut(name) {
            Some(processes) => {
                for process in processes.iter_mut() {
                    println!("killing process for {}", process.handle.id());
                    process.handle.kill()?;
                }
                processes.clear();
            }
            None => {
                todo!() // Handle error
            }
        }
        Ok(())
    }

    /// this function must spawn a child given the argument in the config, it's definition will probably need to change as we take more thing into consideration
    fn spawn_child(
        &mut self,
        program_config: &ProgramConfig,
        name: &str,
    ) -> Result<(), std::io::Error> {
        let split_command: Vec<&str> = program_config.command.split(' ').collect();
        if split_command.len() > 1 {
            let child = Command::new(split_command.first().expect("Empty command"))
                .args(&split_command[1..])
                .spawn()?;
            let process = RunningProcess {
                handle: child,
                started_since: SystemTime::now(),
            };
            self.children
                .entry(name.to_string())
                .or_default()
                .push(process);
        }
        Ok(())
    }

    /// do one round of monitoring
    fn monitor_once(&mut self, shared_config: SharedConfig) {
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
                    .monitor_once(shared_config.clone());
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
