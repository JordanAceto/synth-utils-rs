#![no_std]
#![doc = include_str!("../README.md")]

pub mod adsr;
pub mod lfo;
mod lookup_tables;
pub mod mono_midi_receiver;
mod phase_accumulator;
mod utils;
