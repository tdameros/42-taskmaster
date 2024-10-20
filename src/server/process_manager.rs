/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use crate::config::SharedConfig;
use std::{
    collections::HashMap,
    process::Child,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

/* -------------------------------------------------------------------------- */
/*                                   Struct                                   */
/* -------------------------------------------------------------------------- */
/// this represent the running process
#[derive(Debug, Default)]
pub(super) struct ProcessManager {
    // we may have to move this into the library if we choose to use this struct as a base for the status command
    children: HashMap<String, Child>,
}

pub(super) type SharedProcessManager = Arc<RwLock<ProcessManager>>;

/* -------------------------------------------------------------------------- */
/*                            Struct Implementation                           */
/* -------------------------------------------------------------------------- */
// these are more of a place holder than anything
impl ProcessManager {
    /// return a new ProcessManager
    fn new() -> Self {
        todo!()
    }

    /// return a the handle to the process child has a mutable reference
    fn get_child(&mut self, name: &str) -> Option<&mut Child> {
        todo!()
    }

    /// kill a given child
    fn kill_child(
        &mut self,
        name: &str,
        shared_config: SharedConfig,
    ) -> Result<(), std::io::Error> {
        todo!()
    }

    /// this function must spawn a child given the argument in the config, it's definition will probably need to change as we take more thing into consideration
    fn spawn_child(
        &mut self,
        shared_config: SharedConfig,
        name: &str,
    ) -> Result<(), std::io::Error> {
        todo!()
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

pub(super) fn new_shared_process_manager() -> SharedProcessManager {
    Arc::new(RwLock::new(ProcessManager::new()))
}
