use tokio_util::bytes::{BytesMut};
use std::collections::HashMap;
use tokio_util::codec::{Decoder};
use thiserror::Error;

#[derive(Debug, Default)]
pub struct EslCodec {
    offset: Option<usize>, 
    length: Option<usize>,
}

impl EslCodec {
    pub fn new() -> Self {
        Self::default()
    }
}


#[derive(Debug)]
pub struct EslPacket {
    pub headers: HashMap<String, String>,
    pub payload: Option<String>,
}

#[derive(Debug, Error)]
pub enum EslCodecError {
    #[error("Failed to parse text as UTF-8")]
    MalformedUtf8,
    #[error("Failed to parse packet Content-Length field")]
    InvalidContentLength,
    #[error("Failed to parse headers, socket stream may not be aligned ")]
    InvalidHeaders,
    #[error("IO error")]
    IoError(#[from] std::io::Error)
}

impl Decoder for EslCodec {
    type Item = EslPacket;
    type Error = EslCodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {

        let delim_gap = b"\n\n".len();

        match self {

            // packet found, detecting end of headers
            Self { offset: None, .. } if src.windows(2).position(|each| each == b"\n\n").is_some() => 
            {
                let headers_end_ix = src.windows(2).position(|each| each == b"\n\n").unwrap(); // SAFETY: just guard-mathced against this very condition
                let length = try_parse_content_length(&src)?;
                *self = Self { offset: Some(headers_end_ix), length };
                self.decode(src)
            },

            // Packet has no Content-Length - decoding headers only
            Self { offset: Some(headers_end_index), length: None } => {

                let result = src.split_to(*headers_end_index);
                let headers = head_to_map(&result)?;

                // Move past the gap to read the next header
                let _ = src.split_to(delim_gap);

                *self = Self::default();
                Ok(Some(EslPacket {
                    headers: headers,
                    payload: None
                }))
            },

            // Packet has Content-length, and current buffer holds no less than content-length bytes of payload content - decoding headers, then body
            Self { offset: Some(offset), length: Some(length) } if *offset > 0 && src[*offset + delim_gap ..].len() >= *length => {

                let result  = src.split_to(*length + *offset + delim_gap);
    
                let headers = &result[.. *offset];
                let payload = &result[*offset + delim_gap ..];

                let headers = head_to_map(&headers)?;

                *self = Self::default();
                Ok(Some(EslPacket {
                    headers: headers,
                    payload: Some(String::from(std::str::from_utf8(payload).map_err(|_| Self::Error::MalformedUtf8)?)),
                }))
            },

            _ => Ok(None),
        }
    }
}



fn head_to_map(head: &[u8]) -> Result<HashMap<String, String>, EslCodecError> {

    let headers = std::str::from_utf8(&head).map_err(|_| EslCodecError::MalformedUtf8)?;

    let mut result = HashMap::new();

    for line in headers.lines() {
        let mut it = line.split(": ");
        let k = String::from(it.next().ok_or(EslCodecError::InvalidHeaders)?);
        let v = String::from(it.next().ok_or(EslCodecError::InvalidHeaders)?);
        let _ = result.insert(k, v);
    }

    Ok(result)
}


fn try_parse_content_length(src: &[u8]) -> Result<Option<usize>, EslCodecError> {
    if !src.starts_with(b"Content-Length: ") {
        return Ok(None);
    }

    let len = b"Content-Length: ".len();
    let pos = src.iter().position(|c| *c == b'\n').ok_or(EslCodecError::InvalidContentLength)?;
    let src = &src[len..pos];

    let str = std::str::from_utf8(&src).map_err(|_| EslCodecError::InvalidContentLength)?;
    let len = str.parse::<usize>().map_err(|_| EslCodecError::InvalidContentLength)?;

    Ok(Some(len))
}

