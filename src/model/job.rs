use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Ipp,
    JetDirect,
}

impl Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Protocol::Ipp => "IPP".to_string(),
            Protocol::JetDirect => "JetDirect".to_string(),
        };
        write!(f, "{}", str)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub protocol: Protocol,
    pub source: String,
    pub raw_data: Vec<u8>,
}