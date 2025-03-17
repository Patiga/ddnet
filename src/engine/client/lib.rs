//! DDNet's client engine, Rust part.
//!
//! DDNet's code base is separated into three major parts, `base`, `engine` and
//! `game`.
//!
//! The engine consists of game-independent code such as display setup,
//! low-level network protocol, low-level map format, etc.
//!
//! This crate in particular corresponds to the `src/engine/client` directory
//! and contains code that is solely used in the client.

#![warn(missing_docs)]

#[cfg(test)]
extern crate ddnet_test;

mod backend;
