use std::sync::Arc;

use ippper::model::{PageOrientation, Resolution};
use ippper::server::{serve_adaptive_https, tls_config_from_reader, wrap_as_http_service};
use ippper::service::simple::{PrinterInfoBuilder, SimpleIppDocument, SimpleIppService, SimpleIppServiceHandler};
use log::{debug, info};
use rcgen::{generate_simple_self_signed, CertifiedKey};
use tokio::sync::mpsc;
use uuid::Uuid;
use crate::model::job::{Job, Protocol};
use tokio_util::compat::*;

struct Ipp {
    tx: mpsc::Sender<Job>,
}

impl Ipp {
    pub fn new(tx: mpsc::Sender<Job>) -> Self {
        Self {
            tx
        }
    }
}

impl SimpleIppServiceHandler for Ipp {
    async fn handle_document(
        &self,
        document: SimpleIppDocument,
    ) -> anyhow::Result<()> {
            info!("Received document via IPP");

            // let source = document.job_attributes.originating_user_name;
            let mut buf = Vec::new();

            tokio::io::copy(&mut document.payload.compat(), &mut buf).await?;



            self.tx.send(Job {
                protocol: Protocol::Ipp,
                source: document.format.unwrap(),
                raw_data: buf,
            }).await?;

            Ok(())
    }
}

pub async fn run(addr: std::net::SocketAddr, tx: mpsc::Sender<Job>) -> anyhow::Result<()> {
    info!("Starting IPP server on {}", addr);

    let uuid = Uuid::new_v4();
    debug!("Using UUID {}", uuid);

    let subject_alt_names = vec![
        format!("{}", addr.ip())
    ];

    let CertifiedKey { cert, key_pair } = generate_simple_self_signed(subject_alt_names)?;

    let info = PrinterInfoBuilder::default()
        .uuid(Some(
            uuid
        ))
        .sides_supported(vec![
            "one-sided".to_string(),
            "two-sided-long-edge".to_string(),
            "two-sided-short-edge".to_string(),
        ])
        .printer_resolution_supported(vec![
            Resolution {
                cross_feed: 300,
                feed: 300,
                units: 3,
            },
            Resolution {
                cross_feed: 600,
                feed: 600,
                units: 3,
            },
        ])
        .printer_resolution_default(Some(Resolution {
            cross_feed: 600,
            feed: 600,
            units: 3,
        }))
        .orientation_supported(vec![PageOrientation::Portrait, PageOrientation::Landscape])
        .build()?;

    let ipp_service = Arc::new(SimpleIppService::new(info, Ipp::new(tx)));
    let key = key_pair.serialize_pem();
    let cert = cert.pem();

    let tls_config = Arc::new(tls_config_from_reader(cert.as_bytes(), key.as_bytes())?);

    serve_adaptive_https(addr, wrap_as_http_service(ipp_service), tls_config).await
}