use crate::{RespArray, RespFrame, RespNull};

use super::{extract_args, validate_command, CommandError, CommandExecutor, Get, Set, RESP_OK};

impl CommandExecutor for Get {
    fn execute(self, backend: &crate::Backend) -> RespFrame {
        match backend.get(&self.key) {
            Some(value) => value,
            None => RespFrame::Null(RespNull),
        }
    }
}

impl CommandExecutor for Set {
    fn execute(self, backend: &crate::Backend) -> RespFrame {
        backend.set(self.key.clone(), self.value.clone());
        RESP_OK.clone()
    }
}

impl TryFrom<RespArray> for Get {
    type Error = CommandError;
    fn try_from(value: RespArray) -> Result<Self, Self::Error> {
        validate_command(&value, &["get"], 1)?;

        let mut args = extract_args(value, 1)?.into_iter();
        match args.next() {
            Some(RespFrame::BulkString(key)) => Ok(Get {
                key: String::from_utf8(key.0)?,
            }),
            _ => Err(CommandError::InvalidArgument("Invalid key".to_string())),
        }
    }
}

impl TryFrom<RespArray> for Set {
    type Error = CommandError;
    fn try_from(value: RespArray) -> Result<Self, Self::Error> {
        validate_command(&value, &["set"], 2)?;

        let mut args = extract_args(value, 1)?.into_iter();
        match (args.next(), args.next()) {
            (Some(RespFrame::BulkString(key)), Some(value)) => Ok(Set {
                key: String::from_utf8(key.0)?,
                value,
            }),
            _ => Err(CommandError::InvalidArgument(
                "Invalid key or value".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use bytes::BytesMut;

    use crate::{Backend, RespDecode};

    use super::*;

    #[test]
    fn test_get_try_from_resp_array() -> Result<()> {
        let mut buf = BytesMut::from("*2\r\n$3\r\nget\r\n$3\r\nkey\r\n");

        let frame = RespArray::decode(&mut buf)?;
        let get: Get = frame.try_into()?;
        assert_eq!(get.key, "key");
        Ok(())
    }

    #[test]
    fn test_set_try_from_resp_array() -> Result<()> {
        let mut buf = BytesMut::from("*3\r\n$3\r\nset\r\n$3\r\nkey\r\n$5\r\nvalue\r\n");

        let frame = RespArray::decode(&mut buf)?;
        let set: Set = frame.try_into()?;
        assert_eq!(set.key, "key");
        assert_eq!(set.value, RespFrame::BulkString(b"value".into()));
        Ok(())
    }

    #[test]
    fn test_set_get_command() -> Result<()> {
        let backend = Backend::new();
        let cmd = Set {
            key: "key".to_string(),
            value: RespFrame::BulkString(b"value".into()),
        };
        let result = cmd.execute(&backend);
        assert_eq!(result, RESP_OK.clone());

        let cmd = Get {
            key: "key".to_string(),
        };
        let value = cmd.execute(&backend);
        assert_eq!(value, RespFrame::BulkString(b"value".into()));
        Ok(())
    }
}
