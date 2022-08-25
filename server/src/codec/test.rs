use tokio_test::io::Builder;

use super::*;

const CODEC_OPTS: CodecOptions = CodecOptions {
    rate_limit: None,
    allow_binary_cmd: true,
};

async fn run<T>(lines: T, opts: Option<CodecOptions>)
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    let stats = Arc::new(Stats::new());
    let pixmap = Arc::new(Pixmap::new(400, 800));

    let lines = Box::pin(lines);

    let lines = Lines::new(lines, stats, pixmap, opts.unwrap_or(CODEC_OPTS));

    lines.await;
}

#[tokio::test]
async fn response_newline() {
    let test = Builder::new()
        // Test all commands that require a response
        .read(b"PX 16 16\r\n")
        .write(b"PX 16 16 000000\r\n")
        .read(b"SIZE\r\n")
        .write(b"SIZE 400 800\r\n")
        .read(b"HELP\r\n")
        .write(format!("{}\r\n", Cmd::help_list(&CODEC_OPTS)).as_bytes())
        // Test different variations of newlines
        .read(b"PX 16 16\n")
        .write(b"PX 16 16 000000\r\n")
        // Verify that adding a few whitespaces after the command doesn't make a difference
        .read(b"PX 16 16                     \n")
        .write(b"PX 16 16 000000\r\n")
        // Using an out of bounds index should return an error
        .read(b"PX 1000 0\r\n")
        .write(b"ERR x coordinate out of bound\r\n")
        .build();

    run(test, None).await;
}

#[tokio::test]
async fn binary_command() {
    let test = Builder::new()
        // Verify the size
        .read(&[b'P', b'B', 5, 0, 5, 0, 0xAB, 0xCD, 0xEF, 0xFF])
        .read(b"PX 5 5\n")
        .write(b"PX 5 5 ABCDEF\r\n")
        .build();

    run(test, None).await;
}

#[tokio::test]
async fn binary_command_with_binopt() {
    let codec_opts = Some(CodecOptions {
        allow_binary_cmd: false,
        rate_limit: None,
    });

    let test = Builder::new()
        // Note: we need the `\n` so that the program will detect that a command has
        // been passed in, as binary commands are supposed to be disabled.
        .read(&[b'P', b'B', 5, 0, 5, 0, 0xAB, 0xCD, 0xEF, 0xFF, b'\n'])
        .write(b"ERR")
        .build();

    run(test, codec_opts).await;
}
