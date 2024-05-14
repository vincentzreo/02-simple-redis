use bytes::BytesMut;

use crate::{extract_fixed_data, RespDecode, RespEncode, RespError};

// - boolean: "#<t|f>\r\n"
impl RespEncode for bool {
    fn encode(self) -> Vec<u8> {
        format!("#{}\r\n", if self { "t" } else { "f" }).into_bytes()
    }
}

impl RespDecode for bool {
    const PREFIX: &'static str = "#";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        match extract_fixed_data(buf, "#t\r\n", "Bool") {
            Ok(_) => Ok(true),
            Err(_) => match extract_fixed_data(buf, "#f\r\n", "Bool") {
                Ok(_) => Ok(false),
                Err(e) => Err(e),
            },
        }
    }
    fn expect_length(_buf: &[u8]) -> Result<usize, RespError> {
        Ok(4)
    }
}

#[cfg(test)]
mod tests {
    use crate::RespFrame;

    use super::*;

    #[test]
    fn test_boolean_decode() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"#t\r\n");
        let frame = bool::decode(&mut buf).unwrap();
        assert!(frame);

        buf.extend_from_slice(b"#f\r\n");
        let frame = bool::decode(&mut buf).unwrap();
        assert!(!frame);
    }

    #[test]
    fn test_boolean() {
        let frame: RespFrame = true.into();
        assert_eq!(frame.encode(), b"#t\r\n");

        let frame: RespFrame = false.into();
        assert_eq!(frame.encode(), b"#f\r\n");
    }
}
