mod reimpl;

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
    let mut stream = TcpStream::connect(&addr).await?;

    let (mut reader, mut writer) = stream.into_split();

    log::info!("Connected to {}", addr);
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;

    let mut packet_no = 0;

    actix_web::rt::spawn(async move {
        let mut buf = vec![0; 16 * 1024];
        loop {
            let res = reader.read(&mut buf).await;

            match res {
                Ok(n) => {
                    if n == 0 {
                        break;
                    }
                    let b = buf[..n].to_vec();
                    session.binary(b).await.unwrap();
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
                Message::Ping(bytes) => {}
                Message::Text(msg) => {
                    log::debug!("Got text: {msg} - ignoring")
                }
                Message::Binary(msg) => {
                    packet_no += 1;
                    //let packet_delay = 0.031 + (packet_no as f64 / 1000.0).sin() * 0.03;

                    //tokio::time::sleep(std::time::Duration::from_secs_f64(packet_delay)).await;

                    writer.write_all(&msg).await.unwrap();

                    log::debug!("Got binary: {} bytes", msg.len());
                }
                _ => break,
            }
        }
    });

    Ok(response)
}

use clap::Parser;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread::{sleep, spawn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tungstenite::accept;

/// A WebSocket echo server
fn light() {
    let server = TcpListener::bind("127.0.0.1:9001").unwrap();
    for stream in server.incoming() {
        let mut websocket = accept(stream.unwrap()).unwrap();
        spawn(move || {
            loop {
                let msg = match websocket.read() {
                    Ok(msg) => msg,
                    Err(e) => match e {
                        tungstenite::Error::ConnectionClosed => {
                            println!("Connection closed");
                            break;
                        }
                        _ => {
                            println!("Error reading message: {:?}", e);
                            break;
                        }
                    },
                };

                println!("Received message {}", msg.len());
                sleep(std::time::Duration::from_secs(1));
                // We do not want to send back ping/pong messages.
                /*if msg.is_binary() || msg.is_text() {
                    websocket.send(msg).unwrap();
                }*/
            }
        });
    }
}

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
    let use_tungstenite = false;
    let args = Args::parse();
    if use_tungstenite {
        light();
    } else {
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
    }

    Ok(())
}
