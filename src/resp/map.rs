use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use bytes::{Buf, BytesMut};

use crate::{
    calc_total_length, parse_length, RespDecode, RespEncode, RespError, RespFrame, SimpleString,
};

use super::{BUF_CAP, CRLF_LEN};

#[derive(Debug, Clone, PartialEq)]
pub struct RespMap(pub(crate) HashMap<String, RespFrame>);

// - map: "%<number-of-entries>\r\n<key-1><value-1>...<key-n><value-n>"
impl RespEncode for RespMap {
    fn encode(self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(BUF_CAP);
        buf.extend_from_slice(&format!("%{}\r\n", self.len()).into_bytes());
        for (key, value) in self.0 {
            buf.extend_from_slice(&SimpleString::new(key).encode());
            buf.extend_from_slice(&value.encode());
        }
        buf
    }
}

// - map: "%<number-of-entries>\r\n<key-1><value-1>...<key-n><value-n>"
impl RespDecode for RespMap {
    const PREFIX: &'static str = "%";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        let total_len = calc_total_length(buf, end, len, Self::PREFIX)?;

        if buf.len() < total_len {
            return Err(RespError::NotComplete);
        }
        buf.advance(end + CRLF_LEN);

        let mut map = RespMap::new();
        for _ in 0..len {
            let key = SimpleString::decode(buf)?;
            let value = RespFrame::decode(buf)?;
            map.insert(key.0, value);
        }
        Ok(map)
    }
    fn expect_length(buf: &[u8]) -> Result<usize, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        calc_total_length(buf, end, len, Self::PREFIX)
    }
}

impl Deref for RespMap {
    type Target = HashMap<String, RespFrame>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RespMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for RespMap {
    fn default() -> Self {
        Self::new()
    }
}

impl RespMap {
    pub fn new() -> Self {
        RespMap(HashMap::new())
    }
}

impl From<HashMap<String, RespFrame>> for RespMap {
    fn from(map: HashMap<String, RespFrame>) -> Self {
        RespMap(map)
    }
}

#[cfg(test)]
mod tests {
    use crate::BulkString;

    use super::*;

    #[test]
    fn test_map_decode() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"%2\r\n+key1\r\n$6\r\nvalue1\r\n+key2\r\n$6\r\nvalue2\r\n");
        let frame = RespMap::decode(&mut buf).unwrap();
        let mut map = RespMap::new();
        map.insert(
            "key1".to_string(),
            BulkString::new(b"value1".to_vec()).into(),
        );
        map.insert(
            "key2".to_string(),
            BulkString::new(b"value2".to_vec()).into(),
        );
        assert_eq!(frame, map);
    }

    #[test]
    fn test_map() {
        let mut map = RespMap::new();
        map.insert(
            "name".to_string(),
            BulkString::new("Alice".as_bytes().to_vec()).into(),
        );
        map.insert("age".to_string(), (-18.21).into());

        let frame: RespFrame = map.into();
        let frame_binding = frame.encode();
        let frame_res = String::from_utf8_lossy(&frame_binding);
        assert!(frame_res.contains("%2\r\n"));
        assert!(frame_res.contains("+name\r\n$5\r\nAlice\r\n"));
        assert!(frame_res.contains("+age\r\n,-18.21\r\n"));
    }
}
