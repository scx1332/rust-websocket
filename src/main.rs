use actix_web::{middleware::Logger, web, App, HttpRequest, HttpServer, Responder};
use actix_ws::Message;
use futures_util::StreamExt;
use std::env;

async fn ws(
    req: HttpRequest,
    body: web::Payload,
    path: web::Path<(String,)>,
) -> actix_web::Result<impl Responder> {
    //let b = body.to_bytes_limited(100).await.unwrap();
    //connecting to tcp server:
    let addr = path.into_inner().0;
    let stream = TcpStream::connect(&addr).await?;

    let (mut reader, mut writer) = stream.into_split();

    log::info!("Connected to {}", addr);
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;

    actix_web::rt::spawn(async move {
        let mut buf = vec![0; 60 * 1000];
        loop {
            let res = reader.read(&mut buf).await;

            match res {
                Ok(n) => {
                    if n == 0 {
                        break;
                    }
                    session.binary(buf[..n].to_vec()).await.unwrap();
                }
                Err(e) => {
                    log::error!("Error reading from tcp stream: {}", e);
                    break;
                }
            }
        }
    });
    actix_web::rt::spawn(async move {
        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                Message::Ping(_bytes) => {
                    //@todo: implement
                }
                Message::Text(msg) => {
                    writer.write_all(msg.as_bytes()).await.unwrap();
                }
                Message::Binary(msg) => {
                    writer.write_all(&msg).await.unwrap();
                }
                _ => break,
            }
        }
    });

    Ok(response)
}

use clap::Parser;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(long, default_value = "8080")]
    listen_port: u16,

    #[clap(long, default_value = "127.0.0.1")]
    listen_addr: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env::set_var(
        "RUST_LOG",
        env::var("RUST_LOG").unwrap_or("info".to_string()),
    );
    env_logger::init();
    let args = Args::parse();
    log::info!(
        "Starting server on {}:{}",
        args.listen_addr,
        args.listen_port
    );

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .route("/ws/{addr}", web::get().to(ws))
    })
    .bind(format!("{}:{}", args.listen_addr, args.listen_port))?
    .run()
    .await?;

    Ok(())
}
