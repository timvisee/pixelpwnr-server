use std::{pin::Pin, sync::Arc};

use pixelpwnr_render::Pixmap;

use crate::{
    codec::{CodecOptions, Lines},
    stats::Stats,
};

pub fn tokio_synthetic_client(pixmap: Arc<Pixmap>, stats: Arc<Stats>, opts: CodecOptions) {
    use std::io::Error;
    use std::task::{Context, Poll};

    let mut buffer = Vec::new();

    for x in 0..800u16 {
        for y in 0..600u16 {
            buffer.extend_from_slice(b"PB");
            buffer.extend_from_slice(&x.to_le_bytes());
            buffer.extend_from_slice(&y.to_le_bytes());
            buffer.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
        }
    }

    struct RepeatWriter {
        cursor: usize,
        buffer: Vec<u8>,
    }

    impl tokio::io::AsyncRead for RepeatWriter {
        fn poll_read(
            mut self: Pin<&mut Self>,
            _: &mut Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            assert!(buf.filled().len() == 0);

            let mut left_to_write = buf.capacity();

            while left_to_write != 0 {
                let cursor = self.cursor;
                let to_write = &self.buffer[cursor..];

                let to_write_len = left_to_write.min(self.buffer.len() - cursor);

                buf.put_slice(&to_write[..to_write_len]);
                self.as_mut().cursor = (cursor + to_write_len) % self.buffer.len();

                left_to_write -= to_write_len;
            }

            Poll::Ready(Ok(()))
        }
    }

    impl tokio::io::AsyncWrite for RepeatWriter {
        fn poll_write(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize, Error>> {
            println!("{}", String::from_utf8_lossy(buf));
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Error>> {
            Poll::Ready(Ok(()))
        }
    }

    let mut repeat_writer = RepeatWriter { cursor: 0, buffer };

    tokio::spawn(async move {
        let socket = Pin::new(&mut repeat_writer);

        // Wrap the socket with the Lines codec,
        // to interact with lines instead of raw bytes
        let mut lines_val = Lines::new(socket, stats.clone(), pixmap, opts);
        let lines = Pin::new(&mut lines_val);

        let result = lines.await;

        println!("{result}");
    });
}

pub fn sync_synthetic_client(pixmap: Arc<Pixmap>, stats: Arc<Stats>, opts: CodecOptions) {
    let mut buffer = Vec::new();

    for x in 0..800usize {
        for y in 0..600usize {
            buffer.push(crate::cmd::Cmd::SetPixel(
                x,
                y,
                pixelpwnr_render::Color::new(0xFFFF_FFFF),
            ));
        }
    }

    std::thread::spawn(move || {
        // Wrap the socket with the Lines codec,
        // to interact with lines instead of raw bytes
        let lines = Lines::new(buffer.as_ref(), stats.clone(), pixmap, opts);

        let result = lines.blast();

        println!("{result:?}");
    });
}
