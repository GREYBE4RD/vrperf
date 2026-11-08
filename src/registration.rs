//! Installs to ~/.local/bin and registers a user-local SteamVR dashboard
//! manifest. Installation only runs when explicitly requested.

use std::env;
use std::error::Error;
use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const KEY: &str = "vrperf.linux.dashboard";

fn install_dir() -> Result<PathBuf, String> {
    let home = env::var_os("HOME").ok_or("HOME is not set")?;
    Ok(PathBuf::from(home).join(".local/share/vrperf"))
}

fn binary_path() -> Result<PathBuf, String> {
    let home = env::var_os("HOME").ok_or("HOME is not set")?;
    Ok(PathBuf::from(home).join(".local/bin/vrperf"))
}

fn to_c_path(path: &Path) -> Result<CString, String> {
    CString::new(path.as_os_str().as_bytes())
        .map_err(|_| "Path contains an invalid NUL character".into())
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn manifest(binary: &Path) -> String {
    let escaped_path = json_string(&binary.to_string_lossy());
    format!(r#"{{
  "source": "builtin",
  "applications": [{{
    "app_key": "{KEY}",
    "launch_type": "binary",
    "binary_path": {escaped_path},
    "binary_path_linux": {escaped_path},
    "is_dashboard_overlay": true,
    "strings": {{ "en_us": {{ "name": "vrperf", "description": "Linux PC VR performance overlay" }} }}
  }}]
}}
"#)
}

struct Applications(*const openvr_sys::VR_IVRApplications_FnTable);
impl Applications {
    fn connect() -> Result<Self, String> {
        // SAFETY: Caller must hold an initialized OpenVR context.
        unsafe {
            let mut error = 0;
            let api = openvr_sys::VR_GetGenericInterface(
                c"FnTable:IVRApplications_008".as_ptr(), &mut error,
            ) as *const openvr_sys::VR_IVRApplications_FnTable;
            if api.is_null() { return Err(format!("SteamVR application API unavailable: {error}")); }
            Ok(Self(api))
        }
    }
    fn add(&self, path: &Path) -> Result<(), String> {
        let path = to_c_path(path)?;
        // SAFETY: The string lives for the duration of this FFI call.
        let result = unsafe { (*self.0).AddApplicationManifest
            .ok_or("AddApplicationManifest unavailable")?(path.as_ptr() as *mut _, false) };
        // Re-registering the same application key after upgrading is not fatal.
        if result != 0 && result != 100 {
            return Err(format!("SteamVR manifest registration failed ({result})"));
        }
        Ok(())
    }
    fn autolaunch(&self, enabled: bool) -> Result<(), String> {
        let key = CString::new(KEY).expect("static key");
        // SAFETY: The string lives for the duration of the FFI call.
        let result = unsafe { (*self.0).SetApplicationAutoLaunch
            .ok_or("SetApplicationAutoLaunch unavailable")?(key.as_ptr() as *mut _, enabled) };
        if result != 0 { return Err(format!("SteamVR autostart setting failed ({result})")); }
        Ok(())
    }
    fn remove(&self, path: &Path) -> Result<(), String> {
        let path = to_c_path(path)?;
        // SAFETY: The string lives for the duration of the FFI call.
        let result = unsafe { (*self.0).RemoveApplicationManifest
            .ok_or("RemoveApplicationManifest unavailable")?(path.as_ptr() as *mut _) };
        if result != 0 { return Err(format!("SteamVR manifest removal failed ({result})")); }
        Ok(())
    }
}

pub fn install() -> Result<(), Box<dyn Error>> {
    // Check SteamVR before changing the installed files.
    let context = unsafe { openvr::init(openvr::ApplicationType::Utility) }
        .map_err(|e| format!("Connect to running SteamVR before --install: {e}"))?;
    let apps = Applications::connect()?;

    let dir = install_dir()?;
    let binary = binary_path()?;
    let bin_dir = binary.parent().ok_or("Invalid executable path")?;
    fs::create_dir_all(&dir)?;
    fs::create_dir_all(bin_dir)?;

    // Use a temporary file and rename to avoid leaving a partial executable.
    let executable = env::current_exe()?;
    if executable != binary {
        let staging = bin_dir.join("vrperf.tmp");
        fs::copy(&executable, &staging)?;
        fs::set_permissions(&staging, fs::Permissions::from_mode(0o755))?;
        fs::rename(&staging, &binary)?;
    }

    let path = dir.join("vrperf.vrmanifest");
    let manifest_staging = dir.join("vrperf.vrmanifest.tmp");
    fs::write(&manifest_staging, manifest(&binary))?;
    fs::rename(&manifest_staging, &path)?;

    // Re-add the same manifest path so SteamVR reloads its updated binary path.
    apps.add(&path)?;
    apps.autolaunch(true)?;
    drop(context);

    // Only remove the previous v1.6 executable after registration succeeds.
    let old_binary = dir.join("vrperf");
    if old_binary != binary && old_binary.is_file() {
        if let Err(error) = fs::remove_file(&old_binary) {
            eprintln!("vrperf: could not remove previous executable {}: {error}", old_binary.display());
        }
    }

    println!("Installed: {}", binary.display());
    println!("Registered SteamVR startup overlay: {KEY}");
    println!("Restart SteamVR to test automatic startup.");
    Ok(())
}

pub fn uninstall() -> Result<(), Box<dyn Error>> {
    let dir = install_dir()?;
    let path = dir.join("vrperf.vrmanifest");
    // Keep the manifest in place until registration has been removed.
    let context = unsafe { openvr::init(openvr::ApplicationType::Utility) }
        .map_err(|e| format!("Connect to running SteamVR before --uninstall: {e}"))?;
    let apps = Applications::connect()?;
    apps.autolaunch(false)?;
    apps.remove(&path)?;
    drop(context);
    if path.exists() { fs::remove_file(&path)?; }
    let executable = binary_path()?;
    if executable.exists() || executable.is_symlink() { fs::remove_file(&executable)?; }
    // Also clean up an old v1.6 installation if present.
    let old_binary = dir.join("vrperf");
    if old_binary.exists() { fs::remove_file(old_binary)?; }
    println!("Removed SteamVR autostart registration and installed vrperf binary.");
    Ok(())
}

pub fn visibility_path() -> Option<PathBuf> {
    if let Some(config) = env::var_os("XDG_CONFIG_HOME") {
        if !config.is_empty() {
            return Some(PathBuf::from(config).join("vrperf/visible"));
        }
    }
    env::var_os("HOME").map(|home| PathBuf::from(home).join(".config/vrperf/visible"))
}

pub fn read_visible() -> bool {
    let Some(path) = visibility_path() else { return true; };
    fs::read_to_string(path).map(|s| s.trim() != "off").unwrap_or(true)
}

pub fn write_visible(enabled: bool) -> Result<(), String> {
    let path = visibility_path().ok_or("Could not determine the configuration directory")?;
    let dir = path.parent().ok_or("Invalid configuration directory")?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    fs::write(path, if enabled { "on\n" } else { "off\n" }).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manifest_has_matching_dashboard_key_and_binary() {
        let s = manifest(Path::new("/home/test/.local/bin/vrperf"));
        assert!(s.contains("\"app_key\": \"vrperf.linux.dashboard\""));
        assert!(s.contains("\"is_dashboard_overlay\": true"));
        assert!(s.contains("\"binary_path_linux\""));
        assert!(s.contains("/home/test/.local/bin/vrperf"));
    }
    #[test]
    fn json_escapes_quotes_and_backslashes() {
        assert_eq!(json_string("a"), "\"a\"");
        assert!(json_string("a\"b").contains("\\\""));
    }
}
