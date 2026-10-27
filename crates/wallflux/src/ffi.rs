// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

//! The C interface the Qt program draws a scene through.
//!
//! Four calls, matching `include/wallflux_scene.h`: load a wallpaper, tell it
//! where the mouse is, draw it, let it go. Every one of them is made on Qt's
//! render thread with Qt's OpenGL context current, and none of them may let
//! a panic out -- unwinding across `extern "C"` is undefined behaviour, and
//! these wallpapers are other people's shaders off the internet. A scene that
//! panics once is not drawn again; the program around it carries on.

use std::ffi::{CStr, c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::render::{DEFAULT_BUDGET, Scene};
use glow::HasContext;

/// What the C side holds: the scene, and the GL functions it was loaded with.
pub struct WallfluxScene {
    gl: glow::Context,
    scene: Scene,
    /// Set once the scene has panicked, after which it is left alone.
    broken: bool,
}

/// Looks up an OpenGL function by name, as Qt's context does.
pub type Loader = unsafe extern "C" fn(name: *const c_char, user: *mut c_void) -> *const c_void;

/// Loads the scene wallpaper in `folder`, which is the folder holding its
/// `scene.pkg` and `project.json`.
///
/// Returns null if it cannot be drawn at all, with the reason written into
/// `error` -- at most `error_len` bytes, always ending in a nought. Parts of a
/// scene that will not build are left out and named on stderr; only a scene
/// with nothing left to draw is refused.
///
/// # Safety
/// `folder` must be a NUL-terminated path, `error` must hold `error_len`
/// bytes, and the OpenGL context `loader` belongs to must be current.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wallflux_scene_load(
    folder: *const c_char,
    loader: Loader,
    user: *mut c_void,
    error: *mut c_char,
    error_len: usize,
) -> *mut WallfluxScene {
    let say = |message: &str| {
        if error.is_null() || error_len == 0 {
            return;
        }
        let bytes = message.as_bytes();
        let n = bytes.len().min(error_len - 1);
        // SAFETY: the caller said `error` holds `error_len` bytes.
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), error.cast::<u8>(), n);
            *error.add(n) = 0;
        }
    };
    if folder.is_null() {
        say("no folder given");
        return std::ptr::null_mut();
    }
    // SAFETY: the caller said it is NUL-terminated.
    let folder = unsafe { CStr::from_ptr(folder) }.to_string_lossy().into_owned();

    let loaded = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: the context is current, as the caller promised.
        let gl = unsafe {
            glow::Context::from_loader_function_cstr(|name| loader(name.as_ptr(), user))
        };
        Scene::load_within(&gl, std::path::Path::new(&folder), DEFAULT_BUDGET)
            .map(|scene| (gl, scene))
    }));
    match loaded {
        Ok(Ok((gl, scene))) => {
            for skipped in &scene.skipped {
                eprintln!("wallflux scene {folder}: skipped {skipped}");
            }
            Box::into_raw(Box::new(WallfluxScene {
                gl,
                scene,
                broken: false,
            }))
        }
        Ok(Err(reason)) => {
            say(&reason);
            std::ptr::null_mut()
        }
        Err(_) => {
            say("the renderer failed while loading this wallpaper");
            std::ptr::null_mut()
        }
    }
}

/// Draws the scene as it is `seconds` after it started, into whatever
/// framebuffer is bound, at `width` by `height`.
///
/// Returns false once the scene has failed, after which it draws nothing.
///
/// # Safety
/// `scene` must come from `wallflux_scene_load` and not have been freed, and
/// the context it was loaded with must be current.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wallflux_scene_draw(
    scene: *mut WallfluxScene,
    width: u32,
    height: u32,
    seconds: f32,
) -> bool {
    // SAFETY: the caller said it is a live scene.
    let Some(this) = (unsafe { scene.as_mut() }) else {
        return false;
    };
    if this.broken {
        return false;
    }
    let drawn = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: the context is current, as the caller promised.
        unsafe {
            this.gl.viewport(0, 0, width as i32, height as i32);
            this.gl.clear_color(0.0, 0.0, 0.0, 1.0);
            this.gl.clear(glow::COLOR_BUFFER_BIT);
        }
        this.scene.draw(&this.gl, (width, height), seconds);
    }));
    if drawn.is_err() {
        eprintln!("wallflux scene: the renderer failed while drawing, and stops here");
        this.broken = true;
    }
    !this.broken
}

/// Says where the mouse is, as a fraction of a `width` by `height` output
/// from its top-left, or that there is none when `inside` is false.
///
/// # Safety
/// `scene` must come from `wallflux_scene_load` and not have been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wallflux_scene_point_at(
    scene: *mut WallfluxScene,
    inside: bool,
    x: f32,
    y: f32,
    width: u32,
    height: u32,
) {
    // SAFETY: the caller said it is a live scene.
    let Some(this) = (unsafe { scene.as_mut() }) else {
        return;
    };
    let pointer = inside.then_some((x, y));
    if catch_unwind(AssertUnwindSafe(|| this.scene.point_at(pointer, (width, height)))).is_err() {
        this.broken = true;
    }
}

/// Lets the scene go. Its GPU objects are left to the context, which the
/// renderer has never given anything back to and which goes when the window
/// does.
///
/// # Safety
/// `scene` must come from `wallflux_scene_load`, and is not to be used again.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wallflux_scene_free(scene: *mut WallfluxScene) {
    if !scene.is_null() {
        // SAFETY: the caller handed back what `wallflux_scene_load` made.
        drop(unsafe { Box::from_raw(scene) });
    }
}

/// Writes the sounds the scene in `folder` plays into `cache`, and describes
/// them as JSON in `out`: a list of `{files, mode, volume, mintime, maxtime,
/// startsilent}`. Needs no OpenGL and may be called from any thread.
///
/// Returns the length the JSON needs, without its closing nought. Where that
/// is not less than `out_len` nothing is written and the caller asks again
/// with room enough. A scene that cannot be read plays nothing, which is
/// `[]`, and says why on stderr.
///
/// # Safety
/// `folder` and `cache` must be NUL-terminated paths, and `out` must hold
/// `out_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wallflux_scene_sounds(
    folder: *const c_char,
    cache: *const c_char,
    out: *mut c_char,
    out_len: usize,
) -> usize {
    if folder.is_null() || cache.is_null() {
        return 0;
    }
    // SAFETY: the caller said both are NUL-terminated.
    let (folder, cache) = unsafe {
        (
            CStr::from_ptr(folder).to_string_lossy().into_owned(),
            CStr::from_ptr(cache).to_string_lossy().into_owned(),
        )
    };
    let json = catch_unwind(|| {
        match crate::sounds::unpack(std::path::Path::new(&folder), std::path::Path::new(&cache)) {
            Ok(sounds) => crate::sounds::to_json(&sounds),
            Err(reason) => {
                eprintln!("wallflux scene {folder}: no sound -- {reason}");
                "[]".to_owned()
            }
        }
    })
    .unwrap_or_else(|_| "[]".to_owned());
    let bytes = json.as_bytes();
    if !out.is_null() && bytes.len() < out_len {
        // SAFETY: the caller said `out` holds `out_len` bytes.
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), out.cast::<u8>(), bytes.len());
            *out.add(bytes.len()) = 0;
        }
    }
    bytes.len()
}

