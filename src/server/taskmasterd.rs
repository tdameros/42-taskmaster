/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use std::io;
use tcl::get_server_address;
use tokio::net::TcpListener;

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
    let (_socket, _address) = listener.accept().await?;
    Ok(())
}
