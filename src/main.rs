extern crate bytes;
extern crate futures;
extern crate pixelpwnr_render;
extern crate tokio;
extern crate tokio_io;
extern crate tokio_proto;
extern crate tokio_service;

use std::io;

use bytes::BytesMut;
use futures::future;
use futures::future::FutureResult;
use pixelpwnr_render::Color;
use pixelpwnr_render::Pixmap;
use pixelpwnr_render::Renderer;
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::codec::{Framed, Encoder, Decoder};
use tokio_proto::TcpServer;
use tokio_proto::pipeline::ServerProto;
use tokio_service::Service;

struct PixCodec;

impl Encoder for PixCodec {
  type Item = String;
  type Error = io::Error;

  fn encode(&mut self, msg: String, buf: &mut BytesMut) -> io::Result<()> {
      buf.extend(msg.as_bytes());
      buf.extend(b"\n");
      Ok(())
  }
}

impl Decoder for PixCodec {
  type Item = String;
  type Error = io::Error;

  fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<String>> {
      if let Some(i) = buf.iter().position(|&b| b == b'\n') {
          // remove the serialized frame from the buffer.
          let line = buf.split_to(i);

          // Also remove the '\n'
          buf.split_to(1);

          // Turn this data into a UTF string and return it in a Frame.
          match std::str::from_utf8(&line) {
              Ok(s) => Ok(Some(s.to_string())),
              Err(_) => Err(io::Error::new(io::ErrorKind::Other,
                                           "invalid UTF-8")),
          }
      } else {
          Ok(None)
      }
  }
}

struct PixProto;

impl<T: AsyncRead + AsyncWrite + 'static> ServerProto<T> for PixProto {
    type Request = String;
    type Response = String;
    type Transport = Framed<T, PixCodec>;
    type BindTransport = Result<Self::Transport, io::Error>;

    fn bind_transport(&self, io: T) -> Self::BindTransport {
        Ok(io.framed(PixCodec))
    }
}

struct Echo;

impl Service for Echo {
    type Request = String;
    type Response = String;
    type Error = io::Error;
    type Future = FutureResult<Self::Response, Self::Error>;

    fn call(&self, req: Self::Request) -> Self::Future {
        future::ok(req)
    }
}

fn main() {
    // Specify the localhost address
    let addr = "0.0.0.0:8080".parse().unwrap();

    // The builder requires a protocol and an address
    let server = TcpServer::new(PixProto, addr);

    // We provide a way to *instantiate* the service for each new
    // connection; here, we just immediately return a new instance.
    server.serve(|| Ok(Echo));

	render();
}

fn render() {
    // Build a pixelmap
    let mut pixmap = Pixmap::new(800, 600);
    pixmap.set_pixel(10, 10, Color::from_rgb(255, 0, 0));
    pixmap.set_pixel(20, 40, Color::from_rgb(0, 255, 0));

    // Build and run the renderer
    let mut renderer = Renderer::new(&pixmap);
    renderer.run();
}
