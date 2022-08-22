use super::*;

const CODEC_OPTS: CodecOptions = CodecOptions {
    rate_limit: None,
    allow_binary_cmd: true,
};

#[tokio::test]
async fn response_newline() {
    let stats = Arc::new(Stats::new());
    let pixmap = Arc::new(Pixmap::new(400, 800));

    let mut test = tokio_test::io::Builder::new()
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

    let test = Pin::new(&mut test);

    let lines = Lines::new(test, stats.clone(), pixmap.clone(), CODEC_OPTS);

    lines.await;
}
