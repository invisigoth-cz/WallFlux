//! Compiles every shader a wallpaper library uses, and reports what the driver
//! rejects.
//!
//! The preprocessor can only be checked against a real compiler: a missing
//! helper or a wrong bridge shows up as a compile error naming the line, and
//! nothing else finds those. This builds an off-screen GL context so it can run
//! without taking over the screen.
//!
//! ```text
//! cargo run -p wallflux --example compile -- ~/.steam/.../431960
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use glow::HasContext;
use khronos_egl as egl;
use wpe_format::shader::{Stage, preprocess};
use wpe_format::{Assets, Material, Package, Resolver};

fn main() {
    let Some(root) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: compile <wallpaper-dir>");
        std::process::exit(2);
    };
    let assets = Assets::locate();
    if assets.is_none() {
        eprintln!("no Wallpaper Engine installation found; built-in shaders will be missing");
    }

    let (_egl, gl) = headless_gl().unwrap_or_else(|e| {
        eprintln!("cannot get a GL context: {e}");
        std::process::exit(1);
    });
    unsafe {
        println!("GL: {}\n", gl.get_parameter_string(glow::VERSION));
    }

    let mut packages: Vec<PathBuf> = Vec::new();
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "pkg") {
                packages.push(path);
            }
        }
    }
    packages.sort();

    let mut tried = 0usize;
    let mut compiled = 0usize;
    let mut seen = BTreeSet::new();
    let mut failures: Vec<(String, String)> = Vec::new();
    let mut missing_includes = BTreeSet::new();

    for path in packages {
        let Ok(package) = Package::read(&path) else {
            continue;
        };
        let resolver = Resolver::new(&package, assets.as_ref());
        let wallpaper = path
            .parent()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();

        for (name, bytes) in package.iter() {
            if !(name.starts_with("materials/") && name.ends_with(".json")) {
                continue;
            }
            let Ok(material) = Material::parse(bytes) else {
                continue;
            };
            for pass in &material.passes {
                if pass.shader.is_empty() {
                    continue;
                }
                // Combos change which branches compile, so a shader is compiled
                // once per distinct set rather than once per name.
                let combos: BTreeMap<String, String> = pass
                    .defines()
                    .map(|(k, v)| (k, v.to_string().trim_matches('"').to_owned()))
                    .collect();
                let key = format!("{}|{:?}", pass.shader, combos);
                if !seen.insert(key) {
                    continue;
                }

                for (stage, ext) in [(Stage::Vertex, "vert"), (Stage::Fragment, "frag")] {
                    let Some(src) = resolver.shader(&pass.shader, ext) else {
                        continue;
                    };
                    let src = String::from_utf8_lossy(&src).into_owned();
                    let (full, missing) = preprocess(&src, stage, &combos, |include| {
                        resolver
                            .get(&format!("shaders/{include}"))
                            .map(|b| String::from_utf8_lossy(&b).into_owned())
                    });
                    for m in missing {
                        missing_includes.insert(m.to_string());
                    }
                    tried += 1;
                    match compile(&gl, stage, &full) {
                        Ok(()) => compiled += 1,
                        Err(e) => failures.push((
                            format!("{wallpaper} {}.{ext}", pass.shader),
                            first_errors(&e),
                        )),
                    }
                }
            }
        }
    }

    println!("shaders compiled  {compiled}/{tried}");
    if !missing_includes.is_empty() {
        println!("missing includes  {}", missing_includes.len());
        for m in missing_includes.iter().take(5) {
            println!("  {m}");
        }
    }
    if failures.is_empty() {
        println!("\nevery shader compiled");
    } else {
        println!("\n{} failed:", failures.len());
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for (_, why) in &failures {
            *counts.entry(why.as_str()).or_default() += 1;
        }
        let mut ranked: Vec<_> = counts.into_iter().collect();
        ranked.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        for (why, n) in ranked.iter().take(8) {
            println!("  {n:>4}x  {why}");
        }
        println!("\nfirst few, with the shader named:");
        for (what, why) in failures.iter() {
            println!("  {what}: {why}");
        }
    }
}

/// Keeps the first couple of error lines, which name the real problem; drivers
/// then report a cascade that says nothing new.
fn first_errors(log: &str) -> String {
    log.lines()
        .filter(|l| !l.trim().is_empty())
        .take(2)
        .collect::<Vec<_>>()
        .join(" / ")
}

fn compile(gl: &glow::Context, stage: Stage, source: &str) -> Result<(), String> {
    unsafe {
        let kind = match stage {
            Stage::Vertex => glow::VERTEX_SHADER,
            Stage::Fragment => glow::FRAGMENT_SHADER,
        };
        let shader = gl.create_shader(kind)?;
        gl.shader_source(shader, source);
        gl.compile_shader(shader);
        let ok = gl.get_shader_compile_status(shader);
        let log = gl.get_shader_info_log(shader);
        gl.delete_shader(shader);
        if ok { Ok(()) } else { Err(log) }
    }
}

type Egl = egl::DynamicInstance<egl::EGL1_4>;

/// A GL context with no window, so this can run without touching the screen.
fn headless_gl() -> Result<(Egl, glow::Context), String> {
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
        .ok_or("no GLES 3 config")?;
    let context = egl
        .create_context(
            display,
            config,
            None,
            &[
                // From EGL_KHR_create_context; EGL 1.4 has no named constants.
                0x3098, /* CONTEXT_MAJOR_VERSION */
                3,
                0x30FB, /* CONTEXT_MINOR_VERSION */
                3,
                0x30FD, /* CONTEXT_OPENGL_PROFILE_MASK */
                0x0001, /* CORE */
                egl::NONE,
            ],
        )
        .map_err(|e| e.to_string())?;
    let surface = egl
        .create_pbuffer_surface(
            display,
            config,
            &[egl::WIDTH, 16, egl::HEIGHT, 16, egl::NONE],
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
