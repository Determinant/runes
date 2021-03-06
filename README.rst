RuNES
=====

.. image:: https://img.shields.io/crates/v/runes.svg
   :target: https://github.com/Determinant/runes
.. image:: https://img.shields.io/crates/l/runes.svg
   :target: https://github.com/Determinant/runes

As we know, there have been a ton of NES emulator implementations in various
kinds of languages (mostly C). All of these emulators come with different
accuracy and portability. RuNES is an attempt to build a reasonably accurate
(cycle-level accurate), light-weight and efficient emulation core library using
Rust. Unlike sprocketnes_ or pinky_, RuNES strives to provide with a clean and
minimal core library without standard library (i.e., without Box, Rc, Vectors,
etc.) that could be compiled and easily ported to embedded environments. Of
course, a minimal but usable SDL-based GUI is also provided as a demonstration
of use.

Feature
=======

- Core library with minimal use of the Rust core crate, and zero use of std.
- Lightweight and clean code base.
- Support standard 6502 instruction set (unofficial instruction will be
  considered in the future).
- Load/save the machine state.
- Cycle-level accuracy (in-progress).

Guidelines
==========

- Never use std in the core library.
- At the same time, avoid ``unsafe`` as much as possible, but use it at
  discretion to improve performance: remove unnecessary checks within a very
  localized context.

- Keep the core library code base minimal.

.. _sprocketnes: https://github.com/pcwalton/sprocketnes
.. _pinky: https://github.com/koute/pinky

Build the Example Emulator
==========================

GNU/Linux
---------

.. code-block:: sh

    # for Ubuntu
    # install Rust toolchain (https://www.rustup.rs/):
    # $ sudo apt install curl gcc
    # $ curl https://sh.rustup.rs -sSf | sh
    # and add $HOME/.cargo/bin to your $PATH
    # install SDL2 by:
    # $ sudo apt install libsdl2-dev        # install SDL2 library

    git clone https://github.com/Determinant/runes.git
    cd runes
    cargo build --examples --release        # build the binary
    target/release/examples/runes --help    # see the help message
    
Mac OS X
--------

.. code-block:: sh

    # install homebrew
    /usr/bin/ruby -e "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/master/install)"
    # install Xcode command line tools
    xcode-select --install
    # install SDL2
    brew install sdl2
    # rust up
    curl https://sh.rustup.rs -sSf | sh
    # build RuNES and enjoy
    git clone https://github.com/Determinant/runes.git
    cargo build --examples --release
