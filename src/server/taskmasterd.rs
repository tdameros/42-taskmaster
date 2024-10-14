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
    println!("Hello, world from taskmasterd!");

    let listener = TcpListener::bind(get_server_address())
        .await
        .expect("Failed to bind tcp listener");

    println!("Bind in {}", get_server_address());

    loop {
        println!("accepting client");
        if let Err(error) = routine(&listener).await {
            eprintln!("a error has occured while accepting a client: {error}");
        }
        println!("client accepted");
    }
}

async fn routine(listener: &TcpListener) -> io::Result<()> {
    println!("in routine");
    let (socket, address) = listener.accept().await?;
    println!("Client connected: {}", address.ip());
    println!("haha: {:?}", socket.peer_addr());

    Ok(())
}
