mod config;
mod dashboard;
mod registration;
mod devices;
mod scene;
mod gpu_texture;
mod hardware;
mod overlay;
mod render;
mod timing;

use std::error::Error;
use std::thread;
use std::time::{Duration, Instant};

use config::{Config, Upload};
use devices::DeviceStats;
use scene::SceneProbe;
use gpu_texture::GlUploader;
use hardware::HardwareMonitor;
use overlay::Positioner;
use render::Renderer;
use timing::FrameMonitor;

fn main() {
    if let Err(error) = run() {
        eprintln!("vrperf: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    // SteamVR may append an empty argument when autostarting a dashboard app.
    // Ignore those while retaining strict handling of nonempty options.
    let args: Vec<_> = std::env::args().skip(1).filter(|arg| !arg.is_empty()).collect();
    match args.as_slice() {
        [arg] if arg == "--install" => return registration::install(),
        [arg] if arg == "--uninstall" => return registration::uninstall(),
        _ => {}
    }
    let config = Config::from_args()?;
    // SAFETY: Only one OpenVR context exists. It lives longer than the OpenVR interfaces.
    let context = unsafe { openvr::init(openvr::ApplicationType::Overlay) }
        .map_err(|e| format!("SteamVR connection failed: {e}"))?;
    let system = context.system().map_err(|e| format!("Cannot get SteamVR system: {e:?}"))?;
    let compositor = context.compositor().map_err(|e| format!("Cannot get SteamVR compositor: {e:?}"))?;
    let mut overlays = context.overlay().map_err(|e| format!("Cannot get SteamVR overlays: {e:?}"))?;
    let handle = overlays.create_overlay("vrperf.linux.performance\0", "vrperf PC performance\0")
        .map_err(|e| format!("Cannot create overlay (already running?): {e:?}"))?;
    overlays.set_width(handle, config.width_m)
        .map_err(|e| format!("Cannot set overlay width: {e:?}"))?;
    overlays.set_opacity(handle, config.opacity)
        .map_err(|e| format!("Cannot set overlay opacity: {e:?}"))?;

    let mut renderer = Renderer::new();
    let mut positioner = Positioner::new();
    let mut frames = FrameMonitor::new();
    let mut scene = SceneProbe::new()?;
    let mut hardware = HardwareMonitor::new();
    let mut devices = DeviceStats::default();
    let mut vr_settings = context.settings().ok();
    let mut last_render = Instant::now() - Duration::from_secs(2);
    let mut last_hw = Instant::now() - Duration::from_secs(1);
    let mut last_position = Instant::now() - Duration::from_secs(1);
    let mut last_scene = Instant::now() - Duration::from_secs(1);
    let mut last_devices = Instant::now() - Duration::from_secs(11);
    let mut last_resolution = Instant::now() - Duration::from_secs(6);
    let interval = Duration::from_millis(config.interval_ms);
    let mut gpu = if config.upload == Upload::Gl {
        match GlUploader::new(render::WIDTH, render::HEIGHT) {
            Ok(gl) => match gl.set_vertical_flip(handle, true) {
                Ok(()) => Some(gl),
                Err(error) => {
                    eprintln!("vrperf: {error}; using raw uploads instead");
                    None
                }
            },
            Err(error) => {
                eprintln!("vrperf: {error}; using raw uploads instead");
                None
            }
        }
    } else { None };
    let mut enabled = registration::read_visible();
    // A dashboard failure does not prevent the working v1.3 performance
    // overlay from running; it is reported explicitly to the user.
    let mut dashboard = match dashboard::Dashboard::new(&mut overlays, enabled) {
        Ok(d) => Some(d),
        Err(error) => {
            eprintln!("vrperf: dashboard tab unavailable: {error}");
            None
        }
    };
    if dashboard.is_none() && !enabled {
        eprintln!("vrperf: restoring the performance display because the dashboard is unavailable");
        enabled = true;
    }
    // Apply saved visibility immediately rather than showing the overlay for
    // a moment when the user previously switched it off.
    positioner.update(&system, &mut overlays, handle, &config, enabled)?;
    let mut failures = 0_u32;
    let mut last_warning = Instant::now() - Duration::from_secs(60);

    eprintln!("vrperf {} connected to SteamVR ({:?} mode, {} upload)", env!("CARGO_PKG_VERSION"),
        config.mode, if gpu.is_some() { "OpenGL" } else { "raw" });
    loop {
        // SteamVR asks registered overlays to quit during shutdown.
        for _ in 0..128 {
            let Some(event) = system.poll_next_event() else { break; };
            if matches!(event.event, openvr::system::Event::Quit(_)) {
                system.acknowledge_quit_exiting();
                return Ok(());
            }
        }
        if let Some(ref mut dashboard) = dashboard {
            match dashboard.poll(&mut overlays, enabled) {
                Ok(true) => {
                    enabled = !enabled;
                    if let Err(error) = registration::write_visible(enabled) {
                        eprintln!("vrperf: visibility could not be saved: {error}");
                    }
                    positioner.update(&system, &mut overlays, handle, &config, enabled)?;
                    if enabled {
                        frames = FrameMonitor::new();
                        last_render = Instant::now() - interval;
                    }
                    eprintln!("vrperf: overlay {}", if enabled { "shown" } else { "hidden" });
                }
                Ok(false) => {}
                Err(error) => eprintln!("vrperf: dashboard input error: {error}"),
            }
        }
        // The dashboard still accepts input when the performance image is
        // hidden. Avoid metrics collection and texture uploads until enabled.
        if !enabled {
            thread::sleep(Duration::from_millis(50));
            continue;
        }
        if last_scene.elapsed() >= Duration::from_millis(100) {
            if let Some(active_process) = scene.poll() {
                frames.set_scene(active_process, &compositor);
            }
            last_scene = Instant::now();
        }
        frames.poll(&compositor);
        if last_hw.elapsed() >= Duration::from_millis(500) {
            hardware.update();
            last_hw = Instant::now();
        }
        if last_devices.elapsed() >= Duration::from_secs(10) {
            devices.update_batteries(&system);
            last_devices = Instant::now();
        }
        if last_resolution.elapsed() >= Duration::from_secs(5) {
            if let Some(ref mut settings) = vr_settings {
                devices.update_resolution(settings);
            }
            last_resolution = Instant::now();
        }
        if last_position.elapsed() >= Duration::from_secs(1) {
            positioner.update(&system, &mut overlays, handle, &config, enabled)?;
            last_position = Instant::now();
        }
        if last_render.elapsed() >= interval {
            let hz = config.hz.unwrap_or_else(|| {
                system.float_tracked_device_property(
                    openvr::tracked_device_index::HMD,
                    openvr::property::DisplayFrequency_Float,
                ).ok().filter(|hz| hz.is_finite() && *hz >= 30.0 && *hz <= 240.0)
                    .unwrap_or(90.0)
            });
            renderer.draw(&frames, &hardware.stats, &devices, hz);
            let result = if let Some(ref mut gl) = gpu {
                gl.upload(handle, renderer.pixels())
            } else {
                overlays.set_raw_data(handle, renderer.pixels(), render::WIDTH, render::HEIGHT, 4)
                    .map_err(|error| format!("SetOverlayRaw: {error:?}"))
            };
            match result {
                Ok(()) => failures = 0,
                Err(error) => {
                    failures += 1;
                    if last_warning.elapsed() >= Duration::from_secs(5) {
                        eprintln!("vrperf: image upload error: {error} (retrying)");
                        last_warning = Instant::now();
                    }
                    if gpu.is_some() && failures >= 3 {
                        eprintln!("vrperf: OpenGL path failed repeatedly; falling back to raw uploads");
                        if let Some(gl) = gpu.take() {
                            if let Err(error) = gl.set_vertical_flip(handle, false) {
                                eprintln!("vrperf: could not reset raw texture bounds: {error}");
                            }
                        }
                        failures = 0;
                    }
                }
            }
            last_render = Instant::now();
        }
        thread::sleep(Duration::from_millis(50));
    }
}
