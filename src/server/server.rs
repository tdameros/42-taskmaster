/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use std::{
    thread::{sleep, JoinHandle},
    time::Duration,
};

use crate::process_manager::SharedProcessManager;
use config::{Config, SharedConfig};
use logger::{new_shared_logger, SharedLogger};
use process_manager::{new_shared_process_manager, ProcessManager};
use tcl::message::{receive, send, Request, Response};
use tokio::net::{TcpListener, TcpStream};

/* -------------------------------------------------------------------------- */
/*                                   Module                                   */
/* -------------------------------------------------------------------------- */
mod config;
mod logger;
mod process_manager;
mod running_process;

/* -------------------------------------------------------------------------- */
/*                                    Main                                    */
/* -------------------------------------------------------------------------- */
#[tokio::main]
async fn main() {
    // create a logger instance
    let shared_logger = new_shared_logger().expect("Can't create the logger");
    log_info!(shared_logger, "Starting a new server instance");

    // load the config
    let shared_config = config::new_shared_config()
        .expect("please provide a file named 'config.yaml' at the root of this rust project");
    log_info!(shared_logger, "Loading Config: {shared_config:?}");

    // launch the process manager
    let shared_process_manager = new_shared_process_manager(&shared_config, &shared_logger);

    // start the listener
    log_info!(shared_logger, "Starting Taskmaster Daemon");
    let listener = TcpListener::bind(tcl::SOCKET_ADDRESS)
        .await
        .expect("Failed to bind tcp listener");

    // start the process monitoring
    let _monitoring_handle = start_monitor(
        shared_process_manager.clone(),
        shared_config.clone(),
        shared_logger.clone(),
    ); // in case we need it

    // handle the client connection
    loop {
        log_info!(shared_logger, "Waiting for Client To arrive");
        match listener.accept().await {
            Ok((socket, _)) => {
                tokio::spawn(ClientHandler::handle_client(
                    socket,
                    shared_logger.clone(),
                    shared_config.clone(),
                    shared_process_manager.clone(),
                ));
                log_info!(shared_logger, "Client Accepted");
            }
            Err(error) => {
                log_error!(shared_logger, "{}", format!("Accepting Client: {error}"));
            }
        }
    }
}

async fn start_monitor(
    shared_process_manager: SharedProcessManager,
    shared_config: SharedConfig,
    shared_logger: SharedLogger,
) -> JoinHandle<()> {
    loop {
        match ProcessManager::monitor(
            shared_process_manager.clone(),
            shared_config.clone(),
            shared_logger.clone(),
            Duration::from_secs(1),
        )
        .await
        {
            Ok(handle) => {
                return handle;
            }
            Err(error) => {
                log_error!(
                    shared_logger,
                    "Can't spawn monitoring thread: {error}, retrying in 5 second"
                );
                sleep(Duration::from_secs(5));
            }
        }
    }
}

struct ClientHandler {}

impl ClientHandler {
    /// do the actual match of the client request
    async fn handle_client(
        mut socket: TcpStream,
        shared_logger: SharedLogger,
        shared_config: SharedConfig,
        shared_process_manager: SharedProcessManager,
    ) {
        use Request as R;
        loop {
            match receive::<Request>(&mut socket).await {
                Ok(message) => {
                    let response = match message {
                        R::Status => {
                            log_info!(shared_logger, "Status Request gotten");
                            Response::Status(
                                shared_process_manager
                                    .write()
                                    .expect("Can't acquire process manager")
                                    .get_processes_state(),
                            )
                        }
                        R::Start(name) => {
                            log_info!(shared_logger, "Start Request gotten");
                            ClientHandler::handle_start(
                                name,
                                shared_logger.clone(),
                                shared_config.clone(),
                                shared_process_manager.clone(),
                            )
                            .await
                        }
                        R::Stop(name) => {
                            log_info!(shared_logger, "Stop Request gotten");
                            ClientHandler::handle_stop(
                                name,
                                shared_logger.clone(),
                                shared_config.clone(),
                                shared_process_manager.clone(),
                            )
                            .await
                        }
                        R::Restart(name) => {
                            log_info!(shared_logger, "Restart Request gotten");
                            ClientHandler::handle_restart(
                                name,
                                shared_logger.clone(),
                                shared_config.clone(),
                                shared_process_manager.clone(),
                            )
                            .await
                        }
                        R::Reload => {
                            log_info!(shared_logger, "Reload Request gotten");
                            ClientHandler::handle_reload(
                                shared_logger.clone(),
                                shared_config.clone(),
                            )
                            .await
                        }
                    };
                    if let Err(error) = send(&mut socket, &response).await {
                        log_error!(shared_logger, "{}", error);
                    }
                }
                Err(error) => {
                    // if the error occurred because the client disconnected then the task of this thread is finished
                    if error.client_disconnected() {
                        log_info!(shared_logger, "Client Disconnected");
                        return;
                    } else {
                        log_error!(shared_logger, "{error}");
                    }
                }
            };
        }
    }

    async fn handle_restart(
        name: String,
        shared_logger: SharedLogger,
        shared_config: SharedConfig,
        shared_process_manager: SharedProcessManager,
    ) -> Response {
        let mut response = Self::handle_stop(
            name.clone(),
            shared_logger.clone(),
            shared_config.clone(),
            shared_process_manager.clone(),
        )
        .await;
        if let Response::Error(error) = response {
            return Response::Error(error.to_string());
        }
        response = Self::handle_start(
            name.clone(),
            shared_logger,
            shared_config,
            shared_process_manager,
        )
        .await;
        if let Response::Error(error) = response {
            return Response::Error(error.to_string());
        }
        Response::Success(format!("`{name}` restarted successfully"))
    }

    async fn handle_reload(shared_logger: SharedLogger, shared_config: SharedConfig) -> Response {
        log_info!(shared_logger, "Reload Request gotten");
        match Config::load() {
            Ok(new_config) => {
                *shared_config
                    .write()
                    .expect("One of the holder of this lock panicked") = new_config;
                log_info!(
                    shared_logger,
                    "The config has been reloaded: {shared_config:?}"
                );
                Response::Success("Configuration reloaded successfully".to_string())
            }
            Err(error) => {
                // TODO send the error back to the client saying something like the config was not able to be reloaded due to : error; for it to display
                log_error!(
                    shared_logger,
                    "The config file could not be reloaded, due to {error}"
                );
                Response::Error(format!("Configuration could not be reloaded ({error})"))
            }
        }
    }

    async fn handle_start(
        name: String,
        shared_logger: SharedLogger,
        shared_config: SharedConfig,
        shared_process_manager: SharedProcessManager,
    ) -> Response {
        let mut manager = shared_process_manager
            .write()
            .expect("One of the holder of this lock panicked");
        match shared_config
            .read()
            .expect("One of the holder of this lock panicked")
            .programs
            .get(&name)
        {
            Some(config) => {
                manager.spawn_program(&name, config, &shared_logger);
                Response::Success(format!("`{name}` has been start with successful"))
            }
            None => {
                log_error!(shared_logger, "No program named '{}' found", name);
                Response::Error(format!("`{name}` could not be started (program not found)"))
            }
        }
    }

    async fn handle_stop(
        name: String,
        shared_logger: SharedLogger,
        shared_config: SharedConfig,
        shared_process_manager: SharedProcessManager,
    ) -> Response {
        let mut manager = shared_process_manager
            .write()
            .expect("One of the holder of this lock panicked");
        let read_config = shared_config.read().expect("lock has been poison");
        match manager.shutdown_childs(&name, &read_config, &shared_logger) {
            Ok(()) => Response::Success(format!("`{name}` has been stop with successful")),
            Err(error) => {
                log_error!(shared_logger, "Failed to kill child process: {error}");
                Response::Error(format!("`{name}` could not be stopped ({error})"))
            }
        }
    }
}
