/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use config::{Config, SharedConfig};
use logger::{new_shared_logger, SharedLogger};
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
                tokio::spawn(handle_client(
                    socket,
                    shared_logger.clone(),
                    shared_config.clone(),
                ));
                log_info!(shared_logger, "Client Accepted");
            }
            Err(error) => {
                log_error!(shared_logger, "{}", format!("Accepting Client: {error}"));
            }
        }
    }
}

/// do the actual match of the client request
async fn handle_client(
    mut socket: TcpStream,
    shared_logger: SharedLogger,
    shared_config: SharedConfig,
) {
    use Request as R;
    loop {
        match receive::<Request>(&mut socket).await {
            Ok(message) => match message {
                R::Status => {
                    log_info!(shared_logger, "Status Request gotten");
                }
                R::Start(_) => {
                    log_info!(shared_logger, "Start Request gotten");
                }
                R::Stop(_) => {
                    log_info!(shared_logger, "Stop Request gotten");
                }
                R::Restart(_) => {
                    log_info!(shared_logger, "Restart Request gotten");
                }
                R::Reload => {
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
