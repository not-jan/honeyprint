use crate::{
    codec::Codec,
    render::Renderer,
};
use anyhow::{
    anyhow,
    Result,
};
use log::info;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;

pub struct Printer {
    renderer: Arc<Renderer>,
}

impl Printer {
    pub fn new(renderer: Arc<Renderer>) -> Self {
        Self { renderer }
    }

    pub async fn process(&self, stream: TcpStream) -> Result<()> {
        let peer_addr = stream.peer_addr()?;
        let mut framed = Framed::new(stream, Codec::default());
        info!("Got connection from {:?}", peer_addr);
        let job = framed.next().await.ok_or(anyhow!("Connection closed"))??;
        info!("Got job from {:?} [size={}]", peer_addr, job.len());
        self.renderer.render(job.as_ref()).await?;

        Ok(())
    }
}
