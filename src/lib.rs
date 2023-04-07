#![no_std]
#![doc = include_str!("../README.md")]

pub mod adsr;
pub mod glide_processor;
pub mod lfo;
mod lookup_tables;
pub mod mono_midi_receiver;
mod phase_accumulator;
pub mod ribbon_controller;
mod utils;
