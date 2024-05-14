use std::ops::Deref;

use bytes::{Buf, BytesMut};

use crate::{calc_total_length, parse_length, RespDecode, RespEncode, RespError, RespFrame};

use super::{BUF_CAP, CRLF_LEN};

#[derive(Debug, Clone, PartialEq)]
pub struct RespSet(pub(crate) Vec<RespFrame>);

impl RespEncode for RespSet {
    fn encode(self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(BUF_CAP);
        buf.extend_from_slice(&format!("~{}\r\n", self.len()).into_bytes());
        for frame in self.0 {
            buf.extend_from_slice(&frame.encode());
        }
        buf
    }
}

// - set: "~<number-of-elements>\r\n<element-1>...<element-n>"
impl RespDecode for RespSet {
    const PREFIX: &'static str = "~";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        let total_len = calc_total_length(buf, end, len, Self::PREFIX)?;

        if buf.len() < total_len {
            return Err(RespError::NotComplete);
        }
        buf.advance(end + CRLF_LEN);

        let mut set = Vec::new();
        for _ in 0..len {
            set.push(RespFrame::decode(buf)?);
        }
        Ok(RespSet::new(set))
    }
    fn expect_length(buf: &[u8]) -> Result<usize, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        calc_total_length(buf, end, len, Self::PREFIX)
    }
}

impl Deref for RespSet {
    type Target = Vec<RespFrame>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl RespSet {
    pub fn new(s: impl Into<Vec<RespFrame>>) -> Self {
        RespSet(s.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::{BulkString, RespArray, SimpleString};

    use super::*;

    #[test]
    fn test_set_decode() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"~3\r\n+key1\r\n$6\r\nvalue1\r\n+key2\r\n");
        let frame = RespSet::decode(&mut buf).unwrap();
        assert_eq!(
            frame,
            RespSet::new(vec![
                SimpleString::new("key1".to_string()).into(),
                BulkString::new(b"value1".to_vec()).into(),
                SimpleString::new("key2".to_string()).into(),
            ])
        );
    }

    #[test]
    fn test_set() {
        let frame: RespFrame = RespSet::new(vec![
            RespArray::new(vec![1234.into(), true.into()]).into(),
            BulkString::new("world".as_bytes().to_vec()).into(),
        ])
        .into();
        assert_eq!(
            frame.encode(),
            b"~2\r\n*2\r\n:+1234\r\n#t\r\n$5\r\nworld\r\n"
        );
    }
}
