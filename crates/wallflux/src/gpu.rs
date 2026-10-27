// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

//! An OpenGL ES context on a Wayland surface.
//!
//! Wallpaper Engine ships its shaders as GLSL, so the renderer speaks GL
//! rather than translating them into another shading language. This sets up the
//! EGL side of that: a `wl_egl_window` wrapping the layer surface, with
//! `eglSwapBuffers` taking over the job of committing frames.
//!
//! Desktop GL rather than GLES, because the shaders are written for it: GLSL ES
//! rejects an undefined name in an `#if`, which desktop GLSL reads as zero, and
//! the wallpapers rely on that for every option a material leaves unset.
//! Compiling the library against both put it at 79 shaders out of 110 on GLES
//! and 105 on desktop GL.

use glow::HasContext;
use khronos_egl as egl;
use smithay_client_toolkit::reexports::client::Connection;
use smithay_client_toolkit::reexports::client::backend::ObjectId;
use wayland_egl::WlEglSurface;

/// khronos-egl carries its own loader, so this must use its type rather than
/// naming `libloading` here: a second copy of that crate would not match.
type Egl = egl::DynamicInstance<egl::EGL1_4>;

pub struct Gpu {
    egl: Egl,
    display: egl::Display,
    context: egl::Context,
    surface: egl::Surface,
    /// Owns the `wl_egl_window`; dropping it tears down the EGL surface's
    /// backing, so it has to outlive the surface above.
    window: WlEglSurface,
    pub gl: glow::Context,
    size: (u32, u32),
}

impl Gpu {
    /// Binds a GLES 3 context to `surface`, sized `width` by `height`.
    pub fn new(
        conn: &Connection,
        surface: &ObjectId,
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        let egl: Egl = unsafe { egl::DynamicInstance::<egl::EGL1_4>::load_required() }
            .map_err(|e| format!("no usable libEGL: {e}"))?;

        let display = unsafe {
            egl.get_display(conn.backend().display_ptr() as *mut std::ffi::c_void)
                .ok_or("EGL has no display for this Wayland connection")?
        };
        egl.initialize(display)
            .map_err(|e| format!("eglInitialize: {e}"))?;
        egl.bind_api(egl::OPENGL_API)
            .map_err(|e| format!("eglBindAPI: {e}"))?;

        let config = egl
            .choose_first_config(
                display,
                &[
                    egl::SURFACE_TYPE,
                    egl::WINDOW_BIT,
                    egl::RENDERABLE_TYPE,
                    egl::OPENGL_BIT,
                    egl::RED_SIZE,
                    8,
                    egl::GREEN_SIZE,
                    8,
                    egl::BLUE_SIZE,
                    8,
                    egl::ALPHA_SIZE,
                    8,
                    egl::NONE,
                ],
            )
            .map_err(|e| format!("eglChooseConfig: {e}"))?
            .ok_or("no EGL config with desktop GL and 8-bit colour")?;

        let context = egl
            .create_context(
                display,
                config,
                None,
                &[
                    // From EGL_KHR_create_context; EGL 1.4 has no named
                    // constants for these.
                    0x3098, // CONTEXT_MAJOR_VERSION
                    3,
                    0x30FB, // CONTEXT_MINOR_VERSION
                    3,
                    0x30FD, // CONTEXT_OPENGL_PROFILE_MASK
                    0x0001, // CORE
                    egl::NONE,
                ],
            )
            .map_err(|e| format!("eglCreateContext: {e}"))?;

        let window = WlEglSurface::new(surface.clone(), width as i32, height as i32)
            .map_err(|e| format!("wl_egl_window: {e}"))?;
        let egl_surface = unsafe {
            egl.create_window_surface(display, config, window.ptr() as *mut std::ffi::c_void, None)
                .map_err(|e| format!("eglCreateWindowSurface: {e}"))?
        };

        egl.make_current(display, Some(egl_surface), Some(egl_surface), Some(context))
            .map_err(|e| format!("eglMakeCurrent: {e}"))?;

        let gl = unsafe {
            glow::Context::from_loader_function(|name| {
                egl.get_proc_address(name)
                    .map_or(std::ptr::null(), |p| p as *const _)
            })
        };

        Ok(Self {
            egl,
            display,
            context,
            surface: egl_surface,
            window,
            gl,
            size: (width, height),
        })
    }

    pub fn renderer(&self) -> String {
        unsafe {
            format!(
                "{} / {}",
                self.gl.get_parameter_string(glow::RENDERER),
                self.gl.get_parameter_string(glow::SHADING_LANGUAGE_VERSION)
            )
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if self.size == (width, height) {
            return;
        }
        self.size = (width, height);
        self.window.resize(width as i32, height as i32, 0, 0);
    }

    /// Presents what has been drawn. Presenting commits the surface, so the
    /// caller does not commit separately.
    pub fn present(&self) -> Result<(), String> {
        self.egl
            .swap_buffers(self.display, self.surface)
            .map_err(|e| format!("eglSwapBuffers: {e}"))
    }
}

impl Drop for Gpu {
    fn drop(&mut self) {
        let _ = self.egl.make_current(self.display, None, None, None);
        let _ = self.egl.destroy_surface(self.display, self.surface);
        let _ = self.egl.destroy_context(self.display, self.context);
        let _ = self.egl.terminate(self.display);
    }
}
