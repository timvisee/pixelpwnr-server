# pixelpwnr server (WIP, prototype)
A blazingly fast GPU accelerated [pixelflut][pixelflut] ([video][pixelflut-video])
server in [Rust][rust].

**Note:** This is currently an experiment, and is heavily tested with.

## Features
* Blazingly fast pixelflut rendering
* GPU accelerated
* Highly concurrent, to support many connections
* Linux, Windows and macOS
* ...

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
* Rust nightly (v1.24 or higher)

## Relevant projects
* [pixelpwnr (client)][pixelpwnr]
* [pixelpwnr-render (GPU rendering backend)][pixelpwnr-render]

## License
This project is released under the GNU GPL-3.0 license.
Check out the [LICENSE](LICENSE) file for more information.


[pixelflut]: https://cccgoe.de/wiki/Pixelflut
[pixelflut-video]: https://vimeo.com/92827556/
[pixelpwnr]: https://github.com/timvisee/pixelpwnr
[pixelpwnr-render]: https://github.com/timvisee/pixelpwnr-render
[rust]: https://www.rust-lang.org/
[rustup]: https://rustup.rs/
