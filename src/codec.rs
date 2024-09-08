use anyhow::bail;
use bytes::{
    Buf,
    BytesMut,
};
use tokio_util::codec::Decoder;

#[derive(Debug, Copy, Clone, Default)]
pub struct Codec {
    finished: bool,
}

#[derive(Debug, Clone)]
pub struct PrintJob(Vec<u8>);

impl AsRef<[u8]> for PrintJob {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl PrintJob {
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl Decoder for Codec {
    type Error = anyhow::Error;
    type Item = PrintJob;

    fn decode(&mut self, src: &mut BytesMut) -> anyhow::Result<Option<Self::Item>, Self::Error> {
        if self.finished {
            bail!("Already finished");
        }

        for (i, b) in src.iter().enumerate() {
            if *b == 4 {
                self.finished = true;

                let data = src[..i + 1].to_vec();
                src.advance(i);

                return Ok(Some(PrintJob(data)));
            }
        }

        Ok(None)
    }
}
