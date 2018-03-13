# pixelpwnr render (WIP, prototype)
A blazingly fast GPU accelerated [pixelflut][pixelflut] ([video][pixelflut-video])
renderer in [Rust][rust], for use in a high performance pixelflut server.

This is just a renderer, and is intended to be implemented in a server.  
For a quick server implementation using this renderer, see:  
[→ pixelpwnr-server (server)][pixelpwnr-server]

**Note:** This is currently an experiment, and is heavily tested with.
This prototype renderer will be implemented in a quick server if successful. 

## Features
* Blazingly fast pixelflut rendering
* GPU accelerated
* Highly concurrent, to support many connections
* Linux, Windows and macOS
* ...

## Requirements
* Rust nightly (v1.24 or higher)
* Some build essentials (Ubuntu package: `build-essential`)
* `freetype2` development files (Ubuntu package: `libfreetype6-dev`)

## Relevant projects
* [pixelpwnr (client)][pixelpwnr]
* [pixelpwnr-server (server)][pixelpwnr-server]

## License
This project is released under the GNU GPL-3.0 license.
Check out the [LICENSE](LICENSE) file for more information.


[pixelflut]: https://cccgoe.de/wiki/Pixelflut
[pixelflut-video]: https://vimeo.com/92827556/
[pixelpwnr]: https://github.com/timvisee/pixelpwnr
[pixelpwnr-server]: https://github.com/timvisee/pixelpwnr-server
[rust]: https://www.rust-lang.org/
[rustup]: https://rustup.rs/
