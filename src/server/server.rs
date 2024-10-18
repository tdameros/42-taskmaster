/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use logger::{new_shared_logger, SharedLogger};
use tcl::message::{receive, Request};
use tokio::net::{TcpListener, TcpStream};

/* -------------------------------------------------------------------------- */
/*                                   Module                                   */
/* -------------------------------------------------------------------------- */
mod config;
mod logger;

/* -------------------------------------------------------------------------- */
/*                                    Main                                    */
/* -------------------------------------------------------------------------- */
#[tokio::main]
async fn main() {
    let shared_logger = new_shared_logger().expect("Can't create the logger");
    log_info!(shared_logger, "Starting a new server instance");

    // load the config
    log_info!(shared_logger, "Starting Taskmaster Daemon");
    let shared_config = config::new_shared_config()
        .expect("please provide a file named 'config.yaml' at the root of this rust project");
    log_info!(
        shared_logger,
        "{}",
        format!("Loading Config: {shared_config:?}")
    );

    // start the listener
    let listener = TcpListener::bind(tcl::SOCKET_ADDRESS)
        .await
        .expect("Failed to bind tcp listener");

    // handle the client connection
    loop {
        log_info!(shared_logger, "Waiting for Client To arrive");
        match listener.accept().await {
            Ok((socket, _)) => {
                tokio::spawn(handle_client(socket, shared_logger.clone()));
                log_info!(shared_logger, "Client Accepted");
            }
            Err(error) => {
                log_error!(shared_logger, "{}", format!("Accepting Client: {error}"));
            }
        }
    }
}

/// do the actual match of the client request
async fn handle_client(mut socket: TcpStream, shared_logger: SharedLogger) {
    loop {
        match receive::<Request>(&mut socket).await {
            Ok(message) => match message {
                Request::Status => {
                    log_info!(shared_logger, "Status Request gotten");
                }
                Request::Start(_) => {
                    log_info!(shared_logger, "Start Request gotten");
                }
                Request::Stop(_) => {
                    log_info!(shared_logger, "Stop Request gotten");
                }
                Request::Restart(_) => {
                    log_info!(shared_logger, "Restart Request gotten");
                }
                Request::Reload => {
                    log_info!(shared_logger, "Reload Request gotten");
                }
            },
            Err(error) => {
                // if the error occurred because the client disconnected then the task of this thread is finished
                if error.client_disconnected() {
                    log_error!(shared_logger, "Client Disconnected");
                    break;
                } else {
                    log_error!(shared_logger, "{error}");
                }
            }
        }
    }
}
