# process-viewer [![Build Status](https://travis-ci.org/GuillaumeGomez/process-viewer.png?branch=master)](https://travis-ci.org/GuillaumeGomez/process-viewer)
A process viewer GUI in rust. It provides current status of your processes (cpu and memory usage) and your system (usage of every core and of your RAM, and the temperature of your components if this information is available).

It can be run on the following platforms:

 * Linux
 * Raspberry
 * macOS
 * FreeBSD
 * Windows (for cross-compilation to Windows, you can give a try to https://hub.docker.com/r/etrombly/rust-crosscompile)

Please run it in release mode to have good performance:

```bash
cargo run --release
```

or to install it as binary

```bash
cargo install process_viewer
```

### Building/running on Linux, MacOS

Take a look at the [gtk-rs installation guide](https://gtk-rs.org/gtk4-rs/stable/latest/book/installation.html) to know how install GTK dependencies.

### Running on Raspberry

It'll be difficult to build on Raspberry pi directly. A good way-around is to be build on Linux before sending it to your Raspberry pi:

```bash
rustup target add armv7-unknown-linux-gnueabihf
cargo build --target=armv7-unknown-linux-gnueabihf
```

## Donations

If you appreciate my work and want to support me, you can do it here:

[![Become a patron](https://c5.patreon.com/external/logo/become_a_patron_button.png)](https://www.patreon.com/GuillaumeGomez)

## Screenshots

![screenshot](http://guillaume-gomez.fr/image/process-viewer-screen1.png)
![screenshot](http://guillaume-gomez.fr/image/process-viewer-screen2.png)
