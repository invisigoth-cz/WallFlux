//! The scene renderer as a library, for the Qt program to draw through.
//!
//! `ffi` is the whole of what C++ sees; the rest is public for the examples,
//! which draw into a file rather than onto a desktop.

pub mod ffi;
pub mod particles;
pub mod render;
pub mod texture;
pub mod transform;
