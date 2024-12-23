use crate::{
    cmd::{CommandError, Get},
    RespArray, RespFrame, RespNull,
};

use super::{extract_args, validate_command, CommandExecutor, Set, RESP_OK};

impl CommandExecutor for Get {
    /// Executes the `Get` command on the provided backend.
    ///
    /// This function attempts to retrieve the value associated with the key
    /// stored in the `Get` command from the backend. If the key is found,
    /// the corresponding value is returned as a `RespFrame`. If the key is not
    /// found, a `RespFrame::Null` is returned.
    ///
    /// # Arguments
    ///
    /// * `backend` - A reference to the `Backend` where the key-value data is stored.
    ///
    /// # Returns
    ///
    /// A `RespFrame` containing either the value associated with the key or
    /// a null response if the key is not present.
    fn execute(self, backend: &crate::Backend) -> RespFrame {
        match backend.get(&self.key) {
            Some(value) => value,
            None => RespFrame::Null(RespNull),
        }
    }
}

impl CommandExecutor for Set {
    /// Executes the `Set` command on the provided backend.
    ///
    /// This function attempts to store the value associated with the key
    /// stored in the `Set` command in the backend. A successful store
    /// results in a `RespFrame` containing an `Ok` response.
    fn execute(self, backend: &crate::Backend) -> RespFrame {
        backend.set(self.key, self.value);
        RESP_OK.clone()
    }
}

impl TryFrom<RespArray> for Get {
    type Error = CommandError;

    /// Converts a RESP array into a `Get` command.
    ///
    /// The RESP array must have exactly 2 elements: the command name "get", and the key.
    /// The key must be a BulkString frame.
    ///
    /// If the conversion is successful, a `Get` struct with the key field set is returned.
    /// If the conversion fails, an `Err` containing the `CommandError` is returned.
    fn try_from(frame: RespArray) -> Result<Self, Self::Error> {
        validate_command(&frame, &["get"], 1)?;
        let mut args = extract_args(frame, 1)?.into_iter();
        match args.next() {
            Some(RespFrame::BulkString(key)) => Ok(Get {
                key: String::from_utf8(key.0)?,
            }),
            _ => Err(CommandError::InvalidArguments("Invalid key".to_string())),
        }
    }
}

impl TryFrom<RespArray> for Set {
    type Error = CommandError;
    /// Converts a RESP array into a `Set` command.
    ///
    /// The RESP array must have exactly 3 elements: the command name "set", the key,
    /// and the value. The key must be a BulkString frame, and the value can be any
    /// `RespFrame`.
    ///
    /// If the conversion is successful, a `Set` struct with the key and value fields
    /// set is returned. If the conversion fails, an `Err` containing the
    /// `CommandError` is returned.
    fn try_from(value: RespArray) -> Result<Self, Self::Error> {
        validate_command(&value, &["set"], 2)?;
        let mut args = extract_args(value, 1)?.into_iter();
        match (args.next(), args.next()) {
            (Some(RespFrame::BulkString(key)), Some(value)) => Ok(Set {
                key: String::from_utf8(key.0)?,
                value,
            }),
            _ => Err(CommandError::InvalidArguments(
                "Invalid key or value".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Backend, RespDecode};
    use anyhow::Result;
    use bytes::BytesMut;

    /// Tests the conversion of a RESP array into a `Get` command.
    ///
    /// This test verifies that a RESP array with the command "get" and a key
    /// "hello" is correctly parsed into a `Get` struct with the key field set
    /// to "hello".
    ///
    /// The test uses a `BytesMut` buffer to simulate a RESP array and checks
    /// the `key` field of the resulting `Get` command to ensure it matches
    /// the expected value.
    #[test]
    fn test_get_from_resp_array() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*2\r\n$3\r\nget\r\n$5\r\nhello\r\n");
        let frame = RespArray::decode(&mut buf)?;
        let result: Get = frame.try_into()?;
        assert_eq!(result.key, "hello");

        Ok(())
    }

    #[test]
    fn test_set_from_resp_array() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*3\r\n$3\r\nset\r\n$5\r\nhello\r\n$5\r\nworld\r\n");
        let frame = RespArray::decode(&mut buf)?;
        let result: Set = frame.try_into()?;

        assert_eq!(result.key, "hello");
        assert_eq!(result.value, RespFrame::BulkString(b"world".into()));

        Ok(())
    }

    #[test]
    fn test_set_get_command() -> Result<()> {
        let backend = Backend::new();
        let cmd = Set {
            key: "hello".to_string(),
            value: RespFrame::BulkString(b"world".into()),
        };
        let result = cmd.execute(&backend);
        assert_eq!(result, RESP_OK.clone());

        let cmd = Get {
            key: "hello".to_string(),
        };
        let result = cmd.execute(&backend);
        assert_eq!(result, RespFrame::BulkString(b"world".into()));

        Ok(())
    }
}
