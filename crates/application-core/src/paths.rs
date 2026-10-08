//! Shared app-data and download roots for desktop, seed, and tools.
//!
//! Override with `FINOS_APP_DATA` / `FINOS_DOWNLOAD_DIR`. Never hardcode an owner machine path.

use std::path::{Path, PathBuf};

pub const APP_DATA_FOLDER: &str = "com.finos.desktop";
pub const DOWNLOAD_FOLDER: &str = "Financial";

pub fn looks_like_sync_folder(path: &Path) -> bool {
    let s = path.to_string_lossy().to_lowercase();
    s.contains("onedrive")
        || s.contains("dropbox")
        || s.contains("icloud")
        || s.contains("mobile documents")
        || s.contains("clouddocs")
}

/// Directory that holds `local.sqlite`, restart token, page-loaded log, atlas evidence.
///
/// Order: `FINOS_APP_DATA` → platform default under Application Support / LOCALAPPDATA →
/// sync-folder fallback to `<local>/finos`.
pub fn profile_a_app_dir() -> PathBuf {
    if let Some(raw) = std::env::var_os("FINOS_APP_DATA") {
        if !raw.is_empty() {
            return PathBuf::from(raw);
        }
    }
    let preferred = platform_preferred_app_data();
    resolve_app_data_dir(preferred)
}

/// Prefer Tauri's `app_local_data_dir` when the host has it; otherwise platform default.
pub fn resolve_app_data_dir(preferred: PathBuf) -> PathBuf {
    if looks_like_sync_folder(&preferred) {
        if let Some(local) = local_data_parent() {
            return local.join("finos");
        }
    }
    preferred
}

fn platform_preferred_app_data() -> PathBuf {
    if let Some(parent) = local_data_parent() {
        return parent.join(APP_DATA_FOLDER);
    }
    PathBuf::from(APP_DATA_FOLDER)
}

/// Windows `%LOCALAPPDATA%`, macOS `~/Library/Application Support`, else XDG or home.
fn local_data_parent() -> Option<PathBuf> {
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        if !local.is_empty() {
            return Some(PathBuf::from(local));
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return Some(PathBuf::from(home).join("Library").join("Application Support"));
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
            if !xdg.is_empty() {
                return Some(PathBuf::from(xdg));
            }
        }
        if let Some(home) = std::env::var_os("HOME") {
            return Some(PathBuf::from(home).join(".local").join("share"));
        }
        if let Some(profile) = std::env::var_os("USERPROFILE") {
            return Some(PathBuf::from(profile).join("AppData").join("Local"));
        }
    }
    None
}

/// Owner downloads: Excel exports and snapshot folders. Not the database.
///
/// Order: `FINOS_DOWNLOAD_DIR` → `~/Documents/Financial` (or `%USERPROFILE%\Documents\Financial`).
pub fn download_dir() -> PathBuf {
    if let Some(root) = std::env::var_os("FINOS_DOWNLOAD_DIR") {
        if !root.is_empty() {
            return PathBuf::from(root);
        }
    }
    documents_dir().join(DOWNLOAD_FOLDER)
}

fn documents_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join("Documents");
    }
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        return PathBuf::from(profile).join("Documents");
    }
    PathBuf::from("Documents")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn download_dir_uses_documents_financial_when_env_clear() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("FINOS_DOWNLOAD_DIR");
        let path = download_dir();
        let s = path.to_string_lossy().replace('/', "\\");
        assert!(
            s.ends_with("Documents\\Financial") || s.ends_with("Documents/Financial"),
            "expected …/Documents/Financial from HOME/USERPROFILE, got {s}"
        );
    }

    #[test]
    fn download_dir_honors_env_override() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = std::env::temp_dir().join("finos-download-override-test");
        std::env::set_var("FINOS_DOWNLOAD_DIR", &tmp);
        assert_eq!(download_dir(), tmp);
        std::env::remove_var("FINOS_DOWNLOAD_DIR");
    }

    #[test]
    fn profile_a_honors_finos_app_data() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = std::env::temp_dir().join("finos-app-data-override-test");
        std::env::set_var("FINOS_APP_DATA", &tmp);
        assert_eq!(profile_a_app_dir(), tmp);
        std::env::remove_var("FINOS_APP_DATA");
    }

    #[test]
    fn sync_folder_detects_icloud_mobile_documents() {
        let preferred = PathBuf::from(
            "/Users/me/Library/Mobile Documents/com~apple~CloudDocs/com.finos.desktop",
        );
        assert!(looks_like_sync_folder(&preferred));
    }
}
