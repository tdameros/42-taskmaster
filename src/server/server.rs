/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use config::Config;
use std::io;
use tcl::message::{receive, Request};
use tokio::net::{TcpListener, TcpStream};

/* -------------------------------------------------------------------------- */
/*                                   Module                                   */
/* -------------------------------------------------------------------------- */
mod config;

/* -------------------------------------------------------------------------- */
/*                                    Main                                    */
/* -------------------------------------------------------------------------- */
#[tokio::main]
async fn main() {
    // load the config
    println!("Starting Taskmaster Daemon");
    let config = Config::load()
        .expect("please provide a file named 'config.yaml' at the root of this rust project");
    println!("{config:?}");

    // start the listener
    let listener = TcpListener::bind(tcl::SOCKET_ADDRESS)
        .await
        .expect("Failed to bind tcp listener");

    // handle the client connection
    loop {
        println!("Waiting for Client To arrive");
        if let Err(error) = routine(&listener).await {
            eprintln!("An error has occurred while accepting a client: {error}");
        }
        println!("Client Accepted");
    }
}

/// do the reception of client and spawn a handler for each client connected
async fn routine(listener: &TcpListener) -> io::Result<()> {
    let (socket, _address) = listener.accept().await?;

    tokio::spawn(handle_client(socket));
    Ok(())
}

/// do the actual match of the client request
async fn handle_client(mut socket: TcpStream) {
    loop {
        match receive::<Request>(&mut socket).await {
            Ok(message) => match message {
                Request::Status => todo!(),
                Request::Start(_) => todo!(),
                Request::Stop(_) => todo!(),
                Request::Restart(_) => todo!(),
                Request::Reload => todo!(),
            },
            Err(error) => {
                // if the error occurred because the client disconnected then the task of this thread is finished
                if error.client_disconnected() {
                    println!("Client has disconnected");
                    break;
                } else {
                    eprintln!("{error}")
                }
            }
        }
    }
}
