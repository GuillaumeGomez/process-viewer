# process-viewer [![Build Status](https://travis-ci.org/GuillaumeGomez/process-viewer.png?branch=master)](https://travis-ci.org/GuillaumeGomez/process-viewer)
A process viewer GUI in rust. It provides current status of your processes (cpu and memory usage) and your system (usage of every core and of your RAM, and the temperature of your components if this information is available).

## WARNING!
For now, it only builds on rust __nightly__!

## OSX Build Instructions
```sh
# Install dependencies with homebrew
brew install glib
brew install cairo
brew install pango
brew install atk
brew install gdk-pixbuf
brew install gtk+3

# Building and running
cargo build
cargo run
```


![screenshot](http://guillaume-gomez.fr/image/screen1.png)
![screenshot](http://guillaume-gomez.fr/image/screen2.png)

It can be run on the following platforms:

 * Linux
 * Mac OSX


## Donations

If you appreciate my work and want to support me, you can do it here:

[![Become a patron](https://c5.patreon.com/external/logo/become_a_patron_button.png)](https://www.patreon.com/GuillaumeGomez)
