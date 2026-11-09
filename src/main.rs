mod capture;
mod calibration;
mod config;
mod dashboard;
mod registration;
mod devices;
mod scene;
mod gpu_texture;
mod hardware;
mod input;
mod overlay;
mod render;
mod timing;

use std::error::Error;
use std::thread;
use std::time::{Duration, Instant};

use config::{Config, Upload};
use dashboard::{Action as DashboardAction, State as DashboardState};
use devices::DeviceStats;
use scene::SceneProbe;
use gpu_texture::GlUploader;
use hardware::HardwareMonitor;
use overlay::Positioner;
use input::Input;
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
    let mut config = Config::from_args()?;
    let mut pending_capture = config.capture_frame.take();
    if !config.explicit_mode { config.mode = registration::read_mode(); }
    // SAFETY: Only one OpenVR context exists. It lives longer than the OpenVR interfaces.
    let context = unsafe { openvr::init(openvr::ApplicationType::Overlay) }
        .map_err(|e| format!("SteamVR connection failed: {e}"))?;
    let system = context.system().map_err(|e| format!("Cannot get SteamVR system: {e:?}"))?;
    // IVRInput must load the action manifest before the first PollNextEvent.
    // Keep the performance display available if action initialization fails.
    let mut input = match Input::new() {
        Ok(input) => Some(input),
        Err(error) => { eprintln!("vrperf: watch calibration input unavailable: {error}"); None }
    };
    let compositor = context.compositor().map_err(|e| format!("Cannot get SteamVR compositor: {e:?}"))?;
    let mut overlays = context.overlay().map_err(|e| format!("Cannot get SteamVR overlays: {e:?}"))?;
    let handle = overlays.create_overlay("vrperf.linux.performance\0", "vrperf PC performance\0")
        .map_err(|e| format!("Cannot create overlay (already running?): {e:?}"))?;
    let mounts = [
        registration::read_watch_mount(config::Mode::Left),
        registration::read_watch_mount(config::Mode::Right),
    ];
    let mut positioner = Positioner::new(mounts);
    overlays.set_width(handle, positioner.width(config.mode, config.width_m))
        .map_err(|e| format!("Cannot set overlay width: {e:?}"))?;
    // Fixed opacity with straight RGBA pixels. Avoid partial-alpha colour fringes.
    overlays.set_opacity(handle, 0.93)
        .map_err(|e| format!("Cannot set overlay opacity: {e:?}"))?;
    let mut renderer = Renderer::for_mode(config.mode);
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
    let mut gpu = create_uploader(&config, &renderer, handle);
    let mut enabled = registration::read_visible();
    // Keep the performance overlay running if the optional dashboard fails.
    let mut dashboard = match dashboard::Dashboard::new(&mut overlays,
        DashboardState::new(enabled, config.mode, positioner.calibrating(),
            positioner.width(config.mode, config.width_m), &devices)) {
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
    positioner.update(&system, &mut overlays, handle, &config, enabled, [None;2])?;
    let mut failures = 0_u32;
    let mut last_warning = Instant::now() - Duration::from_secs(60);
    let mut next_upload_attempt = Instant::now();
    let mut next_dashboard_attempt = Instant::now();
    let mut last_dashboard_warning = Instant::now() - Duration::from_secs(60);

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
        let grab = input.as_mut()
            .map(|input| input.poll(positioner.calibrating()))
            .unwrap_or([None;2]);
        if let Some(ref mut dashboard) = dashboard {
            match dashboard.poll() {
                Ok(Some(DashboardAction::Toggle)) => {
                    if positioner.calibrating().is_none() {
                        enabled = !enabled;
                        if let Err(error) = registration::write_visible(enabled) {
                            eprintln!("vrperf: visibility could not be saved: {error}");
                        }
                        positioner.update(&system, &mut overlays, handle, &config, enabled, grab)?;
                        if enabled { frames=FrameMonitor::new();last_render=Instant::now()-interval; }
                    }
                }
                Ok(Some(DashboardAction::SetMode(mode))) => {
                    if positioner.calibrating().is_some() { positioner.cancel_calibration(); }
                    if mode != config.mode {
                        change_mode(mode, &mut config, &mut renderer, &mut overlays,
                            handle, &mut gpu, &mut positioner, &system, enabled, grab)?;
                        last_render=Instant::now()-interval;
                    } else {
                        overlays.set_width(handle,positioner.width(mode,config.width_m))
                            .map_err(|e|format!("Watch width: {e:?}"))?;
                        positioner.update(&system,&mut overlays,handle,&config,enabled,grab)?;
                    }
                }
                Ok(Some(DashboardAction::StartCalibration(side))) => {
                    positioner.start_calibration(side)?;
                    if !enabled {
                        enabled=true;
                        if let Err(error)=registration::write_visible(true) {
                            eprintln!("vrperf: visibility could not be saved: {error}");
                        }
                    }
                    if side != config.mode {
                        change_mode(side,&mut config,&mut renderer,&mut overlays,handle,
                            &mut gpu,&mut positioner,&system,enabled,grab)?;
                    } else {
                        overlays.set_width(handle,positioner.width(side,config.width_m))
                            .map_err(|e|format!("Watch width: {e:?}"))?;
                        positioner.update(&system,&mut overlays,handle,&config,enabled,grab)?;
                    }
                    eprintln!("vrperf: calibrating {side:?} watch: CLOSE the SteamVR dashboard first. Calibration works in-game or in SteamVR Home. Release, then hold the OPPOSITE controller's grip (or trigger) and move/rotate the watch. Reopen the dashboard to SAVE or CANCEL.");
                    last_render=Instant::now()-interval;
                }
                Ok(Some(DashboardAction::SizeDelta(delta))) => {
                    if positioner.calibrating().is_some() {
                        positioner.adjust_width(delta);
                        overlays.set_width(handle,positioner.width(config.mode,config.width_m))
                            .map_err(|e|format!("Watch size change: {e:?}"))?;
                    }
                }
                Ok(Some(DashboardAction::SaveCalibration)) => {
                    if let Some((side,mount))=positioner.save_calibration() {
                        if let Err(error)=registration::write_watch_mount(side,&mount) {
                            eprintln!("vrperf: could not save {side:?} watch calibration: {error}");
                        } else {eprintln!("vrperf: {side:?} watch calibration saved");}
                        positioner.update(&system,&mut overlays,handle,&config,enabled,grab)?;
                    }
                }
                Ok(Some(DashboardAction::CancelCalibration)) => {
                    if positioner.calibrating().is_some() {
                        positioner.cancel_calibration();
                        overlays.set_width(handle,positioner.width(config.mode,config.width_m))
                            .map_err(|e|format!("Watch width: {e:?}"))?;
                        positioner.update(&system,&mut overlays,handle,&config,enabled,grab)?;
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    if last_dashboard_warning.elapsed() >= Duration::from_secs(5) {
                        eprintln!("vrperf: dashboard input error: {error}");
                        last_dashboard_warning=Instant::now();
                    }
                }
            }
            if Instant::now() >= next_dashboard_attempt {
                match dashboard.refresh(&mut overlays, DashboardState::new(enabled, config.mode,
                    positioner.calibrating(), positioner.width(config.mode, config.width_m), &devices)) {
                    Ok(()) => next_dashboard_attempt = Instant::now() + Duration::from_millis(100),
                    Err(error) => {
                        if last_dashboard_warning.elapsed() >= Duration::from_secs(5) {
                            eprintln!("vrperf: dashboard refresh error: {error} (retrying slowly)");
                            last_dashboard_warning = Instant::now();
                        }
                        next_dashboard_attempt = Instant::now() + Duration::from_secs(2);
                    }
                }
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
                devices.app_resolution_pct = None;
                devices.combined_resolution_pct = None;
                last_resolution = Instant::now() - Duration::from_secs(6);
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
                let key = scene.application_key();
                devices.update_resolution(settings, key.as_deref());
            }
            last_resolution = Instant::now();
        }
        if config.mode.wrist() || last_position.elapsed() >= Duration::from_secs(1) {
            positioner.update(&system, &mut overlays, handle, &config, enabled, grab)?;
            last_position = Instant::now();
        }
        if last_render.elapsed() >= interval && Instant::now() >= next_upload_attempt {
            let hz = config.hz.unwrap_or_else(|| {
                system.float_tracked_device_property(
                    openvr::tracked_device_index::HMD,
                    openvr::property::DisplayFrequency_Float,
                ).ok().filter(|hz| hz.is_finite() && *hz >= 30.0 && *hz <= 240.0)
                    .unwrap_or(90.0)
            });
            renderer.draw(&frames, &hardware.stats, &devices, hz);
            if let Some(path) = pending_capture.take() {
                let (width, height) = renderer.dimensions();
                match capture::save_rgba_png(std::path::Path::new(&path),
                    renderer.pixels(), width, height) {
                    Ok(()) => eprintln!("vrperf: saved straight RGBA source frame to {path}"),
                    Err(error) => eprintln!("vrperf: capture failed: {error}"),
                }
            }
            let result = if let Some(ref mut gl) = gpu {
                gl.upload(handle, renderer.pixels())
            } else {
                overlays.set_raw_data(handle, renderer.pixels(),
                    renderer.dimensions().0, renderer.dimensions().1, 4)
                    .map_err(|error| format!("SetOverlayRaw: {error:?}"))
            };
            match result {
                Ok(()) => { failures = 0; next_upload_attempt = Instant::now(); },
                Err(error) => {
                    failures += 1;
                    if last_warning.elapsed() >= Duration::from_secs(5) {
                        eprintln!("vrperf: image upload error: {error} (retrying)");
                        last_warning = Instant::now();
                    }
                    // A SteamVR RequestFailed error is not an OpenGL failure.
                    // Fast retries can flood SteamVR. Preserve the stable GL path.
                    if failures >= 3 {
                        next_upload_attempt = Instant::now() + Duration::from_secs(2);
                    }
                    if gpu.is_some() && failures >= 3 && error == "OpenGL texture upload failed" {
                        eprintln!("vrperf: local OpenGL texture upload failed; using raw fallback");
                        if let Some(gl) = gpu.take() {
                            if let Err(error) = gl.reset_texture_bounds(handle) {
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


fn change_mode(mode: config::Mode, config:&mut Config, renderer:&mut Renderer,
    overlays:&mut openvr::Overlay, handle:openvr::overlay::OverlayHandle,
    gpu:&mut Option<GlUploader>, positioner:&mut Positioner, system:&openvr::System,
    enabled:bool, grab:[Option<bool>;2]) -> Result<(),Box<dyn Error>> {
    config.mode=mode;
    if let Err(error)=registration::write_mode(mode) {
        eprintln!("vrperf: mode could not be saved: {error}");
    }
    *renderer=Renderer::for_mode(mode);
    overlays.set_width(handle,positioner.width(mode,config.width_m))
        .map_err(|e|format!("Cannot set overlay width: {e:?}"))?;
    let mut use_raw=false;
    if let Some(gl)=gpu.as_mut() {
        let (width,height)=renderer.dimensions();
        if let Err(error)=gl.set_dimensions(handle,width,height) {
            eprintln!("vrperf: texture bounds failed: {error}; switching to raw uploads");
            if let Err(error)=gl.reset_texture_bounds(handle) {
                eprintln!("vrperf: texture bounds reset failed: {error}");
            }
            use_raw=true;
        }
    }
    if use_raw {*gpu=None;}
    positioner.update(system,overlays,handle,config,enabled,grab)?;
    Ok(())
}

fn create_uploader(config: &Config, renderer: &Renderer,
    handle: openvr::overlay::OverlayHandle) -> Option<GlUploader> {
    if config.upload != Upload::Gl { return None; }
    let (width, height) = renderer.dimensions();
    // A single fixed allocation fits either head or wrist dimensions.
    // Mode switching updates only the visible texture coordinates.
    match GlUploader::new(render::WIDTH, render::WRIST_HEIGHT) {
        Ok(mut gl) => match gl.set_dimensions(handle, width, height) {
            Ok(()) => Some(gl),
            Err(error) => { eprintln!("vrperf: {error}; using raw uploads instead"); None }
        },
        Err(error) => { eprintln!("vrperf: {error}; using raw uploads instead"); None }
    }
}
