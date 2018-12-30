use std::io;

use app::LINE_LENGTH_MAX;
use bytes::BytesMut;
use tokio::codec::{Encoder, Decoder, LinesCodec};

use cmd::{Request, RequestResult, Response};

/// A `Codec` implementation that handles the pixelflut protocol.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PixCodec {
    lines: LinesCodec,
}

impl PixCodec {
    /// Construct a new pix codec.
    pub fn new() -> Self {
        Self::from(LinesCodec::new_with_max_length(LINE_LENGTH_MAX))
    }

    /// Construct a new pix codec based on the given line codec.
    pub fn from(lines: LinesCodec) -> Self {
        Self {
            lines,
        }
    }
}

impl Decoder for PixCodec {
    type Item = RequestResult;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<RequestResult>, io::Error> {
        self.lines
            .decode(buf)
            .map(|line|
                line.map(|line| Request::decode(line.as_bytes()))
            )
    }

    // TODO: is this required, it is already provided, does that work?
    fn decode_eof(&mut self, buf: &mut BytesMut) -> Result<Option<RequestResult>, io::Error> {
        self.lines
            .decode_eof(buf)
            .map(|line|
                line.map(|line| Request::decode(line.as_bytes()))
            )
    }
}

impl Encoder for PixCodec {
    type Item = Response;
    type Error = io::Error;

    fn encode(&mut self, response: Response, buf: &mut BytesMut) -> Result<(), io::Error> {
        self.lines.encode(response.to_string(), buf)
    }
}
