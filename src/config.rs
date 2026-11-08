use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Head,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Upload {
    Gl,
    Raw,
}

#[derive(Debug)]
pub struct Config {
    pub mode: Mode,
    pub upload: Upload,
    pub hz: Option<f32>,
    pub width_m: f32,
    pub opacity: f32,
    pub position: [f32; 3],
    pub interval_ms: u64,
}

impl Config {
    pub fn from_args() -> Result<Self, String> {
        Self::parse_args(env::args().skip(1))
    }

    fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Self, String> {
        let mut mode = Mode::Head;
        let mut upload = Upload::Gl;
        let mut hz = None;
        let mut width_m = 0.415;
        let mut opacity = 0.93;
        let mut position: Option<[f32; 3]> = None;
        let mut interval_ms = 200;
        // SteamVR can supply a trailing empty argument for registered overlay apps.
        let mut args = args.into_iter().filter(|arg| !arg.is_empty());

        while let Some(arg) = args.next() {
            let value = match arg.as_str() {
                "--help" | "-h" => {
                    println!("{}", Self::help());
                    std::process::exit(0);
                }
                "--version" => {
                    println!("vrperf {}", env!("CARGO_PKG_VERSION"));
                    std::process::exit(0);
                }
                "--mode" | "--hz" | "--width" | "--opacity" | "--position" | "--interval-ms" | "--upload" =>
                    args.next().ok_or_else(|| format!("Missing value for {arg}"))?,
                _ => return Err(format!("Unknown argument: {arg}\n{}", Self::help())),
            };

            match arg.as_str() {
                "--mode" => {
                    mode = match value.as_str() {
                        "head" => Mode::Head,
                        "left" => Mode::Left,
                        "right" => Mode::Right,
                        _ => return Err("--mode must be head, left or right".into()),
                    }
                }
                "--upload" => {
                    upload = match value.as_str() {
                        "gl" => Upload::Gl,
                        "raw" => Upload::Raw,
                        _ => return Err("--upload must be gl or raw".into()),
                    };
                }
                "--hz" => {
                    if value != "auto" {
                        let v = value.parse::<f32>().map_err(|_| "Invalid --hz".to_string())?;
                        if !v.is_finite() || !(30.0..=240.0).contains(&v) {
                            return Err("--hz must be between 30 and 240, or auto".into());
                        }
                        hz = Some(v);
                    }
                }
                "--width" => width_m = parse_f32(&value, 0.1, 1.5, "--width")?,
                "--opacity" => opacity = parse_f32(&value, 0.1, 1.0, "--opacity")?,
                "--position" => {
                    let coords: Vec<_> = value.split(',').collect();
                    if coords.len() != 3 {
                        return Err("--position must be x,y,z (metres), with no spaces".into());
                    }
                    position = Some([
                        parse_f32(coords[0], -3.0, 3.0, "x")?,
                        parse_f32(coords[1], -3.0, 3.0, "y")?,
                        parse_f32(coords[2], -3.0, 3.0, "z")?,
                    ]);
                }
                "--interval-ms" => {
                    interval_ms = value.parse::<u64>().map_err(|_| "Invalid --interval-ms".to_string())?;
                    if !(100..=1000).contains(&interval_ms) {
                        return Err("--interval-ms must be 100 to 1000".into());
                    }
                }
                _ => unreachable!(),
            }
        }

        // HMD faces negative Z. Controller-relative poses may vary by hardware.
        let position = position.unwrap_or(match mode {
            Mode::Head => [0.0, -0.32, -0.85],
            Mode::Left | Mode::Right => [0.0, 0.12, -0.13],
        });
        Ok(Self { mode, upload, hz, width_m, opacity, position, interval_ms })
    }

    fn help() -> &'static str {
        "vrperf - minimal SteamVR PC performance overlay\n\
         Usage: vrperf [options]\n\
         --mode head|left|right      HMD-fixed (default) or controller-relative overlay\n\
         --upload gl|raw           Upload method, default gl (raw fallback available)\n\
         --hz auto|NUMBER          HMD refresh rate (default auto)\n\
         --position X,Y,Z          Offset in metres relative to device\n\
         --width METRES            Overlay width, default 0.415\n\
         --opacity 0.1..1.0        Overlay opacity, default 0.93\n\
         --interval-ms 100..1000   Display update period, default 200\n\
         --install                 Install and register SteamVR autostart (start SteamVR first)\n\
         --uninstall               Unregister and remove installed binary\n\
         --version                 Print version\n\
         -h, --help                Show this help"
    }
}

fn parse_f32(raw: &str, min: f32, max: f32, name: &str) -> Result<f32, String> {
    let value = raw.parse::<f32>().map_err(|_| format!("Invalid {name}: {raw}"))?;
    if !value.is_finite() || !(min..=max).contains(&value) {
        return Err(format!("{name} must be between {min} and {max}"));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{parse_f32, Config};

    #[test]
    fn ignore_empty_steamvr_arguments() {
        let config = Config::parse_args(["".to_string()]).unwrap();
        assert_eq!(config.interval_ms, 200);
        let config = Config::parse_args([
            "".to_string(), "--interval-ms".to_string(), "1000".to_string(), "".to_string(),
        ]).unwrap();
        assert_eq!(config.interval_ms, 1000);
    }

    #[test]
    fn reject_nonempty_unknown_arguments() {
        assert!(Config::parse_args(["--unknown".to_string()]).is_err());
    }

    #[test]
    fn reject_nan_and_out_of_bounds() {
        assert!(parse_f32("NaN", 0.0, 1.0, "test").is_err());
        assert!(parse_f32("2", 0.0, 1.0, "test").is_err());
        assert_eq!(parse_f32("0.5", 0.0, 1.0, "test"), Ok(0.5));
    }
}
