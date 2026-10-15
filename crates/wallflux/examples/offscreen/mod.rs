//! A GL context with no window, for the tools that draw to a file.
//!
//! Shared by the tools rather than written out in each: what they render has
//! to come from the same context, or a difference between them says nothing.

use khronos_egl as egl;

pub type Egl = egl::DynamicInstance<egl::EGL1_4>;

pub fn headless(width: u32, height: u32) -> Result<(Egl, glow::Context), String> {
    let egl: Egl = unsafe { egl::DynamicInstance::<egl::EGL1_4>::load_required() }
        .map_err(|e| format!("no usable libEGL: {e}"))?;
    let display = unsafe { egl.get_display(egl::DEFAULT_DISPLAY) }.ok_or("no default display")?;
    egl.initialize(display).map_err(|e| e.to_string())?;
    egl.bind_api(egl::OPENGL_API).map_err(|e| e.to_string())?;
    let config = egl
        .choose_first_config(
            display,
            &[
                egl::SURFACE_TYPE,
                egl::PBUFFER_BIT,
                egl::RENDERABLE_TYPE,
                egl::OPENGL_BIT,
                egl::NONE,
            ],
        )
        .map_err(|e| e.to_string())?
        .ok_or("no desktop GL config")?;
    let context = egl
        .create_context(
            display,
            config,
            None,
            &[0x3098, 3, 0x30FB, 3, 0x30FD, 0x0001, egl::NONE],
        )
        .map_err(|e| e.to_string())?;
    let surface = egl
        .create_pbuffer_surface(
            display,
            config,
            &[
                egl::WIDTH,
                width as i32,
                egl::HEIGHT,
                height as i32,
                egl::NONE,
            ],
        )
        .map_err(|e| e.to_string())?;
    egl.make_current(display, Some(surface), Some(surface), Some(context))
        .map_err(|e| e.to_string())?;
    let gl = unsafe {
        glow::Context::from_loader_function(|name| {
            egl.get_proc_address(name)
                .map_or(std::ptr::null(), |p| p as *const _)
        })
    };
    Ok((egl, gl))
}
