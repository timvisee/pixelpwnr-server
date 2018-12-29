use std::{cmp, io, str, usize};

use bytes::{BufMut, BytesMut};
use tokio::codec::{Encoder, Decoder, LinesCodec};

use cmd::Cmd;

/// A `Codec` implementation that handles the pixelflut protocol.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PixCodec {
    lines: LinesCodec,
}

impl PixCodec {
    /// Construct a new pix codec.
    pub fn new() -> Self {
        // TODO: obtain the line limit number from a constant somewhere
        Self::from(LinesCodec::new_with_max_length(80))
    }

    /// Construct a new pix codec based on the given line codec.
    pub fn from(lines: LinesCodec) -> Self {
        Self {
            lines,
        }
    }
}

impl Decoder for PixCodec {
    type Item = Result<Cmd, ()>;
    // TODO: use a custom error type to describe all error cases
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Result<Cmd, ()>>, io::Error> {
        self.lines
            .decode(buf)
            .map(|line|
                line.map(|line|
                    // TODO: handle unrecognized commands
                    Cmd::decode(line.as_bytes()).map_err(|e| ())
                )
            )
    }

    // TODO: is this required, it is already provided, does that work?
    fn decode_eof(&mut self, buf: &mut BytesMut) -> Result<Option<Result<Cmd, ()>>, io::Error> {
        self.lines
            .decode_eof(buf)
            .map(|line|
                line.map(|line|{
                    println!("PARSING EOF: {}", line);

                    // TODO: handle unrecognized commands
                    Cmd::decode(line.as_bytes()).map_err(|e| ())
                })
            )
    }
}

impl Encoder for PixCodec {
    // TODO: use a custom response type
    type Item = String;
    type Error = io::Error;

    fn encode(&mut self, line: String, buf: &mut BytesMut) -> Result<(), io::Error> {
        self.lines.encode(line, buf)
    }
}
