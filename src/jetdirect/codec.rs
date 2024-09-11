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




impl Decoder for Codec {
    type Error = anyhow::Error;
    type Item = Vec<u8>;

    fn decode(&mut self, src: &mut BytesMut) -> anyhow::Result<Option<Self::Item>, Self::Error> {
        if self.finished {
            bail!("Already finished");
        }
        
        // Read until EOF
        for (i, b) in src.iter().enumerate() {
            if *b == 4 {
                self.finished = true;

                let data = src[..i + 1].to_vec();
                src.advance(i);

                return Ok(Some(data));
            }
        }

        Ok(None)
    }
}
