use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::Deserialize;

#[derive(Deserialize)]
struct Message {
    message: String,
}

async fn handle_message(message: web::Json<Message>) -> impl Responder {
    println!("Received message: {}", message.message);
    HttpResponse::Ok().body("Message received")
}

/// this is only to demonstrate that a POST request is indeed send to the address specified in the config
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Server starting on http://127.0.0.1:8080");

    HttpServer::new(|| App::new().route("/", web::post().to(handle_message)))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}

// use to demonstrate that with request it work
// use reqwest;
// use serde_json::json;

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let access_token = std::env::var("API_KEY").unwrap_or_default();
//     let client = reqwest::Client::new();

//     let res = client.post("https://api.pushbullet.com/v2/pushes")
//         .header("Access-Token", access_token)
//         .json(&json!({
//             "type": "note",
//             "title": "Test Notification",
//             "body": "This is a test message from Rust!"
//         }))
//         .send()
//         .await?;

//     println!("Status: {}", res.status());
//     println!("Response: {}", res.text().await?);

//     Ok(())
// }
