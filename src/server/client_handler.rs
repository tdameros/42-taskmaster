/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use tcl::message::{receive, send, Request, Response};
use tokio::net::TcpStream;

use crate::{
    config::{Config, SharedConfig},
    log_error, log_info,
    logger::SharedLogger,
    process_manager::SharedProcessManager,
};

/* -------------------------------------------------------------------------- */
/*                                   Struct                                   */
/* -------------------------------------------------------------------------- */
pub(super) struct ClientHandler {}

/* -------------------------------------------------------------------------- */
/*                               Implementation                               */
/* -------------------------------------------------------------------------- */
impl ClientHandler {
    /// do the actual match of the client request
    pub(super) async fn handle_client(
        mut socket: TcpStream,
        shared_logger: SharedLogger,
        shared_config: SharedConfig,
        shared_process_manager: SharedProcessManager,
    ) {
        use Request as R;
        loop {
            match receive::<Request>(&mut socket).await {
                Ok(message) => {
                    println!("{:#?}", shared_process_manager.read().unwrap());
                    let response = match message {
                        R::Status => {
                            log_info!(shared_logger, "Status Request gotten");
                            shared_process_manager
                                .write()
                                .expect("Can't acquire process manager")
                                .get_status()
                        }
                        R::Start(name) => {
                            log_info!(shared_logger, "Start Request gotten");
                            shared_process_manager
                                .write()
                                .unwrap()
                                .start_program(&name, &shared_logger)
                        }
                        R::Stop(name) => {
                            log_info!(shared_logger, "Stop Request gotten");
                            shared_process_manager
                                .write()
                                .unwrap()
                                .stop_program(&name, &shared_logger)
                        }
                        R::Restart(name) => {
                            log_info!(shared_logger, "Restart Request gotten");
                            shared_process_manager
                                .write()
                                .unwrap()
                                .restart_program(&name, &shared_logger)
                        }
                        R::Reload => {
                            log_info!(shared_logger, "Reload Request gotten");
                            match Config::load() {
                                Ok(config) => {
                                    *shared_config.write().unwrap() = config;
                                    tcl::mylibc::kill(tcl::mylibc::getppid(), tcl::mylibc::SIGHUP).expect("should not be possible");
                                    shared_process_manager.write().unwrap().reload_config(
                                        &shared_config.read().unwrap(),
                                        &shared_logger,
                                    );
                                    Response::Success("Config Reload Successful".to_owned())
                                }
                                Err(e) => Response::Error(e.to_string()),
                            }
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
}
