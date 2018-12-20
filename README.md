# pixelpwnr server (WIP, prototype)

**Note:** This project is in the prototype phase,
and is heavily in development and tested with.
Further optimization of this server for high-performance
use will be done at a later time.

A blazingly fast GPU accelerated [pixelflut][pixelflut] ([video][pixelflut-video])
server in [Rust][rust].

## Features
* Blazingly fast pixelflut rendering
* GPU accelerated
* Highly concurrent, to support many connections
* Linux, Windows and macOS
* ...

## Current problems
In the current prototype version, the following main problems exist:

* Windows connectivity doesn't work
* Connections not explicitly closed aren't dropped, as there is no timeout

These should be fixed for the first release.

## Installation
For installation, Git and Rust cargo are required.
Install the latest version of Rust with [rustup][rustup].

Then, clone and install `pixelpwnr-server` with:
```bash
# Clone the project
git clone https://github.com/timvisee/pixelpwnr-server.git
cd pixelpwnr-server

# Install pixelpwnr server
cargo install -f

# Start using pixelpwnr server
pixelpwnr-server --help

# or run it directly from Cargo
cargo run --release -- --help
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

## Requirements
* Rust (v1.31 or higher)
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
* [pixelpwnr (client)][pixelpwnr]
* [pixelpwnr-render (GPU rendering backend)][pixelpwnr-render]

## License
This project is released under the GNU GPL-3.0 license.
Check out the [LICENSE](LICENSE) file for more information.


[filedescriptorlimit]: https://unix.stackexchange.com/questions/84227/limits-on-the-number-of-file-descriptors
[pixelflut]: https://cccgoe.de/wiki/Pixelflut
[pixelflut-video]: https://vimeo.com/92827556/
[pixelpwnr]: https://github.com/timvisee/pixelpwnr
[pixelpwnr-render]: https://github.com/timvisee/pixelpwnr-render
[rust]: https://www.rust-lang.org/
[rustup]: https://rustup.rs/
