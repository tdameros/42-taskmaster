/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */
use crate::config::{Config, SharedConfig};
use client_handler::ClientHandler;
use logger::{new_shared_logger, SharedLogger};
use process_manager::{manager::new_shared_process_manager, ProgramManager, SharedProcessManager};
use tokio::signal::unix::{signal, SignalKind};
use tokio::{net::TcpListener, task::JoinHandle, time::Duration};
/* -------------------------------------------------------------------------- */
/*                                   Module                                   */
/* -------------------------------------------------------------------------- */
mod better_logs;
mod client_handler;
mod config;
mod logger;
pub mod process_manager;
mod ring_buffer;
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
    let shared_process_manager = new_shared_process_manager(&*(shared_config.read().await));
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

    log_info!(shared_logger, "Here");

    start_sighup_monitor(
        shared_process_manager.clone(),
        shared_config.clone(),
        shared_logger.clone(),
    )
    .await;

    // handle the client connection
    loop {
        log_info!(shared_logger, "Waiting for Client To arrive");
        match listener.accept().await {
            Ok((socket, _)) => {
                let shared_logger_clone = shared_logger.clone();
                let shared_config_clone = shared_config.clone();
                let shared_process_manager_clone = shared_process_manager.clone();
                tokio::spawn(async move {
                    ClientHandler::handle_client(
                        socket,
                        shared_logger_clone,
                        shared_config_clone,
                        shared_process_manager_clone,
                    )
                    .await;
                });
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
    ProgramManager::monitor(
        shared_process_manager.clone(),
        shared_logger.clone(),
        Duration::from_secs(1),
    )
    .await
}

async fn start_sighup_monitor(
    shared_process_manager: SharedProcessManager,
    shared_config: SharedConfig,
    shared_logger: SharedLogger,
) {
    let mut signal = signal(SignalKind::hangup()).expect("Failed to bind SIGHUP signal");
    tokio::spawn(async move {
        loop {
            signal.recv().await;
            match Config::load() {
                Ok(config) => {
                    *shared_config.clone().write().await = config;
                    shared_process_manager
                        .clone()
                        .write()
                        .await
                        .reload_config(&(*shared_config.read().await), &shared_logger)
                        .await;
                }
                Err(error) => {
                    eprintln!("Failed to reload config: {error}")
                }
            };
        }
    });
}
