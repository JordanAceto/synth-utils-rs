# Rust Synth-Utils [![Build](https://github.com/JordanAceto/synth-utils-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/JordanAceto/synth-utils-rs/actions/workflows/ci.yml)


## Lightweight modulation sources and utilities for music synthesizers

This crate does not provide any sound generation, only control and modulation sources.

The intended use is in small real time embedded applications. For example, a desktop synthesizer with analog signal path
and digital envelopes, LFOs, and MIDI-to-CV.

## Demos

### Cargo examples

The [`examples`](https://github.com/JordanAceto/synth-utils-rs/tree/main/examples) folder contains demos that can be run with `cargo run --example [filename without .rs extension]`

for example `cargo run --example adsr_plot`. Some of these use the [`plotters`](https://crates.io/crates/plotters) crate to generate plots of the various utilities.

### Embedded

Within the main examples directory is [`stm32f103_examples/examples`](https://github.com/JordanAceto/synth-utils-rs/tree/main/examples/stm32f103_examples/examples), a subdirectory dedicated to microcontroller based examples. These have been tested on an STM32F103 nucleo board.

## Contributing

[Pull requests](https://github.com/JordanAceto/synth-utils-rs/pulls) and [bug reports](https://github.com/JordanAceto/synth-utils-rs/issues) are very welcome. Suggestions and constructive criticism are welcomed as well.
