/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use std::io;
use tcl::get_server_address;
use tcl::message::{receive_message, send_message, Message, Message::Test};
use tokio::net::{TcpListener, TcpStream};

/* -------------------------------------------------------------------------- */
/*                                    Main                                    */
/* -------------------------------------------------------------------------- */
#[tokio::main]
async fn main() {
    println!("Starting Taskmaster Daemon");

    let listener = TcpListener::bind(get_server_address())
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
        match receive_message::<Message>(&mut socket).await {
            Ok(message) => match message {
                Test(string) => {
                    println!("Message: {string}");
                    if let Err(error) = send_message(&mut socket, &Message::Test(string)).await {
                        println!("{error}");
                    }
                }
            },
            Err(error) => eprintln!("{error}"),
        }
    }
}
