mod reimpl;

use actix_web::{middleware::Logger, web, App, HttpRequest, HttpServer, Responder};
use reimpl::Message;
use futures_util::StreamExt;

async fn ws(req: HttpRequest, body: web::Payload) -> actix_web::Result<impl Responder> {


    let (response, mut session, mut msg_stream) = reimpl::handle(&req, body)?;

    actix_web::rt::spawn(async move {
        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                Message::Ping(bytes) => {
                    if session.pong(&bytes).await.is_err() {
                        return;
                    }
                }
                Message::Text(msg) => println!("Got text: {msg}"),
                Message::Binary(msg) => {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    println!("Got binary: {} bytes", msg.len());
                },
                _ => break,
            }
        }

        let _ = session.close(None).await;
    });

    Ok(response)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .route("/ws", web::get().to(ws))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await?;

    Ok(())
}
