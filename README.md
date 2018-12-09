# process-viewer [![Build Status](https://travis-ci.org/GuillaumeGomez/process-viewer.png?branch=master)](https://travis-ci.org/GuillaumeGomez/process-viewer)
A process viewer GUI in rust. It provides current status of your processes (cpu and memory usage) and your system (usage of every core and of your RAM, and the temperature of your components if this information is available).

It can be run on the following platforms:

 * Linux
 * Raspberry
 * Mac OSX
 * Windows

Please run it in release mode to have good performance:

```bash
cargo run --release
```

### Building/running on Linux, MacOS and Ubuntu-based Distros

Running ```process-viewer``` on Gnome-based Ubuntu (>=17.10) should work out of the box.  
For Debian, Ubuntu-derivatives, Fedora and MacOS refer to the [gtk-rs installation guide](http://gtk-rs.org/docs/requirements.html).

### Building/running on Windows

You'll need to follow the [gtk-rs installation guide](http://gtk-rs.org/docs/requirements.html#windows). If you still have issues to run the generated binary, just copy the `.dll`s into the executable's folder.

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

![screenshot](http://guillaume-gomez.fr/image/screen1.png)
![screenshot](http://guillaume-gomez.fr/image/screen2.png)
