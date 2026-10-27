//! Draws a wallpaper on the background layer of a Wayland desktop.
//!
//! The surface is placed on the background layer, under everything the desktop
//! puts on screen, and given an **empty input region**, so every click, drag
//! and scroll passes straight through to whatever is above it.
//!
//! The namespace given when the surface is created is not cosmetic on KWin: it
//! decides the window type the surface is treated as. It is an argument here so
//! the choice can be tested rather than assumed.
//!
//! Runs for a few seconds and exits, so a failed experiment leaves nothing
//! behind. The clock is its own thread rather than a count of frames, because
//! a compositor stops asking for frames once the surface is completely covered
//! -- correct of it, and the reason a wallpaper costs nothing while you work,
//! but it means frames are not a measure of time passing.
//!
//! ```text
//! cargo run -p wallflux -- <wallpaper-dir> [seconds] [namespace]
//! ```

mod audio;
mod gpu;
mod particles;
mod render;
mod texture;
mod transform;

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use smithay_client_toolkit::compositor::{
    CompositorHandler, CompositorState, FrameCallbackData, Region,
};
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::reexports::client::globals::registry_queue_init;
use smithay_client_toolkit::reexports::client::protocol::{wl_output, wl_surface};
use smithay_client_toolkit::reexports::client::{Connection, Proxy, QueueHandle};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{
    Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
    LayerSurfaceConfigure,
};
use smithay_client_toolkit::{delegate_dispatch2, delegate_registry, registry_handlers};

use gpu::Gpu;
use render::Scene;

fn main() {
    let Some(dir) = std::env::args_os().nth(1).map(std::path::PathBuf::from) else {
        eprintln!("usage: wallflux <wallpaper-dir> [seconds] [namespace]");
        std::process::exit(2);
    };
    let seconds: u64 = std::env::args()
        .nth(2)
        .and_then(|a| a.parse().ok())
        .unwrap_or(8);
    let namespace = std::env::args().nth(3).unwrap_or_else(|| "desktop".into());

    let conn =
        Connection::connect_to_env().expect("no Wayland display; is this a Wayland session?");
    let (globals, mut queue) = registry_queue_init(&conn).unwrap();
    let qh = queue.handle();

    let compositor = CompositorState::bind(&globals, &qh).expect("wl_compositor missing");
    let shell = LayerShell::bind(&globals, &qh)
        .expect("this compositor has no wlr-layer-shell, so it cannot host a wallpaper");

    let surface = compositor.create_surface(&qh);

    // Nothing is added to the region, so it stays empty and the surface takes
    // no input at all. Without this the wallpaper would swallow every click
    // meant for the desktop behind it.
    let region = Region::new(&compositor).expect("cannot create a region");
    surface.set_input_region(Some(region.wl_region()));

    let layer = shell.create_layer_surface(&qh, surface, Layer::Background, Some(&namespace), None);
    layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
    layer.set_exclusive_zone(-1); // never push other surfaces around
    layer.set_keyboard_interactivity(KeyboardInteractivity::None);
    layer.commit();

    let mut app = App {
        dir,
        registry_state: RegistryState::new(&globals),
        output_state: OutputState::new(&globals, &qh),
        conn: conn.clone(),
        layer,
        gpu: None,
        scene: None,
        size: (0, 0),
        started: Instant::now(),
        frames: Arc::new(AtomicU32::new(0)),
        run_for: Duration::from_secs(seconds),
        done: false,
        ears: audio::Audio::start(),
    };
    if app.ears.is_none() {
        eprintln!("no sound to listen to; wallpapers that follow it will run quiet");
    }

    println!("background layer, namespace {namespace:?}, for {seconds}s");

    let counter = Arc::clone(&app.frames);
    let run_for = app.run_for;
    std::thread::spawn(move || {
        std::thread::sleep(run_for);
        let drawn = counter.load(Ordering::Relaxed);
        let seconds = run_for.as_secs_f32();
        println!(
            "drew {drawn} frames in {seconds:.1}s ({:.0} fps)",
            drawn as f32 / seconds
        );
        if drawn <= 1 {
            println!("only one frame: the surface is covered, so nothing asked for more");
        }
        std::process::exit(0);
    });

    while !app.done {
        queue.blocking_dispatch(&mut app).unwrap();
    }
}

struct App {
    dir: std::path::PathBuf,
    registry_state: RegistryState,
    output_state: OutputState,
    conn: Connection,
    layer: LayerSurface,
    gpu: Option<Gpu>,
    scene: Option<Scene>,
    size: (u32, u32),
    started: Instant,
    frames: Arc<AtomicU32>,
    run_for: Duration,
    done: bool,
    /// What the speakers are playing. None where there is no sound server to
    /// ask, and the scene then runs as it would in a silent room.
    ears: Option<audio::Audio>,
}

impl App {
    fn draw(&mut self, qh: &QueueHandle<Self>) {
        if self.started.elapsed() >= self.run_for {
            self.done = true;
            return;
        }
        let (w, h) = self.size;
        if w == 0 || h == 0 {
            return;
        }

        let surface = self.layer.wl_surface().clone();
        if self.gpu.is_none() {
            match Gpu::new(&self.conn, &surface.id(), w, h) {
                Ok(gpu) => {
                    println!("GL: {}", gpu.renderer());
                    self.gpu = Some(gpu);
                }
                Err(e) => {
                    eprintln!("cannot start GL: {e}");
                    self.done = true;
                    return;
                }
            }
        }
        let Some(gpu) = self.gpu.as_mut() else { return };
        gpu.resize(w, h);

        if self.scene.is_none() {
            match Scene::load_within(&gpu.gl, &self.dir, render::DEFAULT_BUDGET) {
                Ok(loaded) => {
                    println!(
                        "scene {}x{}, {} objects: {}",
                        loaded.scene_size.0,
                        loaded.scene_size.1,
                        loaded.len(),
                        loaded.object_names().collect::<Vec<_>>().join(", ")
                    );
                    for skipped in &loaded.skipped {
                        eprintln!("  skipped {skipped}");
                    }
                    self.scene = Some(loaded);
                }
                Err(e) => {
                    eprintln!("cannot load wallpaper: {e}");
                    self.done = true;
                    return;
                }
            }
        }
        let Some(scene) = self.scene.as_mut() else {
            return;
        };

        // Ask for the next frame before presenting: swapping buffers commits
        // the surface, and a callback requested afterwards would miss it.
        surface.frame(qh, FrameCallbackData(surface.clone()));
        if let Some(ears) = &self.ears {
            scene.hear(ears.bands());
        }
        scene.draw(&gpu.gl, (w, h), self.started.elapsed().as_secs_f32());
        if let Err(e) = gpu.present() {
            eprintln!("frame failed: {e}");
            self.done = true;
            return;
        }
        self.frames.fetch_add(1, Ordering::Relaxed);
    }
}

impl LayerShellHandler for App {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        self.done = true;
    }

    fn configure(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        _: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _: u32,
    ) {
        if configure.new_size.0 == 0 || configure.new_size.1 == 0 {
            return;
        }
        let first = self.size == (0, 0);
        self.size = configure.new_size;
        if first {
            println!(
                "compositor gave us {}x{} on the background layer",
                self.size.0, self.size.1
            );
            self.started = Instant::now();
        }
        self.draw(qh);
    }
}

impl CompositorHandler for App {
    fn frame(&mut self, _: &Connection, qh: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {
        self.draw(qh);
    }

    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }

    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

delegate_registry!(App);
delegate_dispatch2!(App);
