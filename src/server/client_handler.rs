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
use tcl::error::TaskmasterError;
use tcl::message::{
    receive_with_shared_tcp_stream, send_with_shared_tcp_stream, Request, Response,
};
use tokio::{
    io::{split, ReadHalf, WriteHalf},
    net::TcpStream,
    sync::Mutex,
    task::JoinHandle,
    time::{sleep, Duration},
};
/* -------------------------------------------------------------------------- */
/*                                   Struct                                   */
/* -------------------------------------------------------------------------- */
pub(super) struct ClientHandler {}

struct Client {
    shared_writer: Arc<Mutex<WriteHalf<TcpStream>>>,
    shared_reader: Arc<Mutex<ReadHalf<TcpStream>>>,
    shared_logger: SharedLogger,
    shared_config: SharedConfig,
    shared_process_manager: SharedProcessManager,
    attached_task: Option<JoinHandle<()>>,
}

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
        let mut client = Client::new(socket, shared_logger, shared_config, shared_process_manager);
        loop {
            match client.receive_request().await {
                Ok(request) => {
                    let response = client.execute_request(request).await;
                    if let Err(error) =
                        send_with_shared_tcp_stream(client.shared_writer.clone(), &response).await
                    {
                        log_error!(client.shared_logger, "{}", error);
                    }
                }
                Err(error) => {
                    if error.connection_lost() {
                        log_info!(client.shared_logger, "Client Disconnected");
                        return;
                    } else {
                        log_error!(client.shared_logger, "{error}");
                    }
                }
            }
        }
    }
}

impl Client {
    pub fn new(
        socket: TcpStream,
        shared_logger: SharedLogger,
        shared_config: SharedConfig,
        shared_process_manager: SharedProcessManager,
    ) -> Self {
        let (reader, writer) = split(socket);
        let shared_writer = Arc::new(Mutex::new(writer));
        let shared_reader = Arc::new(Mutex::new(reader));
        Self {
            shared_writer,
            shared_reader,
            shared_logger,
            shared_config,
            shared_process_manager,
            attached_task: None,
        }
    }

    pub async fn receive_request(&mut self) -> Result<Request, TaskmasterError> {
        receive_with_shared_tcp_stream::<Request>(self.shared_reader.clone()).await
    }

    /// Execute the request and return the response
    pub async fn execute_request(&mut self, request: Request) -> Response {
        match request {
            Request::Status => self.status().await,
            Request::Start(name) => self.start(name).await,
            Request::Stop(name) => self.stop(name).await,
            Request::Restart(name) => self.restart(name).await,
            Request::Reload => self.reload().await,
            Request::Attach(name) => self.attach(name).await,
            Request::Detach => self.detach().await,
        }
    }

    async fn status(&self) -> Response {
        log_info!(self.shared_logger, "Status Request gotten");
        self.shared_process_manager.write().await.get_status()
    }

    async fn start(&self, name: String) -> Response {
        log_info!(self.shared_logger, "Start Request gotten");
        self.shared_process_manager
            .write()
            .await
            .start_program(&name, &self.shared_logger)
            .await
    }

    async fn restart(&self, name: String) -> Response {
        log_info!(self.shared_logger, "Restart Request gotten");
        self.shared_process_manager
            .write()
            .await
            .restart_program(&name, &self.shared_logger)
            .await
    }

    async fn stop(&self, name: String) -> Response {
        log_info!(self.shared_logger, "Stop Request gotten");
        self.shared_process_manager
            .write()
            .await
            .stop_program(&name, &self.shared_logger)
            .await
    }

    async fn reload(&self) -> Response {
        log_info!(self.shared_logger, "Reload Request gotten");
        match Config::load() {
            Ok(config) => {
                *self.shared_config.write().await = config;
                self.shared_process_manager
                    .write()
                    .await
                    .reload_config(&(*self.shared_config.read().await), &self.shared_logger)
                    .await;
                Response::Success("Config Reload Successful".to_owned())
            }
            Err(e) => Response::Error(e.to_string()),
        }
    }

    async fn attach(&mut self, name: String) -> Response {
        if let Some(ref mut _task) = self.attached_task {
            log_info!(self.shared_logger, "Already attached");
            Response::Error("Already attached".to_owned())
        } else {
            let task = self.launch_attach_task(name).await;
            match task {
                Some(task) => {
                    self.attached_task = Some(task);
                    Response::Success("Attach Successful".to_owned())
                }
                None => Response::Error("Program not found".to_owned()),
            }
        }
    }

    async fn detach(&mut self) -> Response {
        if let Some(ref mut task) = self.attached_task {
            task.abort();
            while !task.is_finished() {
                sleep(Duration::from_millis(100)).await;
            }
            self.attached_task = None;
            Response::Success("Detach Successful".to_owned())
        } else {
            Response::Error("Not attached".to_owned())
        }
    }

    /// Launch the attach task to continuously send the stdout of the program
    async fn launch_attach_task(&mut self, name: String) -> Option<JoinHandle<()>> {
        let broadcast = self
            .shared_process_manager
            .write()
            .await
            .subscribe(&name)
            .await;
        let history = self
            .shared_process_manager
            .write()
            .await
            .get_history(&name)
            .await;
        if let (Some(broadcast), Some(history)) = (broadcast, history) {
            let shared_writer = self.shared_writer.clone();
            let shared_logger = self.shared_logger.clone();
            Some(tokio::spawn(Self::transfer_stdout(
                broadcast,
                history,
                shared_writer,
                shared_logger,
            )))
        } else {
            None
        }
    }

    /// Transfer the stdout history of the program to the client and then listen for new stdout
    async fn transfer_stdout(
        mut broadcast: tokio::sync::broadcast::Receiver<String>,
        history: RingBuffer<String>,
        shared_writer: Arc<Mutex<WriteHalf<TcpStream>>>,
        shared_logger: SharedLogger,
    ) {
        for line in history.iter() {
            let response = Response::RawStream(line.clone());
            if let Err(error) = send_with_shared_tcp_stream(shared_writer.clone(), &response).await
            {
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
}
