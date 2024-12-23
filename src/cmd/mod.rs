mod hmap;
mod map;

use crate::{Backend, RespArray, RespError, RespFrame, SimpleString};
use enum_dispatch::enum_dispatch;
use lazy_static::lazy_static;
use thiserror::Error;

lazy_static! {
    static ref RESP_OK: RespFrame = SimpleString::new("OK").into();
}

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Invalid command: {0}")]
    InvalidCommand(String),
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
    #[error("{0}")]
    RespError(#[from] RespError),
    #[error("Utf8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

#[enum_dispatch]
pub trait CommandExecutor {
    fn execute(self, backend: &Backend) -> RespFrame;
}

#[enum_dispatch(CommandExecutor)]
pub enum Command {
    Get(Get),
    Set(Set),
    HGet(HGet),
    HSet(HSet),
    HGetAll(HGetAll),
}

#[derive(Debug)]
pub struct Get {
    key: String,
}

#[derive(Debug)]
pub struct Set {
    key: String,
    value: RespFrame,
}

#[derive(Debug)]
pub struct HGet {
    key: String,
    field: String,
}

#[derive(Debug)]
pub struct HSet {
    key: String,
    field: String,
    value: RespFrame,
}

#[derive(Debug)]
pub struct HGetAll {
    key: String,
}

impl TryFrom<RespArray> for Command {
    type Error = CommandError;

    fn try_from(frame: RespArray) -> Result<Self, Self::Error> {
        match frame.first() {
            Some(RespFrame::BulkString(ref cmd)) => match cmd.as_ref() {
                b"get" => Ok(Get::try_from(frame)?.into()),
                b"set" => Ok(Set::try_from(frame)?.into()),
                b"hget" => Ok(HGet::try_from(frame)?.into()),
                b"hset" => Ok(HSet::try_from(frame)?.into()),
                b"hgetall" => Ok(HGetAll::try_from(frame)?.into()),
                _ => Err(CommandError::InvalidCommand(format!(
                    "Invalid command: {}",
                    String::from_utf8_lossy(cmd.as_ref())
                ))),
            },
            _ => Err(CommandError::InvalidCommand(
                "Command must have a BulkString as the first argument".to_string(),
            )),
        }
    }
}

/// Validate a RESP array as a command.
///
/// The RESP array must have length `names.len() + n_args`, and the first `names.len()` elements
/// must be BulkString frames that match the given command names case-insensitively.
///
/// If the validation fails, a `CommandError` is returned explaining the error.
fn validate_command(
    value: &RespArray,
    names: &[&'static str],
    n_args: usize,
) -> Result<(), CommandError> {
    if value.len() != n_args + names.len() {
        return Err(CommandError::InvalidArguments(format!(
            "{} command must have exactly {} argument",
            names.join(" "),
            n_args
        )));
    }

    for (i, name) in names.iter().enumerate() {
        match value[i] {
            RespFrame::BulkString(ref cmd) => {
                if cmd.as_ref().to_ascii_lowercase() != name.as_bytes() {
                    return Err(CommandError::InvalidCommand(format!(
                        "Invalid command: expected {}, got {}",
                        name,
                        String::from_utf8_lossy(cmd.as_ref())
                    )));
                }
            }
            _ => {
                return Err(CommandError::InvalidCommand(
                    "Command must have a BulkString as the first argument".to_string(),
                ))
            }
        }
    }
    Ok(())
}

/// Extract arguments from a RESP array.
///
/// `start` is the index of the first argument. All elements from `start` to the end of the array
/// are collected into a `Vec<RespFrame>`.
///
/// # Example
///
/// let array = RespArray(vec![RespFrame::BulkString("HSET".into()), RespFrame::BulkString("key".into()), RespFrame::BulkString("field".into()), RespFrame::BulkString("value".into())]);
/// let args = extract_args(array, 1).unwrap();
/// assert_eq!(args, vec![RespFrame::BulkString("key".into()), RespFrame::BulkString("field".into()), RespFrame::BulkString("value".into())])
fn extract_args(value: RespArray, start: usize) -> Result<Vec<RespFrame>, CommandError> {
    Ok(value.0.into_iter().skip(start).collect::<Vec<RespFrame>>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RespDecode, RespNull};
    use anyhow::Result;
    use bytes::BytesMut;

    #[test]
    fn test_extract_args() {
        let array = RespArray(vec![
            RespFrame::BulkString("HSET".into()),
            RespFrame::BulkString("key".into()),
            RespFrame::BulkString("field".into()),
            RespFrame::BulkString("value".into()),
        ]);
        let args = extract_args(array, 1).unwrap();
        assert_eq!(
            args,
            vec![
                RespFrame::BulkString("key".into()),
                RespFrame::BulkString("field".into()),
                RespFrame::BulkString("value".into())
            ]
        );
    }

    #[test]
    fn test_command() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*2\r\n$3\r\nget\r\n$5\r\nhello\r\n");
        let frame = RespArray::decode(&mut buf)?;
        let cmd: Command = frame.try_into()?;
        let backend = Backend::new();
        let ret = cmd.execute(&backend);
        assert_eq!(ret, RespFrame::Null(RespNull));
        Ok(())
    }
}
