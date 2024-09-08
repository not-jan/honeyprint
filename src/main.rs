use crate::{
    print::Printer,
    render::Renderer,
};
use anyhow::{
    anyhow,
    Result,
};
use bytes::Bytes;
use clap::Parser;
use futures::SinkExt;
use std::{
    net::{
        Ipv4Addr,
        SocketAddrV4,
    },
    sync::Arc,
};
use log::info;
use tokio::net::{
        TcpListener,
        UdpSocket,
    };
use tokio_stream::StreamExt;
use tokio_util::{
    codec::BytesCodec,
    udp::UdpFramed,
};

mod codec;
mod print;
mod render;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
/// A tool to catch hooligans sending printer jobs across the internet
struct Cli {
    /// What address to bind to
    #[clap(short, long, default_value = "127.0.0.1", env = "HONEYPRINT_ADDRESS")]
    address: Ipv4Addr,
    /// What port to bind to
    #[clap(short, long, default_value = "9100", env = "HONEYPRINT_PORT")]
    port: u16,
    /// What port to bind to for status
    #[clap(short, long, default_value = "9101", env = "HONEYPRINT_STATUS_PORT")]
    status_port: u16,
    /// Maximum file size to accept on the print server in bytes
    #[clap(
        short,
        long,
        default_value = "1048576",
        env = "HONEYPRINT_MAX_FILE_SIZE"
    )]
    max_file_size: usize,
    /// Maximum file size to send to Discord in bytes
    #[clap(
        short,
        long,
        default_value = "524288",
        env = "HONEYPRINT_MAX_FILE_SIZE_DISCORD"
    )]
    max_file_size_discord: usize,
    /// Discord webhook URL
    #[clap(short, long, env = "HONEYPRINT_WEBHOOK_URL")]
    webhook_url: String,
    /// Timeout for rendering PDFs
    #[clap(short, long, default_value = "5", env = "HONEYPRINT_TIMEOUT")]
    timeout: u64,
}



#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Cli::parse();
    let status_addr = SocketAddrV4::new(args.address, args.status_port);
    let addr = SocketAddrV4::new(args.address, args.port);

    info!("Starting honeyprint on {}:{}", args.address, args.port);
    let status_socket = UdpSocket::bind(&status_addr).await?;
    let mut status_framed = UdpFramed::new(status_socket, BytesCodec::new());

    let print_socket = TcpListener::bind(&addr).await?;

    let renderer = Arc::new(Renderer::new(
        args.max_file_size,
        args.max_file_size_discord,
        args.webhook_url,
        args.timeout,
    ));

    let status = tokio::spawn(async move {
        while let Some(Ok((data, addr))) = status_framed.next().await {
            info!("Got data: {:?} from {:?}", data, addr);
            match status_framed.send((Bytes::from(&b"OK"[..]), addr)).await {
                Ok(_) => {
                    println!("Sent OK");
                }
                Err(e) => {
                    println!("Failed to send OK; error = {}", e);
                }
            }
        }
    });

    let prints = tokio::spawn(async move {
        let r = renderer.clone();
        loop {
            if let Ok((stream, _)) = print_socket.accept().await {
                let printer = Printer::new(r.clone());

                tokio::spawn(async move {
                    if let Err(e) = printer.process(stream).await {
                        info!("failed to process connection; error = {}", e);
                    }
                });
            }
        }
    });

    tokio::try_join!(status, prints).map_err(|e| anyhow!(e))?;
    Ok(())
}
