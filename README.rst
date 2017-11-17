RuNES
=====

As we know, there have been a ton of NES emulators implementation in various
kinds of languages (mostly C). All of these emulators come with different
accuracy and portability. RuNES is an attempt to implement a reasonably
accurate (instruction level accurate), light-weight and efficient emulation
core library written in Rust. Unlike sprocketnes_ or pinky_, RuNES strives to
provide with a clean and minimal core library without standard library (i.e.,
without Box, Rc, Vectors, etc.) that could be compiled and easily ported to
embedded environments. Of course, a simple SDL-based GUI is implemented as a
demonstration of use.

Feature
=======

- Core library with minimal use of the Rust core crate, and zero use of std.
- Support standard 6502 instruction set (unofficial instruction will be
  considered in the future).

- Instruction-level accuracy with accurate CPU/PPU timing.

Guidelines
==========

- Never use std in the core library.
- At the same time, avoid ``unsafe`` as much as possible, but use it at
  discretion to improve performance: remove unnecessary checks within a very
  localized context.

- Keep the core library code base minimal.

.. _sprocketnes: https://github.com/pcwalton/sprocketnes
.. _pinky: https://github.com/koute/pinky

Build
=====

::
    cargo build --release
