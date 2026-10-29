// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

//! The scene renderer as a library, for the Qt program to draw through.
//!
//! `ffi` is the whole of what C++ sees; the rest is public for the examples,
//! which draw into a file rather than onto a desktop.

pub mod audio;
pub mod ffi;
pub mod particles;
pub mod render;
pub mod script;
pub mod sounds;
pub mod texture;
pub mod transform;
pub mod video;
