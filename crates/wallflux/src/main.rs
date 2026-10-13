//! Opens a wallpaper-sized surface on the background layer and animates it.
//!
//! This exists to settle one question before any rendering work is built on
//! top of it: whether a wallpaper can sit behind a working desktop rather than
//! replacing it. Two things make that so. The surface is placed on the
//! background layer, under everything the desktop puts on screen, and it is
//! given an **empty input region**, so every click, drag and scroll passes
//! straight through to whatever is above it.
//!
//! The namespace given when the surface is created is not cosmetic on KWin: it
//! decides the window type the surface is treated as, and so where in the
//! stack it lands relative to the desktop that draws the icons. It is an
//! argument here so the choice can be tested rather than assumed.
//!
//! Runs for a few seconds and exits, so a failed experiment cannot leave
//! anything behind.
//!
//! ```text
//! cargo run -p wallflux -- [seconds] [namespace]
//! ```

use std::time::{Duration, Instant};

use smithay_client_toolkit::compositor::{
    CompositorHandler, CompositorState, FrameCallbackData, Region,
};
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::reexports::client::globals::registry_queue_init;
use smithay_client_toolkit::reexports::client::protocol::{wl_output, wl_shm, wl_surface};
use smithay_client_toolkit::reexports::client::{Connection, QueueHandle};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{
    Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
    LayerSurfaceConfigure,
};
use smithay_client_toolkit::shm::slot::SlotPool;
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use smithay_client_toolkit::{delegate_dispatch2, delegate_registry, registry_handlers};

fn main() {
    let seconds: u64 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(8);
    let namespace = std::env::args().nth(2).unwrap_or_else(|| "desktop".into());

    let conn =
        Connection::connect_to_env().expect("no Wayland display; is this a Wayland session?");
    let (globals, mut queue) = registry_queue_init(&conn).unwrap();
    let qh = queue.handle();

    let compositor = CompositorState::bind(&globals, &qh).expect("wl_compositor missing");
    let shell = LayerShell::bind(&globals, &qh)
        .expect("this compositor has no wlr-layer-shell, so it cannot host a wallpaper");
    let shm = Shm::bind(&globals, &qh).expect("wl_shm missing");

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

    let pool = SlotPool::new(4, &shm).expect("cannot create a buffer pool");

    let mut app = Wallpaper {
        registry_state: RegistryState::new(&globals),
        output_state: OutputState::new(&globals, &qh),
        shm,
        pool,
        layer,
        width: 0,
        height: 0,
        configured: false,
        started: Instant::now(),
        frames: 0,
        run_for: Duration::from_secs(seconds),
        done: false,
    };

    println!("background layer, namespace {namespace:?}, for {seconds}s");
    while !app.done {
        queue.blocking_dispatch(&mut app).unwrap();
    }
    let elapsed = app.started.elapsed().as_secs_f32();
    println!(
        "drew {} frames in {elapsed:.1}s ({:.0} fps)",
        app.frames,
        app.frames as f32 / elapsed
    );
}

struct Wallpaper {
    registry_state: RegistryState,
    output_state: OutputState,
    shm: Shm,
    pool: SlotPool,
    layer: LayerSurface,
    width: u32,
    height: u32,
    configured: bool,
    started: Instant,
    frames: u32,
    run_for: Duration,
    done: bool,
}

impl Wallpaper {
    fn draw(&mut self, qh: &QueueHandle<Self>) {
        if self.started.elapsed() >= self.run_for {
            self.done = true;
            return;
        }
        let (w, h) = (self.width, self.height);
        if w == 0 || h == 0 {
            return;
        }

        let stride = w as i32 * 4;
        let (buffer, canvas) = self
            .pool
            .create_buffer(w as i32, h as i32, stride, wl_shm::Format::Xrgb8888)
            .expect("cannot create a buffer");

        // Something obviously moving, so the question "is it animating while I
        // work?" can be answered by looking rather than by trusting a log line.
        let t = self.started.elapsed().as_secs_f32();
        for (i, pixel) in canvas.chunks_exact_mut(4).enumerate() {
            let x = (i as u32 % w) as f32 / w as f32;
            let y = (i as u32 / w) as f32 / h as f32;
            let wave = ((x * 6.0 + t * 1.5).sin() * 0.5 + 0.5) * ((y * 4.0 - t).cos() * 0.5 + 0.5);
            pixel.copy_from_slice(&[
                (40.0 + 180.0 * wave) as u8,        // blue
                (20.0 + 90.0 * (1.0 - wave)) as u8, // green
                (90.0 + 120.0 * wave * y) as u8,    // red
                255,
            ]);
        }

        self.frames += 1;
        self.layer
            .wl_surface()
            .damage_buffer(0, 0, w as i32, h as i32);
        let surface = self.layer.wl_surface().clone();
        surface.frame(qh, FrameCallbackData(surface.clone()));
        buffer
            .attach_to(self.layer.wl_surface())
            .expect("cannot attach buffer");
        self.layer.commit();
    }
}

impl LayerShellHandler for Wallpaper {
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
        (self.width, self.height) = configure.new_size;
        if self.width == 0 || self.height == 0 {
            return;
        }
        if !self.configured {
            self.configured = true;
            println!(
                "compositor gave us {}x{} on the background layer",
                self.width, self.height
            );
            self.started = Instant::now();
        }
        self.draw(qh);
    }
}

impl CompositorHandler for Wallpaper {
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

impl OutputHandler for Wallpaper {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl ShmHandler for Wallpaper {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for Wallpaper {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

delegate_registry!(Wallpaper);
delegate_dispatch2!(Wallpaper);
