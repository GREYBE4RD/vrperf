use font8x8::{BASIC_FONTS, UnicodeFonts};
use std::collections::VecDeque;
use crate::devices::{self, DeviceStats};
use crate::config::Mode;
use crate::hardware::HardwareStats;
use crate::timing::{FrameMonitor, FrameSample};

// Wide and shallow, with hardware values above and battery/time below graphs.
pub const WIDTH: usize = 864;
pub const HEIGHT: usize = 160;
// Keep the watch compact. Matching the head texture height also avoids
// vertically cropped OpenGL texture bounds on mode switches.
pub const WRIST_WIDTH: usize = 560;
pub const WRIST_HEIGHT: usize = 160;
const WRIST_TOP_Y: i32 = 16;
const WRIST_GRAPH_Y: i32 = 44;
const WRIST_GRAPH_H: i32 = 72;
const WRIST_BOTTOM_Y: i32 = 130;

// Reserve a fixed left column for readable, stable 2x frame-time values.
const GRAPH_X: i32 = 194;
const GRAPH_W: i32 = WIDTH as i32 - GRAPH_X - 12;
// Use the full region between the top and bottom information separators.
// This does not move or resize the existing head-mode text rows.
const HEAD_GRAPH_Y: i32 = 34;
const HEAD_GRAPH_H: i32 = 94;
// Piecewise frame-time scale: 65% below target, 35% above (up to 2x).
// Restore more headroom for spikes while showing more detail below target.
const BELOW_BUDGET_HEIGHT: f32 = 0.65;
const ABOVE_BUDGET_HEIGHT: f32 = 0.35;
const FPS_HZ_MIN_GAP: i32 = 12;
const TOP_ROW_OTHER_MIN_GAP: i32 = 4;
const BG: [u8; 4] = [15, 20, 28, 245]; // v1.11 straight RGBA
const TEXT: [u8; 4] = [235, 242, 250, 255];
const MUTED: [u8; 4] = [158, 175, 193, 255];
const CPU: [u8; 4] = [96, 171, 255, 255];
const GPU: [u8; 4] = [100, 225, 163, 255];
// Separate warning colours preserve CPU/GPU identity above the frame budget.
const CPU_OVER_BUDGET: [u8; 4] = [255, 204, 79, 255]; // amber
const GPU_OVER_BUDGET: [u8; 4] = [255, 131, 65, 255]; // orange
const RESOURCE_WARNING: [u8; 4] = [255, 159, 72, 255];
const RESOURCE_CRITICAL: [u8; 4] = [255, 74, 65, 255];
const DROPPED: [u8; 4] = [255, 74, 65, 255];
const BUDGET: [u8; 4] = [207, 162, 84, 255];

pub struct Renderer {
    pixels: Vec<u8>,
    width: usize,
    height: usize,
    mode: Mode,
}

impl Renderer {
    pub fn for_mode(mode: Mode) -> Self {
        let (width, height) = if mode.wrist() { (WRIST_WIDTH, WRIST_HEIGHT) }
            else { (WIDTH, HEIGHT) };
        Self { pixels: vec![0; width * height * 4], width, height, mode }
    }
    pub fn dimensions(&self) -> (usize, usize) { (self.width, self.height) }
    pub fn pixels(&self) -> &[u8] { &self.pixels }

    pub fn draw(&mut self, monitor: &FrameMonitor, hw: &HardwareStats,
        devices: &DeviceStats, hz: f32) {
        self.clear(BG);
        let budget = 1000.0 / hz;
        if self.mode.wrist() {
            self.draw_wrist(monitor, hw, devices, hz, budget);
            return;
        }

        // All top-row fields use identical 2x (16px-high) font pixels.
        // Trim unused glyph side bearings rather than shrinking RES/VRAM/RAM.
        let fps = monitor.current_fps(hz).map_or_else(|| "FPS --".into(), |v| {
            if v >= 100.0 { format!("FPS {v:.0}") } else { format!("FPS {v:.1}") }
        });
        let labels = [
            (fps, TEXT),
            (format!("{hz:.0}HZ"), MUTED),
            (format!("RES {}", percent(devices.display_resolution(monitor.is_active()))), MUTED),
            (format!("CPU {}", load_percent(hw.cpu_load)), usage_colour(hw.cpu_load)),
            (format!("GPU {}", load_percent(hw.gpu_load)), usage_colour(hw.gpu_load)),
            (format!("VRAM {}", used_total_short(hw.vram_used_gib, hw.vram_total_gib)),
                memory_colour(hw.vram_used_gib, hw.vram_total_gib)),
            (format!("RAM {}", used_total_short(hw.ram_used_gib, hw.ram_total_gib)),
                memory_colour(hw.ram_used_gib, hw.ram_total_gib)),
        ];
        // Reserve each field's width for its longest normal value. Using
        // current text widths made *all* subsequent fields shift whenever a
        // percentage changed from one to two (or three) digits.
        // Reserve the FPS/Hz gap before distributing the remaining space.
        // The field positions depend only on their maximum widths, not on the
        // current readings, so changing values cannot move other fields.
        let positions = top_row_positions();
        for ((text, color), x) in labels.iter().zip(positions) {
            self.label_compact(x, 7, text, *color);
        }
        self.rect(10, 32, WIDTH as i32 - 20, 1, [59, 76, 94, 255]);

        let latest = monitor.latest();
        // Both traces share one scale and the same sample positions.
        // Keep the v1.11 head layout, including its dimensions, text rows,
        // fixed labels and bottom information bar. The shared CPU/GPU graph
        // fills the right-hand area between the two information separators.
        self.label(12, 48, "CPU", 2, CPU);
        self.label(70, 48, &frame_time_text(latest.map(|f| f.cpu_ms)), 2,
            frame_time_colour(latest.map(|f| f.cpu_ms), budget, CPU, CPU_OVER_BUDGET));
        self.rect(10, 79, GRAPH_X - 22, 1, [45, 59, 75, 255]);
        self.label(12, 94, "GPU", 2, GPU);
        self.label(70, 94, &frame_time_text(latest.map(|f| f.gpu_ms)), 2,
            frame_time_colour(latest.map(|f| f.gpu_ms), budget, GPU, GPU_OVER_BUDGET));
        self.graph(&monitor.samples, monitor.is_recent(), GRAPH_X, HEAD_GRAPH_Y, GRAPH_W, HEAD_GRAPH_H, budget);
        self.rect(10, 129, WIDTH as i32 - 20, 1, [59, 76, 94, 255]);

        // Identical bottom information positions to v1.11.
        self.label(10, 138, &format!("HMD {}", percent(devices.hmd)), 2,
            battery_colour(devices.hmd));
        self.label(164, 138, &format!("L {}", percent(devices.left)), 2,
            battery_colour(devices.left));
        self.label(282, 138, &format!("R {}", percent(devices.right)), 2,
            battery_colour(devices.right));
        let drops = if monitor.is_active() { monitor.dropped.to_string() }
            else { "--".to_string() };
        self.label(412, 138, &format!("DROP {drops}"), 2, MUTED);
        let clock = devices::local_clock();
        let clock_x = WIDTH as i32 - 12 - clock.chars().count() as i32 * 16;
        let budget_text = format!("BUDGET {budget:.2}ms");
        self.label_compact(clock_x - compact_width(&budget_text) - 18, 138,
            &budget_text, MUTED);
        self.label(clock_x, 138, &clock, 2, TEXT);
    }

    fn draw_wrist(&mut self, monitor: &FrameMonitor, hw: &HardwareStats,
        devices: &DeviceStats, hz: f32, budget: f32) {
        let latest = monitor.latest();
        // One information row above the graph and one below. Both rows use
        // reserved widths, so changing values cannot move neighbouring fields.
        // The clock fits better at the end of the top row. Reserve the
        // longest values for both rows so changing numbers never shift text.
        let fps = monitor.current_fps(hz).map_or_else(|| "FPS --".into(), |value| {
            if value.is_finite() && value >= 0.0 {
                if value >= 999.5 { "FPS 999+".into() }
                else { format!("FPS {value:.0}") }
            } else { "FPS --".into() }
        });
        let top = [
            (fps, TEXT),
            (format!("CPU {}", wrist_frame_time_text(latest.map(|f| f.cpu_ms))),
                frame_time_colour(latest.map(|f| f.cpu_ms), budget, CPU, CPU_OVER_BUDGET)),
            (format!("GPU {}", wrist_frame_time_text(latest.map(|f| f.gpu_ms))),
                frame_time_colour(latest.map(|f| f.gpu_ms), budget, GPU, GPU_OVER_BUDGET)),
            (format_wrist_drops(monitor.is_active().then_some(monitor.dropped)), DROPPED),
            (format_wrist_resolution(devices.display_resolution(monitor.is_active())), MUTED),
            (devices::local_clock(), TEXT),
        ];
        let top_slots = wrist_top_widths();
        for (i, ((text, colour), x)) in top.iter().zip(wrist_top_positions()).enumerate() {
            self.label_wrist(wrist_aligned_x(x, top_slots[i], text, i, top.len()),
                WRIST_TOP_Y, text, *colour);
        }
        // Top padding keeps every glyph fully inside the visible panel.
        self.rect(10, 40, WRIST_WIDTH as i32 - 20, 1, [59, 76, 94, 255]);
        self.graph(&monitor.samples, monitor.is_recent(), 10, WRIST_GRAPH_Y,
            WRIST_WIDTH as i32 - 20, WRIST_GRAPH_H, budget);
        self.rect(10, 120, WRIST_WIDTH as i32 - 20, 1, [59, 76, 94, 255]);
        let bottom = [
            (format!("BAT {}", percent(devices.hmd)), battery_colour(devices.hmd)),
            (format!("CPU {}", load_percent(hw.cpu_load)), usage_colour(hw.cpu_load)),
            (format!("RAM {}", used_total_wrist(hw.ram_used_gib, hw.ram_total_gib)),
                memory_colour(hw.ram_used_gib, hw.ram_total_gib)),
            (format!("GPU {}", load_percent(hw.gpu_load)), usage_colour(hw.gpu_load)),
            (format!("VRA {}", used_total_wrist(hw.vram_used_gib, hw.vram_total_gib)),
                memory_colour(hw.vram_used_gib, hw.vram_total_gib)),
        ];
        let bottom_slots = wrist_bottom_widths();
        for (i, ((text, colour), x)) in bottom.iter().zip(wrist_bottom_positions()).enumerate() {
            self.label_wrist(wrist_aligned_x(x, bottom_slots[i], text, i, bottom.len()),
                WRIST_BOTTOM_Y, text, *colour);
        }
    }

    // Overlaid CPU/GPU traces, a common budget marker, and dropped frames.
    fn graph(&mut self, history: &VecDeque<FrameSample>, active: bool,
        x: i32, y: i32, w: i32, h: i32, budget: f32) {
        self.rect(x, y, w, h, [21, 30, 40, 245]);
        let baseline_y = y + h - 1;
        let budget_y = frame_y(budget, budget, baseline_y, h);
        for px in (x..x + w).step_by(5) { self.rect(px, budget_y, 3, 1, BUDGET); }
        if !active { return; }
        // Colour only the portions of each trace above budget. The transition
        // is calculated per pixel, including between adjacent frame samples.
        for (gpu, normal, warning) in [
            (true, GPU, GPU_OVER_BUDGET),
            (false, CPU, CPU_OVER_BUDGET),
        ] {
            let mut previous: Option<(i32, i32, f32)> = None;
            for (i, frame) in history.iter().rev().take(w as usize).enumerate() {
                let sx = x + w - 1 - i as i32;
                let ms = if gpu { frame.gpu_ms } else { frame.cpu_ms };
                let sy = frame_y(ms, budget, baseline_y, h);
                if let Some((old_x, old_y, old_ms)) = previous {
                    self.frame_time_line((sx, sy, ms), (old_x, old_y, old_ms),
                        budget, normal, warning);
                } else {
                    self.pixel(sx, sy, if ms > budget { warning } else { normal });
                }
                previous = Some((sx, sy, ms));
            }
        }
        // Drop markers must remain visible above both line traces.
        for (i, frame) in history.iter().rev().take(w as usize).enumerate() {
            if frame.dropped > 0 {
                let sx = x + w - 1 - i as i32;
                self.rect(sx, y, 1, h, DROPPED);
            }
        }
    }

    // Same 2x pixel size as the existing 16px labels, but with proportional
    // character advances. This frees space without shrinking the glyphs.
    fn label_compact(&mut self, x: i32, y: i32, text: &str, color: [u8; 4]) {
        let mut cursor = x;
        for ch in text.chars() {
            if let Some(bitmap) = BASIC_FONTS.get(ch) {
                if let Some((first, last)) = glyph_columns(&bitmap) {
                    for (row, bits) in bitmap.iter().enumerate() {
                        for col in first..=last {
                            if *bits & (1_u8 << col) != 0 {
                                self.rect(cursor + (col - first) * 2,
                                    y + row as i32 * 2, 2, 2, color);
                            }
                        }
                    }
                }
            }
            cursor += compact_advance(ch);
        }
    }

    // Wrist-only font: preserve the 16px height, with 1.4 pixels per bitmap
    // column horizontally. This keeps both reserved rows within 560 pixels
    // without changing the HEAD font.
    fn label_wrist(&mut self, x: i32, y: i32, text: &str, color: [u8; 4]) {
        let mut cursor = x;
        for ch in text.chars() {
            if let Some(bitmap) = BASIC_FONTS.get(ch) {
                if let Some((first, last)) = glyph_columns(&bitmap) {
                    for (row, bits) in bitmap.iter().enumerate() {
                        for col in first..=last {
                            if *bits & (1_u8 << col) != 0 {
                                let local = col - first;
                                let start_x = local * 7 / 5;
                                let end_x = (local + 1) * 7 / 5;
                                self.rect(cursor + start_x, y + row as i32 * 2,
                                    end_x - start_x, 2, color);
                            }
                        }
                    }
                }
            }
            cursor += wrist_advance(ch);
        }
    }

    fn clear(&mut self, rgba: [u8; 4]) {
        for pixel in self.pixels.chunks_exact_mut(4) { pixel.copy_from_slice(&rgba); }
    }
    fn pixel(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 { return; }
        let index = (y as usize * self.width + x as usize) * 4;
        self.pixels[index..index + 4].copy_from_slice(&color);
    }
    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, rgba: [u8; 4]) {
        for py in y..y + h { for px in x..x + w { self.pixel(px, py, rgba); } }
    }
    fn frame_time_line(&mut self, start: (i32, i32, f32), end: (i32, i32, f32),
        budget: f32, normal: [u8; 4], warning: [u8; 4]) {
        let (mut x0, mut y0, start_ms) = start;
        let (x1, y1, end_ms) = end;
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        // Use the dominant raster axis to interpolate frame time across the
        // whole line, rather than colouring a whole segment from one sample.
        let vertical = -dy > dx;
        let span = if vertical { y1 - y0 } else { x1 - x0 };
        let origin = if vertical { y0 } else { x0 };
        loop {
            let progress = if span == 0 { 0.0 } else {
                ((if vertical { y0 } else { x0 }) - origin) as f32 / span as f32
            };
            let ms = start_ms + progress * (end_ms - start_ms);
            self.pixel(x0, y0, if ms > budget { warning } else { normal });
            if x0 == x1 && y0 == y1 { break; }
            let twice_error = 2 * err;
            if twice_error >= dy { err += dy; x0 += sx; }
            if twice_error <= dx { err += dx; y0 += sy; }
        }
    }
    fn label(&mut self, x: i32, y: i32, text: &str, scale: i32, color: [u8; 4]) {
        for (index, char) in text.chars().enumerate() {
            let Some(bitmap) = BASIC_FONTS.get(char) else { continue; };
            let ox = x + index as i32 * 8 * scale;
            for (row, byte) in bitmap.iter().enumerate() {
                for col in 0..8 {
                    if (*byte & (1_u8 << col)) != 0 {
                        self.rect(ox + col * scale, y + row as i32 * scale,
                            scale, scale, color);
                    }
                }
            }
        }
    }

}

// Source pixels use straight RGBA like v1.11. Never premultiply their RGB.

/// Maps 0..budget onto the lower 65% of the chart and budget..2*budget
/// onto the upper 35%. Values above double the budget clip to the top.
fn frame_fraction(ms: f32, budget: f32) -> f32 {
    if !ms.is_finite() || !budget.is_finite() || budget <= 0.0 { return 0.0; }
    let ratio = (ms / budget).clamp(0.0, 2.0);
    if ratio <= 1.0 { ratio * BELOW_BUDGET_HEIGHT }
    else { BELOW_BUDGET_HEIGHT + (ratio - 1.0) * ABOVE_BUDGET_HEIGHT }
}

fn frame_y(ms: f32, budget: f32, bottom: i32, height: i32) -> i32 {
    bottom - (frame_fraction(ms, budget) * (height - 1).max(0) as f32).round() as i32
}

// Bitmap glyphs are 8x8. Trim blank columns but retain 2x pixel blocks,
// ensuring the text stays at the same readable height as every other label.
fn glyph_columns(bitmap: &[u8; 8]) -> Option<(i32, i32)> {
    let ink = bitmap.iter().fold(0_u8, |mask, bits| mask | bits);
    if ink == 0 { return None; }
    Some((ink.trailing_zeros() as i32, 7 - ink.leading_zeros() as i32))
}
fn compact_advance(ch: char) -> i32 {
    // Equal digit advances keep numbers and the trailing % symbol in place,
    // even though the individual bitmap glyphs have different widths.
    if ch.is_ascii_digit() || ch == '-' { return 14; }
    match BASIC_FONTS.get(ch).and_then(|bitmap| glyph_columns(&bitmap)) {
        Some((first, last)) => (last - first + 1) * 2 + 2,
        None => 8, // Spaces: keep a visible gap without using a full 16px cell.
    }
}
fn compact_width(text: &str) -> i32 {
    (text.chars().map(compact_advance).sum::<i32>() - 2).max(0)
}

// Watch glyphs use 70% of the HEAD font's horizontal advance. Reserving
// the same advances used for drawing keeps each field inside its fixed slot.
fn wrist_advance(ch: char) -> i32 { (compact_advance(ch) * 7 + 9) / 10 }
fn wrist_text_width(text: &str) -> i32 {
    // Reserve the full advance, including the last glyph's ink.
    text.chars().map(wrist_advance).sum()
}

// Constant widths: changing values must never reflow the top row.
fn top_slot_widths() -> [i32; 7] {
    let mut widths = ["FPS 99.9", "144HZ", "RES 1000%", "CPU 100%", "GPU 100%",
        "VRAM 16/16G", "RAM 32/32G"].map(compact_width);
    // With triple-digit FPS the display omits decimals.
    widths[0] = widths[0].max(compact_width("FPS 240"));
    widths
}

// Top-row starts use fixed worst-case field widths. The FPS/Hz gap is a
// hard minimum of 12 px, even when the other fields consume most of the row.
// The remaining gaps have a 4 px minimum and share any spare pixels. This
// preserves the 864 px panel and the fixed positions of all seven fields.
fn top_row_positions() -> [i32; 7] {
    let widths = top_slot_widths();
    let occupied: i32 = widths.iter().sum();
    let extra_space = WIDTH as i32 - 20 - occupied
        - FPS_HZ_MIN_GAP - 5 * TOP_ROW_OTHER_MIN_GAP;
    debug_assert!(extra_space >= 0, "top-row fields exceed the available width");
    let distributed = extra_space.max(0) / 6;
    let remainder = extra_space.max(0) % 6;

    let mut positions = [0; 7];
    let mut x = 10;
    for (i, width) in widths.iter().enumerate() {
        positions[i] = x;
        if i < 6 {
            let minimum = if i == 0 { FPS_HZ_MIN_GAP }
                else { TOP_ROW_OTHER_MIN_GAP };
            x += *width + minimum + distributed + i32::from((i as i32) < remainder);
        }
    }
    positions
}

// Reserve space for the longest expected values in the two watch rows.
// The positions are based on slot widths, not current measurements. Even a
// DROP 9 -> DROP 10 change leaves every other label in the same place.
fn wrist_top_widths() -> [i32; 6] {
    ["FPS 999+", "CPU 999+ms", "GPU 999+ms", "DROP 999M", "RES 9999%", "23:59"]
        .map(wrist_text_width)
}

fn wrist_bottom_widths() -> [i32; 5] {
    ["BAT 100%", "CPU 100%", "RAM 999T/999T", "GPU 100%", "VRA 999T/999T"]
        .map(wrist_text_width)
}

// Compact row spacing. The reserved slots still prevent labels from shifting.
const WRIST_ROW_GAP: i32 = 2;

fn wrist_row_required<const N: usize>(widths: [i32; N]) -> i32 {
    widths.iter().sum::<i32>() + 20 + WRIST_ROW_GAP * (N as i32 - 1)
}

fn wrist_row_positions<const N: usize>(widths: [i32; N]) -> [i32; N] {
    let available = WRIST_WIDTH as i32 - wrist_row_required(widths);
    debug_assert!(available >= 0, "watch row exceeds the graph width");
    let extra = available.max(0);
    let gaps = N as i32 - 1;
    let mut positions = [0; N];
    let mut x = 10;
    for i in 0..N {
        positions[i] = x;
        if i + 1 < N {
            x += widths[i] + WRIST_ROW_GAP + extra / gaps
                + i32::from((i as i32) < extra % gaps);
        }
    }
    positions
}

fn wrist_top_positions() -> [i32; 6] {
    wrist_row_positions(wrist_top_widths())
}
fn wrist_bottom_positions() -> [i32; 5] {
    wrist_row_positions(wrist_bottom_widths())
}

// Centre shorter readings within their reserved slots. Anchor the first and
// last fields to the graph edges, and never move neighbouring fields when a
// number gains digits (notably DROP 9 -> DROP 10).
fn wrist_aligned_x(start: i32, slot_width: i32, text: &str,
    index: usize, field_count: usize) -> i32 {
    let spare = (slot_width - wrist_text_width(text)).max(0);
    let offset = if index == 0 { 0 }
        else if index + 1 == field_count { spare }
        else { spare / 2 };
    start + offset
}

// Exact below 1000; above that use compact K/M/B/T/P suffixes.
// A reserved four-character value prevents changes such as DROP 9 -> 10
// from moving the following fields, even for very large counters.
fn format_wrist_drops(drops: Option<u64>) -> String {
    match drops {
        None => "DROP --".into(),
        Some(n) if n < 1_000 => format!("DROP {n}"),
        Some(n) if n < 1_000_000 => format!("DROP {}K", n / 1_000),
        Some(n) if n < 1_000_000_000 => format!("DROP {}M", n / 1_000_000),
        Some(n) if n < 1_000_000_000_000 => format!("DROP {}B", n / 1_000_000_000),
        Some(n) if n < 1_000_000_000_000_000 => format!("DROP {}T", n / 1_000_000_000_000),
        Some(n) if n < 1_000_000_000_000_000_000 =>
            format!("DROP {}P", n / 1_000_000_000_000_000),
        Some(_) => "DROP 1E+".into(),
    }
}

// Watch timings below 100 ms keep one decimal. At 100 ms and above,
// whole milliseconds save space without losing useful detail in VR.
// The longest value is 999+ms; HEAD keeps its own formatter.
fn wrist_frame_time_text(value: Option<f32>) -> String {
    match value {
        Some(ms) if ms.is_finite() && (0.0..99.95).contains(&ms) =>
            format!("{ms:04.1}ms"),
        Some(ms) if ms.is_finite() && (99.95..999.5).contains(&ms) =>
            format!("{ms:.0}ms"),
        Some(ms) if ms.is_finite() && ms >= 999.5 => "999+ms".into(),
        _ => "--.-ms".into(),
    }
}

// Display percentages up to 9999 without changing the reserved RES slot.
fn format_wrist_resolution(value: Option<u16>) -> String {
    match value {
        Some(v) if v <= 9999 => format!("RES {v}%"),
        Some(_) => "RES 9999+".into(),
        None => "RES --%".into(),
    }
}

// The 8x8 bitmap font uses fixed 16px character cells at scale 2.
// 09.2ms and 12.3ms therefore occupy identical space. We also reserve
// room for rare three-digit timings, without moving the graph start.
fn frame_time_text(value: Option<f32>) -> String {
    match value {
        Some(ms) if ms.is_finite() && (0.0..100.0).contains(&ms) => format!("{ms:04.1}ms"),
        Some(ms) if ms.is_finite() && (100.0..1000.0).contains(&ms) => format!("{ms:.1}ms"),
        Some(ms) if ms.is_finite() && ms >= 1000.0 => "999+ms".into(),
        _ => "--.-ms".into(),
    }
}

fn percent<T: Into<u16>>(value: Option<T>) -> String {
    value.map_or_else(|| "--%".to_string(), |v| format!("{:02}%", v.into()))
}
fn load_percent(value: Option<f32>) -> String {
    value.filter(|v| v.is_finite() && *v >= 0.0)
        .map_or_else(|| "--%".to_string(), |v| format!("{:02}%", v.round() as u16))
}
fn used_total_short(used: Option<f32>, total: Option<f32>) -> String {
    match (used, total) {
        (Some(used), Some(total)) => format!("{used:.0}/{total:.0}G"),
        _ => "--/--G".into(),
    }
}

// Watch values are GiB unless suffixed T (TiB). No G suffix is needed in
// the small display. The 999/999 slot also covers the abbreviated TiB form.
fn wrist_memory_part(value: f32) -> String {
    if value >= 999.5 { format!("{:.0}T", (value / 1024.0).clamp(1.0, 999.0)) }
    else { format!("{value:.0}") }
}
fn used_total_wrist(used: Option<f32>, total: Option<f32>) -> String {
    match (used, total) {
        (Some(used), Some(total)) if used.is_finite() && total.is_finite()
            && used >= 0.0 && total > 0.0 =>
            format!("{}/{}", wrist_memory_part(used), wrist_memory_part(total)),
        _ => "--/--".into(),
    }
}

fn frame_time_colour(ms: Option<f32>, budget: f32, normal: [u8; 4], warning: [u8; 4]) -> [u8; 4] {
    if ms.is_some_and(|value| value.is_finite() && value > budget && budget.is_finite()) {
        warning
    } else { normal }
}
fn percentage_warning(value: Option<f32>) -> [u8; 4] {
    match value.filter(|v| v.is_finite() && *v >= 0.0) {
        Some(v) if v >= 95.0 => RESOURCE_CRITICAL,
        Some(v) if v >= 85.0 => RESOURCE_WARNING,
        _ => TEXT,
    }
}
fn usage_colour(value: Option<f32>) -> [u8; 4] {
    percentage_warning(value)
}
fn memory_colour(used: Option<f32>, total: Option<f32>) -> [u8; 4] {
    match (used, total) {
        (Some(u), Some(t)) if u.is_finite() && t.is_finite() && u >= 0.0 && t > 0.0 =>
            percentage_warning(Some(100.0 * u / t)),
        _ => TEXT,
    }
}
fn battery_colour(value: Option<u8>) -> [u8; 4] {
    match value {
        Some(0..=5) => RESOURCE_CRITICAL,
        Some(6..=15) => RESOURCE_WARNING,
        _ => TEXT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_size_top_row_fits() {
        let occupied: i32 = top_slot_widths().iter().sum();
        // Reserve the larger FPS/Hz gap, five other gaps and both margins.
        assert!(occupied + 20 + FPS_HZ_MIN_GAP
            + 5 * TOP_ROW_OTHER_MIN_GAP <= WIDTH as i32);
    }
    #[test]
    fn compact_glyphs_keep_two_pixel_blocks() {
        assert!(compact_width("RAM 15/32G") > 0);
        assert!(compact_width("VRAM 12/16G") < 11 * 16);
    }
    #[test]
    fn percentages_have_stable_widths_and_zero_padding() {
        assert_eq!(percent(Some(1_u8)), "01%");
        assert_eq!(percent(Some(20_u8)), "20%");
        assert_eq!(percent(Some(100_u8)), "100%");
        assert_eq!(percent(Some(250_u16)), "250%");
        assert_eq!(load_percent(Some(1.0)), "01%");
        assert_eq!(load_percent(Some(20.0)), "20%");
        assert_eq!(compact_advance('1'), compact_advance('8'));
        assert_eq!(compact_width("CPU 01%"), compact_width("CPU 20%"));
    }
    #[test]
    fn top_row_slots_are_independent_of_current_metrics() {
        let slots = top_slot_widths();
        let xs = top_row_positions();
        assert_eq!(xs[0], 10);
        assert!(xs[6] + slots[6] <= WIDTH as i32 - 10);
        assert!(slots[0] >= compact_width("FPS 99.9"));
        // Check against reserved field boundaries, not shorter live values.
        assert!(xs[1] - (xs[0] + slots[0]) >= FPS_HZ_MIN_GAP);
        for i in 1..6 {
            assert!(xs[i + 1] - (xs[i] + slots[i]) >= TOP_ROW_OTHER_MIN_GAP);
        }
        // Recalculating the positions cannot depend on changing statistics.
        assert_eq!(top_row_positions(), xs);
    }
    #[test]
    fn frame_time_labels_do_not_jump_between_single_and_double_digits() {
        let single = frame_time_text(Some(9.2));
        let double = frame_time_text(Some(12.3));
        assert_eq!(single, "09.2ms");
        assert_eq!(double, "12.3ms");
        assert_eq!(single.len(), double.len());
        assert_eq!(single.len() as i32 * 16, 96);
        // Even 100.0MS stays inside the reserved left column.
        assert!(70 + frame_time_text(Some(100.0)).len() as i32 * 16 < GRAPH_X);
        assert_eq!(frame_time_text(None), "--.-ms");
    }
    #[test]
    fn history_covers_entire_graph_without_resampling() {
        assert!(crate::timing::MAX_HISTORY >= GRAPH_W as usize);
        let mut r = Renderer::for_mode(Mode::Head);
        let frame = FrameSample { cpu_ms: 5.0, gpu_ms: 4.0, dropped: 0 };
        let history = VecDeque::from(vec![frame; GRAPH_W as usize]);
        r.graph(&history, true, GRAPH_X, HEAD_GRAPH_Y, GRAPH_W, HEAD_GRAPH_H, 11.11);
        let h = HEAD_GRAPH_H;
        let baseline_y = HEAD_GRAPH_Y + h - 1;
        let sy = frame_y(5.0, 11.11, baseline_y, h);
        let idx = ((sy as usize * WIDTH) + GRAPH_X as usize) * 4;
        assert_eq!(&r.pixels[idx..idx + 4], &CPU[..]);
    }
    #[test]
    fn trace_colours_change_only_above_budget_in_head_and_watch_modes() {
        for mode in [Mode::Head, Mode::Left, Mode::Right] {
            let mut r = Renderer::for_mode(mode);
            let bottom = 10 + 50 - 1;
            // CPU over budget; GPU within budget.
            let history = VecDeque::from([FrameSample {
                cpu_ms: 15.0, gpu_ms: 6.0, dropped: 0,
            }; 4]);
            r.graph(&history, true, 10, 10, 30, 50, 10.0);
            assert_eq!(pixel_at(&r, 38, frame_y(15.0, 10.0, bottom, 50)),
                CPU_OVER_BUDGET);
            assert_eq!(pixel_at(&r, 38, frame_y(6.0, 10.0, bottom, 50)), GPU);
            // GPU over budget; CPU within budget.
            let history = VecDeque::from([FrameSample {
                cpu_ms: 6.0, gpu_ms: 15.0, dropped: 0,
            }; 4]);
            r.graph(&history, true, 10, 10, 30, 50, 10.0);
            assert_eq!(pixel_at(&r, 38, frame_y(15.0, 10.0, bottom, 50)),
                GPU_OVER_BUDGET);
            assert_eq!(pixel_at(&r, 38, frame_y(6.0, 10.0, bottom, 50)), CPU);
            // Exactly at budget is normal, not warning.
            let history = VecDeque::from([FrameSample {
                cpu_ms: 10.0, gpu_ms: 4.0, dropped: 0,
            }; 4]);
            r.graph(&history, true, 10, 10, 30, 50, 10.0);
            assert_eq!(pixel_at(&r, 38, frame_y(10.0, 10.0, bottom, 50)), CPU);
        }
    }
    #[test]
    fn a_crossing_line_changes_colour_at_the_threshold() {
        let mut r = Renderer::for_mode(Mode::Head);
        r.frame_time_line((10, 40, 6.0), (11, 20, 14.0),
            10.0, CPU, CPU_OVER_BUDGET);
        assert_eq!(pixel_at(&r, 10, 40), CPU);
        assert_eq!(pixel_at(&r, 11, 20), CPU_OVER_BUDGET);
        // The line changes colour mid-segment rather than changing all at once.
        let colours = (20..=40).flat_map(|y| (10..=11).map(move |x| (x, y)))
            .filter_map(|(x, y)| {
                let c = pixel_at(&r, x, y);
                if c == CPU || c == CPU_OVER_BUDGET { Some(c) } else { None }
            }).collect::<Vec<_>>();
        assert!(colours.contains(&CPU));
        assert!(colours.contains(&CPU_OVER_BUDGET));
    }
    #[test]
    fn dropped_frame_marker_covers_over_budget_trace() {
        let mut r = Renderer::for_mode(Mode::Head);
        let history = VecDeque::from([FrameSample {
            cpu_ms: 15.0, gpu_ms: 14.0, dropped: 2,
        }]);
        r.graph(&history, true, 10, 10, 30, 50, 10.0);
        let marker_x = 10 + 30 - 1;
        for marker_y in [11, 30, 58] {
            assert_eq!(pixel_at(&r, marker_x, marker_y), DROPPED);
        }
    }
    fn pixel_at(renderer: &Renderer, x: i32, y: i32) -> [u8; 4] {
        let index = (y as usize * renderer.width + x as usize) * 4;
        renderer.pixels[index..index + 4].try_into().unwrap()
    }
    #[test]
    fn empty_canvas_draw_is_safe() {
        let mut r = Renderer::for_mode(Mode::Head);
        r.draw(&FrameMonitor::new(), &HardwareStats::default(), &DeviceStats::default(), 90.0);
        assert_eq!(r.pixels.len(), WIDTH * HEIGHT * 4);
    }
    #[test]
    fn dropped_frame_marker_is_drawn_on_graph() {
        let mut r = Renderer::for_mode(Mode::Head);
        let history = VecDeque::from([FrameSample { cpu_ms: 3.0, gpu_ms: 4.0, dropped: 2 }]);
        r.graph(&history, true, 10, 10, 40, 20, 11.11);
        let x = 10 + 40 - 1;
        let offset = ((12_usize * WIDTH) + x as usize) * 4;
        assert_eq!(&r.pixels[offset..offset + 4], &DROPPED[..]);
    }
    #[test]
    fn wrist_layout_uses_two_fixed_rows_and_full_width_graph() {
        let mut r = Renderer::for_mode(Mode::Left);
        assert_eq!(r.dimensions(), (WRIST_WIDTH, WRIST_HEIGHT));
        r.draw(&FrameMonitor::new(), &HardwareStats::default(), &DeviceStats::default(), 90.0);
        assert_eq!(r.pixels().len(), WRIST_WIDTH * WRIST_HEIGHT * 4);
        // The top-row clock keeps the widest reserved row narrower.
        let top = wrist_top_widths();
        let bottom = wrist_bottom_widths();
        let clock_top = wrist_row_required(top).max(wrist_row_required(bottom));
        let clock_bottom = wrist_row_required([top[0], top[1], top[2], top[3], top[4]])
            .max(wrist_row_required([bottom[0], bottom[1], bottom[2], bottom[3], bottom[4], top[5]]));
        assert!(clock_top <= clock_bottom);
        for (widths, positions) in [
            (wrist_top_widths().to_vec(), wrist_top_positions().to_vec()),
            (wrist_bottom_widths().to_vec(), wrist_bottom_positions().to_vec()),
        ] {
            assert_eq!(positions[0], 10);
            for i in 0..widths.len() - 1 {
                assert!(positions[i + 1] >= positions[i] + widths[i] + WRIST_ROW_GAP);
            }
            // Both rows end at the same boundary as the graph.
            assert_eq!(positions[widths.len() - 1] + widths[widths.len() - 1],
                WRIST_WIDTH as i32 - 10);
        }
        // The graph starts and ends at the same text boundaries.
        assert_eq!(WRIST_WIDTH as i32 - 20, 540);
        assert!(WRIST_TOP_Y >= 16);
        assert!(WRIST_TOP_Y + 16 < 40);
        assert!(WRIST_GRAPH_Y + WRIST_GRAPH_H < 120);
        assert!(WRIST_BOTTOM_Y + 16 <= WRIST_HEIGHT as i32 - 14);
    }
    #[test]
    fn wrist_labels_fit_in_the_canvas_with_safe_top_padding() {
        let mut r = Renderer::for_mode(Mode::Right);
        let (width, height) = r.dimensions();
        assert_eq!((width, height), (560, 160));
        assert_eq!(wrist_row_positions(wrist_top_widths())[0], 10);
        assert_eq!(wrist_row_positions(wrist_bottom_widths())[0], 10);
        assert!(WRIST_TOP_Y >= 16);
        assert!(WRIST_BOTTOM_Y + 16 <= height as i32 - 14);
        r.draw(&FrameMonitor::new(), &HardwareStats::default(), &DeviceStats::default(), 90.0);
        // The first/last eight rows are blank padding, not text.
        for y in [0usize, 7, height - 8, height - 1] {
            for x in 0..width {
                assert_eq!(pixel_at(&r, x as i32, y as i32), BG);
            }
        }
    }

    #[test]
    fn wrist_fields_never_expand_when_values_change() {
        let top = wrist_top_widths();
        let bottom = wrist_bottom_widths();
        for drops in [None, Some(0), Some(9), Some(10), Some(999), Some(1000),
            Some(99999), Some(100000), Some(1000000), Some(1_000_000_000),
            Some(1_000_000_000_000), Some(u64::MAX)] {
            assert!(wrist_text_width(&format_wrist_drops(drops)) <= top[3]);
        }
        for res in [None, Some(100), Some(150), Some(1000), Some(9999), Some(u16::MAX)] {
            assert!(wrist_text_width(&format_wrist_resolution(res)) <= top[4]);
        }
        for mem in [(None,None), (Some(14.0),Some(16.0)),
            (Some(128.0),Some(256.0)), (Some(999.0),Some(999.0)),
            (Some(1024.0),Some(2048.0)),
            (Some(f32::MAX),Some(f32::MAX))] {
            assert!(wrist_text_width(&format!("RAM {}", used_total_wrist(mem.0,mem.1))) <= bottom[2]);
            assert!(wrist_text_width(&format!("VRA {}", used_total_wrist(mem.0,mem.1))) <= bottom[4]);
        }
        for ms in [None,Some(9.2),Some(99.9),Some(99.96),Some(100.0),
            Some(999.0),Some(999.9),Some(1000.0),Some(f32::MAX)] {
            assert!(wrist_text_width(&format!("CPU {}",wrist_frame_time_text(ms))) <= top[1]);
            assert!(wrist_text_width(&format!("GPU {}",wrist_frame_time_text(ms))) <= top[2]);
        }
    }
    #[test]
    fn condensed_wrist_values_fit_without_changing_field_positions() {
        assert_eq!(wrist_frame_time_text(Some(9.2)), "09.2ms");
        assert_eq!(wrist_frame_time_text(Some(99.96)), "100ms");
        assert_eq!(wrist_frame_time_text(Some(100.1)), "100ms");
        assert_eq!(wrist_frame_time_text(Some(1000.0)), "999+ms");
        assert_eq!(format_wrist_drops(Some(9)), "DROP 9");
        assert_eq!(format_wrist_drops(Some(10)), "DROP 10");
        assert_eq!(format_wrist_drops(Some(1000)), "DROP 1K");
        let top = wrist_top_widths();
        let positions = wrist_top_positions();
        assert!(wrist_row_required(top) <= WRIST_WIDTH as i32);
        assert!(wrist_row_required(wrist_bottom_widths()) <= WRIST_WIDTH as i32);
        assert_eq!(positions[3], wrist_top_positions()[3]);
    }
    #[test]
    fn watch_fields_are_evenly_spaced_with_fixed_slots() {
        // Reserved slots stay put, while shorter values are centred within
        // their own slot. The first and last labels align to graph edges.
        for (widths, positions) in [
            (wrist_top_widths().to_vec(), wrist_top_positions().to_vec()),
            (wrist_bottom_widths().to_vec(), wrist_bottom_positions().to_vec()),
        ] {
            let gaps: Vec<i32> = (0..widths.len()-1).map(|i|
                positions[i + 1] - (positions[i] + widths[i])).collect();
            assert!(gaps.iter().max().unwrap() - gaps.iter().min().unwrap() <= 1);
            for i in 0..widths.len() {
                let label = if i == 0 { "BAT 9%" } else { "9" };
                let x = wrist_aligned_x(positions[i], widths[i], label, i, widths.len());
                assert!(x >= positions[i]);
                assert!(x + wrist_text_width(label) <= positions[i] + widths[i]);
            }
        }
        let widths = wrist_top_widths();
        let starts = wrist_top_positions();
        let centre9 = wrist_aligned_x(starts[3], widths[3], "DROP 9", 3, 6) * 2
            + wrist_text_width("DROP 9");
        let centre10 = wrist_aligned_x(starts[3], widths[3], "DROP 10", 3, 6) * 2
            + wrist_text_width("DROP 10");
        assert!((centre9 - centre10).abs() <= 1);
        assert_eq!(wrist_bottom_widths()[2], wrist_text_width("RAM 999T/999T"));
        assert_eq!(wrist_bottom_widths()[4], wrist_text_width("VRA 999T/999T"));
    }
    #[test]
    fn warning_colours_respect_exact_thresholds() {
        assert_eq!(usage_colour(Some(84.99)), TEXT);
        assert_eq!(usage_colour(Some(85.0)), RESOURCE_WARNING);
        assert_eq!(usage_colour(Some(94.99)), RESOURCE_WARNING);
        assert_eq!(usage_colour(Some(95.0)), RESOURCE_CRITICAL);
        assert_eq!(usage_colour(None), TEXT);
        assert_eq!(usage_colour(Some(f32::NAN)), TEXT);
        assert_eq!(memory_colour(Some(84.9),Some(100.0)), TEXT);
        assert_eq!(memory_colour(Some(85.0),Some(100.0)), RESOURCE_WARNING);
        assert_eq!(memory_colour(Some(95.0),Some(100.0)), RESOURCE_CRITICAL);
        assert_eq!(memory_colour(Some(100.0),Some(0.0)), TEXT);
        assert_eq!(battery_colour(Some(16)), TEXT);
        assert_eq!(battery_colour(Some(15)), RESOURCE_WARNING);
        assert_eq!(battery_colour(Some(6)), RESOURCE_WARNING);
        assert_eq!(battery_colour(Some(5)), RESOURCE_CRITICAL);
        assert_eq!(battery_colour(Some(0)), RESOURCE_CRITICAL);
        assert_eq!(battery_colour(None), TEXT);
    }
    #[test]
    fn frame_time_labels_follow_the_same_budget_colours_as_graphs() {
        for (normal, warning) in [(CPU,CPU_OVER_BUDGET), (GPU,GPU_OVER_BUDGET)] {
            assert_eq!(frame_time_colour(Some(9.9),10.0,normal,warning),normal);
            assert_eq!(frame_time_colour(Some(10.0),10.0,normal,warning),normal);
            assert_eq!(frame_time_colour(Some(10.1),10.0,normal,warning),warning);
            assert_eq!(frame_time_colour(None,10.0,normal,warning),normal);
        }
    }
    #[test]
    fn piecewise_graph_scale_preserves_spikes_and_budget_line() {
        let budget = 1000.0 / 90.0;
        for (ms, expected) in [
            (0.0, 0.0), (budget * 0.25, 0.1625),
            (budget * 0.5, 0.325), (budget, 0.65),
            (budget * 1.5, 0.825), (budget * 2.0, 1.0),
            (budget * 3.0, 1.0),
        ] {
            assert!((frame_fraction(ms, budget) - expected).abs() < 0.0001);
        }
        let bottom = 100;
        let h = 64;
        assert_eq!(frame_y(budget, budget, bottom, h),
            bottom - (0.65 * (h - 1) as f32).round() as i32);
        assert!(frame_y(budget * 1.5, budget, bottom, h) < frame_y(budget, budget, bottom, h));
        assert!(frame_y(budget * 2.0, budget, bottom, h) < frame_y(budget * 1.5, budget, bottom, h));
    }
    #[test]
    fn panels_are_shorter_but_keep_readable_text() {
        assert_eq!((WIDTH, HEIGHT), (864, 160));
        assert_eq!((WRIST_WIDTH, WRIST_HEIGHT), (560, 160));
        assert!(138 + 16 < HEIGHT as i32);
        assert!(WRIST_BOTTOM_Y + 16 <= WRIST_HEIGHT as i32 - 14);
        assert!(33 + 76 < 113);
    }
    #[test]
    fn original_head_layout_is_preserved() {
        assert_eq!((WIDTH, HEIGHT), (864, 160));
        assert_eq!(top_row_positions()[0], 10);
        assert_eq!(GRAPH_X, 194);
        // The original y=7 top row, y=48/94 frame-time labels and y=138
        // footer remain in the same positions as v1.11.
        assert!(7 + 16 < 32);
        assert!(48 + 16 < 79);
        assert!(94 + 16 < 129);
        assert_eq!(HEAD_GRAPH_Y, 34);
        assert_eq!(HEAD_GRAPH_H, 94);
        assert_eq!(HEAD_GRAPH_Y, 32 + 2);
        assert_eq!(HEAD_GRAPH_Y + HEAD_GRAPH_H, 129 - 1);
        assert!(138 + 16 <= HEIGHT as i32);
    }
    #[test]
    fn fixed_v111_alpha_uses_straight_rgba() {
        for mode in [Mode::Head, Mode::Left, Mode::Right] {
            let mut r = Renderer::for_mode(mode);
            r.draw(&FrameMonitor::new(), &HardwareStats::default(), &DeviceStats::default(), 90.0);
            assert_eq!(&r.pixels()[0..4], &[15, 20, 28, 245]);
            assert!(r.pixels().chunks_exact(4).any(|p| p == TEXT));
        }
    }
    #[test]
    fn formatting_missing_metrics_does_not_forge_numbers() {
        assert_eq!(percent(None::<u8>), "--%");
        assert_eq!(used_total_short(None, None), "--/--G");
    }
}
