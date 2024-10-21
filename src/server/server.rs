/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use crate::process_manager::SharedProcessManager;
use config::{Config, SharedConfig};
use logger::{new_shared_logger, SharedLogger};
use process_manager::new_shared_process_manager;
use tcl::message::{receive, Request};
use tokio::net::{TcpListener, TcpStream};
/* -------------------------------------------------------------------------- */
/*                                   Module                                   */
/* -------------------------------------------------------------------------- */
mod config;
mod logger;
mod process_manager;

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
                Ok(message) => match message {
                    R::Status => {
                        log_info!(shared_logger, "Status Request gotten");
                    }
                    R::Start(name) => {
                        log_info!(shared_logger, "Start Request gotten");
                        ClientHandler::handle_start(
                            name,
                            shared_logger.clone(),
                            shared_config.clone(),
                            shared_process_manager.clone(),
                        )
                        .await;
                    }
                    R::Stop(name) => {
                        log_info!(shared_logger, "Stop Request gotten");
                        ClientHandler::handle_stop(
                            name,
                            shared_logger.clone(),
                            shared_config.clone(),
                            shared_process_manager.clone(),
                        )
                        .await;
                    }
                    R::Restart(name) => {
                        log_info!(shared_logger, "Restart Request gotten");
                        ClientHandler::handle_stop(
                            name.clone(),
                            shared_logger.clone(),
                            shared_config.clone(),
                            shared_process_manager.clone(),
                        )
                        .await;
                        ClientHandler::handle_start(
                            name,
                            shared_logger.clone(),
                            shared_config.clone(),
                            shared_process_manager.clone(),
                        )
                        .await;
                    }
                    R::Reload => {
                        log_info!(shared_logger, "Reload Request gotten");
                        ClientHandler::handle_reload(shared_logger.clone(), shared_config.clone())
                            .await;
                    }
                },
                Err(error) => {
                    // if the error occurred because the client disconnected then the task of this thread is finished
                    if error.client_disconnected() {
                        log_info!(shared_logger, "Client Disconnected");
                        return;
                    } else {
                        log_error!(shared_logger, "{error}");
                    }
                }
            }
        }
    }

    async fn handle_reload(shared_logger: SharedLogger, shared_config: SharedConfig) {
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
            }
            Err(error) => {
                // TODO send the error back to the client saying something like the config was not able to be reloaded due to : error; for it to display
                log_error!(
                    shared_logger,
                    "The config file could not be reloaded, due to {error}"
                );
            }
        }
    }

    async fn handle_start(
        name: String,
        shared_logger: SharedLogger,
        shared_config: SharedConfig,
        shared_process_manager: SharedProcessManager,
    ) {
        let mut manager = shared_process_manager
            .write()
            .expect("One of the holder of this lock panicked");
        match shared_config.read().expect("One of the holder of this lock panicked").programs.get(&name) {
            Some(config) => {
               manager.spawn_program(&name, &config); 
                // TODO: Implement response ACK
            },
            None => {
                log_error!(shared_logger, "No program named '{}' found", name);
            }
        }
    }

    async fn handle_stop(
        name: String,
        shared_logger: SharedLogger,
        shared_config: SharedConfig,
        shared_process_manager: SharedProcessManager,
    ) {
        let mut manager = shared_process_manager
            .write()
            .expect("One of the holder of this lock panicked");
        match manager.kill_childs(&name, shared_config.clone()) {
            Ok(()) => {
                // TODO: Implement response ACK
            }
            Err(error) => {
                log_error!(shared_logger, "Failed to kill child process: {error}");
            }
        }
    }
}
