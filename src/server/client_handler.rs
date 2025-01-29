/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */
use crate::{
    config::{Config, SharedConfig},
    log_error, log_info,
    logger::SharedLogger,
    process_manager::SharedProcessManager,
};
use std::sync::Arc;
use tcl::message::{
    receive_with_shared_tcp_stream, send_with_shared_tcp_stream, Request, Response,
};
use tokio::{
    io::{split, WriteHalf},
    net::TcpStream,
    sync::Mutex,
    time::{sleep, Duration},
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
        socket: TcpStream,
        shared_logger: SharedLogger,
        shared_config: SharedConfig,
        shared_process_manager: SharedProcessManager,
    ) {
        use Request as R;
        let (reader, writer) = split(socket);
        let shared_writer = Arc::new(Mutex::new(writer));
        let shared_reader = Arc::new(Mutex::new(reader));
        loop {
            match receive_with_shared_tcp_stream::<Request>(shared_reader.clone()).await {
                Ok(message) => {
                    let response = match message {
                        R::Status => {
                            log_info!(shared_logger, "Status Request gotten");
                            shared_process_manager.write().await.get_status()
                        }
                        R::Start(name) => {
                            log_info!(shared_logger, "Start Request gotten");
                            shared_process_manager
                                .write()
                                .await
                                .start_program(&name, &shared_logger)
                                .await
                        }
                        R::Stop(name) => {
                            log_info!(shared_logger, "Stop Request gotten");
                            shared_process_manager
                                .write()
                                .await
                                .stop_program(&name, &shared_logger)
                                .await
                        }
                        R::Restart(name) => {
                            log_info!(shared_logger, "Restart Request gotten");
                            shared_process_manager
                                .write()
                                .await
                                .restart_program(&name, &shared_logger)
                                .await
                        }
                        R::Reload => {
                            log_info!(shared_logger, "Reload Request gotten");
                            match Config::load() {
                                Ok(config) => {
                                    *shared_config.write().await = config;
                                    shared_process_manager
                                        .write()
                                        .await
                                        .reload_config(
                                            &(*shared_config.read().await),
                                            &shared_logger,
                                        )
                                        .await;
                                    Response::Success("Config Reload Successful".to_owned())
                                }
                                Err(e) => Response::Error(e.to_string()),
                            }
                        }
                        R::Attach(name) => {
                            attach(
                                name,
                                shared_process_manager.clone(),
                                shared_writer.clone(),
                                shared_logger.clone(),
                            )
                            .await
                        }
                    };
                    if let Err(error) =
                        send_with_shared_tcp_stream(shared_writer.clone(), &response).await
                    {
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

async fn attach(
    name: String,
    shared_process_manager: SharedProcessManager,
    shared_writer: Arc<Mutex<WriteHalf<TcpStream>>>,
    shared_logger: SharedLogger,
) -> Response {
    let broadcast = shared_process_manager.write().await.subscribe(&name).await;
    if let Some(mut broadcast) = broadcast {
        tokio::spawn(async move {
            loop {
                let e = broadcast.recv().await.unwrap();
                let response = Response::RawStream(e);
                if let Err(error) =
                    send_with_shared_tcp_stream(shared_writer.clone(), &response).await
                {
                    log_error!(shared_logger, "{}", error);
                }
                sleep(Duration::from_secs(1)).await;
            }
        });
        Response::Success("Attach Successful".to_owned())
    } else {
        Response::Error("Program not found".to_owned())
    }
}
