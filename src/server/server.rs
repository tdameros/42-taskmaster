/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use std::io;
use tcl::{config::Config, message::{receive, send, Response::{self, Test}}};
use tokio::net::{TcpListener, TcpStream};

/* -------------------------------------------------------------------------- */
/*                                    Main                                    */
/* -------------------------------------------------------------------------- */
#[tokio::main]
async fn main() {
    println!("Starting Taskmaster Daemon");
    let config = Config::load().unwrap();
    println!("{config:?}");

    let listener = TcpListener::bind(tcl::SOCKET_ADDRESS)
        .await
        .expect("Failed to bind tcp listener");

    loop {
        println!("Waiting for Client To arrive");
        if let Err(error) = routine(&listener).await {
            eprintln!("An error has occurred while accepting a client: {error}");
        }
        println!("Client Accepted");
    }
}

async fn routine(listener: &TcpListener) -> io::Result<()> {
    let (socket, _address) = listener.accept().await?;

    tokio::spawn(handle_client(socket));
    Ok(())
}

async fn handle_client(mut socket: TcpStream) {
    loop {
        match receive::<Response>(&mut socket).await {
            Ok(message) => match message {
                Test(string) => {
                    println!("Message: {string}");
                    if let Err(error) = send(&mut socket, &Response::Test(string)).await {
                        println!("{error}");
                    }
                }
            },
            Err(error) => eprintln!("{error}"),
        }
    }
}
