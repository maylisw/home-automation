# ESP32

A collection of projects and utils for ESP32 boards written in Rust.

## Install

```bash
brew install cmake ninja dfu-util ccache
cargo install ldproxy cargo-generate espflash

rustup toolchain install nightly --component rust-src
rustup component add rustc
```

## Setup

Initial setup was done using a template as per the [esp-rs book](https://docs.esp-rs.org/book/writing-your-own-application/generate-project/index.html).
This was then adapted to act as a single large Rust workspace following the advice
in [Large Rust Workspaces](https://matklad.github.io/2021/08/22/large-rust-workspaces.html)

```bash
cargo generate esp-rs/esp-idf-template cargo
```

## Boards

- ESP32-C6-DevKitC-1-N8 - 8MB SPI Flash
