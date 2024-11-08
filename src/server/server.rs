/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

#[cfg(feature = "better_log")]
use better_logs::send_notification;
use client_handler::ClientHandler;
use logger::{new_shared_logger, SharedLogger};
use process_manager::{manager::new_shared_process_manager, ProgramManager, SharedProcessManager};
use std::{
    thread::{sleep, JoinHandle},
    time::Duration,
};
use tokio::net::TcpListener;

/* -------------------------------------------------------------------------- */
/*                                   Module                                   */
/* -------------------------------------------------------------------------- */
mod better_logs;
mod client_handler;
mod config;
mod logger;
pub mod process_manager;

/* -------------------------------------------------------------------------- */
/*                                    Main                                    */
/* -------------------------------------------------------------------------- */
#[tokio::main]
async fn main() {
    // test manuel sans utiliser reqwest
    // let access_token = "o.8bTjZDwKe70TfNqzjMm42HfCcSYkFqA5".to_string();
    // let title = "Test Notification".to_string();
    // let body = "This is a test notification from Rust!".to_string();

    // send_notification(access_token, title, body);

    // create a logger instance
    let shared_logger = new_shared_logger().expect("Can't create the logger");
    log_info!(shared_logger, "Starting a new server instance");

    // load the config
    let shared_config = config::new_shared_config()
        .expect("please provide a file named 'config.yaml' at the root of this rust project");
    log_info!(shared_logger, "Loading Config: {shared_config:?}");

    // launch the process manager
    let shared_process_manager = new_shared_process_manager(&shared_config.read().unwrap());
    log_info!(shared_logger, "Process Manager created");
    log_debug!(shared_logger, "{shared_process_manager:?}");

    // start the listener
    log_info!(shared_logger, "Starting Taskmaster Daemon");
    let listener = TcpListener::bind(tcl::SOCKET_ADDRESS)
        .await
        .expect("Failed to bind tcp listener");

    // start the process monitoring
    let _monitoring_handle =
        start_monitor(shared_process_manager.clone(), shared_logger.clone()).await; // in case we need it

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
    shared_logger: SharedLogger,
) -> JoinHandle<()> {
    loop {
        match ProgramManager::monitor(
            shared_process_manager.clone(),
            shared_logger.clone(),
            Duration::from_secs(1),
        )
        .await
        {
            Ok(handle) => {
                log_info!(shared_logger, "the monitoring loop is on");
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
