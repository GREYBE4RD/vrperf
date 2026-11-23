use std::collections::VecDeque;
use std::time::{Duration, Instant};

// Keep enough individual frames to fill the widest graph after warm-up.
pub(crate) const MAX_HISTORY: usize = 768;
const OPENVR_HISTORY: u32 = 128;
const FPS_WINDOW_SECONDS: f64 = 1.0;

#[derive(Clone, Copy)]
pub struct FrameSample {
    pub cpu_ms: f32,
    pub gpu_ms: f32,
    pub dropped: u32,
}

pub struct FrameMonitor {
    pub samples: VecDeque<FrameSample>,
    pub fps: Option<f32>,
    pub dropped: u64,
    active_scene: Option<u32>,
    // OpenVR supplies a clock time tied to the frame, not the time we happened to poll.
    // FPS is measured over a time window; an instantaneous WaitGetPoses interval
    // can give misleading values such as 600 FPS.
    frame_times: VecDeque<f64>,
    last_index: Option<u32>,
    last_frame_at: Option<Instant>,
}

impl FrameMonitor {
    pub fn new() -> Self {
        Self {
            samples: VecDeque::with_capacity(MAX_HISTORY),
            fps: None,
            dropped: 0,
            active_scene: None,
            frame_times: VecDeque::with_capacity(256),
            last_index: None,
            last_frame_at: None,
        }
    }

    /// Do not interpret compositor-only frames as application frame times.
    /// Reset all per-game history on entering/leaving or changing a scene.
    pub fn set_scene(&mut self, process: Option<u32>, compositor: &openvr::Compositor) {
        if self.active_scene == process { return; }
        self.active_scene = process;
        self.samples.clear();
        self.frame_times.clear();
        self.fps = None;
        self.dropped = 0;
        self.last_frame_at = None;
        // Ignore compositor history from the previous scene at transition.
        self.last_index = compositor.get_frame_timing(0).map(|f| f.m_nFrameIndex);
    }

    pub fn is_active(&self) -> bool { self.active_scene.is_some() }

    fn push_time(&mut self, time: f64) {
        if !time.is_finite() || time <= 0.0 { return; }
        if let Some(&previous) = self.frame_times.back() {
            if time <= previous {
                // Ignore duplicate or invalid timestamps, but clear after an obvious clock reset.
                if previous - time > 2.0 { self.frame_times.clear(); }
                else { return; }
            }
        }
        self.frame_times.push_back(time);
        while self.frame_times.len() > 2
            && time - self.frame_times[1] > FPS_WINDOW_SECONDS {
            self.frame_times.pop_front();
        }
        while self.frame_times.len() > 256 { self.frame_times.pop_front(); }
        self.fps = Self::calculate_fps(&self.frame_times);
    }

    fn calculate_fps(times: &VecDeque<f64>) -> Option<f32> {
        let first = *times.front()?;
        let last = *times.back()?;
        let delta = last - first;
        if times.len() < 4 || delta < 0.2 { return None; }
        let fps = (times.len() - 1) as f64 / delta;
        (fps.is_finite() && fps > 0.0).then_some(fps as f32)
    }

    pub fn poll(&mut self, compositor: &openvr::Compositor) {
        if !self.is_active() { return; }
        let Some(newest) = compositor.get_frame_timing(0) else { return; };
        let frame_id = newest.m_nFrameIndex;
        if self.last_index == Some(frame_id) { return; }
        let count = match self.last_index {
            Some(last) => {
                let difference = frame_id.wrapping_sub(last);
                if difference == 0 || difference > 200_000 {
                    self.samples.clear();
                    self.frame_times.clear();
                    self.fps = None;
                    self.dropped = 0;
                    self.last_index = None;
                    1
                } else {
                    difference.min(OPENVR_HISTORY)
                }
            }
            None => 1,
        };

        // Oldest first. This preserves individual frames and short spikes in the graphs.
        for ago in (0..count).rev() {
            let Some(frame) = compositor.get_frame_timing(ago) else { continue; };
            if let Some(last) = self.last_index {
                if frame.m_nFrameIndex == last { continue; }
            }
            let cpu = frame.m_flNewFrameReadyMs - frame.m_flNewPosesReadyMs
                + frame.m_flCompositorRenderCpuMs;
            let gpu = frame.m_flPreSubmitGpuMs;
            if !(cpu.is_finite() && gpu.is_finite()) || cpu < 0.0 || gpu < 0.0
                || cpu > 1000.0 || gpu > 1000.0 {
                continue;
            }
            if self.samples.len() == MAX_HISTORY { self.samples.pop_front(); }
            self.samples.push_back(FrameSample { cpu_ms: cpu, gpu_ms: gpu, dropped: frame.m_nNumDroppedFrames });
            self.dropped = self.dropped.saturating_add(frame.m_nNumDroppedFrames as u64);
            self.push_time(frame.m_flSystemTimeInSeconds);
            self.last_index = Some(frame.m_nFrameIndex);
            self.last_frame_at = Some(Instant::now());
        }
    }

    pub fn is_recent(&self) -> bool {
        self.is_active() && self.last_frame_at.is_some_and(|time| time.elapsed() < Duration::from_secs(2))
    }

    pub fn latest(&self) -> Option<FrameSample> {
        self.is_recent().then(|| self.samples.back().copied()).flatten()
    }

    pub fn current_fps(&self, hz: f32) -> Option<f32> {
        if !self.is_recent() { return None; }
        self.fps.map(|value| value.min(hz))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_idle() {
        let monitor = FrameMonitor::new();
        assert!(!monitor.is_active());
        assert_eq!(monitor.latest().map(|frame| frame.gpu_ms), None);
    }

    #[test]
    fn rolling_fps_averages_frames_not_single_intervals() {
        let mut monitor = FrameMonitor::new();
        for index in 0..90 {
            monitor.push_time(100.0 + index as f64 / 90.0);
        }
        let fps = monitor.fps.unwrap();
        assert!((fps - 90.0).abs() < 0.1, "{fps}");
    }

    #[test]
    fn fps_rejects_bad_timestamps() {
        let mut monitor = FrameMonitor::new();
        for time in [100.0, 100.01, 100.01, f64::NAN, 100.02, 100.03] {
            monitor.push_time(time);
        }
        assert_eq!(monitor.frame_times.len(), 4);
    }
}
