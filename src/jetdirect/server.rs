use log::info;
use anyhow::Result;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use crate::jetdirect::codec::Codec;
use crate::model::job::{Job, Protocol};

struct JetDirect {
  tx: mpsc::Sender<Job>,
}

impl JetDirect {
    pub fn new(tx: mpsc::Sender<Job>) -> Self {
        Self {
            tx
        }
    }

    async fn process(&self, stream: TcpStream) -> Result<()> {
        let peer_addr = stream.peer_addr()?;
        let mut framed = Framed::new(stream, Codec::default());

        while let Some(Ok(job)) = framed.next().await {
            info!("Got JetDirect job from {:?} [size={}]", peer_addr, job.len());

            self.tx.send(Job {
                protocol: Protocol::JetDirect,
                source: format!("{}", peer_addr),
                raw_data: job,
            }).await?;
        }

        Ok(())
    }
}




pub async fn run(addr: std::net::SocketAddr, tx: mpsc::Sender<Job>) -> Result<()> {
    let listener = TcpListener::bind(&addr).await?;

    let handle = tokio::spawn(async move {
        let sender = tx.clone();
        while let Ok((stream, _)) = listener.accept().await {
            info!("Got JetDirect connection from {:?}", stream.peer_addr()?);
            let sender2 = sender.clone();
            tokio::spawn(async move {
                if let Err(e) = JetDirect::new(sender2.clone()).process(stream).await {
                    info!("failed to process JetDirect connection; error = {}", e);
                }
            });
        }

        Result::<()>::Ok(())
    });


    handle.await?
}