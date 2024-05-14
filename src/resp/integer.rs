use bytes::BytesMut;

use crate::{extract_simaple_frame_data, RespDecode, RespEncode, RespError};

use super::CRLF_LEN;

// integer: ":[<+|->]<value>\r\n"
impl RespEncode for i64 {
    fn encode(self) -> Vec<u8> {
        let sign = if self < 0 { "" } else { "+" };
        format!(":{}{}\r\n", sign, self).into_bytes()
    }
}

impl RespDecode for i64 {
    const PREFIX: &'static str = ":";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let end = extract_simaple_frame_data(buf, Self::PREFIX)?;
        let data = buf.split_to(end + 2);
        let s = String::from_utf8_lossy(&data[1..end]);
        Ok(s.parse()?)
    }
    fn expect_length(buf: &[u8]) -> Result<usize, RespError> {
        let end = extract_simaple_frame_data(buf, Self::PREFIX)?;
        Ok(end + CRLF_LEN)
    }
}

#[cfg(test)]
mod tests {
    use crate::RespFrame;

    use super::*;

    #[test]
    fn test_integer_decode() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b":+123\r\n");
        let frame = i64::decode(&mut buf).unwrap();
        assert_eq!(frame, 123);

        buf.extend_from_slice(b":-123\r\n");
        let frame = i64::decode(&mut buf).unwrap();
        assert_eq!(frame, -123);
    }

    #[test]
    fn test_integer() {
        let frame: RespFrame = 123.into();
        assert_eq!(frame.encode(), b":+123\r\n");

        let frame: RespFrame = (-123).into();
        assert_eq!(frame.encode(), b":-123\r\n");
    }
}
