#![no_std]

//! TunerStudio Protocol Implementation
//!
//! This crate implements the Speeduino-compatible protocol for communication
//! with TunerStudio. It's no_std compatible for use on Arduino AVR platforms.

pub mod och_block;
pub mod commands;
pub mod types;

pub use och_block::OchBlock;
pub use commands::{Command, SIGNATURE, VERSION};
pub use types::*;
