# GeeBee

![Pipeline](https://gitlab.com/chaosteil/geebee-rs/badges/master/pipeline.svg)

A barebones gameboy emulator written in Rust.

![Screenshot](https://gitlab.com/chaosteil/geebee-rs/-/raw/master/screenshot.png)

This emulator is in no shape or form attempting to be correct - merely good enough. This is a learning project for the fun of it.

## How to

To build and run, use cargo:

```sh
$ cargo run -- -r path/to/rom.gbc
```

## Controls

* `WASD` for directional pad
* `N` & `M` for `B` & `A` respectively
* `Z` for `Start` and `X` for `Select`.

## What is done

* This emulator will successfully load bootroms for both DMG and CGB, and even somewhat play games.
* Tested under Windows and Linux.
* CPU instructions and instruction timings tests from Blargg's hardware test ROMs pass.
* There are still some rendering bugs associated with CGB mode.
* Some memory mapping issues still need to be ironed out.
