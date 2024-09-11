
use anyhow::{
    anyhow,
    Result,
};

use clap::Parser;

use std::{
    net::{
        Ipv4Addr,
        SocketAddrV4,
    },

};
use std::net::SocketAddr;

use log::{error, info};
use serenity::all::{CreateEmbed, ExecuteWebhook, Webhook};
use serenity::builder::CreateAttachment;
use serenity::http::Http;
use tokio::process::Command;
use tokio::sync::mpsc;
use crate::model::job::{Job, Protocol};


mod ipp;
mod jetdirect;
mod model;

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
        short = 'd',
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

    let (tx, mut rx) = mpsc::channel::<Job>(10);

    let jet_direct_addr = SocketAddrV4::new(args.address, args.port);
    let ipp_addr = SocketAddrV4::new(args.address, 631);
    let webhook_url = args.webhook_url.clone();
    info!("Starting honeyprint on {}:{}", args.address, args.port);


    let _job_handler = tokio::spawn(async move {
        while let Some(job) = rx.recv().await {
            info!("Got job from {:?}", job.source);
            let http = Http::new("");
            let job_webhook_url = webhook_url.clone();
            let webhook = Webhook::from_url(&http, &webhook_url).await?;

            let embed = CreateEmbed::default().title("New print job").fields(vec![
                ("Source", job.source, false),
                ("Protocol", job.protocol.to_string(), false),
                ("Size", job.raw_data.len().to_string(), false),
            ]);

            let attachment_name = match job.protocol {
                Protocol::Ipp => "input.job",
                Protocol::JetDirect => "input.ps",
            };

            let job_data = job.raw_data.clone();


            tokio::spawn(async move {
                let tempdir = tempfile::tempdir()?;

                let input_file = tempdir.path().join("input.ps");
                let output_file = tempdir.path().join("output.pdf");

                std::fs::write(&input_file, &job_data)?;

                let input = input_file.to_str().unwrap();
                let output = output_file.to_str().unwrap();

                let mut child = Command::new("ps2pdf")
                    .arg(input)
                    .arg(output).spawn()?;


                child.wait().await?;
                let http = Http::new("");
                let webhook = Webhook::from_url(&http, &job_webhook_url).await?;

                let attachment = CreateAttachment::path(output_file).await?;
                let builder = ExecuteWebhook::new().username("HoneyPrint").add_file(attachment);
                webhook.execute(&http, false, builder).await?;

                Result::<()>::Ok(())
            });


            let attachment = CreateAttachment::bytes(job.raw_data, attachment_name);

            let builder = ExecuteWebhook::new().username("HoneyPrint").embed(embed).add_file(attachment);

            match webhook.execute(&http, false, builder).await {
                Ok(_) => {
                    info!("Sent job to Discord");
                }
                Err(e) => {
                    error!("Failed to send job to Discord; error = {}", e);
                }
            }
        }

        Result::<()>::Ok(())
    });


    let ipp = ipp::server::run(SocketAddr::V4(ipp_addr), tx.clone());
    let jetdirect = jetdirect::server::run(SocketAddr::V4(jet_direct_addr), tx.clone());

    tokio::try_join!(ipp, jetdirect).map_err(|e| anyhow!(e))?;
    Ok(())
}
