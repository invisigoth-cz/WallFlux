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
    program: glow::Program,
    vao: glow::VertexArray,
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

        let program = unsafe { build_program(&gl)? };
        let vao = unsafe {
            gl.create_vertex_array()
                .map_err(|e| format!("vertex array: {e}"))?
        };

        Ok(Self {
            egl,
            display,
            context,
            surface: egl_surface,
            window,
            gl,
            program,
            vao,
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

    /// Draws one frame and presents it. Presenting commits the surface, so the
    /// caller does not commit separately.
    pub fn draw(&self, time: f32) -> Result<(), String> {
        let (w, h) = self.size;
        unsafe {
            self.gl.viewport(0, 0, w as i32, h as i32);
            self.gl.use_program(Some(self.program));
            if let Some(loc) = self.gl.get_uniform_location(self.program, "u_time") {
                self.gl.uniform_1_f32(Some(&loc), time);
            }
            if let Some(loc) = self.gl.get_uniform_location(self.program, "u_resolution") {
                self.gl.uniform_2_f32(Some(&loc), w as f32, h as f32);
            }
            // The quad is generated in the vertex shader from gl_VertexID, so
            // there are no buffers to bind; the array object is only here
            // because GLES 3 requires one to be bound.
            self.gl.bind_vertex_array(Some(self.vao));
            self.gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
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

const VERTEX: &str = r#"#version 330 core
// A full-screen quad with no vertex data: the four corners come from the
// vertex index, which keeps the first renderer free of buffer plumbing.
out vec2 v_uv;
void main() {
    vec2 corner = vec2(float(gl_VertexID & 1), float((gl_VertexID >> 1) & 1));
    v_uv = corner;
    gl_Position = vec4(corner * 2.0 - 1.0, 0.0, 1.0);
}
"#;

const FRAGMENT: &str = r#"#version 330 core
in vec2 v_uv;
out vec4 fragColor;
uniform float u_time;
uniform vec2 u_resolution;
void main() {
    vec2 p = v_uv * vec2(u_resolution.x / u_resolution.y, 1.0);
    float wave = sin(p.x * 6.0 + u_time * 1.5) * 0.5 + 0.5;
    float band = cos(p.y * 4.0 - u_time) * 0.5 + 0.5;
    float v = wave * band;
    fragColor = vec4(0.35 + 0.5 * v * v_uv.y, 0.08 + 0.35 * (1.0 - v), 0.16 + 0.7 * v, 1.0);
}
"#;

unsafe fn build_program(gl: &glow::Context) -> Result<glow::Program, String> {
    unsafe {
        let program = gl
            .create_program()
            .map_err(|e| format!("create program: {e}"))?;
        let mut shaders = Vec::new();
        for (kind, source) in [
            (glow::VERTEX_SHADER, VERTEX),
            (glow::FRAGMENT_SHADER, FRAGMENT),
        ] {
            let shader = gl
                .create_shader(kind)
                .map_err(|e| format!("create shader: {e}"))?;
            gl.shader_source(shader, source);
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                return Err(format!(
                    "shader failed to compile: {}",
                    gl.get_shader_info_log(shader)
                ));
            }
            gl.attach_shader(program, shader);
            shaders.push(shader);
        }
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            return Err(format!(
                "program failed to link: {}",
                gl.get_program_info_log(program)
            ));
        }
        for shader in shaders {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }
        Ok(program)
    }
}
