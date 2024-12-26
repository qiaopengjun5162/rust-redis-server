use anyhow::Result;
use futures::SinkExt;
use tokio::net::TcpStream;
use tokio_stream::StreamExt;
use tokio_util::codec::{Decoder, Encoder, Framed};
use tracing::info;

use crate::{
    cmd::{Command, CommandExecutor},
    Backend, RespDecode, RespEncode, RespError, RespFrame,
};

#[derive(Debug)]
struct RespFrameCodec;

#[derive(Debug)]
struct RedisRequest {
    frame: RespFrame,
    backend: Backend,
}

#[derive(Debug)]
struct RedisResponse {
    frame: RespFrame,
}

pub async fn stream_handler(stream: TcpStream, backend: Backend) -> Result<()> {
    // how to get a frame from the stream?
    let mut framed = Framed::new(stream, RespFrameCodec);
    loop {
        match framed.next().await {
            Some(Ok(frame)) => {
                info!("Received request: {:?}", frame);
                // handle the frame
                let request = RedisRequest {
                    frame,
                    backend: backend.clone(),
                };
                let response = request_handler(request).await?;
                info!("Received response: {:?}", response);
                // send the response to the stream
                framed.send(response.frame).await?;
            }
            Some(Err(e)) => return Err(e),

            None => return Ok(()),
        }
    }
}

/// Handles a single Redis request by executing the command and returning the response.
///
/// # Parameters
///
/// * `request`: A `RedisRequest` struct containing the incoming request frame and the backend to execute the command.
///
/// # Returns
///
/// * `Result<RedisResponse, anyhow::Error>`: On success, returns a `RedisResponse` containing the response frame.
///   On error, returns an `anyhow::Error` containing the error details.
async fn request_handler(request: RedisRequest) -> Result<RedisResponse, anyhow::Error> {
    let (frame, backend) = (request.frame, request.backend);
    // let cmd: Command = frame.try_into()?;
    let cmd = Command::try_from(frame)?;
    info!("Executing command: {:?}", cmd);
    let frame = cmd.execute(&backend);
    Ok(RedisResponse { frame })
}

impl Encoder<RespFrame> for RespFrameCodec {
    type Error = anyhow::Error;

    /// Encodes a `RespFrame` into a byte buffer.
    ///
    /// # Parameters
    ///
    /// * `item`: The `RespFrame` to encode.
    /// * `dst`: A mutable reference to a `BytesMut` buffer where the encoded data will be written.
    ///
    /// # Returns
    ///
    /// * `Result<(), Self::Error>`: On success, returns `Ok(())`. On error, returns a `Self::Error`.
    fn encode(&mut self, item: RespFrame, dst: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        let encoded = item.encode();
        dst.extend_from_slice(&encoded);
        Ok(())
    }
}

impl Decoder for RespFrameCodec {
    type Item = RespFrame;
    type Error = anyhow::Error;

    /// Decodes a `RespFrame` from a byte buffer.
    ///
    /// # Parameters
    ///
    /// * `src`: A mutable reference to a `BytesMut` buffer containing the encoded data.
    ///
    /// # Returns
    ///
    /// * `Result<Option<RespFrame>>`: On success, returns `Ok(Some(frame))`. If the input is incomplete, returns `Ok(None)`. On error, returns a `Self::Error`.
    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<RespFrame>> {
        match RespFrame::decode(src) {
            Ok(frame) => Ok(Some(frame)),
            Err(RespError::NotComplete) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
