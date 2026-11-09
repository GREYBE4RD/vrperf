use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode { Head, Left, Right }
impl Mode {
    pub fn name(self) -> &'static str {
        match self { Self::Head => "head", Self::Left => "left", Self::Right => "right" }
    }
    pub fn from_name(s: &str) -> Option<Self> {
        match s { "head" => Some(Self::Head), "left" => Some(Self::Left),
            "right" => Some(Self::Right), _ => None }
    }
    pub fn wrist(self) -> bool { self != Self::Head }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Upload { Gl, Raw }

#[derive(Debug)]
pub struct Config {
    pub mode: Mode,
    pub upload: Upload,
    pub hz: Option<f32>,
    pub width_m: f32,
    pub position: [f32; 3],
    pub interval_ms: u64,
    pub explicit_mode: bool,
    pub capture_frame: Option<String>,
}
impl Config {
    pub fn from_args() -> Result<Self, String> { Self::parse_args(env::args().skip(1)) }
    fn parse_args<I: IntoIterator<Item=String>>(args: I) -> Result<Self, String> {
        let mut config = Self { mode: Mode::Head, upload: Upload::Gl, hz: None,
            width_m: 0.415, position: [0.0, -0.32, -0.85], interval_ms: 200,
            explicit_mode: false, capture_frame: None };
        let mut args = args.into_iter().filter(|a| !a.is_empty());
        while let Some(option) = args.next() {
            match option.as_str() {
                "--help" | "-h" => { println!("{}", Self::help()); std::process::exit(0); }
                "--version" => { println!("vrperf {}", env!("CARGO_PKG_VERSION")); std::process::exit(0); }
                _ => {}
            }
            let value = match option.as_str() {
                "--mode" | "--upload" | "--hz" | "--width" | "--position" | "--interval-ms" | "--capture-frame" =>
                    args.next().ok_or_else(|| format!("Missing value for {option}"))?,
                _ => return Err(format!("Unknown option: {option}\n{}", Self::help())),
            };
            match option.as_str() {
                "--mode" => {
                    config.mode = Mode::from_name(&value).ok_or("--mode must be head, left or right")?;
                    config.explicit_mode = true;
                }
                "--upload" => config.upload = match value.as_str() {
                    "gl" => Upload::Gl, "raw" => Upload::Raw, _ => return Err("--upload must be gl or raw".into()),
                },
                "--hz" => if value != "auto" { config.hz = Some(parse_f32(&value,30.0,240.0,"--hz")?); },
                "--width" => config.width_m = parse_f32(&value,0.1,1.5,"--width")?,
                "--position" => {
                    let parts: Vec<_> = value.split(',').collect();
                    if parts.len()!=3 { return Err("--position requires x,y,z".into()); }
                    config.position = [parse_f32(parts[0],-3.0,3.0,"x")?,
                        parse_f32(parts[1],-3.0,3.0,"y")?, parse_f32(parts[2],-3.0,3.0,"z")?];
                }
                "--interval-ms" => {
                    config.interval_ms = value.parse().map_err(|_| "Invalid --interval-ms")?;
                    if !(100..=1000).contains(&config.interval_ms) { return Err("--interval-ms must be 100..1000".into()); }
                }
                "--capture-frame" => {
                    if value.is_empty() { return Err("--capture-frame requires a PNG path".into()); }
                    config.capture_frame = Some(value);
                }
                _ => unreachable!(),
            }
        }
        Ok(config)
    }
    pub fn head_position(&self) -> [f32;3] { self.position }
    fn help() -> &'static str {
        "vrperf - Linux SteamVR performance overlay\n\
         --mode head|left|right       Fixed head display or calibrated watch (L/R)\n\
         --upload gl|raw            Image upload method (GL recommended)\n\
         --hz auto|NUMBER           HMD refresh rate\n\
         --width METRES             Head overlay width (default 0.415)\n\
         --position x,y,z           Head-relative position in metres\n\
         --interval-ms 100..1000    Display update interval (default 200)\n\
         --capture-frame PATH       Save the source RGBA image as PNG\n\
         --install                  Install with SteamVR autostart\n\
         --uninstall                Unregister and uninstall\n\
         --version                  Print version\n\
         --help                     Show this help\n\
         Configure each watch's position and size in the vrperf SteamVR dashboard."
    }
}
fn parse_f32(text: &str, minimum: f32, maximum: f32, label: &str) -> Result<f32,String> {
    let value: f32 = text.parse().map_err(|_| format!("Invalid {label}: {text}"))?;
    if !value.is_finite() || !(minimum..=maximum).contains(&value) {
        return Err(format!("{label} must be between {minimum} and {maximum}"));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn modes_and_empty_steamvr_args() {
        assert_eq!(Config::parse_args(["".into()]).unwrap().mode, Mode::Head);
        assert_eq!(Config::parse_args(["--mode".into(),"left".into()]).unwrap().mode, Mode::Left);
        assert!(Config::parse_args(["--mode".into(),"flat".into()]).is_err());
    }
    #[test] fn no_removed_options() {
        for opt in ["--opacity", "--wrist-style", "--wrist-placement"] {
            assert!(Config::parse_args([opt.into(), "0.5".into()]).is_err());
        }
    }
    #[test] fn rejects_bad_values() {
        assert!(Config::parse_args(["--hz".into(),"NaN".into()]).is_err());
        assert!(Config::parse_args(["--interval-ms".into(),"3".into()]).is_err());
        assert!(Config::parse_args(["--position".into(),"0,0".into()]).is_err());
        assert!(Config::parse_args(["--unexpected".into()]).is_err());
    }
}
