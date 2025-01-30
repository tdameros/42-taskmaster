/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;

/* -------------------------------------------------------------------------- */
/*                                  Function                                  */
/* -------------------------------------------------------------------------- */
pub fn send_http_message(address: String, message: String) {
    thread::spawn(move || {
        // Connect to the server
        let stream_result = TcpStream::connect(address.to_owned());
        if stream_result.is_err() {
            return;
        }
        let mut stream = stream_result.unwrap();

        // Prepare the JSON payload
        let body = format!("{{\"message\":\"{}\"}}", message);

        // Construct the HTTP POST request with JSON content type
        let request = format!(
            "POST / HTTP/1.1\r\n\
             Host: {}\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             \r\n\
             {}",
            address,
            body.len(),
            body
        );

        // Send the request
        stream.write_all(request.as_bytes()).unwrap();

        // Read the response
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();

        println!("Server response: {}", response);
    });
}

#[cfg(feature = "reqwest")]
pub async fn send_notification(token: String, title: String, body: String) {
    tokio::spawn(async move {
        let client = reqwest::Client::new();

        let res = client
            .post("https://api.pushbullet.com/v2/pushes")
            .header("Access-Token", token)
            .json(&serde_json::json!({
                "type": "note",
                "title": title,
                "body": body
            }))
            .send()
            .await;
        if let Ok(result) = res {
            println!("Status: {}", result.status());
            let _ = result.text().await.map(|res| {
                println!("Response: {}", res);
            });
        }
    });
}
