/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use std::io;
use std::net::TcpStream;
use tcl::get_server_address;

/* -------------------------------------------------------------------------- */
/*                                    Main                                    */
/* -------------------------------------------------------------------------- */
fn main() {
    println!("Hello, world from taskmasterctl!");
    if let Err(error) = connect() {
        eprintln!("a error has occurred while connecting to the server: {error}");
    }
}

fn connect() -> io::Result<()> {
    let _listener = TcpStream::connect(get_server_address())?;
    Ok(())
}
