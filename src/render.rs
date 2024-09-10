use anyhow::{
    bail,
    Result,
};
use log::{
    error,
    info,
};
use serenity::all::{
    CreateAttachment,
    ExecuteWebhook,
    Http,
    Webhook,
};
use std::{
    os::unix::fs::MetadataExt,
    path::{
        Path,
        PathBuf,
    },
    time::Duration,
};
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    process::Command,
    time::timeout,
};

#[derive(Clone, Debug)]
pub struct Renderer {
    max_file_size: usize,
    max_file_size_discord: usize,
    webhook_url: String,
    timeout: u64,
}

impl Renderer {
    pub fn new(
        max_file_size: usize,
        max_file_size_discord: usize,
        webhook_url: String,
        timeout: u64,
    ) -> Self {
        Self {
            max_file_size,
            max_file_size_discord,
            webhook_url,
            timeout,
        }
    }

    async fn send(&self, path: impl AsRef<Path>) -> Result<()> {
        let meta = std::fs::metadata(&path)?;
        let size = meta.size() as usize;

        if size > self.max_file_size {
            bail!("File is too large to send to Discord");
        }

        let http = Http::new("");
        let webhook = Webhook::from_url(&http, &self.webhook_url).await?;

        let builder = ExecuteWebhook::new().username("HoneyPrint").add_file(
            CreateAttachment::path(&path)
                .await?
                .description("output.pdf"),
        );

        webhook.execute(&http, false, builder).await?;

        Ok(())
    }

    pub async fn render(&self, input: &[u8]) -> Result<()> {
        let temp = tempfile::tempdir()?;
        let mut output_path = PathBuf::from(temp.path());
        output_path.push("output.pdf");

        let mut input_path = PathBuf::from(temp.path());
        input_path.push("input.ps");


        let mut file = File::create(&input_path).await?;
        file.write_all(input).await?;
        file.flush().await?;

        match self.send(&input_path).await {
            Ok(_) =>  info!("Sent PostScript to Discord"),
            Err(e) => {
                error!("Failed to send PostScript to Discord; error = {}", e)
            }

        }


        let child = Command::new("ps2pdf")
            .arg(&input_path)
            .arg(&output_path)
            .spawn()?;
        
        

        match timeout(Duration::from_secs(self.timeout), child.wait_with_output()).await {
            Ok(_) => {
                info!("Rendered PDF, sending to Discord");

                let meta = std::fs::metadata(&output_path)?;
                let size = meta.size() as usize;

                if size > self.max_file_size_discord {
                    bail!("File is too large to send to Discord");
                }

                self.send(&output_path).await?;
            }
            Err(e) => {
                error!("Failed to render PDF; error = {}", e);
                bail!("Failed to render PDF; error = {}", e);
            }
        }

        Ok(())
    }
}
