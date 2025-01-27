/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */
use crate::ring_buffer::RingBuffer;
use crate::{
    config::{Config, SharedConfig},
    log_error, log_info,
    logger::SharedLogger,
    process_manager::SharedProcessManager,
};
use std::sync::Arc;
use libc::printf;
use tcl::message::{
    receive_with_shared_tcp_stream, send_with_shared_tcp_stream, Request, Response,
};
use tokio::{
    io::{split, WriteHalf},
    net::TcpStream,
    sync::Mutex,
};
use tokio::task::JoinHandle;
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
        let mut is_attach = false;
        let mut task: Option<JoinHandle<()>> = None;
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
                            if is_attach {
                                log_info!(shared_logger, "Already attached");
                                return;
                            }
                        let _task = attach(
                                name,
                                shared_process_manager.clone(),
                                shared_writer.clone(),
                                shared_logger.clone(),
                            )
                            .await;
                            match _task {
                                Some(_task) => {
                                    task = Some(_task);
                                    Response::Success("Attach Successful".to_owned())
                                }
                                None => Response::Error("Program not found".to_owned()),
                            }
                        }
                        R::Detach => {
                            is_attach = false;
                            if let Some(ref my_task) = task {
                                my_task.abort();
                                println!("Task Aborted");
                                println!("Task Status: {:?}", my_task.is_finished());
                            }
                            Response::Success("Detach Successful".to_owned())
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
) -> Option<JoinHandle<()>> {
    let broadcast = shared_process_manager.write().await.subscribe(&name).await;
    let history = shared_process_manager
        .write()
        .await
        .get_history(&name)
        .await;
    if let (Some(broadcast), Some(history)) = (broadcast, history) {
        Some(tokio::spawn(transfer_stdout(
            broadcast,
            history,
            shared_writer,
            shared_logger,
        )))
        // (Response::Success("Attach Successful".to_owned()), Some(task))
    } else {
        None
            // (Response::Error("Program not found".to_owned()), None)
    }
}

async fn transfer_stdout(
    mut broadcast: tokio::sync::broadcast::Receiver<String>,
    history: RingBuffer<String>,
    shared_writer: Arc<Mutex<WriteHalf<TcpStream>>>,
    shared_logger: SharedLogger,
) {
    for line in history.iter() {
        let response = Response::RawStream(line.clone());
        if let Err(error) = send_with_shared_tcp_stream(shared_writer.clone(), &response).await {
            log_error!(shared_logger, "{}", error);
        }
    }
    loop {
        let message = broadcast.recv().await;
        match message {
            Ok(message) => {
                let response = Response::RawStream(message);
                if let Err(error) =
                    send_with_shared_tcp_stream(shared_writer.clone(), &response).await
                {
                    log_error!(shared_logger, "{}", error);
                }
            }
            Err(error) => {
                log_error!(shared_logger, "{}", error);
                break;
            }
        }
    }
}
