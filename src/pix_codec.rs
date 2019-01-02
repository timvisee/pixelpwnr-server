use std::io::{
    Error as IoError,
    Result as IoResult,
};

use app::LINE_LENGTH_MAX;
use bytes::BytesMut;
use tokio::codec::{Encoder, Decoder};

use cmd::{Request, RequestResult, Response};

/// A `Codec` implementation that handles the pixelflut protocol.
#[derive(Clone, Debug)]
pub struct PixCodec;

impl PixCodec {
    /// Construct a new pix codec.
    pub fn new() -> Self {
        Self
    }
}

impl Decoder for PixCodec {
    type Item = RequestResult;
    type Error = IoError;

    fn decode(&mut self, buf: &mut BytesMut) -> IoResult<Option<RequestResult>> {
        if let Some(i) = buf.iter().position(|&b| b == b'\n') {
            // Split the line of the buffer, take a slice without the newline
            let line = buf.split_to(i + 1);
            let line = &line[..line.len() - 1];

            Ok(Some(Request::decode(line)))
        } else if buf.len() > LINE_LENGTH_MAX { // longest possible command
            // Err(ErrorKind::LineTooLong.into())
            // TODO: report line too long error
            eprintln!("LINE TOO LONG!");
            // TODO: consume bytes
            Ok(None)
        } else {
            Ok(None)
        }
    }
}

impl Encoder for PixCodec {
    type Item = Response;
    type Error = IoError;

    fn encode(&mut self, command: Response, buf: &mut BytesMut) -> IoResult<()> {
        buf.extend(format!("{}\n", command.to_string()).as_bytes());
        Ok(())
    }
}
