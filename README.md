# pixelpwnr server

A blazingly fast GPU accelerated [pixelflut][pixelflut] ([video][pixelflut-video])
server in [Rust][rust].

## Features

* Blazingly fast pixelflut rendering
* GPU accelerated
* Highly concurrent, to support many connections
* Linux, Windows and macOS
* Optional binary PX command for reduced bandwidth requirements (enabled by default).

## Installation

For installation, Git and Rust cargo are required.
Install the latest version of Rust with [rustup][rustup].

Then, clone and install `pixelpwnr-server` with:
```bash
# Clone the project
git clone https://github.com/timvisee/pixelpwnr-server.git
cd pixelpwnr-server

# Install pixelpwnr server
cargo install --path server -f

# Start using pixelpwnr server
pixelpwnr-server --help

# or run it directly from Cargo
cargo run --bin pixelpwnr-server --release -- --help
```

Or just build it and invoke the binary directly (Linux/macOS):
```bash
# Clone the project
git clone https://github.com/timvisee/pixelpwnr-server.git
cd pixelpwnr-server

# Build the project (release version)
cargo build --release

# Start using pixelpwnr-server
./target/release/pixelpwnr-server --help
```
## The binary PX command

This implementation adds a new command to the protocol. 

This type of command is enabled by default, but can be disabled by passing the `--no-binary` flag to `pixelflut-server` when running the exectuable.

The command is laid out as follows:

```
PBxyrgba
```

where:
* `x` and `y` are Little-Endian u16 values describing the X and Y coordinate of the pixel to set.
* `r`, `g`, `b` and `a` are single-byte values describing the R, G, B, and A components of the color to set the pixel to.
* It is important to note that this command does _not_ end in a newline. Appending a newline simply causes the server to interpret that newline as an empty command (which is fine).

## Requirements

* Rust (MSRV v1.58.1 or higher)
* Build essentials (Ubuntu package: `build-essential`)
* `freetype2` development files (Ubuntu package: `libfreetype6-dev`)

## Performance

Here are some points that help with the pixelflut server performance,
under heavy load:

- Use a `--release` build.
- Use a CPU with as many cores as possible.
- Use a fast Ethernet connection, preferably 10Gb/s+.
- Use a dedicated graphics card.
- Use a Linux machine.
- Increase the [file descriptor limit][filedescriptorlimit] (on Linux).
- Quit as many other running programs.

## Relevant projects

- [pixelpwnr][pixelpwnr]: client to flut (animated) images
- [pixelpwnr-cast][pixelpwnr-cast]: cast your screen to a pixelflut server

## License
This project is released under the GNU GPL-3.0 license.
Check out the [LICENSE](LICENSE) file for more information.


[filedescriptorlimit]: https://unix.stackexchange.com/questions/84227/limits-on-the-number-of-file-descriptors
[pixelflut]: https://cccgoe.de/wiki/Pixelflut
[pixelflut-video]: https://vimeo.com/92827556/
[pixelpwnr]: https://github.com/timvisee/pixelpwnr
[pixelpwnr-cast]: https://github.com/timvisee/pixelpwnr-cast
[rust]: https://www.rust-lang.org/
[rustup]: https://rustup.rs/
